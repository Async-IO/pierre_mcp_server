// ABOUTME: Integration tests for Terra provider (150+ wearables unified API)
// ABOUTME: Tests cover converters, cache, webhook handling, and provider functionality
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Terra Provider Integration Tests
//!
//! Tests for the Terra provider module including:
//! - Model converters (Terra JSON to Pierre models)
//! - Data cache operations
//! - Webhook signature validation and processing
//! - Provider trait implementation

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
#![allow(missing_docs)]

use chrono::{Duration, Utc};
use pierre_mcp_server::models::{
    Activity, HealthMetrics, MealType, RecoveryMetrics, SleepSession, SportType,
};
use pierre_mcp_server::pagination::{PaginationDirection, PaginationParams};
use pierre_mcp_server::providers::core::FitnessProvider;
use pierre_mcp_server::providers::spi::ProviderDescriptor;
use pierre_mcp_server::providers::terra::models::{
    TerraActivity, TerraCaloriesData, TerraDistanceData, TerraMetadata, TerraUser,
};
use pierre_mcp_server::providers::terra::webhook::{
    SignatureValidation, WebhookResult, WebhookSignatureValidator,
};
use pierre_mcp_server::providers::terra::{
    TerraApiClient, TerraApiConfig, TerraConverters, TerraDataCache, TerraDescriptor,
    TerraProvider, TerraWebhookHandler,
};
use ring::hmac;
use std::sync::Arc;

// ============================================================================
// Helper Functions
// ============================================================================

fn make_test_user() -> TerraUser {
    TerraUser {
        user_id: "test_user_123".to_owned(),
        provider: Some("GARMIN".to_owned()),
        last_webhook_update: None,
        reference_id: Some("ref_456".to_owned()),
    }
}

fn make_test_activity(id: &str, hours_ago: i64) -> Activity {
    Activity {
        id: id.to_owned(),
        name: format!("Test Activity {id}"),
        sport_type: SportType::Run,
        start_date: Utc::now() - Duration::hours(hours_ago),
        duration_seconds: 3600,
        provider: "terra:garmin".to_owned(),
        ..Default::default()
    }
}

// ============================================================================
// Converter Tests
// ============================================================================

#[test]
fn test_activity_conversion() {
    let terra_activity = TerraActivity {
        metadata: Some(TerraMetadata {
            start_time: Some(Utc::now()),
            end_time: Some(Utc::now() + Duration::hours(1)),
            activity_type: Some(1), // Running
            name: Some("Morning Run".to_owned()),
            summary_id: Some("act_123".to_owned()),
            city: Some("Montreal".to_owned()),
            country: Some("Canada".to_owned()),
            upload_type: None,
        }),
        distance_data: Some(TerraDistanceData {
            distance_meters: Some(5000.0),
            steps: Some(5500),
            elevation_gain_metres: Some(50.0),
            ..Default::default()
        }),
        calories_data: Some(TerraCaloriesData {
            total_burned_calories: Some(400.0),
            ..Default::default()
        }),
        ..Default::default()
    };

    let user = make_test_user();
    let activity = TerraConverters::activity_from_terra(&terra_activity, &user);

    assert_eq!(activity.id, "act_123");
    assert_eq!(activity.name, "Morning Run");
    assert_eq!(activity.sport_type, SportType::Run);
    assert_eq!(activity.distance_meters, Some(5000.0));
    assert_eq!(activity.elevation_gain, Some(50.0));
    assert_eq!(activity.calories, Some(400));
    assert_eq!(activity.steps, Some(5500));
    assert_eq!(activity.city, Some("Montreal".to_owned()));
    assert_eq!(activity.provider, "terra:garmin");
}

