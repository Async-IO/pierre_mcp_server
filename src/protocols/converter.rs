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
#[derive(Debug, Clone, PartialEq)]
pub enum ProtocolType {
    MCP,
    A2A,
}

/// Protocol converter for translating between protocol formats
pub struct ProtocolConverter;

impl ProtocolConverter {
    /// Convert A2A request to universal format
    pub fn a2a_to_universal(
        request: A2ARequest,
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
                            "Tool name not found in A2A request".to_string(),
                        )
                    })?
                    .to_string()
            }
            method => {
                return Err(crate::protocols::ProtocolError::ConversionFailed(format!(
                    "Unsupported A2A method: {}",
                    method
                )));
            }
        };

        // Extract parameters
        let parameters = request
            .params
            .as_ref()
            .and_then(|p| p.get("arguments"))
            .cloned()
            .unwrap_or(Value::Object(serde_json::Map::new()));

        Ok(UniversalRequest {
            tool_name,
            parameters,
            user_id: user_id.to_string(),
            protocol: "a2a".to_string(),
        })
    }

    /// Convert universal response to A2A format
    pub fn universal_to_a2a(response: UniversalResponse, request_id: Option<Value>) -> A2AResponse {
        if response.success {
            A2AResponse {
                jsonrpc: "2.0".to_string(),
                result: response.result,
                error: None,
                id: request_id,
            }
        } else {
            A2AResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(crate::a2a::protocol::A2AError {
                    code: -32603,
                    message: response.error.unwrap_or("Internal error".to_string()),
                    data: None,
                }),
                id: request_id,
            }
        }
    }

    /// Convert MCP tool call to universal format
    pub fn mcp_to_universal(tool_call: ToolCall, user_id: &str) -> UniversalRequest {
        UniversalRequest {
            tool_name: tool_call.name,
            parameters: tool_call
                .arguments
                .unwrap_or(Value::Object(serde_json::Map::new())),
            user_id: user_id.to_string(),
            protocol: "mcp".to_string(),
        }
    }

    /// Convert universal response to MCP format
    pub fn universal_to_mcp(response: UniversalResponse) -> ToolResponse {
        if response.success {
            let result_text =
                serde_json::to_string_pretty(&response.result.as_ref().unwrap_or(&Value::Null))
                    .unwrap_or("{}".to_string());

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
                        response.error.unwrap_or("Unknown error".to_string())
                    ),
                }],
                is_error: true,
                structured_content: None,
            }
        }
    }

    /// Detect protocol type from request format
    pub fn detect_protocol(
        request_data: &str,
    ) -> Result<ProtocolType, crate::protocols::ProtocolError> {
        // Try to parse as JSON first
        let json: Value = serde_json::from_str(request_data).map_err(|_| {
            crate::protocols::ProtocolError::ConversionFailed("Invalid JSON".to_string())
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
            "Unknown protocol format".to_string(),
        ))
    }

    /// Convert tool definition to A2A format
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
    pub fn tool_to_mcp_format(
        tool: &crate::protocols::universal::UniversalTool,
    ) -> crate::mcp::schema::Tool {
        crate::mcp::schema::Tool {
            name: tool.name.clone(),
            description: tool.description.clone(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a2a_to_universal_conversion() {
        let a2a_request = A2ARequest {
            jsonrpc: "2.0".to_string(),
            method: "a2a/tools/call".to_string(),
            params: Some(serde_json::json!({
                "tool": "get_activities",
                "arguments": {
                    "limit": 10
                }
            })),
            id: Some(Value::Number(1.into())),
        };

        let universal = ProtocolConverter::a2a_to_universal(a2a_request, "test_user").unwrap();

        assert_eq!(universal.tool_name, "get_activities");
        assert_eq!(universal.user_id, "test_user");
        assert_eq!(universal.protocol, "a2a");
        assert_eq!(
            universal.parameters.get("limit").unwrap().as_u64().unwrap(),
            10
        );
    }

    #[test]
    fn test_universal_to_a2a_conversion_success() {
        let universal_response = UniversalResponse {
            success: true,
            result: Some(serde_json::json!({"activities": []})),
            error: None,
            metadata: None,
        };

        let a2a_response =
            ProtocolConverter::universal_to_a2a(universal_response, Some(Value::Number(1.into())));

        assert_eq!(a2a_response.jsonrpc, "2.0");
        assert!(a2a_response.result.is_some());
        assert!(a2a_response.error.is_none());
    }

    #[test]
    fn test_universal_to_a2a_conversion_error() {
        let universal_response = UniversalResponse {
            success: false,
            result: None,
            error: Some("Tool not found".to_string()),
            metadata: None,
        };

        let a2a_response =
            ProtocolConverter::universal_to_a2a(universal_response, Some(Value::Number(1.into())));

        assert_eq!(a2a_response.jsonrpc, "2.0");
        assert!(a2a_response.result.is_none());
        assert!(a2a_response.error.is_some());
        assert_eq!(a2a_response.error.unwrap().message, "Tool not found");
    }

    #[test]
    fn test_mcp_to_universal_conversion() {
        let mcp_call = ToolCall {
            name: "get_activities".to_string(),
            arguments: Some(serde_json::json!({"limit": 5})),
        };

        let universal = ProtocolConverter::mcp_to_universal(mcp_call, "test_user");

        assert_eq!(universal.tool_name, "get_activities");
        assert_eq!(universal.user_id, "test_user");
        assert_eq!(universal.protocol, "mcp");
        assert_eq!(
            universal.parameters.get("limit").unwrap().as_u64().unwrap(),
            5
        );
    }

    #[test]
    fn test_universal_to_mcp_conversion_success() {
        let universal_response = UniversalResponse {
            success: true,
            result: Some(serde_json::json!({"data": "test"})),
            error: None,
            metadata: None,
        };

        let mcp_response = ProtocolConverter::universal_to_mcp(universal_response);

        assert!(!mcp_response.is_error);
        assert_eq!(mcp_response.content.len(), 1);
        match &mcp_response.content[0] {
            crate::mcp::schema::Content::Text { text } => {
                assert!(text.contains("\"data\""));
                assert!(text.contains("\"test\""));
            }
            crate::mcp::schema::Content::Image { .. } => {
                panic!("Expected text content, got image");
            }
            crate::mcp::schema::Content::Resource { .. } => {
                panic!("Expected text content, got resource");
            }
        }
    }

    #[test]
    fn test_universal_to_mcp_conversion_error() {
        let universal_response = UniversalResponse {
            success: false,
            result: None,
            error: Some("Invalid parameters".to_string()),
            metadata: None,
        };

        let mcp_response = ProtocolConverter::universal_to_mcp(universal_response);

        assert!(mcp_response.is_error);
        assert_eq!(mcp_response.content.len(), 1);
        match &mcp_response.content[0] {
            crate::mcp::schema::Content::Text { text } => {
                assert!(text.contains("Invalid parameters"));
            }
            crate::mcp::schema::Content::Image { .. } => {
                panic!("Expected text content, got image");
            }
            crate::mcp::schema::Content::Resource { .. } => {
                panic!("Expected text content, got resource");
            }
        }
    }

    #[test]
    fn test_detect_protocol_a2a() {
        let a2a_request = r#"{"jsonrpc": "2.0", "method": "a2a/tools/call", "id": 1}"#;
        let protocol = ProtocolConverter::detect_protocol(a2a_request).unwrap();
        assert_eq!(protocol, ProtocolType::A2A);
    }

    #[test]
    fn test_detect_protocol_mcp() {
        let mcp_request = r#"{"method": "tools/call", "params": {}}"#;
        let protocol = ProtocolConverter::detect_protocol(mcp_request).unwrap();
        assert_eq!(protocol, ProtocolType::MCP);
    }

    #[test]
    fn test_detect_protocol_unknown() {
        let unknown_request = r#"{"some": "data"}"#;
        let result = ProtocolConverter::detect_protocol(unknown_request);
        assert!(result.is_err());
    }
}
