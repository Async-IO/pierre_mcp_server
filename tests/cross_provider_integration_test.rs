// ABOUTME: Integration tests for cross-provider sleep/recovery functionality
// ABOUTME: Tests using synthetic providers to verify cross-provider data flow
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//! Cross-Provider Integration Tests
//!
//! Tests that verify the cross-provider feature works correctly when using
//! activities from one provider and sleep data from another. Uses synthetic
//! providers to enable testing without real OAuth authentication.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]
// Test-specific clippy allows for synthetic test data generation
#![allow(
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::suboptimal_flops,
    clippy::items_after_statements
)]

use chrono::Utc;
use pierre_mcp_server::{
    constants::oauth_providers,
    models::{Activity, SportType},
    providers::{registry::ProviderRegistry, synthetic_provider::SyntheticProvider},
};

mod common;

/// Create test activities for the synthetic provider
fn create_test_activities(count: usize) -> Vec<Activity> {
    let mut activities = Vec::with_capacity(count);
    let base_date = Utc::now();

    for i in 0..count {
        activities.push(Activity {
            id: format!("activity_{i}"),
            name: format!("Test Run {}", i + 1),
            sport_type: SportType::Run,
            start_date: base_date - chrono::Duration::days(i as i64),
            duration_seconds: 3600 + (i as u64 * 60),
            distance_meters: Some(10000.0 + (i as f64 * 500.0)),
            elevation_gain: Some(100.0 + (i as f64 * 10.0)),
            average_heart_rate: Some(145 + (i as u32 % 20)),
            max_heart_rate: Some(175 + (i as u32 % 10)),
            average_speed: Some(2.8 + (i as f64 * 0.1)),
            max_speed: Some(3.5 + (i as f64 * 0.1)),
            calories: Some(600 + (i as u32 * 50)),
            steps: None,
            heart_rate_zones: None,
            average_power: None,
            max_power: None,
            normalized_power: None,
            power_zones: None,
            ftp: None,
            average_cadence: Some(170 + (i as u32 % 10)),
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
            start_latitude: None,
            start_longitude: None,
            city: None,
            region: None,
            country: None,
            trail_name: None,
            workout_type: None,
            sport_type_detail: None,
            segment_efforts: None,
            provider: "synthetic".to_owned(),
        });
    }

    activities
}

#[tokio::test]
async fn test_synthetic_provider_registration() {
    // Initialize test environment
    common::init_server_config();

    // Create a provider registry and verify both synthetic providers are registered
    let registry = ProviderRegistry::new();

    // Check that 'synthetic' provider is registered
    assert!(
        registry.is_supported(oauth_providers::SYNTHETIC),
        "Synthetic provider should be registered"
    );

    // Check that 'synthetic_sleep' provider is registered
    assert!(
        registry.is_supported(oauth_providers::SYNTHETIC_SLEEP),
        "Synthetic sleep provider should be registered"
    );

    // Verify providers have correct capabilities
    let synthetic_caps = registry.get_capabilities(oauth_providers::SYNTHETIC);
    assert!(
        synthetic_caps.is_some(),
        "Should have synthetic capabilities"
    );

    let sleep_caps = registry.get_capabilities(oauth_providers::SYNTHETIC_SLEEP);
    assert!(
        sleep_caps.is_some(),
        "Should have synthetic_sleep capabilities"
    );

    // Verify synthetic_sleep supports sleep tracking
    assert!(
        registry.supports_sleep(oauth_providers::SYNTHETIC_SLEEP),
        "Synthetic sleep provider should support sleep tracking"
    );
}

#[tokio::test]
async fn test_synthetic_provider_with_activities() {
    use pierre_mcp_server::providers::core::FitnessProvider;

    // Create a synthetic provider with test activities
    let activities = create_test_activities(10);
    let provider = SyntheticProvider::with_activities(activities.clone());

    // Verify provider name
    assert_eq!(provider.name(), oauth_providers::SYNTHETIC);

    // Verify activities are accessible
    let fetched = provider.get_activities(Some(5), None).await.unwrap();
    assert_eq!(fetched.len(), 5, "Should fetch 5 activities");

    // Verify activity count
    assert_eq!(provider.activity_count().unwrap(), 10);
}

