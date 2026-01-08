// ABOUTME: LLM provider abstraction layer for pluggable AI model integration
// ABOUTME: Defines the contract for LLM providers (Gemini, OpenAI, etc.) with streaming support
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # LLM Provider Service Provider Interface
//!
//! This module defines the contract that LLM providers must implement to integrate
//! with the Pierre chat system. The design mirrors the fitness provider SPI pattern
//! for consistency and extensibility.
//!
//! ## Key Concepts
//!
//! - **`LlmCapabilities`**: Bitflags describing provider features (streaming, function calling, etc.)
//! - **`LlmProvider`**: Async trait for chat completion with streaming support
//! - **`ChatMessage`**: Role-based message structure for conversations
//! - **`ChatRequest`**: Request configuration including model, temperature, etc.
//!
//! ## Example: Using a Provider
//!
//! ```rust,no_run
//! use pierre_mcp_server::llm::{
//!     LlmProvider, ChatMessage, ChatRequest, MessageRole,
//! };
//!
//! async fn example(provider: &dyn LlmProvider) {
//!     let messages = vec![
//!         ChatMessage::system("You are a helpful fitness assistant."),
//!         ChatMessage::user("What's a good warm-up routine?"),
//!     ];
//!
//!     let request = ChatRequest::new(messages);
//!     let response = provider.complete(&request).await;
//! }
//! ```

mod gemini;
mod groq;
mod openai_compatible;
pub mod prompts;
mod provider;

pub use gemini::{
    ChatResponseWithTools, FunctionCall, FunctionDeclaration, FunctionResponse, GeminiProvider,
    Tool,
};
pub use groq::GroqProvider;
pub use openai_compatible::{OpenAiCompatibleConfig, OpenAiCompatibleProvider};
pub use prompts::get_pierre_system_prompt;
pub use provider::ChatProvider;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tokio_stream::Stream;

use crate::errors::AppError;

// ============================================================================
// Capability Flags
// ============================================================================

bitflags::bitflags! {
    /// LLM provider capability flags using bitflags for efficient storage
    ///
    /// Indicates which features a provider supports. Used by the system to
    /// select appropriate providers and configure request handling.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
    pub struct LlmCapabilities: u8 {
        /// Provider supports streaming responses
        const STREAMING = 0b0000_0001;
        /// Provider supports function/tool calling
        const FUNCTION_CALLING = 0b0000_0010;
        /// Provider supports vision/image input
        const VISION = 0b0000_0100;
        /// Provider supports JSON mode output
        const JSON_MODE = 0b0000_1000;
        /// Provider supports system messages
        const SYSTEM_MESSAGES = 0b0001_0000;
    }
}

impl LlmCapabilities {
    /// Create capabilities for a basic text-only provider
    #[must_use]
    pub const fn text_only() -> Self {
        Self::STREAMING.union(Self::SYSTEM_MESSAGES)
    }

    /// Create capabilities for a full-featured provider (like Gemini Pro)
    #[must_use]
    pub const fn full_featured() -> Self {
        Self::STREAMING
            .union(Self::FUNCTION_CALLING)
            .union(Self::VISION)
            .union(Self::JSON_MODE)
            .union(Self::SYSTEM_MESSAGES)
    }

    /// Check if streaming is supported
    #[must_use]
    pub const fn supports_streaming(&self) -> bool {
        self.contains(Self::STREAMING)
    }

    /// Check if function calling is supported
    #[must_use]
    pub const fn supports_function_calling(&self) -> bool {
        self.contains(Self::FUNCTION_CALLING)
    }

    /// Check if vision is supported
    #[must_use]
    pub const fn supports_vision(&self) -> bool {
        self.contains(Self::VISION)
    }

    /// Check if JSON mode is supported
    #[must_use]
    pub const fn supports_json_mode(&self) -> bool {
        self.contains(Self::JSON_MODE)
    }

    /// Check if system messages are supported
    #[must_use]
    pub const fn supports_system_messages(&self) -> bool {
        self.contains(Self::SYSTEM_MESSAGES)
    }
}

// ============================================================================
// Message Types
// ============================================================================

/// Role of a message in the conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// System instruction message
    System,
    /// User input message
    User,
    /// Assistant response message
    Assistant,
}

impl MessageRole {
    /// Convert to string representation for API calls
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

/// A single message in a chat conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Role of the message sender
    pub role: MessageRole,
    /// Content of the message
    pub content: String,
}

