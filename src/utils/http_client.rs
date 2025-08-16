// ABOUTME: Shared HTTP client utilities with connection pooling and timeout configuration
// ABOUTME: Provides singleton and configurable HTTP clients to eliminate redundant client creation

use reqwest::{Client, ClientBuilder};
use std::sync::OnceLock;
use std::time::Duration;

/// Global shared HTTP client with default configuration
static SHARED_CLIENT: OnceLock<Client> = OnceLock::new();

/// Get or create the shared HTTP client with default settings
///
/// This client uses connection pooling and reasonable timeouts.
/// Prefer this over creating new clients for better performance.
///
/// # Returns
/// A reference to the shared `reqwest::Client`
pub fn shared_client() -> &'static Client {
    SHARED_CLIENT.get_or_init(|| {
        ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new())
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
/// This client has shorter timeouts optimized for OAuth token exchanges
/// which should be fast operations.
///
/// # Returns
/// A new `reqwest::Client` optimized for OAuth operations
#[must_use]
pub fn oauth_client() -> Client {
    create_client_with_timeout(15, 5) // 15s request timeout, 5s connect timeout
}

/// Create a new HTTP client optimized for API calls
///
/// This client has longer timeouts suitable for external API calls
/// that might take more time to process.
///
/// # Returns
/// A new `reqwest::Client` optimized for API operations
#[must_use]
pub fn api_client() -> Client {
    create_client_with_timeout(60, 10) // 60s request timeout, 10s connect timeout
}
