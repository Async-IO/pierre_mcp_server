// ABOUTME: Integration tests for recipe MCP tool handlers ("Combat des Chefs" architecture)
// ABOUTME: Tests get_recipe_constraints, validate_recipe, save_recipe, list_recipes, get_recipe, delete_recipe, search_recipes
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Recipe Tool Handler Integration Tests
//!
//! Tests the 7 recipe MCP tools via the `UniversalToolExecutor`:
//! - `get_recipe_constraints`: Get macro targets for LLM recipe generation
//! - `validate_recipe`: Validate recipe nutrition via USDA
//! - `save_recipe`: Save recipe to user's collection
//! - `list_recipes`: List user's saved recipes
//! - `get_recipe`: Get specific recipe by ID
//! - `delete_recipe`: Delete recipe from collection
//! - `search_recipes`: Search recipes by name/tags/description

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use anyhow::Result;
use pierre_mcp_server::{
    database_plugins::DatabaseProvider,
    models::User,
    protocols::universal::{UniversalRequest, UniversalResponse, UniversalToolExecutor},
};
use serde_json::json;
use std::env;
use std::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;

/// Timeout for USDA API calls (external government API can be slow)
const USDA_API_TIMEOUT: Duration = Duration::from_secs(30);

mod common;

// ============================================================================
// Test Setup
// ============================================================================

/// Create test executor for recipe tool tests
async fn create_recipe_test_executor() -> Result<UniversalToolExecutor> {
    common::init_server_config();
    common::init_test_http_clients();

    let resources = common::create_test_server_resources().await?;
    Ok(UniversalToolExecutor::new(resources))
}

/// Create a test user in the database
async fn create_test_user_for_recipes(executor: &UniversalToolExecutor) -> Result<Uuid> {
    let user = User::new(
        format!("recipe_test_{}@example.com", Uuid::new_v4()),
        "password_hash".to_owned(),
        Some("Recipe Test User".to_owned()),
    );
    let user_id = user.id;
    executor.resources.database.create_user(&user).await?;
    Ok(user_id)
}

/// Create a test request with user ID
fn create_test_request(
    tool_name: &str,
    parameters: serde_json::Value,
    user_id: Uuid,
) -> UniversalRequest {
    UniversalRequest {
        tool_name: tool_name.to_owned(),
        parameters,
        user_id: user_id.to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    }
}

/// Check if USDA API key is configured
fn usda_api_key_available() -> bool {
    env::var("USDA_API_KEY").is_ok()
}

/// Execute a USDA API tool call with timeout and graceful error handling.
///
/// Returns:
/// - `Ok(Some(response))` - API call succeeded
/// - `Ok(None)` - API call failed due to infrastructure issues (timeout, network, rate limit)
///   The test should skip gracefully in this case.
/// - `Err(e)` - Unexpected error that should fail the test
async fn execute_usda_api_call_with_timeout(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
    test_name: &str,
) -> Result<Option<UniversalResponse>> {
    match timeout(USDA_API_TIMEOUT, executor.execute_tool(request)).await {
        Ok(Ok(response)) => {
            // API call completed - check if it failed due to infrastructure issues
            if !response.success {
                let error_msg = response
                    .error
                    .as_ref()
                    .map_or("Unknown error", String::as_str);
                if is_infrastructure_error(error_msg) {
                    println!("Skipping {test_name} - external API issue: {error_msg}");
                    return Ok(None);
                }
            }
            Ok(Some(response))
        }
        Ok(Err(err)) => {
            // Tool execution returned an error - check if it's network-related
            let error_string = err.to_string();
            if is_infrastructure_error(&error_string) {
                println!("Skipping {test_name} - execution error (likely network): {error_string}");
                return Ok(None);
            }
            Err(err.into())
        }
        Err(_elapsed) => {
            // Timeout occurred
            println!(
                "Skipping {test_name} - USDA API timeout after {} seconds",
                USDA_API_TIMEOUT.as_secs()
            );
            Ok(None)
        }
    }
}

