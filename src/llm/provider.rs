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
//!     let provider = ChatProvider::from_env().await?;
//!     let request = ChatRequest::new(vec![
//!         ChatMessage::user("Hello!"),
//!     ]);
//!     let response = provider.complete(&request).await?;
//!     println!("{}", response.content);
//!     Ok(())
//! }
//! ```

use std::fmt;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info};
use uuid::Uuid;

use super::{
    ChatMessage, ChatRequest, ChatResponse, ChatResponseWithTools, ChatStream, FunctionResponse,
    GeminiProvider, GroqProvider, LlmCapabilities, LlmProvider, OpenAiCompatibleProvider, Tool,
};
use crate::config::LlmProviderType;
use crate::database_plugins::factory::Database;
use crate::errors::AppError;
use crate::tenant::llm_manager::{
    LlmCredentials, LlmProvider as TenantLlmProvider, TenantLlmManager,
};

/// Unified chat provider that wraps Gemini, Groq, or local LLM
///
/// This enum provides a consistent interface regardless of which
/// underlying provider is configured.
pub enum ChatProvider {
    /// Google Gemini provider with full tool calling support
    Gemini(GeminiProvider),
    /// Groq provider for fast, cost-effective inference
    Groq(GroqProvider),
    /// Local LLM provider via `OpenAI`-compatible API (Ollama, vLLM, `LocalAI`)
    Local(OpenAiCompatibleProvider),
}

impl ChatProvider {
    /// Create a provider from environment configuration
    ///
    /// Reads `PIERRE_LLM_PROVIDER` to determine which provider to use:
    /// - `groq` (default): Creates `GroqProvider` (requires `GROQ_API_KEY`)
    /// - `gemini`: Creates `GeminiProvider` (requires `GEMINI_API_KEY`)
    /// - `local`/`ollama`/`vllm`/`localai`: Creates `OpenAiCompatibleProvider`
    ///
    /// When `PIERRE_LLM_FALLBACK_ENABLED=true`, if the primary provider fails,
    /// attempts to use the fallback provider specified by `PIERRE_LLM_PROVIDER_FALLBACK`.
    ///
    /// # Errors
    ///
    /// Returns an error if the required API key environment variable is missing
    /// (for cloud providers) or if the local server cannot be reached, and
    /// fallback is disabled or also fails.
    pub async fn from_env() -> Result<Self, AppError> {
        let provider_type = LlmProviderType::from_env();

        info!(
            "Initializing LLM provider: {} (set {} to change)",
            provider_type,
            LlmProviderType::ENV_VAR
        );

        match Self::create_provider(provider_type) {
            Ok(provider) => {
                debug!(
                    "Provider {} initialized with model: {}",
                    provider.display_name(),
                    provider.default_model()
                );
                Ok(provider)
            }
            Err(primary_error) => Self::try_fallback(provider_type, primary_error).await,
        }
    }

    /// Attempt to initialize a fallback provider after primary fails
    async fn try_fallback(
        primary_type: LlmProviderType,
        primary_error: AppError,
    ) -> Result<Self, AppError> {
        let fallback_enabled = LlmProviderType::is_fallback_enabled();
        let fallback_provider = LlmProviderType::fallback_provider_from_env();

        let Some(fallback) = fallback_provider else {
            return Err(primary_error);
        };

        if !fallback_enabled || fallback == primary_type {
            return Err(primary_error);
        }

        let wait_secs = LlmProviderType::fallback_wait_secs();
        info!(
            "Primary provider {} failed, waiting {}s before fallback to {}",
            primary_type, wait_secs, fallback
        );

        sleep(Duration::from_secs(wait_secs)).await;

        match Self::create_provider(fallback) {
            Ok(provider) => {
                info!(
                    "Fallback provider {} initialized with model: {}",
                    provider.display_name(),
                    provider.default_model()
                );
                Ok(provider)
            }
            Err(fallback_error) => Err(AppError::config(format!(
                "Both primary ({primary_type}) and fallback ({fallback}) providers failed. \
                Primary: {primary_error}. Fallback: {fallback_error}"
            ))),
        }
    }

