// ABOUTME: MCP protocol schema definitions and message structures
// ABOUTME: Defines JSON-RPC protocol schemas for Model Context Protocol communication
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! MCP Protocol Schema Definitions
//!
//! This module contains type-safe definitions for all MCP protocol messages,
//! capabilities, and tool schemas. This ensures protocol compliance and makes
//! it easy to modify the schema without hardcoding JSON.

use crate::constants::{
    json_fields::{ACTIVITY_ID, LIMIT, OFFSET, PROVIDER},
    tools::{
        ANALYZE_ACTIVITY, ANNOUNCE_OAUTH_SUCCESS, CHECK_OAUTH_NOTIFICATIONS, CONNECT_PROVIDER,
        CONNECT_TO_PIERRE, DELETE_FITNESS_CONFIG, DISCONNECT_PROVIDER, GET_ACTIVITIES,
        GET_ACTIVITY_INTELLIGENCE, GET_ATHLETE, GET_CONNECTION_STATUS, GET_FITNESS_CONFIG,
        GET_NOTIFICATIONS, GET_STATS, LIST_FITNESS_CONFIGS, MARK_NOTIFICATIONS_READ,
        SET_FITNESS_CONFIG,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// JSON-RPC and notification method constants
const JSONRPC_VERSION: &str = "2.0";
const METHOD_PROGRESS: &str = "notifications/progress";
const METHOD_CANCELLED: &str = "notifications/cancelled";
const METHOD_OAUTH_COMPLETED: &str = "notifications/oauth_completed";

// Note: Schema type strings ("string", "object", etc.) and property descriptions
// must be converted to String via .into() when inserted into HashMap/Vec because
// serde requires owned data for serialization. These allocations are necessary
// and cannot be eliminated without changing the serde data model to use Cow or &'static str.

/// MCP Protocol Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolInfo {
    /// MCP protocol version (e.g., "2025-06-18")
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
}

/// Server Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// Server name identifier
    pub name: String,
    /// Server version string
    pub version: String,
}

/// MCP Tool Schema Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    /// Tool name identifier
    pub name: String,
    /// Human-readable tool description
    pub description: String,
    /// JSON Schema for tool input parameters
    #[serde(rename = "inputSchema")]
    pub input_schema: JsonSchema,
}

/// JSON Schema Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchema {
    /// Schema type (e.g., "object", "string")
    #[serde(rename = "type")]
    pub schema_type: String,
    /// Property definitions for object schemas
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, PropertySchema>>,
    /// List of required property names
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

/// Tool Call for executing a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Name of the tool to execute
    pub name: String,
    /// Tool arguments as JSON
    pub arguments: Option<serde_json::Value>,
}

/// Tool Response after execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
    /// Response content items
    pub content: Vec<Content>,
    /// Whether the tool execution resulted in an error
    #[serde(rename = "isError")]
    pub is_error: bool,
    /// Structured response data
    #[serde(rename = "structuredContent", skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<serde_json::Value>,
}

/// Content types for MCP messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Content {
    /// Plain text content
    #[serde(rename = "text")]
    Text {
        /// Text content string
        text: String,
    },
    /// Image content with base64 data
    #[serde(rename = "image")]
    Image {
        /// Base64-encoded image data
        data: String,
        /// MIME type of the image (e.g., "image/png")
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    /// Resource reference with URI
    #[serde(rename = "resource")]
    Resource {
        /// URI of the resource
        uri: String,
        /// Optional text description of the resource
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        /// MIME type of the resource
        #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
    /// Progress update for long-running operations
    #[serde(rename = "progress")]
    Progress {
        /// Token identifying the operation
        #[serde(rename = "progressToken")]
        progress_token: String,
        /// Current progress value
        progress: f64,
        /// Optional total value for calculating percentage
        total: Option<f64>,
    },
}

/// Tool definition structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Tool name identifier
    pub name: String,
    /// Human-readable tool description
    pub description: String,
    /// JSON Schema for tool input as raw JSON value
    pub input_schema: serde_json::Value,
}

