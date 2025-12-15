// ABOUTME: Tenant-aware logging utilities for structured, contextual logging
// ABOUTME: Provides logging macros and utilities that automatically include tenant and user context
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::constants::network_config::HTTP_CLIENT_ERROR_THRESHOLD;
use tracing::Span;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Context for provider API call logging
pub struct ProviderApiContext<'a> {
    /// User ID making the API call
    pub user_id: Uuid,
    /// Tenant ID for multi-tenant isolation
    pub tenant_id: Uuid,
    /// Provider name (e.g., "strava", "fitbit")
    pub provider: &'a str,
    /// API endpoint being called
    pub endpoint: &'a str,
    /// HTTP method (GET, POST, etc.)
    pub method: &'a str,
    /// Whether the call succeeded
    pub success: bool,
    /// Call duration in milliseconds
    pub duration_ms: u64,
    /// HTTP status code if available
    pub status_code: Option<u16>,
}

/// Tenant-aware logging utilities
pub struct TenantLogger;

impl TenantLogger {
    /// Log MCP tool call with tenant context
    pub fn log_mcp_tool_call(
        user_id: Uuid,
        tenant_id: Uuid,
        tool_name: &str,
        success: bool,
        duration_ms: u64,
    ) {
        info!(
            user_id = %user_id,
            tenant_id = %tenant_id,
            tool_name = %tool_name,
            success = %success,
            duration_ms = %duration_ms,
            event_type = "mcp_tool_call",
            "MCP tool call completed"
        );
    }

    /// Log authentication event with tenant context
    pub fn log_auth_event(
        user_id: Option<Uuid>,
        tenant_id: Option<Uuid>,
        auth_method: &str,
        success: bool,
        error_details: Option<&str>,
    ) {
        if success {
            info!(
                user_id = ?user_id,
                tenant_id = ?tenant_id,
                auth_method = %auth_method,
                success = %success,
                event_type = "authentication",
                "Authentication successful"
            );
        } else {
            warn!(
                user_id = ?user_id,
                tenant_id = ?tenant_id,
                auth_method = %auth_method,
                success = %success,
                error_details = ?error_details,
                event_type = "authentication",
                "Authentication failed"
            );
        }
    }

    /// Log HTTP request with tenant context
    pub fn log_http_request(
        user_id: Option<Uuid>,
        tenant_id: Option<Uuid>,
        method: &str,
        path: &str,
        status_code: u16,
        duration_ms: u64,
    ) {
        if status_code < HTTP_CLIENT_ERROR_THRESHOLD {
            info!(
                user_id = ?user_id,
                tenant_id = ?tenant_id,
                http_method = %method,
                http_path = %path,
                http_status = %status_code,
                duration_ms = %duration_ms,
                event_type = "http_request",
                "HTTP request completed"
            );
        } else {
            warn!(
                user_id = ?user_id,
                tenant_id = ?tenant_id,
                http_method = %method,
                http_path = %path,
                http_status = %status_code,
                duration_ms = %duration_ms,
                event_type = "http_request",
                "HTTP request failed"
            );
        }
    }

    /// Log database operation with tenant context
    pub fn log_database_operation(
        user_id: Option<Uuid>,
        tenant_id: Option<Uuid>,
        operation: &str,
        table: &str,
        success: bool,
        duration_ms: u64,
        rows_affected: Option<usize>,
    ) {
        debug!(
            user_id = ?user_id,
            tenant_id = ?tenant_id,
            db_operation = %operation,
            db_table = %table,
            success = %success,
            duration_ms = %duration_ms,
            rows_affected = ?rows_affected,
            event_type = "database_operation",
            "Database operation completed"
        );
    }

    /// Log security event with tenant context
    pub fn log_security_event(
        user_id: Option<Uuid>,
        tenant_id: Option<Uuid>,
        event_type: &str,
        severity: &str,
        details: &str,
    ) {
        warn!(
            user_id = ?user_id,
            tenant_id = ?tenant_id,
            security_event = %event_type,
            security_severity = %severity,
            security_details = %details,
            event_type = "security_event",
            "Security event detected"
        );
    }

    /// Log provider API call with tenant context
    pub fn log_provider_api_call(context: &ProviderApiContext) {
        debug!(
            user_id = %context.user_id,
            tenant_id = %context.tenant_id,
            provider = %context.provider,
            api_endpoint = %context.endpoint,
            api_method = %context.method,
            success = %context.success,
            duration_ms = %context.duration_ms,
            status_code = ?context.status_code,
            event_type = "provider_api_call",
            "Provider API call completed"
        );
    }
}

/// Record tenant context in current span
pub fn record_tenant_context(user_id: Uuid, tenant_id: Uuid, auth_method: &str) {
    let span = Span::current();
    span.record("user_id", user_id.to_string())
        .record("tenant_id", tenant_id.to_string())
        .record("auth_method", auth_method);
}

/// Record request context in current span
pub fn record_request_context(request_id: &str, method: &str, path: &str) {
    let span = Span::current();
    span.record("request_id", request_id)
        .record("http_method", method)
        .record("http_path", path);
}

/// Record performance metrics in current span
pub fn record_performance_metrics(duration_ms: u64, success: bool) {
    let span = Span::current();
    span.record("duration_ms", duration_ms)
        .record("success", success);
}

/// Create a tenant-aware span for operations
#[macro_export]
macro_rules! tenant_span {
    (info, $name:expr, $user_id:expr, $tenant_id:expr) => {
        tracing::info_span!(
            $name,
            user_id = %$user_id,
            tenant_id = %$tenant_id,
            duration_ms = tracing::field::Empty,
            success = tracing::field::Empty,
        )
    };
    (debug, $name:expr, $user_id:expr, $tenant_id:expr) => {
        tracing::debug_span!(
            $name,
            user_id = %$user_id,
            tenant_id = %$tenant_id,
            duration_ms = tracing::field::Empty,
            success = tracing::field::Empty,
        )
    };
}

/// Create a request-aware span for HTTP operations
#[macro_export]
macro_rules! request_span {
    (info, $name:expr, $request_id:expr, $method:expr, $path:expr) => {
        tracing::info_span!(
            $name,
            request_id = %$request_id,
            http_method = %$method,
            http_path = %$path,
            user_id = tracing::field::Empty,
            tenant_id = tracing::field::Empty,
            duration_ms = tracing::field::Empty,
            status_code = tracing::field::Empty,
        )
    };
    (debug, $name:expr, $request_id:expr, $method:expr, $path:expr) => {
        tracing::debug_span!(
            $name,
            request_id = %$request_id,
            http_method = %$method,
            http_path = %$path,
            user_id = tracing::field::Empty,
            tenant_id = tracing::field::Empty,
            duration_ms = tracing::field::Empty,
            status_code = tracing::field::Empty,
        )
    };
}
