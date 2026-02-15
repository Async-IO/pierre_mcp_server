// ABOUTME: Integration tests for the coaches route handlers
// ABOUTME: Tests coach CRUD, favorites, usage tracking, and authentication flows
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

mod common;
mod helpers;

use common::{create_test_server_resources, create_test_user, create_test_user_with_email};
use helpers::axum_test::AxumTestRequest;
use pierre_mcp_server::database::coaches::{
    CoachCategory, CoachVisibility, CoachesManager, CreateSystemCoachRequest,
};
use pierre_mcp_server::database_plugins::DatabaseProvider;
use pierre_mcp_server::routes::coaches::{
    CoachResponse, CoachesRoutes, ListCoachesResponse, RecordUsageResponse, ToggleFavoriteResponse,
};

use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

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

    // Create the coaches router
    let router = CoachesRoutes::routes(resources);

    (router, format!("Bearer {token}"))
}

// ============================================================================
// Coach CRUD Tests
// ============================================================================

#[tokio::test]
async fn test_create_coach() {
    let (router, auth_token) = setup_test_environment().await;

    let response = AxumTestRequest::post("/api/coaches")
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "Marathon Coach",
            "system_prompt": "You are an expert marathon training coach.",
            "description": "Helps with marathon training plans",
            "category": "training",
            "tags": ["running", "marathon", "endurance"]
        }))
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::CREATED);

    let coach: CoachResponse = response.json();
    assert_eq!(coach.title, "Marathon Coach");
    assert_eq!(
        coach.system_prompt,
        "You are an expert marathon training coach."
    );
    assert_eq!(
        coach.description,
        Some("Helps with marathon training plans".to_owned())
    );
    assert_eq!(coach.category, "training");
    assert_eq!(coach.tags, vec!["running", "marathon", "endurance"]);
    assert!(!coach.is_favorite);
    assert_eq!(coach.use_count, 0);
}

#[tokio::test]
async fn test_create_coach_minimal() {
    let (router, auth_token) = setup_test_environment().await;

    let response = AxumTestRequest::post("/api/coaches")
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "Simple Coach",
            "system_prompt": "You are helpful."
        }))
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::CREATED);

    let coach: CoachResponse = response.json();
    assert_eq!(coach.title, "Simple Coach");
    assert_eq!(coach.system_prompt, "You are helpful.");
    assert!(coach.description.is_none());
    assert_eq!(coach.category, "custom"); // default category
    assert!(coach.tags.is_empty());
}

#[tokio::test]
async fn test_list_coaches() {
    let (router, auth_token) = setup_test_environment().await;

    // Create a coach first
    let create_response = AxumTestRequest::post("/api/coaches")
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "Test Coach",
            "system_prompt": "Test prompt"
        }))
        .send(router.clone())
        .await;

    assert_eq!(create_response.status_code(), StatusCode::CREATED);

    // List coaches
    let list_response = AxumTestRequest::get("/api/coaches")
        .header("authorization", &auth_token)
        .send(router)
        .await;

    assert_eq!(list_response.status_code(), StatusCode::OK);

    let list: ListCoachesResponse = list_response.json();
    assert_eq!(list.total, 1);
    assert_eq!(list.coaches.len(), 1);
    assert_eq!(list.coaches[0].title, "Test Coach");
}

#[tokio::test]
async fn test_get_coach() {
    let (router, auth_token) = setup_test_environment().await;

    // Create a coach first
    let create_response = AxumTestRequest::post("/api/coaches")
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "Get Test Coach",
            "system_prompt": "Test prompt"
        }))
        .send(router.clone())
        .await;

    let created: CoachResponse = create_response.json();

    // Get the coach
    let get_response = AxumTestRequest::get(&format!("/api/coaches/{}", created.id))
        .header("authorization", &auth_token)
        .send(router)
        .await;

    assert_eq!(get_response.status_code(), StatusCode::OK);

    let coach: CoachResponse = get_response.json();
    assert_eq!(coach.id, created.id);
    assert_eq!(coach.title, "Get Test Coach");
}

