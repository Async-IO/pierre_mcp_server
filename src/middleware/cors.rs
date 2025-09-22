// ABOUTME: CORS middleware configuration for HTTP API endpoints
// ABOUTME: Provides Cross-Origin Resource Sharing setup for web client access

/// Configure CORS settings for the MCP server
///
/// Allows cross-origin requests from any origin with standard headers
/// including fitness provider credentials and tenant information.
#[must_use]
pub fn setup_cors() -> warp::cors::Builder {
    warp::cors()
        .allow_any_origin()
        .allow_headers(vec![
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
