// ABOUTME: MCP protocol schema definitions and message structures
// ABOUTME: Defines JSON-RPC protocol schemas for Model Context Protocol communication
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! MCP Protocol Schema Definitions
//!
//! This module contains type-safe definitions for all MCP protocol messages,
//! capabilities, and tool schemas. This ensures protocol compliance and makes
//! it easy to modify the schema without hardcoding JSON.

use crate::constants::{
    get_server_config,
    json_fields::{ACTIVITY_ID, AFTER, BEFORE, FORMAT, LIMIT, MODE, OFFSET, PROVIDER, SPORT_TYPE},
    tools::{
        ANALYZE_ACTIVITY, CONNECT_PROVIDER, DELETE_FITNESS_CONFIG, DELETE_RECIPE,
        DISCONNECT_PROVIDER, GET_ACTIVITIES, GET_ACTIVITY_INTELLIGENCE, GET_ATHLETE,
        GET_CONNECTION_STATUS, GET_FITNESS_CONFIG, GET_RECIPE, GET_RECIPE_CONSTRAINTS, GET_STATS,
        LIST_FITNESS_CONFIGS, LIST_RECIPES, SAVE_RECIPE, SEARCH_RECIPES, SET_FITNESS_CONFIG,
        VALIDATE_RECIPE,
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
    /// Server completion (auto-complete) capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion: Option<CompletionCapability>,
    /// Server sampling (LLM calls) capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapability>,
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

/// Completion (auto-complete) capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionCapability {}

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
    pub oauth_credentials: Option<HashMap<String, OAuthAppCredentials>>,
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
                        let host = get_server_config()
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
                    let host = get_server_config()
                        .map_or_else(|| "localhost".to_owned(), |c| c.host.clone());
                    OAuth2Capability {
                        discovery_url: format!("http://{host}:{http_port}/.well-known/oauth-authorization-server"),
                        authorization_endpoint: format!("http://{host}:{http_port}/oauth2/authorize"),
                        token_endpoint: format!("http://{host}:{http_port}/oauth2/token"),
                        registration_endpoint: format!("http://{host}:{http_port}/oauth2/register"),
                    }
                }),
                completion: Some(CompletionCapability {}),
                sampling: Some(SamplingCapability {}),
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

// === MCP SAMPLING (LLM CALL) TYPES ===

/// Request to create a message using the client's LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessageRequest {
    /// Messages to send to the LLM
    pub messages: Vec<PromptMessage>,
    /// Optional model preferences
    #[serde(rename = "modelPreferences", skip_serializing_if = "Option::is_none")]
    pub model_preferences: Option<ModelPreferences>,
    /// Optional system prompt
    #[serde(rename = "systemPrompt", skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// Include context from MCP servers
    #[serde(rename = "includeContext", skip_serializing_if = "Option::is_none")]
    pub include_context: Option<String>,
    /// Maximum tokens to generate
    #[serde(rename = "maxTokens")]
    pub max_tokens: i32,
    /// Sampling temperature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Stop sequences
    #[serde(rename = "stopSequences", skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Result from create message request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessageResult {
    /// Role of the message (usually "assistant")
    pub role: String,
    /// Content of the generated message
    pub content: MessageContent,
    /// Model that was used
    pub model: String,
    /// Stop reason for completion
    #[serde(rename = "stopReason", skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
}

/// Message content wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContent {
    /// Type of content (usually "text")
    #[serde(rename = "type")]
    pub content_type: String,
    /// Text content
    pub text: String,
}

/// Model preferences for sampling
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelPreferences {
    /// Model hints in preference order
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hints: Option<Vec<ModelHint>>,
    /// Cost priority (0.0-1.0, where 1.0 prefers cheaper models)
    #[serde(rename = "costPriority", skip_serializing_if = "Option::is_none")]
    pub cost_priority: Option<f64>,
    /// Speed priority (0.0-1.0, where 1.0 prefers faster models)
    #[serde(rename = "speedPriority", skip_serializing_if = "Option::is_none")]
    pub speed_priority: Option<f64>,
    /// Intelligence priority (0.0-1.0, where 1.0 prefers more capable models)
    #[serde(
        rename = "intelligencePriority",
        skip_serializing_if = "Option::is_none"
    )]
    pub intelligence_priority: Option<f64>,
}

