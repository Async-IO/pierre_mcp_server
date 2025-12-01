// ABOUTME: Health check route handlers for service monitoring and status endpoints
// ABOUTME: Provides system health, readiness, and liveness endpoints for monitoring infrastructure
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Health check routes for service monitoring
//!
//! This module provides health, readiness, and liveness endpoints
//! for monitoring and load balancer health checks.

/// Health routes implementation
pub struct HealthRoutes;

impl HealthRoutes {
    /// Create all health check routes
    pub fn routes() -> axum::Router {
        use axum::{routing::get, Json, Router};

        async fn health_handler() -> Json<serde_json::Value> {
            Json(serde_json::json!({
                "status": "healthy",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }

        async fn ready_handler() -> Json<serde_json::Value> {
            Json(serde_json::json!({
                "status": "ready",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }

        Router::new()
            .route("/health", get(health_handler))
            .route("/ready", get(ready_handler))
    }
}