/// Check if an error message indicates infrastructure/network issues rather than logic errors.
fn is_infrastructure_error(error_msg: &str) -> bool {
    let error_lower = error_msg.to_lowercase();
    error_lower.contains("rate limit")
        || error_lower.contains("timeout")
        || error_lower.contains("503")
        || error_lower.contains("500")
        || error_lower.contains("502")
        || error_lower.contains("504")
        || error_lower.contains("connection")
        || error_lower.contains("network")
        || error_lower.contains("dns")
        || error_lower.contains("temporarily unavailable")
        || error_lower.contains("service unavailable")
        || error_lower.contains("timed out")
}

// ============================================================================
// Tool Registration Tests
// ============================================================================

#[tokio::test]
async fn test_recipe_tools_registered() -> Result<()> {
    let executor = create_recipe_test_executor().await?;

    let tool_names: Vec<String> = executor
        .list_tools()
        .iter()
        .map(|tool| tool.name().to_owned())
        .collect();

    let expected_tools = vec![
        "get_recipe_constraints",
        "validate_recipe",
        "save_recipe",
        "list_recipes",
        "get_recipe",
        "delete_recipe",
        "search_recipes",
    ];

    for expected_tool in expected_tools {
        assert!(
            tool_names.contains(&expected_tool.to_owned()),
            "Missing recipe tool: {expected_tool}"
        );
    }

    Ok(())
}

// ============================================================================
// get_recipe_constraints Tests
// ============================================================================