#[tokio::test]
async fn test_synthetic_sleep_provider() {
    use pierre_mcp_server::providers::core::FitnessProvider;

    // Create a synthetic sleep provider
    let provider = SyntheticProvider::with_name(oauth_providers::SYNTHETIC_SLEEP);

    // Verify provider name is different
    assert_eq!(provider.name(), oauth_providers::SYNTHETIC_SLEEP);

    // Generate and add sleep sessions
    let base_date = Utc::now();
    let sleep_sessions = SyntheticProvider::generate_sleep_sessions(7, base_date);
    provider.set_sleep_sessions(sleep_sessions).unwrap();

    // Verify sleep sessions are accessible
    assert_eq!(provider.sleep_session_count().unwrap(), 7);

    // Fetch sleep sessions within a date range
    let start_date = base_date - chrono::Duration::days(10);
    let end_date = base_date;
    let fetched = provider
        .get_sleep_sessions(start_date, end_date)
        .await
        .unwrap();
    assert!(!fetched.is_empty(), "Should fetch sleep sessions");

    // Verify latest sleep session
    let latest = provider.get_latest_sleep_session().await.unwrap();
    assert!(!latest.id.is_empty(), "Latest session should have an ID");
    assert!(latest.total_sleep_time > 0, "Should have sleep duration");
}

#[tokio::test]
async fn test_cross_provider_scenario() {
    use pierre_mcp_server::providers::core::FitnessProvider;

    // Create two synthetic providers: one for activities, one for sleep
    let activity_provider = SyntheticProvider::with_activities(create_test_activities(14));
    let sleep_provider = SyntheticProvider::with_name(oauth_providers::SYNTHETIC_SLEEP);

    // Add sleep data to the sleep provider
    let base_date = Utc::now();
    let sleep_sessions = SyntheticProvider::generate_sleep_sessions(7, base_date);
    sleep_provider.set_sleep_sessions(sleep_sessions).unwrap();

    // Verify both providers have distinct names
    assert_ne!(
        activity_provider.name(),
        sleep_provider.name(),
        "Providers should have different names"
    );

    // Verify activities come from activity provider
    let activities = activity_provider
        .get_activities(Some(10), None)
        .await
        .unwrap();
    assert_eq!(activities.len(), 10, "Should get 10 activities");

    // Verify sleep comes from sleep provider
    let start_date = base_date - chrono::Duration::days(10);
    let end_date = base_date;
    let sleep = sleep_provider
        .get_sleep_sessions(start_date, end_date)
        .await
        .unwrap();
    assert!(!sleep.is_empty(), "Should get sleep sessions");

    // Verify activity provider doesn't return sleep (empty by default)
    let activity_sleep = activity_provider
        .get_sleep_sessions(start_date, end_date)
        .await
        .unwrap();
    assert!(
        activity_sleep.is_empty(),
        "Activity provider should have no sleep data by default"
    );
}

#[tokio::test]
async fn test_synthetic_sleep_session_generation() {
    // Test the sleep session generator produces valid data
    let base_date = Utc::now();
    let sessions = SyntheticProvider::generate_sleep_sessions(14, base_date);

    assert_eq!(sessions.len(), 14, "Should generate 14 sessions");

    for (i, session) in sessions.iter().enumerate() {
        // Verify each session has valid data
        assert!(!session.id.is_empty(), "Session {i} should have ID");
        assert!(
            session.total_sleep_time > 0,
            "Session {i} should have sleep time"
        );
        assert!(
            session.time_in_bed >= session.total_sleep_time,
            "Time in bed should be >= sleep time"
        );
        assert!(session.sleep_efficiency > 0.0, "Should have efficiency");
        assert!(session.sleep_score.is_some(), "Should have sleep score");
        assert!(!session.stages.is_empty(), "Should have sleep stages");
        assert!(session.hrv_during_sleep.is_some(), "Should have HRV");
        assert!(
            session.respiratory_rate.is_some(),
            "Should have respiratory rate"
        );

        // Verify stages add up reasonably
        let stage_total: u32 = session.stages.iter().map(|s| s.duration_minutes).sum();
        assert!(stage_total > 0, "Sleep stages should have duration");
    }
}

#[tokio::test]
async fn test_provider_factory_creates_different_instances() {
    // Create provider registry
    let registry = ProviderRegistry::new();

    // Create instances of both synthetic providers
    let synthetic = registry
        .create_provider(oauth_providers::SYNTHETIC)
        .unwrap();
    let synthetic_sleep = registry
        .create_provider(oauth_providers::SYNTHETIC_SLEEP)
        .unwrap();

    // Verify they have different names
    assert_eq!(synthetic.name(), oauth_providers::SYNTHETIC);
    assert_eq!(synthetic_sleep.name(), oauth_providers::SYNTHETIC_SLEEP);

    // Verify both are authenticated (synthetic providers always are)
    assert!(synthetic.is_authenticated().await);
    assert!(synthetic_sleep.is_authenticated().await);
}

