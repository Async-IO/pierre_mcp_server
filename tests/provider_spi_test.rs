// ABOUTME: Tests for Provider SPI (Service Provider Interface)
// ABOUTME: Verifies provider descriptors and capabilities work correctly
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used)]
#![allow(missing_docs)]

use pierre_mcp_server::providers::ProviderCapabilities;
#[cfg(any(
    feature = "provider-strava",
    feature = "provider-garmin",
    feature = "provider-synthetic"
))]
use pierre_mcp_server::providers::{ProviderBundle, ProviderDescriptor};

#[cfg(feature = "provider-strava")]
use pierre_mcp_server::providers::StravaDescriptor;

#[cfg(feature = "provider-garmin")]
use pierre_mcp_server::providers::GarminDescriptor;

#[cfg(feature = "provider-synthetic")]
use pierre_mcp_server::providers::SyntheticDescriptor;

#[test]
#[cfg(feature = "provider-strava")]
fn test_strava_descriptor() {
    let desc = StravaDescriptor;
    assert_eq!(desc.name(), "strava");
    assert_eq!(desc.display_name(), "Strava");
    assert!(desc.requires_oauth());
    assert!(!desc.supports_sleep());
    assert!(!desc.supports_recovery());

    let config = desc.to_config();
    assert_eq!(config.name, "strava");
    assert!(config.auth_url.contains("strava.com"));
}

#[test]
#[cfg(feature = "provider-garmin")]
fn test_garmin_descriptor() {
    let desc = GarminDescriptor;
    assert_eq!(desc.name(), "garmin");
    assert!(desc.requires_oauth());
    assert!(desc.supports_sleep());
    assert!(desc.supports_recovery());
    assert!(desc.supports_health());
}

#[test]
#[cfg(feature = "provider-synthetic")]
fn test_synthetic_descriptor() {
    let desc = SyntheticDescriptor;
    assert_eq!(desc.name(), "synthetic");
    assert!(!desc.requires_oauth());
    assert!(desc.supports_sleep()); // Synthetic supports all for testing
    assert!(desc.oauth_endpoints().is_none());
}

#[test]
fn test_provider_capabilities() {
    let activity = ProviderCapabilities::activity_only();
    assert!(activity.requires_oauth());
    assert!(activity.supports_activities());
    assert!(!activity.supports_sleep());

    let full = ProviderCapabilities::full_health();
    assert!(full.requires_oauth());
    assert!(full.supports_sleep());
    assert!(full.supports_recovery());

    let synthetic = ProviderCapabilities::synthetic();
    assert!(!synthetic.requires_oauth());
    assert!(synthetic.supports_activities());
}

// ============================================================================
// NEW TESTS: ProviderBundle and Factory Functions
// ============================================================================

#[test]
#[cfg(feature = "provider-strava")]
fn test_strava_oauth_params() {
    let desc = StravaDescriptor;
    let oauth_params = desc.oauth_params();

    assert!(oauth_params.is_some());
    if let Some(params) = oauth_params {
        assert_eq!(params.scope_separator, ",");
        assert!(params.use_pkce);
        assert_eq!(params.additional_auth_params.len(), 1);
        assert_eq!(params.additional_auth_params[0].0, "approval_prompt");
        assert_eq!(params.additional_auth_params[0].1, "force");
    }
}

#[test]
#[cfg(feature = "provider-garmin")]
fn test_garmin_oauth_params() {
    let desc = GarminDescriptor;
    let oauth_params = desc.oauth_params();

    assert!(oauth_params.is_some());
    if let Some(params) = oauth_params {
        assert_eq!(params.scope_separator, ",");
        assert!(!params.use_pkce); // Garmin uses OAuth 1.0a
        assert!(params.additional_auth_params.is_empty());
    }
}

#[test]
#[cfg(feature = "provider-synthetic")]
fn test_synthetic_no_oauth_params() {
    let desc = SyntheticDescriptor;
    assert!(desc.oauth_params().is_none());
    assert!(desc.oauth_endpoints().is_none());
}

#[test]
#[cfg(feature = "provider-strava")]
fn test_provider_bundle_creation() {
    use pierre_mcp_server::providers::core::{FitnessProvider, ProviderConfig};

    // Create a test factory function
    fn test_factory(_config: ProviderConfig) -> Box<dyn FitnessProvider> {
        use pierre_mcp_server::providers::synthetic_provider::SyntheticProvider;
        Box::new(SyntheticProvider::new())
    }

    let descriptor = Box::new(StravaDescriptor);
    let bundle = ProviderBundle::new(descriptor, test_factory);

    assert_eq!(bundle.name(), "strava");

    // Test that create_provider works
    let provider = bundle.create_provider();
    assert_eq!(provider.name(), "synthetic"); // Using synthetic factory for test
}

