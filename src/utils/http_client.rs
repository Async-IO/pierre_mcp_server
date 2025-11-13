// ABOUTME: Shared HTTP client utilities with connection pooling and timeout configuration
// ABOUTME: Provides singleton and configurable HTTP clients to eliminate redundant client creation
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::config::environment::HttpClientConfig;
use reqwest::{Client, ClientBuilder};
use reqwest_middleware::{ClientBuilder as MiddlewareClientBuilder, ClientWithMiddleware};
use std::sync::OnceLock;
use std::time::Duration;

/// Global HTTP client configuration
static CLIENT_CONFIG: OnceLock<HttpClientConfig> = OnceLock::new();

/// Global shared HTTP client with configured timeouts
static SHARED_CLIENT: OnceLock<Client> = OnceLock::new();

/// Global shared HTTP client with retry middleware
static SHARED_CLIENT_WITH_RETRY: OnceLock<ClientWithMiddleware> = OnceLock::new();

/// Get client configuration with fallback to defaults
///
/// Returns defaults if HTTP client configuration was not initialized at server startup
fn get_config() -> &'static HttpClientConfig {
    static DEFAULT_CONFIG: OnceLock<HttpClientConfig> = OnceLock::new();
    CLIENT_CONFIG
        .get()
        .unwrap_or_else(|| DEFAULT_CONFIG.get_or_init(HttpClientConfig::default))
}

/// Initialize HTTP client configuration
///
/// Must be called once at server startup before any HTTP clients are created.
/// This enables proper dependency injection of timeout configuration.
///
/// # Panics
/// Panics if called more than once (configuration cannot be changed after initialization)
pub fn initialize_http_clients(config: HttpClientConfig) {
    assert!(
        CLIENT_CONFIG.set(config).is_ok(),
        "HTTP client configuration already initialized"
    );
}

// NOTE: Retry middleware removed to eliminate reqwest-retry dependency
// and reduce duplicate dependencies. Tower-based retry can be added if needed.
// For now, clients are created without retry middleware for simplicity.

/// Get or create the shared HTTP client with configured timeout settings
///
/// This client uses connection pooling and configurable timeouts.
/// Prefer this over creating new clients for better performance.
///
/// Configuration must be initialized via `initialize_http_clients()` at server startup.
///
/// # Returns
/// A reference to the shared `reqwest::Client`
///
/// # Panics
/// Panics if HTTP client configuration was not initialized at server startup
pub fn shared_client() -> &'static Client {
    SHARED_CLIENT.get_or_init(|| {
        let config = get_config();

        ClientBuilder::new()
            .timeout(Duration::from_secs(config.shared_client_timeout_secs))
            .connect_timeout(Duration::from_secs(
                config.shared_client_connect_timeout_secs,
            ))
            .build()
            .unwrap_or_else(|_| Client::new())
    })
}

/// Get or create the shared HTTP client with middleware support
///
/// This client supports middleware extensions. Use this when you need
/// request/response middleware capabilities.
///
/// Configuration must be initialized via `initialize_http_clients()` at server startup.
///
/// # Returns
/// A reference to the shared `ClientWithMiddleware`
///
/// # Panics
/// Panics if HTTP client configuration was not initialized at server startup
pub fn shared_client_with_retry() -> &'static ClientWithMiddleware {
    SHARED_CLIENT_WITH_RETRY.get_or_init(|| {
        let config = get_config();

        let base_client = ClientBuilder::new()
            .timeout(Duration::from_secs(config.shared_client_timeout_secs))
            .connect_timeout(Duration::from_secs(
                config.shared_client_connect_timeout_secs,
            ))
            .build()
            .unwrap_or_else(|_| Client::new());

        // NOTE: Retry middleware removed - add tower-based retry if needed
        MiddlewareClientBuilder::new(base_client).build()
    })
}

/// Create a new HTTP client with custom timeout settings
///
/// Use this when you need specific timeout configurations
/// that differ from the shared client defaults.
///
/// # Arguments
/// * `timeout_secs` - Request timeout in seconds
/// * `connect_timeout_secs` - Connection timeout in seconds
///
/// # Returns
/// A new `reqwest::Client` with custom timeouts
///
/// # Errors
/// Returns a default client if custom client creation fails
#[must_use]
pub fn create_client_with_timeout(timeout_secs: u64, connect_timeout_secs: u64) -> Client {
    ClientBuilder::new()
        .timeout(Duration::from_secs(timeout_secs))
        .connect_timeout(Duration::from_secs(connect_timeout_secs))
        .build()
        .unwrap_or_else(|_| Client::new())
}

