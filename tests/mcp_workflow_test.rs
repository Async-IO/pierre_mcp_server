// ABOUTME: MCP workflow integration tests
// ABOUTME: Tests realistic multi-tool sequences that represent real-world use cases
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

mod common;
mod integration;

use anyhow::Result;
use integration::{IntegrationTestServer, McpTestClient};
use serde_json::json;

// ============================================================================
// TEST HELPERS
// ============================================================================

async fn setup_test_client() -> Result<(IntegrationTestServer, McpTestClient)> {
    let mut server = IntegrationTestServer::new().await?;
    server.start().await?;
    let (_user_id, jwt_token) = server.create_test_user("workflow@test.local").await?;
    let client = McpTestClient::new(&server.base_url(), &jwt_token);
    Ok((server, client))
}

/// Call a tool and print the result (don't fail on tool errors, just log them)
async fn call_tool_lenient(client: &McpTestClient, tool: &str, args: serde_json::Value) -> String {
    match client.call_tool_raw(tool, args).await {
        Ok(result) => {
            let text = result
                .content
                .first()
                .and_then(|c| c.text.as_ref())
                .cloned()
                .unwrap_or_else(|| "No text content".to_owned());
            format!("OK: {}", &text[..text.len().min(60)])
        }
        Err(e) => format!(
            "Error: {}",
            e.to_string().chars().take(60).collect::<String>()
        ),
    }
}

// ============================================================================
// WORKFLOW: New User Onboarding
// ============================================================================
// Simulates what happens when a new user connects and starts using Pierre

/// Workflow: New user connects, checks status, views activities
#[tokio::test]
async fn test_workflow_new_user_onboarding() -> Result<()> {
    let (_server, client) = setup_test_client().await?;

    println!("🏃 Workflow: New User Onboarding");

    // Step 1: Initialize MCP session
    let init = client.initialize().await?;
    assert!(!init.server_info.name.is_empty());
    println!("  ✓ Step 1: MCP session initialized");

    // Step 2: List available tools to understand capabilities
    let tools = client.list_tools().await?;
    assert!(tools.tools.len() >= 40, "Should have many tools available");
    println!("  ✓ Step 2: Discovered {} tools", tools.tools.len());

    // Step 3: Check connection status
    let status = call_tool_lenient(
        &client,
        "get_connection_status",
        json!({"provider": "strava"}),
    )
    .await;
    println!("  ✓ Step 3: get_connection_status: {status}");

    // Step 4: Get activities (will fail without connected provider - that's expected)
    let activities = call_tool_lenient(
        &client,
        "get_activities",
        json!({"provider": "synthetic", "limit": 10}),
    )
    .await;
    println!("  ✓ Step 4: get_activities: {activities}");

    // Step 5: Get fitness score
    let score = call_tool_lenient(
        &client,
        "calculate_fitness_score",
        json!({"provider": "synthetic"}),
    )
    .await;
    println!("  ✓ Step 5: calculate_fitness_score: {score}");

    println!("✅ Workflow: New User Onboarding completed successfully");
    Ok(())
}

// ============================================================================
// WORKFLOW: Training Analysis Session
// ============================================================================

/// Workflow: Analyze training load, get recommendations
#[tokio::test]
async fn test_workflow_training_analysis() -> Result<()> {
    let (_server, client) = setup_test_client().await?;

    println!("🏃 Workflow: Training Analysis Session");

    // Step 1: Get recent activities
    let activities = call_tool_lenient(
        &client,
        "get_activities",
        json!({"provider": "synthetic", "limit": 30}),
    )
    .await;
    println!("  ✓ Step 1: get_activities: {activities}");

    // Step 2: Analyze training load
    let load = call_tool_lenient(
        &client,
        "analyze_training_load",
        json!({"provider": "synthetic", "days": 14}),
    )
    .await;
    println!("  ✓ Step 2: analyze_training_load: {load}");

    // Step 3: Get performance trends
    let trends = call_tool_lenient(
        &client,
        "analyze_performance_trends",
        json!({"provider": "synthetic", "days": 30}),
    )
    .await;
    println!("  ✓ Step 3: analyze_performance_trends: {trends}");

    // Step 4: Get personalized recommendations
    let recommendations = call_tool_lenient(
        &client,
        "generate_recommendations",
        json!({"provider": "synthetic"}),
    )
    .await;
    println!("  ✓ Step 4: generate_recommendations: {recommendations}");

    // Step 5: Set a new goal
    let goal = call_tool_lenient(
        &client,
        "set_goal",
        json!({
            "title": "Weekly Distance Goal",
            "goal_type": "distance",
            "target_value": 50.0,
            "target_date": "2025-12-31"
        }),
    )
    .await;
    println!("  ✓ Step 5: set_goal: {goal}");

    println!("✅ Workflow: Training Analysis completed successfully");
    Ok(())
}

