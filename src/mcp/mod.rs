// ABOUTME: Model Context Protocol (MCP) implementation for AI assistant integration
// ABOUTME: Multi-tenant MCP server functionality for MCP clients and AI assistants
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

/// MCP request processing and routing
pub mod mcp_request_processor;
/// Multi-tenant MCP server implementation
pub mod multitenant;
/// OAuth 2.0 authorization flow management
pub mod oauth_flow_manager;
/// Progress notification handling
pub mod progress;
/// MCP protocol types and message handling
pub mod protocol;
/// Resource management for MCP
pub mod resources;
/// Sampling peer for server-initiated LLM requests
pub mod sampling_peer;
/// MCP JSON schema definitions
pub mod schema;
/// Server lifecycle management
pub mod server_lifecycle;
/// Tenant isolation and context management
pub mod tenant_isolation;
/// MCP tool handler implementations
pub mod tool_handlers;
/// Per-tenant MCP tool selection and filtering
pub mod tool_selection;
/// Transport layer abstraction
pub mod transport_manager;
