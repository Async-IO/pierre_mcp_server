// ABOUTME: Google Vertex AI LLM provider for GCP-native Gemini access
// ABOUTME: Uses service account authentication, ideal for Cloud Run deployments
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Vertex AI Provider
//!
//! Implementation of the `LlmProvider` trait for Google's Vertex AI platform.
//! This provider uses GCP service account authentication, making it ideal for
//! Cloud Run and other GCP deployments.
//!
//! ## Configuration
//!
//! Required environment variables:
//! - `GCP_PROJECT_ID`: Your GCP project ID
//! - `GCP_REGION`: Region for Vertex AI (default: `us-central1`)
//!
//! Authentication is handled via Application Default Credentials (ADC):
//! - In Cloud Run: Automatic via metadata server
//! - Locally: Run `gcloud auth application-default login`
//!
//! ## Supported Models
//!
//! - `gemini-1.5-flash` (default): Fast, cost-effective
//! - `gemini-1.5-pro`: Advanced reasoning
//! - `gemini-2.0-flash-exp`: Latest experimental
//!
//! ## Pricing Advantage
//!
//! Unlike Google AI Studio (free tier with 20 req/day limit), Vertex AI
//! is pay-per-use with no artificial rate limits, making it suitable for
//! production chat applications.

use std::env;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures_util::stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::process::Command as TokioCommand;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

use super::{
    ChatMessage, ChatRequest, ChatResponse, ChatResponseWithTools, ChatStream, FunctionCall,
    LlmCapabilities, LlmProvider, MessageRole, StreamChunk, TokenUsage, Tool,
};
use crate::errors::AppError;

/// Environment variable for GCP project ID
const GCP_PROJECT_ID_ENV: &str = "GCP_PROJECT_ID";

/// Environment variable for GCP region
const GCP_REGION_ENV: &str = "GCP_REGION";

/// Default GCP region for Vertex AI
const DEFAULT_REGION: &str = "us-central1";

/// Default model to use
const DEFAULT_MODEL: &str = "gemini-1.5-flash";

/// Available Vertex AI Gemini models
const AVAILABLE_MODELS: &[&str] = &[
    "gemini-1.5-flash",
    "gemini-1.5-pro",
    "gemini-2.0-flash-exp",
    "gemini-1.0-pro",
];

/// Token refresh buffer - refresh 5 minutes before expiry
const TOKEN_REFRESH_BUFFER_SECS: u64 = 300;

// ============================================================================
// API Request/Response Types (same as Gemini, different endpoint)
// ============================================================================

/// Vertex AI API request structure
#[derive(Debug, Serialize)]
struct VertexRequest {
    contents: Vec<VertexContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<VertexContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Tool>>,
}

/// Content structure for Vertex AI API
#[derive(Debug, Serialize, Deserialize)]
struct VertexContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    parts: Vec<ContentPart>,
}

/// Part of content (text, function call, or function response)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum ContentPart {
    /// Text content
    Text { text: String },
    /// Function call from the model
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: FunctionCallData,
    },
    /// Function response from the user
    FunctionResponse {
        #[serde(rename = "functionResponse")]
        function_response: FunctionResponseData,
    },
}

/// Function call data
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FunctionCallData {
    name: String,
    args: serde_json::Value,
}

/// Function response data
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FunctionResponseData {
    name: String,
    response: serde_json::Value,
}

/// Generation configuration
#[derive(Debug, Serialize)]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    candidate_count: Option<u32>,
}

/// Vertex AI API response structure
#[derive(Debug, Deserialize)]
struct VertexResponse {
    candidates: Option<Vec<Candidate>>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<UsageMetadata>,
    error: Option<VertexError>,
}

/// Response candidate
#[derive(Debug, Deserialize)]
struct Candidate {
    content: Option<VertexContent>,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

/// Usage metadata from Vertex AI response
#[derive(Debug, Deserialize)]
struct UsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt: Option<u32>,
    #[serde(rename = "candidatesTokenCount")]
    candidates: Option<u32>,
    #[serde(rename = "totalTokenCount")]
    total: Option<u32>,
}

/// API error response
#[derive(Debug, Deserialize)]
struct VertexError {
    message: String,
}

// ============================================================================
// GCP Authentication
// ============================================================================

/// Cached access token with expiry tracking
struct CachedToken {
    token: String,
    expires_at: Instant,
}

/// Response from GCP metadata server token endpoint
#[derive(Deserialize)]
struct MetadataTokenResponse {
    access_token: String,
}

/// GCP access token provider using Application Default Credentials
struct GcpAuth {
    cached_token: Arc<RwLock<Option<CachedToken>>>,
    client: Client,
}