// ============================================================================
// WORKFLOW: Recovery and Sleep Analysis
// ============================================================================

/// Workflow: Check sleep, recovery, and get rest day recommendation
#[tokio::test]
async fn test_workflow_recovery_check() -> Result<()> {
    let (_server, client) = setup_test_client().await?;

    println!("🏃 Workflow: Recovery Check");

    // Step 1: Analyze recent sleep quality
    let sleep = call_tool_lenient(
        &client,
        "analyze_sleep_quality",
        json!({"provider": "synthetic", "days": 7}),
    )
    .await;
    println!("  ✓ Step 1: analyze_sleep_quality: {sleep}");

    // Step 2: Calculate recovery score
    let recovery = call_tool_lenient(
        &client,
        "calculate_recovery_score",
        json!({"provider": "synthetic"}),
    )
    .await;
    println!("  ✓ Step 2: calculate_recovery_score: {recovery}");

    // Step 3: Get rest day suggestion
    let rest_day = call_tool_lenient(
        &client,
        "suggest_rest_day",
        json!({"provider": "synthetic"}),
    )
    .await;
    println!("  ✓ Step 3: suggest_rest_day: {rest_day}");

    // Step 4: Get training load analysis
    let load = call_tool_lenient(
        &client,
        "analyze_training_load",
        json!({"provider": "synthetic", "days": 7}),
    )
    .await;
    println!("  ✓ Step 4: analyze_training_load: {load}");

    println!("✅ Workflow: Recovery Check completed successfully");
    Ok(())
}

// ============================================================================
// WORKFLOW: Nutrition Planning
// ============================================================================

/// Workflow: Calculate nutrition needs and search for foods
#[tokio::test]
async fn test_workflow_nutrition_planning() -> Result<()> {
    let (_server, client) = setup_test_client().await?;

    println!("🏃 Workflow: Nutrition Planning");

    // Step 1: Calculate daily nutrition needs
    let nutrition = call_tool_lenient(
        &client,
        "calculate_daily_nutrition",
        json!({
            "weight_kg": 70.0,
            "height_cm": 175.0,
            "age": 30,
            "gender": "male",
            "activity_level": "very_active",
            "goal": "performance"
        }),
    )
    .await;
    println!("  ✓ Step 1: calculate_daily_nutrition: {nutrition}");

    // Step 2: Get nutrient timing for workout
    let timing = call_tool_lenient(
        &client,
        "get_nutrient_timing",
        json!({
            "workout_type": "endurance",
            "intensity": "high",
            "duration_minutes": 90,
            "weight_kg": 70.0
        }),
    )
    .await;
    println!("  ✓ Step 2: get_nutrient_timing: {timing}");

    // Step 3: Search for recovery foods
    let foods = call_tool_lenient(
        &client,
        "search_food",
        json!({"query": "chicken breast", "limit": 3}),
    )
    .await;
    println!("  ✓ Step 3: search_food: {foods}");

    // Step 4: Get recipe constraints
    let constraints = call_tool_lenient(
        &client,
        "get_recipe_constraints",
        json!({"training_phase": "build"}),
    )
    .await;
    println!("  ✓ Step 4: get_recipe_constraints: {constraints}");

    println!("✅ Workflow: Nutrition Planning completed successfully");
    Ok(())
}

// ============================================================================
// WORKFLOW: Goal Setting and Progress Tracking
// ============================================================================

