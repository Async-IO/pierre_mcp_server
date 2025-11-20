// ABOUTME: Core A2A protocol message handling and JSON-RPC implementation
// ABOUTME: Processes A2A protocol requests, tool execution, and task management for agent communication
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! A2A Protocol Implementation
//!
// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - JSON-RPC message ownership for protocol serialization
// - Request/response ownership across async boundaries
//!
//! Implements the core A2A (Agent-to-Agent) protocol for Pierre,
//! providing JSON-RPC 2.0 based communication between AI agents.

use crate::database_plugins::DatabaseProvider;
use crate::types::json_schemas;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{info, warn};
use uuid::Uuid;

// Phase 2: Type aliases pointing to unified JSON-RPC foundation

/// A2A protocol request (JSON-RPC 2.0 request)
pub type A2ARequest = crate::jsonrpc::JsonRpcRequest;
/// A2A protocol response (JSON-RPC 2.0 response)
pub type A2AResponse = crate::jsonrpc::JsonRpcResponse;

/// A2A Protocol Error types
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum A2AError {
    /// Invalid request parameters or format
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    /// Client not registered
    #[error("Client not registered: {0}")]
    ClientNotRegistered(String),
    /// Database operation failed
    #[error("Database error: {0}")]
    DatabaseError(String),
    /// Internal server error
    #[error("Internal error: {0}")]
    InternalError(String),
    /// Client has been deactivated
    #[error("Client deactivated: {0}")]
    ClientDeactivated(String),
    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
    /// Session expired or invalid
    #[error("Session expired: {0}")]
    SessionExpired(String),
    /// Invalid session token
    #[error("Invalid session token: {0}")]
    InvalidSessionToken(String),
    /// Insufficient permissions
    #[error("Insufficient permissions: {0}")]
    InsufficientPermissions(String),
    /// Resource not found
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),
    /// Service temporarily unavailable
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
}

// Phase 2: Type alias for error response

/// A2A protocol error response (JSON-RPC 2.0 error)
pub type A2AErrorResponse = crate::jsonrpc::JsonRpcError;

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

/// A2A Initialize Request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AInitializeRequest {
    /// A2A protocol version
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    /// Client information
    #[serde(rename = "clientInfo")]
    pub client_info: A2AClientInfo,
    /// Client capabilities
    pub capabilities: Vec<String>,
    /// Optional OAuth application credentials provided by the client
    #[serde(
        rename = "oauthCredentials",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub oauth_credentials: Option<HashMap<String, crate::mcp::schema::OAuthAppCredentials>>,
}

/// A2A Client Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AClientInfo {
    /// Client application name
    pub name: String,
    /// Client application version
    pub version: String,
    /// Optional client description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A2A Initialize Response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AInitializeResponse {
    /// Negotiated protocol version
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    /// Server information
    #[serde(rename = "serverInfo")]
    pub server_info: A2AServerInfo,
    /// Server capabilities
    pub capabilities: Vec<String>,
}

/// A2A Server Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AServerInfo {
    /// Server application name
    pub name: String,
    /// Server application version
    pub version: String,
    /// Optional server description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl A2AInitializeResponse {
    /// Create a new A2A initialize response with server information
    #[must_use]
    pub fn new(protocol_version: String, server_name: String, server_version: String) -> Self {
        Self {
            protocol_version,
            server_info: A2AServerInfo {
                name: server_name,
                version: server_version,
                description: Some(
                    "AI-powered fitness data analysis and insights platform".to_owned(),
                ),
            },
            capabilities: vec![
                "message/send".to_owned(),
                "message/stream".to_owned(),
                "tasks/create".to_owned(),
                "tasks/get".to_owned(),
                "tasks/cancel".to_owned(),
                "tasks/pushNotificationConfig/set".to_owned(),
                "tools/list".to_owned(),
                "tools/call".to_owned(),
            ],
        }
    }
}

/// A2A Message structure for agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AMessage {
    /// Unique message identifier
    pub id: String,
    /// Message content parts (text, data, or files)
    pub parts: Vec<MessagePart>,
    /// Optional metadata key-value pairs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,
}

/// A2A Message Part types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessagePart {
    /// Plain text message content
    #[serde(rename = "text")]
    Text {
        /// Text content
        content: String,
    },
    /// Structured data content (JSON)
    #[serde(rename = "data")]
    Data {
        /// Data content as JSON value
        content: Value,
    },
    /// File attachment content
    #[serde(rename = "file")]
    File {
        /// File name
        name: String,
        /// MIME type of the file
        mime_type: String,
        /// File content (base64 encoded)
        content: String,
    },
}

