// ABOUTME: Groq LLM provider implementation with streaming support
// ABOUTME: Uses OpenAI-compatible API for Llama, Mixtral models via Groq's fast LPU inference
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Groq Provider
//!
//! Implementation of the `LlmProvider` trait for Groq's LPU-accelerated inference.
//!
//! ## Features
//!
//! - **Tool/Function Calling**: Full support via OpenAI-compatible tool calling API
//! - **Streaming**: Real-time response streaming for better UX
//! - **Fast Inference**: Groq's LPU provides low-latency responses
//!
//! ## Configuration
//!
//! Set the `GROQ_API_KEY` environment variable with your API key from
//! Groq Console: <https://console.groq.com/keys>
//!
//! ## Rate Limits
//!
//! The free tier has a 12,000 tokens-per-minute (TPM) limit. For tool-heavy
//! workflows with multiple iterations, consider using Gemini which has more
//! generous rate limits. Set `PIERRE_LLM_PROVIDER=gemini` to switch.
//!
//! ## Supported Models
//!
//! - `llama-3.3-70b-versatile` (default): High-quality general purpose
//! - `llama-3.1-8b-instant`: Fast responses for simple tasks
//! - `mixtral-8x7b-32768`: Long context window (32K tokens)
//!
//! ## Example
//!
//! ```rust,no_run
//! use pierre_mcp_server::llm::{GroqProvider, LlmProvider, ChatRequest, ChatMessage};
//! use pierre_mcp_server::errors::AppError;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), AppError> {
//!     let provider = GroqProvider::from_env()?;
//!     let request = ChatRequest::new(vec![
//!         ChatMessage::user("What is machine learning?"),
//!     ]);
//!     let response = provider.complete(&request).await?;
//!     println!("{}", response.content);
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use tokio::time::sleep;
use tracing::{debug, error, info, instrument, warn};

use super::sse_parser::{self, RetryConfig};
use super::{
    ChatMessage, ChatRequest, ChatResponse, ChatResponseWithTools, ChatStream, FunctionCall,
    LlmCapabilities, LlmProvider, StreamChunk, TokenUsage, Tool,
};
use crate::errors::{AppError, ErrorCode};

/// Environment variable for Groq API key
const GROQ_API_KEY_ENV: &str = "GROQ_API_KEY";

/// Environment variable for default model
const GROQ_DEFAULT_MODEL_ENV: &str = "GROQ_DEFAULT_MODEL";

/// Environment variable for fallback model
const GROQ_FALLBACK_MODEL_ENV: &str = "GROQ_FALLBACK_MODEL";

/// Hardcoded fallback if env vars not set
const HARDCODED_DEFAULT_MODEL: &str = "llama-3.3-70b-versatile";

/// Available Groq models
const AVAILABLE_MODELS: &[&str] = &[
    "llama-3.3-70b-versatile",
    "llama-3.1-8b-instant",
    "llama-3.1-70b-versatile",
    "mixtral-8x7b-32768",
    "gemma2-9b-it",
];

/// Base URL for the Groq API (OpenAI-compatible)
const API_BASE_URL: &str = "https://api.groq.com/openai/v1";

/// Environment variable for maximum retries
const GROQ_MAX_RETRIES_ENV: &str = "GROQ_MAX_RETRIES";

/// Environment variable for initial retry delay in milliseconds
const GROQ_INITIAL_RETRY_DELAY_MS_ENV: &str = "GROQ_INITIAL_RETRY_DELAY_MS";

/// Environment variable for maximum retry delay in milliseconds
const GROQ_MAX_RETRY_DELAY_MS_ENV: &str = "GROQ_MAX_RETRY_DELAY_MS";

// ============================================================================
// API Request/Response Types (OpenAI-compatible format)
// ============================================================================

/// Groq API request structure (OpenAI-compatible)
#[derive(Debug, Serialize)]
struct GroqRequest {
    model: String,
    messages: Vec<GroqMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<GroqTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
}

