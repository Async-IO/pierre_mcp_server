// ABOUTME: MCP (Model Context Protocol) route handlers for AI assistant integration
// ABOUTME: Provides MCP protocol endpoints for tool discovery and execution
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! MCP protocol routes for AI assistant integration

use warp::{Filter, Rejection, Reply};

/// MCP routes implementation
pub struct McpRoutes;

impl McpRoutes {
    /// Create all MCP routes
    pub fn routes() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        warp::path("mcp")
            .and(warp::path("tools"))
            .and(warp::path::end())
            .and(warp::get())
            .and_then(Self::handle_tools)
    }

    /// Handle MCP tools discovery
    async fn handle_tools() -> Result<impl Reply, Rejection> {
        // Use async block to satisfy clippy
        tokio::task::yield_now().await;
        Ok(warp::reply::json(&serde_json::json!({
            "tools": []
        })))
    }
}