/// Hint for model selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelHint {
    /// Model name (e.g., "claude-3-5-sonnet")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Prompt message for LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMessage {
    /// Role of the message sender
    pub role: String,
    /// Content of the message
    pub content: Content,
}

impl PromptMessage {
    /// Create a user message
    #[must_use]
    pub fn user(content: Content) -> Self {
        Self {
            role: "user".to_owned(),
            content,
        }
    }

    /// Create an assistant message
    #[must_use]
    pub fn assistant(content: Content) -> Self {
        Self {
            role: "assistant".to_owned(),
            content,
        }
    }
}

// === MCP COMPLETION (AUTO-COMPLETE) TYPES ===

/// Request for completion suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteRequest {
    /// Reference to the item being completed
    #[serde(rename = "ref")]
    pub ref_: CompletionReference,
    /// Current argument being completed
    pub argument: ArgumentValue,
}

/// Reference to completion context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionReference {
    /// Type of reference
    #[serde(rename = "type")]
    pub type_: String,
    /// Name of the tool/resource/prompt
    pub name: String,
}

/// Argument value for completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgumentValue {
    /// Name of the argument
    pub name: String,
    /// Current value being typed
    pub value: String,
}

/// Result from completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteResult {
    /// Completion suggestions
    pub completion: Completion,
}

impl Default for CompleteResult {
    fn default() -> Self {
        Self {
            completion: Completion {
                values: vec![],
                total: Some(0),
                has_more: Some(false),
            },
        }
    }
}

/// Completion suggestion list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Completion {
    /// Suggested completion values
    pub values: Vec<String>,
    /// Total number of possible completions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,
    /// Whether there are more completions available
    #[serde(rename = "hasMore", skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
}

// === MCP ROOTS TYPES ===

/// Root directory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Root {
    /// URI of the root directory
    pub uri: String,
    /// Human-readable name
    pub name: String,
}

/// Get all available tools (public interface for tests)
#[must_use]
pub fn get_tools() -> Vec<ToolSchema> {
    create_fitness_tools()
}

/// Creates the standard format property for output serialization
///
/// This helper ensures consistent format parameter documentation across all
/// data-returning tools. Use this for tools that return substantial data payloads.
fn format_property() -> PropertySchema {
    PropertySchema {
        property_type: "string".into(),
        description: Some(
            "Output serialization format: 'json' (default, universal) or 'toon' (Token-Oriented Object Notation - ~40% fewer tokens, optimized for LLM input). Use 'toon' for large datasets.".into(),
        ),
    }
}

