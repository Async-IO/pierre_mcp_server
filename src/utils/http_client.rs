// ABOUTME: Shared HTTP client utilities with connection pooling and timeout configuration
// ABOUTME: Provides singleton and configurable HTTP clients to eliminate redundant client creation
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::config::environment::HttpClientConfig;
use reqwest::{Client, ClientBuilder};
use reqwest_middleware::{ClientBuilder as MiddlewareClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use std::sync::OnceLock;
use std::time::Duration;

/// Global HTTP client configuration
static CLIENT_CONFIG: OnceLock<HttpClientConfig> = OnceLock::new();

/// Global shared HTTP client with configured timeouts
static SHARED_CLIENT: OnceLock<Client> = OnceLock::new();

/// Global shared HTTP client with retry middleware
static SHARED_CLIENT_WITH_RETRY: OnceLock<ClientWithMiddleware> = OnceLock::new();

/// Initialize HTTP client configuration
///
/// Must be called once at server startup before any HTTP clients are created.
/// This enables proper dependency injection of timeout configuration.
///
/// # Panics
/// Panics if called more than once (configuration cannot be changed after initialization)
pub fn initialize_http_clients(config: HttpClientConfig) {
    CLIENT_CONFIG
        .set(config)
        .expect("HTTP client configuration already initialized");
}

/// Create an exponential backoff retry policy from configuration
///
/// # Arguments
/// * `config` - HTTP client configuration with retry settings
///
/// # Returns
/// A configured `ExponentialBackoff` policy with jitter
fn create_retry_policy(config: &HttpClientConfig) -> ExponentialBackoff {
    let base_delay = Duration::from_millis(config.retry_base_delay_ms);
    let max_delay = Duration::from_millis(config.retry_max_delay_ms);

    ExponentialBackoff::builder()
        .retry_bounds(base_delay, max_delay)
        .build_with_max_retries(config.max_retries)
}

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
        let config = CLIENT_CONFIG.get().expect(
            "HTTP client configuration not initialized - call initialize_http_clients() at startup",
        );

        ClientBuilder::new()
            .timeout(Duration::from_secs(config.shared_client_timeout_secs))
            .connect_timeout(Duration::from_secs(
                config.shared_client_connect_timeout_secs,
            ))
            .build()
            .unwrap_or_else(|_| Client::new())
    })
}

/// Get or create the shared HTTP client with retry middleware
///
/// This client includes exponential backoff with jitter for transient errors.
/// Use this for external API calls that may experience temporary failures.
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
        let config = CLIENT_CONFIG.get().expect(
            "HTTP client configuration not initialized - call initialize_http_clients() at startup",
        );

        let base_client = ClientBuilder::new()
            .timeout(Duration::from_secs(config.shared_client_timeout_secs))
            .connect_timeout(Duration::from_secs(
                config.shared_client_connect_timeout_secs,
            ))
            .build()
            .unwrap_or_else(|_| Client::new());

        if config.enable_retries {
            let retry_policy = create_retry_policy(config);
            MiddlewareClientBuilder::new(base_client)
                .with(RetryTransientMiddleware::new_with_policy(retry_policy))
                .build()
        } else {
            MiddlewareClientBuilder::new(base_client).build()
        }
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
    let config = CLIENT_CONFIG.get().expect(
        "HTTP client configuration not initialized - call initialize_http_clients() at startup",
    );

    create_client_with_timeout(
        config.oauth_client_timeout_secs,
        config.oauth_client_connect_timeout_secs,
    )
}

/// Create a new HTTP client optimized for OAuth flows with retry middleware
///
/// This client includes exponential backoff with jitter for handling transient OAuth server errors.
/// Configuration must be initialized via `initialize_http_clients()` at server startup.
///
/// # Returns
/// A new `ClientWithMiddleware` optimized for OAuth operations
///
/// # Panics
/// Panics if HTTP client configuration was not initialized at server startup
#[must_use]
pub fn oauth_client_with_retry() -> ClientWithMiddleware {
    let config = CLIENT_CONFIG.get().expect(
        "HTTP client configuration not initialized - call initialize_http_clients() at startup",
    );

    let base_client = create_client_with_timeout(
        config.oauth_client_timeout_secs,
        config.oauth_client_connect_timeout_secs,
    );

    if config.enable_retries {
        let retry_policy = create_retry_policy(config);
        MiddlewareClientBuilder::new(base_client)
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build()
    } else {
        MiddlewareClientBuilder::new(base_client).build()
    }
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
    let config = CLIENT_CONFIG.get().expect(
        "HTTP client configuration not initialized - call initialize_http_clients() at startup",
    );

    create_client_with_timeout(
        config.api_client_timeout_secs,
        config.api_client_connect_timeout_secs,
    )
}

/// Create a new HTTP client optimized for API calls with retry middleware
///
/// This client includes exponential backoff with jitter for handling transient API failures.
/// Use this for calls to external provider APIs (Strava, Garmin, etc.) that may rate limit or have temporary issues.
/// Configuration must be initialized via `initialize_http_clients()` at server startup.
///
/// # Returns
/// A new `ClientWithMiddleware` optimized for API operations
///
/// # Panics
/// Panics if HTTP client configuration was not initialized at server startup
#[must_use]
pub fn api_client_with_retry() -> ClientWithMiddleware {
    let config = CLIENT_CONFIG.get().expect(
        "HTTP client configuration not initialized - call initialize_http_clients() at startup",
    );

    let base_client = create_client_with_timeout(
        config.api_client_timeout_secs,
        config.api_client_connect_timeout_secs,
    );

    if config.enable_retries {
        let retry_policy = create_retry_policy(config);
        MiddlewareClientBuilder::new(base_client)
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build()
    } else {
        MiddlewareClientBuilder::new(base_client).build()
    }
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
    CLIENT_CONFIG
        .get()
        .expect(
            "HTTP client configuration not initialized - call initialize_http_clients() at startup",
        )
        .health_check_timeout_secs
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
    CLIENT_CONFIG
        .get()
        .expect(
            "HTTP client configuration not initialized - call initialize_http_clients() at startup",
        )
        .oauth_callback_notification_timeout_secs
}
