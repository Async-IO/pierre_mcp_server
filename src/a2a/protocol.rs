// ABOUTME: Core A2A protocol message handling and JSON-RPC implementation
// ABOUTME: Processes A2A protocol requests, tool execution, and task management for agent communication
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A2A Protocol Implementation
//!
//! Implements the core A2A (Agent-to-Agent) protocol for Pierre,
//! providing JSON-RPC 2.0 based communication between AI agents.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// A2A JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ARequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    pub id: Option<Value>,
}

/// A2A JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<A2AErrorResponse>,
    pub id: Option<Value>,
}

/// A2A Protocol Error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum A2AError {
    /// Invalid request parameters or format
    InvalidRequest(String),
    /// Authentication failed
    AuthenticationFailed(String),
    /// Client not registered
    ClientNotRegistered(String),
    /// Database operation failed
    DatabaseError(String),
    /// Internal server error
    InternalError(String),
    /// Client has been deactivated
    ClientDeactivated(String),
    /// Rate limit exceeded
    RateLimitExceeded(String),
    /// Session expired or invalid
    SessionExpired(String),
    /// Invalid session token
    InvalidSessionToken(String),
    /// Insufficient permissions
    InsufficientPermissions(String),
    /// Resource not found
    ResourceNotFound(String),
    /// Service temporarily unavailable
    ServiceUnavailable(String),
}

impl std::fmt::Display for A2AError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRequest(msg) => write!(f, "Invalid request: {msg}"),
            Self::AuthenticationFailed(msg) => write!(f, "Authentication failed: {msg}"),
            Self::ClientNotRegistered(msg) => write!(f, "Client not registered: {msg}"),
            Self::DatabaseError(msg) => write!(f, "Database error: {msg}"),
            Self::InternalError(msg) => write!(f, "Internal error: {msg}"),
            Self::ClientDeactivated(msg) => write!(f, "Client deactivated: {msg}"),
            Self::RateLimitExceeded(msg) => write!(f, "Rate limit exceeded: {msg}"),
            Self::SessionExpired(msg) => write!(f, "Session expired: {msg}"),
            Self::InvalidSessionToken(msg) => write!(f, "Invalid session token: {msg}"),
            Self::InsufficientPermissions(msg) => write!(f, "Insufficient permissions: {msg}"),
            Self::ResourceNotFound(msg) => write!(f, "Resource not found: {msg}"),
            Self::ServiceUnavailable(msg) => write!(f, "Service unavailable: {msg}"),
        }
    }
}

impl std::error::Error for A2AError {}

/// A2A Protocol Error Response for JSON-RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AErrorResponse {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl From<A2AError> for A2AErrorResponse {
    fn from(error: A2AError) -> Self {
        let (code, message) = match error {
            A2AError::InvalidRequest(msg) => (-32602, format!("Invalid params: {msg}")),
            A2AError::AuthenticationFailed(msg) => {
                (-32001, format!("Authentication failed: {msg}"))
            }
            A2AError::ClientNotRegistered(msg) => (-32003, format!("Client not registered: {msg}")),
            A2AError::DatabaseError(msg) => (-32000, format!("Database error: {msg}")),
            A2AError::InternalError(msg) => (-32603, format!("Internal error: {msg}")),
            A2AError::ClientDeactivated(msg) => (-32004, format!("Client deactivated: {msg}")),
            A2AError::RateLimitExceeded(msg) => (-32005, format!("Rate limit exceeded: {msg}")),
            A2AError::SessionExpired(msg) => (-32006, format!("Session expired: {msg}")),
            A2AError::InvalidSessionToken(msg) => (-32007, format!("Invalid session token: {msg}")),
            A2AError::InsufficientPermissions(msg) => {
                (-32008, format!("Insufficient permissions: {msg}"))
            }
            A2AError::ResourceNotFound(msg) => (-32009, format!("Resource not found: {msg}")),
            A2AError::ServiceUnavailable(msg) => (-32010, format!("Service unavailable: {msg}")),
        };

        Self {
            code,
            message,
            data: None,
        }
    }
}

/// A2A Message structure for agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AMessage {
    pub id: String,
    pub parts: Vec<MessagePart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,
}

/// A2A Message Part types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessagePart {
    #[serde(rename = "text")]
    Text { content: String },
    #[serde(rename = "data")]
    Data { content: Value },
    #[serde(rename = "file")]
    File {
        name: String,
        mime_type: String,
        content: String, // base64 encoded
    },
}

/// A2A Task structure for long-running operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ATask {
    pub id: String,
    pub status: TaskStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    // Additional fields for database compatibility
    pub client_id: String,
    pub task_type: String,
    pub input_data: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Task status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// A2A Protocol Server implementation