impl GcpAuth {
    fn new(client: Client) -> Self {
        Self {
            cached_token: Arc::new(RwLock::new(None)),
            client,
        }
    }

    /// Get a valid access token, refreshing if necessary
    async fn get_token(&self) -> Result<String, AppError> {
        // Check cached token
        {
            let cache = self.cached_token.read().await;
            if let Some(ref cached) = *cache {
                if cached.expires_at > Instant::now() {
                    return Ok(cached.token.clone());
                }
            }
        }

        // Refresh token
        let token = self.fetch_new_token().await?;

        // Cache it
        {
            let mut cache = self.cached_token.write().await;
            *cache = Some(CachedToken {
                token: token.clone(),
                // GCP tokens are valid for 1 hour, refresh 5 min early
                expires_at: Instant::now() + Duration::from_secs(3600 - TOKEN_REFRESH_BUFFER_SECS),
            });
        }

        Ok(token)
    }

    /// Fetch a new access token from GCP metadata server or ADC
    async fn fetch_new_token(&self) -> Result<String, AppError> {
        // Try metadata server first (works in Cloud Run, GCE, GKE)
        if let Ok(token) = self.fetch_from_metadata_server().await {
            debug!("Obtained GCP token from metadata server");
            return Ok(token);
        }

        // Fall back to gcloud CLI (works locally after `gcloud auth application-default login`)
        if let Ok(token) = self.fetch_from_gcloud_cli().await {
            debug!("Obtained GCP token from gcloud CLI");
            return Ok(token);
        }

        Err(AppError::config(
            "Failed to obtain GCP access token. In Cloud Run, this should be automatic. \
             Locally, run: gcloud auth application-default login"
                .to_owned(),
        ))
    }

    /// Fetch token from GCP metadata server (Cloud Run, GCE, GKE)
    async fn fetch_from_metadata_server(&self) -> Result<String, AppError> {
        let url = "http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token";

        let response = self
            .client
            .get(url)
            .header("Metadata-Flavor", "Google")
            .timeout(Duration::from_secs(2))
            .send()
            .await
            .map_err(|e| AppError::internal(format!("Metadata server request failed: {e}")))?;

        if !response.status().is_success() {
            return Err(AppError::internal("Metadata server returned error"));
        }

        let token_response: MetadataTokenResponse = response
            .json()
            .await
            .map_err(|e| AppError::internal(format!("Failed to parse token response: {e}")))?;

        Ok(token_response.access_token)
    }

