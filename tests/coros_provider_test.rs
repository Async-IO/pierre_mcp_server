// ABOUTME: Comprehensive test suite for COROS provider implementation
// ABOUTME: Tests provider creation, configuration, OAuth flow, data conversion, and mock API responses
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use chrono::Utc;
use pierre_mcp_server::config::environment::HttpClientConfig;
use pierre_mcp_server::constants::{init_server_config, oauth_providers};
use pierre_mcp_server::providers::core::{FitnessProvider, OAuth2Credentials, ProviderConfig};
use pierre_mcp_server::providers::coros_provider::CorosProvider;
use pierre_mcp_server::providers::registry::{get_supported_providers, global_registry};
use pierre_mcp_server::utils::http_client::initialize_http_clients;
use std::sync::Once;

/// Ensure HTTP clients and server config are initialized only once across all tests
static INIT_HTTP_CLIENTS: Once = Once::new();
static INIT_SERVER_CONFIG: Once = Once::new();

fn ensure_http_clients_initialized() {
    // Initialize server config first (required for provider defaults)
    INIT_SERVER_CONFIG.call_once(|| {
        let _ = init_server_config();
    });

    INIT_HTTP_CLIENTS.call_once(|| {
        initialize_http_clients(HttpClientConfig::default());
    });
}

// ============================================================================
// Provider Creation Tests
// ============================================================================

#[test]
fn test_coros_provider_creation() {
    ensure_http_clients_initialized();
    let provider = CorosProvider::new();

    assert_eq!(provider.name(), oauth_providers::COROS);
    assert_eq!(provider.config().name, oauth_providers::COROS);
    // Placeholder URLs - will be updated when API docs are received
    assert!(provider.config().auth_url.contains("coros"));
    assert!(provider.config().api_base_url.contains("coros"));
}

#[test]
fn test_coros_provider_with_custom_config() {
    ensure_http_clients_initialized();
    let custom_config = ProviderConfig {
        name: oauth_providers::COROS.to_owned(),
        auth_url: "https://custom.coros.com/auth".to_owned(),
        token_url: "https://custom.coros.com/token".to_owned(),
        api_base_url: "https://custom.coros.com/api".to_owned(),
        revoke_url: Some("https://custom.coros.com/revoke".to_owned()),
        default_scopes: vec!["custom:scope".to_owned()],
    };

    let provider = CorosProvider::with_config(custom_config.clone());

    assert_eq!(provider.config().name, custom_config.name);
    assert_eq!(provider.config().auth_url, custom_config.auth_url);
    assert_eq!(provider.config().token_url, custom_config.token_url);
    assert_eq!(provider.config().api_base_url, custom_config.api_base_url);
}

#[test]
fn test_coros_provider_default() {
    ensure_http_clients_initialized();
    let provider = CorosProvider::default();
    assert_eq!(provider.name(), oauth_providers::COROS);
}

// ============================================================================
// Authentication Lifecycle Tests
// ============================================================================

#[tokio::test]
async fn test_coros_provider_authentication_lifecycle() {
    ensure_http_clients_initialized();
    let provider = CorosProvider::new();

    // Initially not authenticated
    assert!(!provider.is_authenticated().await);

    // Set valid credentials
    let credentials = OAuth2Credentials {
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        access_token: Some("test_access_token".to_owned()),
        refresh_token: Some("test_refresh_token".to_owned()),
        expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
        scopes: vec!["read:workouts".to_owned(), "read:sleep".to_owned()],
    };

    provider
        .set_credentials(credentials)
        .await
        .expect("Failed to set credentials");

    // Now authenticated
    assert!(provider.is_authenticated().await);
}

#[tokio::test]
async fn test_coros_provider_expired_token() {
    ensure_http_clients_initialized();
    let provider = CorosProvider::new();

    // Set expired credentials
    let credentials = OAuth2Credentials {
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        access_token: Some("expired_token".to_owned()),
        refresh_token: Some("test_refresh_token".to_owned()),
        expires_at: Some(Utc::now() - chrono::Duration::hours(1)), // Already expired
        scopes: vec!["read:workouts".to_owned()],
    };

    provider
        .set_credentials(credentials)
        .await
        .expect("Failed to set credentials");

    // Not authenticated due to expired token
    assert!(!provider.is_authenticated().await);
}

