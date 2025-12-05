<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 26: LLM Provider Architecture

This chapter explores Pierre's LLM (Large Language Model) provider abstraction layer, which enables pluggable AI model integration for chat functionality. The architecture mirrors the fitness provider SPI pattern, providing a consistent approach to external service integration.

## What You'll Learn

- Trait-based LLM provider abstraction
- Capability detection with bitflags
- Implementing the Gemini provider
- Streaming responses with SSE
- Provider registry pattern
- Adding custom LLM providers
- Error handling best practices
- Testing LLM integrations

## Architecture Overview

The LLM module follows the same pluggable architecture pattern used for fitness providers. This design allows runtime registration of multiple AI providers while maintaining a consistent interface.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Chat System                                         │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │                    LlmProviderRegistry                               │  │
│   │              Default provider + named lookup                         │  │
│   └────────────────────────────┬────────────────────────────────────────┘  │
│                                │                                            │
│         ┌──────────────────────┼──────────────────────┐                    │
│         │                      │                      │                    │
│         ▼                      ▼                      ▼                    │
│   ┌───────────┐         ┌───────────┐         ┌───────────┐               │
│   │  Gemini   │         │  OpenAI   │         │  Ollama   │               │
│   │ Provider  │         │ Provider  │         │ Provider  │               │
│   │           │         │  (future) │         │  (future) │               │
│   └─────┬─────┘         └─────┬─────┘         └─────┬─────┘               │
│         │                     │                     │                      │
│         └─────────────────────┴─────────────────────┘                      │
│                               │                                            │
│                               ▼                                            │
│               ┌───────────────────────────────┐                            │
│               │      LlmProvider Trait        │                            │
│               │  ┌─────────────────────────┐  │                            │
│               │  │ + name()                │  │                            │
│               │  │ + capabilities()        │  │                            │
│               │  │ + complete()            │  │                            │
│               │  │ + complete_stream()     │  │                            │
│               │  │ + health_check()        │  │                            │
│               │  └─────────────────────────┘  │                            │
│               └───────────────────────────────┘                            │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Module Structure

```
src/llm/
├── mod.rs      # Trait definitions, types, registry
└── gemini.rs   # Google Gemini implementation
```

**Source**: `src/lib.rs`
```rust
/// LLM provider abstraction for AI chat integration
pub mod llm;
```

## Capability Detection with Bitflags

LLM providers have varying capabilities. We use bitflags for efficient storage and querying:

**Source**: `src/llm/mod.rs`
```rust
bitflags::bitflags! {
    /// LLM provider capability flags using bitflags for efficient storage
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
```

**Helper methods**:
```rust
impl LlmCapabilities {
    /// Create capabilities for a basic text-only provider
    pub const fn text_only() -> Self {
        Self::STREAMING.union(Self::SYSTEM_MESSAGES)
    }

    /// Create capabilities for a full-featured provider
    pub const fn full_featured() -> Self {
        Self::STREAMING
            .union(Self::FUNCTION_CALLING)
            .union(Self::VISION)
            .union(Self::JSON_MODE)
            .union(Self::SYSTEM_MESSAGES)
    }

    /// Check if streaming is supported
    pub const fn supports_streaming(&self) -> bool {
        self.contains(Self::STREAMING)
    }
}
```

**Usage**:
```rust
let caps = provider.capabilities();

if caps.supports_streaming() && caps.supports_function_calling() {
    // Use advanced features
} else if caps.supports_streaming() {
    // Use basic streaming
}
```

## The LlmProvider Trait

The core abstraction that all providers implement:

**Source**: `src/llm/mod.rs`
```rust
/// Type alias for boxed stream of chat chunks
pub type ChatStream = Pin<Box<dyn Stream<Item = Result<StreamChunk, AppError>> + Send>>;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Unique provider identifier (e.g., "gemini", "openai")
    fn name(&self) -> &'static str;

    /// Human-readable display name for the provider
    fn display_name(&self) -> &'static str;

    /// Provider capabilities (streaming, function calling, etc.)
    fn capabilities(&self) -> LlmCapabilities;

    /// Default model to use if not specified in request
    fn default_model(&self) -> &'static str;

    /// Available models for this provider
    fn available_models(&self) -> &'static [&'static str];

    /// Perform a chat completion (non-streaming)
    async fn complete(&self, request: &ChatRequest) -> Result<ChatResponse, AppError>;

    /// Perform a streaming chat completion
    async fn complete_stream(&self, request: &ChatRequest) -> Result<ChatStream, AppError>;

    /// Check if the provider is healthy and reachable
    async fn health_check(&self) -> Result<bool, AppError>;
}
```

## Message Types

### MessageRole

Enum representing conversation roles:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

impl MessageRole {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}
```

### ChatMessage

Individual message in a conversation:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

impl ChatMessage {
    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
        }
    }

    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }
}
```

