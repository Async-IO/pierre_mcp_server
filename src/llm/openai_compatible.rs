// ABOUTME: Generic OpenAI-compatible LLM provider for local and cloud endpoints
// ABOUTME: Supports Ollama, vLLM, LocalAI, and any OpenAI-compatible API
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # `OpenAI`-Compatible Provider
//!
//! Generic implementation for any `OpenAI`-compatible LLM endpoint.
//! This enables integration with local LLM servers like Ollama, vLLM, and `LocalAI`.
//!
//! ## Configuration
//!
//! Set environment variables to configure the local provider:
//! - `LOCAL_LLM_BASE_URL`: Base URL (default: <http://localhost:11434/v1> for Ollama)
//! - `LOCAL_LLM_MODEL`: Model to use (default: `qwen2.5:14b-instruct`)
//! - `LOCAL_LLM_API_KEY`: API key (optional, empty for local servers)
//!
//! ## Supported Backends
//!
//! - **Ollama**: <http://localhost:11434/v1>
//! - **vLLM**: <http://localhost:8000/v1>
//! - **`LocalAI`**: <http://localhost:8080/v1>
//! - **Any `OpenAI`-compatible endpoint**
//!
//! ## Example
//!
//! ```rust,no_run
//! use pierre_mcp_server::llm::{OpenAiCompatibleProvider, LlmProvider, ChatRequest, ChatMessage};
//! use pierre_mcp_server::errors::AppError;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), AppError> {
//!     // Create provider for local Ollama instance
//!     let provider = OpenAiCompatibleProvider::from_env()?;
//!     let request = ChatRequest::new(vec![
//!         ChatMessage::user("What is machine learning?"),
//!     ]);
//!     let response = provider.complete(&request).await?;
//!     println!("{}", response.content);
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;
use futures_util::{future, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::time::Duration;
use tracing::{debug, error, info, instrument, warn};

use super::{
    ChatMessage, ChatRequest, ChatResponse, ChatResponseWithTools, ChatStream, FunctionCall,
    LlmCapabilities, LlmProvider, StreamChunk, TokenUsage, Tool,
};
use crate::errors::{AppError, ErrorCode};

// ============================================================================
// Configuration Constants
// ============================================================================

/// Environment variable for local LLM base URL
const LOCAL_LLM_BASE_URL_ENV: &str = "LOCAL_LLM_BASE_URL";

/// Environment variable for local LLM model
const LOCAL_LLM_MODEL_ENV: &str = "LOCAL_LLM_MODEL";

/// Environment variable for local LLM API key (optional)
const LOCAL_LLM_API_KEY_ENV: &str = "LOCAL_LLM_API_KEY";

/// Default base URL (Ollama)
const DEFAULT_BASE_URL: &str = "http://localhost:11434/v1";

/// Default model for local inference
const DEFAULT_MODEL: &str = "qwen2.5:14b-instruct";

/// Connection timeout for local servers (more lenient than cloud)
const CONNECT_TIMEOUT_SECS: u64 = 30;

/// Request timeout (local inference can be slower)
const REQUEST_TIMEOUT_SECS: u64 = 300;

// ============================================================================
// API Request/Response Types (OpenAI-compatible format)
// ============================================================================

/// OpenAI-compatible API request structure
#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAiTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
}

/// Tool definition for OpenAI-compatible API
#[derive(Debug, Clone, Serialize)]
struct OpenAiTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAiFunction,
}

/// Function definition within a tool
#[derive(Debug, Clone, Serialize)]
struct OpenAiFunction {
    name: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parameters: Option<Value>,
}

/// Message structure for OpenAI-compatible API
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

impl From<&ChatMessage> for OpenAiMessage {
    fn from(msg: &ChatMessage) -> Self {
        Self {
            role: msg.role.as_str().to_owned(),
            content: msg.content.clone(),
        }
    }
}