/// Tool definition for Groq API (OpenAI-compatible format)
#[derive(Debug, Clone, Serialize)]
struct GroqTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: GroqFunction,
}

/// Function definition within a tool
#[derive(Debug, Clone, Serialize)]
struct GroqFunction {
    name: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parameters: Option<Value>,
}

/// Message structure for Groq API (OpenAI-compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GroqMessage {
    role: String,
    content: String,
}

impl From<&ChatMessage> for GroqMessage {
    fn from(msg: &ChatMessage) -> Self {
        Self {
            role: msg.role.as_str().to_owned(),
            content: msg.content.clone(),
        }
    }
}

/// Groq API response structure (OpenAI-compatible)
#[derive(Debug, Deserialize)]
struct GroqResponse {
    choices: Vec<GroqChoice>,
    #[serde(default)]
    usage: Option<GroqUsage>,
    model: String,
}

/// Choice in Groq response
#[derive(Debug, Deserialize)]
struct GroqChoice {
    message: GroqResponseMessage,
    finish_reason: Option<String>,
}

/// Message in Groq response
#[derive(Debug, Deserialize)]
struct GroqResponseMessage {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<GroqToolCall>>,
}

/// Tool call in Groq response (OpenAI-compatible)
#[derive(Debug, Clone, Deserialize)]
struct GroqToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: GroqFunctionCall,
}

/// Function call details in Groq response
#[derive(Debug, Clone, Deserialize)]
struct GroqFunctionCall {
    name: String,
    arguments: String,
}

/// Usage statistics in Groq response
#[derive(Debug, Deserialize)]
struct GroqUsage {
    /// Tokens used in the prompt
    #[serde(rename = "prompt_tokens")]
    prompt: u32,
    /// Tokens generated in completion
    #[serde(rename = "completion_tokens")]
    completion: u32,
    /// Total tokens used
    #[serde(rename = "total_tokens")]
    total: u32,
}

/// Streaming chunk structure (OpenAI-compatible)
#[derive(Debug, Deserialize)]
struct GroqStreamChunk {
    choices: Vec<GroqStreamChoice>,
}

/// Choice in streaming chunk
#[derive(Debug, Deserialize)]
struct GroqStreamChoice {
    delta: GroqDelta,
    finish_reason: Option<String>,
}

/// Delta content in streaming chunk
#[derive(Debug, Deserialize)]
struct GroqDelta {
    #[serde(default)]
    content: Option<String>,
}

/// Groq API error response
#[derive(Debug, Deserialize)]
struct GroqErrorResponse {
    error: GroqErrorDetail,
}

/// Error detail structure
#[derive(Debug, Deserialize)]
struct GroqErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: Option<String>,
}

// ============================================================================
// Provider Implementation
// ============================================================================

/// Groq LLM provider using LPU-accelerated inference
///
/// Provides access to open-source models (Llama, Mixtral) with
/// extremely fast inference speeds via Groq's Language Processing Units.
pub struct GroqProvider {
    client: Client,
    api_key: String,
    default_model: String,
    fallback_model: String,
    /// Retry configuration for transient failures (429, 503, network errors)
    retry_config: RetryConfig,
}

impl GroqProvider {
    /// Create a new Groq provider with the given API key
    ///
    /// Uses environment variables for model configuration:
    /// - `GROQ_DEFAULT_MODEL`: Primary model (default: llama-3.3-70b-versatile)
    /// - `GROQ_FALLBACK_MODEL`: Fallback model (default: llama-3.3-70b-versatile)
    #[must_use]
    pub fn new(api_key: String) -> Self {
        let default_model =
            env::var(GROQ_DEFAULT_MODEL_ENV).unwrap_or_else(|_| HARDCODED_DEFAULT_MODEL.to_owned());
        let fallback_model = env::var(GROQ_FALLBACK_MODEL_ENV)
            .unwrap_or_else(|_| HARDCODED_DEFAULT_MODEL.to_owned());

        let max_retries = env::var(GROQ_MAX_RETRIES_ENV)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(RetryConfig::default_config().max_retries);
        let initial_delay_ms = env::var(GROQ_INITIAL_RETRY_DELAY_MS_ENV)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(RetryConfig::default_config().initial_delay_ms);
        let max_delay_ms = env::var(GROQ_MAX_RETRY_DELAY_MS_ENV)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(RetryConfig::default_config().max_delay_ms);

        Self {
            client: Client::new(),
            api_key,
            default_model,
            fallback_model,
            retry_config: RetryConfig {
                max_retries,
                initial_delay_ms,
                max_delay_ms,
            },
        }
    }

