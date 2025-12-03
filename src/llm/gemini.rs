// ABOUTME: Google Gemini LLM provider implementation with streaming support
// ABOUTME: Supports Gemini Pro and Gemini Pro Vision models via the Generative AI API
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Gemini Provider
//!
//! Implementation of the `LlmProvider` trait for Google's Gemini models.
//!
//! ## Configuration
//!
//! Set the `GEMINI_API_KEY` environment variable with your API key from
//! Google AI Studio: <https://makersuite.google.com/app/apikey>
//!
//! ## Supported Models
//!
//! - `gemini-2.0-flash-exp` (default): Fast, efficient model
//! - `gemini-1.5-pro`: Advanced reasoning capabilities
//! - `gemini-1.5-flash`: Balanced performance and cost
//!
//! ## Example
//!
//! ```rust,no_run
//! use pierre_mcp_server::llm::{GeminiProvider, LlmProvider, ChatRequest, ChatMessage};
//!
//! #[tokio::main]
//! async fn main() {
//!     let provider = GeminiProvider::from_env().expect("GEMINI_API_KEY not set");
//!     let request = ChatRequest::new(vec![
//!         ChatMessage::user("What is machine learning?"),
//!     ]);
//!     let response = provider.complete(&request).await.expect("API call failed");
//!     println!("{}", response.content);
//! }
//! ```

use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, instrument, warn};

use super::{
    ChatMessage, ChatRequest, ChatResponse, ChatStream, LlmCapabilities, LlmProvider, MessageRole,
    StreamChunk, TokenUsage,
};
use crate::errors::AppError;

/// Environment variable for Gemini API key
const GEMINI_API_KEY_ENV: &str = "GEMINI_API_KEY";

/// Default model to use
const DEFAULT_MODEL: &str = "gemini-2.0-flash-exp";

/// Available Gemini models
const AVAILABLE_MODELS: &[&str] = &[
    "gemini-2.0-flash-exp",
    "gemini-1.5-pro",
    "gemini-1.5-flash",
    "gemini-1.0-pro",
];

/// Base URL for the Gemini API
const API_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

// ============================================================================
// API Request/Response Types
// ============================================================================

/// Gemini API request structure
#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

/// Content structure for Gemini API
#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    parts: Vec<ContentPart>,
}

/// Part of content (text, image, etc.)
#[derive(Debug, Serialize, Deserialize)]
struct ContentPart {
    text: String,
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

/// Gemini API response structure
#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<Candidate>>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<UsageMetadata>,
    error: Option<GeminiError>,
}

/// Response candidate
#[derive(Debug, Deserialize)]
struct Candidate {
    content: Option<GeminiContent>,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

/// Usage metadata from Gemini API response
#[derive(Debug, Deserialize)]
struct UsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt: Option<u32>,
    #[serde(rename = "candidatesTokenCount")]
    candidates: Option<u32>,
    #[serde(rename = "totalTokenCount")]
    total: Option<u32>,
}

/// API error response from Gemini
#[derive(Debug, Deserialize)]
struct GeminiError {
    message: String,
}

/// Streaming response chunk
#[derive(Debug, Deserialize)]
struct StreamingResponse {
    candidates: Option<Vec<Candidate>>,
}

// ============================================================================
// Provider Implementation
// ============================================================================

/// Google Gemini LLM provider
pub struct GeminiProvider {
    api_key: String,
    client: Client,
    default_model: String,
}