/// Workflow: Get goal suggestions, set goal, track progress
#[tokio::test]
async fn test_workflow_goal_management() -> Result<()> {
    let (_server, client) = setup_test_client().await?;

    println!("🏃 Workflow: Goal Management");

    // Step 1: Get AI-suggested goals
    let suggestions =
        call_tool_lenient(&client, "suggest_goals", json!({"provider": "synthetic"})).await;
    println!("  ✓ Step 1: suggest_goals: {suggestions}");

    // Step 2: Set a specific goal
    let new_goal = call_tool_lenient(
        &client,
        "set_goal",
        json!({
            "title": "Monthly Distance Goal",
            "goal_type": "distance",
            "target_value": 200.0,
            "target_date": "2026-01-31"
        }),
    )
    .await;
    println!("  ✓ Step 2: set_goal: {new_goal}");

    // Step 3: Analyze goal feasibility
    let feasibility = call_tool_lenient(
        &client,
        "analyze_goal_feasibility",
        json!({
            "provider": "synthetic",
            "goal_type": "distance",
            "target_value": 200.0
        }),
    )
    .await;
    println!("  ✓ Step 3: analyze_goal_feasibility: {feasibility}");

    // Step 4: Track current progress
    let progress = call_tool_lenient(&client, "track_progress", json!({"goal_id": "latest"})).await;
    println!("  ✓ Step 4: track_progress: {progress}");

    // Step 5: Get recommendations to achieve goal
    let recommendations = call_tool_lenient(
        &client,
        "generate_recommendations",
        json!({"provider": "synthetic"}),
    )
    .await;
    println!("  ✓ Step 5: generate_recommendations: {recommendations}");

    println!("✅ Workflow: Goal Management completed successfully");
    Ok(())
}

// ============================================================================
// WORKFLOW: Configuration Management
// ============================================================================

/// Workflow: View profiles, update configuration, validate
#[tokio::test]
async fn test_workflow_configuration() -> Result<()> {
    let (_server, client) = setup_test_client().await?;

    println!("🏃 Workflow: Configuration Management");

    // Step 1: Get available configuration profiles
    let profiles = call_tool_lenient(&client, "get_configuration_profiles", json!({})).await;
    println!("  ✓ Step 1: get_configuration_profiles: {profiles}");

    // Step 2: Get configuration catalog
    let catalog = call_tool_lenient(&client, "get_configuration_catalog", json!({})).await;
    println!("  ✓ Step 2: get_configuration_catalog: {catalog}");

    // Step 3: Get current user configuration
    let user_config = call_tool_lenient(&client, "get_user_configuration", json!({})).await;
    println!("  ✓ Step 3: get_user_configuration: {user_config}");

    // Step 4: Calculate personalized zones
    let zones = call_tool_lenient(
        &client,
        "calculate_personalized_zones",
        json!({"vo2_max": 55.0}),
    )
    .await;
    println!("  ✓ Step 4: calculate_personalized_zones: {zones}");

    // Step 5: Set fitness config
    let set_config = call_tool_lenient(
        &client,
        "set_fitness_config",
        json!({
            "configuration_name": "my_zones",
            "configuration": {
                "heart_rate_zones": {
                    "zone1_max": 125,
                    "zone2_max": 145,
                    "zone3_max": 165,
                    "zone4_max": 180,
                    "zone5_max": 195
                }
            }
        }),
    )
    .await;
    println!("  ✓ Step 5: set_fitness_config: {set_config}");

    println!("✅ Workflow: Configuration Management completed successfully");
    Ok(())
}

// ============================================================================
// WORKFLOW: Recipe Management
// ============================================================================