    /// Create a Groq provider from environment variable
    ///
    /// # Errors
    ///
    /// Returns an error if `GROQ_API_KEY` is not set
    pub fn from_env() -> Result<Self, AppError> {
        let api_key = env::var(GROQ_API_KEY_ENV).map_err(|_| {
            AppError::config(format!(
                "Missing {GROQ_API_KEY_ENV} environment variable. Get your API key from https://console.groq.com/keys"
            ))
        })?;

        let provider = Self::new(api_key);
        info!(
            default_model = %provider.default_model,
            fallback_model = %provider.fallback_model,
            max_retries = provider.retry_config.max_retries,
            initial_delay_ms = provider.retry_config.initial_delay_ms,
            "Groq provider initialized"
        );

        Ok(provider)
    }

    /// Get the default model
    #[must_use]
    pub fn default_model(&self) -> &str {
        &self.default_model
    }

    /// Get the fallback model
    #[must_use]
    pub fn fallback_model(&self) -> &str {
        &self.fallback_model
    }

    /// Build the API URL for a given endpoint
    fn api_url(endpoint: &str) -> String {
        format!("{API_BASE_URL}/{endpoint}")
    }

    /// Convert internal messages to Groq format
    fn convert_messages(messages: &[ChatMessage]) -> Vec<GroqMessage> {
        messages.iter().map(GroqMessage::from).collect()
    }

    /// Parse error response from Groq API
    fn parse_error_response(status: reqwest::StatusCode, body: &str) -> AppError {
        if let Ok(error_response) = serde_json::from_str::<GroqErrorResponse>(body) {
            let error_type = error_response
                .error
                .error_type
                .unwrap_or_else(|| "unknown".to_owned());

            match status.as_u16() {
                401 => AppError::auth_invalid(format!(
                    "Groq API authentication failed: {}",
                    error_response.error.message
                )),
                429 => {
                    // Extract user-friendly rate limit message
                    let user_message =
                        Self::extract_rate_limit_message(&error_response.error.message);
                    AppError::new(ErrorCode::ExternalRateLimited, user_message)
                }
                400 => AppError::invalid_input(format!(
                    "Groq API validation error: {}",
                    error_response.error.message
                )),
                _ => AppError::external_service(
                    "Groq",
                    format!("{} - {}", error_type, error_response.error.message),
                ),
            }
        } else {
            debug!(
                status = %status,
                body_preview = %body.chars().take(200).collect::<String>(),
                "Groq API returned non-JSON error response"
            );
            AppError::external_service("Groq", format!("unexpected error response (HTTP {status})"))
        }
    }

    /// Extract a user-friendly rate limit message from Groq error
    ///
    /// Groq rate limit errors look like:
    /// "Rate limit reached for model X in organization Y on tokens per minute (TPM): Limit N, Used M, Requested R. Please try again in Xs."
    fn extract_rate_limit_message(message: &str) -> String {
        // Try to extract "Please try again in Xs" wait time
        if let Some(retry_pos) = message.find("Please try again in ") {
            let after_prefix = &message[retry_pos + 20..]; // Skip "Please try again in "
                                                           // Find the 's' or 'ms' that ends the time value
            if let Some(end_pos) = after_prefix.find(['s', '.']) {
                let time_str = &after_prefix[..end_pos];
                if let Ok(seconds) = time_str.parse::<f64>() {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let seconds_int = seconds.ceil() as u64;
                    return format!(
                        "Groq AI rate limit reached. Please try again in {seconds_int} seconds."
                    );
                }
            }
        }
        // Fallback: provide a helpful message with the limit info if available
        if message.contains("tokens per minute") {
            "Groq AI rate limit reached (tokens per minute). Please wait a moment and try again."
                .to_owned()
        } else {
            "Groq AI rate limit reached. Please wait a moment and try again.".to_owned()
        }
    }