#[tokio::test]
async fn test_update_coach() {
    let (router, auth_token) = setup_test_environment().await;

    // Create a coach first
    let create_response = AxumTestRequest::post("/api/coaches")
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "Original Title",
            "system_prompt": "Original prompt"
        }))
        .send(router.clone())
        .await;

    let created: CoachResponse = create_response.json();

    // Update the coach
    let update_response = AxumTestRequest::put(&format!("/api/coaches/{}", created.id))
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "Updated Title",
            "system_prompt": "Updated prompt",
            "category": "nutrition"
        }))
        .send(router.clone())
        .await;

    assert_eq!(update_response.status_code(), StatusCode::OK);

    // Verify the update
    let get_response = AxumTestRequest::get(&format!("/api/coaches/{}", created.id))
        .header("authorization", &auth_token)
        .send(router)
        .await;

    let coach: CoachResponse = get_response.json();
    assert_eq!(coach.title, "Updated Title");
    assert_eq!(coach.system_prompt, "Updated prompt");
    assert_eq!(coach.category, "nutrition");
}

#[tokio::test]
async fn test_delete_coach() {
    let (router, auth_token) = setup_test_environment().await;

    // Create a coach first
    let create_response = AxumTestRequest::post("/api/coaches")
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "To Delete",
            "system_prompt": "Will be deleted"
        }))
        .send(router.clone())
        .await;

    let created: CoachResponse = create_response.json();

    // Delete the coach
    let delete_response = AxumTestRequest::delete(&format!("/api/coaches/{}", created.id))
        .header("authorization", &auth_token)
        .send(router.clone())
        .await;

    assert_eq!(delete_response.status_code(), StatusCode::NO_CONTENT);

    // Verify deletion - should return 404
    let get_response = AxumTestRequest::get(&format!("/api/coaches/{}", created.id))
        .header("authorization", &auth_token)
        .send(router)
        .await;

    assert_eq!(get_response.status_code(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Favorites Tests
// ============================================================================

#[tokio::test]
async fn test_toggle_favorite() {
    let (router, auth_token) = setup_test_environment().await;

    // Create a coach
    let create_response = AxumTestRequest::post("/api/coaches")
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "Favorite Test",
            "system_prompt": "Test prompt"
        }))
        .send(router.clone())
        .await;

    let created: CoachResponse = create_response.json();
    assert!(!created.is_favorite);

    // Toggle favorite ON
    let toggle_response = AxumTestRequest::post(&format!("/api/coaches/{}/favorite", created.id))
        .header("authorization", &auth_token)
        .send(router.clone())
        .await;

    assert_eq!(toggle_response.status_code(), StatusCode::OK);
    let toggle_result: ToggleFavoriteResponse = toggle_response.json();
    assert!(toggle_result.is_favorite);

    // Toggle favorite OFF
    let toggle_response = AxumTestRequest::post(&format!("/api/coaches/{}/favorite", created.id))
        .header("authorization", &auth_token)
        .send(router)
        .await;

    assert_eq!(toggle_response.status_code(), StatusCode::OK);
    let toggle_result: ToggleFavoriteResponse = toggle_response.json();
    assert!(!toggle_result.is_favorite);
}

#[tokio::test]
async fn test_list_favorites_only() {
    let (router, auth_token) = setup_test_environment().await;

    // Create two coaches
    let create1 = AxumTestRequest::post("/api/coaches")
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "Coach 1",
            "system_prompt": "Prompt 1"
        }))
        .send(router.clone())
        .await;
    let coach1: CoachResponse = create1.json();

    AxumTestRequest::post("/api/coaches")
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "Coach 2",
            "system_prompt": "Prompt 2"
        }))
        .send(router.clone())
        .await;

    // Mark coach1 as favorite
    AxumTestRequest::post(&format!("/api/coaches/{}/favorite", coach1.id))
        .header("authorization", &auth_token)
        .send(router.clone())
        .await;

    // List only favorites
    let list_response = AxumTestRequest::get("/api/coaches?favorites_only=true")
        .header("authorization", &auth_token)
        .send(router)
        .await;

    assert_eq!(list_response.status_code(), StatusCode::OK);

    let list: ListCoachesResponse = list_response.json();
    // total shows all coaches, coaches.len() shows filtered result
    assert_eq!(list.total, 2);
    assert_eq!(list.coaches.len(), 1);
    assert_eq!(list.coaches[0].title, "Coach 1");
}