/// A2A Task structure for long-running operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ATask {
    /// Unique task identifier
    pub id: String,
    /// Current status of the task
    pub status: TaskStatus,
    /// When the task was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the task completed (if finished)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Task result data (if completed successfully)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Client ID that created this task
    pub client_id: String,
    /// Type of task being performed
    pub task_type: String,
    /// Input data for the task
    pub input_data: Value,
    /// Output data from the task (if completed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_data: Option<Value>,
    /// Detailed error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// When the task was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Task status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// Task is queued but not yet started
    Pending,
    /// Task is currently executing
    Running,
    /// Task finished successfully
    Completed,
    /// Task failed with an error
    Failed,
    /// Task was cancelled by user or system
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
    /// A2A protocol version
    pub version: String,
    /// Optional server resources for MCP integration
    pub resources: Option<std::sync::Arc<crate::mcp::resources::ServerResources>>,
}

impl A2AServer {
    /// Create a new A2A server without resources
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: crate::a2a::A2A_VERSION.to_owned(),
            resources: None,
        }
    }

    /// Create a new A2A server with server resources
    #[must_use]
    pub fn new_with_resources(
        resources: std::sync::Arc<crate::mcp::resources::ServerResources>,
    ) -> Self {
        Self {
            version: crate::a2a::A2A_VERSION.to_owned(),
            resources: Some(resources),
        }
    }

    /// Handle incoming A2A request
    pub async fn handle_request(&self, request: A2ARequest) -> A2AResponse {
        match request.method.as_str() {
            "a2a/initialize" => {
                // Use OAuth-aware initialization if authentication is provided
                if request.auth_token.is_some() && self.resources.is_some() {
                    self.handle_initialize_with_oauth(request).await
                } else {
                    self.handle_initialize(request)
                }
            }
            "message/send" | "a2a/message/send" => Self::handle_message_send(request),
            "message/stream" | "a2a/message/stream" => Self::handle_message_stream(request),
            "tasks/create" | "a2a/tasks/create" => self.handle_task_create(request).await,
            "tasks/get" | "a2a/tasks/get" => self.handle_task_get(request).await,
            "tasks/cancel" => Self::handle_task_cancel(request),
            "tasks/resubscribe" | "a2a/tasks/resubscribe" => Self::handle_task_resubscribe(request),
            "tasks/pushNotificationConfig/set" => Self::handle_push_notification_config(request),
            "a2a/tasks/list" => self.handle_task_list(request).await,
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

    /// Handle A2A initialize request with `ServerResources` for OAuth credential storage
    async fn handle_initialize_with_oauth(&self, request: A2ARequest) -> A2AResponse {
        // Extract resources with defensive error handling
        let Some(resources) = self.resources.as_ref() else {
            return A2AResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(A2AErrorResponse {
                    code: -32603,
                    message: "Internal error: Server resources not initialized".to_owned(),
                    data: None,
                }),
                id: request.id.clone(),
            };
        };

        let user_id = match Self::authenticate_request(&request, resources) {
            Ok(user_id) => user_id,
            Err(error_response) => return *error_response,
        };

        // Handle OAuth credentials if provided
        let response = self.handle_initialize_internal(request.clone()); // Safe: Request ownership for internal call

        // If initialization successful and OAuth credentials provided, store them
        if response.error.is_none() {
            if let Some(params) = &request.params {
                if let Ok(init_request) =
                    serde_json::from_value::<A2AInitializeRequest>(params.clone())
                // Safe: JSON value ownership for deserialization
                {
                    if let Some(oauth_creds) = init_request.oauth_credentials {
                        if let Err(e) =
                            Self::store_oauth_credentials(oauth_creds, &user_id, resources).await
                        {
                            warn!(
                                "Failed to store OAuth credentials during A2A initialization: {e}"
                            );
                        } else {
                            info!("Successfully stored OAuth credentials for A2A user {user_id}");
                        }
                    }
                }
            }
        }

        response
    }

    /// Internal A2A initialize handler with proper protocol version negotiation
    fn handle_initialize_internal(&self, request: A2ARequest) -> A2AResponse {
        // Parse A2A initialize request parameters
        let init_request = request.params.as_ref().and_then(|params| {
            serde_json::from_value::<A2AInitializeRequest>(params.clone()) // Safe: JSON value ownership for deserialization
                .inspect_err(|e| {
                    tracing::warn!(
                        error = ?e,
                        params = ?params,
                        "Failed to parse A2A initialize request parameters - using defaults"
                    );
                })
                .ok()
        });

        let (protocol_version, client_name) = if let Some(req) = init_request {
            // For now, accept any protocol version - A2A is more flexible
            (req.protocol_version, req.client_info.name)
        } else {
            // Default values if parsing fails
            tracing::debug!("A2A initialize: using default values (no valid params provided)");
            (crate::a2a::A2A_VERSION.to_owned(), "unknown".to_owned())
        };

        info!("A2A initialization from client: {client_name} with protocol version: {protocol_version}");

        // Create A2A initialize response
        let init_response = A2AInitializeResponse::new(
            protocol_version,
            "pierre-a2a-server".to_owned(),
            self.version.clone(), // Safe: String ownership for response
        );

        match serde_json::to_value(&init_response) {
            Ok(result) => A2AResponse {
                jsonrpc: "2.0".into(),
                result: Some(result),
                error: None,
                id: request.id,
            },
            Err(e) => A2AResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(A2AErrorResponse {
                    code: -32603,
                    message: format!("Failed to serialize A2A initialize response: {e}"),
                    data: None,
                }),
                id: request.id,
            },
        }
    }

    /// Authenticate the A2A request and extract user information
    fn authenticate_request(
        request: &A2ARequest,
        resources: &std::sync::Arc<crate::mcp::resources::ServerResources>,
    ) -> Result<uuid::Uuid, Box<A2AResponse>> {
        let request_id = request
            .id
            .clone() // Safe: JSON value ownership for request ID
            .unwrap_or_else(|| serde_json::Value::Number(serde_json::Number::from(0)));

        // Extract auth token from request
        let auth_token = request.auth_token.as_deref().ok_or_else(|| {
            Box::new(A2AResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(A2AErrorResponse {
                    code: -32001,
                    message: "Authentication token required for OAuth credential storage"
                        .to_owned(),
                    data: None,
                }),
                id: Some(request_id.clone()), // Safe: JSON value ownership for error response
            })
        })?;

        // Validate token and extract user_id
        match resources
            .auth_manager
            .validate_token(auth_token, &resources.jwks_manager)
        {
            Ok(claims) => uuid::Uuid::parse_str(&claims.sub).map_or_else(
                |_| {
                    Err(Box::new(A2AResponse {
                        jsonrpc: "2.0".into(),
                        result: None,
                        error: Some(A2AErrorResponse {
                            code: -32001,
                            message: "Invalid user ID in authentication token".to_owned(),
                            data: None,
                        }),
                        id: Some(request_id.clone()), // Safe: JSON value ownership for error response
                    }))
                },
                Ok,
            ),
            Err(_) => Err(Box::new(A2AResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(A2AErrorResponse {
                    code: -32001,
                    message: "Invalid authentication token".to_owned(),
                    data: None,
                }),
                id: Some(request_id),
            })),
        }
    }

    /// Store OAuth credentials provided during A2A initialization
    async fn store_oauth_credentials(
        oauth_creds: HashMap<String, crate::mcp::schema::OAuthAppCredentials>,
        user_id: &uuid::Uuid,
        resources: &std::sync::Arc<crate::mcp::resources::ServerResources>,
    ) -> Result<(), String> {
        for (provider, creds) in oauth_creds {
            info!("Storing OAuth credentials for provider {provider} for A2A user {user_id}");

            // Store encrypted OAuth app credentials in database
            // Use default redirect URI for A2A clients
            let redirect_uri = format!("urn:ietf:wg:oauth:2.0:oob:{provider}:a2a");
            resources
                .database
                .store_user_oauth_app(
                    *user_id,
                    &provider,
                    &creds.client_id,
                    &creds.client_secret,
                    &redirect_uri,
                )
                .await
                .map_err(|e| format!("Failed to store {provider} OAuth credentials: {e}"))?;
        }

        Ok(())
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
        // Get base URL from environment or use default
        let base_url =
            std::env::var("PIERRE_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_owned());

        // Extract stream_id or task_id from params (support both for flexibility)
        let params = request.params.as_ref().unwrap_or(&serde_json::Value::Null);
        let stream_id = params
            .get("stream_id")
            .or_else(|| params.get("task_id"))
            .and_then(|v| v.as_str());

        if let Some(id) = stream_id {
            // Return SSE streaming endpoint for specific stream/task
            A2AResponse {
                jsonrpc: "2.0".into(),
                result: Some(serde_json::json!({
                    "stream_url": format!("{}/a2a/tasks/{}/stream", base_url, id),
                    "stream_type": "text/event-stream",
                    "protocol": "SSE",
                    "keep_alive_interval_seconds": 15,
                    "status": "streaming_available"
                })),
                error: None,
                id: request.id,
            }
        } else {
            // Return generic streaming info if no specific ID provided
            A2AResponse {
                jsonrpc: "2.0".into(),
                result: Some(serde_json::json!({
                    "streaming_supported": true,
                    "stream_type": "text/event-stream",
                    "protocol": "SSE",
                    "status": "available"
                })),
                error: None,
                id: request.id,
            }
        }
    }

    async fn handle_task_create(&self, request: A2ARequest) -> A2AResponse {
        // Parse request parameters using typed struct
        let params_value = request.params.as_ref().unwrap_or(&serde_json::Value::Null);

        let task_params =
            match serde_json::from_value::<json_schemas::A2ATaskCreateParams>(params_value.clone())
            {
                Ok(params) => params,
                Err(e) => {
                    tracing::error!("Failed to parse A2A task create parameters: {}", e);
                    return A2AResponse {
                        jsonrpc: "2.0".into(),
                        result: None,
                        error: Some(A2AErrorResponse {
                            code: -32602,
                            message: format!("Invalid parameters: {e}"),
                            data: None,
                        }),
                        id: request.id,
                    };
                }
            };

        let client_id = task_params.client_id;
        let task_type = task_params.task_type;

        // Persist task to database and get generated task_id
        let task_id = if let Some(resources) = &self.resources {
            let database = &resources.database;
            match database
                .create_a2a_task(
                    &client_id,
                    None, // session_id - optional
                    &task_type,
                    params_value,
                )
                .await
            {
                Ok(id) => {
                    // Task successfully persisted
                    tracing::info!("Created A2A task {} for client {}", id, client_id);
                    id
                }
                Err(e) => {
                    // Database error - return error response
                    return A2AResponse {
                        jsonrpc: "2.0".into(),
                        result: None,
                        error: Some(A2AErrorResponse {
                            code: -32000,
                            message: format!("Failed to persist task: {e}"),
                            data: None,
                        }),
                        id: request.id,
                    };
                }
            }
        } else {
            // No database - generate a local ID
            Uuid::new_v4().to_string()
        };

        let task = A2ATask {
            id: task_id,
            status: TaskStatus::Pending,
            created_at: chrono::Utc::now(),
            completed_at: None,
            result: None,
            error: None,
            client_id,
            task_type,
            input_data: params_value.clone(), // Safe: JSON value ownership for task input
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
                    message: "Internal error: Failed to serialize task".to_owned(),
                    data: Some(serde_json::json!({
                        "error": e.to_string(),
                        "context": "Task serialization failed"
                    })),
                }),
                id: request.id,
            },
        }
    }

    async fn handle_task_get(&self, request: A2ARequest) -> A2AResponse {
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
        if let Some(resources) = &self.resources {
            let database = &resources.database;
            // Parse task_id from validated params using typed struct
            let params_value = request.params.as_ref().unwrap_or(&serde_json::Value::Null);

            let task_params = match serde_json::from_value::<json_schemas::A2ATaskGetParams>(
                params_value.clone(),
            ) {
                Ok(params) => params,
                Err(e) => {
                    tracing::error!("Failed to parse A2A task get parameters: {}", e);
                    return A2AResponse {
                        jsonrpc: "2.0".into(),
                        result: None,
                        error: Some(A2AErrorResponse {
                            code: -32602,
                            message: format!("Invalid parameters: {e}"),
                            data: None,
                        }),
                        id: request.id,
                    };
                }
            };

            let task_id = &task_params.task_id;

            // Get task from database
            match database.get_a2a_task(task_id).await {
                Ok(Some(task)) => {
                    return A2AResponse {
                        jsonrpc: "2.0".into(),
                        result: Some(serde_json::to_value(task).unwrap_or_default()),
                        error: None,
                        id: request.id,
                    };
                }
                Ok(None) => {
                    return A2AResponse {
                        jsonrpc: "2.0".into(),
                        result: None,
                        error: Some(A2AErrorResponse {
                            code: -32601,
                            message: format!("Task not found: {task_id}"),
                            data: None,
                        }),
                        id: request.id,
                    };
                }
                Err(e) => {
                    return A2AResponse {
                        jsonrpc: "2.0".into(),
                        result: None,
                        error: Some(A2AErrorResponse {
                            code: -32000,
                            message: format!("Database error: {e}"),
                            data: None,
                        }),
                        id: request.id,
                    };
                }
            }
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

    async fn handle_task_list(&self, request: A2ARequest) -> A2AResponse {
        // Query database for actual tasks if available
        if let Some(resources) = &self.resources {
            let database = &resources.database;
            // Parse list parameters using typed struct (with defaults for missing fields)
            let params_value = request.params.as_ref().unwrap_or(&serde_json::Value::Null);

            let list_params =
                serde_json::from_value::<json_schemas::A2ATaskListParams>(params_value.clone())
                    .unwrap_or(json_schemas::A2ATaskListParams {
                        client_id: None,
                        status: None,
                        limit: 20,
                        offset: None,
                    });

            let client_id = list_params.client_id.as_deref();
            let limit = Some(list_params.limit);
            let offset = list_params.offset;

            // Parse status string into enum if provided
            let status_filter =
                list_params
                    .status
                    .as_deref()
                    .and_then(|status_str| match status_str {
                        "pending" => Some(TaskStatus::Pending),
                        "running" => Some(TaskStatus::Running),
                        "completed" => Some(TaskStatus::Completed),
                        "failed" => Some(TaskStatus::Failed),
                        "cancelled" => Some(TaskStatus::Cancelled),
                        _ => None,
                    });

            // Query database for tasks
            match database
                .list_a2a_tasks(client_id, status_filter.as_ref(), limit, offset)
                .await
            {
                Ok(tasks) => {
                    let tasks_json = serde_json::to_value(&tasks).unwrap_or_default();
                    A2AResponse {
                        jsonrpc: "2.0".into(),
                        result: Some(serde_json::json!({
                            "tasks": tasks_json,
                            "total": tasks.len(),
                            "limit": limit,
                            "offset": offset.unwrap_or(0)
                        })),
                        error: None,
                        id: request.id,
                    }
                }
                Err(e) => A2AResponse {
                    jsonrpc: "2.0".into(),
                    result: None,
                    error: Some(A2AErrorResponse {
                        code: -32000,
                        message: format!("Database error: {e}"),
                        data: None,
                    }),
                    id: request.id,
                },
            }
        } else {
            A2AResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(A2AErrorResponse {
                    code: -32000,
                    message: "Database not available for task listing".into(),
                    data: None,
                }),
                id: request.id,
            }
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

    async fn handle_tool_call(&self, request: A2ARequest) -> A2AResponse {
        // Implement tool execution through universal tool layer
        let params = request.params.unwrap_or_default();

        let tool_name = params
            .get("tool_name")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| {
                tracing::warn!("Missing tool_name in A2A tool execution request, using 'unknown'");
                "unknown"
            });

        let tool_params = params
            .get("parameters")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        // Create universal request
        let universal_request = crate::protocols::universal::UniversalRequest {
            tool_name: tool_name.to_owned(),
            parameters: serde_json::Value::Object(tool_params),
            user_id: "unknown".into(), // In production, this would come from authentication
            protocol: "a2a".into(),
            tenant_id: None, // A2A protocol doesn't have tenant context yet
            progress_token: None,
            cancellation_token: None,
            progress_reporter: None,
        };

        // Check if we have proper ServerResources injected
        let resources = match &self.resources {
            Some(res) => res.clone(), // Safe: Arc clone for server resources
            None => {
                // Return error if ServerResources are not available
                return A2AResponse {
                    jsonrpc: "2.0".into(),
                    result: None,
                    error: Some(A2AErrorResponse {
                        code: -32000,
                        message: "A2A server not properly configured with ServerResources".into(),
                        data: None,
                    }),
                    id: request.id,
                };
            }
        };

        let executor = crate::protocols::universal::UniversalToolExecutor::new(resources);

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

    fn handle_task_resubscribe(request: A2ARequest) -> A2AResponse {
        // Get base URL from environment or use default
        let base_url =
            std::env::var("PIERRE_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_owned());

        let params = request.params.as_ref().unwrap_or(&serde_json::Value::Null);
        let task_id = params.get("task_id").and_then(|v| v.as_str());

        if let Some(task_id) = task_id {
            // Return resubscription information with new stream endpoint
            A2AResponse {
                jsonrpc: "2.0".into(),
                result: Some(serde_json::json!({
                    "task_id": task_id,
                    "stream_url": format!("{}/a2a/tasks/{}/stream", base_url, task_id),
                    "stream_type": "text/event-stream",
                    "protocol": "SSE",
                    "reconnected": true,
                    "message": "Task stream available for resubscription"
                })),
                error: None,
                id: request.id,
            }
        } else {
            A2AResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(A2AErrorResponse {
                    code: -32602,
                    message: "Missing required parameter: task_id".into(),
                    data: None,
                }),
                id: request.id,
            }
        }
    }

    fn handle_task_cancel(request: A2ARequest) -> A2AResponse {
        let params = request.params.unwrap_or_default();
        let task_id = params
            .get("task_id")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| {
                tracing::warn!("Missing task_id in A2A task cancel request, using 'unknown'");
                "unknown"
            });

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
