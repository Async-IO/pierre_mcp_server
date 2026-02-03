// ABOUTME: Unit tests for the LLM provider abstraction layer
// ABOUTME: Tests capabilities, message handling, provider registry, and Gemini implementation
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// Test files don't require documentation - this is a rustc lint (not clippy)
#![allow(missing_docs)]

use pierre_mcp_server::config::{LlmModelConfig, LlmProviderType};
use pierre_mcp_server::llm::{
    ChatMessage, ChatRequest, GeminiProvider, GroqProvider, LlmCapabilities, LlmProvider,
    LlmProviderRegistry, MessageRole,
};

/// Helper to create a test model config
fn test_model_config() -> LlmModelConfig {
    LlmModelConfig {
        default_model: "test-model".to_owned(),
        fallback_model: "test-model".to_owned(),
    }
}

// ============================================================================
// LlmCapabilities Tests
// ============================================================================

#[test]
fn test_capabilities_text_only() {
    let caps = LlmCapabilities::text_only();
    assert!(caps.supports_streaming());
    assert!(caps.supports_system_messages());
    assert!(!caps.supports_function_calling());
    assert!(!caps.supports_vision());
}

#[test]
fn test_capabilities_full_featured() {
    let caps = LlmCapabilities::full_featured();
    assert!(caps.supports_streaming());
    assert!(caps.supports_function_calling());
    assert!(caps.supports_vision());
    assert!(caps.supports_json_mode());
    assert!(caps.supports_system_messages());
}

// ============================================================================
// MessageRole Tests
// ============================================================================

#[test]
fn test_message_role_as_str() {
    assert_eq!(MessageRole::System.as_str(), "system");
    assert_eq!(MessageRole::User.as_str(), "user");
    assert_eq!(MessageRole::Assistant.as_str(), "assistant");
}

// ============================================================================
// ChatMessage Tests
// ============================================================================

#[test]
fn test_chat_message_constructors() {
    let system = ChatMessage::system("You are helpful");
    assert_eq!(system.role, MessageRole::System);
    assert_eq!(system.content, "You are helpful");

    let user = ChatMessage::user("Hello");
    assert_eq!(user.role, MessageRole::User);

    let assistant = ChatMessage::assistant("Hi there!");
    assert_eq!(assistant.role, MessageRole::Assistant);
}

// ============================================================================
// ChatRequest Tests
// ============================================================================

#[test]
fn test_chat_request_builder() {
    let request = ChatRequest::new(vec![ChatMessage::user("Hello")])
        .with_model("gemini-pro")
        .with_temperature(0.7)
        .with_max_tokens(1000)
        .with_streaming();

    assert_eq!(request.model, Some("gemini-pro".to_owned()));
    assert_eq!(request.temperature, Some(0.7));
    assert_eq!(request.max_tokens, Some(1000));
    assert!(request.stream);
}

// ============================================================================
// LlmProviderRegistry Tests
// ============================================================================

#[test]
fn test_registry_operations() {
    let registry = LlmProviderRegistry::new();
    assert!(registry.list().is_empty());
    assert!(registry.default_provider().is_none());
}

// ============================================================================
// GeminiProvider Tests
// ============================================================================

#[test]
fn test_gemini_provider_metadata() {
    let provider = GeminiProvider::with_config("test-key", &test_model_config());
    assert_eq!(provider.name(), "gemini");
    assert_eq!(provider.display_name(), "Google Gemini");
    assert!(!provider.available_models().is_empty());
}

#[test]
fn test_gemini_capabilities() {
    let provider = GeminiProvider::with_config("test-key", &test_model_config());
    let caps = provider.capabilities();
    assert!(caps.supports_streaming());
    assert!(caps.supports_function_calling());
    assert!(caps.supports_vision());
    assert!(caps.supports_system_messages());
}

#[test]
fn test_gemini_debug_redacts_api_key() {
    let provider = GeminiProvider::with_config("super-secret-key", &test_model_config());
    let debug_output = format!("{provider:?}");
    assert!(!debug_output.contains("super-secret-key"));
    assert!(debug_output.contains("[REDACTED]"));
}

#[test]
fn test_gemini_with_custom_model() {
    let config = LlmModelConfig {
        default_model: "gemini-2.5-pro".to_owned(),
        fallback_model: "gemini-2.5-pro".to_owned(),
    };
    let provider = GeminiProvider::with_config("key", &config);
    // Check via the trait method since default_model field is private
    let debug_output = format!("{provider:?}");
    assert!(debug_output.contains("gemini-2.5-pro"));
}

// ============================================================================
// GroqProvider Tests
// ============================================================================

#[test]
fn test_groq_provider_metadata() {
    let provider = GroqProvider::new("test-key".to_owned());
    assert_eq!(provider.name(), "groq");
    assert_eq!(provider.display_name(), "Groq (Llama/Mixtral)");
    assert!(!provider.available_models().is_empty());
}

#[test]
fn test_groq_capabilities() {
    let provider = GroqProvider::new("test-key".to_owned());
    let caps = provider.capabilities();
    assert!(caps.supports_streaming());
    assert!(caps.supports_function_calling());
    assert!(caps.supports_system_messages());
}

// ============================================================================
// LlmProviderType Tests
// ============================================================================

#[test]
fn test_llm_provider_type_default() {
    let provider_type = LlmProviderType::default();
    assert_eq!(provider_type, LlmProviderType::Groq);
}

#[test]
fn test_llm_provider_type_from_str() {
    assert_eq!(
        LlmProviderType::from_str_or_default("groq"),
        LlmProviderType::Groq
    );
    assert_eq!(
        LlmProviderType::from_str_or_default("gemini"),
        LlmProviderType::Gemini
    );
    assert_eq!(
        LlmProviderType::from_str_or_default("google"),
        LlmProviderType::Gemini
    );
    // Unknown values default to Groq
    assert_eq!(
        LlmProviderType::from_str_or_default("unknown"),
        LlmProviderType::Groq
    );
}

#[test]
fn test_llm_provider_type_display() {
    assert_eq!(format!("{}", LlmProviderType::Groq), "groq");
    assert_eq!(format!("{}", LlmProviderType::Gemini), "gemini");
}

#[test]
fn test_llm_provider_type_env_var_name() {
    assert_eq!(LlmProviderType::ENV_VAR, "PIERRE_LLM_PROVIDER");
}
