// ABOUTME: Error code constants for JSON-RPC and MCP protocol errors
// ABOUTME: Defines standard error codes and corresponding error messages

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
pub const ERROR_TOKEN_INVALID: i32 = -32603; // Internal error - token invalid
pub const ERROR_TOKEN_MALFORMED: i32 = -32602; // Invalid params - malformed token

/// MCP protocol version error codes
pub const ERROR_VERSION_MISMATCH: i32 = -32602; // Invalid params - unsupported protocol version

/// MCP-specific error codes for better diagnostics
pub const ERROR_TOOL_EXECUTION: i32 = -32000; // Server error - tool execution failed
pub const ERROR_RESOURCE_ACCESS: i32 = -32001; // Server error - resource access failed
pub const ERROR_AUTHENTICATION: i32 = -32002; // Server error - authentication failed
pub const ERROR_AUTHORIZATION: i32 = -32003; // Server error - authorization failed
pub const ERROR_SERIALIZATION: i32 = -32004; // Server error - data serialization failed
pub const ERROR_PROGRESS_TRACKING: i32 = -32005; // Server error - progress tracking failed
pub const ERROR_OPERATION_CANCELLED: i32 = -32006; // Server error - operation cancelled

/// Common error messages
pub const MSG_METHOD_NOT_FOUND: &str = "Method not found";
pub const MSG_INVALID_PARAMS: &str = "Invalid parameters";
pub const MSG_INTERNAL_ERROR: &str = "Internal error";
pub const MSG_AUTH_REQUIRED: &str = "Authentication required";
pub const MSG_AUTH_FAILED: &str = "Authentication failed";
pub const MSG_INVALID_TOKEN: &str = "Invalid or expired token";

/// Token-specific error messages
pub const MSG_TOKEN_EXPIRED: &str = "JWT token has expired";
pub const MSG_TOKEN_INVALID: &str = "JWT token signature is invalid";
pub const MSG_TOKEN_MALFORMED: &str = "JWT token is malformed";

/// MCP protocol version error messages
pub const MSG_VERSION_MISMATCH: &str = "Unsupported MCP protocol version";

/// MCP-specific error messages
pub const MSG_TOOL_EXECUTION: &str = "Tool execution failed";
pub const MSG_RESOURCE_ACCESS: &str = "Resource access failed";
pub const MSG_AUTHENTICATION: &str = "Authentication failed";
pub const MSG_AUTHORIZATION: &str = "Authorization failed";
pub const MSG_SERIALIZATION: &str = "Data serialization failed";
