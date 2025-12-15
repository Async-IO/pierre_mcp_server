// ABOUTME: MCP HTTP client for integration testing
// ABOUTME: Provides typed MCP protocol operations over HTTP transport
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used)]
// Allow dead code in test infrastructure - methods/fields designed for future test expansion
#![allow(dead_code)]
// Async methods in test code don't need to be Send
#![allow(clippy::future_not_send)]

use anyhow::Result;
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// MCP test client for HTTP transport
pub struct McpTestClient {
    http_client: Client,
    base_url: String,
    auth_token: String,
    request_id: AtomicU64,
}

/// MCP JSON-RPC error structure
#[derive(Debug, Clone, Deserialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    #[serde(default)]
    pub data: Option<Value>,
}

/// Tool information from tools/list
#[derive(Debug, Clone, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Response from tools/list
#[derive(Debug, Clone, Deserialize)]
pub struct ToolsListResponse {
    pub tools: Vec<ToolInfo>,
}

/// MCP tool call result content
#[derive(Debug, Clone, Deserialize)]
pub struct ToolResultContent {
    #[serde(rename = "type")]
    pub content_type: String,
    #[serde(default)]
    pub text: Option<String>,
}

/// MCP tool call result
#[derive(Debug, Clone, Deserialize)]
pub struct ToolCallResult {
    pub content: Vec<ToolResultContent>,
    #[serde(rename = "isError", default)]
    pub is_error: bool,
}

/// Server capabilities from initialize response
#[derive(Debug, Clone, Deserialize)]
pub struct ServerCapabilities {
    #[serde(default)]
    pub tools: Option<Value>,
    #[serde(default)]
    pub prompts: Option<Value>,
    #[serde(default)]
    pub resources: Option<Value>,
}

/// Initialize response
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: ServerInfo,
}

/// Server info from initialize
#[derive(Debug, Clone, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

impl McpTestClient {
    /// Create a new MCP test client
    pub fn new(base_url: &str, auth_token: &str) -> Self {
        Self {
            http_client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: base_url.to_owned(),
            auth_token: auth_token.to_owned(),
            request_id: AtomicU64::new(1),
        }
    }

    /// Get the next request ID
    fn next_request_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Send a raw MCP JSON-RPC request
    pub async fn send_request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let request_id = self.next_request_id();
        let request = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
            "params": params.unwrap_or(json!({}))
        });

        let response = self
            .http_client
            .post(format!("{}/mcp", self.base_url))
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.auth_token))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "MCP request failed with HTTP status: {}",
                response.status()
            ));
        }

        let json_response: Value = response.json().await?;

        // Check for JSON-RPC error
        if let Some(error) = json_response.get("error") {
            let mcp_error: McpError = serde_json::from_value(error.clone())?;
            return Err(anyhow::anyhow!(
                "MCP error {}: {}",
                mcp_error.code,
                mcp_error.message
            ));
        }

        Ok(json_response)
    }

    /// Send MCP request and extract the result field
    pub async fn send_request_for_result(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value> {
        let response = self.send_request(method, params).await?;
        response
            .get("result")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Response missing 'result' field"))
    }

    /// Initialize the MCP session
    pub async fn initialize(&self) -> Result<InitializeResult> {
        let params = json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "integration-test-client",
                "version": "1.0.0"
            }
        });

        let result = self
            .send_request_for_result("initialize", Some(params))
            .await?;
        Ok(serde_json::from_value(result)?)
    }

    /// List available tools
    pub async fn list_tools(&self) -> Result<ToolsListResponse> {
        let result = self.send_request_for_result("tools/list", None).await?;
        Ok(serde_json::from_value(result)?)
    }

    /// Call a tool with typed arguments
    pub async fn call_tool<T: Serialize>(
        &self,
        name: &str,
        arguments: &T,
    ) -> Result<ToolCallResult> {
        let params = json!({
            "name": name,
            "arguments": serde_json::to_value(arguments)?
        });

        let result = self
            .send_request_for_result("tools/call", Some(params))
            .await?;
        Ok(serde_json::from_value(result)?)
    }

    /// Call a tool with raw JSON arguments
    pub async fn call_tool_raw(&self, name: &str, arguments: Value) -> Result<ToolCallResult> {
        let params = json!({
            "name": name,
            "arguments": arguments
        });

        let result = self
            .send_request_for_result("tools/call", Some(params))
            .await?;
        Ok(serde_json::from_value(result)?)
    }

    /// Call a tool and parse the text content as JSON
    pub async fn call_tool_json<T: DeserializeOwned, A: Serialize>(
        &self,
        name: &str,
        arguments: &A,
    ) -> Result<T> {
        let result = self.call_tool(name, arguments).await?;

        if result.is_error {
            let error_text = result
                .content
                .first()
                .and_then(|c| c.text.as_ref())
                .map_or("Unknown error", String::as_str);
            return Err(anyhow::anyhow!("Tool returned error: {error_text}"));
        }

        let text = result
            .content
            .first()
            .and_then(|c| c.text.as_ref())
            .ok_or_else(|| anyhow::anyhow!("Tool result has no text content"))?;

        Ok(serde_json::from_str(text)?)
    }

    /// Call a tool expecting an error
    pub async fn call_tool_expect_error<A: Serialize>(
        &self,
        name: &str,
        arguments: &A,
    ) -> Result<McpError> {
        let params = json!({
            "name": name,
            "arguments": serde_json::to_value(arguments)?
        });

        let response = self.send_request("tools/call", Some(params)).await;

        match response {
            Ok(json) => {
                if let Some(error) = json.get("error") {
                    Ok(serde_json::from_value(error.clone())?)
                } else {
                    Err(anyhow::anyhow!("Expected error but got success"))
                }
            }
            Err(e) => {
                // Parse error message for MCP error
                let msg = e.to_string();
                if msg.starts_with("MCP error") {
                    Ok(McpError {
                        code: -32000,
                        message: msg,
                        data: None,
                    })
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Get connection status for a provider
    pub async fn get_connection_status(&self, provider: &str) -> Result<Value> {
        self.call_tool_json("get_connection_status", &json!({ "provider": provider }))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_error_deserialize() {
        let json = r#"{"code": -32600, "message": "Invalid Request"}"#;
        let error: McpError = serde_json::from_str(json).unwrap();
        assert_eq!(error.code, -32600);
        assert_eq!(error.message, "Invalid Request");
    }
}
