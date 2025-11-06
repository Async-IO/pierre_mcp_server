// ABOUTME: Protocol handlers module providing MCP, A2A, and REST API interfaces
// ABOUTME: Unified entry point for all communication protocols supported by Pierre server
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! # Universal Protocol Support
//!
//! This module provides a universal interface for executing tools
//! across different protocols (MCP, A2A) supported by Pierre.

/// Protocol conversion utilities
pub mod converter;
/// Universal protocol abstractions
pub mod universal;

/// Protocol converter for translating between protocols
pub use converter::ProtocolConverter;
/// Protocol type enumeration
pub use converter::ProtocolType;
/// Universal request structure
pub use universal::UniversalRequest;
/// Universal response structure
pub use universal::UniversalResponse;
/// Universal tool definition
pub use universal::UniversalTool;
/// Universal tool executor
pub use universal::UniversalToolExecutor;

/// Common error types for protocol operations
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    /// The requested protocol is not supported (e.g., unknown protocol type)
    #[error("Unsupported protocol: {0}")]
    UnsupportedProtocol(String),

    /// The requested tool does not exist
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    /// The provided parameters are invalid for the tool
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    /// Configuration error occurred during protocol setup
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Tool execution failed with an error
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    /// Failed to convert between protocol formats
    #[error("Conversion failed: {0}")]
    ConversionFailed(String),

    /// Failed to serialize or deserialize data
    #[error("Serialization failed: {0}")]
    SerializationError(String),

    /// Database operation failed
    #[error("Database operation failed: {0}")]
    DatabaseError(String),

    /// The requested plugin was not found
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    /// Error occurred while executing a plugin
    #[error("Plugin error: {0}")]
    PluginError(String),

    /// The provided schema is invalid
    #[error("Invalid schema: {0}")]
    InvalidSchema(String),

    /// User's subscription tier is insufficient for the requested operation
    #[error("Insufficient subscription tier: {0}")]
    InsufficientSubscription(String),

    /// Rate limit has been exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    /// The request is malformed or invalid
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// An internal server error occurred
    #[error("Internal error: {0}")]
    InternalError(String),
}
