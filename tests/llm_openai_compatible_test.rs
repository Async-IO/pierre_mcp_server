// ABOUTME: Unit tests for OpenAI-compatible LLM provider
// ABOUTME: Validates configuration, from_env parsing, and provider type variants
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::config::LlmProviderType;
use pierre_mcp_server::llm::{LlmCapabilities, OpenAiCompatibleConfig, OpenAiCompatibleProvider};
use std::env;

// =============================================================================
// OpenAiCompatibleConfig Tests
// =============================================================================

#[test]
fn test_openai_compatible_config_ollama() {
    let config = OpenAiCompatibleConfig::ollama("qwen2.5:7b");

    assert_eq!(config.base_url, "http://localhost:11434/v1");
    assert!(config.api_key.is_none());
    assert_eq!(config.default_model, "qwen2.5:7b");
    assert_eq!(config.provider_name, "ollama");
    assert_eq!(config.display_name, "Ollama (Local)");
    assert!(config.capabilities.supports_streaming());
    assert!(config.capabilities.supports_function_calling());
    assert!(config.capabilities.supports_system_messages());
    assert!(!config.capabilities.supports_vision());
}

#[test]
fn test_openai_compatible_config_vllm() {
    let config = OpenAiCompatibleConfig::vllm("meta-llama/Llama-3.1-8B");

    assert_eq!(config.base_url, "http://localhost:8000/v1");
    assert!(config.api_key.is_none());
    assert_eq!(config.default_model, "meta-llama/Llama-3.1-8B");
    assert_eq!(config.provider_name, "vllm");
    assert_eq!(config.display_name, "vLLM (Local)");
    assert!(config.capabilities.supports_streaming());
    assert!(config.capabilities.supports_function_calling());
    assert!(config.capabilities.supports_system_messages());
    assert!(config.capabilities.supports_json_mode());
    assert!(!config.capabilities.supports_vision());
}

#[test]
fn test_openai_compatible_config_local_ai() {
    let config = OpenAiCompatibleConfig::local_ai("mistral-7b");

    assert_eq!(config.base_url, "http://localhost:8080/v1");
    assert!(config.api_key.is_none());
    assert_eq!(config.default_model, "mistral-7b");
    assert_eq!(config.provider_name, "localai");
    assert_eq!(config.display_name, "LocalAI");
    assert!(config.capabilities.supports_streaming());
    assert!(config.capabilities.supports_function_calling());
    assert!(config.capabilities.supports_system_messages());
}

#[test]
fn test_openai_compatible_config_default() {
    let config = OpenAiCompatibleConfig::default();

    assert_eq!(config.base_url, "http://localhost:11434/v1");
    assert!(config.api_key.is_none());
    assert_eq!(config.default_model, "qwen2.5:14b-instruct");
    assert_eq!(config.provider_name, "local");
    assert_eq!(config.display_name, "Local LLM");
}

#[test]
fn test_openai_compatible_config_capabilities_differ() {
    let ollama = OpenAiCompatibleConfig::ollama("test");
    let vllm = OpenAiCompatibleConfig::vllm("test");

    // vLLM has JSON mode, Ollama does not
    assert!(!ollama.capabilities.supports_json_mode());
    assert!(vllm.capabilities.supports_json_mode());

    // Both have streaming and function calling
    assert!(ollama.capabilities.supports_streaming());
    assert!(vllm.capabilities.supports_streaming());
    assert!(ollama.capabilities.supports_function_calling());
    assert!(vllm.capabilities.supports_function_calling());
}

// =============================================================================
// OpenAiCompatibleProvider Creation Tests
// =============================================================================

#[test]
fn test_openai_compatible_provider_new() {
    let config = OpenAiCompatibleConfig::ollama("qwen2.5:7b");
    let provider = OpenAiCompatibleProvider::new(config);

    assert!(provider.is_ok());
}

#[test]
fn test_openai_compatible_provider_new_vllm() {
    let config = OpenAiCompatibleConfig::vllm("llama3.1:8b");
    let provider = OpenAiCompatibleProvider::new(config);

    assert!(provider.is_ok());
}

#[test]
fn test_openai_compatible_provider_new_localai() {
    let config = OpenAiCompatibleConfig::local_ai("mistral-7b");
    let provider = OpenAiCompatibleProvider::new(config);

    assert!(provider.is_ok());
}

// =============================================================================
// Environment Variable Parsing Tests
// =============================================================================

