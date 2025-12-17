// ABOUTME: Unified LLM provider selector for runtime provider switching
// ABOUTME: Abstracts over Gemini and Groq providers based on environment configuration
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # LLM Provider Selector
//!
//! This module provides a unified interface for LLM providers that can be
//! configured at runtime via environment variables.
//!
//! ## Configuration
//!
//! Set `PIERRE_LLM_PROVIDER` environment variable:
//! - `groq` (default): Use Groq for cost-effective open-source models
//! - `gemini`: Use Google Gemini for full-featured capabilities
//!
//! ## Example
//!
//! ```rust,no_run
//! use pierre_mcp_server::llm::{ChatMessage, ChatRequest, ChatProvider};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), pierre_mcp_server::errors::AppError> {
//!     let provider = ChatProvider::from_env()?;
//!     let request = ChatRequest::new(vec![
//!         ChatMessage::user("Hello!"),
//!     ]);
//!     let response = provider.complete(&request).await?;
//!     println!("{}", response.content);
//!     Ok(())
//! }
//! ```

use std::fmt;
use tracing::{debug, info};

use super::{
    ChatMessage, ChatRequest, ChatResponse, ChatResponseWithTools, ChatStream, FunctionResponse,
    GeminiProvider, GroqProvider, LlmCapabilities, LlmProvider, Tool,
};
use crate::config::LlmProviderType;
use crate::errors::AppError;

/// Unified chat provider that wraps either Gemini or Groq
///
/// This enum provides a consistent interface regardless of which
/// underlying provider is configured.
pub enum ChatProvider {
    /// Google Gemini provider with full tool calling support
    Gemini(GeminiProvider),
    /// Groq provider for fast, cost-effective inference
    Groq(GroqProvider),
}

impl ChatProvider {
    /// Create a provider from environment configuration
    ///
    /// Reads `PIERRE_LLM_PROVIDER` to determine which provider to use:
    /// - `groq` (default): Creates `GroqProvider` (requires `GROQ_API_KEY`)
    /// - `gemini`: Creates `GeminiProvider` (requires `GEMINI_API_KEY`)
    ///
    /// # Errors
    ///
    /// Returns an error if the required API key environment variable is missing.
    pub fn from_env() -> Result<Self, AppError> {
        let provider_type = LlmProviderType::from_env();

        info!(
            "Initializing LLM provider: {} (set {} to change)",
            provider_type,
            LlmProviderType::ENV_VAR
        );

        match provider_type {
            LlmProviderType::Groq => {
                let provider = GroqProvider::from_env()?;
                debug!(
                    "Using Groq provider with model: {}",
                    provider.default_model()
                );
                Ok(Self::Groq(provider))
            }
            LlmProviderType::Gemini => {
                let provider = GeminiProvider::from_env()?;
                debug!(
                    "Using Gemini provider with model: {}",
                    provider.default_model()
                );
                Ok(Self::Gemini(provider))
            }
        }
    }

    /// Create a Gemini provider explicitly
    ///
    /// # Errors
    ///
    /// Returns an error if `GEMINI_API_KEY` is not set.
    pub fn gemini() -> Result<Self, AppError> {
        Ok(Self::Gemini(GeminiProvider::from_env()?))
    }

    /// Create a Groq provider explicitly
    ///
    /// # Errors
    ///
    /// Returns an error if `GROQ_API_KEY` is not set.
    pub fn groq() -> Result<Self, AppError> {
        Ok(Self::Groq(GroqProvider::from_env()?))
    }

    /// Get the provider type
    #[must_use]
    pub const fn provider_type(&self) -> LlmProviderType {
        match self {
            Self::Gemini(_) => LlmProviderType::Gemini,
            Self::Groq(_) => LlmProviderType::Groq,
        }
    }

    /// Check if this provider supports tool calling
    #[must_use]
    pub fn supports_tool_calling(&self) -> bool {
        self.capabilities().supports_function_calling()
    }

    /// Perform a chat completion with tool/function calling support
    ///
    /// Both Gemini and Groq support native function/tool calling via their
    /// respective APIs (Gemini native, Groq OpenAI-compatible).
    ///
    /// # Errors
    ///
    /// Returns an error if the API call fails.
    pub async fn complete_with_tools(
        &self,
        request: &ChatRequest,
        tools: Option<Vec<Tool>>,
    ) -> Result<ChatResponseWithTools, AppError> {
        match self {
            Self::Gemini(provider) => provider.complete_with_tools(request, tools).await,
            Self::Groq(provider) => provider.complete_with_tools(request, tools).await,
        }
    }

    /// Add function responses to messages for multi-turn tool execution
    ///
    /// This helper adds function response content back to the conversation
    /// for the next LLM iteration.
    pub fn add_function_responses_to_messages(
        messages: &mut Vec<ChatMessage>,
        function_responses: &[FunctionResponse],
    ) {
        for func_response in function_responses {
            let response_text =
                serde_json::to_string(&func_response.response).unwrap_or_else(|_| "{}".to_owned());
            messages.push(ChatMessage::user(format!(
                "[Tool Result for {}]: {}",
                func_response.name, response_text
            )));
        }
    }
}

// Delegate LlmProvider trait methods to the underlying provider
impl ChatProvider {
    /// Get provider name
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Gemini(p) => p.name(),
            Self::Groq(p) => p.name(),
        }
    }

    /// Get provider display name
    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Gemini(p) => p.display_name(),
            Self::Groq(p) => p.display_name(),
        }
    }

    /// Get provider capabilities
    #[must_use]
    pub fn capabilities(&self) -> LlmCapabilities {
        match self {
            Self::Gemini(p) => p.capabilities(),
            Self::Groq(p) => p.capabilities(),
        }
    }

    /// Get default model
    #[must_use]
    pub fn default_model(&self) -> &'static str {
        match self {
            Self::Gemini(p) => p.default_model(),
            Self::Groq(p) => p.default_model(),
        }
    }

    /// Get available models
    #[must_use]
    pub fn available_models(&self) -> &'static [&'static str] {
        match self {
            Self::Gemini(p) => p.available_models(),
            Self::Groq(p) => p.available_models(),
        }
    }

    /// Perform a chat completion
    ///
    /// # Errors
    ///
    /// Returns an error if the API call fails.
    pub async fn complete(&self, request: &ChatRequest) -> Result<ChatResponse, AppError> {
        match self {
            Self::Gemini(p) => p.complete(request).await,
            Self::Groq(p) => p.complete(request).await,
        }
    }

    /// Perform a streaming chat completion
    ///
    /// # Errors
    ///
    /// Returns an error if the API call fails.
    pub async fn complete_stream(&self, request: &ChatRequest) -> Result<ChatStream, AppError> {
        match self {
            Self::Gemini(p) => p.complete_stream(request).await,
            Self::Groq(p) => p.complete_stream(request).await,
        }
    }

    /// Check provider health
    ///
    /// # Errors
    ///
    /// Returns an error if the health check fails.
    pub async fn health_check(&self) -> Result<bool, AppError> {
        match self {
            Self::Gemini(p) => p.health_check().await,
            Self::Groq(p) => p.health_check().await,
        }
    }
}

impl fmt::Debug for ChatProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gemini(_) => f.debug_tuple("ChatProvider::Gemini").finish(),
            Self::Groq(_) => f.debug_tuple("ChatProvider::Groq").finish(),
        }
    }
}