### ChatRequest (Builder Pattern)

Request configuration using the builder pattern with const fn methods:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
}

impl ChatRequest {
    /// Create a new chat request with messages
    pub const fn new(messages: Vec<ChatMessage>) -> Self {
        Self {
            messages,
            model: None,
            temperature: None,
            max_tokens: None,
            stream: false,
        }
    }

    /// Set the model to use (consuming builder)
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the temperature (const fn - no allocation)
    pub const fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set the maximum tokens (const fn)
    pub const fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Enable streaming (const fn)
    pub const fn with_streaming(mut self) -> Self {
        self.stream = true;
        self
    }
}
```

## Gemini Provider Implementation

The Gemini provider demonstrates the implementation pattern:

**Source**: `src/llm/gemini.rs`

### Structure

```rust
/// Environment variable for Gemini API key
const GEMINI_API_KEY_ENV: &str = "GEMINI_API_KEY";

/// Default model to use
const DEFAULT_MODEL: &str = "gemini-2.0-flash-exp";

/// API base URL
const API_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Available Gemini models
const AVAILABLE_MODELS: &[&str] = &[
    "gemini-2.0-flash-exp",
    "gemini-1.5-pro",
    "gemini-1.5-flash",
    "gemini-1.0-pro",
];

pub struct GeminiProvider {
    api_key: String,
    client: Client,
    default_model: String,
}
```

### API Request/Response Types

```rust
/// Gemini API request format
#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiContent>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    parts: Vec<ContentPart>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ContentPart {
    text: String,
}
```

### Message Conversion

Gemini handles system messages differently - via a separate `system_instruction` field:

```rust
impl GeminiProvider {
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

    /// Convert our message role to Gemini's role format
    const fn convert_role(role: MessageRole) -> &'static str {
        match role {
            MessageRole::System | MessageRole::User => "user",
            MessageRole::Assistant => "model",
        }
    }
}
```

### Non-Streaming Implementation

```rust
#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn complete(&self, request: &ChatRequest) -> Result<ChatResponse, AppError> {
        let model = request.model.as_deref().unwrap_or(&self.default_model);
        let url = self.build_url(model, "generateContent");

        let gemini_request = Self::build_gemini_request(request);

        let response = self
            .client
            .post(&url)
            .json(&gemini_request)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("HTTP request failed: {e}")))?;

        let status = response.status();
        let gemini_response: GeminiResponse = response
            .json()
            .await
            .map_err(|e| AppError::internal(format!("Failed to parse response: {e}")))?;

        // Check for API errors
        if let Some(error) = gemini_response.error {
            return Err(AppError::internal(format!("Gemini API error: {}", error.message)));
        }

        let content = Self::extract_content(&gemini_response)?;
        let usage = gemini_response
            .usage_metadata
            .as_ref()
            .map(Self::convert_usage);

        Ok(ChatResponse {
            content,
            model: model.to_owned(),
            usage,
            finish_reason: /* ... */,
        })
    }
}
```

### Streaming with SSE

Gemini uses Server-Sent Events (SSE) for streaming:

```rust
async fn complete_stream(&self, request: &ChatRequest) -> Result<ChatStream, AppError> {
    let model = request.model.as_deref().unwrap_or(&self.default_model);
    let url = self.build_url(model, "streamGenerateContent");

    let gemini_request = Self::build_gemini_request(request);

    let response = self
        .client
        .post(&url)
        .query(&[("alt", "sse")])  // Request SSE format
        .json(&gemini_request)
        .send()
        .await?;

    // Create a stream from the SSE response
    let byte_stream = response.bytes_stream();

    let stream = byte_stream.filter_map(|result| async move {
        match result {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);

                // Parse SSE format: lines starting with "data: "
                for line in text.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if let Ok(response) = serde_json::from_str::<StreamingResponse>(data) {
                            // Extract text delta from response
                            if let Some(text) = /* extract text */ {
                                return Some(Ok(StreamChunk {
                                    delta: text,
                                    is_final: /* check finish_reason */,
                                    finish_reason: /* ... */,
                                }));
                            }
                        }
                    }
                }
                None
            }
            Err(e) => Some(Err(AppError::internal(format!("Stream error: {e}")))),
        }
    });

    Ok(Box::pin(stream))
}
```

### Debug Implementation (API Key Redaction)

Never expose API keys in logs:

```rust
impl std::fmt::Debug for GeminiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GeminiProvider")
            .field("default_model", &self.default_model)
            .field("api_key", &"[REDACTED]")
            // Omit `client` field as HTTP clients are not useful to debug
            .finish_non_exhaustive()
    }
}
```

## Provider Registry

The registry manages multiple providers with default selection:

```rust
pub struct LlmProviderRegistry {
    providers: HashMap<String, Box<dyn LlmProvider>>,
    default_provider: Option<String>,
}