// ============================================================================
// Usage Tracking Tests
// ============================================================================

#[tokio::test]
async fn test_record_usage() {
    let (router, auth_token) = setup_test_environment().await;

    // Create a coach
    let create_response = AxumTestRequest::post("/api/coaches")
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "Usage Test",
            "system_prompt": "Test prompt"
        }))
        .send(router.clone())
        .await;

    let created: CoachResponse = create_response.json();
    assert_eq!(created.use_count, 0);

    // Record usage
    let usage_response = AxumTestRequest::post(&format!("/api/coaches/{}/usage", created.id))
        .header("authorization", &auth_token)
        .send(router.clone())
        .await;

    assert_eq!(usage_response.status_code(), StatusCode::OK);
    let usage_result: RecordUsageResponse = usage_response.json();
    assert!(usage_result.success);

    // Verify use_count increased
    let get_response = AxumTestRequest::get(&format!("/api/coaches/{}", created.id))
        .header("authorization", &auth_token)
        .send(router)
        .await;

    let coach: CoachResponse = get_response.json();
    assert_eq!(coach.use_count, 1);
}

// ============================================================================
// Search Tests
// ============================================================================

#[tokio::test]
async fn test_search_coaches() {
    let (router, auth_token) = setup_test_environment().await;

    // Create coaches with different content
    AxumTestRequest::post("/api/coaches")
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "Marathon Training Expert",
            "system_prompt": "Running coach",
            "tags": ["marathon", "running"]
        }))
        .send(router.clone())
        .await;

    AxumTestRequest::post("/api/coaches")
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "Nutrition Advisor",
            "system_prompt": "Diet coach",
            "tags": ["diet", "nutrition"]
        }))
        .send(router.clone())
        .await;

    // Search for "marathon"
    let search_response = AxumTestRequest::get("/api/coaches/search?q=marathon")
        .header("authorization", &auth_token)
        .send(router)
        .await;

    assert_eq!(search_response.status_code(), StatusCode::OK);

    let results: ListCoachesResponse = search_response.json();
    assert_eq!(results.total, 1);
    assert_eq!(results.coaches[0].title, "Marathon Training Expert");
}

// ============================================================================
// Category Filter Tests
// ============================================================================

#[tokio::test]
async fn test_list_by_category() {
    let (router, auth_token) = setup_test_environment().await;

    // Create coaches in different categories
    AxumTestRequest::post("/api/coaches")
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "Training Coach",
            "system_prompt": "Training",
            "category": "training"
        }))
        .send(router.clone())
        .await;

    AxumTestRequest::post("/api/coaches")
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "Nutrition Coach",
            "system_prompt": "Nutrition",
            "category": "nutrition"
        }))
        .send(router.clone())
        .await;

    // List only training coaches
    let list_response = AxumTestRequest::get("/api/coaches?category=training")
        .header("authorization", &auth_token)
        .send(router)
        .await;

    assert_eq!(list_response.status_code(), StatusCode::OK);

    let list: ListCoachesResponse = list_response.json();
    // total shows all coaches, coaches.len() shows filtered result
    assert_eq!(list.total, 2);
    assert_eq!(list.coaches.len(), 1);
    assert_eq!(list.coaches[0].category, "training");
}

// ============================================================================
// Pagination Tests
// ============================================================================

#[tokio::test]
async fn test_list_coaches_pagination() {
    let (router, auth_token) = setup_test_environment().await;

    // Create multiple coaches
    for i in 1..=5 {
        AxumTestRequest::post("/api/coaches")
            .header("authorization", &auth_token)
            .json(&json!({
                "title": format!("Coach {}", i),
                "system_prompt": format!("Prompt {}", i)
            }))
            .send(router.clone())
            .await;
    }

    // Get first page (limit=2)
    let page1_response = AxumTestRequest::get("/api/coaches?limit=2&offset=0")
        .header("authorization", &auth_token)
        .send(router.clone())
        .await;

    let page1: ListCoachesResponse = page1_response.json();
    assert_eq!(page1.coaches.len(), 2);
    assert_eq!(page1.total, 5);

    // Get second page
    let page2_response = AxumTestRequest::get("/api/coaches?limit=2&offset=2")
        .header("authorization", &auth_token)
        .send(router)
        .await;

    let page2: ListCoachesResponse = page2_response.json();
    assert_eq!(page2.coaches.len(), 2);
}

