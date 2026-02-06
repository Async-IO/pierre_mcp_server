// ABOUTME: Dashboard route handlers for monitoring and analytics
// ABOUTME: Provides REST endpoints for viewing system status, usage analytics, and request logs
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Dashboard routes for monitoring and analytics
//!
//! This module provides endpoints for viewing usage statistics, rate limit status,
//! request logs, and other monitoring data. All handlers require valid JWT authentication.

use crate::{
    auth::AuthResult, dashboard_routes::DashboardRoutes as DashboardService, errors::AppError,
    mcp::resources::ServerResources, security::cookies::get_cookie_value,
};
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
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
    #[serde(default)]
    time_range: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    tool: Option<String>,
}

/// Query parameters for tool usage
#[derive(Deserialize)]
struct ToolUsageQuery {
    #[serde(default)]
    api_key_id: Option<String>,
    #[serde(default = "default_time_range")]
    time_range: String,
}

fn default_time_range() -> String {
    "7d".to_owned()
}

/// Dashboard routes
pub struct DashboardRoutes;

impl DashboardRoutes {
    /// Create all dashboard routes
    ///
    /// Routes are prefixed with /api to match frontend API conventions:
    /// - /api/dashboard/overview - Dashboard overview (status, user, admin)
    /// - /api/dashboard/analytics - Usage analytics with configurable time range
    /// - /api/dashboard/rate-limits - Rate limit status
    /// - /api/dashboard/request-logs - Request logs with filtering
    /// - /api/dashboard/request-stats - Detailed request statistics
    /// - /api/dashboard/tool-usage - Tool usage breakdown
    pub fn routes(resources: Arc<ServerResources>) -> Router {
        Router::new()
            // Primary dashboard endpoints matching frontend API calls
            .route(
                "/api/dashboard/overview",
                get(Self::handle_dashboard_overview),
            )
            .route(
                "/api/dashboard/analytics",
                get(Self::handle_usage_analytics),
            )
            .route("/api/dashboard/rate-limits", get(Self::handle_rate_limits))
            .route(
                "/api/dashboard/request-logs",
                get(Self::handle_request_logs),
            )
            .route(
                "/api/dashboard/request-stats",
                get(Self::handle_detailed_stats),
            )
            .route("/api/dashboard/tool-usage", get(Self::handle_tool_usage))
            // Alternative routes without /api prefix
            .route("/dashboard/status", get(Self::handle_dashboard_overview))
            .route("/dashboard/user", get(Self::handle_dashboard_overview))
            .route("/dashboard/admin", get(Self::handle_dashboard_overview))
            .route("/dashboard/detailed", get(Self::handle_detailed_stats))
            .route("/dashboard/usage", get(Self::handle_usage_analytics))
            .route("/dashboard/rate-limits", get(Self::handle_rate_limits))
            .route("/dashboard/logs", get(Self::handle_request_logs))
            .with_state(resources)
    }

    /// Extract and authenticate user from authorization header or cookie
    async fn authenticate(
        headers: &HeaderMap,
        resources: &Arc<ServerResources>,
    ) -> Result<AuthResult, AppError> {
        // Try Authorization header first
        let auth_value =
            if let Some(auth_header) = headers.get("authorization").and_then(|h| h.to_str().ok()) {
                auth_header.to_owned()
            } else if let Some(token) = get_cookie_value(headers, "auth_token") {
                // Fall back to auth_token cookie, format as Bearer token
                format!("Bearer {token}")
            } else {
                return Err(AppError::auth_invalid(
                    "Missing authorization header or cookie",
                ));
            };

        resources
            .auth_middleware
            .authenticate_request(Some(&auth_value))
            .await
            .map_err(|e| AppError::auth_invalid(format!("Authentication failed: {e}")))
    }

    /// Handle dashboard overview request
    async fn handle_dashboard_overview(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;

        let service = DashboardService::new(resources);
        let response = service.get_dashboard_overview(auth).await?;

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle detailed stats request
    async fn handle_detailed_stats(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;

        let service = DashboardService::new(resources);
        let response = service.get_request_stats(auth, None, None).await?;

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle usage analytics request
    async fn handle_usage_analytics(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
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
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;

        let service = DashboardService::new(resources);
        let response = service.get_rate_limit_overview(auth).await?;

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle request logs request
    async fn handle_request_logs(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Query(params): Query<RequestLogsQuery>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;

        let service = DashboardService::new(resources);
        let response = service
            .get_request_logs(
                auth,
                params.api_key.as_deref(),
                params.time_range.as_deref(),
                params.status.as_deref(),
                params.tool.as_deref(),
            )
            .await?;

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle tool usage breakdown request
    async fn handle_tool_usage(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Query(params): Query<ToolUsageQuery>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;

        let service = DashboardService::new(resources);
        let response = service
            .get_tool_usage_breakdown(
                auth,
                params.api_key_id.as_deref(),
                Some(params.time_range.as_str()),
            )
            .await?;

        Ok((StatusCode::OK, Json(response)).into_response())
    }
}