/// Create all fitness provider tool schemas
fn create_fitness_tools() -> Vec<ToolSchema> {
    vec![
        // Connection tools
        // Note: connect_to_pierre removed - SDK bridge handles it locally via RFC 8414 discovery
        create_connect_provider_tool(),
        create_get_connection_status_tool(),
        create_disconnect_provider_tool(),
        // Original tools
        create_get_activities_tool(),
        create_get_athlete_tool(),
        create_get_stats_tool(),
        create_get_activity_intelligence_tool(),
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
        // Nutrition Tools
        create_calculate_daily_nutrition_tool(),
        create_get_nutrient_timing_tool(),
        create_search_food_tool(),
        create_get_food_details_tool(),
        create_analyze_meal_nutrition_tool(),
        // Sleep & Recovery Tools
        create_analyze_sleep_quality_tool(),
        create_calculate_recovery_score_tool(),
        create_suggest_rest_day_tool(),
        create_track_sleep_trends_tool(),
        create_optimize_sleep_schedule_tool(),
        // Recipe Management Tools ("Combat des Chefs" architecture)
        create_get_recipe_constraints_tool(),
        create_validate_recipe_tool(),
        create_save_recipe_tool(),
        create_list_recipes_tool(),
        create_get_recipe_tool(),
        create_delete_recipe_tool(),
        create_search_recipes_tool(),
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
            description: Some("Maximum number of activities to return. Safe limits to avoid context overflow: format=toon + mode=summary: ≤300, format=toon + mode=detailed: ≤30, format=json + mode=summary: ≤150, format=json + mode=detailed: ≤15".into()),
        },
    );

    properties.insert(
        OFFSET.to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Number of activities to skip (for pagination)".into()),
        },
    );

    properties.insert(
        BEFORE.to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some(
                "Unix timestamp (seconds) - return activities before this time".into(),
            ),
        },
    );

    properties.insert(
        AFTER.to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some(
                "Unix timestamp (seconds) - return activities after this time. If not specified, defaults to 90 days ago to prevent context overflow.".into(),
            ),
        },
    );

    properties.insert(
        MODE.to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Response detail level: 'summary' returns compact data (id, name, sport_type, start_date, distance_meters, duration_seconds) - use for listing/browsing many activities. 'detailed' returns full activity data with GPS, segments, laps - use only when analyzing a specific activity. Default: 'summary'. WARNING: 'detailed' mode with many activities will overflow LLM context.".into(),
            ),
        },
    );

    properties.insert(
        SPORT_TYPE.to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Filter by sport type (e.g., 'NordicSki', 'Run', 'Ride', 'Swim'). Case-insensitive. Returns only activities matching this sport type.".into(),
            ),
        },
    );

    properties.insert(FORMAT.to_owned(), format_property());

    ToolSchema {
        name: GET_ACTIVITIES.to_owned(),
        description: "Get fitness activities from a provider. Use mode='summary' (default) for listing activities - returns compact data safe for LLM context. Use mode='detailed' only for single activity analysis. Combine with before/after timestamps and sport_type filter to efficiently query large date ranges. Response metadata includes pagination info (offset, limit, returned_count, has_more) to enable intelligent pagination through large result sets. Response includes token_estimate with estimated_tokens, context_usage_percent, and guidance for managing LLM context limits. Default: 90-day time window applied when 'after' not specified.".into(),
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

    properties.insert(FORMAT.to_owned(), format_property());

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

    properties.insert(FORMAT.to_owned(), format_property());

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

    properties.insert(FORMAT.to_owned(), format_property());

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

    properties.insert(FORMAT.to_owned(), format_property());

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

    properties.insert(FORMAT.to_owned(), format_property());

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

    properties.insert(FORMAT.to_owned(), format_property());

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

    properties.insert(FORMAT.to_owned(), format_property());

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

    properties.insert(FORMAT.to_owned(), format_property());

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

    properties.insert(FORMAT.to_owned(), format_property());

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
            description: Some(
                "Fitness provider for activity data (e.g., 'strava', 'garmin')".into(),
            ),
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

    properties.insert(
        "sleep_provider".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Optional sleep/recovery provider (e.g., 'whoop', 'garmin'). If specified, factors recovery quality into fitness score.".into(),
            ),
        },
    );

    properties.insert(FORMAT.to_owned(), format_property());

    ToolSchema {
        name: "calculate_fitness_score".into(),
        description: "Calculate comprehensive fitness score based on recent training load, consistency, and performance trends. Optionally integrates sleep/recovery data for holistic assessment.".into(),
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

    properties.insert(FORMAT.to_owned(), format_property());

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
            description: Some(
                "Fitness provider for activity data (e.g., 'strava', 'garmin')".into(),
            ),
        },
    );

    properties.insert(
        "timeframe".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Time period for load analysis ('week', 'month', 'quarter')".into()),
        },
    );

    properties.insert(
        "sleep_provider".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Optional sleep/recovery provider (e.g., 'whoop', 'garmin'). If specified, includes recovery metrics in training load analysis.".into(),
            ),
        },
    );

    properties.insert(FORMAT.to_owned(), format_property());

    ToolSchema {
        name: "analyze_training_load".into(),
        description:
            "Analyze training load balance, recovery needs, and load distribution. Optionally integrates sleep/recovery data for holistic load assessment."
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

/// Create the `calculate_daily_nutrition` tool schema
fn create_calculate_daily_nutrition_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "weight_kg".to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Body weight in kilograms".into()),
        },
    );

    properties.insert(
        "height_cm".to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Height in centimeters".into()),
        },
    );

    properties.insert(
        "age".to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Age in years (max 150)".into()),
        },
    );

    properties.insert(
        "gender".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Gender: 'male' or 'female'".into()),
        },
    );

    properties.insert(
        "activity_level".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Activity level: 'sedentary', 'lightly_active', 'moderately_active', 'very_active', or 'extra_active'".into(),
            ),
        },
    );

    properties.insert(
        "training_goal".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Training goal: 'maintenance', 'weight_loss', 'muscle_gain', or 'endurance_performance'".into(),
            ),
        },
    );

    ToolSchema {
        name: "calculate_daily_nutrition".to_owned(),
        description: "Calculate daily calorie and macronutrient needs using Mifflin-St Jeor BMR formula. Returns BMR, TDEE, and macros (protein, carbs, fat) adjusted for training goal.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![
                "weight_kg".to_owned(),
                "height_cm".to_owned(),
                "age".to_owned(),
                "gender".to_owned(),
                "activity_level".to_owned(),
                "training_goal".to_owned(),
            ]),
        },
    }
}

