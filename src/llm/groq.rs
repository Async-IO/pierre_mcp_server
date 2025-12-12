// ABOUTME: Groq LLM provider implementation with streaming support
// ABOUTME: Uses OpenAI-compatible API for Llama, Mixtral models via Groq's fast LPU inference
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Groq Provider
//!
//! Implementation of the `LlmProvider` trait for Groq's LPU-accelerated inference.
//!
//! ## Configuration
//!
//! Set the `GROQ_API_KEY` environment variable with your API key from
//! Groq Console: <https://console.groq.com/keys>
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
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, instrument, warn};

use super::{
    ChatMessage, ChatRequest, ChatResponse, ChatStream, LlmCapabilities, LlmProvider, StreamChunk,
    TokenUsage,
};
use crate::errors::AppError;

/// Environment variable for Groq API key
const GROQ_API_KEY_ENV: &str = "GROQ_API_KEY";

/// Default model to use
const DEFAULT_MODEL: &str = "llama-3.3-70b-versatile";

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
}

impl GroqProvider {
    /// Create a new Groq provider with the given API key
    #[must_use]
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    /// Create a Groq provider from environment variable
    ///
    /// # Errors
    ///
    /// Returns an error if `GROQ_API_KEY` is not set
    pub fn from_env() -> Result<Self, AppError> {
        let api_key = std::env::var(GROQ_API_KEY_ENV).map_err(|_| {
            AppError::config(format!(
                "Missing {GROQ_API_KEY_ENV} environment variable. Get your API key from https://console.groq.com/keys"
            ))
        })?;

        Ok(Self::new(api_key))
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
                429 => AppError::external_service(
                    "Groq",
                    format!("Rate limit exceeded: {}", error_response.error.message),
                ),
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
            AppError::external_service(
                "Groq",
                format!(
                    "API error ({}): {}",
                    status,
                    body.chars().take(200).collect::<String>()
                ),
            )
        }
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

    fn default_model(&self) -> &'static str {
        DEFAULT_MODEL
    }

    fn available_models(&self) -> &'static [&'static str] {
        AVAILABLE_MODELS
    }

    #[instrument(skip(self, request), fields(model = %request.model.as_deref().unwrap_or(DEFAULT_MODEL)))]
    async fn complete(&self, request: &ChatRequest) -> Result<ChatResponse, AppError> {
        let model = request.model.as_deref().unwrap_or(DEFAULT_MODEL);

        debug!("Sending chat completion request to Groq");

        let groq_request = GroqRequest {
            model: model.to_owned(),
            messages: Self::convert_messages(&request.messages),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: Some(false),
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

        let content = choice.message.content.unwrap_or_default();

        debug!(
            "Received response from Groq: {} chars, finish_reason: {:?}",
            content.len(),
            choice.finish_reason
        );

        Ok(ChatResponse {
            content,
            model: groq_response.model,
            usage: groq_response.usage.map(|u| TokenUsage {
                prompt_tokens: u.prompt,
                completion_tokens: u.completion,
                total_tokens: u.total,
            }),
            finish_reason: choice.finish_reason,
        })
    }

    #[instrument(skip(self, request), fields(model = %request.model.as_deref().unwrap_or(DEFAULT_MODEL)))]
    async fn complete_stream(&self, request: &ChatRequest) -> Result<ChatStream, AppError> {
        let model = request.model.as_deref().unwrap_or(DEFAULT_MODEL);

        debug!("Sending streaming chat completion request to Groq");

        let groq_request = GroqRequest {
            model: model.to_owned(),
            messages: Self::convert_messages(&request.messages),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: Some(true),
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
                error!("Failed to send streaming request to Groq API: {}", e);
                AppError::external_service("Groq", format!("Failed to connect: {e}"))
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
                                match serde_json::from_str::<GroqStreamChunk>(json_str) {
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
                                        warn!("Failed to parse Groq stream chunk: {}", e);
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
                        error!("Error reading Groq stream: {}", e);
                        Err(AppError::external_service(
                            "Groq",
                            format!("Stream read error: {e}"),
                        ))
                    }
                }
            })
            .filter(|result| {
                // Filter out empty deltas unless it's the final chunk
                futures_util::future::ready(
                    result
                        .as_ref()
                        .map_or(true, |chunk| !chunk.delta.is_empty() || chunk.is_final),
                )
            });

        Ok(Box::pin(stream))
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
