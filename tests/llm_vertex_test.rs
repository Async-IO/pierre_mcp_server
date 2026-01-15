// ABOUTME: Unit tests for the Vertex AI LLM provider
// ABOUTME: Validates configuration and provider trait implementation
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::llm::{LlmProvider, VertexAiProvider};

#[test]
fn test_default_model() {
    let provider = VertexAiProvider::new("my-project", "us-central1");
    assert_eq!(provider.default_model(), "gemini-1.5-flash");
}

#[test]
fn test_with_custom_model() {
    let provider =
        VertexAiProvider::new("my-project", "us-central1").with_default_model("gemini-1.5-pro");
    assert_eq!(provider.default_model(), "gemini-1.5-pro");
}

#[test]
fn test_provider_name() {
    let provider = VertexAiProvider::new("my-project", "us-central1");
    assert_eq!(provider.name(), "vertex");
    assert_eq!(provider.display_name(), "Google Vertex AI");
}