/// Create the `get_nutrient_timing` tool schema
///
/// Supports cross-provider integration: if `activity_provider` is specified,
/// workout intensity is auto-inferred from recent training load.
fn create_get_nutrient_timing_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "weight_kg".to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Body weight in kilograms".into()),
        },
    );

    properties.insert(
        "daily_protein_g".to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Daily protein target in grams".into()),
        },
    );

    properties.insert(
        "workout_intensity".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Workout intensity: 'low', 'moderate', or 'high'. Optional if activity_provider specified (auto-inferred from recent training load).".into()
            ),
        },
    );

    properties.insert(
        "activity_provider".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Fitness provider for activity data (e.g., 'strava', 'garmin'). If provided, workout intensity is auto-inferred from recent training load.".into()
            ),
        },
    );

    properties.insert(
        "days_back".to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some(
                "Number of days of activity history to analyze for intensity inference (default: 7).".into()
            ),
        },
    );

    ToolSchema {
        name: "get_nutrient_timing".to_owned(),
        description: "Get optimal pre-workout and post-workout nutrition recommendations following ISSN (International Society of Sports Nutrition) guidelines. Returns timing windows, macros, and hydration targets. Supports cross-provider integration for automatic workout intensity inference.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            // weight_kg and daily_protein_g always required
            // workout_intensity OR activity_provider required (validated in handler)
            required: Some(vec![
                "weight_kg".to_owned(),
                "daily_protein_g".to_owned(),
            ]),
        },
    }
}

/// Create the `search_food` tool schema
fn create_search_food_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "query".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Food name or description to search for".into()),
        },
    );

    properties.insert(
        "page_size".to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Number of results per page (default: 10, max: 200)".into()),
        },
    );

    properties.insert(
        "page_number".to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Page number to retrieve (1-indexed, default: 1)".into()),
        },
    );

    properties.insert(FORMAT.to_owned(), format_property());

    ToolSchema {
        name: "search_food".to_owned(),
        description: "Search USDA FoodData Central database for foods by name or description. Returns food ID, name, brand, and category for each match.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["query".to_owned()]),
        },
    }
}

