// ABOUTME: MCP client for Pierre Fitness API server
// ABOUTME: Handles JWT authentication and stdio transport for Claude Desktop integration

use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::env;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as AsyncBufReader};

#[derive(Debug)]
struct Config {
    server_url: String,
    jwt_token: Option<String>,
    timeout_seconds: u64,
}

impl Config {
    fn from_env() -> Self {
        let server_url =
            env::var("PIERRE_MCP_URL").unwrap_or_else(|_| "http://127.0.0.1:8080/mcp".to_string());

        let jwt_token = env::var("PIERRE_JWT_TOKEN").ok();

        let timeout_seconds = env::var("PIERRE_MCP_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        Self {
            server_url,
            jwt_token,
            timeout_seconds,
        }
    }

    fn validate(&self) -> Result<()> {
        if self.jwt_token.is_none() {
            anyhow::bail!("PIERRE_JWT_TOKEN environment variable is required");
        }
        Ok(())
    }
}

struct McpClient {
    client: Client,
    config: Config,
}

impl McpClient {
    fn new(config: Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client, config })
    }

    async fn send_request(&self, request_json: &str) -> Result<String> {
        let mut http_request = self
            .client
            .post(&self.config.server_url)
            .header("Content-Type", "application/json")
            .header("User-Agent", "pierre-mcp-client/0.1.0");

        if let Some(token) = &self.config.jwt_token {
            http_request = http_request.header("Authorization", format!("Bearer {token}"));
        }

        let response = http_request
            .body(request_json.to_string())
            .send()
            .await
            .context("Failed to send request to Pierre MCP server")?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .context("Failed to read response from Pierre MCP server")?;

        if !status.is_success() {
            // Try to parse error response
            if let Ok(error_json) = serde_json::from_str::<Value>(&response_text) {
                if let Some(error) = error_json.get("error") {
                    anyhow::bail!("Server error: {}", error);
                }
            }
            anyhow::bail!("HTTP error {}: {}", status, response_text);
        }

        Ok(response_text)
    }

    fn create_error_response(request_id: Option<i64>, code: i32, message: &str) -> String {
        let id = request_id.unwrap_or(0);
        serde_json::json!({
            "jsonrpc": "2.0",
            "error": {
                "code": code,
                "message": message
            },
            "id": id
        })
        .to_string()
    }

    fn extract_request_id(json_line: &str) -> Option<i64> {
        serde_json::from_str::<Value>(json_line)
            .ok()?
            .get("id")?
            .as_i64()
    }

    async fn run_stdio_transport(&self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let reader = AsyncBufReader::new(stdin);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await? {
            let trimmed_line = line.trim();
            if trimmed_line.is_empty() {
                continue;
            }

            let request_id = Self::extract_request_id(trimmed_line);

            // Validate JSON-RPC format
            if let Err(e) = serde_json::from_str::<Value>(trimmed_line) {
                let error_response =
                    Self::create_error_response(request_id, -32700, &format!("Parse error: {e}"));
                stdout.write_all(error_response.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
                continue;
            }

            match self.send_request(trimmed_line).await {
                Ok(response) => {
                    // Ensure response is properly formatted JSON
                    if !response.trim().is_empty() {
                        stdout.write_all(response.trim().as_bytes()).await?;
                        stdout.write_all(b"\n").await?;
                        stdout.flush().await?;
                    }
                }
                Err(e) => {
                    let error_response = Self::create_error_response(
                        request_id,
                        -32603,
                        &format!("Internal error: {e}"),
                    );
                    stdout.write_all(error_response.as_bytes()).await?;
                    stdout.write_all(b"\n").await?;
                    stdout.flush().await?;
                }
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Config::from_env();

    // Validate configuration
    config.validate().context("Invalid configuration")?;

    // Create and run client
    let client = McpClient::new(config).context("Failed to create MCP client")?;

    client
        .run_stdio_transport()
        .await
        .context("MCP client transport failed")?;

    Ok(())
}
