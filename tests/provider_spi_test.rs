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
use pierre_mcp_server::providers::ProviderDescriptor;

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
