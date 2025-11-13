// ABOUTME: Server-Sent Events (SSE) implementation for real-time notifications and MCP protocol streaming
// ABOUTME: Provides unified SSE infrastructure for both OAuth notifications and MCP bidirectional communication
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

/// Central SSE manager for connection lifecycle and message routing
pub mod manager;
/// OAuth notification streaming for user-specific events
pub mod notifications;
/// MCP protocol streaming for bidirectional client-server communication
pub mod protocol;
/// HTTP route handlers for SSE endpoints
pub mod routes;

pub use manager::{ConnectionMetadata, ConnectionType, SseManager};
pub use notifications::NotificationStream;
pub use protocol::McpProtocolStream;
pub use routes::SseRoutes;
