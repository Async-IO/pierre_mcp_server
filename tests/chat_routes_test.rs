// ABOUTME: Integration tests for the chat route handlers
// ABOUTME: Tests conversation CRUD, messaging, and authentication flows
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

mod common;
mod helpers;

use common::{create_test_server_resources, create_test_user};
use helpers::axum_test::AxumTestRequest;
use pierre_mcp_server::routes::chat::{ChatRoutes, ConversationListResponse, ConversationResponse};

use axum::http::StatusCode;
use serde_json::json;

// ============================================================================
// Test Helpers
// ============================================================================

async fn setup_test_environment() -> (axum::Router, String) {
    let resources = create_test_server_resources().await.unwrap();
    let (_user_id, user) = create_test_user(&resources.database).await.unwrap();

    // Generate a JWT token for the user
    let token = resources
        .auth_manager
        .generate_token(&user, &resources.jwks_manager)
        .unwrap();

    // Create the chat router
    let router = ChatRoutes::routes(resources);

    (router, format!("Bearer {token}"))
}

// ============================================================================
// Conversation CRUD Tests
// ============================================================================

#[tokio::test]
async fn test_create_conversation() {
    let (router, auth_token) = setup_test_environment().await;

    let response = AxumTestRequest::post("/api/chat/conversations")
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "Test Conversation",
            "model": "gemini-2.0-flash-exp"
        }))
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::CREATED);

    let conv: ConversationResponse = response.json();
    assert_eq!(conv.title, "Test Conversation");
    assert_eq!(conv.model, "gemini-2.0-flash-exp");
    assert_eq!(conv.total_tokens, 0);
    assert!(conv.system_prompt.is_none());
}

#[tokio::test]
async fn test_create_conversation_with_system_prompt() {
    let (router, auth_token) = setup_test_environment().await;

    let response = AxumTestRequest::post("/api/chat/conversations")
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "Fitness Chat",
            "model": "gemini-1.5-pro",
            "system_prompt": "You are a helpful fitness assistant."
        }))
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::CREATED);

    let conv: ConversationResponse = response.json();
    assert_eq!(conv.title, "Fitness Chat");
    assert_eq!(conv.model, "gemini-1.5-pro");
    assert_eq!(
        conv.system_prompt,
        Some("You are a helpful fitness assistant.".to_owned())
    );
}

#[tokio::test]
async fn test_list_conversations() {
    let (router, auth_token) = setup_test_environment().await;

    // Create a conversation first
    let create_response = AxumTestRequest::post("/api/chat/conversations")
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "Test Conversation"
        }))
        .send(router.clone())
        .await;

    assert_eq!(create_response.status_code(), StatusCode::CREATED);

    // List conversations
    let list_response = AxumTestRequest::get("/api/chat/conversations")
        .header("authorization", &auth_token)
        .send(router)
        .await;

    assert_eq!(list_response.status_code(), StatusCode::OK);

    let list: ConversationListResponse = list_response.json();
    assert_eq!(list.total, 1);
    assert_eq!(list.conversations.len(), 1);
    assert_eq!(list.conversations[0].title, "Test Conversation");
}

#[tokio::test]
async fn test_get_conversation() {
    let (router, auth_token) = setup_test_environment().await;

    // Create a conversation first
    let create_response = AxumTestRequest::post("/api/chat/conversations")
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "Get Test Conv"
        }))
        .send(router.clone())
        .await;

    let created: ConversationResponse = create_response.json();

    // Get the conversation
    let get_response = AxumTestRequest::get(&format!("/api/chat/conversations/{}", created.id))
        .header("authorization", &auth_token)
        .send(router)
        .await;

    assert_eq!(get_response.status_code(), StatusCode::OK);

    let conv: ConversationResponse = get_response.json();
    assert_eq!(conv.id, created.id);
    assert_eq!(conv.title, "Get Test Conv");
}

