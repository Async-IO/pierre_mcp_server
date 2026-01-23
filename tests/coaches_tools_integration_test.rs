// ABOUTME: Integration tests for coach MCP tool handlers (custom AI personas)
// ABOUTME: Tests all 10 coach tools via the UniversalToolExecutor for MCP protocol compliance
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Coach Tool Handler Integration Tests
//!
//! Tests the 10 coach MCP tools via the `UniversalToolExecutor`:
//! - `list_coaches`: List user's coaches with filtering
//! - `create_coach`: Create a new custom coach
//! - `get_coach`: Get a specific coach by ID
//! - `update_coach`: Update coach details
//! - `delete_coach`: Delete a coach
//! - `toggle_coach_favorite`: Toggle favorite status
//! - `search_coaches`: Search coaches by query
//! - `activate_coach`: Set a coach as active
//! - `deactivate_coach`: Deactivate active coach
//! - `get_active_coach`: Get currently active coach

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use anyhow::Result;
use pierre_mcp_server::database_plugins::DatabaseProvider;
use pierre_mcp_server::protocols::universal::{UniversalRequest, UniversalToolExecutor};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

mod common;

// ============================================================================
// Test Setup
// ============================================================================

/// Create test executor for coach tool tests
async fn create_coach_test_executor() -> Result<Arc<UniversalToolExecutor>> {
    common::init_server_config();
    common::init_test_http_clients();

    let resources = common::create_test_server_resources().await?;
    Ok(Arc::new(UniversalToolExecutor::new(resources)))
}

/// Create a test user with tenant for coaches tests
/// Uses common helper that properly creates both user and tenant with foreign key relationship
async fn create_test_user_for_coaches(executor: &UniversalToolExecutor) -> Result<(Uuid, String)> {
    let email = format!("coach_test_{}@example.com", Uuid::new_v4());
    let (user_id, _user) =
        common::create_test_user_with_email(&executor.resources.database, &email).await?;
    // Get tenant_id from the tenant where user is owner
    let all_tenants = executor.resources.database.get_all_tenants().await?;
    let user_tenant = all_tenants
        .iter()
        .find(|t| t.owner_user_id == user_id)
        .ok_or_else(|| anyhow::anyhow!("User should have a tenant"))?;
    Ok((user_id, user_tenant.id.to_string()))
}

/// Create a test request with user ID and tenant ID
fn create_test_request(
    tool_name: &str,
    parameters: serde_json::Value,
    user_id: Uuid,
    tenant_id: &str,
) -> UniversalRequest {
    UniversalRequest {
        tool_name: tool_name.to_owned(),
        parameters,
        user_id: user_id.to_string(),
        protocol: "test".to_owned(),
        tenant_id: Some(tenant_id.to_owned()),
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    }
}

// ============================================================================
// Tool Registration Tests
// ============================================================================

#[tokio::test]
async fn test_coach_tools_registered() -> Result<()> {
    let executor = create_coach_test_executor().await?;

    let tool_names: Vec<String> = executor
        .list_tools()
        .iter()
        .map(|tool| tool.name().to_owned())
        .collect();

    let expected_tools = vec![
        "list_coaches",
        "create_coach",
        "get_coach",
        "update_coach",
        "delete_coach",
        "toggle_coach_favorite",
        "search_coaches",
        "activate_coach",
        "deactivate_coach",
        "get_active_coach",
    ];

    for expected_tool in expected_tools {
        assert!(
            tool_names.contains(&expected_tool.to_owned()),
            "Missing coach tool: {expected_tool}"
        );
    }

    Ok(())
}

// ============================================================================
// list_coaches Tests
// ============================================================================

#[tokio::test]
async fn test_list_coaches_empty() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    let request = create_test_request("list_coaches", json!({}), user_id, &tenant_id);

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert!(result["coaches"].is_array());
    assert_eq!(result["count"].as_u64().unwrap(), 0);
    assert_eq!(result["total"].as_u64().unwrap(), 0);

    Ok(())
}

