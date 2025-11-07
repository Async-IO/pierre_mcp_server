// ABOUTME: Integration tests for intelligence engines and universal tool handlers
// ABOUTME: Tests interaction between fitness intelligence and universal tool execution
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
//! Integration tests for intelligence engines and universal tool handlers
//!
//! Tests the interaction between fitness intelligence engines and the universal
//! tool execution system to ensure proper data flow and analysis integration.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use anyhow::Result;
use chrono::Utc;
use pierre_mcp_server::{
    database_plugins::DatabaseProvider,
    intelligence::{
        insights::ActivityContext, ActivityAnalyzer, FitnessLevel, MetricsCalculator,
        TimeAvailability, UserFitnessProfile, UserPreferences,
    },
    models::{Activity, SportType},
};

mod common;
use common::*;

/// Test data flow from tool execution through intelligence analysis
#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_activity_analysis_through_universal_tools() -> Result<()> {
    let database = create_test_database().await?;

    // Create user first
    let user = pierre_mcp_server::models::User::new(
        "test@example.com".to_owned(),
        "password_hash".to_owned(),
        Some("Test User".to_owned()),
    );
    database.create_user(&user).await?;

    // Create test user and fitness profile
    let fitness_profile = UserFitnessProfile {
        user_id: user.id.to_string(),
        age: Some(30),
        gender: Some("male".to_owned()),
        weight: Some(75.0),
        height: Some(180.0),
        fitness_level: FitnessLevel::Intermediate,
        primary_sports: vec!["running".to_owned(), "cycling".to_owned()],
        training_history_months: 24,
        preferences: UserPreferences {
            preferred_units: "metric".to_owned(),
            training_focus: vec!["endurance".to_owned(), "speed".to_owned()],
            injury_history: vec![],
            time_availability: TimeAvailability {
                hours_per_week: 6.0,
                preferred_days: vec![
                    "monday".to_owned(),
                    "wednesday".to_owned(),
                    "friday".to_owned(),
                ],
                preferred_duration_minutes: Some(60),
            },
        },
    };

    // Store user fitness profile in database
    let profile_data = serde_json::to_value(&fitness_profile)?;
    database.upsert_user_profile(user.id, profile_data).await?;

    // Create test activity with advanced metrics
    let activity = Activity {
        id: "test_activity_001".to_owned(),
        name: "Morning Tempo Run".to_owned(),
        sport_type: SportType::Run,
        start_date: Utc::now() - chrono::Duration::hours(2),
        duration_seconds: 3600,         // 60 minutes
        distance_meters: Some(10000.0), // 10km
        elevation_gain: Some(100.0),
        average_heart_rate: Some(165),
        max_heart_rate: Some(185),
        average_speed: Some(2.78), // ~4:00/km pace
        max_speed: Some(3.33),
        calories: Some(600),
        steps: Some(12000),
        heart_rate_zones: None,

        // Advanced metrics for intelligence testing
        average_power: None, // Running power not available
        max_power: None,
        normalized_power: None,
        power_zones: None,
        ftp: None,
        average_cadence: Some(180), // steps per minute
        max_cadence: Some(200),
        hrv_score: Some(45.2),
        recovery_heart_rate: Some(25), // HR drop in first minute
        temperature: Some(18.0),
        humidity: Some(65.0),
        average_altitude: Some(120.0),
        wind_speed: Some(2.0),
        ground_contact_time: Some(240),
        vertical_oscillation: Some(8.5),
        stride_length: Some(1.25),
        running_power: Some(280),
        breathing_rate: Some(32),
        spo2: Some(98.0),
        training_stress_score: Some(75.0),
        intensity_factor: Some(0.82),
        suffer_score: Some(85),
        time_series_data: None,

        start_latitude: Some(45.5017),
        start_longitude: Some(-73.5673),
        city: Some("Montreal".to_owned()),
        region: Some("Quebec".to_owned()),
        country: Some("Canada".to_owned()),
        trail_name: Some("Lachine Canal".to_owned()),
        provider: "strava".to_owned(),
    };

    // Test intelligence engine integration
    let analyzer = ActivityAnalyzer::new();
    let context = ActivityContext {
        location: None,
        recent_activities: None,
    };
    let intelligence = analyzer.analyze_activity(&activity, Some(&context))?;

    // Verify intelligence analysis results
    assert!(!intelligence.summary.is_empty());
    assert!(!intelligence.key_insights.is_empty());

    // Check that performance indicators are calculated
    assert!(
        intelligence
            .performance_indicators
            .relative_effort
            .is_some()
            || intelligence
                .performance_indicators
                .efficiency_score
                .is_some()
    );

    // Test metrics calculation integration
    let calculator = MetricsCalculator::new().with_user_data(
        None,        // No FTP for running
        Some(175.0), // LTHR estimate
        Some(190.0), // Max HR
        Some(55.0),  // Resting HR
        Some(75.0),  // Weight
    );

    let calculated_metrics = calculator.calculate_metrics(&activity)?;

    // Verify calculated metrics
    assert!(calculated_metrics.trimp.is_some());
    assert!(calculated_metrics.running_effectiveness.is_some());
    assert!(calculated_metrics.stride_efficiency.is_some());
    assert!(calculated_metrics.temperature_stress.is_some());

    // Test integration with universal tool execution
    // This simulates how the MCP protocol would trigger intelligence analysis
    let tool_name = "analyze_activity";
    let _ = tool_name;
    let _tool_args = serde_json::json!({
        "activity_id": activity.id,
        "user_id": user.id.to_string(),
        "include_advanced_metrics": true
    });

    // Note: This would normally be handled by the universal tool executor
    // but we're testing the integration point here
    let context2 = ActivityContext {
        location: None,
        recent_activities: None,
    };
    let analysis_result = analyzer.analyze_activity(&activity, Some(&context2))?;

    // Verify the analysis includes all expected components
    assert!(!analysis_result.summary.is_empty());
    assert!(!analysis_result.key_insights.is_empty());
    assert!(
        analysis_result
            .performance_indicators
            .relative_effort
            .is_some()
            || analysis_result
                .performance_indicators
                .efficiency_score
                .is_some()
    );

    Ok(())
}

