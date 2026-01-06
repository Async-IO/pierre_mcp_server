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
//! - `gemini-2.5-flash` (default): Latest fast model with improved capabilities
//! - `gemini-2.0-flash-exp`: Experimental fast model
//! - `gemini-1.5-pro`: Advanced reasoning capabilities
//! - `gemini-1.5-flash`: Balanced performance and cost
//!
//! ## Example
//!
//! ```rust,no_run
//! use pierre_mcp_server::llm::{GeminiProvider, LlmProvider, ChatRequest, ChatMessage};
//! use pierre_mcp_server::errors::AppError;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), AppError> {
//!     let provider = GeminiProvider::from_env()?;
//!     let request = ChatRequest::new(vec![
//!         ChatMessage::user("What is machine learning?"),
//!     ]);
//!     let response = provider.complete(&request).await?;
//!     println!("{}", response.content);
//!     Ok(())
//! }
//! ```

use std::env;
use std::fmt::{Debug, Formatter, Result as FmtResult};

use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, instrument, warn};

use super::{
    ChatMessage, ChatRequest, ChatResponse, ChatStream, LlmCapabilities, LlmProvider, MessageRole,
    StreamChunk, TokenUsage,
};
use crate::errors::{AppError, ErrorCode};

/// Environment variable for Gemini API key
const GEMINI_API_KEY_ENV: &str = "GEMINI_API_KEY";

/// Default model to use
const DEFAULT_MODEL: &str = "gemini-2.5-flash";

/// Available Gemini models
const AVAILABLE_MODELS: &[&str] = &[
    "gemini-2.5-flash",
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
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Tool>>,
}

/// Content structure for Gemini API
#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    parts: Vec<ContentPart>,
}

/// Part of content (text, function call, or function response)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContentPart {
    /// Text content
    Text { text: String },
    /// Function call from the model
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: FunctionCall,
    },
    /// Function response from the user
    FunctionResponse {
        #[serde(rename = "functionResponse")]
        function_response: FunctionResponse,
    },
}

/// Function call made by the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Name of the function to call
    pub name: String,
    /// Arguments for the function as JSON object
    pub args: serde_json::Value,
}

/// Response to a function call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionResponse {
    /// Name of the function that was called
    pub name: String,
    /// Response content from the function
    pub response: serde_json::Value,
}

/// Function declaration for tool definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    /// Name of the function
    pub name: String,
    /// Description of what the function does
    pub description: String,
    /// Parameters schema (JSON Schema format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}

/// Tool definition for Gemini API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Function declarations for this tool
    pub function_declarations: Vec<FunctionDeclaration>,
}

/// Response from a chat completion that may contain function calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponseWithTools {
    /// Generated message content (None if function calls present)
    pub content: Option<String>,
    /// Function calls requested by the model
    pub function_calls: Option<Vec<FunctionCall>>,
    /// Model used for generation
    pub model: String,
    /// Token usage statistics
    pub usage: Option<super::TokenUsage>,
    /// Finish reason (stop, length, etc.)
    pub finish_reason: Option<String>,
}

impl ChatResponseWithTools {
    /// Check if this response contains function calls
    #[must_use]
    pub fn has_function_calls(&self) -> bool {
        self.function_calls
            .as_ref()
            .is_some_and(|calls| !calls.is_empty())
    }

