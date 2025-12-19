// ABOUTME: Unit tests for LLM provider module functionality
// ABOUTME: Validates LLM capabilities, message types, chat requests, and provider registry
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::llm::{
    ChatMessage, ChatRequest, ChatResponse, LlmCapabilities, LlmProviderRegistry, MessageRole,
    StreamChunk, TokenUsage,
};

// =============================================================================
// LlmCapabilities Tests
// =============================================================================

#[test]
fn test_llm_capabilities_default() {
    let caps = LlmCapabilities::default();
    assert!(!caps.supports_streaming());
    assert!(!caps.supports_function_calling());
    assert!(!caps.supports_vision());
    assert!(!caps.supports_json_mode());
    assert!(!caps.supports_system_messages());
}

#[test]
fn test_llm_capabilities_text_only() {
    let caps = LlmCapabilities::text_only();

    assert!(caps.supports_streaming());
    assert!(caps.supports_system_messages());
    assert!(!caps.supports_function_calling());
    assert!(!caps.supports_vision());
    assert!(!caps.supports_json_mode());
}

#[test]
fn test_llm_capabilities_full_featured() {
    let caps = LlmCapabilities::full_featured();

    assert!(caps.supports_streaming());
    assert!(caps.supports_function_calling());
    assert!(caps.supports_vision());
    assert!(caps.supports_json_mode());
    assert!(caps.supports_system_messages());
}

#[test]
fn test_llm_capabilities_individual_flags() {
    let streaming = LlmCapabilities::STREAMING;
    assert!(streaming.supports_streaming());
    assert!(!streaming.supports_function_calling());

    let function_calling = LlmCapabilities::FUNCTION_CALLING;
    assert!(function_calling.supports_function_calling());
    assert!(!function_calling.supports_streaming());

    let vision = LlmCapabilities::VISION;
    assert!(vision.supports_vision());

    let json_mode = LlmCapabilities::JSON_MODE;
    assert!(json_mode.supports_json_mode());

    let system_messages = LlmCapabilities::SYSTEM_MESSAGES;
    assert!(system_messages.supports_system_messages());
}

#[test]
fn test_llm_capabilities_combine() {
    let caps = LlmCapabilities::STREAMING | LlmCapabilities::FUNCTION_CALLING;

    assert!(caps.supports_streaming());
    assert!(caps.supports_function_calling());
    assert!(!caps.supports_vision());
}

#[test]
fn test_llm_capabilities_serialization() {
    let caps = LlmCapabilities::full_featured();

    let json = serde_json::to_string(&caps).unwrap();
    let deserialized: LlmCapabilities = serde_json::from_str(&json).unwrap();

    assert_eq!(caps, deserialized);
}

// =============================================================================
// MessageRole Tests
// =============================================================================

#[test]
fn test_message_role_as_str() {
    assert_eq!(MessageRole::System.as_str(), "system");
    assert_eq!(MessageRole::User.as_str(), "user");
    assert_eq!(MessageRole::Assistant.as_str(), "assistant");
}

#[test]
fn test_message_role_serialization() {
    let json = serde_json::to_string(&MessageRole::User).unwrap();
    assert_eq!(json, "\"user\"");

    let deserialized: MessageRole = serde_json::from_str("\"assistant\"").unwrap();
    assert_eq!(deserialized, MessageRole::Assistant);
}

#[test]
fn test_message_role_equality() {
    assert_eq!(MessageRole::System, MessageRole::System);
    assert_ne!(MessageRole::User, MessageRole::Assistant);
}

// =============================================================================
// ChatMessage Tests
// =============================================================================

#[test]
fn test_chat_message_new() {
    let msg = ChatMessage::new(MessageRole::User, "Hello, world!");

    assert_eq!(msg.role, MessageRole::User);
    assert_eq!(msg.content, "Hello, world!");
}

#[test]
fn test_chat_message_system() {
    let msg = ChatMessage::system("You are a helpful assistant.");

    assert_eq!(msg.role, MessageRole::System);
    assert_eq!(msg.content, "You are a helpful assistant.");
}

#[test]
fn test_chat_message_user() {
    let msg = ChatMessage::user("What's the weather?");

    assert_eq!(msg.role, MessageRole::User);
    assert_eq!(msg.content, "What's the weather?");
}

#[test]
fn test_chat_message_assistant() {
    let msg = ChatMessage::assistant("I'd be happy to help!");

    assert_eq!(msg.role, MessageRole::Assistant);
    assert_eq!(msg.content, "I'd be happy to help!");
}

#[test]
fn test_chat_message_with_string() {
    let content = String::from("Dynamic content");
    let msg = ChatMessage::user(content);

    assert_eq!(msg.content, "Dynamic content");
}

