// ABOUTME: Tests for Provider SPI (Service Provider Interface)
// ABOUTME: Verifies provider descriptors and capabilities work correctly
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

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