    /// Fetch token using gcloud CLI (local development)
    async fn fetch_from_gcloud_cli(&self) -> Result<String, AppError> {
        let output = TokioCommand::new("gcloud")
            .args(["auth", "application-default", "print-access-token"])
            .output()
            .await
            .map_err(|e| AppError::internal(format!("Failed to run gcloud: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::internal(format!("gcloud auth failed: {stderr}")));
        }

        let token = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if token.is_empty() {
            return Err(AppError::internal("gcloud returned empty token"));
        }

        Ok(token)
    }
}

// ============================================================================
// Provider Implementation
// ============================================================================

/// Google Vertex AI LLM provider
///
/// Uses GCP service account authentication for production deployments.
/// Ideal for Cloud Run where authentication is automatic.
pub struct VertexAiProvider {
    project_id: String,
    region: String,
    client: Client,
    auth: GcpAuth,
    default_model: String,
}

impl VertexAiProvider {
    /// Create a new Vertex AI provider with explicit configuration
    #[must_use]
    pub fn new(project_id: impl Into<String>, region: impl Into<String>) -> Self {
        let client = Client::new();
        Self {
            project_id: project_id.into(),
            region: region.into(),
            auth: GcpAuth::new(client.clone()),
            client,
            default_model: DEFAULT_MODEL.to_owned(),
        }
    }

    /// Create a provider from environment variables
    ///
    /// Reads:
    /// - `GCP_PROJECT_ID`: Required
    /// - `GCP_REGION`: Optional, defaults to `us-central1`
    ///
    /// # Errors
    ///
    /// Returns an error if `GCP_PROJECT_ID` is not set.
    pub fn from_env() -> Result<Self, AppError> {
        let project_id = env::var(GCP_PROJECT_ID_ENV).map_err(|_| {
            AppError::config(format!(
                "{GCP_PROJECT_ID_ENV} environment variable not set. \
                 Required for Vertex AI provider."
            ))
        })?;

        let region = env::var(GCP_REGION_ENV).unwrap_or_else(|_| DEFAULT_REGION.to_owned());

        info!(
            "Initializing Vertex AI provider for project '{}' in region '{}'",
            project_id, region
        );

        Ok(Self::new(project_id, region))
    }

    /// Set a custom default model
    #[must_use]
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// Build the Vertex AI API URL for a model and method
    fn build_url(&self, model: &str, method: &str) -> String {
        format!(
            "https://{region}-aiplatform.googleapis.com/v1/projects/{project}/locations/{region}/publishers/google/models/{model}:{method}",
            region = self.region,
            project = self.project_id,
            model = model,
            method = method
        )
    }

    /// Convert message role to Vertex AI format
    const fn convert_role(role: MessageRole) -> &'static str {
        match role {
            MessageRole::System | MessageRole::User => "user",
            MessageRole::Assistant => "model",
        }
    }

    /// Convert chat messages to Vertex AI format
    fn convert_messages(messages: &[ChatMessage]) -> (Vec<VertexContent>, Option<VertexContent>) {
        let mut contents = Vec::new();
        let mut system_instruction = None;

        for message in messages {
            if message.role == MessageRole::System {
                system_instruction = Some(VertexContent {
                    role: None,
                    parts: vec![ContentPart::Text {
                        text: message.content.clone(),
                    }],
                });
            } else {
                contents.push(VertexContent {
                    role: Some(Self::convert_role(message.role).to_owned()),
                    parts: vec![ContentPart::Text {
                        text: message.content.clone(),
                    }],
                });
            }
        }

        (contents, system_instruction)
    }

    /// Build a Vertex AI request from a `ChatRequest`
    fn build_request(request: &ChatRequest, tools: Option<Vec<Tool>>) -> VertexRequest {
        let (contents, system_instruction) = Self::convert_messages(&request.messages);

        let generation_config = if request.temperature.is_some() || request.max_tokens.is_some() {
            Some(GenerationConfig {
                temperature: request.temperature,
                max_output_tokens: request.max_tokens,
                candidate_count: Some(1),
            })
        } else {
            None
        };

        VertexRequest {
            contents,
            system_instruction,
            generation_config,
            tools,
        }
    }

    /// Extract text content from response
    fn extract_content(response: &VertexResponse) -> Result<String, AppError> {
        let part = response
            .candidates
            .as_ref()
            .and_then(|c| c.first())
            .and_then(|c| c.content.as_ref())
            .and_then(|c| c.parts.first())
            .ok_or_else(|| AppError::internal("No content in Vertex AI response"))?;

        match part {
            ContentPart::Text { text } => Ok(text.clone()),
            ContentPart::FunctionCall { function_call } => Ok(format!(
                "{{\"function_call\": {{\"name\": \"{}\", \"args\": {}}}}}",
                function_call.name, function_call.args
            )),
            ContentPart::FunctionResponse { .. } => {
                Err(AppError::internal("Unexpected function response in output"))
            }
        }
    }

    /// Extract function calls from response
    fn extract_function_calls(response: &VertexResponse) -> Vec<FunctionCall> {
        response
            .candidates
            .as_ref()
            .and_then(|c| c.first())
            .and_then(|c| c.content.as_ref())
            .map(|c| {
                c.parts
                    .iter()
                    .filter_map(|p| {
                        if let ContentPart::FunctionCall { function_call } = p {
                            Some(FunctionCall {
                                name: function_call.name.clone(),
                                args: function_call.args.clone(),
                            })
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Convert usage metadata
    fn convert_usage(metadata: &UsageMetadata) -> TokenUsage {
        TokenUsage {
            prompt_tokens: metadata.prompt.unwrap_or(0),
            completion_tokens: metadata.candidates.unwrap_or(0),
            total_tokens: metadata.total.unwrap_or(0),
        }
    }

    /// Complete a chat request with tool support
    ///
    /// # Errors
    ///
    /// Returns an error if the API call fails.
    #[instrument(skip(self, request, tools), fields(model = %request.model.as_deref().unwrap_or(DEFAULT_MODEL)))]
    pub async fn complete_with_tools(
        &self,
        request: &ChatRequest,
        tools: Option<Vec<Tool>>,
    ) -> Result<ChatResponseWithTools, AppError> {
        let model = request.model.as_deref().unwrap_or(&self.default_model);
        let url = self.build_url(model, "generateContent");
        let vertex_request = Self::build_request(request, tools);

        let token = self.auth.get_token().await?;

        debug!("Sending request to Vertex AI");

        let response = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&vertex_request)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("HTTP request failed: {e}")))?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|e| AppError::internal(format!("Failed to read response: {e}")))?;

        if !status.is_success() {
            error!(status = %status, body = %response_text, "Vertex AI API error");
            return Err(AppError::internal(format!(
                "Vertex AI API error ({status}): {response_text}"
            )));
        }

        let vertex_response: VertexResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                error!(error = %e, response = %response_text, "Failed to parse response");
                AppError::internal(format!("Failed to parse Vertex AI response: {e}"))
            })?;

        if let Some(error) = vertex_response.error {
            return Err(AppError::internal(format!(
                "Vertex AI error: {}",
                error.message
            )));
        }

        // Check for function calls
        let function_calls = Self::extract_function_calls(&vertex_response);
        if !function_calls.is_empty() {
            debug!(count = function_calls.len(), "Extracted function calls");
            return Ok(ChatResponseWithTools {
                content: None,
                function_calls: Some(function_calls),
                model: model.to_owned(),
                usage: vertex_response
                    .usage_metadata
                    .as_ref()
                    .map(Self::convert_usage),
                finish_reason: vertex_response
                    .candidates
                    .as_ref()
                    .and_then(|c| c.first())
                    .and_then(|c| c.finish_reason.clone()),
            });
        }

        // Extract text content
        let content = Self::extract_content(&vertex_response)?;

        Ok(ChatResponseWithTools {
            content: Some(content),
            function_calls: None,
            model: model.to_owned(),
            usage: vertex_response
                .usage_metadata
                .as_ref()
                .map(Self::convert_usage),
            finish_reason: vertex_response
                .candidates
                .as_ref()
                .and_then(|c| c.first())
                .and_then(|c| c.finish_reason.clone()),
        })
    }
}

#[async_trait]
impl LlmProvider for VertexAiProvider {
    fn name(&self) -> &'static str {
        "vertex"
    }