/// JSON Schema Property Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySchema {
    /// Property type (e.g., "string", "number", "boolean")
    #[serde(rename = "type")]
    pub property_type: String,
    /// Human-readable property description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// MCP Server Capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    /// Experimental capabilities not in MCP spec
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, serde_json::Value>>,
    /// Server logging capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingCapability>,
    /// Server prompts capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,
    /// Server resources capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,
    /// Server tools capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
    /// Server authentication capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthCapability>,
    /// Server OAuth 2.0 capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth2: Option<OAuth2Capability>,
}

/// Tools capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
    /// Whether the server supports list changed notifications
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Logging capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingCapability {}

/// Prompts capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsCapability {
    /// Whether the server supports list changed notifications
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Resources capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesCapability {
    /// Whether the server supports resource subscriptions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,
    /// Whether the server supports list changed notifications
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Authentication capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCapability {
    /// OAuth 2.0 authentication details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth2: Option<OAuth2Capability>,
}

/// OAuth 2.0 capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Capability {
    /// OAuth 2.0 discovery URL
    #[serde(rename = "discoveryUrl")]
    pub discovery_url: String,
    /// OAuth 2.0 authorization endpoint
    #[serde(rename = "authorizationEndpoint")]
    pub authorization_endpoint: String,
    /// OAuth 2.0 token endpoint
    #[serde(rename = "tokenEndpoint")]
    pub token_endpoint: String,
    /// OAuth 2.0 client registration endpoint (RFC 7591)
    #[serde(rename = "registrationEndpoint")]
    pub registration_endpoint: String,
}

/// Client capabilities (for processing client initialize requests)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCapabilities {
    /// Experimental client capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, serde_json::Value>>,
    /// Client sampling capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapability>,
    /// Client roots capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapability>,
}

/// Sampling capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingCapability {}

/// Roots capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsCapability {
    /// Whether the client supports list changed notifications
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Complete MCP Initialize Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResponse {
    /// Negotiated protocol version
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    /// Server information
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
    /// Server capabilities
    pub capabilities: ServerCapabilities,
    /// Optional server instructions for the client
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

/// Initialize Request from client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeRequest {
    /// Client's requested protocol version
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    /// Client information
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
    /// Client capabilities
    pub capabilities: ClientCapabilities,
    /// Optional OAuth application credentials provided by the client
    #[serde(
        rename = "oauthCredentials",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub oauth_credentials: Option<std::collections::HashMap<String, OAuthAppCredentials>>,
}

/// OAuth Application Credentials provided by client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthAppCredentials {
    /// OAuth client ID
    #[serde(rename = "clientId")]
    pub client_id: String,
    /// OAuth client secret
    #[serde(rename = "clientSecret")]
    pub client_secret: String,
}

/// Client Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    /// Client name identifier
    pub name: String,
    /// Client version string
    pub version: String,
}

impl InitializeResponse {
    /// Create a new initialize response with current server configuration
    #[must_use]
    pub fn new(protocol_version: String, server_name: String, server_version: String) -> Self {
        Self::new_with_ports(protocol_version, server_name, server_version, 8081)
    }

    /// Create a new initialize response with specific HTTP port for OAuth endpoints
    #[must_use]
    pub fn new_with_ports(
        protocol_version: String,
        server_name: String,
        server_version: String,
        http_port: u16,
    ) -> Self {
        Self {
            protocol_version,
            server_info: ServerInfo {
                name: server_name,
                version: server_version,
            },
            capabilities: ServerCapabilities {
                experimental: None,
                logging: Some(LoggingCapability {}),
                prompts: None,
                resources: Some(ResourcesCapability {
                    subscribe: None,
                    list_changed: Some(false),
                }),
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                auth: Some(AuthCapability {
                    oauth2: Some({
                        let host = crate::constants::get_server_config()
                            .map_or_else(|| "localhost".to_owned(), |c| c.host.clone());
                        OAuth2Capability {
                            discovery_url: format!("http://{host}:{http_port}/.well-known/oauth-authorization-server"),
                            authorization_endpoint: format!("http://{host}:{http_port}/oauth2/authorize"),
                            token_endpoint: format!("http://{host}:{http_port}/oauth2/token"),
                            registration_endpoint: format!("http://{host}:{http_port}/oauth2/register"),
                        }
                    }),
                }),
                oauth2: Some({
                    let host = crate::constants::get_server_config()
                        .map_or_else(|| "localhost".to_owned(), |c| c.host.clone());
                    OAuth2Capability {
                        discovery_url: format!("http://{host}:{http_port}/.well-known/oauth-authorization-server"),
                        authorization_endpoint: format!("http://{host}:{http_port}/oauth2/authorize"),
                        token_endpoint: format!("http://{host}:{http_port}/oauth2/token"),
                        registration_endpoint: format!("http://{host}:{http_port}/oauth2/register"),
                    }
                }),
            },
            instructions: Some("This server provides fitness data tools for Strava and Fitbit integration. OAuth must be configured at tenant level via REST API. Use `get_activities`, `get_athlete`, and other analytics tools to access your fitness data.".into()),
        }
    }
}