    /// Create a provider for a specific type
    fn create_provider(provider_type: LlmProviderType) -> Result<Self, AppError> {
        match provider_type {
            LlmProviderType::Groq => Self::groq(),
            LlmProviderType::Gemini => Self::gemini(),
            LlmProviderType::Local => Self::local(),
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

    /// Create a local LLM provider explicitly
    ///
    /// Uses environment variables for configuration:
    /// - `LOCAL_LLM_BASE_URL`: API endpoint (default: Ollama at localhost:11434)
    /// - `LOCAL_LLM_MODEL`: Model name (default: qwen2.5:14b-instruct)
    /// - `LOCAL_LLM_API_KEY`: API key (optional)
    ///
    /// # Errors
    ///
    /// Returns an error if the provider cannot be initialized.
    pub fn local() -> Result<Self, AppError> {
        Ok(Self::Local(OpenAiCompatibleProvider::from_env()?))
    }

    // ========================================
    // Tenant-Aware Factory Methods
    // ========================================

    /// Create a provider for a specific tenant and user
    ///
    /// Resolution order for API keys:
    /// 1. User-specific credentials (from `user_llm_credentials` table)
    /// 2. Tenant-level default (from `user_llm_credentials` table with `user_id = NULL`)
    /// 3. Environment variable fallback (`GEMINI_API_KEY`, `GROQ_API_KEY`, etc.)
    ///
    /// # Arguments
    /// * `user_id` - Optional user ID (None uses tenant defaults only)
    /// * `tenant_id` - Tenant ID
    /// * `provider` - Which LLM provider to use (Gemini, Groq, or Local)
    /// * `database` - Database connection for credential lookup
    ///
    /// # Errors
    ///
    /// Returns an error if no credentials are found for the provider.
    pub async fn from_tenant(
        user_id: Option<Uuid>,
        tenant_id: Uuid,
        provider: TenantLlmProvider,
        database: &Database,
    ) -> Result<Self, AppError> {
        let credentials =
            TenantLlmManager::get_credentials(user_id, tenant_id, provider, database).await?;

        Self::from_credentials(credentials)
    }

    /// Create a provider from pre-fetched credentials
    ///
    /// # Errors
    ///
    /// Returns an error if the provider type is not supported.
    pub fn from_credentials(credentials: LlmCredentials) -> Result<Self, AppError> {
        info!(
            "Creating {} provider from {} credentials",
            credentials.provider, credentials.source
        );

        match credentials.provider {
            TenantLlmProvider::Gemini => {
                let mut provider = GeminiProvider::new(&credentials.api_key)?;
                if let Some(model) = credentials.default_model {
                    provider = provider.with_default_model(model);
                }
                Ok(Self::Gemini(provider))
            }
            TenantLlmProvider::Groq => Ok(Self::Groq(GroqProvider::new(credentials.api_key))),
            TenantLlmProvider::Local => {
                use super::OpenAiCompatibleConfig;
                let base_url = credentials
                    .base_url
                    .unwrap_or_else(|| "http://localhost:11434/v1".to_owned());
                let model = credentials
                    .default_model
                    .unwrap_or_else(|| "qwen2.5:14b-instruct".to_owned());
                let config = OpenAiCompatibleConfig {
                    base_url,
                    api_key: if credentials.api_key.is_empty() {
                        None
                    } else {
                        Some(credentials.api_key)
                    },
                    default_model: model.clone(),
                    fallback_model: model,
                    provider_name: "local".to_owned(),
                    display_name: "Local LLM".to_owned(),
                    capabilities: LlmCapabilities::STREAMING
                        | LlmCapabilities::FUNCTION_CALLING
                        | LlmCapabilities::SYSTEM_MESSAGES,
                };
                let provider = OpenAiCompatibleProvider::new(config)?;
                Ok(Self::Local(provider))
            }
            TenantLlmProvider::OpenAi | TenantLlmProvider::Anthropic => {
                Err(AppError::config(format!(
                    "{} provider is not yet supported. Use Gemini, Groq, or Local.",
                    credentials.provider
                )))
            }
        }
    }

    /// Create a Gemini provider with a specific API key
    ///
    /// Use this when you have already resolved the API key from tenant/user credentials.
    ///
    /// # Errors
    ///
    /// Returns an error if LLM model config is not set in environment.
    pub fn gemini_with_key(api_key: &str) -> Result<Self, AppError> {
        Ok(Self::Gemini(GeminiProvider::new(api_key)?))
    }

    /// Create a Groq provider with a specific API key
    ///
    /// Use this when you have already resolved the API key from tenant/user credentials.
    #[must_use]
    pub fn groq_with_key(api_key: String) -> Self {
        Self::Groq(GroqProvider::new(api_key))
    }

    /// Get the provider type
    #[must_use]
    pub const fn provider_type(&self) -> LlmProviderType {
        match self {
            Self::Gemini(_) => LlmProviderType::Gemini,
            Self::Groq(_) => LlmProviderType::Groq,
            Self::Local(_) => LlmProviderType::Local,
        }
    }

    /// Check if this provider supports tool calling
    #[must_use]
    pub fn supports_tool_calling(&self) -> bool {
        self.capabilities().supports_function_calling()
    }

    /// Perform a chat completion with tool/function calling support
    ///
    /// Gemini, Groq, and Local providers all support native function/tool calling
    /// via their respective APIs (Gemini native, Groq/Local OpenAI-compatible).
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
            Self::Local(provider) => provider.complete_with_tools(request, tools).await,
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
            Self::Local(p) => p.name(),
        }
    }

