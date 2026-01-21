// ABOUTME: Server-Sent Events (SSE) implementation for real-time notifications and MCP protocol streaming
// ABOUTME: Provides unified SSE infrastructure for both OAuth notifications and MCP bidirectional communication
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Server-Sent Events (SSE) Module
//!
//! This module provides SSE infrastructure for real-time streaming communication.
//! It is conditionally compiled based on the `transport-sse` feature flag.
//!
//! ## Components
//!
//! - `a2a_task_stream`: A2A task progress streaming
//! - `manager`: Central SSE connection management
//! - `notifications`: OAuth notification streaming
//! - `protocol`: MCP protocol streaming
//! - `routes`: HTTP route handlers for SSE endpoints

/// A2A task streaming for progress updates
#[cfg(feature = "protocol-a2a")]
pub mod a2a_task_stream;

/// Central SSE manager for connection lifecycle and message routing
pub mod manager;

/// OAuth notification streaming for user-specific events
#[cfg(feature = "oauth")]
pub mod notifications;

/// MCP protocol streaming for bidirectional client-server communication
#[cfg(feature = "protocol-mcp")]
pub mod protocol;

/// HTTP route handlers for SSE endpoints
pub mod routes;

// Re-exports
#[cfg(feature = "protocol-a2a")]
pub use a2a_task_stream::A2ATaskStream;

pub use manager::{ConnectionMetadata, ConnectionType, SseManager};

#[cfg(feature = "oauth")]
pub use notifications::NotificationStream;

#[cfg(feature = "protocol-mcp")]
pub use protocol::McpProtocolStream;

pub use routes::SseRoutes;