pub struct A2AServer {
    pub version: String,
    pub database: Option<std::sync::Arc<crate::database_plugins::factory::Database>>,
    pub intelligence: Option<std::sync::Arc<crate::intelligence::ActivityIntelligence>>,
    pub config: Option<std::sync::Arc<crate::config::environment::ServerConfig>>,
}

impl A2AServer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: crate::a2a::A2A_VERSION.to_string(),
            database: None,
            intelligence: None,
            config: None,
        }
    }

    #[must_use]
    pub fn new_with_dependencies(
        database: std::sync::Arc<crate::database_plugins::factory::Database>,
        intelligence: std::sync::Arc<crate::intelligence::ActivityIntelligence>,
    ) -> Self {
        Self {
            version: crate::a2a::A2A_VERSION.to_string(),
            database: Some(database),
            intelligence: Some(intelligence),
            config: None,
        }
    }

    #[must_use]
    pub fn new_with_full_dependencies(
        database: std::sync::Arc<crate::database_plugins::factory::Database>,
        intelligence: std::sync::Arc<crate::intelligence::ActivityIntelligence>,
        config: std::sync::Arc<crate::config::environment::ServerConfig>,
    ) -> Self {
        Self {
            version: crate::a2a::A2A_VERSION.to_string(),
            database: Some(database),
            intelligence: Some(intelligence),
            config: Some(config),
        }
    }

    /// Handle incoming A2A request
    pub async fn handle_request(&self, request: A2ARequest) -> A2AResponse {
        match request.method.as_str() {
            "a2a/initialize" => self.handle_initialize(request),
            "message/send" | "a2a/message/send" => Self::handle_message_send(request),
            "message/stream" | "a2a/message/stream" => Self::handle_message_stream(request),
            "tasks/create" | "a2a/tasks/create" => Self::handle_task_create(request),
            "tasks/get" | "a2a/tasks/get" => self.handle_task_get(request),
            "tasks/cancel" => Self::handle_task_cancel(request),
            "tasks/pushNotificationConfig/set" => Self::handle_push_notification_config(request),
            "a2a/tasks/list" => self.handle_task_list(request),
            "tools/list" | "a2a/tools/list" => Self::handle_tools_list(request),
            "tools/call" | "a2a/tools/call" => self.handle_tool_call(request).await,
            _ => Self::handle_unknown_method(request),
        }
    }

    fn handle_initialize(&self, request: A2ARequest) -> A2AResponse {
        let result = serde_json::json!({
            "version": self.version,
            "capabilities": [
                "message/send",
                "message/stream",
                "tasks/create",
                "tasks/get",
                "tasks/cancel",
                "tasks/pushNotificationConfig/set",
                "tools/list",
                "tools/call"
            ],
            "agent": {
                "name": "Pierre Fitness Intelligence Agent",
                "version": "1.0.0",
                "description": "AI-powered fitness data analysis and insights platform"
            }
        });

        A2AResponse {
            jsonrpc: "2.0".into(),
            result: Some(result),
            error: None,
            id: request.id,
        }
    }

    fn handle_message_send(request: A2ARequest) -> A2AResponse {
        // Message sending would forward requests to appropriate handlers
        A2AResponse {
            jsonrpc: "2.0".into(),
            result: Some(serde_json::json!({"status": "received"})),
            error: None,
            id: request.id,
        }
    }

    fn handle_message_stream(request: A2ARequest) -> A2AResponse {
        // Message streaming is intentionally not supported in this implementation
        // A2A protocol uses stateless request-response pattern for reliability
        A2AResponse {
            jsonrpc: "2.0".into(),
            result: Some(serde_json::json!({
                "status": "streaming_not_supported",
                "message": "Message streaming is not supported by design. A2A protocol uses stateless message delivery.",
                "alternative": "Use a2a/message/send for reliable message delivery",
                "reason": "Stateless design ensures better reliability and scalability"
            })),
            error: None,
            id: request.id,
        }
    }

    fn handle_task_create(request: A2ARequest) -> A2AResponse {
        let task_id = Uuid::new_v4().to_string();

        // Extract client_id and task_type from request parameters
        let params = request.params.as_ref().unwrap_or(&serde_json::Value::Null);
        let client_id = params
            .get("client_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let task_type = params
            .get("task_type")
            .or_else(|| params.get("type"))
            .and_then(|v| v.as_str())
            .unwrap_or("generic")
            .to_string();

        let task = A2ATask {
            id: task_id,
            status: TaskStatus::Pending,
            created_at: chrono::Utc::now(),
            completed_at: None,
            result: None,
            error: None,
            client_id,
            task_type,
            input_data: request.params.unwrap_or(serde_json::Value::Null),
            output_data: None,
            error_message: None,
            updated_at: chrono::Utc::now(),
        };

        match serde_json::to_value(task) {
            Ok(task_value) => A2AResponse {
                jsonrpc: "2.0".into(),
                result: Some(task_value),
                error: None,
                id: request.id,
            },
            Err(e) => A2AResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(A2AErrorResponse {
                    code: -32603,
                    message: "Internal error: Failed to serialize task".to_string(),
                    data: Some(serde_json::json!({
                        "error": e.to_string(),
                        "context": "Task serialization failed"
                    })),
                }),
                id: request.id,
            },
        }
    }

    fn handle_task_get(&self, request: A2ARequest) -> A2AResponse {
        // Validate task ID parameter
        if let Some(params) = request.params.as_ref() {
            if let Some(serde_json::Value::String(_task_id)) = params.get("task_id") {
                // Task ID is valid, continue processing
            } else {
                return A2AResponse {
                    jsonrpc: "2.0".into(),
                    result: None,
                    error: Some(A2AErrorResponse {
                        code: -32602,
                        message: "Invalid params: task_id must be a string".into(),
                        data: None,
                    }),
                    id: request.id,
                };
            }
        } else {
            return A2AResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(A2AErrorResponse {
                    code: -32602,
                    message: "Invalid params: task_id is required".into(),
                    data: None,
                }),
                id: request.id,
            };
        }

        // Query database for actual task data if available
        if self.database.is_some() {
            // Try to get task from database
            // Return error - task storage requires database implementation
            return A2AResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(A2AErrorResponse {
                    code: -32000,
                    message: "Task storage not implemented".into(),
                    data: None,
                }),
                id: request.id,
            };
        }

        // No database available - return error
        A2AResponse {
            jsonrpc: "2.0".into(),
            result: None,
            error: Some(A2AErrorResponse {
                code: -32000,
                message: "Database not available for task retrieval".into(),
                data: None,
            }),
            id: request.id,
        }
    }

    fn handle_task_list(&self, request: A2ARequest) -> A2AResponse {
        // Validate parameters exist
        if let Some(params) = request.params.as_ref() {
            // Parameters were provided, validate them
            tracing::debug!("Task list request with parameters: {:?}", params);
        }

        // Query database for actual tasks if available
        if self.database.is_none() {
            return A2AResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(A2AErrorResponse {
                    code: -32000,
                    message: "Database not available for task listing".into(),
                    data: None,
                }),
                id: request.id,
            };
        }

        // Task storage not implemented yet
        A2AResponse {
            jsonrpc: "2.0".into(),
            result: None,
            error: Some(A2AErrorResponse {
                code: -32000,
                message: "Task listing not implemented".into(),
                data: None,
            }),
            id: request.id,
        }
    }

    fn handle_tools_list(request: A2ARequest) -> A2AResponse {
        // Available tools would be sourced from the universal tool executor
        let tools = serde_json::json!([
            {
                "name": "get_activities",
                "description": "Retrieve user fitness activities",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "limit": {"type": "number", "description": "Number of activities to retrieve"},
                        "before": {"type": "string", "description": "ISO date to get activities before"}
                    }
                }
            },
            {
                "name": "analyze_activity",
                "description": "AI analysis of fitness activity performance",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "activity_id": {"type": "string", "description": "Activity ID to analyze"}
                    },
                    "required": ["activity_id"]
                }
            }
        ]);

        A2AResponse {
            jsonrpc: "2.0".into(),
            result: Some(tools),
            error: None,
            id: request.id,
        }
    }

    /// Create a default server config for A2A protocol
    fn get_or_create_config(&self) -> std::sync::Arc<crate::config::environment::ServerConfig> {
        self.config.as_ref().map_or_else(
            || {
                // Create a minimal fallback config if none provided
                std::sync::Arc::new(
                    crate::config::environment::ServerConfig::from_env()
                        .unwrap_or_else(|_| Self::create_minimal_fallback_config()),
                )
            },
            std::clone::Clone::clone,
        )
    }

    /// Create minimal fallback config for A2A protocol
    fn create_minimal_fallback_config() -> crate::config::environment::ServerConfig {
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
                    scopes: vec!["read".into(), "activity:read_all".into()],
                    enabled: true,
                },
                fitbit: crate::config::environment::OAuthProviderConfig {
                    client_id: std::env::var("FITBIT_CLIENT_ID").ok(),
                    client_secret: std::env::var("FITBIT_CLIENT_SECRET").ok(),
                    redirect_uri: std::env::var("FITBIT_REDIRECT_URI").ok(),
                    scopes: vec!["activity".into(), "profile".into()],
                    enabled: true,
                },
            },
            security: crate::config::environment::SecurityConfig {
                cors_origins: vec!["*".into()],
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
                    base_url: "https://api.openweathermap.org/data/2.5".into(),
                    enabled: false,
                },
                geocoding: crate::config::environment::GeocodingServiceConfig {
                    base_url: "https://nominatim.openstreetmap.org".into(),
                    enabled: true,
                },
                strava_api: crate::config::environment::StravaApiConfig {
                    base_url: "https://www.strava.com/api/v3".into(),
                    auth_url: "https://www.strava.com/oauth/authorize".into(),
                    token_url: "https://www.strava.com/oauth/token".into(),
                },
                fitbit_api: crate::config::environment::FitbitApiConfig {
                    base_url: "https://api.fitbit.com".into(),
                    auth_url: "https://www.fitbit.com/oauth2/authorize".into(),
                    token_url: "https://api.fitbit.com/oauth2/token".into(),
                },
            },
            app_behavior: crate::config::environment::AppBehaviorConfig {
                max_activities_fetch: 100,
                default_activities_limit: 20,
                ci_mode: false,
                protocol: crate::config::environment::ProtocolConfig {
                    mcp_version: "2024-11-05".into(),
                    server_name: "pierre-mcp-server".into(),
                    server_version: env!("CARGO_PKG_VERSION").to_string(),
                },
            },
        }
    }

    async fn handle_tool_call(&self, request: A2ARequest) -> A2AResponse {
        // Implement tool execution through universal tool layer
        let params = request.params.unwrap_or_default();

        let tool_name = params
            .get("tool_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let tool_params = params
            .get("parameters")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        // Create universal request
        let universal_request = crate::protocols::universal::UniversalRequest {
            tool_name: tool_name.to_string(),
            parameters: serde_json::Value::Object(tool_params),
            user_id: "unknown".into(), // In production, this would come from authentication
            protocol: "a2a".into(),
        };

        // Check if we have proper dependencies injected
        let (database, intelligence) = match (&self.database, &self.intelligence) {
            (Some(db), Some(intel)) => (db.clone(), intel.clone()),
            _ => {
                // Return error if dependencies are not available
                return A2AResponse {
                    jsonrpc: "2.0".into(),
                    result: None,
                    error: Some(A2AErrorResponse {
                        code: -32000,
                        message: "A2A server not properly configured with database and intelligence dependencies".into(),
                        data: None,
                    }),
                    id: request.id,
                };
            }
        };

        let server_config = self.get_or_create_config();

        let executor = crate::protocols::universal::UniversalToolExecutor::new(
            database,
            intelligence,
            server_config,
        );

        match executor.execute_tool(universal_request).await {
            Ok(response) => A2AResponse {
                jsonrpc: "2.0".into(),
                result: response.result,
                error: None,
                id: request.id,
            },
            Err(e) => A2AResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(A2AErrorResponse {
                    code: -32000,
                    message: format!("Tool execution failed: {e}"),
                    data: None,
                }),
                id: request.id,
            },
        }
    }

    fn handle_task_cancel(request: A2ARequest) -> A2AResponse {
        let params = request.params.unwrap_or_default();
        let task_id = params
            .get("task_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        // In a full implementation, this would cancel an active task
        // Simulate task cancellation until full task management is implemented
        A2AResponse {
            jsonrpc: "2.0".into(),
            result: Some(serde_json::json!({
                "task_id": task_id,
                "status": "cancelled",
                "cancelled_at": chrono::Utc::now().to_rfc3339()
            })),
            error: None,
            id: request.id,
        }
    }

    fn handle_push_notification_config(request: A2ARequest) -> A2AResponse {
        let params = request.params.unwrap_or_default();

        // Extract notification configuration from params
        let config = params.get("config").cloned().unwrap_or_default();

        // In a full implementation, this would store push notification settings
        A2AResponse {
            jsonrpc: "2.0".into(),
            result: Some(serde_json::json!({
                "status": "configured",
                "config": config,
                "updated_at": chrono::Utc::now().to_rfc3339()
            })),
            error: None,
            id: request.id,
        }
    }

    fn handle_unknown_method(request: A2ARequest) -> A2AResponse {
        let error = A2AErrorResponse {
            code: -32601,
            message: format!("Method not found: {}", request.method),
            data: None,
        };

        A2AResponse {
            jsonrpc: "2.0".into(),
            result: None,
            error: Some(error),
            id: request.id,
        }
    }
}

impl Default for A2AServer {
    fn default() -> Self {
        Self::new()
    }
}
