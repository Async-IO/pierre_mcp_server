// ABOUTME: Tests for Firebase authentication module
// ABOUTME: Validates Firebase config and token parsing utilities
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(missing_docs)]

use pierre_mcp_server::config::environment::FirebaseConfig;

#[test]
fn test_firebase_config_is_configured() {
    let config = FirebaseConfig {
        project_id: Some("test-project".to_owned()),
        api_key: None,
        enabled: true,
        key_cache_ttl_secs: 3600,
    };
    assert!(config.is_configured());

    let disabled = FirebaseConfig {
        project_id: Some("test-project".to_owned()),
        api_key: None,
        enabled: false,
        key_cache_ttl_secs: 3600,
    };
    assert!(!disabled.is_configured());

    let no_project = FirebaseConfig {
        project_id: None,
        api_key: None,
        enabled: true,
        key_cache_ttl_secs: 3600,
    };
    assert!(!no_project.is_configured());
}

#[test]
fn test_firebase_config_defaults() {
    let config = FirebaseConfig::default();
    assert!(!config.is_configured());
    assert!(config.project_id.is_none());
    assert!(!config.enabled);
}
