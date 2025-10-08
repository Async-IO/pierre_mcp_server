// ABOUTME: Unified JSON-RPC 2.0 implementation for all protocols (MCP, A2A)
// ABOUTME: Provides shared request, response, and error types eliminating duplication

//! # JSON-RPC 2.0 Foundation
//!
//! This module provides a unified implementation of JSON-RPC 2.0 used by
//! all protocols in Pierre (MCP, A2A). This eliminates duplication and
//! ensures consistent behavior across protocols.
//!
//! ## Design Goals
//!
//! 1. **Single Source of Truth**: One JSON-RPC implementation
//! 2. **Protocol Agnostic**: Works for MCP, A2A, and future protocols
//! 3. **Type Safe**: Strong typing with serde support
//! 4. **Extensible**: Metadata field for protocol-specific extensions
//!
//! ## Usage
//!
//! ```rust
//! use pierre_mcp_server::jsonrpc::{JsonRpcRequest, JsonRpcResponse};
//! # use serde_json::json;
//! # let params = json!({"key": "value"});
//! # let result = json!({"status": "ok"});
//!
//! // Create a request
//! let request = JsonRpcRequest::new("initialize", Some(params));
//!
//! // Create a success response
//! let response = JsonRpcResponse::success(request.id.clone(), result);
//!
//! // Create an error response
//! let error_response = JsonRpcResponse::error(request.id, -32600, "Invalid Request");
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// JSON-RPC 2.0 version string
pub const JSONRPC_VERSION: &str = "2.0";

/// JSON-RPC 2.0 Request
///
/// This is the unified request structure used by all protocols.
/// Protocol-specific extensions (like MCP/A2A's `auth_token`) are included as optional fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,

    /// Method name to invoke
    pub method: String,

    /// Optional parameters for the method
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,

    /// Request identifier (for correlation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,

    /// Authorization header value (Bearer token) - MCP/A2A extension
    #[serde(rename = "auth", skip_serializing_if = "Option::is_none", default)]
    pub auth_token: Option<String>,

    /// Optional HTTP headers for tenant context and other metadata - MCP extension
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub headers: Option<HashMap<String, Value>>,

    /// Protocol-specific metadata (additional extensions)
    /// Not part of JSON-RPC spec, but useful for future extensions
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub metadata: HashMap<String, String>,
}

/// JSON-RPC 2.0 Response
///
/// Represents a successful response or an error.
/// Exactly one of `result` or `error` must be present.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,

    /// Result of the method call (mutually exclusive with error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,

    /// Error information (mutually exclusive with result)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,

    /// Request identifier for correlation
    pub id: Option<Value>,
}

/// JSON-RPC 2.0 Error Object
///
/// Standard error structure with code, message, and optional data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code (standard codes: -32700 to -32600)
    pub code: i32,

    /// Human-readable error message
    pub message: String,

    /// Additional error information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcRequest {
    /// Create a new JSON-RPC request
    #[must_use]
    pub fn new(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            params,
            id: Some(Value::Number(1.into())),
            auth_token: None,
            headers: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a new request with a specific ID
    #[must_use]
    pub fn with_id(method: impl Into<String>, params: Option<Value>, id: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            params,
            id: Some(id),
            auth_token: None,
            headers: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a notification (no ID, no response expected)
    #[must_use]
    pub fn notification(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            params,
            id: None,
            auth_token: None,
            headers: None,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the request
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get metadata value
    #[must_use]
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

impl JsonRpcResponse {
    /// Create a success response
    #[must_use]
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Create an error response
    #[must_use]
    pub fn error(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
            id,
        }
    }

    /// Create an error response with additional data
    #[must_use]
    pub fn error_with_data(
        id: Option<Value>,
        code: i32,
        message: impl Into<String>,
        data: Value,
    ) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: Some(data),
            }),
            id,
        }
    }

    /// Check if this is a success response
    #[must_use]
    pub const fn is_success(&self) -> bool {
        self.error.is_none() && self.result.is_some()
    }

    /// Check if this is an error response
    #[must_use]
    pub const fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

impl JsonRpcError {
    /// Create a new error
    #[must_use]
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Create an error with data
    #[must_use]
    pub fn with_data(code: i32, message: impl Into<String>, data: Value) -> Self {
        Self {
            code,
            message: message.into(),
            data: Some(data),
        }
    }
}

/// Standard JSON-RPC error codes
pub mod error_codes {
    /// Parse error - Invalid JSON
    pub const PARSE_ERROR: i32 = -32700;

    /// Invalid Request - Invalid JSON-RPC
    pub const INVALID_REQUEST: i32 = -32600;

    /// Method not found
    pub const METHOD_NOT_FOUND: i32 = -32601;

    /// Invalid params
    pub const INVALID_PARAMS: i32 = -32602;

    /// Internal error
    pub const INTERNAL_ERROR: i32 = -32603;

    /// Server error range start
    pub const SERVER_ERROR_START: i32 = -32000;

    /// Server error range end
    pub const SERVER_ERROR_END: i32 = -32099;
}
