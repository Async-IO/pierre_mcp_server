// ABOUTME: Tests for feature flag configuration and validation
// ABOUTME: Verifies feature combinations are valid and logging works correctly
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Tests for feature flag configuration and validation.

use pierre_mcp_server::features::FeatureConfig;

#[test]
fn test_feature_config_new() {
    let config = FeatureConfig::new();
    // With default features (server-full), all should be enabled
    assert!(FeatureConfig::has_any_protocol());
    assert!(FeatureConfig::has_any_transport());
    // Config is a ZST, just verify it can be created
    let _ = config;
}

#[test]
fn test_validate_succeeds_with_default_features() {
    // With server-full default, validation should pass
    let result = FeatureConfig::validate();
    assert!(result.is_ok());
}

#[test]
fn test_has_web_transport_with_default_features() {
    // With server-full, HTTP should be enabled
    assert!(FeatureConfig::transport_http());
    // has_web_transport should be true when HTTP is enabled
    assert!(FeatureConfig::has_web_transport());
}

#[test]
fn test_protocol_checks_with_default_features() {
    // With server-full, all protocols should be enabled
    assert!(FeatureConfig::protocol_rest());
    assert!(FeatureConfig::protocol_mcp());
    assert!(FeatureConfig::protocol_a2a());
    assert!(FeatureConfig::has_any_protocol());
}

#[test]
fn test_transport_checks_with_default_features() {
    // With server-full, all transports should be enabled
    assert!(FeatureConfig::transport_http());
    assert!(FeatureConfig::transport_websocket());
    assert!(FeatureConfig::transport_sse());
    assert!(FeatureConfig::transport_stdio());
    assert!(FeatureConfig::has_any_transport());
}

#[test]
fn test_client_checks_with_default_features() {
    // With server-full, all clients should be enabled
    assert!(FeatureConfig::client_dashboard());
    assert!(FeatureConfig::client_settings());
    assert!(FeatureConfig::client_chat());
    assert!(FeatureConfig::client_coaches());
    assert!(FeatureConfig::has_any_client());
}

#[test]
fn test_infrastructure_checks_with_default_features() {
    // With server-full, oauth should be enabled
    assert!(FeatureConfig::oauth());
    // Note: openapi is NOT part of server-full by default
    // It must be explicitly enabled with --features openapi
}
