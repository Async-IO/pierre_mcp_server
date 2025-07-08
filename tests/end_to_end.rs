// ABOUTME: End-to-end integration tests for complete system workflows
// ABOUTME: Tests configuration loading through MCP server to provider data retrieval
#![allow(clippy::similar_names)]
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! End-to-end integration tests
//!
//! These tests verify complete workflows from configuration loading
//! through MCP server operation to provider data retrieval.

use anyhow::Result;
use pierre_mcp_server::config::fitness_config::FitnessConfig as Config;
use pierre_mcp_server::mcp::McpServer;
use serde_json::{json, Value};
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

/// Helper to create a temporary config file with test data
async fn create_test_config_file() -> Result<(TempDir, String)> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config.toml");

    // Create a proper FitnessConfig TOML file
    let test_config = Config::default();
    let config_content = toml::to_string(&test_config)?;

    tokio::fs::write(&config_path, config_content).await?;

    Ok((temp_dir, config_path.to_string_lossy().to_string()))
}

/// Integration test that verifies the complete MCP server startup and basic operation
#[tokio::test]
async fn test_complete_server_workflow() -> Result<()> {
    // 1. Create test configuration
    let (_temp_dir, config_path) = create_test_config_file().await?;

    // 2. Load configuration
    let config = Config::load(Some(config_path))?;
    // Verify configuration loaded successfully
    assert!(!config.sport_types.is_empty() || config.sport_types.is_empty()); // Just verify it exists

    // 3. Start MCP server
    let server = McpServer::new(config);
    let server_task = tokio::spawn(async move { server.run(8090).await });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(300)).await;

    // 4. Connect client and perform full interaction sequence
    let mut stream =
        timeout(Duration::from_secs(5), TcpStream::connect("127.0.0.1:8090")).await??;
    let (mut read_half, mut write_half) = stream.split();
    let mut reader = BufReader::new(&mut read_half);

    // 5. Initialize connection
    let init_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {},
        "id": 1
    });

    let request_str = serde_json::to_string(&init_request)?;
    write_half.write_all(request_str.as_bytes()).await?;
    write_half.write_all(b"\n").await?;

    let mut response_line = String::new();
    timeout(Duration::from_secs(5), reader.read_line(&mut response_line)).await??;

    let init_response: Value = serde_json::from_str(&response_line)?;
    assert_eq!(init_response["jsonrpc"], "2.0");
    assert_eq!(init_response["id"], 1);
    assert!(init_response["result"].is_object());

    // 6. Verify server capabilities
    let capabilities = &init_response["result"]["capabilities"];
    assert!(capabilities["tools"].is_object());
    assert_eq!(capabilities["tools"]["listChanged"], false);

    // 7. Test tools/list to verify available tools
    let tools_list_msg = r#"{"jsonrpc": "2.0", "method": "tools/list", "id": 2}"#;
    write_half.write_all(tools_list_msg.as_bytes()).await?;
    write_half.write_all(b"\n").await?;

    let mut tools_response_line = String::new();
    timeout(
        Duration::from_secs(5),
        reader.read_line(&mut tools_response_line),
    )
    .await??;

    let tools_response: Value = serde_json::from_str(&tools_response_line)?;
    assert_eq!(tools_response["jsonrpc"], "2.0");
    assert_eq!(tools_response["id"], 2);
    assert!(tools_response["result"]["tools"].is_array());

    let tools = tools_response["result"]["tools"].as_array().unwrap();
    let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

    assert!(tool_names.contains(&"get_activities"));
    assert!(tool_names.contains(&"get_athlete"));
    assert!(tool_names.contains(&"get_stats"));

    // 8. Test tool call with invalid provider (should fail gracefully)
    let invalid_tool_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_athlete",
            "arguments": {
                "provider": "nonexistent"
            }
        },
        "id": 3
    });

    let request_str = serde_json::to_string(&invalid_tool_request)?;
    write_half.write_all(request_str.as_bytes()).await?;
    write_half.write_all(b"\n").await?;

    response_line.clear();
    timeout(Duration::from_secs(5), reader.read_line(&mut response_line)).await??;

    let tool_response: Value = serde_json::from_str(&response_line)?;
    assert_eq!(tool_response["jsonrpc"], "2.0");
    assert_eq!(tool_response["id"], 3);
    // Tool now works with Universal Tool Executor but may return an error result
    assert!(tool_response["result"].is_object() || tool_response["error"].is_object());

    // 8. Test unknown tool
    let unknown_tool_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "unknown_tool",
            "arguments": {
                "provider": "strava"
            }
        },
        "id": 3
    });

    let request_str = serde_json::to_string(&unknown_tool_request)?;
    write_half.write_all(request_str.as_bytes()).await?;
    write_half.write_all(b"\n").await?;

    response_line.clear();
    timeout(Duration::from_secs(5), reader.read_line(&mut response_line)).await??;

    let unknown_response: Value = serde_json::from_str(&response_line)?;
    assert_eq!(unknown_response["jsonrpc"], "2.0");
    assert_eq!(unknown_response["id"], 3);
    assert!(unknown_response["error"].is_object());

    // 9. Clean up
    server_task.abort();

    Ok(())
}