/// Create the `get_food_details` tool schema
fn create_get_food_details_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "fdc_id".to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some(
                "USDA FoodData Central ID for the food (from search_food results)".into(),
            ),
        },
    );

    properties.insert(FORMAT.to_owned(), format_property());

    ToolSchema {
        name: "get_food_details".to_owned(),
        description: "Get detailed nutritional information for a specific food from USDA FoodData Central. Returns complete nutrient breakdown including calories, macros, vitamins, and minerals per 100g serving.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["fdc_id".to_owned()]),
        },
    }
}

/// Create the `analyze_meal_nutrition` tool schema
fn create_analyze_meal_nutrition_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "foods".to_owned(),
        PropertySchema {
            property_type: "array".into(),
            description: Some(
                "Array of food items with 'fdc_id' (number) and 'grams' (number) for each food"
                    .into(),
            ),
        },
    );

    properties.insert(FORMAT.to_owned(), format_property());

    ToolSchema {
        name: "analyze_meal_nutrition".to_owned(),
        description: "Analyze total calories and macronutrients for a meal composed of multiple foods. Each food requires USDA FoodData Central ID and portion size in grams. Returns aggregated nutrition totals.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["foods".to_owned()]),
        },
    }
}

/// Create the `analyze_sleep_quality` tool schema
fn create_analyze_sleep_quality_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "sleep_provider".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Provider to fetch sleep data from: 'whoop', 'fitbit', 'garmin', or 'terra'. Auto-fetches most recent night's data.".into(),
            ),
        },
    );

    properties.insert(
        "sleep_data".to_owned(),
        PropertySchema {
            property_type: "object".into(),
            description: Some(
                "Manual sleep data object (used if sleep_provider not specified) with: date (string), duration_hours (number), efficiency_percent (number), deep_sleep_hours (number), rem_sleep_hours (number), light_sleep_hours (number), awakenings (number), hrv_rmssd_ms (number, optional)".into(),
            ),
        },
    );

    properties.insert(
        "recent_hrv_values".to_owned(),
        PropertySchema {
            property_type: "array".into(),
            description: Some(
                "Optional array of recent HRV RMSSD values (numbers) for trend analysis".into(),
            ),
        },
    );

    properties.insert(
        "baseline_hrv".to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Optional baseline HRV RMSSD value for comparison".into()),
        },
    );

    properties.insert(FORMAT.to_owned(), format_property());

    ToolSchema {
        name: "analyze_sleep_quality".to_owned(),
        description: "Analyze sleep quality using NSF/AASM guidelines. Supports two modes: (1) Provider mode - specify 'sleep_provider' to auto-fetch from connected provider (whoop, fitbit, garmin, terra), (2) Manual mode - provide 'sleep_data' JSON. Returns overall score (0-100), stage breakdown, efficiency rating, and HRV trends. Provides recommendations for sleep optimization.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: None, // Either sleep_provider or sleep_data is required
        },
    }
}