#[test]
fn test_sport_type_mapping() {
    assert_eq!(TerraConverters::map_terra_activity_type(1), SportType::Run);
    assert_eq!(TerraConverters::map_terra_activity_type(5), SportType::Ride);
    assert_eq!(
        TerraConverters::map_terra_activity_type(10),
        SportType::Swim
    );
    assert_eq!(
        TerraConverters::map_terra_activity_type(14),
        SportType::Hike
    );
    assert_eq!(
        TerraConverters::map_terra_activity_type(30),
        SportType::StrengthTraining
    );
    assert_eq!(
        TerraConverters::map_terra_activity_type(999),
        SportType::Workout
    );
}

// Sleep stage mapping is tested implicitly through sleep_from_terra tests
// (map_terra_sleep_stage is private)

#[test]
fn test_meal_type_parsing() {
    assert_eq!(MealType::from_str_lossy("breakfast"), MealType::Breakfast);
    assert_eq!(MealType::from_str_lossy("LUNCH"), MealType::Lunch);
    assert_eq!(MealType::from_str_lossy("Dinner"), MealType::Dinner);
    assert_eq!(MealType::from_str_lossy("snack"), MealType::Snack);
    assert_eq!(MealType::from_str_lossy("unknown"), MealType::Other);
}

// ============================================================================
// Cache Tests
// ============================================================================

#[tokio::test]
async fn test_store_and_retrieve_activities() {
    let cache = TerraDataCache::new_in_memory();
    let user_id = "test_user_123";

    let activity1 = make_test_activity("act1", 2);
    let activity2 = make_test_activity("act2", 1);

    cache.store_activity(user_id, activity1).await;
    cache.store_activity(user_id, activity2).await;

    let activities = cache.get_activities(user_id, None, None).await;
    assert_eq!(activities.len(), 2);
    // Should be sorted by start_date descending (newest first)
    assert_eq!(activities[0].id, "act2");
    assert_eq!(activities[1].id, "act1");
}

#[tokio::test]
async fn test_activity_pagination() {
    let cache = TerraDataCache::new_in_memory();
    let user_id = "test_user";

    for i in 0..10 {
        cache
            .store_activity(user_id, make_test_activity(&format!("act{i}"), i))
            .await;
    }

    let page1 = cache.get_activities(user_id, Some(3), Some(0)).await;
    assert_eq!(page1.len(), 3);

    let page2 = cache.get_activities(user_id, Some(3), Some(3)).await;
    assert_eq!(page2.len(), 3);
}

#[tokio::test]
async fn test_duplicate_prevention() {
    let cache = TerraDataCache::new_in_memory();
    let user_id = "test_user";

    let activity = make_test_activity("act1", 1);
    cache.store_activity(user_id, activity.clone()).await;
    cache.store_activity(user_id, activity).await;

    let activities = cache.get_activities(user_id, None, None).await;
    assert_eq!(activities.len(), 1);
}

#[tokio::test]
async fn test_user_mapping() {
    let cache = TerraDataCache::new_in_memory();

    cache.register_user_mapping("ref123", "terra_abc").await;

    let terra_id = cache.get_terra_user_id("ref123").await;
    assert_eq!(terra_id, Some("terra_abc".to_owned()));

    let missing = cache.get_terra_user_id("unknown").await;
    assert!(missing.is_none());
}

#[tokio::test]
async fn test_cache_stats() {
    let cache = TerraDataCache::new_in_memory();

    cache
        .store_activity("user1", make_test_activity("a1", 1))
        .await;
    cache
        .store_activity("user1", make_test_activity("a2", 2))
        .await;
    cache
        .store_activity("user2", make_test_activity("a3", 3))
        .await;

    let stats = cache.get_stats().await;
    assert_eq!(stats.user_count, 2);
    assert_eq!(stats.total_activities, 3);
}

// ============================================================================
// Provider Tests
// ============================================================================

#[tokio::test]
async fn test_provider_name() {
    let cache = Arc::new(TerraDataCache::new_in_memory());
    let provider = TerraProvider::new(cache);
    assert_eq!(provider.name(), "terra");
}

