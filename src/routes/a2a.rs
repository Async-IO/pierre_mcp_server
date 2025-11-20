// ABOUTME: A2A (Agent-to-Agent) protocol route handlers for inter-agent communication
// ABOUTME: Provides endpoints for agent registration, messaging, and protocol management
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! A2A protocol routes for agent-to-agent communication

use crate::a2a::agent_card::AgentCard;
use axum::Json;

/// A2A routes implementation
pub struct A2ARoutes;

impl A2ARoutes {
    /// Create all A2A routes
    pub fn routes() -> axum::Router {
        use axum::{routing::get, Router};

        Router::new()
            .route("/a2a/status", get(Self::handle_status))
            .route(
                "/.well-known/agent-card.json",
                get(Self::handle_agent_card_discovery),
            )
    }

    /// Handle A2A status
    async fn handle_status() -> Json<serde_json::Value> {
        std::future::ready(Json(serde_json::json!({
            "status": "active"
        })))
        .await
    }

    /// Handle agent card discovery endpoint (RFC 8615 well-known URI)
    ///
    /// Returns the A2A agent card for service discovery, containing:
    /// - Agent capabilities and description
    /// - Supported transport protocols
    /// - Authentication methods
    /// - Tool definitions
    ///
    /// This endpoint is publicly accessible without authentication,
    /// as required for agent discovery per A2A specification.
    async fn handle_agent_card_discovery() -> Json<AgentCard> {
        std::future::ready(Json(AgentCard::new())).await
    }
}