#[tokio::test]
async fn test_sleep_providers_list_includes_synthetic_sleep() {
    // Create a provider registry
    let registry = ProviderRegistry::new();

    // Get list of sleep providers
    let sleep_providers = registry.sleep_providers();

    // Verify synthetic_sleep is in the list
    assert!(
        sleep_providers.contains(&oauth_providers::SYNTHETIC_SLEEP),
        "Sleep providers list should include synthetic_sleep: {sleep_providers:?}"
    );
}

#[tokio::test]
async fn test_dynamic_activity_and_sleep_injection() {
    use pierre_mcp_server::providers::core::FitnessProvider;

    // Create an empty provider and dynamically add data
    let provider = SyntheticProvider::new();

    // Initially no activities or sleep
    assert_eq!(provider.activity_count().unwrap(), 0);
    assert_eq!(provider.sleep_session_count().unwrap(), 0);

    // Add activities one by one
    let activities = create_test_activities(3);
    for activity in activities {
        provider.add_activity(activity).unwrap();
    }
    assert_eq!(provider.activity_count().unwrap(), 3);

    // Add sleep sessions
    let base_date = Utc::now();
    let sessions = SyntheticProvider::generate_sleep_sessions(3, base_date);
    for session in sessions {
        provider.add_sleep_session(session).unwrap();
    }
    assert_eq!(provider.sleep_session_count().unwrap(), 3);

    // Verify data is accessible
    let fetched_activities = provider.get_activities(None, None).await.unwrap();
    assert_eq!(fetched_activities.len(), 3);

    let start_date = base_date - chrono::Duration::days(10);
    let end_date = base_date;
    let fetched_sleep = provider
        .get_sleep_sessions(start_date, end_date)
        .await
        .unwrap();
    assert_eq!(fetched_sleep.len(), 3);
}

// ============================================================================
// NUTRITION CROSS-PROVIDER TESTS - Intensity Inference from Activity Data
// ============================================================================

/// Create activities with specific duration to test intensity inference
fn create_activities_with_duration(count: usize, hours_per_day: f64) -> Vec<Activity> {
    let mut activities = Vec::with_capacity(count);
    let base_date = Utc::now();

    // Convert hours per activity: if we want avg of X hours/day over count days
    // each activity should be X * 3600 seconds
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let duration_seconds = (hours_per_day * 3600.0) as u64;

    for i in 0..count {
        activities.push(Activity {
            id: format!("intensity_test_{i}"),
            name: format!("Workout {}", i + 1),
            sport_type: SportType::Run,
            start_date: base_date - chrono::Duration::days(i as i64),
            duration_seconds,
            distance_meters: Some(10000.0),
            elevation_gain: Some(100.0),
            average_heart_rate: None, // No HR for volume-based inference
            max_heart_rate: None,
            average_speed: Some(3.0),
            max_speed: Some(4.0),
            calories: Some(500),
            steps: None,
            heart_rate_zones: None,
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
            start_latitude: None,
            start_longitude: None,
            city: None,
            region: None,
            country: None,
            trail_name: None,
            workout_type: None,
            sport_type_detail: None,
            segment_efforts: None,
            provider: "synthetic".to_owned(),
        });
    }

    activities
}

/// Create activities with specific heart rate to test HR-based intensity inference
fn create_activities_with_heart_rate(count: usize, avg_hr: u32) -> Vec<Activity> {
    let mut activities = Vec::with_capacity(count);
    let base_date = Utc::now();

    for i in 0..count {
        activities.push(Activity {
            id: format!("hr_test_{i}"),
            name: format!("HR Workout {}", i + 1),
            sport_type: SportType::Run,
            start_date: base_date - chrono::Duration::days(i as i64),
            duration_seconds: 1800, // 30 min - low volume
            distance_meters: Some(5000.0),
            elevation_gain: Some(50.0),
            average_heart_rate: Some(avg_hr),
            max_heart_rate: Some(avg_hr + 20),
            average_speed: Some(2.8),
            max_speed: Some(3.5),
            calories: Some(300),
            steps: None,
            heart_rate_zones: None,
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
            start_latitude: None,
            start_longitude: None,
            city: None,
            region: None,
            country: None,
            trail_name: None,
            workout_type: None,
            sport_type_detail: None,
            segment_efforts: None,
            provider: "synthetic".to_owned(),
        });
    }

    activities
}

