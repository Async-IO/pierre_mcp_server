// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

pub mod multitenant;
pub mod schema;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::config::Config;
use crate::constants::{
    errors::*,
    json_fields::*,
    protocol,
    protocol::{JSONRPC_VERSION, SERVER_VERSION},
};
use crate::database_plugins::factory::Database;
use crate::intelligence::{
    ActivityIntelligence, ContextualFactors, PerformanceMetrics, TimeOfDay, TrendDirection,
    TrendIndicators,
};
use crate::mcp::schema::InitializeResponse;
use crate::protocols::universal::{UniversalRequest, UniversalToolExecutor};
use crate::providers::FitnessProvider;

pub struct McpServer {
    config: Config,
    providers: Arc<RwLock<HashMap<String, Box<dyn FitnessProvider>>>>,
}

impl McpServer {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            providers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn run(self, port: u16) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
        info!("MCP server listening on port {}", port);

        loop {
            let (socket, addr) = listener.accept().await?;
            info!("New connection from {}", addr);

            let providers = self.providers.clone();
            let config = self.config.clone();

            tokio::spawn(async move {
                let (reader, mut writer) = socket.into_split();
                let mut reader = BufReader::new(reader);
                let mut line = String::new();

                while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                    if let Ok(request) = serde_json::from_str::<McpRequest>(&line) {
                        // Create tool executor only when needed (not for initialize)
                        let tool_executor = if request.method == "tools/call" {
                            match create_tool_executor().await {
                                Ok(executor) => Some(executor),
                                Err(e) => {
                                    tracing::error!("Failed to create tool executor: {}", e);
                                    None
                                }
                            }
                        } else {
                            None
                        };

                        let response =
                            handle_request(request, &providers, &config, tool_executor.as_ref())
                                .await;
                        let response_str = serde_json::to_string(&response).unwrap();
                        writer.write_all(response_str.as_bytes()).await.ok();
                        writer.write_all(b"\n").await.ok();
                    }
                    line.clear();
                }
            });
        }
    }
}

/// Create a tool executor for MCP server with proper configuration
async fn create_tool_executor() -> Result<Arc<UniversalToolExecutor>> {
    // Use environment variables for database configuration in production
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "data/pierre.db".to_string());

    // Load or generate encryption key
    let encryption_key = if let Ok(key_path) = std::env::var("ENCRYPTION_KEY_PATH") {
        std::fs::read(&key_path).map_err(|e| {
            anyhow::anyhow!("Failed to read encryption key from {}: {}", key_path, e)
        })?
    } else {
        // For backward compatibility, use a default key file path
        let key_path = "data/encryption.key";
        if std::path::Path::new(key_path).exists() {
            std::fs::read(key_path).map_err(|e| {
                anyhow::anyhow!("Failed to read encryption key from {}: {}", key_path, e)
            })?
        } else {
            // Generate a new key and save it
            let key = crate::database::generate_encryption_key();
            if let Some(parent) = std::path::Path::new(key_path).parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(key_path, key)?;
            tracing::info!("Generated new encryption key: {}", key_path);
            key.to_vec()
        }
    };

    let database = Arc::new(
        Database::new(&database_url, encryption_key)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create database connection: {}", e))?,
    );

    let intelligence = Arc::new(ActivityIntelligence::new(
        "Basic MCP Intelligence".to_string(),
        vec![],
        PerformanceMetrics {
            relative_effort: Some(7.5),
            zone_distribution: None,
            personal_records: vec![],
            efficiency_score: Some(85.0),
            trend_indicators: TrendIndicators {
                pace_trend: TrendDirection::Stable,
                effort_trend: TrendDirection::Improving,
                distance_trend: TrendDirection::Stable,
                consistency_score: 88.0,
            },
        },
        ContextualFactors {
            weather: None,
            location: None,
            time_of_day: TimeOfDay::Morning,
            days_since_last_activity: Some(1),
            weekly_load: None,
        },
    ));

    Ok(Arc::new(UniversalToolExecutor::new(database, intelligence)))
}

#[derive(Debug, Deserialize)]
struct McpRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    method: String,
    params: Option<Value>,
    id: Value,
}

#[derive(Debug, Serialize)]
struct McpResponse {
    jsonrpc: String,
    result: Option<Value>,
    error: Option<McpError>,
    id: Value,
}

#[derive(Debug, Serialize)]
struct McpError {
    code: i32,
    message: String,
    data: Option<Value>,
}

async fn handle_request(
    request: McpRequest,
    _providers: &Arc<RwLock<HashMap<String, Box<dyn FitnessProvider>>>>,
    _config: &Config,
    tool_executor: Option<&Arc<UniversalToolExecutor>>,
) -> McpResponse {
    match request.method.as_str() {
        "initialize" => {
            let init_response = InitializeResponse::new(
                protocol::mcp_protocol_version(),
                protocol::server_name(),
                SERVER_VERSION.to_string(),
            );

            McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: serde_json::to_value(&init_response).ok(),
                error: None,
                id: request.id,
            }
        }
        "tools/call" => {
            if let Some(executor) = tool_executor {
                let params = request.params.unwrap_or_default();
                let tool_name = params[NAME].as_str().unwrap_or("");
                let args = &params[ARGUMENTS];

                handle_tool_call_unified(tool_name, args, executor, request.id).await
            } else {
                McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_INTERNAL_ERROR,
                        message: "Tool executor not available".to_string(),
                        data: None,
                    }),
                    id: request.id,
                }
            }
        }
        _ => McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: None,
            error: Some(McpError {
                code: ERROR_METHOD_NOT_FOUND,
                message: "Method not found".to_string(),
                data: None,
            }),
            id: request.id,
        },
    }
}

async fn handle_tool_call_unified(
    tool_name: &str,
    args: &Value,
    tool_executor: &Arc<UniversalToolExecutor>,
    id: Value,
) -> McpResponse {
    // Create a default user ID for basic MCP server (single-user scenario)
    let user_id = uuid::Uuid::new_v4().to_string();

    // Create UniversalRequest from MCP request
    let universal_request = UniversalRequest {
        user_id,
        tool_name: tool_name.to_string(),
        parameters: args.clone(),
        protocol: "mcp".to_string(),
    };

    // Execute tool using Universal Tool Executor
    match tool_executor.execute_tool(universal_request).await {
        Ok(universal_response) => McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: universal_response.result,
            error: None,
            id,
        },
        Err(protocol_error) => {
            let (error_code, error_message) = match protocol_error {
                crate::protocols::ProtocolError::ToolNotFound(msg) => (ERROR_METHOD_NOT_FOUND, msg),
                crate::protocols::ProtocolError::InvalidParameters(msg) => {
                    (ERROR_INVALID_PARAMS, msg)
                }
                crate::protocols::ProtocolError::ExecutionFailed(msg) => {
                    (ERROR_INTERNAL_ERROR, msg)
                }
                crate::protocols::ProtocolError::UnsupportedProtocol(msg) => {
                    (ERROR_INTERNAL_ERROR, msg)
                }
                crate::protocols::ProtocolError::ConversionFailed(msg) => {
                    (ERROR_INTERNAL_ERROR, msg)
                }
                crate::protocols::ProtocolError::ConfigurationError(msg) => {
                    (ERROR_INTERNAL_ERROR, msg)
                }
            };

            McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: None,
                error: Some(McpError {
                    code: error_code,
                    message: error_message,
                    data: None,
                }),
                id,
            }
        }
    }
}
