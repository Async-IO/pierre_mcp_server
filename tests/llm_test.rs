// ABOUTME: Unit tests for the LLM provider abstraction layer
// ABOUTME: Tests capabilities, message handling, provider registry, and Gemini implementation
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// Test files don't require documentation - this is a rustc lint (not clippy)
#![allow(missing_docs)]

use pierre_mcp_server::llm::{
    ChatMessage, ChatRequest, GeminiProvider, LlmCapabilities, LlmProvider, LlmProviderRegistry,
    MessageRole,
};

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
    let provider = GeminiProvider::new("test-key");
    assert_eq!(provider.name(), "gemini");
    assert_eq!(provider.display_name(), "Google Gemini");
    assert!(!provider.available_models().is_empty());
}

#[test]
fn test_gemini_capabilities() {
    let provider = GeminiProvider::new("test-key");
    let caps = provider.capabilities();
    assert!(caps.supports_streaming());
    assert!(caps.supports_function_calling());
    assert!(caps.supports_vision());
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
fn test_gemini_with_default_model() {
    let provider = GeminiProvider::new("key").with_default_model("gemini-1.5-pro");
    // Check via the trait method since default_model field is private
    let debug_output = format!("{provider:?}");
    assert!(debug_output.contains("gemini-1.5-pro"));
}