/// Progress notification for long-running operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressNotification {
    /// JSON-RPC version ("2.0")
    pub jsonrpc: String,
    /// Method name ("notifications/progress")
    pub method: String,
    /// Progress notification parameters
    pub params: ProgressParams,
}

/// Progress notification parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressParams {
    /// Token identifying the operation being tracked
    #[serde(rename = "progressToken")]
    pub progress_token: String,
    /// Current progress value
    pub progress: f64,
    /// Optional total value for percentage calculation
    pub total: Option<f64>,
    /// Optional human-readable progress message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl ProgressNotification {
    /// Create a new progress notification
    #[must_use]
    pub fn new(
        progress_token: String,
        progress: f64,
        total: Option<f64>,
        message: Option<String>,
    ) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            method: METHOD_PROGRESS.to_owned(),
            params: ProgressParams {
                progress_token,
                progress,
                total,
                message,
            },
        }
    }

    /// Create a new cancellation notification
    #[must_use]
    pub fn cancelled(progress_token: String, message: Option<String>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            method: METHOD_CANCELLED.to_owned(),
            params: ProgressParams {
                progress_token,
                progress: 0.0,
                total: None,
                message,
            },
        }
    }
}

/// OAuth completion notification for MCP clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthCompletedNotification {
    /// JSON-RPC version ("2.0")
    pub jsonrpc: String,
    /// Method name ("notifications/oauth/completed")
    pub method: String,
    /// OAuth completion parameters
    pub params: OAuthCompletedParams,
}

/// OAuth completion notification parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthCompletedParams {
    /// OAuth provider name (e.g., "strava", "google")
    pub provider: String,
    /// Whether the OAuth flow completed successfully
    pub success: bool,
    /// Human-readable status message
    pub message: String,
    /// User ID if authentication succeeded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

impl OAuthCompletedNotification {
    /// Create a new OAuth completion notification
    #[must_use]
    pub fn new(provider: String, success: bool, message: String, user_id: Option<String>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            method: METHOD_OAUTH_COMPLETED.to_owned(),
            params: OAuthCompletedParams {
                provider,
                success,
                message,
                user_id,
            },
        }
    }
}

/// Get all available tools (public interface for tests)
#[must_use]
pub fn get_tools() -> Vec<ToolSchema> {
    create_fitness_tools()
}