/// OpenAI-compatible API response structure
#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
    #[serde(default)]
    usage: Option<OpenAiUsage>,
    model: String,
}

/// Choice in response
#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiResponseMessage,
    finish_reason: Option<String>,
}

/// Message in response
#[derive(Debug, Deserialize)]
struct OpenAiResponseMessage {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAiToolCall>>,
}

/// Tool call in response
#[derive(Debug, Clone, Deserialize)]
struct OpenAiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OpenAiFunctionCall,
}

/// Function call details in response
#[derive(Debug, Clone, Deserialize)]
struct OpenAiFunctionCall {
    name: String,
    arguments: String,
}

/// Usage statistics in response
#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    #[serde(rename = "prompt_tokens")]
    prompt: u32,
    #[serde(rename = "completion_tokens")]
    completion: u32,
    #[serde(rename = "total_tokens")]
    total: u32,
}

/// Streaming chunk structure
#[derive(Debug, Deserialize)]
struct OpenAiStreamChunk {
    choices: Vec<OpenAiStreamChoice>,
}

/// Choice in streaming chunk
#[derive(Debug, Deserialize)]
struct OpenAiStreamChoice {
    delta: OpenAiDelta,
    finish_reason: Option<String>,
}

/// Delta content in streaming chunk
#[derive(Debug, Deserialize)]
struct OpenAiDelta {
    #[serde(default)]
    content: Option<String>,
}

/// Error response structure
#[derive(Debug, Deserialize)]
struct OpenAiErrorResponse {
    error: OpenAiErrorDetail,
}

/// Error detail structure
#[derive(Debug, Deserialize)]
struct OpenAiErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: Option<String>,
}

// ============================================================================
// Provider Configuration
// ============================================================================

/// Configuration for the `OpenAI`-compatible provider
#[derive(Debug, Clone)]
pub struct OpenAiCompatibleConfig {
    /// Base URL for the API (e.g., <http://localhost:11434/v1>)
    pub base_url: String,
    /// API key (optional for local servers)
    pub api_key: Option<String>,
    /// Default model to use
    pub default_model: String,
    /// Provider name for display/logging
    pub provider_name: String,
    /// Provider display name
    pub display_name: String,
    /// Capabilities of this provider
    pub capabilities: LlmCapabilities,
}

impl OpenAiCompatibleConfig {
    /// Create configuration for a local Ollama instance
    #[must_use]
    pub fn ollama(model: &str) -> Self {
        Self {
            base_url: "http://localhost:11434/v1".to_owned(),
            api_key: None,
            default_model: model.to_owned(),
            provider_name: "ollama".to_owned(),
            display_name: "Ollama (Local)".to_owned(),
            capabilities: LlmCapabilities::STREAMING
                | LlmCapabilities::FUNCTION_CALLING
                | LlmCapabilities::SYSTEM_MESSAGES,
        }
    }

    /// Create configuration for a local vLLM instance
    #[must_use]
    pub fn vllm(model: &str) -> Self {
        Self {
            base_url: "http://localhost:8000/v1".to_owned(),
            api_key: None,
            default_model: model.to_owned(),
            provider_name: "vllm".to_owned(),
            display_name: "vLLM (Local)".to_owned(),
            capabilities: LlmCapabilities::STREAMING
                | LlmCapabilities::FUNCTION_CALLING
                | LlmCapabilities::SYSTEM_MESSAGES
                | LlmCapabilities::JSON_MODE,
        }
    }

    /// Create configuration for `LocalAI`
    #[must_use]
    pub fn local_ai(model: &str) -> Self {
        Self {
            base_url: "http://localhost:8080/v1".to_owned(),
            api_key: None,
            default_model: model.to_owned(),
            provider_name: "localai".to_owned(),
            display_name: "LocalAI".to_owned(),
            capabilities: LlmCapabilities::STREAMING
                | LlmCapabilities::FUNCTION_CALLING
                | LlmCapabilities::SYSTEM_MESSAGES,
        }
    }
}