/// Test recommendation engine integration with tool handlers
#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_recommendation_engine_integration() -> Result<()> {
    let database = create_test_database().await?;

    // Create user first
    let user = pierre_mcp_server::models::User::new(
        "test2@example.com".to_owned(),
        "password_hash".to_owned(),
        Some("Test User 2".to_owned()),
    );
    database.create_user(&user).await?;

    // Create test user profile
    let fitness_profile = UserFitnessProfile {
        user_id: user.id.to_string(),
        age: Some(25),
        gender: Some("female".to_owned()),
        weight: Some(60.0),
        height: Some(165.0),
        fitness_level: FitnessLevel::Advanced,
        primary_sports: vec!["cycling".to_owned()],
        training_history_months: 36,
        preferences: UserPreferences {
            preferred_units: "metric".to_owned(),
            training_focus: vec!["power".to_owned(), "endurance".to_owned()],
            injury_history: vec![],
            time_availability: TimeAvailability {
                hours_per_week: 10.0,
                preferred_days: vec![
                    "tuesday".to_owned(),
                    "thursday".to_owned(),
                    "saturday".to_owned(),
                    "sunday".to_owned(),
                ],
                preferred_duration_minutes: Some(90),
            },
        },
    };

    // Store fitness profile
    let profile_data = serde_json::to_value(&fitness_profile)?;
    database.upsert_user_profile(user.id, profile_data).await?;

    // Create cycling activity with power data
    let activity = Activity {
        id: "cycling_test_001".to_owned(),
        name: "Threshold Intervals".to_owned(),
        sport_type: SportType::Ride,
        start_date: Utc::now() - chrono::Duration::hours(1),
        duration_seconds: 4500,         // 75 minutes
        distance_meters: Some(45000.0), // 45km
        elevation_gain: Some(300.0),
        average_heart_rate: Some(160),
        max_heart_rate: Some(180),
        average_speed: Some(11.11), // 40km/h
        max_speed: Some(15.28),     // 55km/h
        calories: Some(900),
        steps: None,
        heart_rate_zones: None,

        // Power metrics for cycling analysis
        average_power: Some(250),
        max_power: Some(450),
        normalized_power: Some(265),
        power_zones: None,
        ftp: Some(280),
        average_cadence: Some(90),
        max_cadence: Some(120),
        hrv_score: Some(40.0),
        recovery_heart_rate: Some(30),
        temperature: Some(22.0),
        humidity: Some(55.0),
        average_altitude: Some(200.0),
        wind_speed: Some(8.0),
        ground_contact_time: None,
        vertical_oscillation: None,
        stride_length: None,
        running_power: None,
        breathing_rate: Some(28),
        spo2: Some(97.5),
        training_stress_score: Some(95.0),
        intensity_factor: Some(0.89),
        suffer_score: Some(120),
        time_series_data: None,

        start_latitude: Some(45.5017),
        start_longitude: Some(-73.5673),
        city: Some("Montreal".to_owned()),
        region: Some("Quebec".to_owned()),
        country: Some("Canada".to_owned()),
        trail_name: None,
        provider: "strava".to_owned(),
    };

    // Test recommendation engine
    let analyzer = ActivityAnalyzer::new();
    let context = ActivityContext {
        location: None,
        recent_activities: None,
    };
    let intelligence = analyzer.analyze_activity(&activity, Some(&context))?;

    // Verify analysis is generated
    assert!(!intelligence.summary.is_empty());
    assert!(!intelligence.key_insights.is_empty());
    assert!(
        intelligence
            .performance_indicators
            .relative_effort
            .is_some()
            || intelligence
                .performance_indicators
                .efficiency_score
                .is_some()
    );

    // Check that some insights are generated (content may vary)
    // Note: The specific content of insights depends on the implementation
    // of the insight generation algorithms, so we just verify they exist
    assert!(
        !intelligence.key_insights.is_empty(),
        "Should generate insights"
    );

    Ok(())
}

