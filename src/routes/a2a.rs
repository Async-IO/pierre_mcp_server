// ABOUTME: A2A (Agent-to-Agent) protocol route handlers for inter-agent communication
// ABOUTME: Provides endpoints for agent registration, messaging, and protocol management
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! A2A protocol routes for agent-to-agent communication

/// A2A routes implementation
pub struct A2ARoutes;

impl A2ARoutes {
    /// Create all A2A routes
    pub fn routes() -> axum::Router {
        use axum::{routing::get, Router};

        Router::new().route("/a2a/status", get(Self::handle_status))
    }

    /// Handle A2A status
    async fn handle_status() -> axum::Json<serde_json::Value> {
        std::future::ready(axum::Json(serde_json::json!({
            "status": "active"
        })))
        .await
    }
}