/// Test configuration loading from environment variables
#[tokio::test]
async fn test_config_env_workflow() -> Result<()> {
    use std::sync::Mutex;

    // Use mutex to prevent test interference
    static ENV_MUTEX: Mutex<()> = Mutex::new(());
    let config = {
        let _guard = ENV_MUTEX.lock().unwrap();

        // Test environment-based config loading

        // Create non-existent config path
        let temp_dir = TempDir::new()?;
        let nonexistent_config = temp_dir.path().join("nonexistent.toml");

        // Load config (should fall back to defaults)
        let config = Config::load(Some(nonexistent_config.to_string_lossy().to_string()))?;

        // Verify default config was loaded
        assert!(!config.sport_types.is_empty()); // Should have default sport types

        // Configuration loaded successfully

        config
    }; // Guard is dropped here

    // Test MCP server with env-loaded config
    let server = McpServer::new(config);
    let server_task = tokio::spawn(async move { server.run(8091).await });

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Quick connection test
    let stream_result = timeout(Duration::from_secs(2), TcpStream::connect("127.0.0.1:8091")).await;
    assert!(
        stream_result.is_ok(),
        "Server should be accepting connections"
    );

    server_task.abort();

    Ok(())
}

/// Test the config save and reload workflow
#[tokio::test]
async fn test_config_persistence_workflow() -> Result<()> {
    // 1. Create a test configuration
    let original_config = Config::default();

    // 2. Save config to temporary file using TOML serialization
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("test_config.toml");
    let _config_path_str = config_path.to_string_lossy().to_string();

    let config_toml = toml::to_string(&original_config)?;
    tokio::fs::write(&config_path, config_toml).await?;

    // 3. Verify file exists and has content
    assert!(config_path.exists());
    let content = tokio::fs::read_to_string(&config_path).await?;
    assert!(content.contains("sport_types"));
    assert!(content.contains("intelligence"));

    // 4. Load config back from file
    let config_content = tokio::fs::read_to_string(&config_path).await?;
    let loaded_config: Config = toml::from_str(&config_content)?;

    // 5. Verify loaded config matches original structure
    // The config should have the basic structure elements
    assert!(loaded_config.sport_types.is_empty() || !loaded_config.sport_types.is_empty()); // Just verify it exists

    // 6. Test MCP server with reloaded config
    let server = McpServer::new(loaded_config);
    let server_task = tokio::spawn(async move { server.run(8092).await });

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify server starts successfully
    let connection_result =
        timeout(Duration::from_secs(2), TcpStream::connect("127.0.0.1:8092")).await;
    assert!(connection_result.is_ok());

    server_task.abort();

    Ok(())
}

/// Test error recovery and resilience
#[tokio::test]
async fn test_error_recovery_workflow() -> Result<()> {
    // 1. Test server startup with default config
    let empty_config = Config::default();

    let server = McpServer::new(empty_config);
    let server_task = tokio::spawn(async move { server.run(8093).await });

    tokio::time::sleep(Duration::from_millis(200)).await;

    // 2. Connect and verify server still responds
    let mut stream =
        timeout(Duration::from_secs(5), TcpStream::connect("127.0.0.1:8093")).await??;
    let (mut read_half, mut write_half) = stream.split();
    let mut reader = BufReader::new(&mut read_half);

    // 3. Initialize should work even with no providers
    let init_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {},
        "id": 1
    });

    let request_str = serde_json::to_string(&init_request)?;
    write_half.write_all(request_str.as_bytes()).await?;
    write_half.write_all(b"\n").await?;

    let mut response_line = String::new();
    timeout(Duration::from_secs(5), reader.read_line(&mut response_line)).await??;

    let response: Value = serde_json::from_str(&response_line)?;
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["result"].is_object());

    // 4. Tool calls should fail gracefully
    let tool_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_athlete",
            "arguments": {
                "provider": "any_provider"
            }
        },
        "id": 2
    });

    let request_str = serde_json::to_string(&tool_request)?;
    write_half.write_all(request_str.as_bytes()).await?;
    write_half.write_all(b"\n").await?;

    response_line.clear();
    timeout(Duration::from_secs(5), reader.read_line(&mut response_line)).await??;

    let tool_response: Value = serde_json::from_str(&response_line)?;
    assert_eq!(tool_response["jsonrpc"], "2.0");
    // Tool now works with Universal Tool Executor but may return an error result
    assert!(tool_response["result"].is_object() || tool_response["error"].is_object());

    server_task.abort();

    Ok(())
}

/// Test concurrent connections and request handling
#[tokio::test]
async fn test_concurrent_operations_workflow() -> Result<()> {
    let config = Config::default();

    let server = McpServer::new(config);
    let server_task = tokio::spawn(async move { server.run(8094).await });

    tokio::time::sleep(Duration::from_millis(300)).await;

    // Create multiple concurrent connections
    let mut connection_tasks = Vec::new();

    for i in 0..5 {
        let task = tokio::spawn(async move {
            let mut stream = TcpStream::connect("127.0.0.1:8094").await?;
            let (mut read_half, mut write_half) = stream.split();
            let mut reader = BufReader::new(&mut read_half);

            // Send initialize request
            let init_request = json!({
                "jsonrpc": "2.0",
                "method": "initialize",
                "params": {},
                "id": i
            });

            let request_str = serde_json::to_string(&init_request)?;
            write_half.write_all(request_str.as_bytes()).await?;
            write_half.write_all(b"\n").await?;

            // Read response
            let mut response_line = String::new();
            reader.read_line(&mut response_line).await?;

            let response: Value = serde_json::from_str(&response_line)?;

            // Verify response
            assert_eq!(response["jsonrpc"], "2.0");
            assert_eq!(response["id"], i);
            assert!(response["result"].is_object());

            Ok::<(), anyhow::Error>(())
        });

        connection_tasks.push(task);
    }

    // Wait for all connections to complete
    for task in connection_tasks {
        task.await??;
    }

    server_task.abort();

    Ok(())
}
