// ABOUTME: MCP client binary that connects Claude Desktop to Pierre MCP Server
// ABOUTME: Lightweight client with no database access - communicates via HTTP with running server

//! # Pierre MCP Client Binary
//!
//! This binary provides an MCP interface for Claude Desktop and other MCP clients.
//! It connects to a running Pierre MCP Server via HTTP and translates MCP protocol
//! calls to HTTP API calls. No database access - completely stateless client.

use anyhow::{Context, Result};
use clap::Parser;
use serde_json::{json, Value};
use std::env;
use std::io::{self, BufRead, Write};
use tracing::{debug, error, info, warn};

#[derive(Parser)]
#[command(name = "pierre-mcp-client")]
#[command(about = "Pierre MCP Client - Connects Claude Desktop to Pierre MCP Server")]
pub struct Args {
    /// Pierre MCP Server URL
    #[arg(long, default_value = "http://localhost:8081")]
    server_url: String,

    /// Enable debug logging
    #[arg(long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    if args.debug {
        tracing_subscriber::fmt().with_env_filter("debug").init();
    } else {
        tracing_subscriber::fmt().with_env_filter("warn").init();
    }

    // Get tenant credentials from environment
    let tenant_id = env::var("TENANT_ID").context("TENANT_ID environment variable is required")?;
    let jwt_token = env::var("TENANT_JWT_TOKEN")
        .context("TENANT_JWT_TOKEN environment variable is required")?;

    info!("Pierre MCP Client starting");
    debug!("Server URL: {}", args.server_url);
    debug!("Tenant ID: {}", tenant_id);

    // Create HTTP client
    let client = reqwest::Client::new();

    // Test connection to server
    match client
        .get(format!("{}/health", args.server_url))
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => {
            info!("Successfully connected to Pierre MCP Server");
        }
        Ok(response) => {
            error!("Pierre MCP Server returned error: {}", response.status());
            return Err(anyhow::anyhow!(
                "Server health check failed: {}",
                response.status()
            ));
        }
        Err(e) => {
            error!("Failed to connect to Pierre MCP Server: {}", e);
            return Err(anyhow::anyhow!("Cannot connect to server: {}", e));
        }
    }

    // Start MCP protocol loop
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        debug!("Received MCP request: {}", line);

        // Parse JSON-RPC request
        let request: Value = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                error!("Invalid JSON-RPC request: {}", e);
                continue;
            }
        };

        // Handle the request
        let response =
            handle_mcp_request(&client, &args.server_url, &tenant_id, &jwt_token, request).await;

        // Send response
        let response_json = serde_json::to_string(&response)?;
        writeln!(stdout, "{response_json}")?;
        stdout.flush()?;

        debug!("Sent MCP response: {}", response_json);
    }

    Ok(())
}

async fn handle_mcp_request(
    client: &reqwest::Client,
    server_url: &str,
    tenant_id: &str,
    jwt_token: &str,
    request: Value,
) -> Value {
    let method = request["method"].as_str().unwrap_or("");
    let id = request["id"].clone();

    match method {
        "initialize" => {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "pierre-fitness",
                        "version": "1.0.0"
                    }
                }
            })
        }
        "tools/list" => match list_tools(client, server_url, tenant_id, jwt_token).await {
            Ok(tools) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": tools
                }
            }),
            Err(e) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32603,
                    "message": format!("Failed to list tools: {}", e)
                }
            }),
        },
        "tools/call" => {
            let tool_name = request["params"]["name"].as_str().unwrap_or("");
            let arguments = &request["params"]["arguments"];

            match call_tool(
                client, server_url, tenant_id, jwt_token, tool_name, arguments,
            )
            .await
            {
                Ok(result) => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
                            }
                        ]
                    }
                }),
                Err(e) => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32603,
                        "message": format!("Tool execution failed: {}", e)
                    }
                }),
            }
        }
        _ => {
            warn!("Unsupported MCP method: {}", method);
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": format!("Method not found: {}", method)
                }
            })
        }
    }
}

async fn list_tools(
    client: &reqwest::Client,
    server_url: &str,
    tenant_id: &str,
    jwt_token: &str,
) -> Result<Value> {
    let response = client
        .post(format!("{server_url}/mcp"))
        .header("Authorization", format!("Bearer {jwt_token}"))
        .header("X-Tenant-ID", tenant_id)
        .header("Content-Type", "application/json")
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 1
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Server returned error: {}",
            response.status()
        ));
    }

    let result: Value = response.json().await?;
    Ok(result["result"]["tools"].clone())
}

async fn call_tool(
    client: &reqwest::Client,
    server_url: &str,
    tenant_id: &str,
    jwt_token: &str,
    tool_name: &str,
    arguments: &Value,
) -> Result<Value> {
    let response = client
        .post(format!("{server_url}/mcp"))
        .header("Authorization", format!("Bearer {jwt_token}"))
        .header("X-Tenant-ID", tenant_id)
        .header("Content-Type", "application/json")
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            },
            "id": 1
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Server returned error: {}",
            response.status()
        ));
    }

    let result: Value = response.json().await?;
    if let Some(error) = result.get("error") {
        return Err(anyhow::anyhow!("Server error: {}", error));
    }

    Ok(result["result"].clone())
}
