// ABOUTME: Protocol data conversion between different fitness platform formats
// ABOUTME: Transforms data between Strava, Fitbit, and internal universal formats
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Protocol Converter
//!
//! Converts between different protocol formats (MCP, A2A) and the universal format.

use crate::a2a::protocol::{A2ARequest, A2AResponse};
use crate::mcp::schema::{ToolCall, ToolResponse};
use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use serde_json::Value;

/// Supported protocol types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolType {
    MCP,
    A2A,
}

/// Protocol converter for translating between protocol formats
pub struct ProtocolConverter;

impl ProtocolConverter {
    /// Convert A2A request to universal format
    ///
    /// # Errors
    ///
    /// Returns an error if the A2A request has an unsupported method or if the tool name is not found in the parameters.
    pub fn a2a_to_universal(
        request: &A2ARequest,
        user_id: &str,
    ) -> Result<UniversalRequest, crate::protocols::ProtocolError> {
        // Extract tool name from A2A method
        let tool_name = match request.method.as_str() {
            "a2a/tools/call" => {
                // Tool name should be in parameters
                request
                    .params
                    .as_ref()
                    .and_then(|p| p.get("tool"))
                    .and_then(|t| t.as_str())
                    .ok_or_else(|| {
                        crate::protocols::ProtocolError::InvalidParameters(
                            "Tool name not found in A2A request".into(),
                        )
                    })?
                    .to_string()
            }
            method => {
                return Err(crate::protocols::ProtocolError::ConversionFailed(format!(
                    "Unsupported A2A method: {method}"
                )));
            }
        };

        // Extract parameters
        let parameters = request
            .params
            .as_ref()
            .and_then(|p| p.get("arguments"))
            .cloned()
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

        Ok(UniversalRequest {
            tool_name,
            parameters,
            user_id: user_id.to_string(),
            protocol: "a2a".into(),
            tenant_id: None,
        })
    }

    /// Convert universal response to A2A format
    #[must_use]
    pub fn universal_to_a2a(response: UniversalResponse, request_id: Option<Value>) -> A2AResponse {
        if response.success {
            A2AResponse {
                jsonrpc: "2.0".into(),
                result: response.result,
                error: None,
                id: request_id,
            }
        } else {
            A2AResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(crate::a2a::protocol::A2AErrorResponse {
                    code: -32603,
                    message: response.error.unwrap_or_else(|| "Internal error".into()),
                    data: None,
                }),
                id: request_id,
            }
        }
    }

    /// Convert MCP tool call to universal format
    #[must_use]
    pub fn mcp_to_universal(
        tool_call: ToolCall,
        user_id: &str,
        tenant_id: Option<String>,
    ) -> UniversalRequest {
        UniversalRequest {
            tool_name: tool_call.name,
            parameters: tool_call
                .arguments
                .unwrap_or_else(|| Value::Object(serde_json::Map::new())),
            user_id: user_id.to_string(),
            protocol: "mcp".into(),
            tenant_id,
        }
    }

    /// Convert universal response to MCP format
    #[must_use]
    pub fn universal_to_mcp(response: UniversalResponse) -> ToolResponse {
        if response.success {
            let result_text =
                serde_json::to_string_pretty(&response.result.as_ref().unwrap_or(&Value::Null))
                    .unwrap_or_else(|_| "{}".into());

            ToolResponse {
                content: vec![crate::mcp::schema::Content::Text { text: result_text }],
                is_error: false,
                structured_content: response.result,
            }
        } else {
            ToolResponse {
                content: vec![crate::mcp::schema::Content::Text {
                    text: format!(
                        "Error: {}",
                        response.error.unwrap_or_else(|| "Unknown error".into())
                    ),
                }],
                is_error: true,
                structured_content: None,
            }
        }
    }

    /// Detect protocol type from request format
    ///
    /// # Errors
    ///
    /// Returns an error if the request data is not valid JSON or if the protocol type cannot be determined.
    pub fn detect_protocol(
        request_data: &str,
    ) -> Result<ProtocolType, crate::protocols::ProtocolError> {
        // Try to parse as JSON first
        let json: Value = serde_json::from_str(request_data).map_err(|_| {
            crate::protocols::ProtocolError::ConversionFailed("Invalid JSON".into())
        })?;

        // Check for A2A indicators
        if json.get("jsonrpc").is_some() && json.get("method").is_some() {
            if let Some(method) = json.get("method").and_then(|m| m.as_str()) {
                if method.starts_with("a2a/") {
                    return Ok(ProtocolType::A2A);
                }
            }
        }

        // Check for MCP indicators
        if json.get("method").is_some() {
            if let Some(method) = json.get("method").and_then(|m| m.as_str()) {
                if method == "tools/call" || method == "initialize" {
                    return Ok(ProtocolType::MCP);
                }
            }
        }

        Err(crate::protocols::ProtocolError::UnsupportedProtocol(
            "Unknown protocol format".into(),
        ))
    }

    /// Convert tool definition to A2A format
    #[must_use]
    pub fn tool_to_a2a_format(tool: &crate::protocols::universal::UniversalTool) -> Value {
        serde_json::json!({
            "name": tool.name,
            "description": tool.description,
            "parameters": {
                "type": "object",
                "properties": {},
                "required": []
            }
        })
    }

    /// Convert tool definition to MCP format
    #[must_use]
    pub fn tool_to_mcp_format(
        tool: &crate::protocols::universal::UniversalTool,
    ) -> crate::mcp::schema::Tool {
        crate::mcp::schema::Tool {
            name: tool.name.clone(), // Safe: String ownership needed for MCP tool schema
            description: tool.description.clone(), // Safe: String ownership for MCP tool schema
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }
}