#[tokio::test]
async fn test_update_conversation_title() {
    let (router, auth_token) = setup_test_environment().await;

    // Create a conversation first
    let create_response = AxumTestRequest::post("/api/chat/conversations")
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "Original Title"
        }))
        .send(router.clone())
        .await;

    let created: ConversationResponse = create_response.json();

    // Update the title
    let update_response = AxumTestRequest::put(&format!("/api/chat/conversations/{}", created.id))
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "Updated Title"
        }))
        .send(router.clone())
        .await;

    assert_eq!(update_response.status_code(), StatusCode::OK);

    // Verify the update
    let get_response = AxumTestRequest::get(&format!("/api/chat/conversations/{}", created.id))
        .header("authorization", &auth_token)
        .send(router)
        .await;

    let conv: ConversationResponse = get_response.json();
    assert_eq!(conv.title, "Updated Title");
}

#[tokio::test]
async fn test_delete_conversation() {
    let (router, auth_token) = setup_test_environment().await;

    // Create a conversation first
    let create_response = AxumTestRequest::post("/api/chat/conversations")
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "To Delete"
        }))
        .send(router.clone())
        .await;

    let created: ConversationResponse = create_response.json();

    // Delete the conversation
    let delete_response =
        AxumTestRequest::delete(&format!("/api/chat/conversations/{}", created.id))
            .header("authorization", &auth_token)
            .send(router.clone())
            .await;

    assert_eq!(delete_response.status_code(), StatusCode::NO_CONTENT);

    // Verify deletion - should return 404
    let get_response = AxumTestRequest::get(&format!("/api/chat/conversations/{}", created.id))
        .header("authorization", &auth_token)
        .send(router)
        .await;

    assert_eq!(get_response.status_code(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Authentication Tests
// ============================================================================

#[tokio::test]
async fn test_create_conversation_unauthorized() {
    let (router, _) = setup_test_environment().await;

    let response = AxumTestRequest::post("/api/chat/conversations")
        .json(&json!({
            "title": "Test Conversation"
        }))
        .send(router)
        .await;

    // Should fail with 401 Unauthorized
    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_conversation_invalid_token() {
    let (router, _) = setup_test_environment().await;

    let response = AxumTestRequest::post("/api/chat/conversations")
        .header("authorization", "Bearer invalid_token")
        .json(&json!({
            "title": "Test Conversation"
        }))
        .send(router)
        .await;

    // Should fail with 401 Unauthorized
    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Pagination Tests
// ============================================================================

#[tokio::test]
async fn test_list_conversations_pagination() {
    let (router, auth_token) = setup_test_environment().await;

    // Create multiple conversations
    for i in 1..=5 {
        AxumTestRequest::post("/api/chat/conversations")
            .header("authorization", &auth_token)
            .json(&json!({
                "title": format!("Conv {}", i)
            }))
            .send(router.clone())
            .await;
    }

    // Get first page (limit=2)
    let page1_response = AxumTestRequest::get("/api/chat/conversations?limit=2&offset=0")
        .header("authorization", &auth_token)
        .send(router.clone())
        .await;

    let page1: ConversationListResponse = page1_response.json();
    assert_eq!(page1.conversations.len(), 2);

    // Get second page
    let page2_response = AxumTestRequest::get("/api/chat/conversations?limit=2&offset=2")
        .header("authorization", &auth_token)
        .send(router)
        .await;

    let page2: ConversationListResponse = page2_response.json();
    assert_eq!(page2.conversations.len(), 2);
}

// ============================================================================
// Not Found Tests
// ============================================================================

#[tokio::test]
async fn test_get_nonexistent_conversation() {
    let (router, auth_token) = setup_test_environment().await;

    let response = AxumTestRequest::get("/api/chat/conversations/nonexistent-id")
        .header("authorization", &auth_token)
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_nonexistent_conversation() {
    let (router, auth_token) = setup_test_environment().await;

    let response = AxumTestRequest::put("/api/chat/conversations/nonexistent-id")
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "New Title"
        }))
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_nonexistent_conversation() {
    let (router, auth_token) = setup_test_environment().await;

    let response = AxumTestRequest::delete("/api/chat/conversations/nonexistent-id")
        .header("authorization", &auth_token)
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}
