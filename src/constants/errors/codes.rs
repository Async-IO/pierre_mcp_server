// ABOUTME: Error code constants for JSON-RPC and MCP protocol errors
// ABOUTME: Defines standard error codes and corresponding error messages
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Error codes for JSON-RPC and MCP protocols

/// Method not found
pub const ERROR_METHOD_NOT_FOUND: i32 = -32601;

/// Invalid parameters
pub const ERROR_INVALID_PARAMS: i32 = -32602;

/// Internal error
pub const ERROR_INTERNAL_ERROR: i32 = -32603;

/// Unauthorized - using standard JSON-RPC Internal Error for better Claude Desktop integration
pub const ERROR_UNAUTHORIZED: i32 = -32603;

/// Token-specific error codes (using standard JSON-RPC codes for better Claude Desktop integration)
pub const ERROR_TOKEN_EXPIRED: i32 = -32603; // Internal error - token expired
/// JWT token signature validation failed (internal error)
pub const ERROR_TOKEN_INVALID: i32 = -32603; // Internal error - token invalid
/// JWT token format is malformed (invalid params)
pub const ERROR_TOKEN_MALFORMED: i32 = -32602; // Invalid params - malformed token

/// MCP protocol version error codes
pub const ERROR_VERSION_MISMATCH: i32 = -32602; // Invalid params - unsupported protocol version

/// MCP-specific error codes for better diagnostics
pub const ERROR_TOOL_EXECUTION: i32 = -32000; // Server error - tool execution failed
/// MCP resource access failed
pub const ERROR_RESOURCE_ACCESS: i32 = -32001; // Server error - resource access failed
/// MCP authentication failed
pub const ERROR_AUTHENTICATION: i32 = -32002; // Server error - authentication failed
/// MCP authorization failed (insufficient permissions)
pub const ERROR_AUTHORIZATION: i32 = -32003; // Server error - authorization failed
/// Data serialization/deserialization failed
pub const ERROR_SERIALIZATION: i32 = -32004; // Server error - data serialization failed
/// Server error code for progress tracking failures
pub const ERROR_PROGRESS_TRACKING: i32 = -32005; // Server error - progress tracking failed
/// Server error code for cancelled operations
pub const ERROR_OPERATION_CANCELLED: i32 = -32006; // Server error - operation cancelled

/// Common error messages
pub const MSG_METHOD_NOT_FOUND: &str = "Method not found";
/// Error message for invalid request parameters
pub const MSG_INVALID_PARAMS: &str = "Invalid parameters";
/// Error message for internal server errors
pub const MSG_INTERNAL_ERROR: &str = "Internal error";
/// Error message when authentication is required but missing
pub const MSG_AUTH_REQUIRED: &str = "Authentication required";
/// Error message when authentication attempt fails
pub const MSG_AUTH_FAILED: &str = "Authentication failed";
/// Error message for invalid or expired authentication tokens
pub const MSG_INVALID_TOKEN: &str = "Invalid or expired token";

/// Token-specific error messages
pub const MSG_TOKEN_EXPIRED: &str = "JWT token has expired";
/// Error message for invalid JWT signature
pub const MSG_TOKEN_INVALID: &str = "JWT token signature is invalid";
/// Error message for malformed JWT token format
pub const MSG_TOKEN_MALFORMED: &str = "JWT token is malformed";

/// MCP protocol version error messages
pub const MSG_VERSION_MISMATCH: &str = "Unsupported MCP protocol version";

/// MCP-specific error messages
pub const MSG_TOOL_EXECUTION: &str = "Tool execution failed";
/// Error message for failed MCP resource access
pub const MSG_RESOURCE_ACCESS: &str = "Resource access failed";
/// Error message for MCP authentication failures
pub const MSG_AUTHENTICATION: &str = "Authentication failed";
/// Error message for insufficient permissions
pub const MSG_AUTHORIZATION: &str = "Authorization failed";
/// Error message for serialization/deserialization failures
pub const MSG_SERIALIZATION: &str = "Data serialization failed";
