// Simple test to debug MCP request deserialization
use serde::{Deserialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
    #[serde(default)]
    pub id: Option<Value>,
    #[serde(rename = "auth", default)]
    pub auth_token: Option<String>,
    #[serde(default)]
    pub headers: Option<std::collections::HashMap<String, Value>>,
}

fn main() {
    println!("Testing MCP request deserialization on {}", std::env::consts::OS);

    let test_json = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    println!("Test JSON: {}", serde_json::to_string_pretty(&test_json).unwrap());

    match serde_json::from_value::<McpRequest>(test_json) {
        Ok(request) => {
            println!("✓ Deserialization SUCCESS!");
            println!("  jsonrpc: {}", request.jsonrpc);
            println!("  method: {}", request.method);
            println!("  params: {:?}", request.params);
            println!("  id: {:?}", request.id);
            println!("  auth_token: {:?}", request.auth_token);
            println!("  headers: {:?}", request.headers);
        }
        Err(e) => {
            println!("✗ Deserialization FAILED: {}", e);
            println!("Error details: {:?}", e);
        }
    }
}