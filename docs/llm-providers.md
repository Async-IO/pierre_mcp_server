<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# LLM Provider Integration

This document describes Pierre's LLM (Large Language Model) provider abstraction layer, which enables pluggable AI model integration with streaming support for chat functionality and recipe generation.

## Overview

The LLM module provides a trait-based abstraction that allows Pierre to integrate with multiple AI providers through a unified interface. The design mirrors the fitness provider SPI pattern for consistency.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                             ChatProvider                                     │
│                  Runtime provider selector (from env)                        │
│           PIERRE_LLM_PROVIDER=groq|gemini|local|ollama|vllm                 │
└──────────────────────────────────┬──────────────────────────────────────────┘
                                   │
           ┌───────────────────────┼───────────────────────┐
           │                       │                       │
           ▼                       ▼                       ▼
    ┌─────────────┐         ┌─────────────┐         ┌─────────────────┐
    │   Gemini    │         │    Groq     │         │ OpenAI-         │
    │  Provider   │         │  Provider   │         │ Compatible      │
    │  (vision,   │         │  (fast LPU  │         │ (Ollama, vLLM,  │
    │   tools)    │         │  inference) │         │  LocalAI)       │
    └──────┬──────┘         └──────┬──────┘         └────────┬────────┘
           │                       │                         │
           │                       │              ┌──────────┴──────────┐
           │                       │              │                     │
           │                       │         ┌────┴────┐          ┌────┴────┐
           │                       │         │ Ollama  │          │  vLLM   │
           │                       │         │localhost│          │localhost│
           │                       │         │ :11434  │          │ :8000   │
           └───────────────────────┴─────────┴────┬────┴──────────┴────┬────┘
                                                  │                    │
                                                  ▼                    ▼
                                         ┌───────────────────────────────────┐
                                         │      LlmProvider Trait            │
                                         │      (shared interface)           │
                                         └───────────────────────────────────┘
```

## Quick Start

### Option 1: Cloud Providers (No Setup Required)

```bash
# Groq (default, cost-effective, fast)
export GROQ_API_KEY="your-groq-api-key"
export PIERRE_LLM_PROVIDER=groq

# Gemini (full-featured with vision)
export GEMINI_API_KEY="your-gemini-api-key"
export PIERRE_LLM_PROVIDER=gemini
```

### Option 2: Local LLM (Privacy-First, No API Costs)

```bash
# Use local Ollama instance
export PIERRE_LLM_PROVIDER=local
export LOCAL_LLM_MODEL=qwen2.5:14b-instruct

# Start Pierre
./bin/start-server.sh
```

---

## Local LLM Setup Guide

Running a local LLM gives you complete privacy, no API costs, and works offline. This section covers setting up Ollama (recommended) on macOS.

### Hardware Requirements

| Model Size | RAM Required | GPU VRAM | Recommended Hardware |
|------------|--------------|----------|---------------------|
| 7B-8B (Q4) | 8GB+ | 8GB | MacBook Air M1/M2 16GB |
| 14B (Q4) | 12GB+ | 12GB | MacBook Air M2 24GB, MacBook Pro |
| 32B (Q4) | 20GB+ | 20-24GB | MacBook Pro M2/M3 Pro 32GB+ |
| 70B (Q4) | 40GB+ | 40-48GB | Mac Studio, High-end workstation |

**Example: Apple Silicon with 24GB unified memory:**
- ✅ Qwen 2.5 7B (~30 tokens/sec)
- ✅ Qwen 2.5 14B (~15-20 tokens/sec) **← Recommended**
- ⚠️ Qwen 2.5 32B (~5-8 tokens/sec, tight fit)

### Step 1: Install Ollama

```bash
# macOS (Homebrew)
brew install ollama

# Or download from https://ollama.ai/download
```

### Step 2: Start Ollama Server

```bash
# Start the Ollama service (runs in background)
ollama serve

# Verify it's running
curl http://localhost:11434/api/version
# Should return: {"version":"0.x.x"}
```

### Step 3: Pull a Model

**Recommended models for function calling:**

```bash
# Best for 24GB RAM (recommended)
ollama pull qwen2.5:14b-instruct

# Faster, lighter alternative
ollama pull qwen2.5:7b-instruct

# If you have 32GB+ RAM
ollama pull qwen2.5:32b-instruct

# Alternative: Llama 3.1 (also excellent)
ollama pull llama3.1:8b-instruct
```

### Step 4: Test the Model

```bash
# Interactive test
ollama run qwen2.5:14b-instruct "What are the benefits of interval training?"

# API test
curl http://localhost:11434/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "qwen2.5:14b-instruct",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

### Step 5: Configure Pierre