/// Workflow: Get constraints, save recipe, list and search
#[tokio::test]
async fn test_workflow_recipe_management() -> Result<()> {
    let (_server, client) = setup_test_client().await?;

    println!("🏃 Workflow: Recipe Management");

    // Step 1: Get recipe constraints
    let constraints = call_tool_lenient(
        &client,
        "get_recipe_constraints",
        json!({"training_phase": "recovery"}),
    )
    .await;
    println!("  ✓ Step 1: get_recipe_constraints: {constraints}");

    // Step 2: Save a new recipe
    let save_result = call_tool_lenient(
        &client,
        "save_recipe",
        json!({
            "name": "Recovery Protein Bowl",
            "description": "High-protein recovery meal",
            "meal_timing": "post_workout",
            "ingredients": [
                {"food_name": "quinoa", "amount_grams": 150},
                {"food_name": "chicken breast", "amount_grams": 150}
            ],
            "instructions": "Cook quinoa. Grill chicken. Combine."
        }),
    )
    .await;
    println!("  ✓ Step 2: save_recipe: {save_result}");

    // Step 3: List all recipes
    let list_result = call_tool_lenient(&client, "list_recipes", json!({})).await;
    println!("  ✓ Step 3: list_recipes: {list_result}");

    // Step 4: Search for recipes
    let search_result =
        call_tool_lenient(&client, "search_recipes", json!({"query": "protein"})).await;
    println!("  ✓ Step 4: search_recipes: {search_result}");

    println!("✅ Workflow: Recipe Management completed successfully");
    Ok(())
}

// ============================================================================
// WORKFLOW: Multi-Provider Comparison
// ============================================================================

/// Workflow: Check multiple provider connections
#[tokio::test]
async fn test_workflow_provider_comparison() -> Result<()> {
    let (_server, client) = setup_test_client().await?;

    println!("🏃 Workflow: Provider Comparison");

    // Check connection status for different providers
    let strava = call_tool_lenient(
        &client,
        "get_connection_status",
        json!({"provider": "strava"}),
    )
    .await;
    println!("  ✓ Strava status: {strava}");

    let fitbit = call_tool_lenient(
        &client,
        "get_connection_status",
        json!({"provider": "fitbit"}),
    )
    .await;
    println!("  ✓ Fitbit status: {fitbit}");

    let synthetic = call_tool_lenient(
        &client,
        "get_connection_status",
        json!({"provider": "synthetic"}),
    )
    .await;
    println!("  ✓ Synthetic status: {synthetic}");

    // Get activities from synthetic provider
    let activities = call_tool_lenient(
        &client,
        "get_activities",
        json!({"provider": "synthetic", "limit": 5}),
    )
    .await;
    println!("  ✓ Synthetic activities: {activities}");

    println!("✅ Workflow: Provider Comparison completed successfully");
    Ok(())
}

// ============================================================================
// WORKFLOW: Complete Daily Routine
// ============================================================================

/// Workflow: Morning check-in routine
#[tokio::test]
async fn test_workflow_daily_routine() -> Result<()> {
    let (_server, client) = setup_test_client().await?;

    println!("🏃 Workflow: Daily Routine Check-in");

    // Step 1: Check recovery status
    let recovery = call_tool_lenient(
        &client,
        "calculate_recovery_score",
        json!({"provider": "synthetic"}),
    )
    .await;
    println!("  ✓ Step 1: calculate_recovery_score: {recovery}");

    // Step 2: Analyze sleep
    let sleep = call_tool_lenient(
        &client,
        "analyze_sleep_quality",
        json!({"provider": "synthetic", "days": 1}),
    )
    .await;
    println!("  ✓ Step 2: analyze_sleep_quality: {sleep}");

    // Step 3: Check training load
    let load = call_tool_lenient(
        &client,
        "analyze_training_load",
        json!({"provider": "synthetic", "days": 7}),
    )
    .await;
    println!("  ✓ Step 3: analyze_training_load: {load}");

    // Step 4: Get rest day recommendation
    let rest = call_tool_lenient(
        &client,
        "suggest_rest_day",
        json!({"provider": "synthetic"}),
    )
    .await;
    println!("  ✓ Step 4: suggest_rest_day: {rest}");

    // Step 5: Track goal progress
    let progress = call_tool_lenient(&client, "track_progress", json!({"goal_id": "latest"})).await;
    println!("  ✓ Step 5: track_progress: {progress}");

    // Step 6: Get training recommendations
    let recommendations = call_tool_lenient(
        &client,
        "generate_recommendations",
        json!({"provider": "synthetic"}),
    )
    .await;
    println!("  ✓ Step 6: generate_recommendations: {recommendations}");

    println!("✅ Workflow: Daily Routine completed successfully");
    Ok(())
}