// ============================================================================
// Authentication Tests
// ============================================================================

#[tokio::test]
async fn test_create_coach_unauthorized() {
    let (router, _) = setup_test_environment().await;

    let response = AxumTestRequest::post("/api/coaches")
        .json(&json!({
            "title": "Test Coach",
            "system_prompt": "Test"
        }))
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_coach_invalid_token() {
    let (router, _) = setup_test_environment().await;

    let response = AxumTestRequest::post("/api/coaches")
        .header("authorization", "Bearer invalid_token")
        .json(&json!({
            "title": "Test Coach",
            "system_prompt": "Test"
        }))
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Not Found Tests
// ============================================================================

#[tokio::test]
async fn test_get_nonexistent_coach() {
    let (router, auth_token) = setup_test_environment().await;

    let response = AxumTestRequest::get("/api/coaches/nonexistent-id")
        .header("authorization", &auth_token)
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_nonexistent_coach() {
    let (router, auth_token) = setup_test_environment().await;

    let response = AxumTestRequest::put("/api/coaches/nonexistent-id")
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "New Title"
        }))
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_nonexistent_coach() {
    let (router, auth_token) = setup_test_environment().await;

    let response = AxumTestRequest::delete("/api/coaches/nonexistent-id")
        .header("authorization", &auth_token)
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_toggle_favorite_nonexistent() {
    let (router, auth_token) = setup_test_environment().await;

    let response = AxumTestRequest::post("/api/coaches/nonexistent-id/favorite")
        .header("authorization", &auth_token)
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_record_usage_nonexistent() {
    let (router, auth_token) = setup_test_environment().await;

    let response = AxumTestRequest::post("/api/coaches/nonexistent-id/usage")
        .header("authorization", &auth_token)
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

// ============================================================================
// System Coach Cross-Tenant Visibility E2E Tests
// ============================================================================

/// E2E test: System coaches should be visible to users via the API
/// This tests the full flow from database to API response
#[tokio::test]
async fn test_system_coaches_visible_in_list() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, user) = create_test_user(&resources.database).await.unwrap();

    // Get the user's tenant from the tenant where they are the owner
    let all_tenants = resources.database.get_all_tenants().await.unwrap();
    let user_tenant = all_tenants
        .iter()
        .find(|t| t.owner_user_id == user_id)
        .unwrap();
    let tenant_id = user_tenant.id;

    // Create a system coach directly in the database
    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let coaches_manager = CoachesManager::new(sqlite_pool);
    let system_request = CreateSystemCoachRequest {
        title: "Platform Coach".to_owned(),
        description: Some("A system-wide coach".to_owned()),
        system_prompt: "You are a platform-wide fitness coach.".to_owned(),
        category: CoachCategory::Training,
        tags: vec!["system".to_owned()],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };
    let system_coach = coaches_manager
        .create_system_coach(user_id, tenant_id, &system_request)
        .await
        .unwrap();

    assert!(system_coach.is_system);

    // Generate a JWT token for the user
    let token = resources
        .auth_manager
        .generate_token(&user, &resources.jwks_manager)
        .unwrap();
    let auth_token = format!("Bearer {token}");

    // Create the coaches router
    let router = CoachesRoutes::routes(resources);

    // List coaches via the API - should include the system coach
    let list_response = AxumTestRequest::get("/api/coaches")
        .header("authorization", &auth_token)
        .send(router)
        .await;

    assert_eq!(list_response.status_code(), StatusCode::OK);

    let list: ListCoachesResponse = list_response.json();
    // Should have at least 1 coach (the system coach)
    assert!(!list.coaches.is_empty());

    // Find the system coach in the response
    let found_system_coach = list.coaches.iter().find(|c| c.title == "Platform Coach");
    assert!(
        found_system_coach.is_some(),
        "System coach should be visible in the list"
    );
    assert!(
        found_system_coach.unwrap().is_system,
        "Coach should be marked as system"
    );
}

/// E2E test: System coaches should be retrievable by ID
#[tokio::test]
async fn test_get_system_coach_by_id() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, user) = create_test_user(&resources.database).await.unwrap();

    // Get the user's tenant from the tenant where they are the owner
    let all_tenants = resources.database.get_all_tenants().await.unwrap();
    let user_tenant = all_tenants
        .iter()
        .find(|t| t.owner_user_id == user_id)
        .unwrap();
    let tenant_id = user_tenant.id;

    // Create a system coach
    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let coaches_manager = CoachesManager::new(sqlite_pool);
    let system_request = CreateSystemCoachRequest {
        title: "Retrievable Coach".to_owned(),
        description: None,
        system_prompt: "You are a coach.".to_owned(),
        category: CoachCategory::Training,
        tags: vec![],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };
    let system_coach = coaches_manager
        .create_system_coach(user_id, tenant_id, &system_request)
        .await
        .unwrap();

    // Generate JWT token
    let token = resources
        .auth_manager
        .generate_token(&user, &resources.jwks_manager)
        .unwrap();
    let auth_token = format!("Bearer {token}");

    let router = CoachesRoutes::routes(resources);

    // Get the system coach by ID via the API
    let get_response = AxumTestRequest::get(&format!("/api/coaches/{}", system_coach.id))
        .header("authorization", &auth_token)
        .send(router)
        .await;

    assert_eq!(get_response.status_code(), StatusCode::OK);

    let coach: CoachResponse = get_response.json();
    assert_eq!(coach.id, system_coach.id.to_string());
    assert_eq!(coach.title, "Retrievable Coach");
    assert!(coach.is_system);
}

