<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# LLM Provider Integration

This document describes Pierre's LLM (Large Language Model) provider abstraction layer, which enables pluggable AI model integration with streaming support for the chat functionality.

## Overview

The LLM module provides a trait-based abstraction that allows Pierre to integrate with multiple AI providers (Gemini, OpenAI, Ollama, etc.) through a unified interface. The design mirrors the fitness provider SPI pattern for consistency.

```
┌─────────────────────────────────────────────────────────────────┐
│                    LlmProviderRegistry                          │
│              Manages multiple LLM providers                     │
└────────────────────────────┬────────────────────────────────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
         ▼                   ▼                   ▼
   ┌───────────┐      ┌───────────┐      ┌───────────┐
   │  Gemini   │      │  OpenAI   │      │  Ollama   │
   │ Provider  │      │ Provider  │      │ Provider  │
   └─────┬─────┘      └─────┬─────┘      └─────┬─────┘
         │                  │                   │
         └──────────────────┴───────────────────┘
                           │
                           ▼
               ┌───────────────────────┐
               │   LlmProvider Trait   │
               │   (shared interface)  │
               └───────────────────────┘
```

## Configuration

### Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `GEMINI_API_KEY` | Google Gemini API key | Yes (for Gemini) |

### Supported Models

#### Gemini (Default Provider)

| Model | Description | Default |
|-------|-------------|---------|
| `gemini-2.0-flash-exp` | Latest experimental flash model | ✓ |
| `gemini-1.5-pro` | Production-ready pro model | |
| `gemini-1.5-flash` | Fast, efficient model | |
| `gemini-1.0-pro` | Legacy pro model | |

## Quick Start

### Basic Usage

```rust
use pierre_mcp_server::llm::{
    GeminiProvider, LlmProvider, ChatMessage, ChatRequest,
};

// Create provider from environment variable
let provider = GeminiProvider::from_env()?;

// Build a chat request
let request = ChatRequest::new(vec![
    ChatMessage::system("You are a helpful fitness assistant."),
    ChatMessage::user("What's a good warm-up routine?"),
])
.with_temperature(0.7)
.with_max_tokens(1000);

// Get a response
let response = provider.complete(&request).await?;
println!("{}", response.content);
```

### Streaming Responses

```rust
use futures_util::StreamExt;

let request = ChatRequest::new(vec![
    ChatMessage::user("Explain the benefits of interval training"),
])
.with_streaming();

let mut stream = provider.complete_stream(&request).await?;

while let Some(chunk) = stream.next().await {
    match chunk {
        Ok(chunk) => {
            print!("{}", chunk.delta);
            if chunk.is_final {
                println!("\n[Done]");
            }
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}
```

## API Reference

### LlmCapabilities

Bitflags indicating provider features:

| Flag | Description |
|------|-------------|
| `STREAMING` | Supports streaming responses |
| `FUNCTION_CALLING` | Supports function/tool calling |
| `VISION` | Supports image input |
| `JSON_MODE` | Supports structured JSON output |
| `SYSTEM_MESSAGES` | Supports system role messages |

```rust
// Check capabilities
let caps = provider.capabilities();
if caps.supports_streaming() {
    // Use streaming API
}
```

### ChatMessage

Message structure for conversations:

```rust
// Constructor methods
let system = ChatMessage::system("You are helpful");
let user = ChatMessage::user("Hello!");
let assistant = ChatMessage::assistant("Hi there!");
```

### ChatRequest

Request configuration with builder pattern:

```rust
let request = ChatRequest::new(messages)
    .with_model("gemini-1.5-pro")    // Override default model
    .with_temperature(0.7)            // 0.0 to 1.0
    .with_max_tokens(2000)            // Max output tokens
    .with_streaming();                // Enable streaming
```

### ChatResponse

Response structure:

| Field | Type | Description |
|-------|------|-------------|
| `content` | `String` | Generated text |
| `model` | `String` | Model used |
| `usage` | `Option<TokenUsage>` | Token counts |
| `finish_reason` | `Option<String>` | Why generation stopped |

### StreamChunk

Streaming chunk structure:

| Field | Type | Description |
|-------|------|-------------|
| `delta` | `String` | Incremental text |
| `is_final` | `bool` | Whether this is the last chunk |
| `finish_reason` | `Option<String>` | Reason if final |

## Provider Registry

The `LlmProviderRegistry` manages multiple providers:

```rust
use pierre_mcp_server::llm::LlmProviderRegistry;

let mut registry = LlmProviderRegistry::new();

// Register providers
registry.register(Box::new(GeminiProvider::from_env()?));
// registry.register(Box::new(OpenAIProvider::from_env()?));

// Set default
registry.set_default("gemini")?;

// Get provider by name
let provider = registry.get("gemini");

// List all registered
let names: Vec<&str> = registry.list();
```

## Adding New Providers

To implement a new LLM provider:

1. **Implement the trait**:

```rust
use async_trait::async_trait;
use pierre_mcp_server::llm::{
    LlmProvider, LlmCapabilities, ChatRequest, ChatResponse,
    ChatStream, AppError,
};

pub struct MyProvider {
    api_key: String,
    // ...
}

#[async_trait]
impl LlmProvider for MyProvider {
    fn name(&self) -> &'static str {
        "myprovider"
    }

    fn display_name(&self) -> &'static str {
        "My Custom Provider"
    }

    fn capabilities(&self) -> LlmCapabilities {
        LlmCapabilities::STREAMING | LlmCapabilities::SYSTEM_MESSAGES
    }

    fn default_model(&self) -> &'static str {
        "my-model-v1"
    }

    fn available_models(&self) -> &'static [&'static str] {
        &["my-model-v1", "my-model-v2"]
    }

    async fn complete(&self, request: &ChatRequest) -> Result<ChatResponse, AppError> {
        // Implementation
    }

    async fn complete_stream(&self, request: &ChatRequest) -> Result<ChatStream, AppError> {
        // Implementation
    }

    async fn health_check(&self) -> Result<bool, AppError> {
        // Implementation
    }
}
```

2. **Register the provider**:

```rust
registry.register(Box::new(MyProvider::new(api_key)));
```

## Error Handling

All provider methods return `Result<T, AppError>`:

```rust
match provider.complete(&request).await {
    Ok(response) => println!("{}", response.content),
    Err(AppError { code, message, .. }) => {
        match code {
            ErrorCode::RateLimitExceeded => // Handle rate limit
            ErrorCode::AuthenticationFailed => // Handle auth error
            _ => // Handle other errors
        }
    }
}
```

## Testing

Run LLM-specific tests:

```bash
# Unit tests
cargo test --test llm_test

# With output
cargo test --test llm_test -- --nocapture
```

## See Also

- [Chapter 26: LLM Provider Architecture](tutorial/chapter-26-llm-providers.md)
- [Configuration Guide](configuration.md)
- [Error Reference](tutorial/appendix-h-error-reference.md)
