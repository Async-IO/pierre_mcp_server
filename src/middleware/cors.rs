// ABOUTME: CORS middleware configuration for HTTP API endpoints
// ABOUTME: Provides Cross-Origin Resource Sharing setup for web client access
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

/// Configure CORS settings for the MCP server
///
/// Allows cross-origin requests with configurable allowed origins.
/// Set `CORS_ALLOWED_ORIGINS` environment variable (comma-separated URLs) to restrict origins.
/// If not set, allows any origin (development mode).
#[must_use]
pub fn setup_cors() -> warp::cors::Builder {
    let mut cors = warp::cors();

    // Configure allowed origins from environment variable
    if let Ok(origins_str) = std::env::var("CORS_ALLOWED_ORIGINS") {
        let origins: Vec<String> = origins_str
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();

        if origins.is_empty() {
            // Empty list defaults to any origin
            cors = cors.allow_any_origin();
        } else {
            // Parse and set specific origins
            for origin in origins {
                cors = cors.allow_origin(origin.as_str());
            }
        }
    } else {
        // No environment variable - default to any origin (development mode)
        cors = cors.allow_any_origin();
    }

    cors.allow_headers(vec![
        "content-type",
        "authorization",
        "x-requested-with",
        "accept",
        "origin",
        "access-control-request-method",
        "access-control-request-headers",
        "x-strava-client-id",
        "x-strava-client-secret",
        "x-fitbit-client-id",
        "x-fitbit-client-secret",
        "x-pierre-api-key",
        "x-tenant-name",
        "x-tenant-id",
    ])
    .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS", "PATCH"])
}
