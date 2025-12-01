// ABOUTME: Tests for provider registry functionality including factory patterns and global registry
// ABOUTME: Validates provider creation, tenant provider creation, and registry operations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

mod common;

use pierre_mcp_server::constants::oauth_providers;
use pierre_mcp_server::providers::core::FitnessProvider;
use pierre_mcp_server::providers::registry::{
    create_provider, create_tenant_provider, global_registry, ProviderRegistry,
};
use uuid::Uuid;

#[test]
fn test_registry_creation() {
    let registry = ProviderRegistry::new();
    assert!(registry.is_supported(oauth_providers::STRAVA));
    assert!(!registry.is_supported("nonexistent"));
}

#[test]
fn test_global_registry() {
    let registry = global_registry();
    assert!(registry.is_supported(oauth_providers::STRAVA));
}

#[test]
fn test_create_provider() {
    common::init_test_http_clients();
    let provider = create_provider(oauth_providers::STRAVA);
    assert!(provider.is_ok());

    let provider = provider.unwrap();
    assert_eq!(provider.name(), oauth_providers::STRAVA);
}

#[tokio::test]
async fn test_create_tenant_provider() {
    common::init_test_http_clients();
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let tenant_provider = create_tenant_provider(oauth_providers::STRAVA, tenant_id, user_id);
    assert!(tenant_provider.is_ok());

    let tenant_provider = tenant_provider.unwrap();
    assert_eq!(tenant_provider.tenant_id(), tenant_id);
    assert_eq!(tenant_provider.user_id(), user_id);
    assert_eq!(tenant_provider.name(), oauth_providers::STRAVA);
}