// ============================================================================
// Hide/Show Coach E2E Tests
// ============================================================================

/// E2E test: User can hide a system coach via the API
#[tokio::test]
async fn test_hide_system_coach_via_api() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, user) = create_test_user(&resources.database).await.unwrap();

    // Get the user's tenant from the tenant where they are the owner
    let all_tenants = resources.database.get_all_tenants().await.unwrap();
    let user_tenant = all_tenants
        .iter()
        .find(|t| t.owner_user_id == user_id)
        .unwrap();
    let tenant_id = user_tenant.id;

    // Create a system coach
    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let coaches_manager = CoachesManager::new(sqlite_pool);
    let system_request = CreateSystemCoachRequest {
        title: "Hideable Coach".to_owned(),
        description: None,
        system_prompt: "You are a coach.".to_owned(),
        category: CoachCategory::Training,
        tags: vec![],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };
    let system_coach = coaches_manager
        .create_system_coach(user_id, tenant_id, &system_request)
        .await
        .unwrap();

    // Generate JWT token
    let token = resources
        .auth_manager
        .generate_token(&user, &resources.jwks_manager)
        .unwrap();
    let auth_token = format!("Bearer {token}");

    let router = CoachesRoutes::routes(resources);

    // Hide the system coach via the API
    let hide_response = AxumTestRequest::post(&format!("/api/coaches/{}/hide", system_coach.id))
        .header("authorization", &auth_token)
        .send(router.clone())
        .await;

    assert_eq!(hide_response.status_code(), StatusCode::OK);

    // Verify the coach is hidden by listing (without include_hidden)
    let list_response = AxumTestRequest::get("/api/coaches")
        .header("authorization", &auth_token)
        .send(router)
        .await;

    assert_eq!(list_response.status_code(), StatusCode::OK);

    let list: ListCoachesResponse = list_response.json();
    // The coach should not appear in the list (it's hidden)
    let found_coach = list.coaches.iter().find(|c| c.title == "Hideable Coach");
    assert!(
        found_coach.is_none(),
        "Hidden coach should not appear in list"
    );
}