#[tokio::test]
async fn test_list_coaches_after_create() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    // Create a coach first
    let create_request = create_test_request(
        "create_coach",
        json!({
            "title": "Test Marathon Coach",
            "system_prompt": "You are a marathon training specialist.",
            "category": "training"
        }),
        user_id,
        &tenant_id,
    );

    let create_response = executor.execute_tool(create_request).await?;
    assert!(create_response.success);

    // Now list coaches
    let list_request = create_test_request("list_coaches", json!({}), user_id, &tenant_id);

    let response = executor.execute_tool(list_request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert_eq!(result["count"].as_u64().unwrap(), 1);
    assert_eq!(result["total"].as_u64().unwrap(), 1);
    let coaches = result["coaches"].as_array().unwrap();
    assert_eq!(coaches[0]["title"].as_str().unwrap(), "Test Marathon Coach");

    Ok(())
}

#[tokio::test]
async fn test_list_coaches_with_category_filter() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    // Create coaches with different categories
    for (title, category) in [
        ("Training Coach", "training"),
        ("Nutrition Coach", "nutrition"),
        ("Recovery Coach", "recovery"),
    ] {
        let request = create_test_request(
            "create_coach",
            json!({
                "title": title,
                "system_prompt": format!("You are a {} specialist.", category),
                "category": category
            }),
            user_id,
            &tenant_id,
        );
        executor.execute_tool(request).await?;
    }

    // Filter by training category
    let request = create_test_request(
        "list_coaches",
        json!({
            "category": "training"
        }),
        user_id,
        &tenant_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    // count = filtered count, total = all coaches
    assert_eq!(result["count"].as_u64().unwrap(), 1);
    assert_eq!(result["total"].as_u64().unwrap(), 3);

    Ok(())
}

#[tokio::test]
async fn test_list_coaches_with_pagination() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    // Create 5 coaches
    for i in 0..5 {
        let request = create_test_request(
            "create_coach",
            json!({
                "title": format!("Coach {}", i),
                "system_prompt": "Generic coach prompt."
            }),
            user_id,
            &tenant_id,
        );
        executor.execute_tool(request).await?;
    }

    // Get first 2 coaches
    let request = create_test_request(
        "list_coaches",
        json!({
            "limit": 2,
            "offset": 0
        }),
        user_id,
        &tenant_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert_eq!(result["count"].as_u64().unwrap(), 2);
    assert_eq!(result["total"].as_u64().unwrap(), 5);
    assert!(result["has_more"].as_bool().unwrap());

    Ok(())
}

#[tokio::test]
async fn test_list_coaches_favorites_only() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    // Create two coaches
    let create_request = create_test_request(
        "create_coach",
        json!({
            "title": "Favorite Coach",
            "system_prompt": "Prompt"
        }),
        user_id,
        &tenant_id,
    );
    let create_response = executor.execute_tool(create_request).await?;
    let favorite_coach_id = create_response.result.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let create_request2 = create_test_request(
        "create_coach",
        json!({
            "title": "Regular Coach",
            "system_prompt": "Prompt"
        }),
        user_id,
        &tenant_id,
    );
    executor.execute_tool(create_request2).await?;

    // Toggle favorite on first coach
    let toggle_request = create_test_request(
        "toggle_coach_favorite",
        json!({
            "coach_id": favorite_coach_id
        }),
        user_id,
        &tenant_id,
    );
    executor.execute_tool(toggle_request).await?;

    // List favorites only
    let request = create_test_request(
        "list_coaches",
        json!({
            "favorites_only": true
        }),
        user_id,
        &tenant_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert_eq!(result["count"].as_u64().unwrap(), 1);
    let coaches = result["coaches"].as_array().unwrap();
    assert_eq!(coaches[0]["title"].as_str().unwrap(), "Favorite Coach");

    Ok(())
}

// ============================================================================
// create_coach Tests
// ============================================================================