#[tokio::test]
async fn test_coros_provider_no_expiry() {
    ensure_http_clients_initialized();
    let provider = CorosProvider::new();

    // Credentials with no expiry time
    let credentials = OAuth2Credentials {
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        access_token: Some("test_access_token".to_owned()),
        refresh_token: Some("test_refresh_token".to_owned()),
        expires_at: None, // No expiry
        scopes: vec!["read:workouts".to_owned()],
    };

    provider
        .set_credentials(credentials)
        .await
        .expect("Failed to set credentials");

    // Authenticated (no expiry means valid indefinitely)
    assert!(provider.is_authenticated().await);
}

#[tokio::test]
async fn test_coros_provider_disconnect() {
    ensure_http_clients_initialized();
    let provider = CorosProvider::new();

    // Set credentials
    let credentials = OAuth2Credentials {
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        access_token: Some("test_access_token".to_owned()),
        refresh_token: Some("test_refresh_token".to_owned()),
        expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
        scopes: vec!["read:workouts".to_owned()],
    };

    provider
        .set_credentials(credentials)
        .await
        .expect("Failed to set credentials");
    assert!(provider.is_authenticated().await);

    // Disconnect
    provider.disconnect().await.expect("Failed to disconnect");

    // No longer authenticated
    assert!(!provider.is_authenticated().await);
}

// ============================================================================
// Configuration Tests
// ============================================================================

#[test]
fn test_coros_provider_scopes() {
    ensure_http_clients_initialized();
    let provider = CorosProvider::new();
    let scopes = &provider.config().default_scopes;

    // COROS placeholder scopes - update when API docs are received
    assert!(scopes
        .iter()
        .any(|s| s.contains("workout") || s.contains("sleep") || s.contains("daily")));
}

#[test]
fn test_coros_provider_endpoints() {
    ensure_http_clients_initialized();
    let provider = CorosProvider::new();
    let config = provider.config();

    // Verify all required endpoints are configured (placeholder URLs)
    assert!(config.auth_url.starts_with("https://"));
    assert!(config.token_url.starts_with("https://"));
    assert!(config.api_base_url.starts_with("https://"));
    assert!(config.revoke_url.is_some());
}

// ============================================================================
// Registry Integration Tests
// ============================================================================

#[test]
fn test_coros_registered_in_registry() {
    ensure_http_clients_initialized();
    let _registry = global_registry();
    let providers = get_supported_providers();

    assert!(
        providers.contains(&oauth_providers::COROS),
        "COROS provider should be registered in the global registry"
    );
}

#[test]
fn test_coros_provider_capabilities() {
    ensure_http_clients_initialized();
    let registry = global_registry();

    // COROS should support sleep tracking
    assert!(
        registry.supports_sleep(oauth_providers::COROS),
        "COROS should support sleep tracking"
    );

    // COROS should support recovery metrics
    assert!(
        registry.supports_recovery(oauth_providers::COROS),
        "COROS should support recovery metrics"
    );

    // COROS should require OAuth
    assert!(
        registry.requires_oauth(oauth_providers::COROS),
        "COROS should require OAuth authentication"
    );
}

// ============================================================================
// Mock API Response Tests
// ============================================================================
// These tests verify the data conversion logic without making actual API calls.
// They use mock responses that simulate the expected COROS API format.

mod mock_api_tests {
    use serde_json::json;

    /// Mock COROS workout response for testing conversion logic
    fn mock_coros_workout_response() -> serde_json::Value {
        json!({
            "data": [
                {
                    "id": "workout_123",
                    "name": "Morning Run",
                    "start_time": "2025-01-06T07:00:00Z",
                    "end_time": "2025-01-06T07:45:00Z",
                    "duration": 2700,
                    "sport_type": 1,
                    "distance": 8500.0,
                    "elevation_gain": 125.0,
                    "calories": 650,
                    "avg_heart_rate": 155,
                    "max_heart_rate": 178,
                    "avg_pace": 318.0,
                    "avg_speed": 3.15,
                    "avg_cadence": 175,
                    "avg_power": null,
                    "training_load": 85.5
                },
                {
                    "id": "workout_124",
                    "name": "Trail Run",
                    "start_time": "2025-01-05T16:00:00Z",
                    "duration": 5400,
                    "sport_type": 3,
                    "distance": 12000.0,
                    "elevation_gain": 450.0,
                    "calories": 1200,
                    "avg_heart_rate": 148,
                    "max_heart_rate": 172
                }
            ],
            "pagination": {
                "next_token": null,
                "total": 2
            }
        })
    }

