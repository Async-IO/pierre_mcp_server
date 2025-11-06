// ABOUTME: Unified Server-Sent Events management for both OAuth notifications and MCP protocol streaming
// ABOUTME: Provides clean separation between user-based notifications and session-based MCP communication
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

/// SSE connection manager
pub mod manager;
/// OAuth notification stream handling
pub mod notifications;
/// MCP protocol stream handling
pub mod protocol;
/// SSE HTTP routes
pub mod routes;

pub use manager::SseManager;
pub use notifications::NotificationStream;
pub use protocol::McpProtocolStream;
