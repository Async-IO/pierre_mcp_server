// ABOUTME: Server-Sent Events (SSE) implementation for real-time notifications and MCP protocol streaming
// ABOUTME: Provides unified SSE infrastructure for both OAuth notifications and MCP bidirectional communication
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

/// A2A task streaming for progress updates
pub mod a2a_task_stream;
/// Central SSE manager for connection lifecycle and message routing
pub mod manager;
/// OAuth notification streaming for user-specific events
pub mod notifications;
/// MCP protocol streaming for bidirectional client-server communication
pub mod protocol;
/// HTTP route handlers for SSE endpoints
pub mod routes;

pub use a2a_task_stream::A2ATaskStream;
pub use manager::{ConnectionMetadata, ConnectionType, SseManager};
pub use notifications::NotificationStream;
pub use protocol::McpProtocolStream;
pub use routes::SseRoutes;
