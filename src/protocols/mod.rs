// ABOUTME: Protocol handlers module providing MCP, A2A, and REST API interfaces
// ABOUTME: Unified entry point for all communication protocols supported by Pierre server
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! # Universal Protocol Support
//!
//! This module provides a universal interface for executing tools
//! across different protocols (MCP, A2A) supported by Pierre.

pub mod converter;
pub mod universal;

pub use converter::{ProtocolConverter, ProtocolType};
pub use universal::{UniversalRequest, UniversalResponse, UniversalTool, UniversalToolExecutor};

/// Common error types for protocol operations
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Unsupported protocol: {0}")]
    UnsupportedProtocol(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Conversion failed: {0}")]
    ConversionFailed(String),

    #[error("Serialization failed: {0}")]
    SerializationError(String),

    #[error("Database operation failed: {0}")]
    DatabaseError(String),

    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    #[error("Plugin error: {0}")]
    PluginError(String),

    #[error("Invalid schema: {0}")]
    InvalidSchema(String),

    #[error("Insufficient subscription tier: {0}")]
    InsufficientSubscription(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}