```bash
# Set environment variables
export PIERRE_LLM_PROVIDER=local
export LOCAL_LLM_BASE_URL=http://localhost:11434/v1
export LOCAL_LLM_MODEL=qwen2.5:14b-instruct

# Or add to .envrc:
echo 'export PIERRE_LLM_PROVIDER=local' >> .envrc
echo 'export LOCAL_LLM_MODEL=qwen2.5:14b-instruct' >> .envrc
direnv allow
```

### Step 6: Start Pierre and Test

```bash
# Start Pierre server
./bin/start-server.sh

# Test chat endpoint
curl -X POST http://localhost:8081/api/chat/conversations \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{"title": "Test Chat"}'
```

---

## Model Recommendations

### For Function Calling (Pierre's 14+ Tools)

| Model | Size | Function Calling | Speed | Notes |
|-------|------|------------------|-------|-------|
| **Qwen 2.5 14B-Instruct** | 14B | ⭐⭐⭐⭐⭐ | Fast | Best balance for most hardware |
| Qwen 2.5 32B-Instruct | 32B | ⭐⭐⭐⭐⭐ | Medium | Best quality, needs 24GB+ |
| Qwen 2.5 7B-Instruct | 7B | ⭐⭐⭐⭐ | Very Fast | Good for lighter hardware |
| Llama 3.1 8B-Instruct | 8B | ⭐⭐⭐⭐ | Very Fast | Meta's latest, excellent |
| Llama 3.3 70B-Instruct | 70B | ⭐⭐⭐⭐⭐ | Slow | Best quality, needs 48GB+ |
| Mistral 7B-Instruct | 7B | ⭐⭐⭐⭐ | Very Fast | Fast and versatile |

### Ollama Model Commands

```bash
# List installed models
ollama list

# Pull a model
ollama pull qwen2.5:14b-instruct

# Remove a model
ollama rm qwen2.5:7b-instruct

# Show model info
ollama show qwen2.5:14b-instruct
```

---

## Configuration Reference

### Environment Variables

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `PIERRE_LLM_PROVIDER` | Provider: `groq`, `gemini`, `local`, `ollama`, `vllm`, `localai` | `groq` | No |
| `GROQ_API_KEY` | Groq API key | - | Yes (for Groq) |
| `GEMINI_API_KEY` | Google Gemini API key | - | Yes (for Gemini) |
| `LOCAL_LLM_BASE_URL` | Local LLM API endpoint | `http://localhost:11434/v1` | No |
| `LOCAL_LLM_MODEL` | Model name for local provider | `qwen2.5:14b-instruct` | No |
| `LOCAL_LLM_API_KEY` | API key for local provider | (empty) | No |

### Provider Capabilities

| Capability | Groq | Gemini | Local (Ollama) |
|------------|------|--------|----------------|
| Streaming | ✅ | ✅ | ✅ |
| Function/Tool Calling | ✅ | ✅ | ✅ |
| Vision/Image Input | ❌ | ✅ | ❌ |
| JSON Mode | ✅ | ✅ | ❌ |
| System Messages | ✅ | ✅ | ✅ |
| Offline Operation | ❌ | ❌ | ✅ |
| Privacy (No Data Sent) | ❌ | ❌ | ✅ |

### Supported Models by Provider

#### Groq (Cloud)

| Model | Description | Default |
|-------|-------------|---------|
| `llama-3.3-70b-versatile` | High-quality general purpose | ✓ |
| `llama-3.1-8b-instant` | Fast responses for simple tasks | |
| `llama-3.1-70b-versatile` | Versatile 70B model | |
| `mixtral-8x7b-32768` | Long context window (32K tokens) | |
| `gemma2-9b-it` | Google's Gemma 2 instruction-tuned | |

**Rate Limits**: Free tier has 12,000 tokens-per-minute limit.

#### Gemini (Cloud)

| Model | Description | Default |
|-------|-------------|---------|
| `gemini-2.5-flash` | Latest fast model with improved capabilities | ✓ |
| `gemini-2.0-flash-exp` | Experimental fast model | |
| `gemini-1.5-pro` | Advanced reasoning capabilities | |
| `gemini-1.5-flash` | Balanced performance and cost | |
| `gemini-1.0-pro` | Legacy pro model | |

#### Local (Ollama/vLLM)

| Model | Description | Recommended For |
|-------|-------------|-----------------|
| `qwen2.5:14b-instruct` | Excellent function calling | 24GB RAM (default) |
| `qwen2.5:7b-instruct` | Fast, good function calling | 16GB RAM |
| `qwen2.5:32b-instruct` | Best quality function calling | 32GB+ RAM |
| `llama3.1:8b-instruct` | Meta's latest 8B | 16GB RAM |
| `llama3.1:70b-instruct` | Meta's latest 70B | 48GB+ RAM |
| `mistral:7b-instruct` | Fast and versatile | 16GB RAM |

