// ABOUTME: HTTP route configuration and setup for MCP server endpoints
// ABOUTME: Handles warp filter creation for auth, OAuth, API keys, dashboard, and A2A routes
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

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
    /// Error message describing what went wrong
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
        let server_context = crate::context::ServerContext::from(resources.as_ref());
        let auth_routes =
            AuthRoutes::new(server_context.auth().clone(), server_context.data().clone());
        let oauth_routes = OAuthRoutes::new(
            server_context.data().clone(),
            server_context.config().clone(),
            server_context.notification().clone(),
        );
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
        crate::middleware::cors::setup_cors()
    }
}
