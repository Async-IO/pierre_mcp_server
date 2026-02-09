// ABOUTME: Protocol configuration constants for various communication protocols
// ABOUTME: Defines timeouts, limits, and configuration for HTTP, WebSocket, and other protocols
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Protocol configuration constants

/// WebSocket protocol constants
pub mod websocket {
    /// Default WebSocket timeout in seconds
    pub const DEFAULT_TIMEOUT_SECS: u64 = 300;
    /// WebSocket ping interval in seconds
    pub const PING_INTERVAL_SECS: u64 = 30;
    /// Maximum WebSocket message size
    pub const MAX_MESSAGE_SIZE: usize = 1_048_576; // 1MB
}

/// HTTP protocol constants
pub mod http {
    /// Default HTTP timeout in seconds
    pub const DEFAULT_TIMEOUT_SECS: u64 = 30;
    /// Maximum header size
    pub const MAX_HEADER_SIZE: usize = 8192; // 8KB
    /// Default keep-alive timeout
    pub const KEEP_ALIVE_TIMEOUT_SECS: u64 = 75;
}

/// MCP protocol constants
pub mod mcp {
    /// Default MCP timeout in seconds
    pub const DEFAULT_TIMEOUT_SECS: u64 = 60;
    /// Maximum MCP message size
    pub const MAX_MESSAGE_SIZE: usize = 10_485_760; // 10MB
}
