// ABOUTME: Integration tests for social route handlers
// ABOUTME: Tests friend connections, pending requests, and user info in responses
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

mod common;
mod helpers;

use common::{create_test_server_resources, create_test_user_with_email};
use helpers::axum_test::AxumTestRequest;
use pierre_mcp_server::routes::social::{
    ListFriendsResponse, PendingRequestsResponse, SocialRoutes,
};

use axum::http::StatusCode;
use serde_json::json;
use std::sync::Arc;

// ============================================================================
// Test Helpers
// ============================================================================

async fn setup_two_users() -> (axum::Router, String, String, String, String) {
    let resources = create_test_server_resources().await.unwrap();

    // Create first user (sender)
    let (user1_id, user1) = create_test_user_with_email(&resources.database, "user1@example.com")
        .await
        .unwrap();

    // Create second user (receiver)
    let (user2_id, user2) = create_test_user_with_email(&resources.database, "user2@example.com")
        .await
        .unwrap();

    // Generate JWT tokens for both users
    let token1 = resources
        .auth_manager
        .generate_token(&user1, &resources.jwks_manager)
        .unwrap();

    let token2 = resources
        .auth_manager
        .generate_token(&user2, &resources.jwks_manager)
        .unwrap();

    // Create the social router
    let router = SocialRoutes::routes(Arc::clone(&resources));

    (
        router,
        format!("Bearer {token1}"),
        format!("Bearer {token2}"),
        user1_id.to_string(),
        user2_id.to_string(),
    )
}

// ============================================================================
// Friend Connection Tests
// ============================================================================

#[tokio::test]
async fn test_send_friend_request() {
    let (router, auth_token1, _auth_token2, _user1_id, user2_id) = setup_two_users().await;

    // Send friend request from user1 to user2
    let response = AxumTestRequest::post("/api/social/friends")
        .header("authorization", &auth_token1)
        .json(&json!({
            "receiver_id": user2_id
        }))
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::CREATED);

    let result: serde_json::Value = response.json();
    assert_eq!(result["status"], "pending");
    assert!(result["id"].as_str().is_some());
}

#[tokio::test]
async fn test_pending_requests_returns_user_info() {
    let (router, auth_token1, auth_token2, _user1_id, user2_id) = setup_two_users().await;

    // User1 sends friend request to User2
    let response = AxumTestRequest::post("/api/social/friends")
        .header("authorization", &auth_token1)
        .json(&json!({
            "receiver_id": user2_id
        }))
        .send(router.clone())
        .await;

    assert_eq!(response.status_code(), StatusCode::CREATED);

    // User2 checks pending requests - should see User1's info
    let response = AxumTestRequest::get("/api/social/friends/pending")
        .header("authorization", &auth_token2)
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let result: PendingRequestsResponse = response.json();
    assert_eq!(result.received.len(), 1);

    let request = &result.received[0];
    // Verify user info is present (sender's info)
    assert!(
        !request.user_email.is_empty(),
        "user_email should not be empty"
    );
    assert!(!request.user_id.is_empty(), "user_id should not be empty");
    assert!(
        request.user_email.contains("user1"),
        "Should contain sender's email: {}",
        request.user_email
    );
}

#[tokio::test]
async fn test_accept_friend_request() {
    let (router, auth_token1, auth_token2, _user1_id, user2_id) = setup_two_users().await;

    // User1 sends friend request
    let response = AxumTestRequest::post("/api/social/friends")
        .header("authorization", &auth_token1)
        .json(&json!({
            "receiver_id": user2_id
        }))
        .send(router.clone())
        .await;

    assert_eq!(response.status_code(), StatusCode::CREATED);
    let request_result: serde_json::Value = response.json();
    let connection_id = request_result["id"].as_str().unwrap();

    // User2 accepts the request
    let response = AxumTestRequest::post(&format!("/api/social/friends/{connection_id}/accept"))
        .header("authorization", &auth_token2)
        .send(router.clone())
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let result: serde_json::Value = response.json();
    assert_eq!(result["status"], "accepted");

    // Verify User1 now sees User2 as a friend
    let response = AxumTestRequest::get("/api/social/friends")
        .header("authorization", &auth_token1)
        .send(router.clone())
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let friends: ListFriendsResponse = response.json();
    assert_eq!(friends.friends.len(), 1);
}