#[test]
#[cfg(feature = "provider-strava")]
fn test_provider_descriptor_to_config() {
    let desc = StravaDescriptor;
    let config = desc.to_config();

    assert_eq!(config.name, "strava");
    assert_eq!(config.auth_url, "https://www.strava.com/oauth/authorize");
    assert_eq!(config.token_url, "https://www.strava.com/oauth/token");
    assert_eq!(config.api_base_url, "https://www.strava.com/api/v3");
    assert!(config.revoke_url.is_some());
    if let Some(ref revoke_url) = config.revoke_url {
        assert_eq!(revoke_url, "https://www.strava.com/oauth/deauthorize");
    }
    assert!(!config.default_scopes.is_empty());
    assert!(config
        .default_scopes
        .contains(&"activity:read_all".to_owned()));
}

#[test]
#[cfg(feature = "provider-synthetic")]
fn test_synthetic_config_no_oauth() {
    let desc = SyntheticDescriptor;
    let config = desc.to_config();

    assert_eq!(config.name, "synthetic");
    // Synthetic uses placeholder URLs
    assert!(config.auth_url.contains("localhost"));
    assert!(config.token_url.contains("localhost"));
    assert!(config.revoke_url.is_none());
    assert!(config.default_scopes.is_empty());
}

#[test]
fn test_capabilities_bitflags() {
    // Test bitflag operations
    let caps = ProviderCapabilities::OAUTH | ProviderCapabilities::ACTIVITIES;
    assert!(caps.contains(ProviderCapabilities::OAUTH));
    assert!(caps.contains(ProviderCapabilities::ACTIVITIES));
    assert!(!caps.contains(ProviderCapabilities::SLEEP_TRACKING));

    // Test full health combination
    let full = ProviderCapabilities::full_health();
    assert!(full.contains(ProviderCapabilities::OAUTH));
    assert!(full.contains(ProviderCapabilities::ACTIVITIES));
    assert!(full.contains(ProviderCapabilities::SLEEP_TRACKING));
    assert!(full.contains(ProviderCapabilities::RECOVERY_METRICS));
    assert!(full.contains(ProviderCapabilities::HEALTH_METRICS));
}

#[test]
#[cfg(feature = "provider-strava")]
fn test_strava_oauth_endpoints() {
    let desc = StravaDescriptor;
    let endpoints = desc.oauth_endpoints();

    assert!(endpoints.is_some());
    if let Some(ep) = endpoints {
        assert_eq!(ep.auth_url, "https://www.strava.com/oauth/authorize");
        assert_eq!(ep.token_url, "https://www.strava.com/oauth/token");
        assert!(ep.revoke_url.is_some());
        if let Some(revoke_url) = ep.revoke_url {
            assert_eq!(revoke_url, "https://www.strava.com/oauth/deauthorize");
        }
    }
}

#[test]
#[cfg(feature = "provider-garmin")]
fn test_garmin_full_descriptor() {
    let desc = GarminDescriptor;

    // Test all descriptor methods
    assert_eq!(desc.name(), "garmin");
    assert_eq!(desc.display_name(), "Garmin Connect");

    // Test capabilities
    let caps = desc.capabilities();
    assert!(caps.requires_oauth());
    assert!(caps.supports_activities());
    assert!(caps.supports_sleep());
    assert!(caps.supports_recovery());
    assert!(caps.supports_health());

    // Test OAuth configuration
    assert!(desc.oauth_endpoints().is_some());
    if let Some(endpoints) = desc.oauth_endpoints() {
        assert!(endpoints.auth_url.contains("garmin.com"));
        assert!(endpoints.token_url.contains("garmin.com"));
    }

    // Test default scopes
    let scopes = desc.default_scopes();
    assert!(!scopes.is_empty());
    assert!(scopes.contains(&"activity:read"));
}

// ============================================================================
// Tests for ActivityQueryParams and time-based filtering
// ============================================================================

#[cfg(feature = "provider-synthetic")]
mod activity_query_params_tests {
    use chrono::{Duration, Utc};
    use pierre_mcp_server::providers::synthetic_provider::SyntheticProvider;
    use pierre_mcp_server::providers::ActivityQueryParams;
    use pierre_mcp_server::providers::CoreFitnessProvider;