    /// Get the text content if present
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        self.content.as_deref()
    }
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
        let api_key = env::var(GEMINI_API_KEY_ENV).map_err(|_| {
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

    /// Complete a chat request with function calling support
    ///
    /// This method allows passing tool definitions to Gemini, enabling the model
    /// to respond with function calls that should be executed.
    ///
    /// # Arguments
    ///
    /// * `request` - The chat request with messages
    /// * `tools` - Optional tool definitions for function calling
    ///
    /// # Returns
    ///
    /// Returns a `ChatResponseWithTools` which may contain either text content
    /// or function calls to execute.
    ///
    /// # Errors
    ///
    /// Returns `AppError` if the HTTP request fails or if the API returns an error response.
    #[instrument(skip(self, request, tools), fields(model = %request.model.as_deref().unwrap_or(DEFAULT_MODEL)))]
    pub async fn complete_with_tools(
        &self,
        request: &ChatRequest,
        tools: Option<Vec<Tool>>,
    ) -> Result<ChatResponseWithTools, AppError> {
        let model = request.model.as_deref().unwrap_or(&self.default_model);
        let url = self.build_url(model, "generateContent");

        let gemini_request = Self::build_gemini_request(request, tools);

        debug!("Sending request with tools to Gemini API");

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
            return Err(Self::map_api_error(status.as_u16(), &response_text));
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

        // Check for function calls first
        let function_calls = Self::extract_function_calls(&gemini_response);
        if !function_calls.is_empty() {
            debug!(
                count = function_calls.len(),
                "Extracted function calls from response"
            );
            return Ok(ChatResponseWithTools {
                content: None,
                function_calls: Some(function_calls),
                model: model.to_owned(),
                usage: gemini_response
                    .usage_metadata
                    .as_ref()
                    .map(Self::convert_usage),
                finish_reason: gemini_response
                    .candidates
                    .as_ref()
                    .and_then(|c| c.first())
                    .and_then(|c| c.finish_reason.clone()),
            });
        }

        // Otherwise extract text content
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

        debug!("Successfully received text response from Gemini");

        Ok(ChatResponseWithTools {
            content: Some(content),
            function_calls: None,
            model: model.to_owned(),
            usage,
            finish_reason,
        })
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
                    parts: vec![ContentPart::Text {
                        text: message.content.clone(),
                    }],
                });
            } else {
                contents.push(GeminiContent {
                    role: Some(Self::convert_role(message.role).to_owned()),
                    parts: vec![ContentPart::Text {
                        text: message.content.clone(),
                    }],
                });
            }
        }

        (contents, system_instruction)
    }

    /// Build a Gemini API request from a `ChatRequest`
    fn build_gemini_request(request: &ChatRequest, tools: Option<Vec<Tool>>) -> GeminiRequest {
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
            tools,
        }
    }

    /// Extract text content from Gemini response
    fn extract_content(response: &GeminiResponse) -> Result<String, AppError> {
        let part = response
            .candidates
            .as_ref()
            .and_then(|c| c.first())
            .and_then(|c| c.content.as_ref())
            .and_then(|c| c.parts.first())
            .ok_or_else(|| AppError::internal("No content in Gemini response"))?;

        match part {
            ContentPart::Text { text } => Ok(text.clone()),
            ContentPart::FunctionCall { function_call } => {
                // If the model wants to call a function, return a JSON representation
                // The caller should check for function calls using extract_function_calls
                Ok(format!(
                    "{{\"function_call\": {{\"name\": \"{}\", \"args\": {}}}}}",
                    function_call.name, function_call.args
                ))
            }
            ContentPart::FunctionResponse { .. } => Err(AppError::internal(
                "Unexpected function response in model output",
            )),
        }
    }

    /// Extract function calls from Gemini response if present
    fn extract_function_calls(response: &GeminiResponse) -> Vec<FunctionCall> {
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
                            Some(function_call.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Convert usage metadata to our token usage format
    fn convert_usage(metadata: &UsageMetadata) -> TokenUsage {
        TokenUsage {
            prompt_tokens: metadata.prompt.unwrap_or(0),
            completion_tokens: metadata.candidates.unwrap_or(0),
            total_tokens: metadata.total.unwrap_or(0),
        }
    }

    /// Map API error status to appropriate error type
    ///
    /// For rate limit (429) and quota errors, returns a user-friendly error
    /// that exposes the actual message from Gemini.
    fn map_api_error(status: u16, response_text: &str) -> AppError {
        // Try to extract error message from JSON response
        let message = serde_json::from_str::<GeminiResponse>(response_text)
            .ok()
            .and_then(|r| r.error)
            .map_or_else(|| response_text.to_owned(), |e| e.message);

        match status {
            429 => {
                // Extract user-friendly quota message
                let user_message = Self::extract_quota_message(&message);
                AppError::new(ErrorCode::ExternalRateLimited, user_message)
            }
            _ => AppError::internal(format!("Gemini API error ({status}): {message}")),
        }
    }

    /// Extract a user-friendly quota/rate limit message from Gemini error
    fn extract_quota_message(message: &str) -> String {
        // Look for "Please retry in X" and extract the time value
        // Example: "Please retry in 6.406453963s."
        if let Some(retry_pos) = message.find("Please retry in ") {
            let after_prefix = &message[retry_pos + 16..]; // Skip "Please retry in "
                                                           // Find the 's' that ends the seconds value (e.g., "6.406453963s")
            if let Some(s_pos) = after_prefix.find('s') {
                let time_str = &after_prefix[..s_pos];
                if let Ok(seconds) = time_str.parse::<f64>() {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let seconds_int = seconds.ceil() as u64;
                    return format!(
                        "AI service quota exceeded. Please try again in {seconds_int} seconds."
                    );
                }
            }
        }
        // Fallback to a generic but informative message
        "AI service quota exceeded. Please wait a moment and try again.".to_owned()
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

        let gemini_request = Self::build_gemini_request(request, None);

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
            return Err(Self::map_api_error(status.as_u16(), &response_text));
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

        let gemini_request = Self::build_gemini_request(request, None);

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
            return Err(Self::map_api_error(status.as_u16(), &error_text));
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

                                                    // Extract text from ContentPart enum
                                                    let delta = match part {
                                                        ContentPart::Text { text } => text.clone(),
                                                        ContentPart::FunctionCall { function_call } => {
                                                            // Serialize function call for streaming
                                                            format!(
                                                                "{{\"function_call\": {{\"name\": \"{}\", \"args\": {}}}}}",
                                                                function_call.name,
                                                                function_call.args
                                                            )
                                                        }
                                                        ContentPart::FunctionResponse { .. } => {
                                                            continue;
                                                        }
                                                    };

                                                    return Some(Ok(StreamChunk {
                                                        delta,
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

impl Debug for GeminiProvider {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("GeminiProvider")
            .field("default_model", &self.default_model)
            .field("api_key", &"[REDACTED]")
            // Omit `client` field as HTTP clients are not useful to debug
            .finish_non_exhaustive()
    }
}
