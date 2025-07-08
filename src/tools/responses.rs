// ABOUTME: Unified response formatting for all APIs
// ABOUTME: Provides consistent response formatting across MCP, A2A, and HTTP APIs

use crate::errors::AppError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Standard response wrapper for all APIs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Response data
    pub data: T,
    /// Response metadata
    pub meta: ResponseMetadata,
}

/// Response metadata for tracking and debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    /// Request ID for tracing
    pub request_id: Option<String>,
    /// Timestamp when response was generated
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Response processing time in milliseconds
    pub processing_time_ms: Option<u64>,
    /// API version
    pub version: String,
    /// User ID (for multi-tenant responses)
    pub user_id: Option<Uuid>,
}

/// Standard error response format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error information
    pub error: ErrorInfo,
    /// Response metadata
    pub meta: ResponseMetadata,
}

/// Error information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    /// Error code for programmatic handling
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Additional error details
    pub details: Option<Value>,
    /// Error context (`request_id`, `user_id`, etc.)
    pub context: Option<Value>,
}

/// Pagination information for list responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    /// Current page number (0-based)
    pub page: usize,
    /// Number of items per page
    pub per_page: usize,
    /// Total number of items
    pub total_items: usize,
    /// Total number of pages
    pub total_pages: usize,
    /// Whether there are more items available
    pub has_next: bool,
    /// Whether there are previous items available
    pub has_prev: bool,
}

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// Response data items
    pub data: Vec<T>,
    /// Pagination information
    pub pagination: PaginationInfo,
    /// Response metadata
    pub meta: ResponseMetadata,
}

/// Parameters for pagination
#[derive(Debug, Clone)]
pub struct PaginationParams {
    pub page: usize,
    pub per_page: usize,
    pub total_items: usize,
    pub request_id: Option<String>,
    pub user_id: Option<Uuid>,
    pub processing_time_ms: Option<u64>,
}

/// Unified response formatter
pub struct ResponseFormatter {
    /// Default API version
    version: String,
}

impl Default for ResponseFormatter {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl ResponseFormatter {
    /// Create a new response formatter with custom version
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into(),
        }
    }

    /// Format a successful response
    pub fn success<T: Serialize>(
        &self,
        data: T,
        request_id: Option<String>,
        user_id: Option<Uuid>,
        processing_time_ms: Option<u64>,
    ) -> ApiResponse<T> {
        ApiResponse {
            data,
            meta: ResponseMetadata {
                request_id,
                timestamp: chrono::Utc::now(),
                processing_time_ms,
                version: self.version.clone(),
                user_id,
            },
        }
    }

    /// Format a paginated response
    #[must_use]
    pub fn paginated<T: Serialize>(
        &self,
        data: Vec<T>,
        params: PaginationParams,
    ) -> PaginatedResponse<T> {
        let total_pages = params.total_items.div_ceil(params.per_page);

        PaginatedResponse {
            data,
            pagination: PaginationInfo {
                page: params.page,
                per_page: params.per_page,
                total_items: params.total_items,
                total_pages,
                has_next: params.page + 1 < total_pages,
                has_prev: params.page > 0,
            },
            meta: ResponseMetadata {
                request_id: params.request_id,
                timestamp: chrono::Utc::now(),
                processing_time_ms: params.processing_time_ms,
                version: self.version.clone(),
                user_id: params.user_id,
            },
        }
    }

    /// Format an error response
    #[must_use]
    pub fn error(
        &self,
        error: &AppError,
        request_id: Option<String>,
        processing_time_ms: Option<u64>,
    ) -> ErrorResponse {
        ErrorResponse {
            error: ErrorInfo {
                code: format!("{:?}", error.code),
                message: error.message.clone(),
                details: None,
                context: None,
            },
            meta: ResponseMetadata {
                request_id: request_id.or_else(|| error.request_id.clone()),
                timestamp: chrono::Utc::now(),
                processing_time_ms,
                version: self.version.clone(),
                user_id: None,
            },
        }
    }

    /// Format an MCP protocol response
    #[must_use]
    pub fn mcp_response(
        &self,
        result: &Value,
        request_id: Option<&String>,
        processing_time_ms: Option<u64>,
    ) -> Value {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": result,
            "meta": {
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "processing_time_ms": processing_time_ms,
                "version": self.version,
            }
        })
    }

    /// Format an MCP protocol error
    #[must_use]
    pub fn mcp_error(
        &self,
        error: &AppError,
        request_id: Option<&String>,
        processing_time_ms: Option<u64>,
    ) -> Value {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "error": {
                "code": error.code.http_status(),
                "message": error.message,
                "data": {
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "processing_time_ms": processing_time_ms,
                    "version": self.version,
                }
            }
        })
    }

    /// Format a simple success message
    pub fn simple_success(
        &self,
        message: impl Into<String>,
        request_id: Option<String>,
        user_id: Option<Uuid>,
        processing_time_ms: Option<u64>,
    ) -> ApiResponse<Value> {
        self.success(
            serde_json::json!({
                "success": true,
                "message": message.into()
            }),
            request_id,
            user_id,
            processing_time_ms,
        )
    }

    /// Format a tool execution result for consistency across protocols
    #[must_use]
    pub fn tool_result(
        &self,
        tool_name: &str,
        result: &Value,
        request_id: Option<String>,
        user_id: Option<Uuid>,
        processing_time_ms: Option<u64>,
    ) -> ApiResponse<Value> {
        self.success(
            serde_json::json!({
                "tool": tool_name,
                "result": result,
                "execution_info": {
                    "success": true,
                    "processing_time_ms": processing_time_ms,
                }
            }),
            request_id,
            user_id,
            processing_time_ms,
        )
    }
}

/// Global response formatter instance
pub static RESPONSE_FORMATTER: std::sync::LazyLock<ResponseFormatter> =
    std::sync::LazyLock::new(ResponseFormatter::default);

/// Convenience functions for common response patterns
pub mod helpers {
    use super::{
        ApiResponse, AppError, ErrorResponse, PaginatedResponse, PaginationParams, Serialize,
        Value, RESPONSE_FORMATTER,
    };

    /// Quick success response
    pub fn success<T: Serialize>(data: T) -> ApiResponse<T> {
        RESPONSE_FORMATTER.success(data, None, None, None)
    }

    /// Quick error response
    pub fn error(err: &AppError) -> ErrorResponse {
        RESPONSE_FORMATTER.error(err, None, None)
    }

    /// Quick paginated response
    pub fn paginated<T: Serialize>(
        data: Vec<T>,
        page: usize,
        per_page: usize,
        total_items: usize,
    ) -> PaginatedResponse<T> {
        RESPONSE_FORMATTER.paginated(
            data,
            PaginationParams {
                page,
                per_page,
                total_items,
                request_id: None,
                user_id: None,
                processing_time_ms: None,
            },
        )
    }

    /// Quick MCP response
    pub fn mcp_success(result: &Value) -> Value {
        RESPONSE_FORMATTER.mcp_response(result, None, None)
    }

    /// Quick MCP error
    pub fn mcp_error(error: &AppError) -> Value {
        RESPONSE_FORMATTER.mcp_error(error, None, None)
    }
}