#[tokio::test]
async fn test_create_coach_success() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    let request = create_test_request(
        "create_coach",
        json!({
            "title": "Elite Running Coach",
            "description": "Specializes in marathon and ultra training",
            "system_prompt": "You are an elite running coach with 20 years of experience training professional marathon runners. Focus on periodization, recovery, and race strategy.",
            "category": "training",
            "tags": ["running", "marathon", "elite", "periodization"]
        }),
        user_id,
        &tenant_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(
        response.success,
        "Create should succeed: {:?}",
        response.error
    );
    let result = response.result.unwrap();

    // Verify all returned fields
    assert!(result["id"].is_string(), "Should return coach id");
    assert_eq!(result["title"].as_str().unwrap(), "Elite Running Coach");
    assert_eq!(result["category"].as_str().unwrap(), "training");
    assert!(result["token_count"].as_u64().unwrap() > 0);
    assert!(result["created_at"].is_string());

    // Verify tags
    let tags = result["tags"].as_array().unwrap();
    assert!(tags.iter().any(|t| t.as_str().unwrap() == "marathon"));

    Ok(())
}

#[tokio::test]
async fn test_create_coach_minimal() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    // Minimal required fields only
    let request = create_test_request(
        "create_coach",
        json!({
            "title": "Simple Coach",
            "system_prompt": "A basic coaching prompt."
        }),
        user_id,
        &tenant_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(
        response.success,
        "Create should succeed with minimal fields"
    );
    let result = response.result.unwrap();

    assert!(result["id"].is_string());
    assert_eq!(result["category"].as_str().unwrap(), "custom");

    Ok(())
}

#[tokio::test]
async fn test_create_coach_missing_title() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    let request = create_test_request(
        "create_coach",
        json!({
            "system_prompt": "A prompt without a title."
        }),
        user_id,
        &tenant_id,
    );

    let result = executor.execute_tool(request).await;

    assert!(result.is_err(), "Should fail without title");

    Ok(())
}

#[tokio::test]
async fn test_create_coach_missing_system_prompt() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    let request = create_test_request(
        "create_coach",
        json!({
            "title": "Coach Without Prompt"
        }),
        user_id,
        &tenant_id,
    );

    let result = executor.execute_tool(request).await;

    assert!(result.is_err(), "Should fail without system_prompt");

    Ok(())
}

// ============================================================================
// get_coach Tests
// ============================================================================

#[tokio::test]
async fn test_get_coach_success() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    // Create a coach first
    let create_request = create_test_request(
        "create_coach",
        json!({
            "title": "Detailed Coach",
            "description": "A coach with full details",
            "system_prompt": "You are a detailed coaching assistant.",
            "category": "nutrition",
            "tags": ["detailed", "nutrition"]
        }),
        user_id,
        &tenant_id,
    );

    let create_response = executor.execute_tool(create_request).await?;
    assert!(create_response.success);
    let coach_id = create_response.result.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Get the coach
    let get_request = create_test_request(
        "get_coach",
        json!({
            "coach_id": coach_id
        }),
        user_id,
        &tenant_id,
    );

    let response = executor.execute_tool(get_request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    // Verify all fields including system_prompt (only in get_coach response)
    assert_eq!(result["id"].as_str().unwrap(), coach_id);
    assert_eq!(result["title"].as_str().unwrap(), "Detailed Coach");
    assert_eq!(
        result["description"].as_str().unwrap(),
        "A coach with full details"
    );
    assert!(result["system_prompt"]
        .as_str()
        .unwrap()
        .contains("detailed coaching"));
    assert_eq!(result["category"].as_str().unwrap(), "nutrition");
    assert!(result["created_at"].is_string());
    assert!(result["updated_at"].is_string());

    Ok(())
}

#[tokio::test]
async fn test_get_coach_not_found() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    let request = create_test_request(
        "get_coach",
        json!({
            "coach_id": Uuid::new_v4().to_string()
        }),
        user_id,
        &tenant_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(!response.success, "Should fail for nonexistent coach");
    assert!(response.error.unwrap().contains("not found"));

    Ok(())
}