#[tokio::test]
async fn test_authentication_flow() {
    let cache = Arc::new(TerraDataCache::new_in_memory());
    let provider = TerraProvider::new(cache);

    assert!(!provider.is_authenticated().await);

    provider.set_terra_user_id("test_user_123").await;
    assert!(provider.is_authenticated().await);
}

#[tokio::test]
async fn test_descriptor() {
    let descriptor = TerraDescriptor;
    assert_eq!(descriptor.name(), "terra");
    assert_eq!(descriptor.display_name(), "Terra (150+ Wearables)");
    assert!(descriptor.capabilities().supports_sleep());
    assert!(descriptor.capabilities().supports_recovery());
    assert!(descriptor.capabilities().supports_health());
    assert!(descriptor.capabilities().supports_activities());
}

#[tokio::test]
async fn test_get_activities_empty() {
    let cache = Arc::new(TerraDataCache::new_in_memory());
    let provider = TerraProvider::new(cache);
    provider.set_terra_user_id("test_user").await;

    let activities = provider
        .get_activities(None, None)
        .await
        .unwrap_or_default();
    assert!(activities.is_empty());
}

#[tokio::test]
async fn test_stats_calculation() {
    let cache = Arc::new(TerraDataCache::new_in_memory());
    let provider = TerraProvider::new(Arc::clone(&cache));
    provider.set_terra_user_id("test_user").await;

    // Add test activities to cache
    let activity = Activity {
        id: "act1".to_owned(),
        name: "Test Run".to_owned(),
        sport_type: SportType::Run,
        start_date: Utc::now(),
        duration_seconds: 3600,
        distance_meters: Some(5000.0),
        elevation_gain: Some(100.0),
        provider: "terra:garmin".to_owned(),
        ..Default::default()
    };
    cache.store_activity("test_user", activity).await;

    let stats = provider.get_stats().await;
    assert!(stats.is_ok(), "get_stats should succeed");
    // Safe: Test assertion - expect stats to be available
    let stats = stats.unwrap();
    assert_eq!(stats.total_activities, 1);
    assert!((stats.total_distance - 5000.0).abs() < 0.01);
}

// ============================================================================
// FitnessProvider Integration Tests
// ============================================================================

#[tokio::test]
async fn test_get_sleep_sessions() {
    let cache = Arc::new(TerraDataCache::new_in_memory());
    let provider = TerraProvider::new(Arc::clone(&cache));
    let user_id = "test_user";
    provider.set_terra_user_id(user_id).await;

    // Create test sleep session
    let sleep_session = SleepSession {
        id: "sleep_001".to_owned(),
        start_time: Utc::now() - Duration::hours(8),
        end_time: Utc::now(),
        time_in_bed: 480,
        total_sleep_time: 450,
        sleep_efficiency: 93.75,
        sleep_score: Some(85.0),
        stages: vec![],
        hrv_during_sleep: Some(45.0),
        respiratory_rate: Some(14.0),
        temperature_variation: None,
        wake_count: Some(2),
        sleep_onset_latency: Some(10),
        provider: "terra:garmin".to_owned(),
    };
    cache.store_sleep_session(user_id, sleep_session).await;

    // Test FitnessProvider::get_sleep_sessions
    let start = Utc::now() - Duration::days(1);
    let end = Utc::now() + Duration::hours(1);
    let sessions = provider.get_sleep_sessions(start, end).await;
    assert!(sessions.is_ok(), "get_sleep_sessions should succeed");
    // Safe: Test assertion - expect sessions to be available
    let sessions = sessions.unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, "sleep_001");
    assert_eq!(sessions[0].total_sleep_time, 450);
}

