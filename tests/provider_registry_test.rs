// ABOUTME: Tests for provider registry functionality including factory patterns and global registry
// ABOUTME: Validates provider creation, tenant provider creation, and registry operations
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use pierre_mcp_server::constants::oauth_providers;
use pierre_mcp_server::providers::core::FitnessProvider;
use pierre_mcp_server::providers::registry::{
    create_provider, create_tenant_provider, global_registry, ProviderRegistry,
};
use std::sync::Once;
use uuid::Uuid;

/// Ensure HTTP clients are initialized only once across all tests
static INIT_HTTP_CLIENTS: Once = Once::new();

fn ensure_http_clients_initialized() {
    INIT_HTTP_CLIENTS.call_once(|| {
        pierre_mcp_server::utils::http_client::initialize_http_clients(
            pierre_mcp_server::config::environment::HttpClientConfig::default(),
        );
    });
}

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
    ensure_http_clients_initialized();
    let provider = create_provider(oauth_providers::STRAVA);
    assert!(provider.is_ok());

    let provider = provider.unwrap();
    assert_eq!(provider.name(), oauth_providers::STRAVA);
}

#[tokio::test]
async fn test_create_tenant_provider() {
    ensure_http_clients_initialized();
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let tenant_provider = create_tenant_provider(oauth_providers::STRAVA, tenant_id, user_id);
    assert!(tenant_provider.is_ok());

    let tenant_provider = tenant_provider.unwrap();
    assert_eq!(tenant_provider.tenant_id(), tenant_id);
    assert_eq!(tenant_provider.user_id(), user_id);
    assert_eq!(tenant_provider.name(), oauth_providers::STRAVA);
}