/// Create a new HTTP client with custom configuration
///
/// Use this when you need specific client configurations
/// beyond just timeout settings.
///
/// # Arguments
/// * `config_fn` - Function to configure the `ClientBuilder`
///
/// # Returns
/// A new `reqwest::Client` with custom configuration
///
/// # Errors
/// Returns a default client if custom client creation fails
pub fn create_custom_client<F>(config_fn: F) -> Client
where
    F: FnOnce(ClientBuilder) -> ClientBuilder,
{
    let builder = ClientBuilder::new();
    config_fn(builder).build().unwrap_or_else(|_| Client::new())
}

/// Create a new HTTP client optimized for OAuth flows
///
/// This client has configured timeouts optimized for OAuth token exchanges.
/// Configuration must be initialized via `initialize_http_clients()` at server startup.
///
/// # Returns
/// A new `reqwest::Client` optimized for OAuth operations
///
/// # Panics
/// Panics if HTTP client configuration was not initialized at server startup
#[must_use]
pub fn oauth_client() -> Client {
    let config = get_config();

    create_client_with_timeout(
        config.oauth_client_timeout_secs,
        config.oauth_client_connect_timeout_secs,
    )
}

/// Create a new HTTP client optimized for OAuth flows with middleware support
///
/// This client supports middleware extensions for OAuth operations.
/// Configuration must be initialized via `initialize_http_clients()` at server startup.
///
/// # Returns
/// A new `ClientWithMiddleware` optimized for OAuth operations
///
/// # Panics
/// Panics if HTTP client configuration was not initialized at server startup
#[must_use]
pub fn oauth_client_with_retry() -> ClientWithMiddleware {
    let config = get_config();

    let base_client = create_client_with_timeout(
        config.oauth_client_timeout_secs,
        config.oauth_client_connect_timeout_secs,
    );

    // NOTE: Retry middleware removed - add tower-based retry if needed
    MiddlewareClientBuilder::new(base_client).build()
}

/// Create a new HTTP client optimized for API calls
///
/// This client has configured timeouts suitable for external API calls.
/// Configuration must be initialized via `initialize_http_clients()` at server startup.
///
/// # Returns
/// A new `reqwest::Client` optimized for API operations
///
/// # Panics
/// Panics if HTTP client configuration was not initialized at server startup
#[must_use]
pub fn api_client() -> Client {
    let config = get_config();

    create_client_with_timeout(
        config.api_client_timeout_secs,
        config.api_client_connect_timeout_secs,
    )
}

/// Create a new HTTP client optimized for API calls with middleware support
///
/// This client supports middleware extensions for API operations.
/// Use this for calls to external provider APIs (Strava, Garmin, etc.).
/// Configuration must be initialized via `initialize_http_clients()` at server startup.
///
/// # Returns
/// A new `ClientWithMiddleware` optimized for API operations
///
/// # Panics
/// Panics if HTTP client configuration was not initialized at server startup
#[must_use]
pub fn api_client_with_retry() -> ClientWithMiddleware {
    let config = get_config();

    let base_client = create_client_with_timeout(
        config.api_client_timeout_secs,
        config.api_client_connect_timeout_secs,
    );

    // NOTE: Retry middleware removed - add tower-based retry if needed
    MiddlewareClientBuilder::new(base_client).build()
}

/// Get health check timeout configuration
///
/// # Returns
/// Health check timeout in seconds
///
/// # Panics
/// Panics if HTTP client configuration was not initialized at server startup
#[must_use]
pub fn get_health_check_timeout_secs() -> u64 {
    get_config().health_check_timeout_secs
}

/// Get OAuth callback notification timeout configuration
///
/// # Returns
/// OAuth callback notification timeout in seconds
///
/// # Panics
/// Panics if HTTP client configuration was not initialized at server startup
#[must_use]
pub fn get_oauth_callback_notification_timeout_secs() -> u64 {
    get_config().oauth_callback_notification_timeout_secs
}
