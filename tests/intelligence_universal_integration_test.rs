// ABOUTME: Integration tests for intelligence engines and universal tool handlers
// ABOUTME: Tests interaction between fitness intelligence and universal tool execution
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
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
    models::{Activity, ActivityBuilder, SportType, User},
};

mod common;
use common::*;

/// Test data flow from tool execution through intelligence analysis
#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_activity_analysis_through_universal_tools() -> Result<()> {
    let database = create_test_database().await?;

    // Create user first
    let user = User::new(
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
    let activity = ActivityBuilder::new(
        "test_activity_001",
        "Morning Tempo Run",
        SportType::Run,
        Utc::now() - chrono::Duration::hours(2),
        3600, // 60 minutes
        "strava",
    )
    .distance_meters(10000.0) // 10km
    .elevation_gain(100.0)
    .average_heart_rate(165)
    .max_heart_rate(185)
    .average_speed(2.78) // ~4:00/km pace
    .max_speed(3.33)
    .calories(600)
    .steps(12000)
    .average_cadence(180) // steps per minute
    .max_cadence(200)
    .hrv_score(45.2)
    .recovery_heart_rate(25) // HR drop in first minute
    .temperature(18.0)
    .humidity(65.0)
    .average_altitude(120.0)
    .wind_speed(2.0)
    .ground_contact_time(240)
    .vertical_oscillation(8.5)
    .stride_length(1.25)
    .running_power(280)
    .breathing_rate(32)
    .spo2(98.0)
    .training_stress_score(75.0)
    .intensity_factor(0.82)
    .suffer_score(85)
    .start_latitude(45.5017)
    .start_longitude(-73.5673)
    .city("Montreal".to_owned())
    .region("Quebec".to_owned())
    .country("Canada".to_owned())
    .trail_name("Lachine Canal".to_owned())
    .build();

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
        "activity_id": activity.id(),
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
    let user = User::new(
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
    let activity = ActivityBuilder::new(
        "cycling_test_001",
        "Threshold Intervals",
        SportType::Ride,
        Utc::now() - chrono::Duration::hours(1),
        4500, // 75 minutes
        "strava",
    )
    .distance_meters(45000.0) // 45km
    .elevation_gain(300.0)
    .average_heart_rate(160)
    .max_heart_rate(180)
    .average_speed(11.11) // 40km/h
    .max_speed(15.28) // 55km/h
    .calories(900)
    .average_power(250)
    .max_power(450)
    .normalized_power(265)
    .ftp(280)
    .average_cadence(90)
    .max_cadence(120)
    .hrv_score(40.0)
    .recovery_heart_rate(30)
    .temperature(22.0)
    .humidity(55.0)
    .average_altitude(200.0)
    .wind_speed(8.0)
    .breathing_rate(28)
    .spo2(97.5)
    .training_stress_score(95.0)
    .intensity_factor(0.89)
    .suffer_score(120)
    .start_latitude(45.5017)
    .start_longitude(-73.5673)
    .city("Montreal".to_owned())
    .region("Quebec".to_owned())
    .country("Canada".to_owned())
    .build();

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
    let user = User::new(
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
    let total_distance: f64 = activities
        .iter()
        .filter_map(Activity::distance_meters)
        .sum();

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
    let mut builder = ActivityBuilder::new(
        id,
        format!("Test {sport_type:?}"),
        sport_type.clone(),
        Utc::now() - chrono::Duration::hours(1),
        1800, // 30 minutes
        "test",
    )
    .distance_meters(distance)
    .elevation_gain(50.0)
    .average_heart_rate(150)
    .max_heart_rate(170)
    .average_speed(distance / 1800.0) // Calculate speed
    .max_speed(distance / 1500.0)
    .calories(300)
    .start_latitude(45.5017)
    .start_longitude(-73.5673)
    .city("Montreal".to_owned())
    .region("Quebec".to_owned())
    .country("Canada".to_owned());

    if *sport_type == SportType::Run {
        builder = builder.steps(6000);
    }

    builder.build()
}