    /// Check if an error from the Groq API is retryable (transient)
    fn is_retryable_error(status: u16) -> bool {
        sse_parser::is_retryable_status(status)
    }

    /// Build an authenticated HTTP request to the Groq API
    fn build_request(&self, groq_request: &GroqRequest) -> reqwest::RequestBuilder {
        self.client
            .post(Self::api_url("chat/completions"))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(groq_request)
    }

    /// Parse a Groq SSE data payload into a `StreamChunk`
    fn parse_stream_data(json_str: &str) -> Option<Result<StreamChunk, AppError>> {
        match serde_json::from_str::<GroqStreamChunk>(json_str) {
            Ok(chunk) => {
                let choice = chunk.choices.into_iter().next()?;
                let delta = choice.delta.content.unwrap_or_default();
                let is_final = choice.finish_reason.is_some();
                Some(Ok(StreamChunk {
                    delta,
                    is_final,
                    finish_reason: choice.finish_reason,
                }))
            }
            Err(e) => {
                warn!("Failed to parse Groq stream chunk: {e}");
                None
            }
        }
    }

    /// Convert internal Tool format to Groq's OpenAI-compatible format
    fn convert_tools(tools: &[Tool]) -> Vec<GroqTool> {
        tools
            .iter()
            .flat_map(|tool| {
                tool.function_declarations.iter().map(|func| GroqTool {
                    tool_type: "function".to_owned(),
                    function: GroqFunction {
                        name: func.name.clone(),
                        description: func.description.clone(),
                        parameters: func.parameters.clone(),
                    },
                })
            })
            .collect()
    }

    /// Convert Groq tool calls to internal `FunctionCall` format
    fn convert_tool_calls(tool_calls: &[GroqToolCall]) -> Vec<FunctionCall> {
        tool_calls
            .iter()
            .map(|call| {
                debug!(
                    tool_call_id = %call.id,
                    tool_call_type = %call.call_type,
                    function_name = %call.function.name,
                    "Converting Groq tool call to FunctionCall"
                );
                // Parse the arguments JSON string
                let args: Value =
                    serde_json::from_str(&call.function.arguments).unwrap_or_default();
                FunctionCall {
                    name: call.function.name.clone(),
                    args,
                }
            })
            .collect()
    }

