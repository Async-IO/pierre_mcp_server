// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Integration tests for MCP server functionality
//!
//! These tests verify that the MCP server correctly handles client connections,
//! processes requests, and returns appropriate responses.

use anyhow::Result;
use pierre_mcp_server::config::{Config, ProviderConfig};
use pierre_mcp_server::mcp::McpServer;
use serde_json::{json, Value};
use std::collections::HashMap;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

/// Helper to create a test configuration
fn create_test_config() -> Config {
    let mut providers = HashMap::new();
    
    // Add a mock provider configuration
    providers.insert("mock_provider".to_string(), ProviderConfig {
        auth_type: "api_key".to_string(),
        client_id: None,
        client_secret: None,
        access_token: None,
        refresh_token: None,
        api_key: Some("test_api_key".to_string()),
    });
    
    Config { providers }
}

/// Helper to send a JSON-RPC request and receive response
async fn send_mcp_request(
    stream: &mut TcpStream,
    reader: &mut BufReader<&mut tokio::net::tcp::OwnedReadHalf>,
    request: Value,
) -> Result<Value> {
    let (mut read_half, mut write_half) = stream.split();
    let mut reader = BufReader::new(&mut read_half);
    
    // Send request
    let request_str = serde_json::to_string(&request)?;
    write_half.write_all(request_str.as_bytes()).await?;
    write_half.write_all(b"\n").await?;
    
    // Read response
    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;
    
    let response: Value = serde_json::from_str(&response_line)?;
    Ok(response)
}

#[tokio::test]
async fn test_mcp_server_initialization() -> Result<()> {
    let config = create_test_config();
    let server = McpServer::new(config);
    
    // Start server in background
    let server_task = tokio::spawn(async move {
        server.run(0).await // Use port 0 for auto-assignment
    });
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Clean up
    server_task.abort();
    
    Ok(())
}

#[tokio::test]
async fn test_mcp_initialize_request() -> Result<()> {
    let config = create_test_config();
    let server = McpServer::new(config);
    
    // Start server on a specific port for testing
    let server_task = tokio::spawn(async move {
        server.run(8081).await
    });
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Connect to server
    let mut stream = timeout(Duration::from_secs(5), TcpStream::connect("127.0.0.1:8081")).await??;
    let (mut read_half, mut write_half) = stream.split();
    let mut reader = BufReader::new(&mut read_half);
    
    // Send initialize request
    let init_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {},
        "id": 1
    });
    
    let request_str = serde_json::to_string(&init_request)?;
    write_half.write_all(request_str.as_bytes()).await?;
    write_half.write_all(b"\n").await?;
    
    // Read response
    let mut response_line = String::new();
    timeout(Duration::from_secs(5), reader.read_line(&mut response_line)).await??;
    
    let response: Value = serde_json::from_str(&response_line)?;
    
    // Verify response structure
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["result"].is_object());
    
    let result = &response["result"];
    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert!(result["serverInfo"].is_object());
    assert!(result["capabilities"].is_object());
    assert!(result["capabilities"]["tools"].is_array());
    
    // Verify expected tools are present
    let tools = result["capabilities"]["tools"].as_array().unwrap();
    let tool_names: Vec<&str> = tools.iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    
    assert!(tool_names.contains(&"get_activities"));
    assert!(tool_names.contains(&"get_athlete"));
    assert!(tool_names.contains(&"get_stats"));
    
    // Clean up
    server_task.abort();
    
    Ok(())
}

#[tokio::test]
async fn test_mcp_unknown_method() -> Result<()> {
    let config = create_test_config();
    let server = McpServer::new(config);
    
    // Start server
    let server_task = tokio::spawn(async move {
        server.run(8082).await
    });
    
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Connect and send unknown method
    let mut stream = timeout(Duration::from_secs(5), TcpStream::connect("127.0.0.1:8082")).await??;
    let (mut read_half, mut write_half) = stream.split();
    let mut reader = BufReader::new(&mut read_half);
    
    let unknown_request = json!({
        "jsonrpc": "2.0",
        "method": "unknown_method",
        "params": {},
        "id": 2
    });
    
    let request_str = serde_json::to_string(&unknown_request)?;
    write_half.write_all(request_str.as_bytes()).await?;
    write_half.write_all(b"\n").await?;
    
    let mut response_line = String::new();
    timeout(Duration::from_secs(5), reader.read_line(&mut response_line)).await??;
    
    let response: Value = serde_json::from_str(&response_line)?;
    
    // Should return method not found error
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32601);
    assert!(response["error"]["message"].as_str().unwrap().contains("Method not found"));
    
    server_task.abort();
    Ok(())
}