/// Create all fitness provider tool schemas
fn create_fitness_tools() -> Vec<ToolSchema> {
    vec![
        // Connection tools
        create_connect_to_pierre_tool(),
        create_connect_provider_tool(),
        create_get_connection_status_tool(),
        create_disconnect_provider_tool(),
        // Original tools
        create_get_activities_tool(),
        create_get_athlete_tool(),
        create_get_stats_tool(),
        create_get_activity_intelligence_tool(),
        create_get_notifications_tool(),
        create_mark_notifications_read_tool(),
        create_announce_oauth_success_tool(),
        create_check_oauth_notifications_tool(),
        // Advanced Analytics Tools
        create_analyze_activity_tool(),
        create_calculate_metrics_tool(),
        create_analyze_performance_trends_tool(),
        create_compare_activities_tool(),
        create_detect_patterns_tool(),
        create_set_goal_tool(),
        create_track_progress_tool(),
        create_suggest_goals_tool(),
        create_analyze_goal_feasibility_tool(),
        create_generate_recommendations_tool(),
        create_calculate_fitness_score_tool(),
        create_predict_performance_tool(),
        create_analyze_training_load_tool(),
        // Configuration Management Tools
        create_get_configuration_catalog_tool(),
        create_get_configuration_profiles_tool(),
        create_get_user_configuration_tool(),
        create_update_user_configuration_tool(),
        create_calculate_personalized_zones_tool(),
        create_validate_configuration_tool(),
        // Fitness Configuration Management Tools
        create_get_fitness_config_tool(),
        create_set_fitness_config_tool(),
        create_list_fitness_configs_tool(),
        create_delete_fitness_config_tool(),
    ]
}

/// Create the `get_activities` tool schema
fn create_get_activities_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        PROVIDER.to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name (e.g., 'strava', 'fitbit')".into()),
        },
    );

    properties.insert(
        LIMIT.to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Maximum number of activities to return".into()),
        },
    );

    properties.insert(
        OFFSET.to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Number of activities to skip (for pagination)".into()),
        },
    );

    ToolSchema {
        name: GET_ACTIVITIES.to_owned(),
        description: "Get fitness activities from a provider".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![PROVIDER.to_owned()]),
        },
    }
}

/// Create the `get_athlete` tool schema
fn create_get_athlete_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        PROVIDER.to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name (e.g., 'strava', 'fitbit')".into()),
        },
    );

    ToolSchema {
        name: GET_ATHLETE.to_owned(),
        description: "Get athlete profile from a provider".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![PROVIDER.to_owned()]),
        },
    }
}

/// Create the `get_stats` tool schema
fn create_get_stats_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        PROVIDER.to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name (e.g., 'strava', 'fitbit')".into()),
        },
    );

    ToolSchema {
        name: GET_STATS.to_owned(),
        description: "Get fitness statistics from a provider".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![PROVIDER.to_owned()]),
        },
    }
}

/// Create the `get_activity_intelligence` tool schema
fn create_get_activity_intelligence_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        PROVIDER.to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name (e.g., 'strava', 'fitbit')".into()),
        },
    );

    properties.insert(
        ACTIVITY_ID.to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("ID of the specific activity to analyze".into()),
        },
    );

    properties.insert(
        "include_weather".into(),
        PropertySchema {
            property_type: "boolean".into(),
            description: Some("Whether to include weather analysis (default: true)".into()),
        },
    );

    properties.insert(
        "include_location".into(),
        PropertySchema {
            property_type: "boolean".into(),
            description: Some("Whether to include location intelligence (default: true)".into()),
        },
    );

    ToolSchema {
        name: GET_ACTIVITY_INTELLIGENCE.to_owned(),
        description: "Generate AI-powered insights and analysis for a specific activity".to_owned(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![PROVIDER.to_owned(), ACTIVITY_ID.to_owned()]),
        },
    }
}

/// Create the `connect_to_pierre` tool schema
fn create_connect_to_pierre_tool() -> ToolSchema {
    let properties = HashMap::new(); // No parameters needed for this tool

    ToolSchema {
        name: CONNECT_TO_PIERRE.to_owned(),
        description: "Connect to Pierre - Authenticate with Pierre Fitness Server to access your fitness data. This will open a browser window for secure login. Use this when you're not connected or need to reconnect.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![]), // No required fields
        },
    }
}

/// Create the `connect_provider` tool schema
fn create_connect_provider_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    // Provider parameter (required)
    properties.insert(
        "provider".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Fitness provider to connect to. Supported providers: 'strava', 'fitbit'".into(),
            ),
        },
    );

    ToolSchema {
        name: CONNECT_PROVIDER.to_owned(),
        description: "Connect to Fitness Provider - Unified authentication flow that connects you to both Pierre and a fitness provider (like Strava or Fitbit) in a single seamless process. This will open a browser window for secure authentication with both systems.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["provider".to_owned()]),
        },
    }
}