/// Create the `calculate_recovery_score` tool schema
fn create_calculate_recovery_score_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "activity_provider".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Provider for activity/training data: 'strava', 'garmin', 'fitbit', 'whoop', or 'terra'. Auto-selects best connected provider if not specified.".into(),
            ),
        },
    );

    properties.insert(
        "sleep_provider".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Provider for sleep/HRV data: 'whoop', 'fitbit', 'garmin', or 'terra'. Auto-fetches most recent sleep data. Auto-selects if not specified.".into(),
            ),
        },
    );

    properties.insert(
        "sleep_data".to_owned(),
        PropertySchema {
            property_type: "object".into(),
            description: Some(
                "Manual sleep data (used if sleep_provider not specified) with: date (string), duration_hours (number), efficiency_percent (number), deep_sleep_hours (number), rem_sleep_hours (number), hrv_rmssd_ms (number, optional)".into(),
            ),
        },
    );

    properties.insert(
        "user_config".to_owned(),
        PropertySchema {
            property_type: "object".into(),
            description: Some(
                "Optional user configuration with: ftp (number), lthr (number), max_hr (number), resting_hr (number), weight_kg (number)".into(),
            ),
        },
    );

    properties.insert(
        "recent_hrv_values".to_owned(),
        PropertySchema {
            property_type: "array".into(),
            description: Some(
                "Optional array of recent HRV RMSSD values for trend analysis".into(),
            ),
        },
    );

    properties.insert(
        "baseline_hrv".to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Optional baseline HRV RMSSD value for comparison".into()),
        },
    );

    properties.insert(FORMAT.to_owned(), format_property());

    ToolSchema {
        name: "calculate_recovery_score".to_owned(),
        description: "Calculate comprehensive recovery score combining Training Stress Balance (TSB), sleep quality, and HRV metrics. Supports cross-provider integration: use 'activity_provider' for training data (e.g., Strava) and 'sleep_provider' for sleep/HRV data (e.g., WHOOP). Auto-selects connected providers if not specified. FALLBACK MODE: If no sleep data is available, provides TSB-only recovery assessment based on training load alone with clear limitations noted. Returns overall score (0-100), recovery category, training readiness, data_completeness indicator, and providers used.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: None, // Auto-selects providers, TSB-only fallback if no sleep data
        },
    }
}

/// Create the `suggest_rest_day` tool schema
fn create_suggest_rest_day_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "activity_provider".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Provider for activity/training data: 'strava', 'garmin', 'fitbit', 'whoop', or 'terra'. Auto-selects if not specified.".into(),
            ),
        },
    );

    properties.insert(
        "sleep_provider".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Provider for sleep/HRV data: 'whoop', 'fitbit', 'garmin', or 'terra'. Auto-selects if not specified.".into(),
            ),
        },
    );

    properties.insert(
        "sleep_data".to_owned(),
        PropertySchema {
            property_type: "object".into(),
            description: Some("Manual sleep data (used if sleep_provider not specified)".into()),
        },
    );

    properties.insert(
        "user_config".to_owned(),
        PropertySchema {
            property_type: "object".into(),
            description: Some(
                "Optional user configuration with: ftp, lthr, max_hr, resting_hr, weight_kg".into(),
            ),
        },
    );

    ToolSchema {
        name: "suggest_rest_day".to_owned(),
        description: "AI-powered rest day recommendation based on training load analysis, recovery metrics, and fatigue indicators. Supports cross-provider integration for comprehensive analysis. Auto-selects connected providers if not specified. FALLBACK MODE: If no sleep data is available, provides TSB-only recommendation based on training load alone with clear limitations noted. Returns whether rest is recommended, confidence level, reasoning, data_completeness indicator, and recovery insights.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: None, // Auto-selects providers, TSB-only fallback if no sleep data
        },
    }
}

/// Create the `track_sleep_trends` tool schema
fn create_track_sleep_trends_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "sleep_provider".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Provider to fetch sleep history from: 'whoop', 'fitbit', 'garmin', or 'terra'. Auto-selects if not specified.".into(),
            ),
        },
    );

    properties.insert(
        "days".to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some(
                "Number of days of sleep history to analyze (default: 14). Minimum 7 days required for trend analysis.".into(),
            ),
        },
    );

    properties.insert(
        "sleep_history".to_owned(),
        PropertySchema {
            property_type: "array".into(),
            description: Some(
                "Manual sleep history array (used if sleep_provider not specified). Each item needs: date (string), duration_hours (number), efficiency_percent (number, optional), deep_sleep_hours (number, optional), rem_sleep_hours (number, optional). Minimum 7 days required.".into(),
            ),
        },
    );

    properties.insert(FORMAT.to_owned(), format_property());

    ToolSchema {
        name: "track_sleep_trends".to_owned(),
        description: "Track sleep patterns over time and identify trends. Supports two modes: (1) Provider mode - specify 'sleep_provider' and 'days' to auto-fetch history, (2) Manual mode - provide 'sleep_history' array. Requires at least 7 days of data. Returns average metrics, trend direction (improving/stable/declining), consistency analysis, and recommendations.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: None, // Either sleep_provider or sleep_history is required
        },
    }
}