    /// Perform a chat completion with tool/function calling support
    ///
    /// # Errors
    ///
    /// Returns an error if the API call fails or response parsing fails.
    #[instrument(skip(self, request, tools), fields(model = %request.model.as_deref().unwrap_or(&self.default_model)))]
    pub async fn complete_with_tools(
        &self,
        request: &ChatRequest,
        tools: Option<Vec<Tool>>,
    ) -> Result<ChatResponseWithTools, AppError> {
        let model = request.model.as_deref().unwrap_or(&self.default_model);

        debug!("Sending chat completion request to Groq with tools");

        let groq_tools = tools.as_ref().map(|t| Self::convert_tools(t));

        let groq_request = GroqRequest {
            model: model.to_owned(),
            messages: Self::convert_messages(&request.messages),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: Some(false),
            tools: groq_tools,
            tool_choice: tools.as_ref().map(|_| "auto".to_owned()),
        };

        let response = self
            .client
            .post(Self::api_url("chat/completions"))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&groq_request)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to send request to Groq API: {}", e);
                AppError::external_service("Groq", format!("Failed to connect: {e}"))
            })?;

        let status = response.status();
        let body = response.text().await.map_err(|e| {
            error!("Failed to read Groq API response: {}", e);
            AppError::external_service("Groq", format!("Failed to read response: {e}"))
        })?;

        if !status.is_success() {
            return Err(Self::parse_error_response(status, &body));
        }

        let groq_response: GroqResponse = serde_json::from_str(&body).map_err(|e| {
            error!("Failed to parse Groq API response: {}", e);
            AppError::external_service("Groq", format!("Failed to parse response: {e}"))
        })?;

        let choice = groq_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| AppError::external_service("Groq", "API returned no choices"))?;

        let content = choice.message.content;
        let function_calls = choice.message.tool_calls.map(|calls| {
            info!("Groq returned {} tool calls", calls.len());
            Self::convert_tool_calls(&calls)
        });

        debug!(
            "Received response from Groq: content={:?}, tool_calls={:?}, finish_reason: {:?}",
            content.as_ref().map(String::len),
            function_calls.as_ref().map(Vec::len),
            choice.finish_reason
        );

        Ok(ChatResponseWithTools {
            content,
            function_calls,
            model: groq_response.model,
            usage: groq_response.usage.map(|u| TokenUsage {
                prompt_tokens: u.prompt,
                completion_tokens: u.completion,
                total_tokens: u.total,
            }),
            finish_reason: choice.finish_reason,
        })
    }
}

