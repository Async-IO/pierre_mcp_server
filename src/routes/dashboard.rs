// ABOUTME: Dashboard route handlers for monitoring and analytics
// ABOUTME: Provides REST endpoints for viewing system status, usage analytics, and request logs
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Dashboard routes for monitoring and analytics
//!
//! This module provides endpoints for viewing usage statistics, rate limit status,
//! request logs, and other monitoring data. All handlers require valid JWT authentication.

use crate::{
    dashboard_routes::DashboardRoutes as DashboardService, errors::AppError,
    mcp::resources::ServerResources,
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

/// Query parameters for usage analytics
#[derive(Deserialize)]
struct UsageAnalyticsQuery {
    #[serde(default = "default_days")]
    days: u32,
}

const fn default_days() -> u32 {
    30
}

/// Query parameters for request logs
#[derive(Deserialize)]
struct RequestLogsQuery {
    #[serde(default)]
    api_key: Option<String>,
    // Note: limit parameter not used by get_request_logs
    // Removed to avoid unused field warning
}

/// Dashboard routes
pub struct DashboardRoutes;

impl DashboardRoutes {
    /// Create all dashboard routes
    pub fn routes(resources: Arc<ServerResources>) -> Router {
        Router::new()
            .route("/dashboard/status", get(Self::handle_dashboard_overview))
            .route("/dashboard/user", get(Self::handle_dashboard_overview))
            .route("/dashboard/admin", get(Self::handle_dashboard_overview))
            .route("/dashboard/detailed", get(Self::handle_detailed_stats))
            .route("/dashboard/usage", get(Self::handle_usage_analytics))
            .route("/dashboard/rate-limits", get(Self::handle_rate_limits))
            .route("/dashboard/logs", get(Self::handle_request_logs))
            .with_state(resources)
    }

    /// Extract and authenticate user from authorization header
    async fn authenticate(
        headers: &axum::http::HeaderMap,
        resources: &Arc<ServerResources>,
    ) -> Result<crate::auth::AuthResult, AppError> {
        let auth_header = headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| AppError::auth_invalid("Missing authorization header"))?;

        resources
            .auth_middleware
            .authenticate_request(Some(auth_header))
            .await
            .map_err(|e| AppError::auth_invalid(format!("Authentication failed: {e}")))
    }

    /// Handle dashboard overview request
    async fn handle_dashboard_overview(
        State(resources): State<Arc<ServerResources>>,
        headers: axum::http::HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;

        let service = DashboardService::new(resources);
        let response = service.get_dashboard_overview(auth).await?;

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle detailed stats request
    async fn handle_detailed_stats(
        State(resources): State<Arc<ServerResources>>,
        headers: axum::http::HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;

        let service = DashboardService::new(resources);
        let response = service.get_request_stats(auth, None, None).await?;

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle usage analytics request
    async fn handle_usage_analytics(
        State(resources): State<Arc<ServerResources>>,
        headers: axum::http::HeaderMap,
        Query(params): Query<UsageAnalyticsQuery>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;

        let service = DashboardService::new(resources);
        let response = service.get_usage_analytics(auth, params.days).await?;

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle rate limits overview request
    async fn handle_rate_limits(
        State(resources): State<Arc<ServerResources>>,
        headers: axum::http::HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;

        let service = DashboardService::new(resources);
        let response = service.get_rate_limit_overview(auth).await?;

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle request logs request
    async fn handle_request_logs(
        State(resources): State<Arc<ServerResources>>,
        headers: axum::http::HeaderMap,
        Query(params): Query<RequestLogsQuery>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;

        let service = DashboardService::new(resources);
        // Note: get_request_logs takes time_range, status, tool, not limit
        // Ignoring limit parameter for now - needs proper implementation
        let response = service
            .get_request_logs(auth, params.api_key.as_deref(), None, None, None)
            .await?;

        Ok((StatusCode::OK, Json(response)).into_response())
    }
}