#[test]
fn test_chat_message_serialization() {
    let msg = ChatMessage::user("Test message");

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"role\":\"user\""));
    assert!(json.contains("\"content\":\"Test message\""));

    let deserialized: ChatMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.role, msg.role);
    assert_eq!(deserialized.content, msg.content);
}

#[test]
fn test_chat_message_clone() {
    let msg = ChatMessage::system("Original");
    let cloned = msg.clone();

    assert_eq!(msg.role, cloned.role);
    assert_eq!(msg.content, cloned.content);
}

// =============================================================================
// ChatRequest Tests
// =============================================================================

#[test]
fn test_chat_request_new() {
    let messages = vec![ChatMessage::user("Hello!")];
    let request = ChatRequest::new(messages);

    assert_eq!(request.messages.len(), 1);
    assert!(request.model.is_none());
    assert!(request.temperature.is_none());
    assert!(request.max_tokens.is_none());
    assert!(!request.stream);
}

#[test]
fn test_chat_request_with_model() {
    let messages = vec![ChatMessage::user("Hi")];
    let request = ChatRequest::new(messages).with_model("gpt-4");

    assert_eq!(request.model, Some("gpt-4".to_owned()));
}

#[test]
fn test_chat_request_with_temperature() {
    let messages = vec![ChatMessage::user("Hi")];
    let request = ChatRequest::new(messages).with_temperature(0.7);

    assert_eq!(request.temperature, Some(0.7));
}

#[test]
fn test_chat_request_with_max_tokens() {
    let messages = vec![ChatMessage::user("Hi")];
    let request = ChatRequest::new(messages).with_max_tokens(1000);

    assert_eq!(request.max_tokens, Some(1000));
}

#[test]
fn test_chat_request_with_streaming() {
    let messages = vec![ChatMessage::user("Hi")];
    let request = ChatRequest::new(messages).with_streaming();

    assert!(request.stream);
}

#[test]
fn test_chat_request_builder_chain() {
    let messages = vec![
        ChatMessage::system("You are helpful."),
        ChatMessage::user("What is 2+2?"),
    ];

    let request = ChatRequest::new(messages)
        .with_model("gemini-pro")
        .with_temperature(0.5)
        .with_max_tokens(500)
        .with_streaming();

    assert_eq!(request.messages.len(), 2);
    assert_eq!(request.model, Some("gemini-pro".to_owned()));
    assert_eq!(request.temperature, Some(0.5));
    assert_eq!(request.max_tokens, Some(500));
    assert!(request.stream);
}

#[test]
fn test_chat_request_serialization() {
    let messages = vec![ChatMessage::user("Test")];
    let request = ChatRequest::new(messages).with_model("test-model");

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("test-model"));

    let deserialized: ChatRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.model, request.model);
}

// =============================================================================
// ChatResponse Tests
// =============================================================================

#[test]
fn test_chat_response_creation() {
    let response = ChatResponse {
        content: "The answer is 4.".to_owned(),
        model: "gemini-pro".to_owned(),
        usage: None,
        finish_reason: Some("stop".to_owned()),
    };

    assert_eq!(response.content, "The answer is 4.");
    assert_eq!(response.model, "gemini-pro");
    assert_eq!(response.finish_reason, Some("stop".to_owned()));
}

#[test]
fn test_chat_response_with_usage() {
    let usage = TokenUsage {
        prompt_tokens: 10,
        completion_tokens: 20,
        total_tokens: 30,
    };

    let response = ChatResponse {
        content: "Response".to_owned(),
        model: "test-model".to_owned(),
        usage: Some(usage),
        finish_reason: None,
    };

    let usage = response.usage.unwrap();
    assert_eq!(usage.prompt_tokens, 10);
    assert_eq!(usage.completion_tokens, 20);
    assert_eq!(usage.total_tokens, 30);
}

#[test]
fn test_chat_response_serialization() {
    let response = ChatResponse {
        content: "Hello!".to_owned(),
        model: "model-1".to_owned(),
        usage: Some(TokenUsage {
            prompt_tokens: 5,
            completion_tokens: 10,
            total_tokens: 15,
        }),
        finish_reason: Some("stop".to_owned()),
    };

    let json = serde_json::to_string(&response).unwrap();
    let deserialized: ChatResponse = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.content, response.content);
    assert_eq!(deserialized.model, response.model);
}

// =============================================================================
// TokenUsage Tests
// =============================================================================

#[test]
fn test_token_usage_creation() {
    let usage = TokenUsage {
        prompt_tokens: 100,
        completion_tokens: 200,
        total_tokens: 300,
    };

    assert_eq!(usage.prompt_tokens, 100);
    assert_eq!(usage.completion_tokens, 200);
    assert_eq!(usage.total_tokens, 300);
}