#[tokio::test]
async fn test_get_coach_missing_id() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    let request = create_test_request("get_coach", json!({}), user_id, &tenant_id);

    let result = executor.execute_tool(request).await;

    assert!(result.is_err(), "Should fail without coach_id");

    Ok(())
}

// ============================================================================
// update_coach Tests
// ============================================================================

#[tokio::test]
async fn test_update_coach_success() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    // Create a coach first
    let create_request = create_test_request(
        "create_coach",
        json!({
            "title": "Original Title",
            "system_prompt": "Original prompt."
        }),
        user_id,
        &tenant_id,
    );

    let create_response = executor.execute_tool(create_request).await?;
    let coach_id = create_response.result.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Update the coach
    let update_request = create_test_request(
        "update_coach",
        json!({
            "coach_id": coach_id,
            "title": "Updated Title",
            "description": "Added description",
            "system_prompt": "Updated coaching prompt with more details.",
            "category": "training",
            "tags": ["updated", "training"]
        }),
        user_id,
        &tenant_id,
    );

    let response = executor.execute_tool(update_request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert_eq!(result["title"].as_str().unwrap(), "Updated Title");
    assert_eq!(result["description"].as_str().unwrap(), "Added description");
    assert!(result["system_prompt"]
        .as_str()
        .unwrap()
        .contains("Updated"));
    assert_eq!(result["category"].as_str().unwrap(), "training");

    Ok(())
}

#[tokio::test]
async fn test_update_coach_partial() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    // Create a coach first
    let create_request = create_test_request(
        "create_coach",
        json!({
            "title": "Partial Update Test",
            "system_prompt": "Original prompt.",
            "category": "nutrition"
        }),
        user_id,
        &tenant_id,
    );

    let create_response = executor.execute_tool(create_request).await?;
    let coach_id = create_response.result.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Partial update - only title
    let update_request = create_test_request(
        "update_coach",
        json!({
            "coach_id": coach_id,
            "title": "Only Title Updated"
        }),
        user_id,
        &tenant_id,
    );

    let response = executor.execute_tool(update_request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert_eq!(result["title"].as_str().unwrap(), "Only Title Updated");
    // Category should remain unchanged
    assert_eq!(result["category"].as_str().unwrap(), "nutrition");

    Ok(())
}

#[tokio::test]
async fn test_update_coach_not_found() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    let request = create_test_request(
        "update_coach",
        json!({
            "coach_id": Uuid::new_v4().to_string(),
            "title": "New Title"
        }),
        user_id,
        &tenant_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(!response.success, "Should fail for nonexistent coach");
    assert!(response.error.unwrap().contains("not found"));

    Ok(())
}

// ============================================================================
// delete_coach Tests
// ============================================================================

#[tokio::test]
async fn test_delete_coach_success() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    // Create a coach first
    let create_request = create_test_request(
        "create_coach",
        json!({
            "title": "Coach to Delete",
            "system_prompt": "This coach will be deleted."
        }),
        user_id,
        &tenant_id,
    );

    let create_response = executor.execute_tool(create_request).await?;
    let coach_id = create_response.result.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Delete the coach
    let delete_request = create_test_request(
        "delete_coach",
        json!({
            "coach_id": coach_id.clone()
        }),
        user_id,
        &tenant_id,
    );

    let response = executor.execute_tool(delete_request).await?;

    assert!(response.success);
    let result = response.result.unwrap();
    assert!(result["deleted"].as_bool().unwrap());
    assert_eq!(result["coach_id"].as_str().unwrap(), coach_id);

    // Verify it's gone
    let get_request = create_test_request(
        "get_coach",
        json!({
            "coach_id": coach_id
        }),
        user_id,
        &tenant_id,
    );

    let get_response = executor.execute_tool(get_request).await?;
    assert!(!get_response.success, "Coach should be deleted");

    Ok(())
}