/// Create the `get_connection_status` tool schema
fn create_get_connection_status_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    // Optional OAuth credentials for Strava
    properties.insert(
        "strava_client_id".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Optional: Your Strava OAuth client ID. If provided with client_secret, will be used instead of server defaults.".into()),
        },
    );

    properties.insert(
        "strava_client_secret".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Optional: Your Strava OAuth client secret. Must be provided with client_id."
                    .into(),
            ),
        },
    );

    // Optional OAuth credentials for Fitbit
    properties.insert(
        "fitbit_client_id".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Optional: Your Fitbit OAuth client ID. If provided with client_secret, will be used instead of server defaults.".into()),
        },
    );

    properties.insert(
        "fitbit_client_secret".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Optional: Your Fitbit OAuth client secret. Must be provided with client_id."
                    .into(),
            ),
        },
    );

    ToolSchema {
        name: GET_CONNECTION_STATUS.to_owned(),
        description: "Check which fitness providers are currently connected and authorized for the user. Returns connection status for all supported providers. Optionally accepts OAuth credentials to use custom apps instead of server defaults.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![]),
        },
    }
}

/// Create the `disconnect_provider` tool schema
fn create_disconnect_provider_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        PROVIDER.to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider to disconnect (e.g., 'strava', 'fitbit')".into()),
        },
    );

    ToolSchema {
        name: DISCONNECT_PROVIDER.to_owned(),
        description: "Disconnect and remove stored tokens for a specific fitness provider. This revokes access to the provider's data.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![PROVIDER.to_owned()]),
        },
    }
}

/// Create mark notifications read tool schema
fn create_mark_notifications_read_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "notification_id".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("ID of specific notification to mark as read (optional - if not provided, marks all as read)".into()),
        },
    );

    ToolSchema {
        name: MARK_NOTIFICATIONS_READ.to_owned(),
        description: "Mark OAuth notifications as read. Provide notification_id to mark specific notification, or omit to mark all unread notifications as read.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![]), // No required fields - can mark all or specific
        },
    }
}

/// Create announce OAuth success tool schema
fn create_announce_oauth_success_tool() -> ToolSchema {
    let mut properties = HashMap::new();
    properties.insert(
        PROVIDER.to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("OAuth provider name (e.g., 'strava', 'fitbit')".into()),
        },
    );
    properties.insert(
        "message".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Success message to display to user".into()),
        },
    );
    properties.insert(
        "notification_id".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Original notification ID that triggered this announcement".into()),
        },
    );
    ToolSchema {
        name: ANNOUNCE_OAUTH_SUCCESS.to_owned(),
        description: "Announce OAuth connection success directly in chat so users can see it. This tool will display a visible message when OAuth authentication completes.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![PROVIDER.to_owned(), "message".to_owned(), "notification_id".to_owned()]),
        },
    }
}

/// Create get notifications tool schema
fn create_get_notifications_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "include_read".to_owned(),
        PropertySchema {
            property_type: "boolean".into(),
            description: Some(
                "Whether to include already read notifications (default: false)".into(),
            ),
        },
    );

    properties.insert(
        "provider".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Filter notifications by provider (optional - e.g., 'strava', 'fitbit')".into(),
            ),
        },
    );

    ToolSchema {
        name: GET_NOTIFICATIONS.to_owned(),
        description: "Get OAuth notifications for the user. By default returns only unread notifications. Optionally filter by provider.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![]), // No required fields
        },
    }
}

// === ADVANCED ANALYTICS TOOLS ===

/// Create the `analyze_activity` tool schema
fn create_analyze_activity_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        PROVIDER.to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name (e.g., 'strava', 'fitbit')".into()),
        },
    );

    properties.insert(
        ACTIVITY_ID.to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("ID of the activity to analyze".into()),
        },
    );

    ToolSchema {
        name: ANALYZE_ACTIVITY.to_owned(),
        description: "Perform deep analysis of an individual activity including insights, metrics, and anomaly detection".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![PROVIDER.to_owned(), ACTIVITY_ID.to_owned()]),
        },
    }
}