#[tokio::test]
async fn test_get_recovery_metrics() {
    let cache = Arc::new(TerraDataCache::new_in_memory());
    let provider = TerraProvider::new(Arc::clone(&cache));
    let user_id = "test_user";
    provider.set_terra_user_id(user_id).await;

    // Create test recovery metrics
    let recovery = RecoveryMetrics {
        date: Utc::now(),
        recovery_score: Some(78.0),
        readiness_score: Some(82.0),
        hrv_status: Some("normal".to_owned()),
        sleep_score: Some(85.0),
        stress_level: Some(25.0),
        training_load: Some(150.0),
        resting_heart_rate: Some(52),
        body_temperature: None,
        resting_respiratory_rate: Some(14.0),
        provider: "terra:garmin".to_owned(),
    };
    cache.store_recovery_metrics(user_id, recovery).await;

    // Test FitnessProvider::get_recovery_metrics
    let start = Utc::now() - Duration::days(1);
    let end = Utc::now() + Duration::hours(1);
    let metrics = provider.get_recovery_metrics(start, end).await;
    assert!(metrics.is_ok(), "get_recovery_metrics should succeed");
    // Safe: Test assertion - expect metrics to be available
    let metrics = metrics.unwrap();
    assert_eq!(metrics.len(), 1);
    assert_eq!(metrics[0].recovery_score, Some(78.0));
    assert_eq!(metrics[0].resting_heart_rate, Some(52));
}

#[tokio::test]
async fn test_get_health_metrics() {
    let cache = Arc::new(TerraDataCache::new_in_memory());
    let provider = TerraProvider::new(Arc::clone(&cache));
    let user_id = "test_user";
    provider.set_terra_user_id(user_id).await;

    // Create test health metrics
    let health = HealthMetrics {
        date: Utc::now(),
        weight: Some(75.5),
        body_fat_percentage: Some(15.2),
        muscle_mass: Some(60.0),
        bone_mass: Some(3.2),
        body_water_percentage: Some(55.0),
        bmr: Some(1750),
        blood_pressure: Some((120, 80)),
        blood_glucose: None,
        vo2_max: Some(48.0),
        provider: "terra:garmin".to_owned(),
    };
    cache.store_health_metrics(user_id, health).await;

    // Test FitnessProvider::get_health_metrics
    let start = Utc::now() - Duration::days(1);
    let end = Utc::now() + Duration::hours(1);
    let metrics = provider.get_health_metrics(start, end).await;
    assert!(metrics.is_ok(), "get_health_metrics should succeed");
    // Safe: Test assertion - expect metrics to be available
    let metrics = metrics.unwrap();
    assert_eq!(metrics.len(), 1);
    assert_eq!(metrics[0].weight, Some(75.5));
    assert_eq!(metrics[0].body_fat_percentage, Some(15.2));
}

#[tokio::test]
async fn test_get_activities_cursor_pagination() {
    let cache = Arc::new(TerraDataCache::new_in_memory());
    let provider = TerraProvider::new(Arc::clone(&cache));
    let user_id = "test_user";
    provider.set_terra_user_id(user_id).await;

    // Add 10 activities with different timestamps
    for i in 0..10 {
        let activity = Activity {
            id: format!("act_{i:02}"),
            name: format!("Activity {i}"),
            sport_type: SportType::Run,
            start_date: Utc::now() - Duration::hours(i),
            duration_seconds: 3600,
            distance_meters: Some((i as f64).mul_add(100.0, 5000.0)),
            provider: "terra:garmin".to_owned(),
            ..Default::default()
        };
        cache.store_activity(user_id, activity).await;
    }

    // Test first page (no cursor)
    let params = PaginationParams {
        limit: 3,
        cursor: None,
        direction: PaginationDirection::Forward,
    };
    let page1 = provider.get_activities_cursor(&params).await;
    assert!(page1.is_ok(), "get_activities_cursor should succeed");
    // Safe: Test assertion - expect page1 to be available
    let page1 = page1.unwrap();
    assert_eq!(page1.items.len(), 3);
    assert!(page1.has_more);
    assert!(page1.next_cursor.is_some());
    // Should be sorted newest first
    assert_eq!(page1.items[0].id, "act_00");
    assert_eq!(page1.items[1].id, "act_01");
    assert_eq!(page1.items[2].id, "act_02");

    // Test second page (with cursor)
    let params2 = PaginationParams {
        limit: 3,
        cursor: page1.next_cursor,
        direction: PaginationDirection::Forward,
    };
    let page2 = provider.get_activities_cursor(&params2).await;
    assert!(page2.is_ok(), "get_activities_cursor page2 should succeed");
    // Safe: Test assertion - expect page2 to be available
    let page2 = page2.unwrap();
    assert_eq!(page2.items.len(), 3);
    assert!(page2.has_more);
    assert_eq!(page2.items[0].id, "act_03");
    assert_eq!(page2.items[1].id, "act_04");
    assert_eq!(page2.items[2].id, "act_05");
}