#[tokio::test]
async fn test_delete_coach_not_found() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    let request = create_test_request(
        "delete_coach",
        json!({
            "coach_id": Uuid::new_v4().to_string()
        }),
        user_id,
        &tenant_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(!response.success, "Should fail for nonexistent coach");
    assert!(response.error.unwrap().contains("not found"));

    Ok(())
}

#[tokio::test]
async fn test_delete_coach_missing_id() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    let request = create_test_request("delete_coach", json!({}), user_id, &tenant_id);

    let result = executor.execute_tool(request).await;

    assert!(result.is_err(), "Should fail without coach_id");

    Ok(())
}

// ============================================================================
// toggle_coach_favorite Tests
// ============================================================================

#[tokio::test]
async fn test_toggle_coach_favorite_on() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    // Create a coach
    let create_request = create_test_request(
        "create_coach",
        json!({
            "title": "Favorite Toggle Test",
            "system_prompt": "Test prompt."
        }),
        user_id,
        &tenant_id,
    );

    let create_response = executor.execute_tool(create_request).await?;
    let coach_id = create_response.result.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Toggle favorite on
    let toggle_request = create_test_request(
        "toggle_coach_favorite",
        json!({
            "coach_id": coach_id.clone()
        }),
        user_id,
        &tenant_id,
    );

    let response = executor.execute_tool(toggle_request).await?;

    assert!(response.success);
    let result = response.result.unwrap();
    assert!(result["is_favorite"].as_bool().unwrap());
    assert_eq!(result["coach_id"].as_str().unwrap(), coach_id);

    Ok(())
}

#[tokio::test]
async fn test_toggle_coach_favorite_off() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    // Create a coach and toggle favorite on
    let create_request = create_test_request(
        "create_coach",
        json!({
            "title": "Double Toggle Test",
            "system_prompt": "Test prompt."
        }),
        user_id,
        &tenant_id,
    );

    let create_response = executor.execute_tool(create_request).await?;
    let coach_id = create_response.result.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // First toggle - on
    let toggle_request1 = create_test_request(
        "toggle_coach_favorite",
        json!({
            "coach_id": coach_id.clone()
        }),
        user_id,
        &tenant_id,
    );
    let response1 = executor.execute_tool(toggle_request1).await?;
    assert!(response1.result.unwrap()["is_favorite"].as_bool().unwrap());

    // Second toggle - off
    let toggle_request2 = create_test_request(
        "toggle_coach_favorite",
        json!({
            "coach_id": coach_id.clone()
        }),
        user_id,
        &tenant_id,
    );
    let response2 = executor.execute_tool(toggle_request2).await?;
    assert!(!response2.result.unwrap()["is_favorite"].as_bool().unwrap());

    Ok(())
}

#[tokio::test]
async fn test_toggle_coach_favorite_not_found() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    let request = create_test_request(
        "toggle_coach_favorite",
        json!({
            "coach_id": Uuid::new_v4().to_string()
        }),
        user_id,
        &tenant_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(!response.success, "Should fail for nonexistent coach");
    assert!(response.error.unwrap().contains("not found"));

    Ok(())
}

// ============================================================================
// search_coaches Tests
// ============================================================================

#[tokio::test]
async fn test_search_coaches_by_title() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    // Create coaches with different titles
    for title in ["Marathon Runner", "Sprint Coach", "Recovery Expert"] {
        let request = create_test_request(
            "create_coach",
            json!({
                "title": title,
                "system_prompt": format!("Specialist in {}.", title.to_lowercase())
            }),
            user_id,
            &tenant_id,
        );
        executor.execute_tool(request).await?;
    }

    // Search for "marathon"
    let request = create_test_request(
        "search_coaches",
        json!({
            "query": "marathon"
        }),
        user_id,
        &tenant_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert_eq!(result["query"].as_str().unwrap(), "marathon");
    assert_eq!(result["count"].as_u64().unwrap(), 1);

    let results = result["results"].as_array().unwrap();
    assert!(results[0]["title"].as_str().unwrap().contains("Marathon"));

    Ok(())
}

