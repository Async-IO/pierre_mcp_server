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
    ///
    /// Routes match frontend API expectations:
    /// - /a2a/status - Basic A2A protocol status
    /// - /a2a/dashboard/overview - Dashboard overview for A2A clients
    /// - /a2a/dashboard/analytics - Usage analytics for A2A clients
    /// - /.well-known/agent-card.json - Agent card discovery
    pub fn routes() -> axum::Router {
        use axum::{routing::get, Router};

        Router::new()
            .route("/a2a/status", get(Self::handle_status))
            .route(
                "/a2a/dashboard/overview",
                get(Self::handle_dashboard_overview),
            )
            .route(
                "/a2a/dashboard/analytics",
                get(Self::handle_dashboard_analytics),
            )
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

    /// Handle A2A dashboard overview
    ///
    /// Returns basic overview data for A2A clients dashboard
    async fn handle_dashboard_overview() -> Json<serde_json::Value> {
        std::future::ready(Json(serde_json::json!({
            "total_clients": 0,
            "active_clients": 0,
            "total_requests": 0,
            "requests_today": 0,
            "status": "active"
        })))
        .await
    }

    /// Handle A2A dashboard analytics
    ///
    /// Returns analytics data for A2A clients
    async fn handle_dashboard_analytics() -> Json<serde_json::Value> {
        std::future::ready(Json(serde_json::json!({
            "daily_requests": [],
            "top_clients": [],
            "request_types": {},
            "period_days": 30
        })))
        .await
    }
}