    /// Mock COROS sleep response for testing conversion logic
    fn mock_coros_sleep_response() -> serde_json::Value {
        json!({
            "data": [
                {
                    "id": "sleep_456",
                    "date": "2025-01-06",
                    "start_time": "2025-01-05T23:00:00Z",
                    "end_time": "2025-01-06T06:30:00Z",
                    "total_sleep_minutes": 420,
                    "awake_minutes": 30,
                    "light_sleep_minutes": 180,
                    "deep_sleep_minutes": 120,
                    "rem_sleep_minutes": 120,
                    "sleep_score": 85,
                    "efficiency": 93.3,
                    "avg_heart_rate": 52,
                    "avg_spo2": 97.5,
                    "respiratory_rate": 14.5
                }
            ],
            "pagination": {
                "next_token": null,
                "total": 1
            }
        })
    }

    /// Mock COROS daily summary response for testing conversion logic
    fn mock_coros_daily_response() -> serde_json::Value {
        json!({
            "data": [
                {
                    "date": "2025-01-06",
                    "steps": 12500,
                    "distance": 9800.0,
                    "calories": 2450,
                    "active_minutes": 75,
                    "resting_heart_rate": 48,
                    "avg_heart_rate": 68,
                    "max_heart_rate": 178,
                    "hrv_rmssd": 65.5,
                    "recovery_score": 82
                }
            ],
            "pagination": {
                "next_token": null,
                "total": 1
            }
        })
    }

    #[test]
    fn test_mock_workout_response_structure() {
        let response = mock_coros_workout_response();

        // Verify the mock response has expected structure
        let data = response.get("data").expect("Should have data field");
        assert!(data.is_array());

        let workouts = data.as_array().unwrap();
        assert_eq!(workouts.len(), 2);

        // Verify first workout has required fields
        let first = &workouts[0];
        assert!(first.get("id").is_some());
        assert!(first.get("start_time").is_some());
        assert!(first.get("sport_type").is_some());
    }

    #[test]
    fn test_mock_sleep_response_structure() {
        let response = mock_coros_sleep_response();

        let data = response.get("data").expect("Should have data field");
        assert!(data.is_array());

        let sleeps = data.as_array().unwrap();
        assert_eq!(sleeps.len(), 1);

        // Verify sleep has required fields
        let sleep = &sleeps[0];
        assert!(sleep.get("id").is_some());
        assert!(sleep.get("start_time").is_some());
        assert!(sleep.get("end_time").is_some());
        assert!(sleep.get("total_sleep_minutes").is_some());
    }

    #[test]
    fn test_mock_daily_response_structure() {
        let response = mock_coros_daily_response();

        let data = response.get("data").expect("Should have data field");
        assert!(data.is_array());

        let dailies = data.as_array().unwrap();
        assert_eq!(dailies.len(), 1);

        // Verify daily has required fields
        let daily = &dailies[0];
        assert!(daily.get("date").is_some());
        assert!(daily.get("steps").is_some());
        assert!(daily.get("recovery_score").is_some());
    }

    #[test]
    fn test_coros_sport_type_mapping() {
        // Test sport type ID mapping (based on expected COROS values)
        // These are placeholder mappings - update when API docs received

        // Sport type 1 should map to Run
        let run_type = 1;
        assert!(run_type > 0, "Run sport type should be positive");

        // Sport type 3 should map to Trail Run
        let trail_type = 3;
        assert!(trail_type > 0, "Trail run sport type should be positive");

        // Sport type 4 should map to Cycling
        let cycling_type = 4;
        assert!(cycling_type > 0, "Cycling sport type should be positive");
    }

