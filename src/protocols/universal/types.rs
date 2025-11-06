// ABOUTME: Core types for universal protocol system
// ABOUTME: Request, response, and executor types used across the universal protocol
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Universal request structure for protocol-agnostic tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalRequest {
    /// Name of the tool to execute
    pub tool_name: String,
    /// Tool-specific parameters as JSON
    pub parameters: Value,
    /// User ID making the request
    pub user_id: String,
    /// Protocol identifier (e.g., "mcp", "a2a")
    pub protocol: String,
    /// Optional tenant ID for multi-tenant isolation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

/// Universal response structure for tool execution results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalResponse {
    /// Whether the tool execution succeeded
    pub success: bool,
    /// Tool execution result as JSON
    pub result: Option<Value>,
    /// Error message if execution failed
    pub error: Option<String>,
    /// Additional metadata about the execution
    pub metadata: Option<HashMap<String, Value>>,
}

/// Universal tool definition with handler function
#[derive(Debug, Clone)]
pub struct UniversalTool {
    /// Tool name identifier
    pub name: String,
    /// Human-readable tool description
    pub description: String,
    /// Handler function for tool execution
    pub handler: fn(
        &UniversalToolExecutor,
        UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError>,
}

/// Type alias for backward compatibility - use `UniversalExecutor` directly in new code
pub type UniversalToolExecutor = crate::protocols::universal::executor::UniversalExecutor;