#[tokio::test]
async fn test_search_coaches_by_tag() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    // Create coaches with tags
    let request1 = create_test_request(
        "create_coach",
        json!({
            "title": "HIIT Coach",
            "system_prompt": "High intensity training.",
            "tags": ["hiit", "cardio", "intense"]
        }),
        user_id,
        &tenant_id,
    );
    executor.execute_tool(request1).await?;

    let request2 = create_test_request(
        "create_coach",
        json!({
            "title": "Yoga Coach",
            "system_prompt": "Flexibility and mindfulness.",
            "tags": ["yoga", "flexibility", "calm"]
        }),
        user_id,
        &tenant_id,
    );
    executor.execute_tool(request2).await?;

    // Search for "hiit" tag
    let request = create_test_request(
        "search_coaches",
        json!({
            "query": "hiit"
        }),
        user_id,
        &tenant_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert_eq!(result["count"].as_u64().unwrap(), 1);

    Ok(())
}

#[tokio::test]
async fn test_search_coaches_no_results() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    let request = create_test_request(
        "search_coaches",
        json!({
            "query": "nonexistent_coach_xyz"
        }),
        user_id,
        &tenant_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert_eq!(result["count"].as_u64().unwrap(), 0);
    assert!(result["results"].as_array().unwrap().is_empty());

    Ok(())
}

#[tokio::test]
async fn test_search_coaches_missing_query() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    let request = create_test_request("search_coaches", json!({}), user_id, &tenant_id);

    let result = executor.execute_tool(request).await;

    assert!(result.is_err(), "Should fail without query");

    Ok(())
}

#[tokio::test]
async fn test_search_coaches_with_limit() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    // Create 10 coaches with "test" in the title
    for i in 0..10 {
        let request = create_test_request(
            "create_coach",
            json!({
                "title": format!("Test Coach {}", i),
                "system_prompt": "A test coach."
            }),
            user_id,
            &tenant_id,
        );
        executor.execute_tool(request).await?;
    }

    // Search with limit
    let request = create_test_request(
        "search_coaches",
        json!({
            "query": "test",
            "limit": 3
        }),
        user_id,
        &tenant_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert_eq!(result["count"].as_u64().unwrap(), 3);

    Ok(())
}

// ============================================================================
// activate_coach / deactivate_coach / get_active_coach Tests
// ============================================================================

#[tokio::test]
async fn test_activate_coach_success() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    // Create a coach
    let create_request = create_test_request(
        "create_coach",
        json!({
            "title": "Coach to Activate",
            "system_prompt": "This coach will be activated."
        }),
        user_id,
        &tenant_id,
    );

    let create_response = executor.execute_tool(create_request).await?;
    let coach_id = create_response.result.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Activate the coach
    let activate_request = create_test_request(
        "activate_coach",
        json!({
            "coach_id": coach_id.clone()
        }),
        user_id,
        &tenant_id,
    );

    let response = executor.execute_tool(activate_request).await?;

    assert!(response.success);
    let result = response.result.unwrap();
    assert_eq!(result["id"].as_str().unwrap(), coach_id);
    assert!(result["is_active"].as_bool().unwrap());
    assert!(result["system_prompt"].is_string()); // Activation returns full details

    Ok(())
}

#[tokio::test]
async fn test_activate_coach_not_found() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    let request = create_test_request(
        "activate_coach",
        json!({
            "coach_id": Uuid::new_v4().to_string()
        }),
        user_id,
        &tenant_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(!response.success, "Should fail for nonexistent coach");
    assert!(response.error.unwrap().contains("not found"));

    Ok(())
}