/// Create the `calculate_metrics` tool schema
fn create_calculate_metrics_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "provider".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name".into()),
        },
    );

    properties.insert(
        "activity_id".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("ID of the activity".into()),
        },
    );

    properties.insert(
        "metrics".into(),
        PropertySchema {
            property_type: "array".into(),
            description: Some(
                "Specific metrics to calculate (e.g., ['trimp', 'power_to_weight', 'efficiency'])"
                    .to_owned(),
            ),
        },
    );

    ToolSchema {
        name: "calculate_metrics".into(),
        description: "Calculate advanced fitness metrics for an activity (TRIMP, power-to-weight ratio, efficiency scores, etc.)".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["provider".into(), "activity_id".into()]),
        },
    }
}

/// Create the `analyze_performance_trends` tool schema
fn create_analyze_performance_trends_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "provider".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name".into()),
        },
    );

    properties.insert(
        "timeframe".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Time period for analysis ('week', 'month', 'quarter', 'sixmonths', 'year')"
                    .to_owned(),
            ),
        },
    );

    properties.insert("metric".into(), PropertySchema {
        property_type: "string".into(),
        description: Some("Metric to analyze trends for ('pace', 'heart_rate', 'power', 'distance', 'duration')".into()),
    });

    properties.insert(
        "sport_type".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Filter by sport type (optional)".into()),
        },
    );

    ToolSchema {
        name: "analyze_performance_trends".into(),
        description: "Analyze performance trends over time with statistical analysis and insights"
            .to_owned(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["provider".into(), "timeframe".into(), "metric".into()]),
        },
    }
}

/// Create the `compare_activities` tool schema
fn create_compare_activities_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "provider".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name".into()),
        },
    );

    properties.insert(
        "activity_id".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Primary activity to compare".into()),
        },
    );

    properties.insert(
        "comparison_type".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Type of comparison ('similar_activities', 'personal_best', 'average', 'recent')"
                    .to_owned(),
            ),
        },
    );

    ToolSchema {
        name: "compare_activities".into(),
        description:
            "Compare an activity against similar activities, personal bests, or historical averages"
                .to_owned(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![
                "provider".into(),
                "activity_id".into(),
                "comparison_type".into(),
            ]),
        },
    }
}

/// Create the `detect_patterns` tool schema
fn create_detect_patterns_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "provider".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name".into()),
        },
    );

    properties.insert("pattern_type".into(), PropertySchema {
        property_type: "string".into(),
        description: Some("Type of pattern to detect ('training_consistency', 'seasonal_trends', 'performance_plateaus', 'injury_risk')".into()),
    });

    properties.insert(
        "timeframe".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Time period for pattern analysis".into()),
        },
    );

    ToolSchema {
        name: "detect_patterns".into(),
        description: "Detect patterns in training data such as consistency trends, seasonal variations, or performance plateaus".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["provider".into(), "pattern_type".into()]),
        },
    }
}

/// Create the `set_goal` tool schema
fn create_set_goal_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "title".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Goal title".into()),
        },
    );

    properties.insert(
        "description".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Goal description".into()),
        },
    );

    properties.insert(
        "goal_type".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Type of goal ('distance', 'time', 'frequency', 'performance', 'custom')"
                    .to_owned(),
            ),
        },
    );

    properties.insert(
        "target_value".into(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Target value to achieve".into()),
        },
    );

    properties.insert(
        "target_date".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Target completion date (ISO format)".into()),
        },
    );

    properties.insert(
        "sport_type".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Sport type for the goal".into()),
        },
    );

    ToolSchema {
        name: "set_goal".into(),
        description: "Create and manage fitness goals with tracking and progress monitoring"
            .to_owned(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![
                "title".into(),
                "goal_type".into(),
                "target_value".into(),
                "target_date".into(),
            ]),
        },
    }
}

/// Create the `track_progress` tool schema
fn create_track_progress_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "goal_id".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("ID of the goal to track".into()),
        },
    );

    ToolSchema {
        name: "track_progress".into(),
        description: "Track progress toward a specific goal with milestone achievements and completion estimates".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["goal_id".into()]),
        },
    }
}

