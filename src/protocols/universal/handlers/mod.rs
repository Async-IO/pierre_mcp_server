// ABOUTME: Tool handlers with single responsibilities
// ABOUTME: Clean separation of concerns replacing monolithic handler functions
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

/// Configuration management tool handlers
pub mod configuration;
/// OAuth provider connection tool handlers
pub mod connections;
/// Fitness provider API integration tool handlers (provider-agnostic)
pub mod fitness_api;
/// Goal setting and tracking tool handlers
pub mod goals;
/// Activity intelligence and analysis tool handlers
pub mod intelligence;
/// Nutrition analysis and USDA database tool handlers
pub mod nutrition;
/// Shared provider helper functions
pub mod provider_helpers;
/// Sleep quality and recovery analysis tool handlers
pub mod sleep_recovery;

// Configuration management handlers
pub use configuration::{
    handle_calculate_personalized_zones, handle_get_configuration_catalog,
    handle_get_configuration_profiles, handle_get_user_configuration,
    handle_update_user_configuration, handle_validate_configuration,
};

// OAuth provider connection handlers

/// Connect to OAuth provider
pub use connections::handle_connect_provider;
/// Disconnect from OAuth provider
pub use connections::handle_disconnect_provider;
/// Get OAuth connection status
pub use connections::handle_get_connection_status;

// Goal setting and tracking handlers

/// Analyze goal feasibility based on training history
pub use goals::handle_analyze_goal_feasibility;
/// Set a new fitness goal
pub use goals::handle_set_goal;
/// Suggest personalized fitness goals
pub use goals::handle_suggest_goals;
/// Track progress toward goals
pub use goals::handle_track_progress;

// Activity intelligence and analysis handlers

/// Analyze performance trends over time
pub use intelligence::handle_analyze_performance_trends;
/// Analyze training load and fatigue
pub use intelligence::handle_analyze_training_load;
/// Calculate overall fitness score
pub use intelligence::handle_calculate_fitness_score;
/// Calculate detailed activity metrics
pub use intelligence::handle_calculate_metrics;
/// Compare multiple activities
pub use intelligence::handle_compare_activities;
/// Detect training patterns
pub use intelligence::handle_detect_patterns;
/// Generate training recommendations
pub use intelligence::handle_generate_recommendations;
/// Get comprehensive activity intelligence
pub use intelligence::handle_get_activity_intelligence;
/// Predict performance for goal distance
pub use intelligence::handle_predict_performance;

// Sleep and recovery analysis handlers
pub use sleep_recovery::{
    handle_analyze_sleep_quality, handle_calculate_recovery_score, handle_optimize_sleep_schedule,
    handle_suggest_rest_day, handle_track_sleep_trends,
};

/// Re-export nutrition analysis and USDA food database handlers
pub use nutrition::{
    handle_analyze_meal_nutrition, handle_calculate_daily_nutrition, handle_get_food_details,
    handle_get_nutrient_timing, handle_search_food,
};

/// Re-export fitness provider API integration handlers (provider-agnostic)
pub use fitness_api::{
    handle_analyze_activity, handle_get_activities, handle_get_athlete, handle_get_stats,
};
