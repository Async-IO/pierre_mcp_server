// ABOUTME: Test fixtures for integration tests
// ABOUTME: Provides predefined test data and argument builders for MCP tools
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Collection of test fixtures for integration tests
pub struct TestFixtures;

impl TestFixtures {
    // ========================================================================
    // Provider Constants
    // ========================================================================

    /// Synthetic provider name
    pub const PROVIDER_SYNTHETIC: &'static str = "synthetic";

    /// Synthetic sleep provider name
    pub const PROVIDER_SYNTHETIC_SLEEP: &'static str = "synthetic_sleep";

    // ========================================================================
    // Default Test Seeds
    // ========================================================================

    /// Default seed for reproducible tests
    pub const DEFAULT_SEED: u64 = 12345;

    /// Alternative seed for comparison tests
    pub const ALT_SEED: u64 = 67890;

    // ========================================================================
    // Tool Argument Builders
    // ========================================================================

    /// Build arguments for get_activities tool
    pub fn get_activities_args(provider: &str, limit: Option<u32>) -> Value {
        let mut args = json!({ "provider": provider });
        if let Some(l) = limit {
            args["limit"] = json!(l);
        }
        args
    }

    /// Build arguments for get_connection_status tool
    pub fn connection_status_args(provider: &str) -> Value {
        json!({ "provider": provider })
    }

    /// Build arguments for analyze_training_load tool
    pub fn training_load_args(days: u32) -> Value {
        json!({ "days": days })
    }

    /// Build arguments for calculate_fitness_score tool
    pub fn fitness_score_args() -> Value {
        json!({})
    }

    /// Build arguments for generate_recommendations tool
    pub fn recommendations_args() -> Value {
        json!({})
    }

    /// Build arguments for set_goal tool
    pub fn set_goal_args(
        goal_type: &str,
        target_value: f64,
        target_unit: &str,
        deadline_days: u32,
    ) -> Value {
        json!({
            "goal_type": goal_type,
            "target_value": target_value,
            "target_unit": target_unit,
            "deadline_days": deadline_days
        })
    }

    /// Build arguments for track_progress tool
    pub fn track_progress_args(goal_id: &str) -> Value {
        json!({ "goal_id": goal_id })
    }

    /// Build arguments for analyze_sleep_quality tool
    pub fn sleep_quality_args(days: u32) -> Value {
        json!({ "days": days })
    }

    /// Build arguments for calculate_recovery_score tool
    pub fn recovery_score_args() -> Value {
        json!({})
    }

    /// Build arguments for calculate_daily_nutrition tool
    pub fn daily_nutrition_args(activity_level: &str, goal: &str) -> Value {
        json!({
            "activity_level": activity_level,
            "goal": goal
        })
    }

    /// Build arguments for search_food tool
    pub fn search_food_args(query: &str, limit: Option<u32>) -> Value {
        let mut args = json!({ "query": query });
        if let Some(l) = limit {
            args["limit"] = json!(l);
        }
        args
    }

    /// Build arguments for get_fitness_config tool
    pub fn fitness_config_args(config_type: &str) -> Value {
        json!({ "config_type": config_type })
    }

    /// Build arguments for save_recipe tool
    pub fn save_recipe_args(name: &str, ingredients: Vec<RecipeIngredient>) -> Value {
        json!({
            "name": name,
            "ingredients": ingredients
        })
    }

    /// Build arguments for list_recipes tool
    pub fn list_recipes_args(limit: Option<u32>) -> Value {
        let mut args = json!({});
        if let Some(l) = limit {
            args["limit"] = json!(l);
        }
        args
    }
}

/// Recipe ingredient for save_recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeIngredient {
    pub name: String,
    pub amount: f64,
    pub unit: String,
}

impl RecipeIngredient {
    pub fn new(name: &str, amount: f64, unit: &str) -> Self {
        Self {
            name: name.to_owned(),
            amount,
            unit: unit.to_owned(),
        }
    }
}

// ============================================================================
// Expected Response Types (for deserializing tool results)
// ============================================================================

/// Activity from get_activities response
#[derive(Debug, Clone, Deserialize)]
pub struct ActivityResponse {
    pub id: String,
    pub name: String,
    pub sport_type: String,
    #[serde(default)]
    pub distance_meters: Option<f64>,
    #[serde(default)]
    pub duration_seconds: Option<u64>,
}

/// Activities list response
#[derive(Debug, Clone, Deserialize)]
pub struct ActivitiesListResponse {
    pub activities: Vec<ActivityResponse>,
    #[serde(default)]
    pub total: Option<u32>,
}

/// Connection status response
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionStatusResponse {
    pub provider: String,
    pub connected: bool,
    #[serde(default)]
    pub athlete_id: Option<String>,
    #[serde(default)]
    pub scopes: Option<Vec<String>>,
}

/// Training load response
#[derive(Debug, Clone, Deserialize)]
pub struct TrainingLoadResponse {
    #[serde(default)]
    pub acute_load: Option<f64>,
    #[serde(default)]
    pub chronic_load: Option<f64>,
    #[serde(default)]
    pub training_status: Option<String>,
}

/// Fitness score response
#[derive(Debug, Clone, Deserialize)]
pub struct FitnessScoreResponse {
    pub score: u32,
    #[serde(default)]
    pub components: Option<Value>,
}

/// Goal response
#[derive(Debug, Clone, Deserialize)]
pub struct GoalResponse {
    pub id: String,
    pub goal_type: String,
    pub target_value: f64,
    #[serde(default)]
    pub current_value: Option<f64>,
    #[serde(default)]
    pub progress_percentage: Option<f64>,
}

/// Sleep quality response
#[derive(Debug, Clone, Deserialize)]
pub struct SleepQualityResponse {
    #[serde(default)]
    pub average_score: Option<f64>,
    #[serde(default)]
    pub average_duration_hours: Option<f64>,
    #[serde(default)]
    pub trend: Option<String>,
}

/// Recovery score response
#[derive(Debug, Clone, Deserialize)]
pub struct RecoveryScoreResponse {
    pub score: u32,
    #[serde(default)]
    pub readiness: Option<String>,
    #[serde(default)]
    pub recommendations: Option<Vec<String>>,
}

/// Recommendation item
#[derive(Debug, Clone, Deserialize)]
pub struct RecommendationItem {
    #[serde(default)]
    pub category: Option<String>,
    pub recommendation: String,
    #[serde(default)]
    pub priority: Option<String>,
}

/// Recommendations response
#[derive(Debug, Clone, Deserialize)]
pub struct RecommendationsResponse {
    pub recommendations: Vec<RecommendationItem>,
}