#[tokio::test]
async fn test_get_latest_sleep_session() {
    let cache = Arc::new(TerraDataCache::new_in_memory());
    let provider = TerraProvider::new(Arc::clone(&cache));
    let user_id = "test_user";
    provider.set_terra_user_id(user_id).await;

    // Create two sleep sessions at different times
    let older_sleep = SleepSession {
        id: "sleep_old".to_owned(),
        start_time: Utc::now() - Duration::hours(32),
        end_time: Utc::now() - Duration::hours(24),
        time_in_bed: 480,
        total_sleep_time: 420,
        sleep_efficiency: 87.5,
        sleep_score: Some(75.0),
        stages: vec![],
        hrv_during_sleep: None,
        respiratory_rate: None,
        temperature_variation: None,
        wake_count: None,
        sleep_onset_latency: None,
        provider: "terra:garmin".to_owned(),
    };

    let newer_sleep = SleepSession {
        id: "sleep_new".to_owned(),
        start_time: Utc::now() - Duration::hours(8),
        end_time: Utc::now(),
        time_in_bed: 480,
        total_sleep_time: 460,
        sleep_efficiency: 95.8,
        sleep_score: Some(92.0),
        stages: vec![],
        hrv_during_sleep: Some(50.0),
        respiratory_rate: Some(13.0),
        temperature_variation: None,
        wake_count: Some(1),
        sleep_onset_latency: Some(5),
        provider: "terra:garmin".to_owned(),
    };

    cache.store_sleep_session(user_id, older_sleep).await;
    cache.store_sleep_session(user_id, newer_sleep).await;

    // Test FitnessProvider::get_latest_sleep_session
    let latest = provider.get_latest_sleep_session().await;
    assert!(latest.is_ok(), "get_latest_sleep_session should succeed");
    // Safe: Test assertion - expect latest to be available
    let latest = latest.unwrap();
    assert_eq!(latest.id, "sleep_new");
    assert_eq!(latest.sleep_score, Some(92.0));
}

// ============================================================================
// Webhook Tests
// ============================================================================

#[tokio::test]
async fn test_webhook_handler_auth_event() {
    let cache = Arc::new(TerraDataCache::new_in_memory());
    let handler = TerraWebhookHandler::new(cache);

    let payload = serde_json::json!({
        "type": "auth",
        "status": "success",
        "user": {
            "user_id": "test_user_123",
            "provider": "GARMIN",
            "reference_id": "my_user_456"
        }
    });

    let result = handler.process(payload.to_string().as_bytes()).await;

    assert!(
        matches!(
            &result,
            WebhookResult::AuthEvent {
                event_type,
                status,
                user_id,
                reference_id,
            } if event_type == "auth"
                && status == "success"
                && *user_id == Some("test_user_123".to_owned())
                && *reference_id == Some("my_user_456".to_owned())
        ),
        "Expected AuthEvent with correct values, got: {result:?}"
    );
}