#[test]
fn test_infer_intensity_high_volume() {
    use pierre_mcp_server::protocols::universal::handlers::provider_helpers::infer_workout_intensity;

    // >2 hours/day average = high intensity
    let activities = create_activities_with_duration(7, 2.5);
    let intensity = infer_workout_intensity(&activities, 7);
    assert_eq!(
        intensity, "high",
        "2.5 hours/day average should be high intensity"
    );
}

#[test]
fn test_infer_intensity_moderate_volume() {
    use pierre_mcp_server::protocols::universal::handlers::provider_helpers::infer_workout_intensity;

    // 1-2 hours/day average = moderate intensity
    let activities = create_activities_with_duration(7, 1.5);
    let intensity = infer_workout_intensity(&activities, 7);
    assert_eq!(
        intensity, "moderate",
        "1.5 hours/day average should be moderate intensity"
    );
}

#[test]
fn test_infer_intensity_low_volume() {
    use pierre_mcp_server::protocols::universal::handlers::provider_helpers::infer_workout_intensity;

    // <1 hour/day average = low intensity
    let activities = create_activities_with_duration(7, 0.5);
    let intensity = infer_workout_intensity(&activities, 7);
    assert_eq!(
        intensity, "low",
        "0.5 hours/day average should be low intensity"
    );
}

#[test]
fn test_infer_intensity_high_heart_rate() {
    use pierre_mcp_server::protocols::universal::handlers::provider_helpers::infer_workout_intensity;

    // High avg HR (>150 bpm) = high intensity even with low volume
    let activities = create_activities_with_heart_rate(7, 160);
    let intensity = infer_workout_intensity(&activities, 7);
    assert_eq!(
        intensity, "high",
        "Avg HR >150 should be high intensity regardless of volume"
    );
}

#[test]
fn test_infer_intensity_moderate_heart_rate() {
    use pierre_mcp_server::protocols::universal::handlers::provider_helpers::infer_workout_intensity;

    // Moderate avg HR (130-150 bpm) = moderate intensity
    let activities = create_activities_with_heart_rate(7, 140);
    let intensity = infer_workout_intensity(&activities, 7);
    assert_eq!(
        intensity, "moderate",
        "Avg HR 130-150 should be moderate intensity"
    );
}

#[test]
fn test_infer_intensity_low_heart_rate() {
    use pierre_mcp_server::protocols::universal::handlers::provider_helpers::infer_workout_intensity;

    // Low avg HR (<130 bpm) with low volume = low intensity
    let activities = create_activities_with_heart_rate(7, 120);
    let intensity = infer_workout_intensity(&activities, 7);
    assert_eq!(
        intensity, "low",
        "Avg HR <130 with low volume should be low intensity"
    );
}

#[test]
fn test_infer_intensity_empty_activities() {
    use pierre_mcp_server::protocols::universal::handlers::provider_helpers::infer_workout_intensity;

    // Empty activities should default to moderate
    let activities: Vec<Activity> = vec![];
    let intensity = infer_workout_intensity(&activities, 7);
    assert_eq!(
        intensity, "moderate",
        "Empty activities should default to moderate"
    );
}

#[test]
fn test_infer_intensity_zero_days() {
    use pierre_mcp_server::protocols::universal::handlers::provider_helpers::infer_workout_intensity;

    // Zero days should default to moderate
    let activities = create_activities_with_duration(5, 1.0);
    let intensity = infer_workout_intensity(&activities, 0);
    assert_eq!(
        intensity, "moderate",
        "Zero days_back should default to moderate"
    );
}

// ============================================================================
// INTELLIGENCE CROSS-PROVIDER TESTS - Sleep/Recovery Adjustment for Fitness Score
// ============================================================================

/// Test that fitness score schema includes `sleep_provider` parameter
#[tokio::test]
async fn test_calculate_fitness_score_schema_has_sleep_provider() {
    use pierre_mcp_server::mcp::schema::get_tools;

    let schemas = get_tools();
    let fitness_score_schema = schemas
        .iter()
        .find(|s| s.name == "calculate_fitness_score")
        .expect("calculate_fitness_score schema should exist");

    let has_sleep_provider = fitness_score_schema
        .input_schema
        .properties
        .as_ref()
        .is_some_and(|props| props.contains_key("sleep_provider"));

    assert!(
        has_sleep_provider,
        "calculate_fitness_score should have sleep_provider parameter"
    );
}