#[tokio::test]
async fn test_list_friends_returns_user_info() {
    let (router, auth_token1, auth_token2, _user1_id, user2_id) = setup_two_users().await;

    // User1 sends friend request to User2
    let response = AxumTestRequest::post("/api/social/friends")
        .header("authorization", &auth_token1)
        .json(&json!({
            "receiver_id": user2_id
        }))
        .send(router.clone())
        .await;

    assert_eq!(response.status_code(), StatusCode::CREATED);
    let request_result: serde_json::Value = response.json();
    let connection_id = request_result["id"].as_str().unwrap();

    // User2 accepts the request
    let response = AxumTestRequest::post(&format!("/api/social/friends/{connection_id}/accept"))
        .header("authorization", &auth_token2)
        .send(router.clone())
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    // User1 lists friends - should have user info
    let response = AxumTestRequest::get("/api/social/friends")
        .header("authorization", &auth_token1)
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let result: ListFriendsResponse = response.json();
    assert_eq!(result.friends.len(), 1);

    let friend = &result.friends[0];
    // Verify user info is present
    assert!(
        !friend.friend_email.is_empty(),
        "friend_email should not be empty"
    );
    assert!(
        !friend.friend_user_id.is_empty(),
        "friend_user_id should not be empty"
    );
    assert!(
        friend.friend_email.contains("user2"),
        "Friend info should contain user2's email: {}",
        friend.friend_email
    );
}

#[tokio::test]
async fn test_decline_friend_request() {
    let (router, auth_token1, auth_token2, _user1_id, user2_id) = setup_two_users().await;

    // User1 sends friend request
    let response = AxumTestRequest::post("/api/social/friends")
        .header("authorization", &auth_token1)
        .json(&json!({
            "receiver_id": user2_id
        }))
        .send(router.clone())
        .await;

    assert_eq!(response.status_code(), StatusCode::CREATED);
    let request_result: serde_json::Value = response.json();
    let connection_id = request_result["id"].as_str().unwrap();

    // User2 declines the request
    let response = AxumTestRequest::post(&format!("/api/social/friends/{connection_id}/decline"))
        .header("authorization", &auth_token2)
        .send(router.clone())
        .await;

    // Decline returns 204 NO_CONTENT on success
    assert_eq!(response.status_code(), StatusCode::NO_CONTENT);

    // Verify no pending requests for User2
    let response = AxumTestRequest::get("/api/social/friends/pending")
        .header("authorization", &auth_token2)
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let pending: PendingRequestsResponse = response.json();
    assert_eq!(pending.received.len(), 0);
}

#[tokio::test]
async fn test_list_friends_with_pagination() {
    let (router, auth_token1, auth_token2, _user1_id, user2_id) = setup_two_users().await;

    // Create a friendship first
    let response = AxumTestRequest::post("/api/social/friends")
        .header("authorization", &auth_token1)
        .json(&json!({
            "receiver_id": user2_id
        }))
        .send(router.clone())
        .await;

    let request_result: serde_json::Value = response.json();
    let connection_id = request_result["id"].as_str().unwrap();

    AxumTestRequest::post(&format!("/api/social/friends/{connection_id}/accept"))
        .header("authorization", &auth_token2)
        .send(router.clone())
        .await;

    // Test with limit parameter
    let response = AxumTestRequest::get("/api/social/friends?limit=10")
        .header("authorization", &auth_token1)
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let result: ListFriendsResponse = response.json();
    assert!(result.friends.len() <= 10);
    assert!(!result.metadata.timestamp.is_empty());
}

// ============================================================================
// Authentication Tests
// ============================================================================

#[tokio::test]
async fn test_friends_endpoint_requires_auth() {
    let (router, _auth_token1, _auth_token2, _user1_id, _user2_id) = setup_two_users().await;

    // Try without auth token
    let response = AxumTestRequest::get("/api/social/friends")
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_pending_requests_requires_auth() {
    let (router, _auth_token1, _auth_token2, _user1_id, _user2_id) = setup_two_users().await;

    let response = AxumTestRequest::get("/api/social/friends/pending")
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_sent_pending_requests_have_receiver_info() {
    let (router, auth_token1, _auth_token2, _user1_id, user2_id) = setup_two_users().await;

    // User1 sends friend request to User2
    let response = AxumTestRequest::post("/api/social/friends")
        .header("authorization", &auth_token1)
        .json(&json!({
            "receiver_id": user2_id
        }))
        .send(router.clone())
        .await;

    assert_eq!(response.status_code(), StatusCode::CREATED);

    // User1 checks their sent pending requests - should see User2's info
    let response = AxumTestRequest::get("/api/social/friends/pending")
        .header("authorization", &auth_token1)
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let result: PendingRequestsResponse = response.json();
    assert_eq!(result.sent.len(), 1);

    let request = &result.sent[0];
    // Verify receiver's user info is present
    assert!(
        !request.user_email.is_empty(),
        "user_email should not be empty"
    );
    assert!(!request.user_id.is_empty(), "user_id should not be empty");
    assert!(
        request.user_email.contains("user2"),
        "Should contain receiver's email: {}",
        request.user_email
    );
}
