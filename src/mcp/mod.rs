// ABOUTME: Model Context Protocol (MCP) implementation for AI assistant integration
// ABOUTME: Provides MCP server functionality for Claude, ChatGPT, and other AI assistants
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

pub mod multitenant;
pub mod schema;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::config::fitness_config::FitnessConfig as Config;
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
        std::fs::read(&key_path)
            .with_context(|| format!("Failed to read encryption key from {}", key_path))?
    } else {
        // For backward compatibility, use a default key file path
        let key_path = "data/encryption.key";
        if std::path::Path::new(key_path).exists() {
            std::fs::read(key_path).with_context(|| {
                format!("Failed to read default encryption key from {}", key_path)
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
            .with_context(|| format!("Failed to create database connection to {}", database_url))?,
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

    // Create a minimal ServerConfig from environment variables for single-tenant mode
    let config = Arc::new(
        crate::config::environment::ServerConfig::from_env().unwrap_or_else(|_| {
            // Fallback config if environment loading fails
            crate::config::environment::ServerConfig {
                mcp_port: 3000,
                http_port: 4000,
                log_level: crate::config::environment::LogLevel::Info,
                database: crate::config::environment::DatabaseConfig {
                    url: crate::config::environment::DatabaseUrl::default(),
                    encryption_key_path: std::path::PathBuf::from("data/encryption.key"),
                    auto_migrate: true,
                    backup: crate::config::environment::BackupConfig {
                        enabled: false,
                        interval_seconds: 3600,
                        retention_count: 7,
                        directory: std::path::PathBuf::from("data/backups"),
                    },
                },
                auth: crate::config::environment::AuthConfig {
                    jwt_secret_path: std::path::PathBuf::from("data/jwt.secret"),
                    jwt_expiry_hours: 24,
                    enable_refresh_tokens: false,
                },
                oauth: crate::config::environment::OAuthConfig {
                    strava: crate::config::environment::OAuthProviderConfig {
                        client_id: std::env::var("STRAVA_CLIENT_ID").ok(),
                        client_secret: std::env::var("STRAVA_CLIENT_SECRET").ok(),
                        redirect_uri: std::env::var("STRAVA_REDIRECT_URI").ok(),
                        scopes: vec!["read".to_string(), "activity:read_all".to_string()],
                        enabled: true,
                    },
                    fitbit: crate::config::environment::OAuthProviderConfig {
                        client_id: std::env::var("FITBIT_CLIENT_ID").ok(),
                        client_secret: std::env::var("FITBIT_CLIENT_SECRET").ok(),
                        redirect_uri: std::env::var("FITBIT_REDIRECT_URI").ok(),
                        scopes: vec!["activity".to_string(), "profile".to_string()],
                        enabled: true,
                    },
                },
                security: crate::config::environment::SecurityConfig {
                    cors_origins: vec!["*".to_string()],
                    rate_limit: crate::config::environment::RateLimitConfig {
                        enabled: false,
                        requests_per_window: 100,
                        window_seconds: 60,
                    },
                    tls: crate::config::environment::TlsConfig {
                        enabled: false,
                        cert_path: None,
                        key_path: None,
                    },
                    headers: crate::config::environment::SecurityHeadersConfig {
                        environment: crate::config::environment::Environment::Development,
                    },
                },
                external_services: crate::config::environment::ExternalServicesConfig {
                    weather: crate::config::environment::WeatherServiceConfig {
                        api_key: std::env::var("OPENWEATHER_API_KEY").ok(),
                        base_url: "https://api.openweathermap.org/data/2.5".to_string(),
                        enabled: false,
                    },
                    geocoding: crate::config::environment::GeocodingServiceConfig {
                        base_url: "https://nominatim.openstreetmap.org".to_string(),
                        enabled: true,
                    },
                    strava_api: crate::config::environment::StravaApiConfig {
                        base_url: "https://www.strava.com/api/v3".to_string(),
                        auth_url: "https://www.strava.com/oauth/authorize".to_string(),
                        token_url: "https://www.strava.com/oauth/token".to_string(),
                    },
                    fitbit_api: crate::config::environment::FitbitApiConfig {
                        base_url: "https://api.fitbit.com".to_string(),
                        auth_url: "https://www.fitbit.com/oauth2/authorize".to_string(),
                        token_url: "https://api.fitbit.com/oauth2/token".to_string(),
                    },
                },
                app_behavior: crate::config::environment::AppBehaviorConfig {
                    max_activities_fetch: 100,
                    default_activities_limit: 20,
                    ci_mode: false,
                    protocol: crate::config::environment::ProtocolConfig {
                        mcp_version: "2024-11-05".to_string(),
                        server_name: "pierre-mcp-server".to_string(),
                        server_version: env!("CARGO_PKG_VERSION").to_string(),
                    },
                },
            }
        }),
    );

    Ok(Arc::new(UniversalToolExecutor::new(
        database,
        intelligence,
        config,
    )))
}

#[derive(Debug, Deserialize)]
struct McpRequest {
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
    // Validate JSON-RPC version
    if request.jsonrpc != crate::constants::protocol::JSONRPC_VERSION {
        return McpResponse {
            jsonrpc: crate::constants::protocol::JSONRPC_VERSION.into(),
            result: None,
            error: Some(McpError {
                code: -32600,
                message: format!(
                    "Invalid JSON-RPC version: expected '{}', got '{}'",
                    crate::constants::protocol::JSONRPC_VERSION,
                    request.jsonrpc
                ),
                data: None,
            }),
            id: request.id,
        };
    }

    match request.method.as_str() {
        "initialize" => {
            // Parse client capabilities from params if provided
            let _client_capabilities = request
                .params
                .as_ref()
                .and_then(|p| serde_json::from_value::<schema::InitializeRequest>(p.clone()).ok());

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
        "ping" => McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: Some(serde_json::json!({})),
            error: None,
            id: request.id,
        },
        "tools/list" => {
            let tools = schema::get_tools();
            McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: Some(serde_json::json!({
                    "tools": tools
                })),
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
        Ok(universal_response) => {
            // Convert to MCP-compliant tool response format
            let tool_response = schema::ToolResponse {
                content: vec![schema::Content::Text {
                    text: universal_response
                        .result
                        .as_ref()
                        .map(|v| serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string()))
                        .unwrap_or_else(|| "No result".to_string()),
                }],
                is_error: !universal_response.success,
                structured_content: universal_response.result,
            };

            McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: serde_json::to_value(tool_response).ok(),
                error: None,
                id,
            }
        }
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
