// ABOUTME: MCP integration test suite module
// ABOUTME: Tests real HTTP server with MCP protocol using synthetic provider
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

pub mod infrastructure;

// Re-export commonly used items
pub use infrastructure::{IntegrationTestServer, McpTestClient};
