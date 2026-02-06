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
//! - `GEMINI_API_KEY`: Required API key from Google AI Studio: <https://makersuite.google.com/app/apikey>
//! - `PIERRE_LLM_DEFAULT_MODEL`: Default model to use (via `LlmModelConfig`)
//! - `PIERRE_LLM_FALLBACK_MODEL`: Fallback model when default fails (via `LlmModelConfig`)
//!
//! ## Supported Models
//!
//! - `gemini-3-flash-preview` (default): Latest flash model with improved capabilities
//! - `gemini-2.5-flash`: Fast model with improved capabilities
//! - `gemini-2.0-flash`: Stable multimodal model
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
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::{debug, error, info, instrument, warn};

use super::sse_parser;
use super::{
    ChatMessage, ChatRequest, ChatResponse, ChatStream, LlmCapabilities, LlmProvider, MessageRole,
    StreamChunk, TokenUsage,
};
use crate::config::LlmModelConfig;
use crate::errors::{AppError, ErrorCode};

/// Environment variable for Gemini API key
const GEMINI_API_KEY_ENV: &str = "GEMINI_API_KEY";

/// Environment variable for maximum retries
const GEMINI_MAX_RETRIES_ENV: &str = "GEMINI_MAX_RETRIES";

/// Environment variable for initial retry delay in milliseconds
const GEMINI_INITIAL_RETRY_DELAY_MS_ENV: &str = "GEMINI_INITIAL_RETRY_DELAY_MS";

/// Environment variable for maximum retry delay in milliseconds
const GEMINI_MAX_RETRY_DELAY_MS_ENV: &str = "GEMINI_MAX_RETRY_DELAY_MS";

/// Available Gemini models
const AVAILABLE_MODELS: &[&str] = &[
    "gemini-3-flash-preview",
    "gemini-2.5-flash",
    "gemini-2.0-flash",
    "gemini-1.5-pro",
    "gemini-1.5-flash",
];

/// Base URL for the Gemini API
const API_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Default maximum number of retries for transient failures
const DEFAULT_MAX_RETRIES: u32 = 3;

/// Default initial delay for exponential backoff (in milliseconds)
const DEFAULT_INITIAL_RETRY_DELAY_MS: u64 = 500;

/// Default maximum delay cap for retries (in milliseconds)
const DEFAULT_MAX_RETRY_DELAY_MS: u64 = 5000;

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
    /// Content parts - may be empty for thinking-only responses from models like gemini-3-flash-preview
    #[serde(default)]
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
    /// Fallback model when default fails (rate limits, errors)
    fallback_model: String,
    /// Maximum number of retries for transient failures
    max_retries: u32,
    /// Initial delay for exponential backoff (in milliseconds)
    initial_retry_delay_ms: u64,
    /// Maximum delay cap for retries (in milliseconds)
    max_retry_delay_ms: u64,
}

impl GeminiProvider {
    /// Create a new Gemini provider with an API key
    ///
    /// Loads model configuration from `LlmModelConfig` environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error if `PIERRE_LLM_DEFAULT_MODEL` or `PIERRE_LLM_FALLBACK_MODEL` are not set.
    pub fn new(api_key: impl Into<String>) -> Result<Self, AppError> {
        let model_config = LlmModelConfig::from_env().map_err(AppError::config)?;
        Ok(Self::with_config(api_key, &model_config))
    }

    /// Create a new Gemini provider with an API key and explicit model config
    #[must_use]
    pub fn with_config(api_key: impl Into<String>, model_config: &LlmModelConfig) -> Self {
        Self {
            api_key: api_key.into(),
            client: Client::new(),
            default_model: model_config.default_model.clone(),
            fallback_model: model_config.fallback_model.clone(),
            max_retries: DEFAULT_MAX_RETRIES,
            initial_retry_delay_ms: DEFAULT_INITIAL_RETRY_DELAY_MS,
            max_retry_delay_ms: DEFAULT_MAX_RETRY_DELAY_MS,
        }
    }

    /// Create a provider from environment variables
    ///
    /// - `GEMINI_API_KEY`: Required API key
    /// - `PIERRE_LLM_DEFAULT_MODEL`: Default model to use
    /// - `PIERRE_LLM_FALLBACK_MODEL`: Fallback model when default fails
    /// - `GEMINI_MAX_RETRIES`: Optional max retries (default: 3)
    /// - `GEMINI_INITIAL_RETRY_DELAY_MS`: Optional initial retry delay in ms (default: 500)
    /// - `GEMINI_MAX_RETRY_DELAY_MS`: Optional max retry delay in ms (default: 5000)
    ///
    /// # Errors
    ///
    /// Returns an error if required environment variables are not set.
    pub fn from_env() -> Result<Self, AppError> {
        let api_key = env::var(GEMINI_API_KEY_ENV).map_err(|_| {
            AppError::config(format!("{GEMINI_API_KEY_ENV} environment variable not set"))
        })?;

        // Load model config from environment
        let model_config = LlmModelConfig::from_env().map_err(AppError::config)?;

        let max_retries = env::var(GEMINI_MAX_RETRIES_ENV)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_MAX_RETRIES);

