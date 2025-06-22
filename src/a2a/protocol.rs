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
    pub error: Option<A2AError>,
    pub id: Option<Value>,
}

/// A2A Protocol Error Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
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
}

/// Task status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// A2A Protocol Server implementation
pub struct A2AServer {
    pub version: String,
    pub database: Option<std::sync::Arc<crate::database_plugins::factory::Database>>,
    pub intelligence: Option<std::sync::Arc<crate::intelligence::ActivityIntelligence>>,
}

impl A2AServer {
    pub fn new() -> Self {
        Self {
            version: crate::a2a::A2A_VERSION.to_string(),
            database: None,
            intelligence: None,
        }
    }

    pub fn new_with_dependencies(
        database: std::sync::Arc<crate::database_plugins::factory::Database>,
        intelligence: std::sync::Arc<crate::intelligence::ActivityIntelligence>,
    ) -> Self {
        Self {
            version: crate::a2a::A2A_VERSION.to_string(),
            database: Some(database),
            intelligence: Some(intelligence),
        }
    }

    /// Handle incoming A2A request
    pub async fn handle_request(&self, request: A2ARequest) -> A2AResponse {
        match request.method.as_str() {
            "a2a/initialize" => self.handle_initialize(request).await,
            "message/send" => self.handle_message_send(request).await,
            "message/stream" => self.handle_message_stream(request).await,
            "tasks/create" => self.handle_task_create(request).await,
            "tasks/get" => self.handle_task_get(request).await,
            "tasks/cancel" => self.handle_task_cancel(request).await,
            "tasks/pushNotificationConfig/set" => {
                self.handle_push_notification_config(request).await
            }
            "tools/list" => self.handle_tools_list(request).await,
            "tools/call" => self.handle_tool_call(request).await,
            // Legacy A2A prefix support (backwards compatibility)
            "a2a/message/send" => self.handle_message_send(request).await,
            "a2a/message/stream" => self.handle_message_stream(request).await,
            "a2a/tasks/create" => self.handle_task_create(request).await,
            "a2a/tasks/get" => self.handle_task_get(request).await,
            "a2a/tasks/list" => self.handle_task_list(request).await,
            "a2a/tools/list" => self.handle_tools_list(request).await,
            "a2a/tools/call" => self.handle_tool_call(request).await,
            _ => self.handle_unknown_method(request).await,
        }
    }