#[tokio::test]
async fn test_get_recipe_constraints_default() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    let request = create_test_request(
        "get_recipe_constraints",
        json!({
            "meal_timing": "general"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success, "Tool should succeed");
    let result = response.result.unwrap();

    // Verify calorie and macro targets are present
    assert!(
        result["calories"].as_f64().is_some(),
        "Should have calories"
    );
    assert!(!result["protein_g"].is_null(), "Should have protein_g");
    assert!(!result["carbs_g"].is_null(), "Should have carbs_g");
    assert!(!result["fat_g"].is_null(), "Should have fat_g");

    // Verify meal timing info
    assert_eq!(result["meal_timing"].as_str().unwrap(), "general");
    assert!(result["meal_timing_description"].is_string());

    // Verify prompt hint for LLM
    assert!(result["prompt_hint"].is_string(), "Should have prompt hint");
    assert!(
        result["prompt_hint"].as_str().unwrap().len() > 50,
        "Prompt hint should be substantial"
    );

    // TDEE-based should be false when no TDEE provided
    assert!(!result["tdee_based"].as_bool().unwrap());

    Ok(())
}

#[tokio::test]
async fn test_get_recipe_constraints_pre_training() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    let request = create_test_request(
        "get_recipe_constraints",
        json!({
            "meal_timing": "pre_training",
            "calories": 500.0
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    // Pre-training should have high carbs (55%)
    let calories = result["calories"].as_f64().unwrap();
    assert!(
        (calories - 500.0).abs() < 1.0,
        "Should use explicit calories"
    );

    // Verify pre-training timing
    assert!(result["meal_timing"].as_str().unwrap().contains("pre"));

    Ok(())
}

#[tokio::test]
async fn test_get_recipe_constraints_post_training() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    let request = create_test_request(
        "get_recipe_constraints",
        json!({
            "meal_timing": "post_training",
            "calories": 600.0
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    // Post-training should have high protein (30%)
    // protein_g = 600 * 0.30 / 4 = 45g
    let protein = result["protein_g"].as_f64().unwrap();
    assert!(
        protein >= 40.0,
        "Post-training should have high protein: {protein}g"
    );

    Ok(())
}

#[tokio::test]
async fn test_get_recipe_constraints_rest_day() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    let request = create_test_request(
        "get_recipe_constraints",
        json!({
            "meal_timing": "rest_day"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    // Rest day should have lower carbs (35%)
    assert!(result["meal_timing"].as_str().unwrap().contains("rest"));

    Ok(())
}

#[tokio::test]
async fn test_get_recipe_constraints_with_tdee() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    let request = create_test_request(
        "get_recipe_constraints",
        json!({
            "meal_timing": "general",
            "tdee": 2500.0
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    // Should be TDEE-based
    assert!(result["tdee_based"].as_bool().unwrap());
    assert!(
        (result["tdee"].as_f64().unwrap() - 2500.0).abs() < 0.1,
        "TDEE should be 2500"
    );
    assert!(result["tdee_proportion"].as_f64().is_some());

    // Calories should be derived from TDEE proportion
    let calories = result["calories"].as_f64().unwrap();
    assert!(
        calories > 0.0 && calories < 2500.0,
        "Meal calories should be fraction of TDEE"
    );

    Ok(())
}

#[tokio::test]
async fn test_get_recipe_constraints_with_time_limits() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    let request = create_test_request(
        "get_recipe_constraints",
        json!({
            "meal_timing": "general",
            "max_prep_time_mins": 30,
            "max_cook_time_mins": 45
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert_eq!(result["max_prep_time_mins"].as_u64().unwrap(), 30);
    assert_eq!(result["max_cook_time_mins"].as_u64().unwrap(), 45);

    Ok(())
}

// ============================================================================
// validate_recipe Tests
// ============================================================================

#[tokio::test]
async fn test_validate_recipe_no_api_key() -> Result<()> {
    if usda_api_key_available() {
        println!("Skipping test_validate_recipe_no_api_key - API key configured");
        return Ok(());
    }

    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    let request = create_test_request(
        "validate_recipe",
        json!({
            "name": "Test Recipe",
            "servings": 4,
            "ingredients": [
                {"name": "chicken breast", "amount": 500.0, "unit": "grams"},
                {"name": "rice", "amount": 2.0, "unit": "cups"}
            ]
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(!response.success, "Should fail without API key");
    assert!(response.error.unwrap().contains("USDA API key"));

    Ok(())
}

#[tokio::test]
async fn test_validate_recipe_missing_servings() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    let request = create_test_request(
        "validate_recipe",
        json!({
            "name": "Test Recipe",
            "ingredients": [
                {"name": "chicken", "amount": 500.0, "unit": "grams"}
            ]
        }),
        user_id,
    );

    let result = executor.execute_tool(request).await;

    assert!(result.is_err(), "Should fail with missing servings");

    Ok(())
}

#[tokio::test]
async fn test_validate_recipe_missing_ingredients() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    let request = create_test_request(
        "validate_recipe",
        json!({
            "name": "Test Recipe",
            "servings": 4
        }),
        user_id,
    );

    let result = executor.execute_tool(request).await;

    assert!(result.is_err(), "Should fail with missing ingredients");

    Ok(())
}

// ============================================================================
// save_recipe Tests
// ============================================================================

#[tokio::test]
async fn test_save_recipe_success() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    let request = create_test_request(
        "save_recipe",
        json!({
            "name": "Test Chicken Stir Fry",
            "description": "A quick and healthy stir fry",
            "servings": 4,
            "prep_time_mins": 15,
            "cook_time_mins": 20,
            "instructions": [
                "Cut chicken into cubes",
                "Heat oil in wok",
                "Stir fry chicken until cooked",
                "Add vegetables",
                "Season and serve"
            ],
            "ingredients": [
                {"name": "chicken breast", "amount": 500.0, "unit": "grams"},
                {"name": "rice", "amount": 2.0, "unit": "cups"},
                {"name": "broccoli", "amount": 200.0, "unit": "grams"}
            ],
            "tags": ["quick", "healthy", "high-protein"],
            "meal_timing": "post_training"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(
        response.success,
        "Save should succeed: {:?}",
        response.error
    );
    let result = response.result.unwrap();

    // Verify recipe was saved with ID
    assert!(result["recipe_id"].is_string(), "Should return recipe_id");
    assert_eq!(result["name"].as_str().unwrap(), "Test Chicken Stir Fry");
    assert_eq!(result["servings"].as_u64().unwrap(), 4);
    assert!(result["meal_timing"].as_str().unwrap().contains("post"));
    assert!(result["created_at"].is_string());

    Ok(())
}

#[tokio::test]
async fn test_save_recipe_minimal() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    // Minimal required fields only
    let request = create_test_request(
        "save_recipe",
        json!({
            "name": "Simple Recipe",
            "servings": 2,
            "instructions": ["Mix ingredients", "Serve"],
            "ingredients": [
                {"name": "pasta", "amount": 200.0, "unit": "grams"}
            ]
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success, "Save should succeed with minimal fields");
    let result = response.result.unwrap();

    assert!(result["recipe_id"].is_string());
    assert_eq!(result["meal_timing"].as_str().unwrap(), "general");

    Ok(())
}

#[tokio::test]
async fn test_save_recipe_missing_name() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    let request = create_test_request(
        "save_recipe",
        json!({
            "servings": 2,
            "instructions": ["Cook"],
            "ingredients": [{"name": "rice", "amount": 100.0, "unit": "grams"}]
        }),
        user_id,
    );

    let result = executor.execute_tool(request).await;

    assert!(result.is_err(), "Should fail without name");

    Ok(())
}

// ============================================================================
// list_recipes Tests
// ============================================================================

#[tokio::test]
async fn test_list_recipes_empty() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    let request = create_test_request("list_recipes", json!({}), user_id);

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert!(result["recipes"].is_array());
    assert_eq!(result["count"].as_u64().unwrap(), 0);

    Ok(())
}

#[tokio::test]
async fn test_list_recipes_after_save() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    // Save a recipe first
    let save_request = create_test_request(
        "save_recipe",
        json!({
            "name": "Test Recipe for List",
            "servings": 2,
            "instructions": ["Step 1"],
            "ingredients": [{"name": "rice", "amount": 100.0, "unit": "grams"}],
            "meal_timing": "pre_training"
        }),
        user_id,
    );

    let save_response = executor.execute_tool(save_request).await?;
    assert!(save_response.success);

    // Now list recipes
    let list_request = create_test_request("list_recipes", json!({}), user_id);

    let response = executor.execute_tool(list_request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert_eq!(result["count"].as_u64().unwrap(), 1);
    let recipes = result["recipes"].as_array().unwrap();
    assert_eq!(recipes[0]["name"].as_str().unwrap(), "Test Recipe for List");

    Ok(())
}

#[tokio::test]
async fn test_list_recipes_with_meal_timing_filter() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    // Save recipes with different timings
    for (name, timing) in [
        ("Pre Training Recipe", "pre_training"),
        ("Post Training Recipe", "post_training"),
        ("General Recipe", "general"),
    ] {
        let request = create_test_request(
            "save_recipe",
            json!({
                "name": name,
                "servings": 2,
                "instructions": ["Step 1"],
                "ingredients": [{"name": "rice", "amount": 100.0, "unit": "grams"}],
                "meal_timing": timing
            }),
            user_id,
        );
        executor.execute_tool(request).await?;
    }

    // Filter by pre_training
    let request = create_test_request(
        "list_recipes",
        json!({
            "meal_timing": "pre_training"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert_eq!(result["count"].as_u64().unwrap(), 1);
    let recipes = result["recipes"].as_array().unwrap();
    assert!(recipes[0]["name"]
        .as_str()
        .unwrap()
        .contains("Pre Training"));

    Ok(())
}

#[tokio::test]
async fn test_list_recipes_with_pagination() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    // Save 5 recipes
    for i in 0..5 {
        let request = create_test_request(
            "save_recipe",
            json!({
                "name": format!("Recipe {}", i),
                "servings": 2,
                "instructions": ["Step 1"],
                "ingredients": [{"name": "rice", "amount": 100.0, "unit": "grams"}]
            }),
            user_id,
        );
        executor.execute_tool(request).await?;
    }

    // Get first 2 recipes
    let request = create_test_request(
        "list_recipes",
        json!({
            "limit": 2,
            "offset": 0
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert_eq!(result["count"].as_u64().unwrap(), 2);

    Ok(())
}

// ============================================================================
// get_recipe Tests
// ============================================================================

#[tokio::test]
async fn test_get_recipe_success() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    // Save a recipe first
    let save_request = create_test_request(
        "save_recipe",
        json!({
            "name": "Recipe to Get",
            "description": "A test description",
            "servings": 4,
            "prep_time_mins": 10,
            "cook_time_mins": 30,
            "instructions": ["Step 1", "Step 2", "Step 3"],
            "ingredients": [
                {"name": "chicken", "amount": 500.0, "unit": "grams"},
                {"name": "rice", "amount": 2.0, "unit": "cups"}
            ],
            "tags": ["dinner", "healthy"]
        }),
        user_id,
    );

    let save_response = executor.execute_tool(save_request).await?;
    assert!(save_response.success);
    let recipe_id = save_response.result.unwrap()["recipe_id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Get the recipe
    let get_request = create_test_request(
        "get_recipe",
        json!({
            "recipe_id": recipe_id
        }),
        user_id,
    );

    let response = executor.execute_tool(get_request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    // Verify all fields
    assert_eq!(result["name"].as_str().unwrap(), "Recipe to Get");
    assert_eq!(
        result["description"].as_str().unwrap(),
        "A test description"
    );
    assert_eq!(result["servings"].as_u64().unwrap(), 4);
    assert_eq!(result["prep_time_mins"].as_u64().unwrap(), 10);
    assert_eq!(result["cook_time_mins"].as_u64().unwrap(), 30);
    assert_eq!(result["total_time_mins"].as_u64().unwrap(), 40);

    // Verify ingredients array
    let ingredients = result["ingredients"].as_array().unwrap();
    assert_eq!(ingredients.len(), 2);
    assert_eq!(ingredients[0]["name"].as_str().unwrap(), "chicken");

    // Verify instructions array
    let instructions = result["instructions"].as_array().unwrap();
    assert_eq!(instructions.len(), 3);

    // Verify tags
    let tags = result["tags"].as_array().unwrap();
    assert!(tags.iter().any(|t| t.as_str().unwrap() == "dinner"));

    Ok(())
}

#[tokio::test]
async fn test_get_recipe_not_found() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    let request = create_test_request(
        "get_recipe",
        json!({
            "recipe_id": Uuid::new_v4().to_string()
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(!response.success, "Should fail for nonexistent recipe");
    assert!(response.error.unwrap().contains("not found"));

    Ok(())
}

#[tokio::test]
async fn test_get_recipe_missing_id() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    let request = create_test_request("get_recipe", json!({}), user_id);

    let result = executor.execute_tool(request).await;

    assert!(result.is_err(), "Should fail without recipe_id");

    Ok(())
}

// ============================================================================
// delete_recipe Tests
// ============================================================================

#[tokio::test]
async fn test_delete_recipe_success() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    // Save a recipe first
    let save_request = create_test_request(
        "save_recipe",
        json!({
            "name": "Recipe to Delete",
            "servings": 2,
            "instructions": ["Step 1"],
            "ingredients": [{"name": "rice", "amount": 100.0, "unit": "grams"}]
        }),
        user_id,
    );

    let save_response = executor.execute_tool(save_request).await?;
    let recipe_id = save_response.result.unwrap()["recipe_id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Delete the recipe
    let delete_request = create_test_request(
        "delete_recipe",
        json!({
            "recipe_id": recipe_id.clone()
        }),
        user_id,
    );

    let response = executor.execute_tool(delete_request).await?;

    assert!(response.success);
    let result = response.result.unwrap();
    assert!(result["deleted"].as_bool().unwrap());
    assert_eq!(result["recipe_id"].as_str().unwrap(), recipe_id);

    // Verify it's gone
    let get_request = create_test_request(
        "get_recipe",
        json!({
            "recipe_id": recipe_id
        }),
        user_id,
    );

    let get_response = executor.execute_tool(get_request).await?;
    assert!(!get_response.success, "Recipe should be deleted");

    Ok(())
}

#[tokio::test]
async fn test_delete_recipe_not_found() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    let request = create_test_request(
        "delete_recipe",
        json!({
            "recipe_id": Uuid::new_v4().to_string()
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(!response.success, "Should fail for nonexistent recipe");
    assert!(response.error.unwrap().contains("not found"));

    Ok(())
}

#[tokio::test]
async fn test_delete_recipe_missing_id() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    let request = create_test_request("delete_recipe", json!({}), user_id);

    let result = executor.execute_tool(request).await;

    assert!(result.is_err(), "Should fail without recipe_id");

    Ok(())
}

// ============================================================================
// search_recipes Tests
// ============================================================================

#[tokio::test]
async fn test_search_recipes_by_name() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    // Save recipes with different names
    for name in ["Chicken Stir Fry", "Beef Tacos", "Vegetable Soup"] {
        let request = create_test_request(
            "save_recipe",
            json!({
                "name": name,
                "servings": 2,
                "instructions": ["Step 1"],
                "ingredients": [{"name": "ingredient", "amount": 100.0, "unit": "grams"}]
            }),
            user_id,
        );
        executor.execute_tool(request).await?;
    }

    // Search for "chicken"
    let request = create_test_request(
        "search_recipes",
        json!({
            "query": "chicken"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert_eq!(result["query"].as_str().unwrap(), "chicken");
    assert_eq!(result["count"].as_u64().unwrap(), 1);

    let results = result["results"].as_array().unwrap();
    assert!(results[0]["name"].as_str().unwrap().contains("Chicken"));

    Ok(())
}

#[tokio::test]
async fn test_search_recipes_by_tag() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    // Save recipes with tags
    let request1 = create_test_request(
        "save_recipe",
        json!({
            "name": "Quick Breakfast",
            "servings": 1,
            "instructions": ["Make it"],
            "ingredients": [{"name": "eggs", "amount": 2.0, "unit": "pieces"}],
            "tags": ["quick", "breakfast", "high-protein"]
        }),
        user_id,
    );
    executor.execute_tool(request1).await?;

    let request2 = create_test_request(
        "save_recipe",
        json!({
            "name": "Slow Dinner",
            "servings": 4,
            "instructions": ["Cook slowly"],
            "ingredients": [{"name": "beef", "amount": 500.0, "unit": "grams"}],
            "tags": ["slow-cooked", "dinner"]
        }),
        user_id,
    );
    executor.execute_tool(request2).await?;

    // Search for "quick" tag
    let request = create_test_request(
        "search_recipes",
        json!({
            "query": "quick"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert_eq!(result["count"].as_u64().unwrap(), 1);

    Ok(())
}

#[tokio::test]
async fn test_search_recipes_no_results() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    let request = create_test_request(
        "search_recipes",
        json!({
            "query": "nonexistent_recipe_xyz"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert_eq!(result["count"].as_u64().unwrap(), 0);
    assert!(result["results"].as_array().unwrap().is_empty());

    Ok(())
}

#[tokio::test]
async fn test_search_recipes_missing_query() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    let request = create_test_request("search_recipes", json!({}), user_id);

    let result = executor.execute_tool(request).await;

    assert!(result.is_err(), "Should fail without query");

    Ok(())
}

#[tokio::test]
async fn test_search_recipes_with_limit() -> Result<()> {
    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    // Save 10 recipes with "test" in the name
    for i in 0..10 {
        let request = create_test_request(
            "save_recipe",
            json!({
                "name": format!("Test Recipe {}", i),
                "servings": 2,
                "instructions": ["Step 1"],
                "ingredients": [{"name": "ingredient", "amount": 100.0, "unit": "grams"}]
            }),
            user_id,
        );
        executor.execute_tool(request).await?;
    }

    // Search with limit
    let request = create_test_request(
        "search_recipes",
        json!({
            "query": "test",
            "limit": 3
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert_eq!(result["count"].as_u64().unwrap(), 3);

    Ok(())
}

// ============================================================================
// User Isolation Tests
// ============================================================================

#[tokio::test]
async fn test_recipe_user_isolation() -> Result<()> {
    let executor = create_recipe_test_executor().await?;

    // Create two users
    let user1_id = create_test_user_for_recipes(&executor).await?;
    let user2_id = create_test_user_for_recipes(&executor).await?;

    // User 1 saves a recipe
    let save_request = create_test_request(
        "save_recipe",
        json!({
            "name": "User 1 Secret Recipe",
            "servings": 2,
            "instructions": ["Step 1"],
            "ingredients": [{"name": "secret", "amount": 100.0, "unit": "grams"}]
        }),
        user1_id,
    );

    let save_response = executor.execute_tool(save_request).await?;
    assert!(save_response.success);
    let recipe_id = save_response.result.unwrap()["recipe_id"]
        .as_str()
        .unwrap()
        .to_owned();

    // User 2 should not see User 1's recipe
    let list_request = create_test_request("list_recipes", json!({}), user2_id);

    let response = executor.execute_tool(list_request).await?;
    let result = response.result.unwrap();

    assert_eq!(
        result["count"].as_u64().unwrap(),
        0,
        "User 2 should not see User 1's recipes"
    );

    // User 2 should not be able to get User 1's recipe
    let get_request = create_test_request(
        "get_recipe",
        json!({
            "recipe_id": recipe_id
        }),
        user2_id,
    );

    let get_response = executor.execute_tool(get_request).await?;
    assert!(
        !get_response.success,
        "User 2 should not access User 1's recipe"
    );

    Ok(())
}

// ============================================================================
// validate_recipe with USDA API (conditional)
// ============================================================================

#[tokio::test]
async fn test_validate_recipe_with_api_key() -> Result<()> {
    if !usda_api_key_available() {
        println!("Skipping test_validate_recipe_with_api_key - no USDA_API_KEY");
        return Ok(());
    }

    let executor = create_recipe_test_executor().await?;
    let user_id = create_test_user_for_recipes(&executor).await?;

    let request = create_test_request(
        "validate_recipe",
        json!({
            "name": "Chicken and Rice",
            "servings": 4,
            "ingredients": [
                {"name": "chicken breast", "amount": 500.0, "unit": "grams"},
                {"name": "white rice", "amount": 300.0, "unit": "grams"}
            ]
        }),
        user_id,
    );

    let response =
        execute_usda_api_call_with_timeout(&executor, request, "test_validate_recipe_with_api_key")
            .await?;

    // Skip test if API had infrastructure issues
    let Some(response) = response else {
        return Ok(());
    };

    assert!(response.success, "Should succeed with API key");
    let result = response.result.unwrap();

    assert!(result["validated"].as_bool().unwrap());
    assert!(result["nutrition_per_serving"].is_object());
    assert!(
        result["nutrition_per_serving"]["calories"]
            .as_f64()
            .unwrap()
            > 0.0
    );
    assert!(result["validation_completeness"].as_f64().is_some());
    assert!(result["usda_matched_count"].as_u64().is_some());

    Ok(())
}
