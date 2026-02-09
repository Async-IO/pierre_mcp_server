// ABOUTME: Protocol error types for MCP and A2A protocol operations
// ABOUTME: Defines ProtocolType enum and ProtocolError with structured context

use std::error::Error;

use super::database::DatabaseError;

/// Supported protocol types
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolType {
    /// Model Context Protocol
    MCP,
    /// Agent-to-Agent protocol
    A2A,
}

/// Common error types for protocol operations
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    /// The requested protocol is not supported
    #[error("Unsupported protocol: {protocol:?}")]
    UnsupportedProtocol {
        /// Protocol type that is not supported
        protocol: ProtocolType,
    },

    /// The requested tool does not exist
    #[error("Tool '{tool_id}' not found. Available tools: {available_count}")]
    ToolNotFound {
        /// ID of the tool that was not found
        tool_id: String,
        /// Number of available tools
        available_count: usize,
    },

    /// Invalid parameter for tool
    #[error("Invalid parameter '{parameter}' for tool '{tool_id}': {reason}")]
    InvalidParameter {
        /// ID of the tool
        tool_id: String,
        /// Name of the invalid parameter
        parameter: &'static str,
        /// Reason why the parameter is invalid
        reason: &'static str,
    },

    /// Missing required parameter
    #[error("Missing required parameter '{parameter}' for tool '{tool_id}'")]
    MissingParameter {
        /// ID of the tool
        tool_id: String,
        /// Name of the missing parameter
        parameter: &'static str,
    },

    /// Configuration error occurred during protocol setup
    #[error("Missing configuration: {key}")]
    ConfigMissing {
        /// Configuration key that is missing
        key: &'static str,
    },

    /// Failed to convert between protocol formats
    #[error("Conversion failed from {from:?} to {to:?}: {reason}")]
    ConversionFailed {
        /// Source protocol type
        from: ProtocolType,
        /// Target protocol type
        to: ProtocolType,
        /// Reason for conversion failure
        reason: &'static str,
    },

    /// Serialization error
    #[error("Serialization failed for {context}")]
    Serialization {
        /// Context where serialization failed
        context: &'static str,
        /// Underlying JSON error
        #[source]
        source: serde_json::Error,
    },

    /// Database error during protocol operation
    #[error("Database error during protocol operation")]
    Database {
        /// Underlying database error
        #[from]
        source: DatabaseError,
    },

    /// Plugin not found
    #[error("Plugin '{plugin_id}' not found")]
    PluginNotFound {
        /// ID of the plugin that was not found
        plugin_id: String,
    },

    /// Plugin execution error
    #[error("Plugin '{plugin_id}' error: {details}")]
    PluginError {
        /// ID of the plugin
        plugin_id: String,
        /// Error details
        details: String,
    },

    /// Invalid schema
    #[error("Invalid schema for {entity}: {reason}")]
    InvalidSchema {
        /// Entity with invalid schema
        entity: &'static str,
        /// Reason why schema is invalid
        reason: String,
    },

    /// User's subscription tier is insufficient
    #[error("Insufficient subscription tier: requires {required}, has {current}")]
    InsufficientSubscription {
        /// Required subscription tier
        required: String,
        /// Current subscription tier
        current: String,
    },

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {requests} requests in {window_secs}s")]
    RateLimitExceeded {
        /// Number of requests made
        requests: u32,
        /// Time window in seconds
        window_secs: u32,
    },

    /// Invalid request structure (detailed)
    #[error("Invalid {protocol:?} request: {reason}")]
    InvalidRequestDetailed {
        /// Protocol type
        protocol: ProtocolType,
        /// Reason why request is invalid
        reason: String,
    },

    /// Invalid request (simple)
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Configuration error (detailed)
    #[error("Configuration error: {message}")]
    ConfigurationErrorDetailed {
        /// Error message
        message: String,
    },

    /// Configuration error (simple)
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Serialization error with structured details
    #[error("Serialization failed: {message}")]
    SerializationErrorDetailed {
        /// Error message
        message: String,
    },

    /// Serialization error (simple)
    #[error("Serialization failed: {0}")]
    SerializationError(String),

    /// Invalid parameters (simple)
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    /// Tool execution failed (detailed)
    #[error("Tool '{tool_id}' execution failed")]
    ExecutionFailedDetailed {
        /// ID of the tool that failed
        tool_id: String,
        /// Underlying error
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },

    /// Tool execution failed (simple)
    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    /// Internal server error
    #[error("Internal error: {0}")]
    InternalError(String),

    /// Operation was cancelled by user request
    #[error("Operation cancelled: {0}")]
    OperationCancelled(String),
}
