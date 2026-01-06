# LLM Provider Integration

This document describes Pierre's LLM (Large Language Model) provider abstraction layer, which enables pluggable AI model integration with streaming support for chat functionality and recipe generation.

## Overview

The LLM module provides a trait-based abstraction that allows Pierre to integrate with multiple AI providers through a unified interface. The design mirrors the fitness provider SPI pattern for consistency.

```
┌─────────────────────────────────────────────────────────────────┐
│                      ChatProvider                                │
│            Runtime provider selector (from env)                  │
│              PIERRE_LLM_PROVIDER=groq|gemini                     │
└────────────────────────────────┬────────────────────────────────┘
                                 │
              ┌──────────────────┴──────────────────┐
              │                                     │
              ▼                                     ▼
       ┌─────────────┐                       ┌─────────────┐
       │   Gemini    │                       │    Groq     │
       │  Provider   │                       │  Provider   │
       │  (vision,   │                       │  (fast LPU  │
       │   tools)    │                       │  inference) │
       └──────┬──────┘                       └──────┬──────┘
              │                                     │
              └─────────────────┬───────────────────┘
                                │
                                ▼
                   ┌───────────────────────┐
                   │   LlmProvider Trait   │
                   │   (shared interface)  │
                   └───────────────────────┘
```

## Quick Start

```bash
# Option 1: Use Groq (default, cost-effective)
export GROQ_API_KEY="your-groq-api-key"
export PIERRE_LLM_PROVIDER=groq  # optional, groq is default

# Option 2: Use Gemini (full-featured with vision)
export GEMINI_API_KEY="your-gemini-api-key"
export PIERRE_LLM_PROVIDER=gemini
```

## Configuration

### Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `PIERRE_LLM_PROVIDER` | Provider selector: `groq` (default) or `gemini` | No |
| `GROQ_API_KEY` | Groq API key from [console.groq.com](https://console.groq.com/keys) | Yes (for Groq) |
| `GEMINI_API_KEY` | Google Gemini API key from [AI Studio](https://makersuite.google.com/app/apikey) | Yes (for Gemini) |

### Supported Models

#### Groq (Default Provider)

Groq provides LPU-accelerated inference for open-source models with extremely fast response times.

| Model | Description | Default |
|-------|-------------|---------|
| `llama-3.3-70b-versatile` | High-quality general purpose | ✓ |
| `llama-3.1-8b-instant` | Fast responses for simple tasks | |
| `llama-3.1-70b-versatile` | Versatile 70B model | |
| `mixtral-8x7b-32768` | Long context window (32K tokens) | |
| `gemma2-9b-it` | Google's Gemma 2 instruction-tuned | |

**Rate Limits**: Free tier has 12,000 tokens-per-minute limit. For tool-heavy workflows, consider Gemini.

#### Gemini

Google's Gemini models with full vision and function calling support.

| Model | Description | Default |
|-------|-------------|---------|
| `gemini-2.5-flash` | Latest fast model with improved capabilities | ✓ |
| `gemini-2.0-flash-exp` | Experimental fast model | |
| `gemini-1.5-pro` | Advanced reasoning capabilities | |
| `gemini-1.5-flash` | Balanced performance and cost | |
| `gemini-1.0-pro` | Legacy pro model | |

### Provider Capabilities

| Capability | Groq | Gemini |
|------------|------|--------|
| Streaming | ✓ | ✓ |
| Function/Tool Calling | ✓ | ✓ |
| Vision/Image Input | ✗ | ✓ |
| JSON Mode | ✓ | ✓ |
| System Messages | ✓ | ✓ |

## Basic Usage

### Using ChatProvider (Recommended)

The `ChatProvider` enum automatically selects the provider based on environment configuration:

```rust
use pierre_mcp_server::llm::{ChatProvider, ChatMessage, ChatRequest};

// Create provider from environment (reads PIERRE_LLM_PROVIDER)
let provider = ChatProvider::from_env()?;

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

### Explicit Provider Selection

```rust
// Force Gemini
let provider = ChatProvider::gemini()?;

// Force Groq
let provider = ChatProvider::groq()?;
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

### Tool/Function Calling

Both providers support tool calling for structured interactions:

```rust
use pierre_mcp_server::llm::{Tool, FunctionDeclaration};

let tools = vec![Tool {
    function_declarations: vec![FunctionDeclaration {
        name: "get_weather".to_string(),
        description: "Get current weather for a location".to_string(),
        parameters: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "location": {"type": "string"}
            },
            "required": ["location"]
        })),
    }],
}];

let response = provider.complete_with_tools(&request, Some(tools)).await?;

if response.has_function_calls() {
    for call in response.function_calls.unwrap() {
        println!("Call function: {} with args: {}", call.name, call.args);
    }
}
```

## Recipe Generation Integration

Pierre uses LLM providers for the "Combat des Chefs" recipe generation architecture. The workflow differs based on whether the client has LLM capabilities:

### LLM Clients (Claude, ChatGPT, etc.)

When an LLM client connects to Pierre, it generates recipes itself:

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  LLM Client  │────▶│ Pierre MCP   │────▶│    USDA      │
│  (Claude)    │     │   Server     │     │  Database    │
└──────────────┘     └──────────────┘     └──────────────┘
       │                    │                    │
       │  1. get_recipe_    │                    │
       │     constraints    │                    │
       │───────────────────▶│                    │
       │                    │                    │
       │  2. Returns macro  │                    │
       │     targets, hints │                    │
       │◀───────────────────│                    │
       │                    │                    │
       │  [LLM generates    │                    │
       │   recipe locally]  │                    │
       │                    │                    │
       │  3. validate_      │                    │
       │     recipe         │                    │
       │───────────────────▶│                    │
       │                    │  Lookup nutrition  │
       │                    │───────────────────▶│
       │                    │◀───────────────────│
       │  4. Validation     │                    │
       │     result + macros│                    │
       │◀───────────────────│                    │
       │                    │                    │
       │  5. save_recipe    │                    │
       │───────────────────▶│                    │
```

### Non-LLM Clients

For clients without LLM capabilities, Pierre uses its internal LLM (via `ChatProvider`):

```rust
// The suggest_recipe tool uses Pierre's configured LLM
let provider = ChatProvider::from_env()?;
let recipe = generate_recipe_with_llm(&provider, constraints).await?;
```

### Recipe Tools

| Tool | Description |
|------|-------------|
| `get_recipe_constraints` | Get macro targets and prompt hints for LLM recipe generation |
| `validate_recipe` | Validate recipe nutrition via USDA FoodData Central |
| `suggest_recipe` | Uses Pierre's internal LLM to generate recipes |
| `save_recipe` | Save validated recipes to user collection |
| `list_recipes` | List user's saved recipes |
| `get_recipe` | Get recipe by ID |
| `search_recipes` | Search recipes by name, tags, or ingredients |

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

## Module Structure

```
src/llm/
├── mod.rs          # Trait definitions, types, registry, exports
├── provider.rs     # ChatProvider enum (runtime selector)
├── gemini.rs       # Google Gemini implementation
├── groq.rs         # Groq LPU implementation
└── prompts/
    └── mod.rs      # System prompts (pierre_system.md)
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

2. **Add to ChatProvider enum** in `src/llm/provider.rs`:

```rust
pub enum ChatProvider {
    Gemini(GeminiProvider),
    Groq(GroqProvider),
    MyProvider(MyProvider),  // Add variant
}
```

3. **Update environment config** in `src/config/environment.rs`

4. **Register tests** in `tests/llm_test.rs`

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
- [Tools Reference - Recipe Management](tools-reference.md#recipe-management)
- [Configuration Guide](configuration.md)
- [Error Reference](tutorial/appendix-h-error-reference.md)