impl Default for OpenAiCompatibleConfig {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_owned(),
            api_key: None,
            default_model: DEFAULT_MODEL.to_owned(),
            provider_name: "local".to_owned(),
            display_name: "Local LLM".to_owned(),
            capabilities: LlmCapabilities::STREAMING
                | LlmCapabilities::FUNCTION_CALLING
                | LlmCapabilities::SYSTEM_MESSAGES,
        }
    }
}

// ============================================================================
// Provider Implementation
// ============================================================================

/// Generic `OpenAI`-compatible LLM provider
///
/// Works with any endpoint that implements the `OpenAI` chat completions API,
/// including Ollama, vLLM, `LocalAI`, and cloud services.
pub struct OpenAiCompatibleProvider {
    client: Client,
    config: OpenAiCompatibleConfig,
}

impl OpenAiCompatibleProvider {
    /// Create a new provider with the given configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created.
    pub fn new(config: OpenAiCompatibleConfig) -> Result<Self, AppError> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(|e| AppError::internal(format!("Failed to create HTTP client: {e}")))?;

        Ok(Self { client, config })
    }

    /// Create a provider from environment variables
    ///
    /// Reads:
    /// - `LOCAL_LLM_BASE_URL`: Base URL (default: Ollama at localhost:11434)
    /// - `LOCAL_LLM_MODEL`: Model name (default: qwen2.5:14b-instruct)
    /// - `LOCAL_LLM_API_KEY`: API key (optional)
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created.
    pub fn from_env() -> Result<Self, AppError> {
        let base_url =
            env::var(LOCAL_LLM_BASE_URL_ENV).unwrap_or_else(|_| DEFAULT_BASE_URL.to_owned());
        let default_model =
            env::var(LOCAL_LLM_MODEL_ENV).unwrap_or_else(|_| DEFAULT_MODEL.to_owned());
        let api_key = env::var(LOCAL_LLM_API_KEY_ENV)
            .ok()
            .filter(|k| !k.is_empty());

        // Detect provider type from URL for better display names
        let (provider_name, display_name) = if base_url.contains(":11434") {
            ("ollama", "Ollama (Local)")
        } else if base_url.contains(":8000") {
            ("vllm", "vLLM (Local)")
        } else if base_url.contains(":8080") {
            ("localai", "LocalAI")
        } else {
            ("local", "Local LLM")
        };

        let config = OpenAiCompatibleConfig {
            base_url,
            api_key,
            default_model,
            provider_name: provider_name.to_owned(),
            display_name: display_name.to_owned(),
            capabilities: LlmCapabilities::STREAMING
                | LlmCapabilities::FUNCTION_CALLING
                | LlmCapabilities::SYSTEM_MESSAGES,
        };

        info!(
            "Initializing {} provider: base_url={}, model={}",
            config.display_name, config.base_url, config.default_model
        );

        Self::new(config)
    }

    /// Build the API URL for a given endpoint
    fn api_url(&self, endpoint: &str) -> String {
        format!(
            "{}/{}",
            self.config.base_url.trim_end_matches('/'),
            endpoint
        )
    }

    /// Convert internal messages to `OpenAI` format
    fn convert_messages(messages: &[ChatMessage]) -> Vec<OpenAiMessage> {
        messages.iter().map(OpenAiMessage::from).collect()
    }

    /// Log message details for debugging LLM interactions
    fn log_messages_debug(messages: &[OpenAiMessage], provider_name: &str, has_tools: bool) {
        for (i, msg) in messages.iter().enumerate() {
            debug!(
                "Message[{i}] role={}, content_len={}",
                msg.role,
                msg.content.len()
            );
            if msg.role == "system" {
                debug!(
                    "System prompt preview: {}...",
                    msg.content.chars().take(200).collect::<String>()
                );
            }
        }
        debug!(
            "Sending chat completion request to {provider_name} with {} messages and tools={has_tools:?}",
            messages.len()
        );
    }

    /// Parse error response from API
    fn parse_error_response(status: reqwest::StatusCode, body: &str) -> AppError {
        if let Ok(error_response) = serde_json::from_str::<OpenAiErrorResponse>(body) {
            let error_type = error_response
                .error
                .error_type
                .unwrap_or_else(|| "unknown".to_owned());

            match status.as_u16() {
                401 => AppError::auth_invalid(format!(
                    "API authentication failed: {}",
                    error_response.error.message
                )),
                429 => {
                    // Use ExternalRateLimited for proper client-facing messages
                    let user_message =
                        Self::extract_rate_limit_message(&error_response.error.message);
                    AppError::new(ErrorCode::ExternalRateLimited, user_message)
                }
                400 => AppError::invalid_input(format!(
                    "API validation error: {}",
                    error_response.error.message
                )),
                404 => AppError::not_found(format!(
                    "Model or endpoint not found: {}",
                    error_response.error.message
                )),
                503 => AppError::external_service(
                    "LocalLLM",
                    format!(
                        "Service unavailable (is the local server running?): {}",
                        error_response.error.message
                    ),
                ),
                _ => AppError::external_service(
                    "LocalLLM",
                    format!("{} - {}", error_type, error_response.error.message),
                ),
            }
        } else {
            // Handle non-JSON error responses (common with local servers)
            match status.as_u16() {
                502..=504 => AppError::external_service(
                    "LocalLLM",
                    "Local LLM server is not responding. Is Ollama/vLLM running?".to_owned(),
                ),
                _ => AppError::external_service(
                    "LocalLLM",
                    format!(
                        "API error ({}): {}",
                        status,
                        body.chars().take(200).collect::<String>()
                    ),
                ),
            }
        }
    }

    /// Extract a user-friendly rate limit message from OpenAI-compatible error
    ///
    /// OpenAI-style rate limit errors may include retry-after info.
    /// Most local LLM servers (Ollama, vLLM) rarely hit rate limits.
    fn extract_rate_limit_message(message: &str) -> String {
        // Try to extract "try again in X" or similar patterns
        if let Some(retry_pos) = message.to_lowercase().find("try again in ") {
            let after_prefix = &message[retry_pos + 13..];
            // Find the number and unit
            if let Some(end_pos) = after_prefix.find(|c: char| !c.is_ascii_digit() && c != '.') {
                let time_str = &after_prefix[..end_pos];
                if let Ok(seconds) = time_str.parse::<f64>() {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let seconds_int = seconds.ceil() as u64;
                    return format!(
                        "LLM rate limit reached. Please try again in {seconds_int} seconds."
                    );
                }
            }
        }
        // Fallback message
        "LLM rate limit reached. Please wait a moment and try again.".to_owned()
    }

    /// Convert internal Tool format to OpenAI-compatible format
    fn convert_tools(tools: &[Tool]) -> Vec<OpenAiTool> {
        tools
            .iter()
            .flat_map(|tool| {
                tool.function_declarations.iter().map(|func| OpenAiTool {
                    tool_type: "function".to_owned(),
                    function: OpenAiFunction {
                        name: func.name.clone(),
                        description: func.description.clone(),
                        parameters: func.parameters.clone(),
                    },
                })
            })
            .collect()
    }

    /// Convert tool calls to internal `FunctionCall` format
    fn convert_tool_calls(tool_calls: &[OpenAiToolCall]) -> Vec<FunctionCall> {
        tool_calls
            .iter()
            .map(|call| {
                debug!(
                    tool_call_id = %call.id,
                    tool_call_type = %call.call_type,
                    function_name = %call.function.name,
                    "Converting tool call to FunctionCall"
                );
                let args: Value =
                    serde_json::from_str(&call.function.arguments).unwrap_or_default();
                FunctionCall {
                    name: call.function.name.clone(),
                    args,
                }
            })
            .collect()
    }

    /// Add authorization header if API key is configured
    fn add_auth_header(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(ref api_key) = self.config.api_key {
            request.header("Authorization", format!("Bearer {api_key}"))
        } else {
            request
        }
    }

    /// Perform a chat completion with tool/function calling support
    ///
    /// # Errors
    ///
    /// Returns an error if the API call fails or response parsing fails.
    #[instrument(skip(self, request, tools), fields(model = %request.model.as_deref().unwrap_or(&self.config.default_model)))]
    pub async fn complete_with_tools(
        &self,
        request: &ChatRequest,
        tools: Option<Vec<Tool>>,
    ) -> Result<ChatResponseWithTools, AppError> {
        let model = request
            .model
            .as_deref()
            .unwrap_or(&self.config.default_model);

        let converted_messages = Self::convert_messages(&request.messages);
        Self::log_messages_debug(
            &converted_messages,
            &self.config.provider_name,
            tools.is_some(),
        );
        let openai_tools = tools.as_ref().map(|t| Self::convert_tools(t));

        let openai_request = OpenAiRequest {
            model: model.to_owned(),
            messages: converted_messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: Some(false),
            tools: openai_tools,
            tool_choice: tools.as_ref().map(|_| "auto".to_owned()),
        };

        let http_request = self
            .client
            .post(self.api_url("chat/completions"))
            .header("Content-Type", "application/json")
            .json(&openai_request);

        let response = self
            .add_auth_header(http_request)
            .send()
            .await
            .map_err(|e| {
                error!(
                    "Failed to send request to {}: {}",
                    self.config.provider_name, e
                );
                if e.is_connect() {
                    AppError::external_service(
                        "LocalLLM",
                        format!(
                            "Cannot connect to {}. Is the server running at {}?",
                            self.config.display_name, self.config.base_url
                        ),
                    )
                } else {
                    AppError::external_service("LocalLLM", format!("Failed to connect: {e}"))
                }
            })?;

        let status = response.status();
        let body = response.text().await.map_err(|e| {
            error!("Failed to read API response: {}", e);
            AppError::external_service("LocalLLM", format!("Failed to read response: {e}"))
        })?;

        if !status.is_success() {
            return Err(Self::parse_error_response(status, &body));
        }

        let openai_response: OpenAiResponse = serde_json::from_str(&body).map_err(|e| {
            error!(
                "Failed to parse API response: {} - body: {}",
                e,
                &body[..body.len().min(500)]
            );
            AppError::external_service("LocalLLM", format!("Failed to parse response: {e}"))
        })?;

        let choice = openai_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| AppError::external_service("LocalLLM", "API returned no choices"))?;

        let content = choice.message.content;
        let function_calls = choice.message.tool_calls.map(|calls| {
            info!(
                "{} returned {} tool calls",
                self.config.provider_name,
                calls.len()
            );
            Self::convert_tool_calls(&calls)
        });

        debug!(
            "Received response from {}: content={:?}, tool_calls={:?}, finish_reason: {:?}",
            self.config.provider_name,
            content.as_ref().map(String::len),
            function_calls.as_ref().map(Vec::len),
            choice.finish_reason
        );

        Ok(ChatResponseWithTools {
            content,
            function_calls,
            model: openai_response.model,
            usage: openai_response.usage.map(|u| TokenUsage {
                prompt_tokens: u.prompt,
                completion_tokens: u.completion,
                total_tokens: u.total,
            }),
            finish_reason: choice.finish_reason,
        })
    }
}