    /// Get provider display name
    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Gemini(p) => p.display_name(),
            Self::Groq(p) => p.display_name(),
            Self::Local(p) => p.display_name(),
        }
    }

    /// Get provider capabilities
    #[must_use]
    pub fn capabilities(&self) -> LlmCapabilities {
        match self {
            Self::Gemini(p) => p.capabilities(),
            Self::Groq(p) => p.capabilities(),
            Self::Local(p) => p.capabilities(),
        }
    }

    /// Get default model
    #[must_use]
    pub fn default_model(&self) -> &str {
        match self {
            Self::Gemini(p) => p.default_model(),
            Self::Groq(p) => p.default_model(),
            Self::Local(p) => p.default_model(),
        }
    }

    /// Get available models
    #[must_use]
    pub fn available_models(&self) -> &'static [&'static str] {
        match self {
            Self::Gemini(p) => p.available_models(),
            Self::Groq(p) => p.available_models(),
            Self::Local(p) => p.available_models(),
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
            Self::Local(p) => p.complete(request).await,
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
            Self::Local(p) => p.complete_stream(request).await,
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
            Self::Local(p) => p.health_check().await,
        }
    }
}

impl fmt::Debug for ChatProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gemini(_) => f.debug_tuple("ChatProvider::Gemini").finish(),
            Self::Groq(_) => f.debug_tuple("ChatProvider::Groq").finish(),
            Self::Local(_) => f.debug_tuple("ChatProvider::Local").finish(),
        }
    }
}

// Implement LlmProvider trait for ChatProvider to enable trait object usage
#[async_trait::async_trait]
impl LlmProvider for ChatProvider {
    fn name(&self) -> &'static str {
        match self {
            Self::Gemini(p) => p.name(),
            Self::Groq(p) => p.name(),
            Self::Local(p) => p.name(),
        }
    }

    fn display_name(&self) -> &'static str {
        match self {
            Self::Gemini(p) => p.display_name(),
            Self::Groq(p) => p.display_name(),
            Self::Local(p) => p.display_name(),
        }
    }

    fn capabilities(&self) -> LlmCapabilities {
        match self {
            Self::Gemini(p) => p.capabilities(),
            Self::Groq(p) => p.capabilities(),
            Self::Local(p) => p.capabilities(),
        }
    }

    fn default_model(&self) -> &str {
        match self {
            Self::Gemini(p) => p.default_model(),
            Self::Groq(p) => p.default_model(),
            Self::Local(p) => p.default_model(),
        }
    }

    fn available_models(&self) -> &'static [&'static str] {
        match self {
            Self::Gemini(p) => p.available_models(),
            Self::Groq(p) => p.available_models(),
            Self::Local(p) => p.available_models(),
        }
    }

    async fn complete(&self, request: &ChatRequest) -> Result<ChatResponse, AppError> {
        match self {
            Self::Gemini(p) => p.complete(request).await,
            Self::Groq(p) => p.complete(request).await,
            Self::Local(p) => p.complete(request).await,
        }
    }

    async fn complete_stream(&self, request: &ChatRequest) -> Result<ChatStream, AppError> {
        match self {
            Self::Gemini(p) => p.complete_stream(request).await,
            Self::Groq(p) => p.complete_stream(request).await,
            Self::Local(p) => p.complete_stream(request).await,
        }
    }

    async fn health_check(&self) -> Result<bool, AppError> {
        match self {
            Self::Gemini(p) => p.health_check().await,
            Self::Groq(p) => p.health_check().await,
            Self::Local(p) => p.health_check().await,
        }
    }
}
