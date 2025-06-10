// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::config::Config;
use crate::providers::{FitnessProvider, create_provider, AuthData};

// MCP Protocol Constants
const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
const JSONRPC_VERSION: &str = "2.0";

// Server Information
const SERVER_NAME: &str = "pierre-mcp-server";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

// JSON-RPC Error Codes (as defined in the JSON-RPC 2.0 specification)
const ERROR_METHOD_NOT_FOUND: i32 = -32601;
const ERROR_INVALID_PARAMS: i32 = -32602;
const ERROR_INTERNAL_ERROR: i32 = -32603;

/// Generate MCP tool schemas for supported fitness providers
fn generate_tool_schemas() -> Value {
    serde_json::json!([
        {
            "name": "get_activities",
            "description": "Get fitness activities from a provider",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "provider": { 
                        "type": "string",
                        "description": "Fitness provider name (e.g., 'strava', 'fitbit')"
                    },
                    "limit": { 
                        "type": "number",
                        "description": "Maximum number of activities to return"
                    },
                    "offset": { 
                        "type": "number",
                        "description": "Number of activities to skip (for pagination)"
                    }
                },
                "required": ["provider"]
            }
        },
        {
            "name": "get_athlete",
            "description": "Get athlete profile from a provider",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "provider": { 
                        "type": "string",
                        "description": "Fitness provider name (e.g., 'strava', 'fitbit')"
                    }
                },
                "required": ["provider"]
            }
        },
        {
            "name": "get_stats",
            "description": "Get fitness statistics from a provider",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "provider": { 
                        "type": "string",
                        "description": "Fitness provider name (e.g., 'strava', 'fitbit')"
                    }
                },
                "required": ["provider"]
            }
        }
    ])
}

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
                        let response = handle_request(request, &providers, &config).await;
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
    providers: &Arc<RwLock<HashMap<String, Box<dyn FitnessProvider>>>>,
    config: &Config,
) -> McpResponse {
    match request.method.as_str() {
        "initialize" => {
            McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: Some(serde_json::json!({
                    "protocolVersion": MCP_PROTOCOL_VERSION,
                    "serverInfo": {
                        "name": SERVER_NAME,
                        "version": SERVER_VERSION
                    },
                    "capabilities": {
                        "tools": generate_tool_schemas()
                    }
                })),
                error: None,
                id: request.id,
            }
        }
        "tools/call" => {
            let params = request.params.unwrap_or_default();
            let tool_name = params["name"].as_str().unwrap_or("");
            let args = &params["arguments"];
            
            handle_tool_call(tool_name, args, providers, config, request.id).await
        }
        _ => {
            McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: None,
                error: Some(McpError {
                    code: ERROR_METHOD_NOT_FOUND,
                    message: "Method not found".to_string(),
                    data: None,
                }),
                id: request.id,
            }
        }
    }
}

async fn handle_tool_call(
    tool_name: &str,
    args: &Value,
    providers: &Arc<RwLock<HashMap<String, Box<dyn FitnessProvider>>>>,
    config: &Config,
    id: Value,
) -> McpResponse {
    let provider_name = args["provider"].as_str().unwrap_or("");
    
    let mut providers_write = providers.write().await;
    if !providers_write.contains_key(provider_name) {
        match create_provider(provider_name) {
            Ok(mut provider) => {
                if let Some(auth_config) = config.providers.get(provider_name) {
                    let auth_data = match &auth_config.auth_type[..] {
                        "oauth2" => AuthData::OAuth2 {
                            client_id: auth_config.client_id.clone().unwrap_or_default(),
                            client_secret: auth_config.client_secret.clone().unwrap_or_default(),
                            access_token: auth_config.access_token.clone(),
                            refresh_token: auth_config.refresh_token.clone(),
                        },
                        "api_key" => AuthData::ApiKey(auth_config.api_key.clone().unwrap_or_default()),
                        _ => {
                            return McpResponse {
                                jsonrpc: JSONRPC_VERSION.to_string(),
                                result: None,
                                error: Some(McpError {
                                    code: ERROR_INVALID_PARAMS,
                                    message: "Invalid auth configuration".to_string(),
                                    data: None,
                                }),
                                id,
                            };
                        }
                    };
                    
                    if let Err(e) = provider.authenticate(auth_data).await {
                        return McpResponse {
                            jsonrpc: JSONRPC_VERSION.to_string(),
                            result: None,
                            error: Some(McpError {
                                code: ERROR_INTERNAL_ERROR,
                                message: format!("Authentication failed: {}", e),
                                data: None,
                            }),
                            id,
                        };
                    }
                }
                providers_write.insert(provider_name.to_string(), provider);
            }
            Err(e) => {
                return McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_INVALID_PARAMS,
                        message: format!("Invalid provider: {}", e),
                        data: None,
                    }),
                    id,
                };
            }
        }
    }
    drop(providers_write);
    
    let providers_read = providers.read().await;
    let provider = providers_read.get(provider_name).unwrap();
    
    let result = match tool_name {
        "get_activities" => {
            let limit = args["limit"].as_u64().map(|n| n as usize);
            let offset = args["offset"].as_u64().map(|n| n as usize);
            
            match provider.get_activities(limit, offset).await {
                Ok(activities) => serde_json::to_value(activities).ok(),
                Err(e) => {
                    return McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_string(),
                        result: None,
                        error: Some(McpError {
                            code: ERROR_INTERNAL_ERROR,
                            message: format!("Failed to get activities: {}", e),
                            data: None,
                        }),
                        id,
                    };
                }
            }
        }
        "get_athlete" => {
            match provider.get_athlete().await {
                Ok(athlete) => serde_json::to_value(athlete).ok(),
                Err(e) => {
                    return McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_string(),
                        result: None,
                        error: Some(McpError {
                            code: ERROR_INTERNAL_ERROR,
                            message: format!("Failed to get athlete: {}", e),
                            data: None,
                        }),
                        id,
                    };
                }
            }
        }
        "get_stats" => {
            match provider.get_stats().await {
                Ok(stats) => serde_json::to_value(stats).ok(),
                Err(e) => {
                    return McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_string(),
                        result: None,
                        error: Some(McpError {
                            code: ERROR_INTERNAL_ERROR,
                            message: format!("Failed to get stats: {}", e),
                            data: None,
                        }),
                        id,
                    };
                }
            }
        }
        _ => None,
    };
    
    McpResponse {
        jsonrpc: JSONRPC_VERSION.to_string(),
        result: result.clone(),
        error: if result.is_none() {
            Some(McpError {
                code: ERROR_METHOD_NOT_FOUND,
                message: "Unknown tool".to_string(),
                data: None,
            })
        } else {
            None
        },
        id,
    }
}