    fn display_name(&self) -> &'static str {
        "Google Vertex AI"
    }

    fn capabilities(&self) -> LlmCapabilities {
        LlmCapabilities::full_featured()
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }

    fn available_models(&self) -> &'static [&'static str] {
        AVAILABLE_MODELS
    }

    #[instrument(skip(self, request), fields(model = %request.model.as_deref().unwrap_or(DEFAULT_MODEL)))]
    async fn complete(&self, request: &ChatRequest) -> Result<ChatResponse, AppError> {
        let model = request.model.as_deref().unwrap_or(&self.default_model);
        let url = self.build_url(model, "generateContent");
        let vertex_request = Self::build_request(request, None);

        let token = self.auth.get_token().await?;

        debug!("Sending request to Vertex AI");

        let response = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&vertex_request)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("HTTP request failed: {e}")))?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|e| AppError::internal(format!("Failed to read response: {e}")))?;

        if !status.is_success() {
            error!(status = %status, "Vertex AI API error");
            return Err(AppError::internal(format!(
                "Vertex AI API error ({status}): {response_text}"
            )));
        }

        let vertex_response: VertexResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                error!(error = %e, "Failed to parse Vertex AI response");
                AppError::internal(format!("Failed to parse response: {e}"))
            })?;

        if let Some(error) = vertex_response.error {
            return Err(AppError::internal(format!(
                "Vertex AI error: {}",
                error.message
            )));
        }

        let content = Self::extract_content(&vertex_response)?;
        let usage = vertex_response
            .usage_metadata
            .as_ref()
            .map(Self::convert_usage);
        let finish_reason = vertex_response
            .candidates
            .as_ref()
            .and_then(|c| c.first())
            .and_then(|c| c.finish_reason.clone());

        debug!("Successfully received Vertex AI response");

        Ok(ChatResponse {
            content,
            model: model.to_owned(),
            usage,
            finish_reason,
        })
    }

    #[instrument(skip(self, request), fields(model = %request.model.as_deref().unwrap_or(DEFAULT_MODEL)))]
    async fn complete_stream(&self, request: &ChatRequest) -> Result<ChatStream, AppError> {
        // Vertex AI streaming uses Server-Sent Events similar to Gemini.
        // Non-streaming fallback provides simpler, more reliable behavior.
        debug!("Using non-streaming mode for Vertex AI request");

        let response = self.complete(request).await?;

        let stream = stream::once(async move {
            Ok(StreamChunk {
                delta: response.content,
                is_final: true,
                finish_reason: response.finish_reason,
            })
        });

        Ok(Box::pin(stream))
    }

    #[instrument(skip(self))]
    async fn health_check(&self) -> Result<bool, AppError> {
        // Try to get a token - this validates auth is working
        match self.auth.get_token().await {
            Ok(_) => {
                debug!("Vertex AI health check passed");
                Ok(true)
            }
            Err(e) => {
                warn!(error = %e, "Vertex AI health check failed");
                Ok(false)
            }
        }
    }
}

impl Debug for VertexAiProvider {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("VertexAiProvider")
            .field("project_id", &self.project_id)
            .field("region", &self.region)
            .field("default_model", &self.default_model)
            .finish_non_exhaustive()
    }
}