mod env_tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex to ensure env var tests don't interfere with each other
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    fn with_clean_env<F, T>(f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let _guard = ENV_MUTEX.lock().unwrap();

        // Save current values
        let saved_base_url = env::var("LOCAL_LLM_BASE_URL").ok();
        let saved_model = env::var("LOCAL_LLM_MODEL").ok();
        let saved_api_key = env::var("LOCAL_LLM_API_KEY").ok();

        // Clear all
        env::remove_var("LOCAL_LLM_BASE_URL");
        env::remove_var("LOCAL_LLM_MODEL");
        env::remove_var("LOCAL_LLM_API_KEY");

        let result = f();

        // Restore
        if let Some(v) = saved_base_url {
            env::set_var("LOCAL_LLM_BASE_URL", v);
        }
        if let Some(v) = saved_model {
            env::set_var("LOCAL_LLM_MODEL", v);
        }
        if let Some(v) = saved_api_key {
            env::set_var("LOCAL_LLM_API_KEY", v);
        }

        result
    }

    #[test]
    fn test_openai_compatible_provider_from_env_defaults() {
        with_clean_env(|| {
            let provider = OpenAiCompatibleProvider::from_env();
            assert!(provider.is_ok());
        });
    }

    #[test]
    fn test_openai_compatible_provider_from_env_custom_url() {
        with_clean_env(|| {
            env::set_var("LOCAL_LLM_BASE_URL", "http://custom-host:9999/v1");
            env::set_var("LOCAL_LLM_MODEL", "custom-model");

            let provider = OpenAiCompatibleProvider::from_env();
            assert!(provider.is_ok());
        });
    }

    #[test]
    fn test_openai_compatible_provider_from_env_with_api_key() {
        with_clean_env(|| {
            env::set_var("LOCAL_LLM_API_KEY", "test-api-key-12345");

            let provider = OpenAiCompatibleProvider::from_env();
            assert!(provider.is_ok());
        });
    }

    #[test]
    fn test_openai_compatible_provider_from_env_empty_api_key_ignored() {
        with_clean_env(|| {
            env::set_var("LOCAL_LLM_API_KEY", "");

            let provider = OpenAiCompatibleProvider::from_env();
            assert!(provider.is_ok());
        });
    }

    #[test]
    fn test_openai_compatible_provider_from_env_ollama_port_detection() {
        with_clean_env(|| {
            env::set_var("LOCAL_LLM_BASE_URL", "http://localhost:11434/v1");

            let provider = OpenAiCompatibleProvider::from_env();
            assert!(provider.is_ok());
        });
    }

    #[test]
    fn test_openai_compatible_provider_from_env_vllm_port_detection() {
        with_clean_env(|| {
            env::set_var("LOCAL_LLM_BASE_URL", "http://localhost:8000/v1");

            let provider = OpenAiCompatibleProvider::from_env();
            assert!(provider.is_ok());
        });
    }

    #[test]
    fn test_openai_compatible_provider_from_env_localai_port_detection() {
        with_clean_env(|| {
            env::set_var("LOCAL_LLM_BASE_URL", "http://localhost:8080/v1");

            let provider = OpenAiCompatibleProvider::from_env();
            assert!(provider.is_ok());
        });
    }
}

// =============================================================================
// LlmProviderType Tests for Local Variants
// =============================================================================

#[test]
fn test_llm_provider_type_local_from_str() {
    assert_eq!(
        LlmProviderType::from_str_or_default("local"),
        LlmProviderType::Local
    );
}

#[test]
fn test_llm_provider_type_ollama_from_str() {
    assert_eq!(
        LlmProviderType::from_str_or_default("ollama"),
        LlmProviderType::Local
    );
}

#[test]
fn test_llm_provider_type_vllm_from_str() {
    assert_eq!(
        LlmProviderType::from_str_or_default("vllm"),
        LlmProviderType::Local
    );
}

#[test]
fn test_llm_provider_type_localai_from_str() {
    assert_eq!(
        LlmProviderType::from_str_or_default("localai"),
        LlmProviderType::Local
    );
}

#[test]
fn test_llm_provider_type_local_display() {
    assert_eq!(LlmProviderType::Local.to_string(), "local");
}

#[test]
fn test_llm_provider_type_case_insensitive() {
    assert_eq!(
        LlmProviderType::from_str_or_default("LOCAL"),
        LlmProviderType::Local
    );
    assert_eq!(
        LlmProviderType::from_str_or_default("Ollama"),
        LlmProviderType::Local
    );
    assert_eq!(
        LlmProviderType::from_str_or_default("VLLM"),
        LlmProviderType::Local
    );
    assert_eq!(
        LlmProviderType::from_str_or_default("LocalAI"),
        LlmProviderType::Local
    );
}

#[test]
fn test_llm_provider_type_default_is_groq() {
    assert_eq!(LlmProviderType::default(), LlmProviderType::Groq);
}

#[test]
fn test_llm_provider_type_unknown_falls_back_to_groq() {
    assert_eq!(
        LlmProviderType::from_str_or_default("unknown"),
        LlmProviderType::Groq
    );
    assert_eq!(
        LlmProviderType::from_str_or_default("invalid"),
        LlmProviderType::Groq
    );
}

