// ABOUTME: Core types for universal protocol system
// ABOUTME: Request, response, and executor types used across the universal protocol

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Universal request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalRequest {
    pub tool_name: String,
    pub parameters: Value,
    pub user_id: String,
    pub protocol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

/// Universal response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalResponse {
    pub success: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
    pub metadata: Option<HashMap<String, Value>>,
}

/// Universal tool definition
#[derive(Debug, Clone)]
pub struct UniversalTool {
    pub name: String,
    pub description: String,
    pub handler: fn(
        &UniversalToolExecutor,
        UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError>,
}

/// Type alias for backward compatibility - use `UniversalExecutor` directly in new code
pub type UniversalToolExecutor = crate::protocols::universal::executor::UniversalExecutor;
