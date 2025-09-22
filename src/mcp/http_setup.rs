// ABOUTME: HTTP route configuration and setup for MCP server endpoints
// ABOUTME: Handles warp filter creation for auth, OAuth, API keys, dashboard, and A2A routes
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # HTTP Route Setup Module
//!
//! Centralizes all HTTP route creation and configuration for the MCP server.
//! Provides warp filter builders for different endpoint categories.

use super::resources::ServerResources;
use crate::a2a_routes::A2ARoutes;
use crate::api_key_routes::ApiKeyRoutes;
use crate::configuration_routes::ConfigurationRoutes;
use crate::dashboard_routes::DashboardRoutes;
use crate::fitness_configuration_routes::FitnessConfigurationRoutes;
use crate::routes::{AuthRoutes, OAuthRoutes};
use std::sync::Arc;

/// HTTP route configuration utilities
pub struct HttpSetup;

/// Error type for HTTP setup operations
#[derive(Debug)]
pub struct ApiError(pub serde_json::Value);

impl warp::reject::Reject for ApiError {}

/// HTTP error type for MCP operations
#[derive(Debug)]
pub struct McpHttpError {
    pub message: String,
}

impl warp::reject::Reject for McpHttpError {}

impl HttpSetup {
    /// Initialize all route handlers with `ServerResources` (eliminates cloning anti-pattern)
    #[must_use]
    pub fn setup_route_handlers_with_resources(
        resources: &Arc<ServerResources>,
    ) -> (
        AuthRoutes,
        OAuthRoutes,
        ApiKeyRoutes,
        DashboardRoutes,
        A2ARoutes,
        Arc<ConfigurationRoutes>,
        Arc<FitnessConfigurationRoutes>,
    ) {
        let auth_routes = AuthRoutes::new(resources.clone()); // Safe: Arc clone for route handler
        let oauth_routes = OAuthRoutes::new(resources.clone()); // Safe: Arc clone for route handler
        let api_key_routes = ApiKeyRoutes::new(resources.clone()); // Safe: Arc clone for route handler
        let dashboard_routes = DashboardRoutes::new(resources.clone()); // Safe: Arc clone for route handler
        let a2a_routes = A2ARoutes::new(resources.clone()); // Safe: Arc clone for route handler
        let configuration_routes = Arc::new(ConfigurationRoutes::new(resources.clone())); // Safe: Arc clone for route handler
        let fitness_configuration_routes =
            Arc::new(FitnessConfigurationRoutes::new(resources.clone())); // Safe: Arc clone for route handler

        (
            auth_routes,
            oauth_routes,
            api_key_routes,
            dashboard_routes,
            a2a_routes,
            configuration_routes,
            fitness_configuration_routes,
        )
    }

    /// Configure CORS settings
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
}
