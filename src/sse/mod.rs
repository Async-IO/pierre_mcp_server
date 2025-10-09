// ABOUTME: Unified Server-Sent Events management for both OAuth notifications and MCP protocol streaming
// ABOUTME: Provides clean separation between user-based notifications and session-based MCP communication
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

pub mod manager;
pub mod notifications;
pub mod protocol;
pub mod routes;

pub use manager::SseManager;
pub use notifications::NotificationStream;
pub use protocol::McpProtocolStream;