/// Create the `optimize_sleep_schedule` tool schema
fn create_optimize_sleep_schedule_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "activity_provider".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Provider for activity/training data: 'strava', 'garmin', 'fitbit', 'whoop', or 'terra'. Auto-selects if not specified.".into(),
            ),
        },
    );

    properties.insert(
        "user_config".to_owned(),
        PropertySchema {
            property_type: "object".into(),
            description: Some(
                "Optional user configuration with: ftp (number), lthr (number), max_hr (number), resting_hr (number), weight_kg (number)".into(),
            ),
        },
    );

    properties.insert(
        "upcoming_workout_intensity".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Intensity of upcoming workout: 'low', 'moderate', or 'high' (default: 'moderate')"
                    .into(),
            ),
        },
    );

    properties.insert(
        "typical_wake_time".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Your typical wake time in 'HH:MM' format (default: '06:00')".into()),
        },
    );

    ToolSchema {
        name: "optimize_sleep_schedule".to_owned(),
        description: "Generate personalized sleep schedule recommendations based on training load, recovery needs, and upcoming workouts. Supports any connected activity provider. Auto-selects provider if not specified. Returns recommended sleep duration, optimal bedtime window, and sleep quality tips tailored to current training phase.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: None, // Auto-selects provider
        },
    }
}

// === RECIPE MANAGEMENT TOOLS ("Combat des Chefs" architecture) ===

/// Create the `get_recipe_constraints` tool schema
fn create_get_recipe_constraints_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "meal_timing".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Training phase for macro targets: 'pre_training', 'post_training', 'rest_day', or 'general'".into(),
            ),
        },
    );

    properties.insert(
        "target_calories".to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some(
                "Target calories for the meal (optional, for portion guidance)".into(),
            ),
        },
    );

    ToolSchema {
        name: GET_RECIPE_CONSTRAINTS.to_owned(),
        description: "Get macro targets and constraints for LLM recipe generation based on training phase. Returns protein/carbs/fat percentages optimized for meal timing (e.g., high carbs pre-training, high protein post-training). Use this before generating recipes to ensure nutrition alignment.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["meal_timing".to_owned()]),
        },
    }
}

/// Create the `validate_recipe` tool schema
fn create_validate_recipe_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "name".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Recipe name".into()),
        },
    );

    properties.insert(
        "servings".to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Number of servings the recipe makes".into()),
        },
    );

    properties.insert(
        "ingredients".to_owned(),
        PropertySchema {
            property_type: "array".into(),
            description: Some(
                "Array of ingredients with: name (string), amount (number), unit (string: 'grams', 'cups', 'tablespoons', 'teaspoons', 'pieces', 'ounces', 'milliliters'), fdc_id (number, optional USDA food ID for validation)".into(),
            ),
        },
    );

    properties.insert(
        "meal_timing".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Intended meal timing: 'pre_training', 'post_training', 'rest_day', or 'general'"
                    .into(),
            ),
        },
    );

    ToolSchema {
        name: VALIDATE_RECIPE.to_owned(),
        description: "Validate a recipe's nutrition against USDA database and calculate per-serving macros. Converts units to grams and looks up ingredients in USDA FoodData Central. Returns validation results with calculated calories, protein, carbs, fat, and any warnings about missing foods or macro targets.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![
                "name".to_owned(),
                "servings".to_owned(),
                "ingredients".to_owned(),
            ]),
        },
    }
}