impl LlmProviderRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            default_provider: None,
        }
    }

    /// Register a provider
    pub fn register(&mut self, provider: Box<dyn LlmProvider>) {
        let name = provider.name().to_string();
        if self.default_provider.is_none() {
            self.default_provider = Some(name.clone());
        }
        self.providers.insert(name, provider);
    }

    /// Get a provider by name
    pub fn get(&self, name: &str) -> Option<&dyn LlmProvider> {
        self.providers.get(name).map(|p| p.as_ref())
    }

    /// Get the default provider
    pub fn default_provider(&self) -> Option<&dyn LlmProvider> {
        self.default_provider
            .as_ref()
            .and_then(|name| self.get(name))
    }

    /// Set the default provider
    pub fn set_default(&mut self, name: &str) -> Result<(), AppError> {
        if self.providers.contains_key(name) {
            self.default_provider = Some(name.to_string());
            Ok(())
        } else {
            Err(AppError::not_found(format!("Provider '{name}' not found")))
        }
    }

    /// List all registered provider names
    pub fn list(&self) -> Vec<&str> {
        self.providers.keys().map(String::as_str).collect()
    }
}
```

## Error Handling

All LLM operations use structured error types:

```rust
// Good: Structured errors
return Err(AppError::config(format!(
    "{GEMINI_API_KEY_ENV} environment variable not set"
)));

return Err(AppError::internal(format!(
    "Gemini API error ({status}): {error_text}"
)));

return Err(AppError::internal("No content in Gemini response"));

// Bad: Never use anyhow! in production code
// return Err(anyhow!("API failed")); // FORBIDDEN
```

## Testing LLM Providers

Tests are in `tests/llm_test.rs` (not in src/ per project conventions):

```rust
#[test]
fn test_capabilities_full_featured() {
    let caps = LlmCapabilities::full_featured();
    assert!(caps.supports_streaming());
    assert!(caps.supports_function_calling());
    assert!(caps.supports_vision());
    assert!(caps.supports_json_mode());
    assert!(caps.supports_system_messages());
}

#[test]
fn test_gemini_debug_redacts_api_key() {
    let provider = GeminiProvider::new("super-secret-key");
    let debug_output = format!("{provider:?}");
    assert!(!debug_output.contains("super-secret-key"));
    assert!(debug_output.contains("[REDACTED]"));
}

#[test]
fn test_chat_request_builder() {
    let request = ChatRequest::new(vec![ChatMessage::user("Hello")])
        .with_model("gemini-pro")
        .with_temperature(0.7)
        .with_max_tokens(1000)
        .with_streaming();

    assert_eq!(request.model, Some("gemini-pro".to_string()));
    assert!(request.stream);
}
```

Run tests:
```bash
cargo test --test llm_test -- --nocapture
```

## Adding a New Provider

To add a new LLM provider (e.g., OpenAI):

1. **Create the provider file** (`src/llm/openai.rs`):

```rust
pub struct OpenAIProvider {
    api_key: String,
    client: Client,
    default_model: String,
}

#[async_trait]
impl LlmProvider for OpenAIProvider {
    fn name(&self) -> &'static str { "openai" }
    fn display_name(&self) -> &'static str { "OpenAI GPT" }
    // ... implement all trait methods
}
```

2. **Export from mod.rs**:

```rust
mod openai;
pub use openai::OpenAIProvider;
```

3. **Add tests** in `tests/llm_test.rs`

4. **Register in application startup**:

```rust
let mut registry = LlmProviderRegistry::new();
registry.register(Box::new(GeminiProvider::from_env()?));
registry.register(Box::new(OpenAIProvider::from_env()?));
```

## Best Practices

1. **API Key Security**: Always redact in Debug impls, never log
2. **Capability Checks**: Query capabilities before using features
3. **Timeout Handling**: Configure appropriate timeouts for HTTP clients
4. **Rate Limiting**: Respect provider rate limits
5. **Error Context**: Provide meaningful error messages
6. **Streaming**: Prefer streaming for long responses
7. **Model Selection**: Allow users to override default models

## Summary

The LLM provider architecture provides:

- **Pluggable Design**: Add providers without changing consumer code
- **Capability Detection**: Query features at runtime
- **Type Safety**: Structured messages and responses
- **Streaming Support**: SSE-based streaming responses
- **Registry Pattern**: Manage multiple providers
- **Security**: API key redaction built-in

## See Also

- [LLM Providers Reference](../llm-providers.md)
- [Chapter 17.5: Pluggable Provider Architecture](chapter-17.5-pluggable-providers.md)
- [Chapter 2: Error Handling](chapter-02-error-handling.md)
- [Appendix H: Error Reference](appendix-h-error-reference.md)