/// Create the `suggest_goals` tool schema
fn create_suggest_goals_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "provider".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name".into()),
        },
    );

    properties.insert(
        "goal_category".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Category of goals to suggest ('distance', 'performance', 'consistency', 'all')"
                    .to_owned(),
            ),
        },
    );

    ToolSchema {
        name: "suggest_goals".into(),
        description: "Generate AI-powered goal suggestions based on user's activity history and fitness level".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["provider".into()]),
        },
    }
}

/// Create the `analyze_goal_feasibility` tool schema
fn create_analyze_goal_feasibility_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "goal_id".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("ID of the goal to analyze".into()),
        },
    );

    ToolSchema {
        name: "analyze_goal_feasibility".into(),
        description: "Assess whether a goal is realistic and achievable based on current performance and timeline".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["goal_id".into()]),
        },
    }
}

/// Create the `generate_recommendations` tool schema
fn create_generate_recommendations_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "provider".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name".into()),
        },
    );

    properties.insert(
        "recommendation_type".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Type of recommendations ('training', 'recovery', 'nutrition', 'equipment', 'all')"
                    .to_owned(),
            ),
        },
    );

    properties.insert(
        "activity_id".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Specific activity to base recommendations on (optional)".into()),
        },
    );

    ToolSchema {
        name: "generate_recommendations".into(),
        description:
            "Generate personalized training recommendations based on activity data and user profile"
                .to_owned(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["provider".into()]),
        },
    }
}

/// Create the `calculate_fitness_score` tool schema
fn create_calculate_fitness_score_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "provider".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name".into()),
        },
    );

    properties.insert(
        "timeframe".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Time period for fitness assessment ('month', 'quarter', 'sixmonths')".into(),
            ),
        },
    );

    ToolSchema {
        name: "calculate_fitness_score".into(),
        description: "Calculate comprehensive fitness score based on recent training load, consistency, and performance trends".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["provider".into()]),
        },
    }
}

/// Create the `predict_performance` tool schema
fn create_predict_performance_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "provider".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name".into()),
        },
    );

    properties.insert(
        "target_sport".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Sport type for prediction".into()),
        },
    );

    properties.insert(
        "target_distance".into(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Target distance for performance prediction".into()),
        },
    );

    properties.insert(
        "target_date".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Target date for prediction (ISO format)".into()),
        },
    );

    ToolSchema {
        name: "predict_performance".into(),
        description: "Predict future performance capabilities based on current fitness trends and training history".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["provider".into(), "target_sport".into(), "target_distance".into()]),
        },
    }
}

/// Create the `analyze_training_load` tool schema
fn create_analyze_training_load_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "provider".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name".into()),
        },
    );

    properties.insert(
        "timeframe".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Time period for load analysis ('week', 'month', 'quarter')".into()),
        },
    );

    ToolSchema {
        name: "analyze_training_load".into(),
        description:
            "Analyze training load balance, recovery needs, and load distribution over time"
                .to_owned(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["provider".into()]),
        },
    }
}

/// Create the `get_configuration_catalog` tool schema
fn create_get_configuration_catalog_tool() -> ToolSchema {
    ToolSchema {
        name: "get_configuration_catalog".into(),
        description: "Get the complete configuration catalog with all available parameters and their metadata".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(HashMap::new()),
            required: Some(vec![]),
        },
    }
}

/// Create the `get_configuration_profiles` tool schema
fn create_get_configuration_profiles_tool() -> ToolSchema {
    ToolSchema {
        name: "get_configuration_profiles".into(),
        description: "Get available configuration profiles (Research, Elite, Recreational, Beginner, Medical, etc.)".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(HashMap::new()),
            required: Some(vec![]),
        },
    }
}

/// Create the `get_user_configuration` tool schema
fn create_get_user_configuration_tool() -> ToolSchema {
    ToolSchema {
        name: "get_user_configuration".into(),
        description:
            "Get current user's configuration including active profile and parameter overrides"
                .to_owned(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(HashMap::new()),
            required: Some(vec![]),
        },
    }
}