    async fn handle_initialize(&self, request: A2ARequest) -> A2AResponse {
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
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id: request.id,
        }
    }

    async fn handle_message_send(&self, request: A2ARequest) -> A2AResponse {
        // Message sending would forward requests to appropriate handlers
        A2AResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({"status": "received"})),
            error: None,
            id: request.id,
        }
    }

    async fn handle_message_stream(&self, request: A2ARequest) -> A2AResponse {
        // Streaming implementation using a task-based approach
        // In a full implementation, this would establish a persistent connection
        // For now, we'll return a response indicating streaming is not yet fully supported
        A2AResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({
                "status": "streaming_not_supported",
                "message": "Message streaming is not yet implemented. Use message/send for single messages.",
                "alternative": "Use a2a/message/send for immediate message delivery"
            })),
            error: None,
            id: request.id,
        }
    }

    async fn handle_task_create(&self, request: A2ARequest) -> A2AResponse {
        let task_id = Uuid::new_v4().to_string();
        let task = A2ATask {
            id: task_id.clone(),
            status: TaskStatus::Pending,
            created_at: chrono::Utc::now(),
            completed_at: None,
            result: None,
            error: None,
        };

        A2AResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::to_value(task).unwrap()),
            error: None,
            id: request.id,
        }
    }

    async fn handle_task_get(&self, request: A2ARequest) -> A2AResponse {
        // Extract task ID from parameters
        let task_id = match request.params.as_ref().and_then(|p| p.get("task_id")) {
            Some(serde_json::Value::String(id)) => id,
            _ => {
                return A2AResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(A2AError {
                        code: -32602,
                        message: "Invalid params: task_id is required".to_string(),
                        data: None,
                    }),
                    id: request.id,
                };
            }
        };

        // For now, return a mock task - in production this would query the database
        let task = serde_json::json!({
            "id": task_id,
            "type": "fitness_analysis",
            "status": "completed",
            "result": {
                "summary": "Task completed successfully",
                "data": {
                    "activities_analyzed": 10,
                    "insights_generated": 3
                }
            },
            "created_at": chrono::Utc::now().to_rfc3339(),
            "completed_at": chrono::Utc::now().to_rfc3339()
        });

        A2AResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(task),
            error: None,
            id: request.id,
        }
    }

    async fn handle_task_list(&self, request: A2ARequest) -> A2AResponse {
        // Extract pagination parameters
        let default_params = serde_json::json!({});
        let params = request.params.as_ref().unwrap_or(&default_params);
        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
        let offset = params.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        // For now, return mock tasks - in production this would query the database
        let mock_tasks = vec![
            serde_json::json!({
                "id": "task_001",
                "type": "fitness_analysis",
                "status": "completed",
                "created_at": chrono::Utc::now().to_rfc3339(),
                "description": "Analyze weekly running performance"
            }),
            serde_json::json!({
                "id": "task_002",
                "type": "goal_tracking",
                "status": "in_progress",
                "created_at": chrono::Utc::now().to_rfc3339(),
                "description": "Track marathon training progress"
            }),
        ];

        // Apply pagination
        let total_count = mock_tasks.len();
        let paginated_tasks: Vec<_> = mock_tasks.into_iter().skip(offset).take(limit).collect();

        A2AResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({
                "tasks": paginated_tasks,
                "pagination": {
                    "total_count": total_count,
                    "limit": limit,
                    "offset": offset,
                    "has_more": offset + limit < total_count
                }
            })),
            error: None,
            id: request.id,
        }
    }

    async fn handle_tools_list(&self, request: A2ARequest) -> A2AResponse {
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
            jsonrpc: "2.0".to_string(),
            result: Some(tools),
            error: None,
            id: request.id,
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
            user_id: "unknown".to_string(), // In production, this would come from authentication
            protocol: "a2a".to_string(),
        };

        // Check if we have proper dependencies injected
        let (database, intelligence) = match (&self.database, &self.intelligence) {
            (Some(db), Some(intel)) => (db.clone(), intel.clone()),
            _ => {
                // Return error if dependencies are not available
                return A2AResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(A2AError {
                        code: -32000,
                        message: "A2A server not properly configured with database and intelligence dependencies".to_string(),
                        data: None,
                    }),
                    id: request.id,
                };
            }
        };

        // Create a minimal config for A2A protocol (TODO: pass proper ServerConfig)
        let fallback_config = std::sync::Arc::new(
            crate::config::environment::ServerConfig::from_env().unwrap_or_else(|_| {
                // Create a minimal fallback config
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

        let executor = crate::protocols::universal::UniversalToolExecutor::new(
            database,
            intelligence,
            fallback_config,
        );

        match executor.execute_tool(universal_request).await {
            Ok(response) => A2AResponse {
                jsonrpc: "2.0".to_string(),
                result: response.result,
                error: None,
                id: request.id,
            },
            Err(e) => A2AResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(A2AError {
                    code: -32000,
                    message: format!("Tool execution failed: {}", e),
                    data: None,
                }),
                id: request.id,
            },
        }
    }

    async fn handle_task_cancel(&self, request: A2ARequest) -> A2AResponse {
        let params = request.params.unwrap_or_default();
        let task_id = params
            .get("task_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        // In a full implementation, this would cancel an active task
        // For now, we'll simulate task cancellation
        A2AResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({
                "task_id": task_id,
                "status": "cancelled",
                "cancelled_at": chrono::Utc::now().to_rfc3339()
            })),
            error: None,
            id: request.id,
        }
    }

    async fn handle_push_notification_config(&self, request: A2ARequest) -> A2AResponse {
        let params = request.params.unwrap_or_default();

        // Extract notification configuration from params
        let config = params.get("config").cloned().unwrap_or_default();

        // In a full implementation, this would store push notification settings
        A2AResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({
                "status": "configured",
                "config": config,
                "updated_at": chrono::Utc::now().to_rfc3339()
            })),
            error: None,
            id: request.id,
        }
    }

    async fn handle_unknown_method(&self, request: A2ARequest) -> A2AResponse {
        let error = A2AError {
            code: -32601,
            message: format!("Method not found: {}", request.method),
            data: None,
        };

        A2AResponse {
            jsonrpc: "2.0".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_a2a_initialize() {
        let server = A2AServer::new();
        let request = A2ARequest {
            jsonrpc: "2.0".to_string(),
            method: "a2a/initialize".to_string(),
            params: None,
            id: Some(serde_json::Value::Number(1.into())),
        };

        let response = server.handle_request(request).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_a2a_unknown_method() {
        let server = A2AServer::new();
        let request = A2ARequest {
            jsonrpc: "2.0".to_string(),
            method: "unknown/method".to_string(),
            params: None,
            id: Some(serde_json::Value::Number(1.into())),
        };

        let response = server.handle_request(request).await;
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32601);
    }

    #[test]
    fn test_message_part_serialization() {
        let text_part = MessagePart::Text {
            content: "Hello, world!".to_string(),
        };

        let json = serde_json::to_string(&text_part).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"content\":\"Hello, world!\""));
    }

    #[test]
    fn test_task_status_serialization() {
        let status = TaskStatus::Running;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"running\"");
    }
}
