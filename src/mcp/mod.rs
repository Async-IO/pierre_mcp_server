// ABOUTME: Model Context Protocol (MCP) implementation for AI assistant integration
// ABOUTME: Multi-tenant MCP server functionality for MCP clients and AI assistants
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

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
/// MCP JSON schema definitions
pub mod schema;
/// Server lifecycle management
pub mod server_lifecycle;
/// Tenant isolation and context management
pub mod tenant_isolation;
/// MCP tool handler implementations
pub mod tool_handlers;
/// Transport layer abstraction
pub mod transport_manager;
