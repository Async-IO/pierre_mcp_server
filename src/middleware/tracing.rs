// ABOUTME: Request tracing middleware for correlation and structured logging
// ABOUTME: Generates request IDs and creates spans for all HTTP requests with tenant context

use tracing::Span;
use uuid::Uuid;
use warp::Filter;

/// Request context that flows through the entire request lifecycle
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: String,
    pub user_id: Option<Uuid>,
    pub tenant_id: Option<Uuid>,
    pub auth_method: Option<String>,
}

impl RequestContext {
    /// Create new request context with generated request ID
    #[must_use]
    pub fn new() -> Self {
        Self {
            request_id: format!("req_{}", Uuid::new_v4().simple()),
            user_id: None,
            tenant_id: None,
            auth_method: None,
        }
    }

    /// Update context with authentication information
    #[must_use]
    pub fn with_auth(mut self, user_id: Uuid, auth_method: String) -> Self {
        self.user_id = Some(user_id);
        self.tenant_id = Some(user_id); // For now, user_id serves as tenant_id
        self.auth_method = Some(auth_method);
        self
    }

    /// Record context in current tracing span
    pub fn record_in_span(&self) {
        let span = Span::current();
        span.record("request_id", &self.request_id);

        if let Some(user_id) = &self.user_id {
            span.record("user_id", user_id.to_string());
        }

        if let Some(tenant_id) = &self.tenant_id {
            span.record("tenant_id", tenant_id.to_string());
        }

        if let Some(auth_method) = &self.auth_method {
            span.record("auth_method", auth_method);
        }
    }
}

impl Default for RequestContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Create request tracing middleware that generates request IDs and creates spans
#[must_use]
pub fn with_request_tracing(
) -> impl Filter<Extract = (RequestContext,), Error = warp::Rejection> + Copy {
    warp::header::optional::<String>("x-request-id").map(|request_id: Option<String>| {
        let request_id = request_id.unwrap_or_else(|| format!("req_{}", Uuid::new_v4().simple()));

        let context = RequestContext {
            request_id,
            user_id: None,
            tenant_id: None,
            auth_method: None,
        };

        // Record request ID in current span immediately
        Span::current().record("request_id", &context.request_id);

        context
    })
}

/// Create a tracing span for HTTP requests
pub fn create_request_span(method: &str, path: &str) -> tracing::Span {
    tracing::info_span!(
        "http_request",
        method = %method,
        path = %path,
        request_id = tracing::field::Empty,
        user_id = tracing::field::Empty,
        tenant_id = tracing::field::Empty,
        auth_method = tracing::field::Empty,
        status_code = tracing::field::Empty,
        duration_ms = tracing::field::Empty,
    )
}

/// Create a tracing span for MCP operations
pub fn create_mcp_span(operation: &str) -> tracing::Span {
    tracing::info_span!(
        "mcp_operation",
        operation = %operation,
        request_id = tracing::field::Empty,
        user_id = tracing::field::Empty,
        tenant_id = tracing::field::Empty,
        tool_name = tracing::field::Empty,
        duration_ms = tracing::field::Empty,
        success = tracing::field::Empty,
    )
}

/// Create a tracing span for database operations
pub fn create_database_span(operation: &str, table: &str) -> tracing::Span {
    tracing::debug_span!(
        "database_operation",
        operation = %operation,
        table = %table,
        request_id = tracing::field::Empty,
        user_id = tracing::field::Empty,
        tenant_id = tracing::field::Empty,
        duration_ms = tracing::field::Empty,
        rows_affected = tracing::field::Empty,
    )
}