    /// Helper to create a synthetic provider with sample activities spanning multiple dates
    fn create_provider_with_activities() -> SyntheticProvider {
        use pierre_mcp_server::models::{Activity, SportType};

        let provider = SyntheticProvider::new();
        let now = Utc::now();

        // Add activities at different times using struct update syntax with Default
        let activities = vec![
            Activity {
                id: "activity_1".to_owned(),
                name: "Morning Run".to_owned(),
                sport_type: SportType::Run,
                start_date: now - Duration::hours(1),
                distance_meters: Some(5000.0),
                duration_seconds: 1800,
                elevation_gain: Some(50.0),
                average_heart_rate: Some(150),
                max_heart_rate: Some(175),
                calories: Some(300),
                provider: "synthetic".to_owned(),
                ..Default::default()
            },
            Activity {
                id: "activity_2".to_owned(),
                name: "Yesterday Run".to_owned(),
                sport_type: SportType::Run,
                start_date: now - Duration::days(1),
                distance_meters: Some(10000.0),
                duration_seconds: 3600,
                elevation_gain: Some(100.0),
                average_heart_rate: Some(145),
                max_heart_rate: Some(170),
                calories: Some(600),
                provider: "synthetic".to_owned(),
                ..Default::default()
            },
            Activity {
                id: "activity_3".to_owned(),
                name: "Last Week Ride".to_owned(),
                sport_type: SportType::Ride,
                start_date: now - Duration::days(7),
                distance_meters: Some(50000.0),
                duration_seconds: 7200,
                elevation_gain: Some(500.0),
                average_heart_rate: Some(135),
                max_heart_rate: Some(160),
                calories: Some(1200),
                provider: "synthetic".to_owned(),
                ..Default::default()
            },
            Activity {
                id: "activity_4".to_owned(),
                name: "Old Ski".to_owned(),
                sport_type: SportType::CrossCountrySkiing,
                start_date: now - Duration::days(30),
                distance_meters: Some(15000.0),
                duration_seconds: 5400,
                elevation_gain: Some(200.0),
                average_heart_rate: Some(140),
                max_heart_rate: Some(165),
                calories: Some(800),
                provider: "synthetic".to_owned(),
                ..Default::default()
            },
        ];

        for activity in activities {
            let result = provider.add_activity(activity);
            assert!(result.is_ok(), "Failed to add activity: {result:?}");
        }

        provider
    }

    #[tokio::test]
    async fn test_get_activities_with_no_filters() {
        let provider = create_provider_with_activities();
        let params = ActivityQueryParams::default();

        let activities = provider.get_activities_with_params(&params).await.unwrap();

        // Should return all activities (default limit applies)
        assert!(!activities.is_empty());
    }

    #[tokio::test]
    async fn test_get_activities_with_after_filter() {
        let provider = create_provider_with_activities();
        let now = Utc::now();

        // Filter for activities after 3 days ago
        let after_timestamp = (now - Duration::days(3)).timestamp();
        let params = ActivityQueryParams {
            limit: None,
            offset: None,
            before: None,
            after: Some(after_timestamp),
        };

        let activities = provider.get_activities_with_params(&params).await.unwrap();

        // Should only get the 2 most recent activities (1 hour ago and 1 day ago)
        assert_eq!(activities.len(), 2);
        assert!(activities
            .iter()
            .all(|a| a.start_date.timestamp() >= after_timestamp));
    }

    #[tokio::test]
    async fn test_get_activities_with_before_filter() {
        let provider = create_provider_with_activities();
        let now = Utc::now();

        // Filter for activities before 2 days ago
        let before_timestamp = (now - Duration::days(2)).timestamp();
        let params = ActivityQueryParams {
            limit: None,
            offset: None,
            before: Some(before_timestamp),
            after: None,
        };

        let activities = provider.get_activities_with_params(&params).await.unwrap();

        // Should only get the 2 older activities (7 days ago and 30 days ago)
        assert_eq!(activities.len(), 2);
        assert!(activities
            .iter()
            .all(|a| a.start_date.timestamp() < before_timestamp));
    }

    #[tokio::test]
    async fn test_get_activities_with_date_range() {
        let provider = create_provider_with_activities();
        let now = Utc::now();

        // Filter for activities between 10 days ago and 2 days ago
        let after_timestamp = (now - Duration::days(10)).timestamp();
        let before_timestamp = (now - Duration::days(2)).timestamp();

        let params = ActivityQueryParams {
            limit: None,
            offset: None,
            before: Some(before_timestamp),
            after: Some(after_timestamp),
        };

        let activities = provider.get_activities_with_params(&params).await.unwrap();

        // Should only get the ride from 7 days ago
        assert_eq!(activities.len(), 1);
        assert_eq!(activities[0].id, "activity_3");
    }

    #[tokio::test]
    async fn test_activity_query_params_with_pagination() {
        let params = ActivityQueryParams::with_pagination(Some(10), Some(5));

        assert_eq!(params.limit, Some(10));
        assert_eq!(params.offset, Some(5));
        assert!(params.before.is_none());
        assert!(params.after.is_none());
    }

    #[tokio::test]
    async fn test_activity_query_params_with_time_range() {
        let before = 1_700_000_000_i64;
        let after = 1_690_000_000_i64;
        let params = ActivityQueryParams::with_time_range(Some(before), Some(after));

        assert!(params.limit.is_none());
        assert!(params.offset.is_none());
        assert_eq!(params.before, Some(before));
        assert_eq!(params.after, Some(after));
    }
}