---

## Testing

### Run All LLM Tests

```bash
# LLM module unit tests
cargo test --test llm_test -- --nocapture

# LLM provider abstraction tests
cargo test --test llm_provider_test -- --nocapture
```

### Test Local Provider Specifically

```bash
# Ensure Ollama is running first
ollama serve &

# Test provider initialization
cargo test test_llm_provider_type -- --nocapture

# Test chat functionality (requires running server)
cargo test --test llm_local_integration_test -- --nocapture
```

### Manual Testing

```bash
# 1. Start Ollama
ollama serve

# 2. Pull test model
ollama pull qwen2.5:7b-instruct

# 3. Set environment
export PIERRE_LLM_PROVIDER=local
export LOCAL_LLM_MODEL=qwen2.5:7b-instruct

# 4. Start Pierre
./bin/start-server.sh

# 5. Test health endpoint
curl http://localhost:8081/health

# 6. Test chat (requires authentication)
# Create admin token first:
cargo run --bin admin-setup -- generate-token --service test --expires-days 1
```

### Validation Checklist for Local LLM

Before deploying with local LLM, verify:

- [ ] Ollama server is running (`curl http://localhost:11434/api/version`)
- [ ] Model is pulled (`ollama list`)
- [ ] Model supports function calling (use Qwen 2.5 or Llama 3.1)
- [ ] Environment variables are set correctly
- [ ] Pierre can connect to Ollama (`curl http://localhost:8081/health`)
- [ ] Chat streaming works
- [ ] Tool execution works (test with fitness tools)

---

## Alternative Local Backends

### vLLM (Production)

For production deployments with high throughput:

```bash
# Install vLLM
pip install vllm

# Start vLLM server
python -m vllm.entrypoints.openai.api_server \
  --model Qwen/Qwen2.5-14B-Instruct \
  --port 8000

# Configure Pierre
export PIERRE_LLM_PROVIDER=vllm
export LOCAL_LLM_BASE_URL=http://localhost:8000/v1
export LOCAL_LLM_MODEL=Qwen/Qwen2.5-14B-Instruct
```

**vLLM advantages:**
- Parallel function calls
- Streaming tool calls
- Higher throughput via PagedAttention
- Better for multiple concurrent users

### LocalAI

```bash
# Run LocalAI with Docker
docker run -p 8080:8080 localai/localai

# Configure Pierre
export PIERRE_LLM_PROVIDER=localai
export LOCAL_LLM_BASE_URL=http://localhost:8080/v1
```

---

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

// Force Local
let provider = ChatProvider::local()?;
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

All three providers (Gemini, Groq, Local) support tool calling:

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

---

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

---

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

---

## Module Structure

```
src/llm/
├── mod.rs              # Trait definitions, types, registry, exports
├── provider.rs         # ChatProvider enum (runtime selector)
├── gemini.rs           # Google Gemini implementation
├── groq.rs             # Groq LPU implementation
├── openai_compatible.rs # Generic OpenAI-compatible provider (Ollama, vLLM, LocalAI)
└── prompts/
    └── mod.rs          # System prompts (pierre_system.md)
```

---

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
    Local(OpenAiCompatibleProvider),
    MyProvider(MyProvider),  // Add variant
}
```

3. **Update environment config** in `src/config/environment.rs`

4. **Register tests** in `tests/llm_test.rs`

---

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

### Common Local LLM Errors

| Error | Cause | Solution |
|-------|-------|----------|
| "Cannot connect to Ollama" | Ollama not running | Run `ollama serve` |
| "Model not found" | Model not pulled | Run `ollama pull MODEL_NAME` |
| "Connection refused" | Wrong port/URL | Check `LOCAL_LLM_BASE_URL` |
| "Timeout" | Model loading or slow inference | Wait, or use smaller model |

---

## Troubleshooting

### Ollama Won't Start

```bash
# Check if already running
pgrep -f ollama

# Kill existing instance
pkill ollama

# Start fresh
ollama serve
```

### Model Too Slow

```bash
# Use a smaller quantization
ollama pull qwen2.5:14b-instruct-q4_K_M

# Or use a smaller model
ollama pull qwen2.5:7b-instruct
```

### Out of Memory

```bash
# Check model size
ollama show qwen2.5:14b-instruct --modelfile

# Use smaller model
ollama pull qwen2.5:7b-instruct

# Or reduce context length in requests
```

### Function Calling Not Working

- Ensure you're using a model trained for function calling (Qwen 2.5, Llama 3.1)
- Verify the model is the instruct/chat variant, not base
- Check tool definitions are valid JSON Schema

---

## See Also

- [Tools Reference - Recipe Management](tools-reference.md#recipe-management)
- [Configuration Guide](configuration.md)
- [Architecture Documentation](architecture.md)