// =============================================================================
// LlmCapabilities Tests for Local Provider
// =============================================================================

#[test]
fn test_local_provider_capabilities() {
    let config = OpenAiCompatibleConfig::default();

    // Default local provider should have streaming and function calling
    assert!(config.capabilities.supports_streaming());
    assert!(config.capabilities.supports_function_calling());
    assert!(config.capabilities.supports_system_messages());

    // Should NOT have vision (local models typically don't)
    assert!(!config.capabilities.supports_vision());
}

#[test]
fn test_ollama_capabilities() {
    let config = OpenAiCompatibleConfig::ollama("qwen2.5:14b-instruct");

    let expected = LlmCapabilities::STREAMING
        | LlmCapabilities::FUNCTION_CALLING
        | LlmCapabilities::SYSTEM_MESSAGES;

    assert_eq!(config.capabilities, expected);
}

#[test]
fn test_vllm_capabilities_include_json_mode() {
    let config = OpenAiCompatibleConfig::vllm("llama3.1:8b");

    let expected = LlmCapabilities::STREAMING
        | LlmCapabilities::FUNCTION_CALLING
        | LlmCapabilities::SYSTEM_MESSAGES
        | LlmCapabilities::JSON_MODE;

    assert_eq!(config.capabilities, expected);
}

// =============================================================================
// Integration-Ready Tests (require running server)
// =============================================================================

#[tokio::test]
#[ignore = "Requires running Ollama server"]
async fn test_openai_compatible_health_check() {
    use pierre_mcp_server::llm::LlmProvider;

    let config = OpenAiCompatibleConfig::ollama("qwen2.5:7b");
    let provider = OpenAiCompatibleProvider::new(config).unwrap();

    let result = provider.health_check().await;
    assert!(result.is_ok(), "Health check should pass: {result:?}");
    assert!(result.unwrap());
}

#[tokio::test]
#[ignore = "Requires running Ollama server with model pulled"]
async fn test_openai_compatible_complete() {
    use pierre_mcp_server::llm::{ChatMessage, ChatRequest, LlmProvider};

    let config = OpenAiCompatibleConfig::ollama("qwen2.5:7b");
    let provider = OpenAiCompatibleProvider::new(config).unwrap();

    let request = ChatRequest::new(vec![ChatMessage::user("Say hello in exactly 3 words.")]);

    let response = provider.complete(&request).await;
    assert!(response.is_ok(), "Completion should succeed: {response:?}");

    let response = response.unwrap();
    assert!(!response.content.is_empty());
}

#[tokio::test]
#[ignore = "Requires running Ollama server with model pulled"]
async fn test_openai_compatible_complete_with_tools() {
    use pierre_mcp_server::llm::{ChatMessage, ChatRequest, FunctionDeclaration, Tool};
    use serde_json::json;

    let config = OpenAiCompatibleConfig::ollama("qwen2.5:14b-instruct");
    let provider = OpenAiCompatibleProvider::new(config).unwrap();

    let tools = vec![Tool {
        function_declarations: vec![FunctionDeclaration {
            name: "get_weather".to_owned(),
            description: "Get current weather for a location".to_owned(),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "City name"
                    }
                },
                "required": ["location"]
            })),
        }],
    }];

    let request = ChatRequest::new(vec![ChatMessage::user(
        "What's the weather like in Paris? Use the get_weather tool.",
    )]);

    let response = provider.complete_with_tools(&request, Some(tools)).await;
    assert!(
        response.is_ok(),
        "Tool completion should succeed: {response:?}"
    );

    let response = response.unwrap();
    // Either we get a tool call or a text response explaining tools aren't available
    assert!(response.function_calls.is_some() || response.content.is_some());
}

#[tokio::test]
#[ignore = "Requires running Ollama server with model pulled"]
async fn test_openai_compatible_streaming() {
    use futures_util::StreamExt;
    use pierre_mcp_server::llm::{ChatMessage, ChatRequest, LlmProvider};

    let config = OpenAiCompatibleConfig::ollama("qwen2.5:7b");
    let provider = OpenAiCompatibleProvider::new(config).unwrap();

    let request = ChatRequest::new(vec![ChatMessage::user("Count from 1 to 5.")]);

    let stream_result = provider.complete_stream(&request).await;
    assert!(stream_result.is_ok(), "Stream should start successfully");

    let mut stream = stream_result.unwrap();
    let mut chunks_received = 0;
    let mut full_content = String::new();

    while let Some(chunk_result) = stream.next().await {
        assert!(chunk_result.is_ok(), "Chunk should be valid");
        let chunk = chunk_result.unwrap();
        full_content.push_str(&chunk.delta);
        chunks_received += 1;
    }

    assert!(chunks_received > 0, "Should receive at least one chunk");
    assert!(!full_content.is_empty(), "Should have content");
}