#[tokio::test]
async fn test_mcp_tools_call_invalid_provider() -> Result<()> {
    let config = create_test_config();
    let server = McpServer::new(config);
    
    let server_task = tokio::spawn(async move {
        server.run(8083).await
    });
    
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    let mut stream = timeout(Duration::from_secs(5), TcpStream::connect("127.0.0.1:8083")).await??;
    let (mut read_half, mut write_half) = stream.split();
    let mut reader = BufReader::new(&mut read_half);
    
    // Send tools/call with invalid provider
    let tools_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_athlete",
            "arguments": {
                "provider": "nonexistent_provider"
            }
        },
        "id": 3
    });
    
    let request_str = serde_json::to_string(&tools_request)?;
    write_half.write_all(request_str.as_bytes()).await?;
    write_half.write_all(b"\n").await?;
    
    let mut response_line = String::new();
    timeout(Duration::from_secs(5), reader.read_line(&mut response_line)).await??;
    
    let response: Value = serde_json::from_str(&response_line)?;
    
    // Should return invalid provider error
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 3);
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32602);
    assert!(response["error"]["message"].as_str().unwrap().contains("Invalid provider"));
    
    server_task.abort();
    Ok(())
}

#[tokio::test]
async fn test_mcp_multiple_connections() -> Result<()> {
    let config = create_test_config();
    let server = McpServer::new(config);
    
    let server_task = tokio::spawn(async move {
        server.run(8084).await
    });
    
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Create multiple concurrent connections
    let mut tasks = Vec::new();
    
    for i in 0..3 {
        let task = tokio::spawn(async move {
            let mut stream = TcpStream::connect("127.0.0.1:8084").await.unwrap();
            let (mut read_half, mut write_half) = stream.split();
            let mut reader = BufReader::new(&mut read_half);
            
            let init_request = json!({
                "jsonrpc": "2.0",
                "method": "initialize",
                "params": {},
                "id": i
            });
            
            let request_str = serde_json::to_string(&init_request).unwrap();
            write_half.write_all(request_str.as_bytes()).await.unwrap();
            write_half.write_all(b"\n").await.unwrap();
            
            let mut response_line = String::new();
            reader.read_line(&mut response_line).await.unwrap();
            
            let response: Value = serde_json::from_str(&response_line).unwrap();
            assert_eq!(response["jsonrpc"], "2.0");
            assert_eq!(response["id"], i);
            
            response
        });
        
        tasks.push(task);
    }
    
    // Wait for all connections to complete
    for task in tasks {
        let response = task.await?;
        assert!(response["result"].is_object());
    }
    
    server_task.abort();
    Ok(())
}

#[tokio::test]
async fn test_mcp_json_rpc_protocol_compliance() -> Result<()> {
    let config = create_test_config();
    let server = McpServer::new(config);
    
    let server_task = tokio::spawn(async move {
        server.run(8085).await
    });
    
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    let mut stream = timeout(Duration::from_secs(5), TcpStream::connect("127.0.0.1:8085")).await??;
    let (mut read_half, mut write_half) = stream.split();
    let mut reader = BufReader::new(&mut read_half);
    
    // Test various JSON-RPC compliance scenarios
    
    // 1. Valid request with string ID
    let request1 = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {},
        "id": "string-id"
    });
    
    let request_str = serde_json::to_string(&request1)?;
    write_half.write_all(request_str.as_bytes()).await?;
    write_half.write_all(b"\n").await?;
    
    let mut response_line = String::new();
    timeout(Duration::from_secs(5), reader.read_line(&mut response_line)).await??;
    
    let response1: Value = serde_json::from_str(&response_line)?;
    assert_eq!(response1["jsonrpc"], "2.0");
    assert_eq!(response1["id"], "string-id");
    assert!(response1["result"].is_object());
    
    // 2. Valid request with null ID
    let request2 = json!({
        "jsonrpc": "2.0",
        "method": "initialize", 
        "params": {},
        "id": null
    });
    
    let request_str = serde_json::to_string(&request2)?;
    write_half.write_all(request_str.as_bytes()).await?;
    write_half.write_all(b"\n").await?;
    
    response_line.clear();
    timeout(Duration::from_secs(5), reader.read_line(&mut response_line)).await??;
    
    let response2: Value = serde_json::from_str(&response_line)?;
    assert_eq!(response2["jsonrpc"], "2.0");
    assert_eq!(response2["id"], Value::Null);
    
    server_task.abort();
    Ok(())
}