#[tokio::test]
async fn test_webhook_handler_activity_event() {
    let cache = Arc::new(TerraDataCache::new_in_memory());
    let handler = TerraWebhookHandler::new(Arc::clone(&cache));

    let payload = serde_json::json!({
        "type": "activity",
        "user": {
            "user_id": "test_user_123",
            "provider": "GARMIN"
        },
        "data": [{
            "metadata": {
                "summary_id": "act_001",
                "name": "Morning Run",
                "start_time": "2024-01-15T08:00:00Z",
                "end_time": "2024-01-15T09:00:00Z",
                "type": 1
            },
            "distance_data": {
                "distance_meters": 5000.0
            },
            "calories_data": {
                "total_burned_calories": 400.0
            }
        }]
    });

    let result = handler.process(payload.to_string().as_bytes()).await;

    assert!(
        matches!(
            &result,
            WebhookResult::Success {
                event_type,
                items_processed,
                user_id,
            } if event_type == "activity"
                && *items_processed == 1
                && user_id == "test_user_123"
        ),
        "Expected Success result with correct values, got: {result:?}"
    );

    // Verify activity was cached
    let activities = cache.get_activities("test_user_123", None, None).await;
    assert_eq!(activities.len(), 1);
    assert_eq!(activities[0].name, "Morning Run");
}

#[tokio::test]
async fn test_webhook_handler_unhandled_event() {
    let cache = Arc::new(TerraDataCache::new_in_memory());
    let handler = TerraWebhookHandler::new(cache);

    let payload = serde_json::json!({
        "type": "unknown_event",
        "user": {
            "user_id": "test_user_123"
        }
    });

    let result = handler.process(payload.to_string().as_bytes()).await;

    assert!(
        matches!(
            &result,
            WebhookResult::Unhandled { event_type } if event_type == "unknown_event"
        ),
        "Expected Unhandled result with unknown_event, got: {result:?}"
    );
}

#[test]
fn test_signature_validation_valid() {
    let secret = "test_secret";
    let body = b"test body";

    // Generate valid signature using ring::hmac
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
    let tag = hmac::sign(&key, body);
    let signature = hex::encode(tag.as_ref());

    let validator = WebhookSignatureValidator::new(secret.to_owned());
    let header = format!("t=1234567890,v1={signature}");
    let result = validator.validate(Some(&header), body);
    assert_eq!(result, SignatureValidation::Valid);
}

#[test]
fn test_signature_validation_missing() {
    let validator = WebhookSignatureValidator::new("test_secret".to_owned());
    let result = validator.validate(None, b"test body");
    assert_eq!(result, SignatureValidation::Missing);
}

#[test]
fn test_signature_validation_invalid_format() {
    let validator = WebhookSignatureValidator::new("test_secret".to_owned());
    let result = validator.validate(Some("invalid_header"), b"test body");
    assert_eq!(result, SignatureValidation::Invalid);
}

#[test]
fn test_signature_validation_wrong_signature() {
    let validator = WebhookSignatureValidator::new("test_secret".to_owned());
    let header = "t=1234567890,v1=wrong_signature";
    let result = validator.validate(Some(header), b"test body");
    assert_eq!(result, SignatureValidation::Invalid);
}

// ============================================================================
// API Client Tests
// ============================================================================

#[test]
fn test_api_config_default() {
    use std::time::Duration as StdDuration;

    let config = TerraApiConfig::default();
    assert_eq!(config.base_url, "https://api.tryterra.co/v2");
    assert_eq!(config.timeout, StdDuration::from_secs(30));
}

#[test]
fn test_api_client_creation() {
    use std::mem::size_of_val;

    let config = TerraApiConfig {
        api_key: "test_key".to_owned(),
        dev_id: "test_dev".to_owned(),
        ..Default::default()
    };
    // Verify client creation succeeds
    let client = TerraApiClient::new(config);
    // Assert client has non-zero size (struct was instantiated)
    assert!(size_of_val(&client) > 0);
}