#[async_trait]
impl LlmProvider for OpenAiCompatibleProvider {
    fn name(&self) -> &'static str {
        // Return a static str based on common provider names
        // This is a limitation of the trait requiring &'static str
        match self.config.provider_name.as_str() {
            "ollama" => "ollama",
            "vllm" => "vllm",
            "localai" => "localai",
            _ => "local",
        }
    }

    fn display_name(&self) -> &'static str {
        match self.config.provider_name.as_str() {
            "ollama" => "Ollama (Local)",
            "vllm" => "vLLM (Local)",
            "localai" => "LocalAI",
            _ => "Local LLM",
        }
    }

    fn capabilities(&self) -> LlmCapabilities {
        self.config.capabilities
    }

    fn default_model(&self) -> &str {
        &self.config.default_model
    }

    fn available_models(&self) -> &'static [&'static str] {
        // Common models available via Ollama
        &[
            "qwen2.5:14b-instruct",
            "qwen2.5:7b-instruct",
            "qwen2.5:32b-instruct",
            "llama3.1:8b-instruct",
            "llama3.1:70b-instruct",
            "llama3.3:70b-instruct",
            "mistral:7b-instruct",
            "hermes2pro:latest",
        ]
    }

    #[instrument(skip(self, request), fields(model = %request.model.as_deref().unwrap_or(&self.config.default_model)))]
    async fn complete(&self, request: &ChatRequest) -> Result<ChatResponse, AppError> {
        let model = request
            .model
            .as_deref()
            .unwrap_or(&self.config.default_model);

        let converted_messages = Self::convert_messages(&request.messages);
        Self::log_messages_debug(&converted_messages, &self.config.provider_name, false);

        let openai_request = OpenAiRequest {
            model: model.to_owned(),
            messages: converted_messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: Some(false),
            tools: None,
            tool_choice: None,
        };

        let http_request = self
            .client
            .post(self.api_url("chat/completions"))
            .header("Content-Type", "application/json")
            .json(&openai_request);

        let response = self
            .add_auth_header(http_request)
            .send()
            .await
            .map_err(|e| {
                error!(
                    "Failed to send request to {}: {}",
                    self.config.provider_name, e
                );
                if e.is_connect() {
                    AppError::external_service(
                        "LocalLLM",
                        format!(
                            "Cannot connect to {}. Is the server running at {}?",
                            self.config.display_name, self.config.base_url
                        ),
                    )
                } else {
                    AppError::external_service("LocalLLM", format!("Failed to connect: {e}"))
                }
            })?;

        let status = response.status();
        let body = response.text().await.map_err(|e| {
            error!("Failed to read API response: {}", e);
            AppError::external_service("LocalLLM", format!("Failed to read response: {e}"))
        })?;

        if !status.is_success() {
            return Err(Self::parse_error_response(status, &body));
        }

        let openai_response: OpenAiResponse = serde_json::from_str(&body).map_err(|e| {
            error!("Failed to parse API response: {}", e);
            AppError::external_service("LocalLLM", format!("Failed to parse response: {e}"))
        })?;

        let choice = openai_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| AppError::external_service("LocalLLM", "API returned no choices"))?;

        let content = choice.message.content.unwrap_or_default();

        debug!(
            "Received response from {}: {} chars, finish_reason: {:?}",
            self.config.provider_name,
            content.len(),
            choice.finish_reason
        );

        Ok(ChatResponse {
            content,
            model: openai_response.model,
            usage: openai_response.usage.map(|u| TokenUsage {
                prompt_tokens: u.prompt,
                completion_tokens: u.completion,
                total_tokens: u.total,
            }),
            finish_reason: choice.finish_reason,
        })
    }

    #[instrument(skip(self, request), fields(model = %request.model.as_deref().unwrap_or(&self.config.default_model)))]
    async fn complete_stream(&self, request: &ChatRequest) -> Result<ChatStream, AppError> {
        let model = request
            .model
            .as_deref()
            .unwrap_or(&self.config.default_model);

        debug!(
            "Sending streaming chat completion request to {}",
            self.config.provider_name
        );

        let openai_request = OpenAiRequest {
            model: model.to_owned(),
            messages: Self::convert_messages(&request.messages),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: Some(true),
            tools: None,
            tool_choice: None,
        };

        let http_request = self
            .client
            .post(self.api_url("chat/completions"))
            .header("Content-Type", "application/json")
            .json(&openai_request);

        let response = self
            .add_auth_header(http_request)
            .send()
            .await
            .map_err(|e| {
                error!(
                    "Failed to send streaming request to {}: {}",
                    self.config.provider_name, e
                );
                if e.is_connect() {
                    AppError::external_service(
                        "LocalLLM",
                        format!(
                            "Cannot connect to {}. Is the server running at {}?",
                            self.config.display_name, self.config.base_url
                        ),
                    )
                } else {
                    AppError::external_service("LocalLLM", format!("Failed to connect: {e}"))
                }
            })?;

        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(Self::parse_error_response(status, &body));
        }

        let byte_stream = response.bytes_stream();

        let stream = byte_stream
            .map(move |chunk_result| {
                match chunk_result {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);

                        // Parse SSE format: "data: {...}\n\n"
                        let mut result_chunks = Vec::new();

                        for line in text.lines() {
                            let line = line.trim();

                            if line.is_empty() {
                                continue;
                            }

                            if line == "data: [DONE]" {
                                result_chunks.push(Ok(StreamChunk {
                                    delta: String::new(),
                                    is_final: true,
                                    finish_reason: Some("stop".to_owned()),
                                }));
                                continue;
                            }

                            if let Some(json_str) = line.strip_prefix("data: ") {
                                match serde_json::from_str::<OpenAiStreamChunk>(json_str) {
                                    Ok(chunk) => {
                                        if let Some(choice) = chunk.choices.into_iter().next() {
                                            let delta = choice.delta.content.unwrap_or_default();
                                            let is_final = choice.finish_reason.is_some();

                                            result_chunks.push(Ok(StreamChunk {
                                                delta,
                                                is_final,
                                                finish_reason: choice.finish_reason,
                                            }));
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Failed to parse stream chunk: {}", e);
                                    }
                                }
                            }
                        }

                        // Return the first chunk or an empty one
                        result_chunks.into_iter().next().unwrap_or_else(|| {
                            Ok(StreamChunk {
                                delta: String::new(),
                                is_final: false,
                                finish_reason: None,
                            })
                        })
                    }
                    Err(e) => {
                        error!("Error reading stream: {}", e);
                        Err(AppError::external_service(
                            "LocalLLM",
                            format!("Stream read error: {e}"),
                        ))
                    }
                }
            })
            .filter(|result| {
                // Filter out empty deltas unless it's the final chunk
                future::ready(
                    result
                        .as_ref()
                        .map_or(true, |chunk| !chunk.delta.is_empty() || chunk.is_final),
                )
            });

        Ok(Box::pin(stream))
    }

    #[instrument(skip(self))]
    async fn health_check(&self) -> Result<bool, AppError> {
        debug!(
            "Performing {} health check at {}",
            self.config.provider_name, self.config.base_url
        );

        // Try the models endpoint for a lightweight health check
        let http_request = self.client.get(self.api_url("models"));

        let response = self
            .add_auth_header(http_request)
            .send()
            .await
            .map_err(|e| {
                error!("{} health check failed: {}", self.config.provider_name, e);
                if e.is_connect() {
                    AppError::external_service(
                        "LocalLLM",
                        format!(
                            "Cannot connect to {}. Is the server running at {}?",
                            self.config.display_name, self.config.base_url
                        ),
                    )
                } else {
                    AppError::external_service("LocalLLM", format!("Health check failed: {e}"))
                }
            })?;

        let healthy = response.status().is_success();

        if healthy {
            debug!("{} health check passed", self.config.provider_name);
        } else {
            warn!(
                "{} health check failed with status: {}",
                self.config.provider_name,
                response.status()
            );
        }

        Ok(healthy)
    }
}
