// ABOUTME: MCP client bridge for Pierre Fitness API server
// ABOUTME: Provides stdio MCP server interface that forwards MCP JSON-RPC over HTTP to pierre-mcp-server

use anyhow::{Context, Result};
use serde_json::Value;
use std::env;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Clone)]
struct Config {
    server_host: String,
    server_port: u16,
    jwt_token: String,
}

impl Config {
    fn from_env() -> Result<Self> {
        let server_host =
            env::var("PIERRE_SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

        let server_port = env::var("PIERRE_SERVER_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8080);

        let jwt_token = env::var("PIERRE_JWT_TOKEN")
            .context("PIERRE_JWT_TOKEN environment variable is required")?;

        Ok(Self {
            server_host,
            server_port,
            jwt_token,
        })
    }
}

struct McpBridge {
    config: Config,
}

impl McpBridge {
    const fn new(config: Config) -> Self {
        Self { config }
    }

    /// Forward MCP JSON-RPC request over HTTP to pierre-mcp-server
    async fn forward_mcp_request(&self, request: &Value) -> Result<Value> {
        // Build HTTP client
        let client = reqwest::Client::new();
        let server_url = format!(
            "http://{}:{}/mcp",
            self.config.server_host, self.config.server_port
        );

        // Add authentication to the request
        let mut authenticated_request = request.clone();
        if let Some(params) = authenticated_request.get_mut("params") {
            if let Some(params_obj) = params.as_object_mut() {
                params_obj.insert(
                    "token".to_string(),
                    serde_json::Value::String(self.config.jwt_token.clone()),
                );
            }
        } else {
            // Add params with token if it doesn't exist
            authenticated_request["params"] = serde_json::json!({
                "token": self.config.jwt_token
            });
        }

        // Send the MCP request as JSON-RPC over HTTP POST
        let response = client
            .post(&server_url)
            .header("Content-Type", "application/json")
            .json(&authenticated_request)
            .send()
            .await
            .with_context(|| format!("Failed to send request to MCP server at {server_url}"))?;

        if !response.status().is_success() {
            anyhow::bail!("MCP server returned error status: {}", response.status());
        }

        // Handle notifications (HTTP 204 No Content) - they should have no response in JSON-RPC
        if response.status() == reqwest::StatusCode::NO_CONTENT {
            return Ok(serde_json::Value::Null);
        }

        // Parse JSON response for regular requests
        let response_json: Value = response
            .json()
            .await
            .with_context(|| "Failed to parse MCP server response")?;

        Ok(response_json)
    }

    /// Main stdio MCP bridge loop
    async fn run_stdio_bridge(&self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut stdout = tokio::io::stdout();

        let mut line = String::new();

        loop {
            line.clear();

            // Read JSON-RPC request from stdin (from Claude Desktop)
            match reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    // Parse MCP request
                    match serde_json::from_str::<Value>(line) {
                        Ok(request) => {
                            // Forward to MCP server over TCP
                            match self.forward_mcp_request(&request).await {
                                Ok(response) => {
                                    // For notifications (Value::Null), don't send anything back to Claude Desktop
                                    if !response.is_null() {
                                        // Send response back to Claude Desktop via stdout
                                        let response_line = serde_json::to_string(&response)?;
                                        stdout.write_all(response_line.as_bytes()).await?;
                                        stdout.write_all(b"\n").await?;
                                        stdout.flush().await?;
                                    }
                                }
                                Err(e) => {
                                    // Send error response
                                    let error_response = serde_json::json!({
                                        "jsonrpc": "2.0",
                                        "id": request.get("id"),
                                        "error": {
                                            "code": -32603,
                                            "message": format!("Bridge error: {e}")
                                        }
                                    });
                                    let response_line = serde_json::to_string(&error_response)?;
                                    stdout.write_all(response_line.as_bytes()).await?;
                                    stdout.write_all(b"\n").await?;
                                    stdout.flush().await?;
                                }
                            }
                        }
                        Err(e) => {
                            // Send parse error response
                            let error_response = serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": null,
                                "error": {
                                    "code": -32700,
                                    "message": format!("Parse error: {e}")
                                }
                            });
                            let response_line = serde_json::to_string(&error_response)?;
                            stdout.write_all(response_line.as_bytes()).await?;
                            stdout.write_all(b"\n").await?;
                            stdout.flush().await?;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading stdin: {e}");
                    break;
                }
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Config::from_env().context("Failed to load configuration")?;

    // Create MCP bridge
    let bridge = McpBridge::new(config);

    // Run stdio MCP bridge
    bridge
        .run_stdio_bridge()
        .await
        .context("MCP bridge failed")?;

    Ok(())
}
