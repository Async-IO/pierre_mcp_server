// ABOUTME: MCP (Model Context Protocol) server configuration types
// ABOUTME: Handles protocol settings, server identification, and app behavior flags
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::constants::{limits, mcp_transport, network_config, rate_limits};
use crate::errors::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::env;

/// MCP (Model Context Protocol) server configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpConfig {
    /// MCP protocol version
    pub protocol_version: String,
    /// MCP server name
    pub server_name: String,
    /// MCP session cache size
    pub session_cache_size: usize,
    /// Maximum request size in bytes
    pub max_request_size: usize,
    /// Maximum response size in bytes
    pub max_response_size: usize,
    /// Notification broadcast channel size
    pub notification_channel_size: usize,
    /// WebSocket channel capacity
    pub websocket_channel_capacity: usize,
    /// TCP keep-alive timeout in seconds
    pub tcp_keep_alive_secs: u64,
}

impl McpConfig {
    /// Load MCP server configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            protocol_version: env_var_or("MCP_PROTOCOL_VERSION", "2025-11-25"),
            server_name: env_var_or("SERVER_NAME", "pierre-mcp-server"),
            session_cache_size: env::var("MCP_SESSION_CACHE_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
            max_request_size: env::var("MCP_MAX_REQUEST_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(limits::MAX_REQUEST_SIZE),
            max_response_size: env::var("MCP_MAX_RESPONSE_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(limits::MAX_RESPONSE_SIZE),
            notification_channel_size: env::var("MCP_NOTIFICATION_CHANNEL_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(mcp_transport::NOTIFICATION_CHANNEL_SIZE),
            websocket_channel_capacity: env::var("MCP_WEBSOCKET_CHANNEL_CAPACITY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(rate_limits::WEBSOCKET_CHANNEL_CAPACITY),
            tcp_keep_alive_secs: env::var("TCP_KEEP_ALIVE_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(network_config::TCP_KEEP_ALIVE_SECS),
        }
    }
}

/// Protocol configuration for MCP server
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProtocolConfig {
    /// MCP protocol version
    pub mcp_version: String,
    /// Server name
    pub server_name: String,
    /// Server version (from Cargo.toml)
    pub server_version: String,
}

impl ProtocolConfig {
    /// Load protocol configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            mcp_version: env_var_or("MCP_PROTOCOL_VERSION", "2025-11-25"),
            server_name: env_var_or("SERVER_NAME", "pierre-mcp-server"),
            server_version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    }
}

/// Application behavior and feature flags configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppBehaviorConfig {
    /// Maximum activities to fetch in one request
    pub max_activities_fetch: usize,
    /// Default limit for activities queries
    pub default_activities_limit: usize,
    /// Enable CI mode for testing
    pub ci_mode: bool,
    /// Auto-approve new user registrations (skip admin approval workflow)
    pub auto_approve_users: bool,
    /// Whether `auto_approve_users` was explicitly set via environment variable.
    /// When true, the env var value takes precedence over database settings.
    pub auto_approve_users_from_env: bool,
    /// Protocol configuration
    pub protocol: ProtocolConfig,
}

impl AppBehaviorConfig {
    /// Load application behavior configuration from environment
    ///
    /// # Errors
    ///
    /// Returns an error if application behavior environment variables are invalid
    pub fn from_env() -> AppResult<Self> {
        // Check if AUTO_APPROVE_USERS was explicitly set in environment
        let (auto_approve_users, auto_approve_users_from_env) = match env::var("AUTO_APPROVE_USERS")
        {
            Ok(value) => {
                let parsed = value.parse().map_err(|e| {
                    AppError::invalid_input(format!("Invalid AUTO_APPROVE_USERS value: {e}"))
                })?;
                (parsed, true)
            }
            Err(_) => (false, false),
        };

        Ok(Self {
            max_activities_fetch: env_var_or("MAX_ACTIVITIES_FETCH", "100").parse().map_err(
                |e| AppError::invalid_input(format!("Invalid MAX_ACTIVITIES_FETCH value: {e}")),
            )?,
            default_activities_limit: env_var_or("DEFAULT_ACTIVITIES_LIMIT", "20")
                .parse()
                .map_err(|e| {
                    AppError::invalid_input(format!("Invalid DEFAULT_ACTIVITIES_LIMIT value: {e}"))
                })?,
            ci_mode: env_var_or("CI", "false")
                .parse()
                .map_err(|e| AppError::invalid_input(format!("Invalid CI value: {e}")))?,
            auto_approve_users,
            auto_approve_users_from_env,
            protocol: ProtocolConfig::from_env(),
        })
    }
}

/// Tokio runtime configuration for controlling async execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokioRuntimeConfig {
    /// Number of worker threads (defaults to CPU count)
    /// Set via `TOKIO_WORKER_THREADS` environment variable
    pub worker_threads: Option<usize>,
    /// Thread stack size in bytes (defaults to Tokio default ~2MB)
    /// Set via `TOKIO_THREAD_STACK_SIZE` environment variable
    pub thread_stack_size: Option<usize>,
    /// Thread name prefix for worker threads
    pub thread_name: String,
    /// Enable I/O driver (should almost always be true)
    pub enable_io: bool,
    /// Enable time driver for timeouts and intervals
    pub enable_time: bool,
}

impl Default for TokioRuntimeConfig {
    fn default() -> Self {
        Self {
            worker_threads: None,    // Use Tokio default (CPU count)
            thread_stack_size: None, // Use Tokio default
            thread_name: "pierre-worker".to_owned(),
            enable_io: true,
            enable_time: true,
        }
    }
}

impl TokioRuntimeConfig {
    /// Load from environment variables
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            worker_threads: env::var("TOKIO_WORKER_THREADS")
                .ok()
                .and_then(|s| s.parse().ok()),
            thread_stack_size: env::var("TOKIO_THREAD_STACK_SIZE")
                .ok()
                .and_then(|s| s.parse().ok()),
            thread_name: env::var("TOKIO_THREAD_NAME")
                .unwrap_or_else(|_| "pierre-worker".to_owned()),
            enable_io: true,
            enable_time: true,
        }
    }
}

/// Get environment variable or default value
fn env_var_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_owned())
}