#[async_trait]
impl LlmProvider for GroqProvider {
    fn name(&self) -> &'static str {
        "groq"
    }

    fn display_name(&self) -> &'static str {
        "Groq (Llama/Mixtral)"
    }

    fn capabilities(&self) -> LlmCapabilities {
        // Groq supports streaming, function calling, and system messages
        // but does not support vision (yet)
        LlmCapabilities::STREAMING
            | LlmCapabilities::FUNCTION_CALLING
            | LlmCapabilities::SYSTEM_MESSAGES
            | LlmCapabilities::JSON_MODE
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }

    fn available_models(&self) -> &'static [&'static str] {
        AVAILABLE_MODELS
    }

    #[instrument(skip(self, request), fields(model = %request.model.as_deref().unwrap_or(&self.default_model)))]
    async fn complete(&self, request: &ChatRequest) -> Result<ChatResponse, AppError> {
        let model = request.model.as_deref().unwrap_or(&self.default_model);

        let groq_request = GroqRequest {
            model: model.to_owned(),
            messages: Self::convert_messages(&request.messages),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: Some(false),
            tools: None,
            tool_choice: None,
        };

        let mut last_error: Option<AppError> = None;

        for attempt in 0..=self.retry_config.max_retries {
            if attempt > 0 {
                let delay = self.retry_config.delay_for_attempt(attempt - 1);
                warn!(
                    attempt = attempt,
                    delay_ms = delay.as_millis(),
                    "Retrying Groq request after transient failure"
                );
                sleep(delay).await;
            }

            debug!(attempt = attempt, "Sending chat completion request to Groq");

            let response = match self.build_request(&groq_request).send().await {
                Ok(r) => r,
                Err(e) => {
                    let error =
                        AppError::external_service("Groq", format!("Failed to connect: {e}"));
                    if sse_parser::is_retryable_request_error(&e)
                        && attempt < self.retry_config.max_retries
                    {
                        last_error = Some(error);
                        continue;
                    }
                    return Err(error);
                }
            };

            let status = response.status();
            let body = match response.text().await {
                Ok(t) => t,
                Err(e) => {
                    let error =
                        AppError::external_service("Groq", format!("Failed to read response: {e}"));
                    if attempt < self.retry_config.max_retries {
                        last_error = Some(error);
                        continue;
                    }
                    return Err(error);
                }
            };

            if !status.is_success() {
                let error = Self::parse_error_response(status, &body);
                if Self::is_retryable_error(status.as_u16())
                    && attempt < self.retry_config.max_retries
                {
                    warn!(status = %status, attempt = attempt, "Groq API returned retryable error");
                    last_error = Some(error);
                    continue;
                }
                return Err(error);
            }

            let groq_response: GroqResponse = serde_json::from_str(&body).map_err(|e| {
                error!("Failed to parse Groq API response: {e}");
                AppError::external_service("Groq", format!("Failed to parse response: {e}"))
            })?;

            let choice = groq_response
                .choices
                .into_iter()
                .next()
                .ok_or_else(|| AppError::external_service("Groq", "API returned no choices"))?;

            let content = choice.message.content.unwrap_or_default();

            debug!(
                attempt = attempt,
                "Received response from Groq: {} chars, finish_reason: {:?}",
                content.len(),
                choice.finish_reason
            );

            return Ok(ChatResponse {
                content,
                model: groq_response.model,
                usage: groq_response.usage.map(|u| TokenUsage {
                    prompt_tokens: u.prompt,
                    completion_tokens: u.completion,
                    total_tokens: u.total,
                }),
                finish_reason: choice.finish_reason,
            });
        }

        Err(last_error
            .unwrap_or_else(|| AppError::external_service("Groq", "Request failed after retries")))
    }

    #[instrument(skip(self, request), fields(model = %request.model.as_deref().unwrap_or(&self.default_model)))]
    async fn complete_stream(&self, request: &ChatRequest) -> Result<ChatStream, AppError> {
        let model = request.model.as_deref().unwrap_or(&self.default_model);

        let groq_request = GroqRequest {
            model: model.to_owned(),
            messages: Self::convert_messages(&request.messages),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: Some(true),
            tools: None,
            tool_choice: None,
        };

        // Retry the initial HTTP request (not the stream itself)
        let mut last_error: Option<AppError> = None;

        for attempt in 0..=self.retry_config.max_retries {
            if attempt > 0 {
                let delay = self.retry_config.delay_for_attempt(attempt - 1);
                warn!(
                    attempt = attempt,
                    delay_ms = delay.as_millis(),
                    "Retrying Groq streaming request after transient failure"
                );
                sleep(delay).await;
            }

            debug!(
                attempt = attempt,
                "Sending streaming chat completion request to Groq"
            );

            let response = match self.build_request(&groq_request).send().await {
                Ok(r) => r,
                Err(e) => {
                    let error =
                        AppError::external_service("Groq", format!("Failed to connect: {e}"));
                    if sse_parser::is_retryable_request_error(&e)
                        && attempt < self.retry_config.max_retries
                    {
                        last_error = Some(error);
                        continue;
                    }
                    return Err(error);
                }
            };

            let status = response.status();

            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                let error = Self::parse_error_response(status, &body);
                if Self::is_retryable_error(status.as_u16())
                    && attempt < self.retry_config.max_retries
                {
                    warn!(status = %status, attempt = attempt, "Groq streaming API returned retryable error");
                    last_error = Some(error);
                    continue;
                }
                return Err(error);
            }

            // Connection succeeded — build SSE stream with shared parser
            let stream = sse_parser::create_sse_stream(
                response.bytes_stream(),
                Self::parse_stream_data,
                "Groq",
            );
            return Ok(stream);
        }

        Err(last_error.unwrap_or_else(|| {
            AppError::external_service("Groq", "Streaming request failed after retries")
        }))
    }

    #[instrument(skip(self))]
    async fn health_check(&self) -> Result<bool, AppError> {
        debug!("Performing Groq API health check");

        // Use the models endpoint for a lightweight health check
        let response = self
            .client
            .get(Self::api_url("models"))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| {
                error!("Groq health check failed: {}", e);
                AppError::external_service("Groq", format!("Health check failed: {e}"))
            })?;

        let healthy = response.status().is_success();

        if healthy {
            debug!("Groq API health check passed");
        } else {
            warn!(
                "Groq API health check failed with status: {}",
                response.status()
            );
        }

        Ok(healthy)
    }
}
