// ABOUTME: Infrastructure module for MCP integration tests
// ABOUTME: Provides test server lifecycle management and MCP HTTP client
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

pub mod mcp_client;
pub mod test_server;

pub use mcp_client::McpTestClient;
pub use test_server::IntegrationTestServer;