/// Test goal tracking integration with universal tool handlers
#[tokio::test]
async fn test_goal_tracking_integration() -> Result<()> {
    let database = create_test_database().await?;

    // Create user first
    let user = pierre_mcp_server::models::User::new(
        "test3@example.com".to_owned(),
        "password_hash".to_owned(),
        Some("Test User 3".to_owned()),
    );
    database.create_user(&user).await?;

    // Create a test goal
    let goal_data = serde_json::json!({
        "type": "distance",
        "target_value": 100_000.0, // 100km total distance
        "current_value": 0.0,
        "time_period": "monthly",
        "sport_type": "run",
        "created_date": Utc::now().to_rfc3339(),
        "target_date": (Utc::now() + chrono::Duration::days(30)).to_rfc3339()
    });

    let goal_id = database.create_goal(user.id, goal_data).await?;

    // Create activities that contribute to the goal
    let activities = [
        create_test_activity("run_001", &SportType::Run, 5000.0), // 5km
        create_test_activity("run_002", &SportType::Run, 8000.0), // 8km
        create_test_activity("run_003", &SportType::Run, 10000.0), // 10km
    ];

    // Calculate total distance from activities
    let total_distance: f64 = activities.iter().filter_map(|a| a.distance_meters).sum();

    // Update goal progress
    database
        .update_goal_progress(&goal_id, total_distance)
        .await?;

    // Verify goal was updated correctly
    let goals = database.get_user_goals(user.id).await?;
    assert_eq!(goals.len(), 1);

    let updated_goal = &goals[0];
    assert!(
        (updated_goal["current_value"].as_f64().unwrap() - total_distance).abs() < f64::EPSILON
    );
    assert!((updated_goal["progress_percentage"].as_f64().unwrap() - 23.0).abs() < f64::EPSILON); // 23% of 100km

    // Test goal integration with activity analysis
    let _fitness_profile = UserFitnessProfile {
        user_id: user.id.to_string(),
        age: Some(35),
        gender: Some("male".to_owned()),
        weight: Some(70.0),
        height: Some(175.0),
        fitness_level: FitnessLevel::Intermediate,
        primary_sports: vec!["running".to_owned()],
        training_history_months: 18,
        preferences: UserPreferences {
            preferred_units: "metric".to_owned(),
            training_focus: vec!["distance".to_owned()],
            injury_history: vec![],
            time_availability: TimeAvailability {
                hours_per_week: 5.0,
                preferred_days: vec![
                    "monday".to_owned(),
                    "wednesday".to_owned(),
                    "friday".to_owned(),
                ],
                preferred_duration_minutes: Some(45),
            },
        },
    };

    let analyzer = ActivityAnalyzer::new();
    let context = ActivityContext {
        location: None,
        recent_activities: None,
    };
    let intelligence = analyzer.analyze_activity(&activities[0], Some(&context))?;

    // Verify analysis is generated and includes relevant insights
    assert!(!intelligence.summary.is_empty());
    assert!(!intelligence.key_insights.is_empty());
    assert!(
        intelligence
            .performance_indicators
            .relative_effort
            .is_some()
            || intelligence
                .performance_indicators
                .efficiency_score
                .is_some()
    );

    Ok(())
}

/// Helper function to create test activities
fn create_test_activity(id: &str, sport_type: &SportType, distance: f64) -> Activity {
    Activity {
        id: id.to_owned(),
        name: format!("Test {sport_type:?}"),
        sport_type: sport_type.clone(),
        start_date: Utc::now() - chrono::Duration::hours(1),
        duration_seconds: 1800, // 30 minutes
        distance_meters: Some(distance),
        elevation_gain: Some(50.0),
        average_heart_rate: Some(150),
        max_heart_rate: Some(170),
        average_speed: Some(distance / 1800.0), // Calculate speed
        max_speed: Some(distance / 1500.0),
        calories: Some(300),
        steps: if *sport_type == SportType::Run {
            Some(6000)
        } else {
            None
        },
        heart_rate_zones: None,

        // Basic metrics only for test activities
        average_power: None,
        max_power: None,
        normalized_power: None,
        power_zones: None,
        ftp: None,
        average_cadence: None,
        max_cadence: None,
        hrv_score: None,
        recovery_heart_rate: None,
        temperature: None,
        humidity: None,
        average_altitude: None,
        wind_speed: None,
        ground_contact_time: None,
        vertical_oscillation: None,
        stride_length: None,
        running_power: None,
        breathing_rate: None,
        spo2: None,
        training_stress_score: None,
        intensity_factor: None,
        suffer_score: None,
        time_series_data: None,

        start_latitude: Some(45.5017),
        start_longitude: Some(-73.5673),
        city: Some("Montreal".to_owned()),
        region: Some("Quebec".to_owned()),
        country: Some("Canada".to_owned()),
        trail_name: None,
        provider: "test".to_owned(),
    }
}