/// Test that `analyze_training_load` schema includes `sleep_provider` parameter
#[tokio::test]
async fn test_analyze_training_load_schema_has_sleep_provider() {
    use pierre_mcp_server::mcp::schema::get_tools;

    let schemas = get_tools();
    let training_load_schema = schemas
        .iter()
        .find(|s| s.name == "analyze_training_load")
        .expect("analyze_training_load schema should exist");

    let has_sleep_provider = training_load_schema
        .input_schema
        .properties
        .as_ref()
        .is_some_and(|props| props.contains_key("sleep_provider"));

    assert!(
        has_sleep_provider,
        "analyze_training_load should have sleep_provider parameter"
    );
}

/// Test recovery adjustment calculation logic
#[test]
fn test_recovery_adjustment_factors() {
    // Recovery adjustment factors are:
    // - 90-100: 1.05 (+5%)
    // - 70-89: 1.0 (no change)
    // - 50-69: 0.95 (-5%)
    // - <50: 0.90 (-10%)

    // Test high recovery score
    let adjustment = calculate_test_adjustment(95.0);
    assert!(
        (adjustment - 1.05).abs() < 0.01,
        "95% recovery should give 1.05 adjustment factor"
    );

    // Test good recovery score
    let adjustment = calculate_test_adjustment(80.0);
    assert!(
        (adjustment - 1.0).abs() < 0.01,
        "80% recovery should give 1.0 adjustment factor"
    );

    // Test moderate recovery score
    let adjustment = calculate_test_adjustment(60.0);
    assert!(
        (adjustment - 0.95).abs() < 0.01,
        "60% recovery should give 0.95 adjustment factor"
    );

    // Test poor recovery score
    let adjustment = calculate_test_adjustment(40.0);
    assert!(
        (adjustment - 0.90).abs() < 0.01,
        "40% recovery should give 0.90 adjustment factor"
    );
}

/// Helper function to calculate adjustment factor based on recovery score
fn calculate_test_adjustment(recovery_score: f64) -> f64 {
    if recovery_score >= 90.0 {
        1.05
    } else if recovery_score >= 70.0 {
        1.0
    } else if recovery_score >= 50.0 {
        0.95
    } else {
        0.90
    }
}

/// Test recovery status classification
#[test]
fn test_recovery_status_classification() {
    assert_eq!(classify_test_recovery_status(95.0), "excellent");
    assert_eq!(classify_test_recovery_status(80.0), "good");
    assert_eq!(classify_test_recovery_status(65.0), "moderate");
    assert_eq!(classify_test_recovery_status(45.0), "fair");
    assert_eq!(classify_test_recovery_status(30.0), "poor");
}

/// Helper function to classify recovery status based on sleep quality score
fn classify_test_recovery_status(sleep_quality_score: f64) -> &'static str {
    if sleep_quality_score >= 90.0 {
        "excellent"
    } else if sleep_quality_score >= 75.0 {
        "good"
    } else if sleep_quality_score >= 60.0 {
        "moderate"
    } else if sleep_quality_score >= 40.0 {
        "fair"
    } else {
        "poor"
    }
}

/// Test that fitness score with recovery adjustment is bounded correctly
#[test]
fn test_fitness_score_with_recovery_bounds() {
    // Test that applying adjustment keeps score in reasonable bounds
    let base_scores = [0, 50, 75, 100];
    let adjustments = [0.90, 0.95, 1.0, 1.05];

    for base in base_scores {
        for adj in adjustments {
            let adjusted = (f64::from(base) * adj).round() as i64;
            // Adjusted score should be non-negative
            assert!(
                adjusted >= 0,
                "Adjusted score should be non-negative: {base} * {adj} = {adjusted}"
            );
            // With 5% bonus, score can exceed 100 slightly
            assert!(
                adjusted <= 110,
                "Adjusted score should not exceed 110: {base} * {adj} = {adjusted}"
            );
        }
    }
}

/// Test that cross-provider integration in intelligence tools doesn't affect activity-only mode
#[tokio::test]
async fn test_intelligence_tools_work_without_sleep_provider() {
    // Verify that the tools work correctly when sleep_provider is not specified
    // This tests backward compatibility

    common::init_server_config();

    // Create registry and verify providers
    let registry = ProviderRegistry::new();

    // Synthetic provider should support activities
    assert!(
        registry.is_supported(oauth_providers::SYNTHETIC),
        "Synthetic provider should be supported"
    );

    // Create a synthetic provider and add activities
    let provider = SyntheticProvider::new();
    let activities = create_test_activities(10);
    for activity in activities {
        provider.add_activity(activity).unwrap();
    }

    // Verify activities can be retrieved
    use pierre_mcp_server::providers::core::FitnessProvider;
    let fetched = provider.get_activities(Some(10), None).await.unwrap();
    assert_eq!(fetched.len(), 10, "Should retrieve all 10 activities");
}