#[tokio::test]
async fn test_deactivate_coach_success() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    // Create and activate a coach
    let create_request = create_test_request(
        "create_coach",
        json!({
            "title": "Coach to Deactivate",
            "system_prompt": "This coach will be deactivated."
        }),
        user_id,
        &tenant_id,
    );

    let create_response = executor.execute_tool(create_request).await?;
    let coach_id = create_response.result.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Activate first
    let activate_request = create_test_request(
        "activate_coach",
        json!({
            "coach_id": coach_id
        }),
        user_id,
        &tenant_id,
    );
    executor.execute_tool(activate_request).await?;

    // Now deactivate
    let deactivate_request =
        create_test_request("deactivate_coach", json!({}), user_id, &tenant_id);

    let response = executor.execute_tool(deactivate_request).await?;

    assert!(response.success);
    let result = response.result.unwrap();
    assert!(result["deactivated"].as_bool().unwrap());

    Ok(())
}

#[tokio::test]
async fn test_deactivate_coach_when_none_active() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    // Deactivate when nothing is active
    let deactivate_request =
        create_test_request("deactivate_coach", json!({}), user_id, &tenant_id);

    let response = executor.execute_tool(deactivate_request).await?;

    assert!(response.success);
    let result = response.result.unwrap();
    assert!(!result["deactivated"].as_bool().unwrap());

    Ok(())
}

#[tokio::test]
async fn test_get_active_coach_when_active() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    // Create and activate a coach
    let create_request = create_test_request(
        "create_coach",
        json!({
            "title": "Active Coach Test",
            "description": "Testing get_active_coach",
            "system_prompt": "This is the active coach prompt."
        }),
        user_id,
        &tenant_id,
    );

    let create_response = executor.execute_tool(create_request).await?;
    let coach_id = create_response.result.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let activate_request = create_test_request(
        "activate_coach",
        json!({
            "coach_id": coach_id.clone()
        }),
        user_id,
        &tenant_id,
    );
    executor.execute_tool(activate_request).await?;

    // Get active coach
    let get_active_request =
        create_test_request("get_active_coach", json!({}), user_id, &tenant_id);

    let response = executor.execute_tool(get_active_request).await?;

    assert!(response.success);
    let result = response.result.unwrap();
    assert!(result["active"].as_bool().unwrap());

    let coach = &result["coach"];
    assert_eq!(coach["id"].as_str().unwrap(), coach_id);
    assert_eq!(coach["title"].as_str().unwrap(), "Active Coach Test");
    assert!(coach["system_prompt"]
        .as_str()
        .unwrap()
        .contains("active coach prompt"));

    Ok(())
}

#[tokio::test]
async fn test_get_active_coach_when_none_active() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    // Get active coach when none is active
    let get_active_request =
        create_test_request("get_active_coach", json!({}), user_id, &tenant_id);

    let response = executor.execute_tool(get_active_request).await?;

    assert!(response.success);
    let result = response.result.unwrap();
    assert!(!result["active"].as_bool().unwrap());
    assert!(result["coach"].is_null());

    Ok(())
}

#[tokio::test]
async fn test_activate_replaces_previous_active() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    // Create two coaches
    let create_request1 = create_test_request(
        "create_coach",
        json!({
            "title": "First Coach",
            "system_prompt": "First coach prompt."
        }),
        user_id,
        &tenant_id,
    );
    let create_response1 = executor.execute_tool(create_request1).await?;
    let coach1_id = create_response1.result.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let create_request2 = create_test_request(
        "create_coach",
        json!({
            "title": "Second Coach",
            "system_prompt": "Second coach prompt."
        }),
        user_id,
        &tenant_id,
    );
    let create_response2 = executor.execute_tool(create_request2).await?;
    let coach2_id = create_response2.result.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Activate first coach
    let activate_request1 = create_test_request(
        "activate_coach",
        json!({
            "coach_id": coach1_id
        }),
        user_id,
        &tenant_id,
    );
    executor.execute_tool(activate_request1).await?;

    // Verify first is active
    let get_active_request1 =
        create_test_request("get_active_coach", json!({}), user_id, &tenant_id);
    let response1 = executor.execute_tool(get_active_request1).await?;
    assert_eq!(
        response1.result.unwrap()["coach"]["title"]
            .as_str()
            .unwrap(),
        "First Coach"
    );

    // Activate second coach (should replace first)
    let activate_request2 = create_test_request(
        "activate_coach",
        json!({
            "coach_id": coach2_id
        }),
        user_id,
        &tenant_id,
    );
    executor.execute_tool(activate_request2).await?;

    // Verify second is now active
    let get_active_request2 =
        create_test_request("get_active_coach", json!({}), user_id, &tenant_id);
    let response2 = executor.execute_tool(get_active_request2).await?;
    assert_eq!(
        response2.result.unwrap()["coach"]["title"]
            .as_str()
            .unwrap(),
        "Second Coach"
    );

    Ok(())
}