#[test]
fn test_token_usage_serialization() {
    let usage = TokenUsage {
        prompt_tokens: 50,
        completion_tokens: 75,
        total_tokens: 125,
    };

    let json = serde_json::to_string(&usage).unwrap();
    assert!(json.contains("prompt_tokens"));
    assert!(json.contains("50"));

    let deserialized: TokenUsage = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.total_tokens, 125);
}

// =============================================================================
// StreamChunk Tests
// =============================================================================

#[test]
fn test_stream_chunk_creation() {
    let chunk = StreamChunk {
        delta: "Hello".to_owned(),
        is_final: false,
        finish_reason: None,
    };

    assert_eq!(chunk.delta, "Hello");
    assert!(!chunk.is_final);
    assert!(chunk.finish_reason.is_none());
}

#[test]
fn test_stream_chunk_final() {
    let chunk = StreamChunk {
        delta: String::new(),
        is_final: true,
        finish_reason: Some("stop".to_owned()),
    };

    assert!(chunk.is_final);
    assert_eq!(chunk.finish_reason, Some("stop".to_owned()));
}

#[test]
fn test_stream_chunk_serialization() {
    let chunk = StreamChunk {
        delta: "world".to_owned(),
        is_final: false,
        finish_reason: None,
    };

    let json = serde_json::to_string(&chunk).unwrap();
    let deserialized: StreamChunk = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.delta, chunk.delta);
    assert_eq!(deserialized.is_final, chunk.is_final);
}

// =============================================================================
// LlmProviderRegistry Tests
// =============================================================================

#[test]
fn test_provider_registry_new() {
    let registry = LlmProviderRegistry::new();

    assert!(registry.list().is_empty());
    assert!(registry.default_provider().is_none());
}

#[test]
fn test_provider_registry_default() {
    let registry = LlmProviderRegistry::default();

    assert!(registry.list().is_empty());
}

#[test]
fn test_provider_registry_get_nonexistent() {
    let registry = LlmProviderRegistry::new();

    assert!(registry.get("nonexistent").is_none());
}

// =============================================================================
// Edge Cases and Error Handling
// =============================================================================

#[test]
fn test_chat_message_empty_content() {
    let msg = ChatMessage::user("");

    assert_eq!(msg.content, "");
    assert_eq!(msg.role, MessageRole::User);
}

#[test]
fn test_chat_message_unicode_content() {
    let msg = ChatMessage::user("日本語テスト 🏃‍♂️ émoji");

    assert_eq!(msg.content, "日本語テスト 🏃‍♂️ émoji");
}

#[test]
fn test_chat_request_empty_messages() {
    let request = ChatRequest::new(vec![]);

    assert!(request.messages.is_empty());
}

#[test]
fn test_chat_request_many_messages() {
    let messages: Vec<ChatMessage> = (0..100)
        .map(|i| ChatMessage::user(format!("Message {i}")))
        .collect();

    let request = ChatRequest::new(messages);

    assert_eq!(request.messages.len(), 100);
}

#[test]
fn test_chat_request_temperature_boundaries() {
    // Temperature can be 0.0 to 2.0 typically
    let request_low = ChatRequest::new(vec![]).with_temperature(0.0);
    let request_high = ChatRequest::new(vec![]).with_temperature(2.0);

    assert_eq!(request_low.temperature, Some(0.0));
    assert_eq!(request_high.temperature, Some(2.0));
}

#[test]
fn test_token_usage_zero_values() {
    let usage = TokenUsage {
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0,
    };

    assert_eq!(usage.prompt_tokens, 0);
    assert_eq!(usage.total_tokens, 0);
}

#[test]
fn test_llm_capabilities_bitwise_operations() {
    let caps = LlmCapabilities::STREAMING;

    // Test union
    let caps_with_vision = caps | LlmCapabilities::VISION;
    assert!(caps_with_vision.supports_streaming());
    assert!(caps_with_vision.supports_vision());

    // Test intersection
    let intersection = caps_with_vision & LlmCapabilities::STREAMING;
    assert!(intersection.supports_streaming());
    assert!(!intersection.supports_vision());

    // Test contains
    assert!(caps_with_vision.contains(LlmCapabilities::STREAMING));
    assert!(caps_with_vision.contains(LlmCapabilities::VISION));
}

#[test]
fn test_chat_response_clone() {
    let response = ChatResponse {
        content: "Test".to_owned(),
        model: "model".to_owned(),
        usage: None,
        finish_reason: None,
    };

    let cloned = response.clone();
    assert_eq!(response.content, cloned.content);
}

#[test]
fn test_stream_chunk_clone() {
    let chunk = StreamChunk {
        delta: "chunk".to_owned(),
        is_final: false,
        finish_reason: None,
    };

    let cloned = chunk.clone();
    assert_eq!(chunk.delta, cloned.delta);
}
