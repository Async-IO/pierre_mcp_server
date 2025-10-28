// ABOUTME: Tool handlers with single responsibilities
// ABOUTME: Clean separation of concerns replacing monolithic handler functions
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

pub mod configuration;
pub mod connections;
pub mod goals;
pub mod intelligence;
pub mod strava_api;

// Configuration management handlers
pub use configuration::{
    handle_calculate_personalized_zones, handle_get_configuration_catalog,
    handle_get_configuration_profiles, handle_get_user_configuration,
    handle_update_user_configuration, handle_validate_configuration,
};

// OAuth provider connection handlers
pub use connections::{
    handle_connect_provider, handle_disconnect_provider, handle_get_connection_status,
};

// Goal setting and tracking handlers
pub use goals::{
    handle_analyze_goal_feasibility, handle_set_goal, handle_suggest_goals, handle_track_progress,
};

// Activity intelligence and analysis handlers
pub use intelligence::{
    handle_analyze_performance_trends, handle_analyze_training_load,
    handle_calculate_fitness_score, handle_calculate_metrics, handle_compare_activities,
    handle_detect_patterns, handle_generate_recommendations, handle_get_activity_intelligence,
    handle_predict_performance,
};

// Strava API integration handlers
pub use strava_api::{
    handle_analyze_activity, handle_get_activities, handle_get_athlete, handle_get_stats,
};