/// Create the `update_user_configuration` tool schema
fn create_update_user_configuration_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "profile".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Configuration profile to apply (optional)".into()),
        },
    );

    properties.insert(
        "parameters".into(),
        PropertySchema {
            property_type: "object".into(),
            description: Some("Parameter overrides to apply (optional)".into()),
        },
    );

    ToolSchema {
        name: "update_user_configuration".into(),
        description: "Update user's configuration by applying a profile and/or parameter overrides"
            .to_owned(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![]),
        },
    }
}

/// Create the `calculate_personalized_zones` tool schema
fn create_calculate_personalized_zones_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "vo2_max".into(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("VO2 max in ml/kg/min".into()),
        },
    );

    properties.insert(
        "resting_hr".into(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Resting heart rate in bpm (optional, defaults to 60)".into()),
        },
    );

    properties.insert(
        "max_hr".into(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Maximum heart rate in bpm (optional, defaults to 190)".into()),
        },
    );

    properties.insert(
        "lactate_threshold".into(),
        PropertySchema {
            property_type: "number".into(),
            description: Some(
                "Lactate threshold as percentage of VO2 max (optional, defaults to 0.85)"
                    .to_owned(),
            ),
        },
    );

    properties.insert(
        "sport_efficiency".into(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Sport efficiency factor (optional, defaults to 1.0)".into()),
        },
    );

    ToolSchema {
        name: "calculate_personalized_zones".into(),
        description: "Calculate personalized training zones (heart rate, pace, power) based on VO2 max and physiological parameters".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["vo2_max".into()]),
        },
    }
}

/// Create the `validate_configuration` tool schema
fn create_validate_configuration_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "parameters".into(),
        PropertySchema {
            property_type: "object".into(),
            description: Some("Configuration parameters to validate".into()),
        },
    );

    ToolSchema {
        name: "validate_configuration".into(),
        description:
            "Validate configuration parameters for physiological limits and scientific bounds"
                .to_owned(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["parameters".into()]),
        },
    }
}

/// Create check OAuth notifications tool schema
fn create_check_oauth_notifications_tool() -> ToolSchema {
    ToolSchema {
        name: CHECK_OAUTH_NOTIFICATIONS.to_owned(),
        description: "Check for new OAuth completion notifications and display them to the user. This tool will announce any successful OAuth connections that happened recently.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: None,
            required: None,
        },
    }
}

// === FITNESS CONFIGURATION TOOLS ===

/// Create the `get_fitness_config` tool schema
fn create_get_fitness_config_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "configuration_name".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Name of the fitness configuration to retrieve (defaults to 'default')".into(),
            ),
        },
    );

    ToolSchema {
        name: GET_FITNESS_CONFIG.to_owned(),
        description: "Get fitness configuration settings including heart rate zones, power zones, and training parameters".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![]), // configuration_name is optional
        },
    }
}

/// Create the `set_fitness_config` tool schema
fn create_set_fitness_config_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "configuration_name".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Name of the fitness configuration to save (defaults to 'default')".into(),
            ),
        },
    );

    properties.insert(
        "configuration".to_owned(),
        PropertySchema {
            property_type: "object".into(),
            description: Some("Fitness configuration object containing zones, thresholds, and training parameters".into()),
        },
    );

    ToolSchema {
        name: SET_FITNESS_CONFIG.to_owned(),
        description: "Save fitness configuration settings for heart rate zones, power zones, and training parameters".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["configuration".to_owned()]), // configuration is required
        },
    }
}

/// Create the `list_fitness_configs` tool schema
fn create_list_fitness_configs_tool() -> ToolSchema {
    ToolSchema {
        name: LIST_FITNESS_CONFIGS.to_owned(),
        description: "List all available fitness configuration names for the user".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(HashMap::new()),
            required: Some(vec![]),
        },
    }
}

/// Create the `delete_fitness_config` tool schema
fn create_delete_fitness_config_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "configuration_name".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Name of the fitness configuration to delete".into()),
        },
    );

    ToolSchema {
        name: DELETE_FITNESS_CONFIG.to_owned(),
        description: "Delete a specific fitness configuration by name".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["configuration_name".to_owned()]),
        },
    }
}
