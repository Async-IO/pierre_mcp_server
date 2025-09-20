// ABOUTME: Model Context Protocol (MCP) implementation for AI assistant integration
// ABOUTME: Multi-tenant MCP server functionality for MCP clients and AI assistants

pub mod http_setup;
pub mod mcp_request_processor;
pub mod multitenant;
pub mod oauth_flow_manager;
pub mod progress;
pub mod protocol;
pub mod resources;
pub mod schema;
pub mod server_lifecycle;
pub mod sse_transport;
pub mod tenant_isolation;
pub mod tool_handlers;
pub mod transport_manager;