impl ChatMessage {
    /// Create a new chat message
    #[must_use]
    pub fn new(role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    /// Create a system message
    #[must_use]
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(MessageRole::System, content)
    }

    /// Create a user message
    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(MessageRole::User, content)
    }

    /// Create an assistant message
    #[must_use]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Assistant, content)
    }
}

// ============================================================================
// Request/Response Types
// ============================================================================

/// Configuration for a chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// Conversation messages
    pub messages: Vec<ChatMessage>,
    /// Model identifier (provider-specific)
    pub model: Option<String>,
    /// Temperature for response randomness (0.0 - 2.0)
    pub temperature: Option<f32>,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    /// Whether to stream the response
    pub stream: bool,
}

impl ChatRequest {
    /// Create a new chat request with messages
    #[must_use]
    pub const fn new(messages: Vec<ChatMessage>) -> Self {
        Self {
            messages,
            model: None,
            temperature: None,
            max_tokens: None,
            stream: false,
        }
    }

    /// Set the model to use
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the temperature
    #[must_use]
    pub const fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set the maximum tokens
    #[must_use]
    pub const fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Enable streaming
    #[must_use]
    pub const fn with_streaming(mut self) -> Self {
        self.stream = true;
        self
    }
}

/// Response from a chat completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Generated message content
    pub content: String,
    /// Model used for generation
    pub model: String,
    /// Token usage statistics
    pub usage: Option<TokenUsage>,
    /// Finish reason (stop, length, etc.)
    pub finish_reason: Option<String>,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Number of tokens in the prompt
    pub prompt_tokens: u32,
    /// Number of tokens in the completion
    pub completion_tokens: u32,
    /// Total tokens used
    pub total_tokens: u32,
}

/// A chunk of a streaming response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Content delta for this chunk
    pub delta: String,
    /// Whether this is the final chunk
    pub is_final: bool,
    /// Finish reason if final
    pub finish_reason: Option<String>,
}

/// Stream type for chat completion responses
pub type ChatStream = Pin<Box<dyn Stream<Item = Result<StreamChunk, AppError>> + Send>>;

// ============================================================================
// Provider Trait
// ============================================================================

/// LLM provider trait for chat completion
///
/// Implement this trait to add a new LLM provider to Pierre.
/// The design follows the async trait pattern for compatibility
/// with tokio-based async runtime.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Unique provider identifier (e.g., "gemini", "openai", "ollama")
    fn name(&self) -> &'static str;

    /// Human-readable display name for the provider
    fn display_name(&self) -> &'static str;

    /// Provider capabilities (streaming, function calling, etc.)
    fn capabilities(&self) -> LlmCapabilities;

    /// Default model to use if not specified in request
    fn default_model(&self) -> &str;

    /// Available models for this provider
    fn available_models(&self) -> &'static [&'static str];

    /// Perform a chat completion (non-streaming)
    async fn complete(&self, request: &ChatRequest) -> Result<ChatResponse, AppError>;

    /// Perform a streaming chat completion
    ///
    /// Returns a stream of chunks that can be consumed incrementally.
    /// Falls back to non-streaming if not supported.
    async fn complete_stream(&self, request: &ChatRequest) -> Result<ChatStream, AppError>;

    /// Check if the provider is healthy and API key is valid
    async fn health_check(&self) -> Result<bool, AppError>;
}

// ============================================================================
// Provider Registry
// ============================================================================

/// Registry for LLM providers
///
/// Manages available providers and provides lookup by name.
pub struct LlmProviderRegistry {
    providers: Vec<Box<dyn LlmProvider>>,
}

impl LlmProviderRegistry {
    /// Create a new empty registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Register a provider
    pub fn register(&mut self, provider: Box<dyn LlmProvider>) {
        self.providers.push(provider);
    }

    /// Get a provider by name
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&dyn LlmProvider> {
        self.providers
            .iter()
            .find(|p| p.name() == name)
            .map(AsRef::as_ref)
    }

    /// List all registered providers
    #[must_use]
    pub fn list(&self) -> Vec<&dyn LlmProvider> {
        self.providers.iter().map(AsRef::as_ref).collect()
    }

    /// Get the default provider (first registered)
    #[must_use]
    pub fn default_provider(&self) -> Option<&dyn LlmProvider> {
        self.providers.first().map(AsRef::as_ref)
    }
}

impl Default for LlmProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