/// Create the `save_recipe` tool schema
fn create_save_recipe_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "name".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Recipe name".into()),
        },
    );

    properties.insert(
        "description".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Recipe description (optional)".into()),
        },
    );

    properties.insert(
        "servings".to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Number of servings".into()),
        },
    );

    properties.insert(
        "prep_time_mins".to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Preparation time in minutes (optional)".into()),
        },
    );

    properties.insert(
        "cook_time_mins".to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Cooking time in minutes (optional)".into()),
        },
    );

    properties.insert(
        "ingredients".to_owned(),
        PropertySchema {
            property_type: "array".into(),
            description: Some(
                "Array of ingredients with: name (string), amount (number), unit (string), grams (number), fdc_id (number, optional), preparation (string, optional)".into(),
            ),
        },
    );

    properties.insert(
        "instructions".to_owned(),
        PropertySchema {
            property_type: "array".into(),
            description: Some("Array of instruction steps as strings".into()),
        },
    );

    properties.insert(
        "tags".to_owned(),
        PropertySchema {
            property_type: "array".into(),
            description: Some(
                "Array of tags (optional, e.g., ['high-protein', 'quick', 'vegetarian'])".into(),
            ),
        },
    );

    properties.insert(
        "meal_timing".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Meal timing category: 'pre_training', 'post_training', 'rest_day', or 'general'"
                    .into(),
            ),
        },
    );

    properties.insert(
        "cached_nutrition".to_owned(),
        PropertySchema {
            property_type: "object".into(),
            description: Some(
                "Pre-validated nutrition data with: calories, protein_g, carbs_g, fat_g, fiber_g (optional), sodium_mg (optional), sugar_g (optional)".into(),
            ),
        },
    );

    ToolSchema {
        name: SAVE_RECIPE.to_owned(),
        description: "Save a validated recipe to user's personal collection. Should be called after validate_recipe to ensure nutrition data is accurate. Stores recipe with cached nutrition for quick access.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![
                "name".to_owned(),
                "servings".to_owned(),
                "ingredients".to_owned(),
                "instructions".to_owned(),
            ]),
        },
    }
}

/// Create the `list_recipes` tool schema
fn create_list_recipes_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "meal_timing".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Filter by meal timing: 'pre_training', 'post_training', 'rest_day', or 'general' (optional)".into(),
            ),
        },
    );

    properties.insert(
        "limit".to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Maximum number of recipes to return (default: 20)".into()),
        },
    );

    properties.insert(
        "offset".to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Number of recipes to skip for pagination (default: 0)".into()),
        },
    );

    properties.insert(FORMAT.to_owned(), format_property());

    ToolSchema {
        name: LIST_RECIPES.to_owned(),
        description: "List user's saved recipes with optional filtering by meal timing. Returns recipe summaries with name, description, meal timing, and cached nutrition per serving.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![]),
        },
    }
}

/// Create the `get_recipe` tool schema
fn create_get_recipe_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "recipe_id".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("ID of the recipe to retrieve".into()),
        },
    );

    properties.insert(FORMAT.to_owned(), format_property());

    ToolSchema {
        name: GET_RECIPE.to_owned(),
        description: "Get a specific recipe by ID from user's collection. Returns full recipe details including ingredients, instructions, and nutrition data.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["recipe_id".to_owned()]),
        },
    }
}

/// Create the `delete_recipe` tool schema
fn create_delete_recipe_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "recipe_id".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("ID of the recipe to delete".into()),
        },
    );

    ToolSchema {
        name: DELETE_RECIPE.to_owned(),
        description: "Delete a recipe from user's collection. This action cannot be undone.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["recipe_id".to_owned()]),
        },
    }
}

/// Create the `search_recipes` tool schema
fn create_search_recipes_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "query".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Search query for recipe name, description, or tags".into()),
        },
    );

    properties.insert(
        "limit".to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Maximum number of results to return (default: 10, max: 100)".into()),
        },
    );

    properties.insert(
        "offset".to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Number of results to skip (for pagination, default: 0)".into()),
        },
    );

    properties.insert(FORMAT.to_owned(), format_property());

    ToolSchema {
        name: SEARCH_RECIPES.to_owned(),
        description: "Search user's recipes by name, description, or tags. Returns matching recipes with relevance ranking.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["query".to_owned()]),
        },
    }
}