/// E2E test: User can show (unhide) a hidden coach via the API
#[tokio::test]
async fn test_show_hidden_coach_via_api() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, user) = create_test_user(&resources.database).await.unwrap();

    // Get the user's tenant from the tenant where they are the owner
    let all_tenants = resources.database.get_all_tenants().await.unwrap();
    let user_tenant = all_tenants
        .iter()
        .find(|t| t.owner_user_id == user_id)
        .unwrap();
    let tenant_id = user_tenant.id;

    // Create a system coach
    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let coaches_manager = CoachesManager::new(sqlite_pool.clone());
    let system_request = CreateSystemCoachRequest {
        title: "Show Me Coach".to_owned(),
        description: None,
        system_prompt: "You are a coach.".to_owned(),
        category: CoachCategory::Training,
        tags: vec![],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };
    let system_coach = coaches_manager
        .create_system_coach(user_id, tenant_id, &system_request)
        .await
        .unwrap();

    // Hide the coach first (directly via manager)
    coaches_manager
        .hide_coach(&system_coach.id.to_string(), user_id)
        .await
        .unwrap();

    // Generate JWT token
    let token = resources
        .auth_manager
        .generate_token(&user, &resources.jwks_manager)
        .unwrap();
    let auth_token = format!("Bearer {token}");

    let router = CoachesRoutes::routes(resources);

    // Show (unhide) the coach via the API - DELETE removes the hide preference
    let show_response = AxumTestRequest::delete(&format!("/api/coaches/{}/hide", system_coach.id))
        .header("authorization", &auth_token)
        .send(router.clone())
        .await;

    assert_eq!(show_response.status_code(), StatusCode::OK);

    // Verify the coach is now visible by listing
    let list_response = AxumTestRequest::get("/api/coaches")
        .header("authorization", &auth_token)
        .send(router)
        .await;

    assert_eq!(list_response.status_code(), StatusCode::OK);

    let list: ListCoachesResponse = list_response.json();
    // The coach should now appear in the list
    let found_coach = list.coaches.iter().find(|c| c.title == "Show Me Coach");
    assert!(
        found_coach.is_some(),
        "Unhidden coach should appear in list"
    );
}

/// E2E test: Hidden coaches appear when `include_hidden=true`
#[tokio::test]
async fn test_list_with_include_hidden() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, user) = create_test_user(&resources.database).await.unwrap();

    // Get the user's tenant from the tenant where they are the owner
    let all_tenants = resources.database.get_all_tenants().await.unwrap();
    let user_tenant = all_tenants
        .iter()
        .find(|t| t.owner_user_id == user_id)
        .unwrap();
    let tenant_id = user_tenant.id;

    // Create a system coach
    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let coaches_manager = CoachesManager::new(sqlite_pool.clone());
    let system_request = CreateSystemCoachRequest {
        title: "Hidden But Findable".to_owned(),
        description: None,
        system_prompt: "You are a coach.".to_owned(),
        category: CoachCategory::Training,
        tags: vec![],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };
    let system_coach = coaches_manager
        .create_system_coach(user_id, tenant_id, &system_request)
        .await
        .unwrap();

    // Hide the coach
    coaches_manager
        .hide_coach(&system_coach.id.to_string(), user_id)
        .await
        .unwrap();

    // Generate JWT token
    let token = resources
        .auth_manager
        .generate_token(&user, &resources.jwks_manager)
        .unwrap();
    let auth_token = format!("Bearer {token}");

    let router = CoachesRoutes::routes(resources);

    // List with include_hidden=true
    let list_response = AxumTestRequest::get("/api/coaches?include_hidden=true")
        .header("authorization", &auth_token)
        .send(router)
        .await;

    assert_eq!(list_response.status_code(), StatusCode::OK);

    let list: ListCoachesResponse = list_response.json();
    // The hidden coach should appear when include_hidden=true
    let found_coach = list
        .coaches
        .iter()
        .find(|c| c.title == "Hidden But Findable");
    assert!(
        found_coach.is_some(),
        "Hidden coach should appear when include_hidden=true"
    );
}

// ============================================================================
// Coach Generation from Conversation Tests
// ============================================================================