// ============================================================================
// User Isolation Tests
// ============================================================================

#[tokio::test]
async fn test_coach_user_isolation() -> Result<()> {
    let executor = create_coach_test_executor().await?;

    // Create two users with their own tenants
    let (user1_id, tenant1_id) = create_test_user_for_coaches(&executor).await?;
    let (user2_id, tenant2_id) = create_test_user_for_coaches(&executor).await?;

    // User 1 creates a coach
    let create_request = create_test_request(
        "create_coach",
        json!({
            "title": "User 1 Secret Coach",
            "system_prompt": "User 1's private coaching prompt."
        }),
        user1_id,
        &tenant1_id,
    );

    let create_response = executor.execute_tool(create_request).await?;
    assert!(create_response.success);
    let coach_id = create_response.result.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // User 2 should not see User 1's coach in list
    let list_request = create_test_request("list_coaches", json!({}), user2_id, &tenant2_id);

    let response = executor.execute_tool(list_request).await?;
    let result = response.result.unwrap();

    assert_eq!(
        result["count"].as_u64().unwrap(),
        0,
        "User 2 should not see User 1's coaches"
    );

    // User 2 should not be able to get User 1's coach
    let get_request = create_test_request(
        "get_coach",
        json!({
            "coach_id": coach_id.clone()
        }),
        user2_id,
        &tenant2_id,
    );

    let get_response = executor.execute_tool(get_request).await?;
    assert!(
        !get_response.success,
        "User 2 should not access User 1's coach"
    );

    // User 2 should not be able to delete User 1's coach
    let delete_request = create_test_request(
        "delete_coach",
        json!({
            "coach_id": coach_id.clone()
        }),
        user2_id,
        &tenant2_id,
    );

    let delete_response = executor.execute_tool(delete_request).await?;
    assert!(
        !delete_response.success,
        "User 2 should not delete User 1's coach"
    );

    // User 2 should not be able to activate User 1's coach
    let activate_request = create_test_request(
        "activate_coach",
        json!({
            "coach_id": coach_id
        }),
        user2_id,
        &tenant2_id,
    );

    let activate_response = executor.execute_tool(activate_request).await?;
    assert!(
        !activate_response.success,
        "User 2 should not activate User 1's coach"
    );

    Ok(())
}

// ============================================================================
// Token Count Tests
// ============================================================================

#[tokio::test]
async fn test_coach_token_count_calculated() -> Result<()> {
    let executor = create_coach_test_executor().await?;
    let (user_id, tenant_id) = create_test_user_for_coaches(&executor).await?;

    // Create a coach with a long system prompt
    let long_prompt = "You are an experienced marathon coach with over 25 years of experience \
        training athletes of all levels. Your approach combines scientific training principles \
        with practical race-day wisdom. You focus on periodization, recovery, nutrition timing, \
        and mental preparation. You always consider the athlete's current fitness level, \
        goals, and available training time when creating personalized plans.";

    let request = create_test_request(
        "create_coach",
        json!({
            "title": "Token Count Test Coach",
            "description": "Testing token count calculation",
            "system_prompt": long_prompt
        }),
        user_id,
        &tenant_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    let token_count = result["token_count"].as_u64().unwrap();
    assert!(
        token_count > 50,
        "Token count should be > 50 for long prompt: {token_count}"
    );
    assert!(
        token_count < 500,
        "Token count should be reasonable: {token_count}"
    );

    Ok(())
}