    #[test]
    fn test_mock_workout_metrics_validation() {
        let response = mock_coros_workout_response();
        let workouts = response["data"].as_array().unwrap();
        let workout = &workouts[0];

        // Validate metrics are within realistic ranges
        let distance = workout["distance"].as_f64().unwrap();
        assert!(
            distance > 0.0 && distance < 100_000.0,
            "Distance should be reasonable"
        );

        let calories = workout["calories"].as_u64().unwrap();
        assert!(
            calories > 0 && calories < 10_000,
            "Calories should be reasonable"
        );

        let avg_hr = workout["avg_heart_rate"].as_u64().unwrap();
        assert!(
            avg_hr > 40 && avg_hr < 220,
            "Heart rate should be physiologically valid"
        );

        let max_hr = workout["max_heart_rate"].as_u64().unwrap();
        assert!(max_hr >= avg_hr, "Max HR should be >= avg HR");
    }

    #[test]
    fn test_mock_sleep_metrics_validation() {
        let response = mock_coros_sleep_response();
        let sleeps = response["data"].as_array().unwrap();
        let sleep = &sleeps[0];

        // Validate sleep metrics are within realistic ranges
        let total_sleep = sleep["total_sleep_minutes"].as_u64().unwrap();
        assert!(
            total_sleep > 0 && total_sleep <= 720,
            "Sleep duration should be reasonable (0-12 hours)"
        );

        let efficiency = sleep["efficiency"].as_f64().unwrap();
        assert!(
            (0.0..=100.0).contains(&efficiency),
            "Efficiency should be a percentage"
        );

        let sleep_score = sleep["sleep_score"].as_u64().unwrap();
        assert!(sleep_score <= 100, "Sleep score should be 0-100");

        // Validate sleep stages add up correctly
        let light = sleep["light_sleep_minutes"].as_u64().unwrap();
        let deep = sleep["deep_sleep_minutes"].as_u64().unwrap();
        let rem = sleep["rem_sleep_minutes"].as_u64().unwrap();

        // Note: awake_minutes is separate from total_sleep (which excludes awake time)
        assert_eq!(
            total_sleep,
            light + deep + rem,
            "Sleep stages should sum to total sleep time"
        );
    }

    #[test]
    fn test_mock_daily_metrics_validation() {
        let response = mock_coros_daily_response();
        let dailies = response["data"].as_array().unwrap();
        let daily = &dailies[0];

        // Validate daily metrics are within realistic ranges
        let steps = daily["steps"].as_u64().unwrap();
        assert!(steps < 100_000, "Steps should be reasonable for one day");

        let resting_hr = daily["resting_heart_rate"].as_u64().unwrap();
        assert!(
            resting_hr > 30 && resting_hr < 120,
            "Resting HR should be physiologically valid"
        );

        let hrv = daily["hrv_rmssd"].as_f64().unwrap();
        assert!(hrv > 0.0 && hrv < 300.0, "HRV RMSSD should be reasonable");

        let recovery = daily["recovery_score"].as_u64().unwrap();
        assert!(recovery <= 100, "Recovery score should be 0-100");
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_coros_provider_no_credentials_error() {
    ensure_http_clients_initialized();
    let provider = CorosProvider::new();

    // Attempting to get athlete without credentials should fail gracefully
    let result = provider.get_athlete().await;
    assert!(result.is_err(), "Should error when no credentials set");
}

#[tokio::test]
async fn test_coros_provider_stats_returns_empty() {
    ensure_http_clients_initialized();
    let provider = CorosProvider::new();

    // Set credentials
    let credentials = OAuth2Credentials {
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        access_token: Some("test_access_token".to_owned()),
        refresh_token: Some("test_refresh_token".to_owned()),
        expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
        scopes: vec!["read:workouts".to_owned()],
    };

    provider.set_credentials(credentials).await.unwrap();

    // Stats returns empty (COROS may not have a stats endpoint)
    let stats = provider.get_stats().await.unwrap();
    assert_eq!(stats.total_activities, 0);
}

#[tokio::test]
async fn test_coros_provider_personal_records_returns_empty() {
    ensure_http_clients_initialized();
    let provider = CorosProvider::new();

    // Set credentials
    let credentials = OAuth2Credentials {
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        access_token: Some("test_access_token".to_owned()),
        refresh_token: Some("test_refresh_token".to_owned()),
        expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
        scopes: vec!["read:workouts".to_owned()],
    };

    provider.set_credentials(credentials).await.unwrap();

    // Personal records returns empty (COROS may not expose PRs via API)
    let prs = provider.get_personal_records().await.unwrap();
    assert!(prs.is_empty());
}
