// ABOUTME: CORS middleware configuration for HTTP API endpoints
// ABOUTME: Provides Cross-Origin Resource Sharing setup for web client access
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use http::{header::HeaderName, HeaderValue, Method};
use tower_http::cors::{AllowOrigin, CorsLayer};

/// Configure CORS settings for the MCP server
///
/// Configures cross-origin requests based on `CORS_ALLOWED_ORIGINS` environment variable.
/// Supports both wildcard ("*") for development and specific origin lists for production.
///
/// # Security Considerations
///
/// - Uses `CORS_ALLOWED_ORIGINS` environment variable for origin control
/// - Falls back to wildcard (*) if env var is empty or "*"
/// - Permits standard HTTP methods (GET, POST, PUT, DELETE, OPTIONS, PATCH)
/// - Includes custom headers for fitness provider authentication
/// - Includes tenant identification headers for multi-tenancy
///
/// # Allowed Headers
///
/// - Standard headers: content-type, authorization, accept, origin
/// - CORS headers: x-requested-with, access-control-request-*
/// - Provider headers: x-strava-client-id, x-fitbit-client-id, etc.
/// - Tenant headers: x-tenant-name, x-tenant-id
/// - API key header: x-pierre-api-key
///
/// # Examples
///
/// ```bash
/// # Allow all origins (development)
/// export CORS_ALLOWED_ORIGINS="*"
///
/// # Allow specific origins (production)
/// export CORS_ALLOWED_ORIGINS="https://app.example.com,https://admin.example.com"
/// ```
pub fn setup_cors(config: &crate::config::environment::ServerConfig) -> CorsLayer {
    // Parse allowed origins from configuration
    let allow_origin =
        if config.cors.allowed_origins.is_empty() || config.cors.allowed_origins == "*" {
            // Development mode: allow any origin
            AllowOrigin::any()
        } else {
            // Production mode: parse comma-separated origin list
            let origins: Vec<HeaderValue> = config
                .cors
                .allowed_origins
                .split(',')
                .filter_map(|s| {
                    let trimmed = s.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        HeaderValue::from_str(trimmed).ok()
                    }
                })
                .collect();

            if origins.is_empty() {
                // Fallback to any if parsing failed
                AllowOrigin::any()
            } else {
                AllowOrigin::list(origins)
            }
        };

    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_headers([
            HeaderName::from_static("content-type"),
            HeaderName::from_static("authorization"),
            HeaderName::from_static("x-requested-with"),
            HeaderName::from_static("accept"),
            HeaderName::from_static("origin"),
            HeaderName::from_static("access-control-request-method"),
            HeaderName::from_static("access-control-request-headers"),
            HeaderName::from_static("x-strava-client-id"),
            HeaderName::from_static("x-strava-client-secret"),
            HeaderName::from_static("x-fitbit-client-id"),
            HeaderName::from_static("x-fitbit-client-secret"),
            HeaderName::from_static("x-pierre-api-key"),
            HeaderName::from_static("x-tenant-name"),
            HeaderName::from_static("x-tenant-id"),
        ])
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
            Method::PATCH,
        ])
}