impl GeminiProvider {
    /// Create a new Gemini provider with an API key
    #[must_use]
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            client: Client::new(),
            default_model: DEFAULT_MODEL.to_owned(),
        }
    }

    /// Create a provider from the `GEMINI_API_KEY` environment variable
    ///
    /// # Errors
    ///
    /// Returns an error if the environment variable is not set.
    pub fn from_env() -> Result<Self, AppError> {
        let api_key = std::env::var(GEMINI_API_KEY_ENV).map_err(|_| {
            AppError::config(format!("{GEMINI_API_KEY_ENV} environment variable not set"))
        })?;
        Ok(Self::new(api_key))
    }

    /// Set a custom default model
    #[must_use]
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// Convert our message role to Gemini's role format
    ///
    /// Note: System messages are handled separately via `system_instruction` field,
    /// but if one appears here, map it to "user" for compatibility.
    const fn convert_role(role: MessageRole) -> &'static str {
        match role {
            MessageRole::System | MessageRole::User => "user",
            MessageRole::Assistant => "model",
        }
    }

    /// Build the API URL for a model and method
    fn build_url(&self, model: &str, method: &str) -> String {
        format!(
            "{API_BASE_URL}/models/{model}:{method}?key={}",
            self.api_key
        )
    }

    /// Convert chat messages to Gemini format
    fn convert_messages(messages: &[ChatMessage]) -> (Vec<GeminiContent>, Option<GeminiContent>) {
        let mut contents = Vec::new();
        let mut system_instruction = None;

        for message in messages {
            if message.role == MessageRole::System {
                // Gemini uses separate system_instruction field
                system_instruction = Some(GeminiContent {
                    role: None,
                    parts: vec![ContentPart {
                        text: message.content.clone(),
                    }],
                });
            } else {
                contents.push(GeminiContent {
                    role: Some(Self::convert_role(message.role).to_owned()),
                    parts: vec![ContentPart {
                        text: message.content.clone(),
                    }],
                });
            }
        }

        (contents, system_instruction)
    }

    /// Build a Gemini API request from a `ChatRequest`
    fn build_gemini_request(request: &ChatRequest) -> GeminiRequest {
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

        GeminiRequest {
            contents,
            system_instruction,
            generation_config,
        }
    }

    /// Extract text content from Gemini response
    fn extract_content(response: &GeminiResponse) -> Result<String, AppError> {
        response
            .candidates
            .as_ref()
            .and_then(|c| c.first())
            .and_then(|c| c.content.as_ref())
            .and_then(|c| c.parts.first())
            .map(|p| p.text.clone())
            .ok_or_else(|| AppError::internal("No content in Gemini response"))
    }

    /// Convert usage metadata to our token usage format
    fn convert_usage(metadata: &UsageMetadata) -> TokenUsage {
        TokenUsage {
            prompt_tokens: metadata.prompt.unwrap_or(0),
            completion_tokens: metadata.candidates.unwrap_or(0),
            total_tokens: metadata.total.unwrap_or(0),
        }
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    fn name(&self) -> &'static str {
        "gemini"
    }

    fn display_name(&self) -> &'static str {
        "Google Gemini"
    }

    fn capabilities(&self) -> LlmCapabilities {
        LlmCapabilities::full_featured()
    }

    fn default_model(&self) -> &'static str {
        DEFAULT_MODEL
    }

    fn available_models(&self) -> &'static [&'static str] {
        AVAILABLE_MODELS
    }

    #[instrument(skip(self, request), fields(model = %request.model.as_deref().unwrap_or(DEFAULT_MODEL)))]
    async fn complete(&self, request: &ChatRequest) -> Result<ChatResponse, AppError> {
        let model = request.model.as_deref().unwrap_or(&self.default_model);
        let url = self.build_url(model, "generateContent");

        let gemini_request = Self::build_gemini_request(request);

        debug!("Sending request to Gemini API");

        let response = self
            .client
            .post(&url)
            .json(&gemini_request)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("HTTP request failed: {e}")))?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|e| AppError::internal(format!("Failed to read response: {e}")))?;

        if !status.is_success() {
            error!(status = %status, "Gemini API error");
            return Err(AppError::internal(format!(
                "Gemini API error ({status}): {response_text}"
            )));
        }

        let gemini_response: GeminiResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                error!(error = %e, response = %response_text, "Failed to parse response");
                AppError::internal(format!("Failed to parse Gemini response: {e}"))
            })?;

        if let Some(error) = gemini_response.error {
            return Err(AppError::internal(format!(
                "Gemini API error: {}",
                error.message
            )));
        }

        let content = Self::extract_content(&gemini_response)?;
        let usage = gemini_response
            .usage_metadata
            .as_ref()
            .map(Self::convert_usage);
        let finish_reason = gemini_response
            .candidates
            .as_ref()
            .and_then(|c| c.first())
            .and_then(|c| c.finish_reason.clone());

        debug!("Successfully received Gemini response");

        Ok(ChatResponse {
            content,
            model: model.to_owned(),
            usage,
            finish_reason,
        })
    }

    #[instrument(skip(self, request), fields(model = %request.model.as_deref().unwrap_or(DEFAULT_MODEL)))]
    async fn complete_stream(&self, request: &ChatRequest) -> Result<ChatStream, AppError> {
        let model = request.model.as_deref().unwrap_or(&self.default_model);
        let url = self.build_url(model, "streamGenerateContent");

        let gemini_request = Self::build_gemini_request(request);

        debug!("Starting streaming request to Gemini API");

        let response = self
            .client
            .post(&url)
            .query(&[("alt", "sse")])
            .json(&gemini_request)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("HTTP request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_owned());
            return Err(AppError::internal(format!(
                "Gemini API error ({status}): {error_text}"
            )));
        }

        // Create a stream from the SSE response
        let byte_stream = response.bytes_stream();

        let stream = byte_stream.filter_map(|result| async move {
            match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);

                    // Parse SSE format: lines starting with "data: "
                    for line in text.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data.trim().is_empty() {
                                continue;
                            }

                            match serde_json::from_str::<StreamingResponse>(data) {
                                Ok(response) => {
                                    if let Some(candidates) = response.candidates {
                                        if let Some(candidate) = candidates.first() {
                                            if let Some(content) = &candidate.content {
                                                if let Some(part) = content.parts.first() {
                                                    let is_final = candidate
                                                        .finish_reason
                                                        .as_ref()
                                                        .is_some_and(|r| r == "STOP");

                                                    return Some(Ok(StreamChunk {
                                                        delta: part.text.clone(),
                                                        is_final,
                                                        finish_reason: candidate
                                                            .finish_reason
                                                            .clone(),
                                                    }));
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!(error = %e, "Failed to parse streaming chunk");
                                }
                            }
                        }
                    }

                    None
                }
                Err(e) => Some(Err(AppError::internal(format!("Stream error: {e}")))),
            }
        });

        Ok(Box::pin(stream) as ChatStream)
    }

    #[instrument(skip(self))]
    async fn health_check(&self) -> Result<bool, AppError> {
        // Try to list models to verify the API key is valid
        let url = format!("{API_BASE_URL}/models?key={}", self.api_key);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("Health check failed: {e}")))?;

        Ok(response.status().is_success())
    }
}

impl std::fmt::Debug for GeminiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GeminiProvider")
            .field("default_model", &self.default_model)
            .field("api_key", &"[REDACTED]")
            // Omit `client` field as HTTP clients are not useful to debug
            .finish_non_exhaustive()
    }
}