        let initial_retry_delay_ms = env::var(GEMINI_INITIAL_RETRY_DELAY_MS_ENV)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_INITIAL_RETRY_DELAY_MS);

        let max_retry_delay_ms = env::var(GEMINI_MAX_RETRY_DELAY_MS_ENV)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_MAX_RETRY_DELAY_MS);

        info!(
            default_model = %model_config.default_model,
            fallback_model = %model_config.fallback_model,
            max_retries = max_retries,
            initial_retry_delay_ms = initial_retry_delay_ms,
            max_retry_delay_ms = max_retry_delay_ms,
            "Gemini provider initialized"
        );

        Ok(Self::with_config(api_key, &model_config).with_retry_config(
            max_retries,
            initial_retry_delay_ms,
            max_retry_delay_ms,
        ))
    }

    /// Set a custom default model
    #[must_use]
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// Set a custom fallback model
    #[must_use]
    pub fn with_fallback_model(mut self, model: impl Into<String>) -> Self {
        self.fallback_model = model.into();
        self
    }

    /// Configure retry behavior
    #[must_use]
    pub const fn with_retry_config(
        mut self,
        max_retries: u32,
        initial_retry_delay_ms: u64,
        max_retry_delay_ms: u64,
    ) -> Self {
        self.max_retries = max_retries;
        self.initial_retry_delay_ms = initial_retry_delay_ms;
        self.max_retry_delay_ms = max_retry_delay_ms;
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
    #[instrument(skip(self, request, tools), fields(model = %request.model.as_deref().unwrap_or("default")))]
    pub async fn complete_with_tools(
        &self,
        request: &ChatRequest,
        tools: Option<Vec<Tool>>,
    ) -> Result<ChatResponseWithTools, AppError> {
        let model = request.model.as_deref().unwrap_or(&self.default_model);
        let url = self.build_url(model, "generateContent");
        let gemini_request = Self::build_gemini_request(request, tools);

        let mut last_error: Option<AppError> = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let delay = self.calculate_retry_delay(attempt - 1);
                warn!(
                    attempt,
                    delay_ms = delay.as_millis(),
                    "Retrying Gemini request with tools"
                );
                sleep(delay).await;
            }

            debug!(attempt, "Sending request with tools to Gemini API");

            match self
                .execute_tools_request(&url, &gemini_request, model, attempt)
                .await
            {
                Ok(response) => return Ok(response),
                Err(e) => {
                    if Self::is_retryable_error(&e) && attempt < self.max_retries {
                        warn!(attempt, error = %e, "Retryable error, will retry");
                        last_error = Some(e);
                        continue;
                    }
                    return Err(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            AppError::internal("Gemini request with tools failed after retries")
        }))
    }

    /// Execute a single tools request and process the response
    async fn execute_tools_request(
        &self,
        url: &str,
        gemini_request: &GeminiRequest,
        model: &str,
        attempt: u32,
    ) -> Result<ChatResponseWithTools, AppError> {
        let response = self
            .client
            .post(url)
            .json(gemini_request)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("HTTP request failed: {e}")))?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|e| AppError::internal(format!("Failed to read response: {e}")))?;

        if !status.is_success() {
            return Err(Self::map_api_error(status.as_u16(), &response_text));
        }

        let gemini_response: GeminiResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                error!(error = %e, response_len = response_text.len(), "Failed to parse Gemini response (body redacted)");
                AppError::internal(format!("Failed to parse Gemini response: {e}"))
            })?;

        if let Some(error) = gemini_response.error {
            return Err(AppError::internal(format!(
                "Gemini API error: {}",
                error.message
            )));
        }

        Self::process_tools_response(&gemini_response, model, attempt)
    }

    /// Process a Gemini response into a `ChatResponseWithTools`
    fn process_tools_response(
        response: &GeminiResponse,
        model: &str,
        attempt: u32,
    ) -> Result<ChatResponseWithTools, AppError> {
        let function_calls = Self::extract_function_calls(response);
        let usage = response.usage_metadata.as_ref().map(Self::convert_usage);
        let finish_reason = response
            .candidates
            .as_ref()
            .and_then(|c| c.first())
            .and_then(|c| c.finish_reason.clone());

        if !function_calls.is_empty() {
            debug!(
                count = function_calls.len(),
                attempt, "Extracted function calls"
            );
            return Ok(ChatResponseWithTools {
                content: None,
                function_calls: Some(function_calls),
                model: model.to_owned(),
                usage,
                finish_reason,
            });
        }

        let content = Self::extract_content(response)?;
        debug!(attempt, "Successfully received text response from Gemini");

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
        // Get candidate content, handling case where parts may be empty (thinking models)
        let content = response
            .candidates
            .as_ref()
            .and_then(|c| c.first())
            .and_then(|c| c.content.as_ref());

        // Handle empty or missing content - can happen with thinking models like gemini-3-flash-preview
        let Some(content) = content else {
            // Check if this is a blocked response
            let finish_reason = response
                .candidates
                .as_ref()
                .and_then(|c| c.first())
                .and_then(|c| c.finish_reason.as_ref());

            if let Some(reason) = finish_reason {
                if reason == "SAFETY" || reason == "RECITATION" || reason == "OTHER" {
                    return Err(AppError::internal(format!(
                        "Response blocked by Gemini safety filter: {reason}"
                    )));
                }
            }
            return Err(AppError::internal(
                "No content in Gemini response - model may still be thinking",
            ));
        };

        // Handle empty parts - thinking models may return content with no parts
        let Some(part) = content.parts.first() else {
            return Err(AppError::internal(
                "Gemini response has no content parts - model may have returned thinking-only output",
            ));
        };

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

    /// Check if an error is retryable
    fn is_retryable_error(error: &AppError) -> bool {
        let message = error.to_string();
        // Retry on 429 rate limit - backoff will help
        if message.contains("429")
            || message.contains("quota exceeded")
            || message.contains("rate limit")
        {
            return true;
        }
        // Retry on 503 overloaded
        if message.contains("503") || message.contains("overloaded") {
            return true;
        }
        // Retry on thinking-only output (model may succeed on retry)
        if message.contains("no content parts") || message.contains("thinking-only") {
            return true;
        }
        // Retry on model still thinking
        if message.contains("still be thinking") {
            return true;
        }
        false
    }

    /// Calculate delay for exponential backoff with jitter
    fn calculate_retry_delay(&self, attempt: u32) -> Duration {
        // Exponential backoff: delay = initial * 2^attempt
        let base_delay = self.initial_retry_delay_ms.saturating_mul(1 << attempt);
        let capped_delay = base_delay.min(self.max_retry_delay_ms);
        // Add small jitter (0-100ms) to avoid thundering herd
        let jitter = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| u64::from(d.subsec_millis()))
            % 100;
        Duration::from_millis(capped_delay + jitter)
    }

    /// Parse a Gemini SSE data payload into a `StreamChunk`
    ///
    /// Gemini's streaming response uses a different JSON structure than
    /// OpenAI-compatible providers, requiring provider-specific parsing.
    fn parse_stream_data(json_str: &str) -> Option<Result<StreamChunk, AppError>> {
        match serde_json::from_str::<StreamingResponse>(json_str) {
            Ok(response) => {
                let candidate = response.candidates?.into_iter().next()?;
                let content = candidate.content?;
                let part = content.parts.first()?;

                let is_final = candidate
                    .finish_reason
                    .as_ref()
                    .is_some_and(|r| r == "STOP");

                let delta = match part {
                    ContentPart::Text { text } => text.clone(),
                    ContentPart::FunctionCall { function_call } => {
                        format!(
                            "{{\"function_call\": {{\"name\": \"{}\", \"args\": {}}}}}",
                            function_call.name, function_call.args
                        )
                    }
                    ContentPart::FunctionResponse { .. } => return None,
                };

                Some(Ok(StreamChunk {
                    delta,
                    is_final,
                    finish_reason: candidate.finish_reason,
                }))
            }
            Err(e) => {
                warn!(error = %e, "Failed to parse Gemini streaming chunk");
                None
            }
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

    fn default_model(&self) -> &str {
        &self.default_model
    }

    fn available_models(&self) -> &'static [&'static str] {
        AVAILABLE_MODELS
    }

    #[instrument(skip(self, request), fields(model = %request.model.as_deref().unwrap_or("default")))]
    async fn complete(&self, request: &ChatRequest) -> Result<ChatResponse, AppError> {
        let model = request.model.as_deref().unwrap_or(&self.default_model);
        let url = self.build_url(model, "generateContent");
        let gemini_request = Self::build_gemini_request(request, None);

        let mut last_error: Option<AppError> = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let delay = self.calculate_retry_delay(attempt - 1);
                warn!(
                    attempt = attempt,
                    delay_ms = delay.as_millis(),
                    "Retrying Gemini request after transient failure"
                );
                sleep(delay).await;
            }

            debug!(attempt = attempt, "Sending request to Gemini API");

            let response = match self.client.post(&url).json(&gemini_request).send().await {
                Ok(r) => r,
                Err(e) => {
                    let error = AppError::internal(format!("HTTP request failed: {e}"));
                    if Self::is_retryable_error(&error) && attempt < self.max_retries {
                        last_error = Some(error);
                        continue;
                    }
                    return Err(error);
                }
            };

            let status = response.status();
            let response_text = match response.text().await {
                Ok(t) => t,
                Err(e) => {
                    let error = AppError::internal(format!("Failed to read response: {e}"));
                    if attempt < self.max_retries {
                        last_error = Some(error);
                        continue;
                    }
                    return Err(error);
                }
            };

            if !status.is_success() {
                let error = Self::map_api_error(status.as_u16(), &response_text);
                if Self::is_retryable_error(&error) && attempt < self.max_retries {
                    warn!(status = %status, attempt = attempt, "Gemini API returned retryable error");
                    last_error = Some(error);
                    continue;
                }
                error!(status = %status, "Gemini API error (not retryable)");
                return Err(error);
            }

            let gemini_response: GeminiResponse = match serde_json::from_str(&response_text) {
                Ok(r) => r,
                Err(e) => {
                    error!(error = %e, response_len = response_text.len(), "Failed to parse Gemini response (body redacted)");
                    return Err(AppError::internal(format!(
                        "Failed to parse Gemini response: {e}"
                    )));
                }
            };

            if let Some(error) = gemini_response.error {
                let app_error = AppError::internal(format!("Gemini API error: {}", error.message));
                if Self::is_retryable_error(&app_error) && attempt < self.max_retries {
                    last_error = Some(app_error);
                    continue;
                }
                return Err(app_error);
            }

            // Try to extract content - may fail on thinking-only responses
            match Self::extract_content(&gemini_response) {
                Ok(content) => {
                    let usage = gemini_response
                        .usage_metadata
                        .as_ref()
                        .map(Self::convert_usage);
                    let finish_reason = gemini_response
                        .candidates
                        .as_ref()
                        .and_then(|c| c.first())
                        .and_then(|c| c.finish_reason.clone());

                    debug!(attempt = attempt, "Successfully received Gemini response");

                    return Ok(ChatResponse {
                        content,
                        model: model.to_owned(),
                        usage,
                        finish_reason,
                    });
                }
                Err(e) => {
                    // Retry on thinking-only output
                    if Self::is_retryable_error(&e) && attempt < self.max_retries {
                        warn!(
                            attempt = attempt,
                            error = %e,
                            "Gemini returned empty content, retrying"
                        );
                        last_error = Some(e);
                        continue;
                    }
                    return Err(e);
                }
            }
        }

        // Should not reach here, but return last error if we do
        Err(last_error.unwrap_or_else(|| AppError::internal("Gemini request failed after retries")))
    }

    #[instrument(skip(self, request), fields(model = %request.model.as_deref().unwrap_or("default")))]
    async fn complete_stream(&self, request: &ChatRequest) -> Result<ChatStream, AppError> {
        let model = request.model.as_deref().unwrap_or(&self.default_model);
        let url = self.build_url(model, "streamGenerateContent");
        let gemini_request = Self::build_gemini_request(request, None);

        // Retry the initial HTTP request (consistent with non-streaming complete())
        let mut last_error: Option<AppError> = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let delay = self.calculate_retry_delay(attempt - 1);
                warn!(
                    attempt = attempt,
                    delay_ms = delay.as_millis(),
                    "Retrying Gemini streaming request after transient failure"
                );
                sleep(delay).await;
            }

            debug!(
                attempt = attempt,
                "Starting streaming request to Gemini API"
            );

            let response = match self
                .client
                .post(&url)
                .query(&[("alt", "sse")])
                .json(&gemini_request)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    let error = AppError::internal(format!("HTTP request failed: {e}"));
                    if Self::is_retryable_error(&error) && attempt < self.max_retries {
                        last_error = Some(error);
                        continue;
                    }
                    return Err(error);
                }
            };

            let status = response.status();
            if !status.is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_owned());
                let error = Self::map_api_error(status.as_u16(), &error_text);
                if Self::is_retryable_error(&error) && attempt < self.max_retries {
                    warn!(status = %status, attempt = attempt, "Gemini streaming API returned retryable error");
                    last_error = Some(error);
                    continue;
                }
                return Err(error);
            }

            // Connection succeeded — use shared SSE parser
            // Gemini does not send [DONE] signal; stream ends with HTTP response
            let stream = sse_parser::create_sse_stream(
                response.bytes_stream(),
                Self::parse_stream_data,
                "Gemini",
            );
            return Ok(stream);
        }

        Err(last_error
            .unwrap_or_else(|| AppError::internal("Gemini streaming request failed after retries")))
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