/// Generate coach endpoint requires authentication
#[tokio::test]
async fn test_generate_coach_requires_auth() {
    let (router, _auth_token) = setup_test_environment().await;

    // Try without auth token
    let response = AxumTestRequest::post("/api/coaches/generate")
        .json(&json!({
            "conversation_id": "00000000-0000-0000-0000-000000000001"
        }))
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

/// Generate coach endpoint returns 404 for non-existent conversation
#[tokio::test]
async fn test_generate_coach_nonexistent_conversation() {
    let (router, auth_token) = setup_test_environment().await;

    let fake_id = Uuid::new_v4().to_string();
    let response = AxumTestRequest::post("/api/coaches/generate")
        .header("authorization", &auth_token)
        .json(&json!({
            "conversation_id": fake_id
        }))
        .send(router)
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

/// Generate coach endpoint returns 400 for conversation with no messages
#[tokio::test]
async fn test_generate_coach_empty_conversation() {
    use pierre_mcp_server::routes::chat::ChatRoutes;

    #[derive(serde::Deserialize)]
    struct ConvResponse {
        id: String,
    }

    let resources = create_test_server_resources().await.unwrap();
    let (_user_id, user) = create_test_user(&resources.database).await.unwrap();

    let token = resources
        .auth_manager
        .generate_token(&user, &resources.jwks_manager)
        .unwrap();
    let auth_token = format!("Bearer {token}");

    // Create a conversation first via chat routes
    let chat_router = ChatRoutes::routes(resources.clone());

    let create_response = AxumTestRequest::post("/api/chat/conversations")
        .header("authorization", &auth_token)
        .json(&json!({
            "title": "Empty Conversation",
            "model": "gemini-1.5-flash"
        }))
        .send(chat_router)
        .await;

    assert_eq!(create_response.status_code(), StatusCode::CREATED);

    let conv: ConvResponse = create_response.json();

    // Now try to generate coach from empty conversation
    let coaches_router = CoachesRoutes::routes(resources);

    let response = AxumTestRequest::post("/api/coaches/generate")
        .header("authorization", &auth_token)
        .json(&json!({
            "conversation_id": conv.id
        }))
        .send(coaches_router)
        .await;

    // Should return 400 because conversation has no messages
    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
}

/// Generate coach endpoint returns 404 when accessing another user's conversation
#[tokio::test]
async fn test_generate_coach_other_users_conversation() {
    use pierre_mcp_server::routes::chat::ChatRoutes;

    #[derive(serde::Deserialize)]
    struct ConvResponse {
        id: String,
    }

    let resources = create_test_server_resources().await.unwrap();

    // Create first user and their conversation
    let (_user1_id, user1) = create_test_user(&resources.database).await.unwrap();
    let token1 = resources
        .auth_manager
        .generate_token(&user1, &resources.jwks_manager)
        .unwrap();
    let auth_token1 = format!("Bearer {token1}");

    // Create a conversation for user1
    let chat_router = ChatRoutes::routes(resources.clone());

    let create_response = AxumTestRequest::post("/api/chat/conversations")
        .header("authorization", &auth_token1)
        .json(&json!({
            "title": "User1 Conversation",
            "model": "gemini-1.5-flash"
        }))
        .send(chat_router)
        .await;

    assert_eq!(create_response.status_code(), StatusCode::CREATED);

    let conv: ConvResponse = create_response.json();

    // Create second user with different email
    let (_user2_id, user2) = create_test_user_with_email(&resources.database, "user2@example.com")
        .await
        .unwrap();
    let token2 = resources
        .auth_manager
        .generate_token(&user2, &resources.jwks_manager)
        .unwrap();
    let auth_token2 = format!("Bearer {token2}");

    // Try to generate coach from user1's conversation as user2
    let coaches_router = CoachesRoutes::routes(resources);

    let response = AxumTestRequest::post("/api/coaches/generate")
        .header("authorization", &auth_token2)
        .json(&json!({
            "conversation_id": conv.id
        }))
        .send(coaches_router)
        .await;

    // Should return 404 (not found for security - don't reveal conversation exists)
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}
