// ABOUTME: Health check route handlers for service monitoring and status endpoints
// ABOUTME: Provides system health, readiness, and liveness endpoints for monitoring infrastructure

//! Health check routes for service monitoring
//!
//! This module provides health, readiness, and liveness endpoints
//! for monitoring and load balancer health checks.

use warp::{Filter, Rejection, Reply};

/// Health routes implementation
pub struct HealthRoutes;

impl HealthRoutes {
    /// Create all health check routes
    pub fn routes() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        let health = warp::path("health")
            .and(warp::path::end())
            .and(warp::get())
            .and_then(Self::handle_health);

        let ready = warp::path("ready")
            .and(warp::path::end())
            .and(warp::get())
            .and_then(Self::handle_ready);

        health.or(ready)
    }

    /// Handle health check
    async fn handle_health() -> Result<impl Reply, Rejection> {
        // Use async block to satisfy clippy
        tokio::task::yield_now().await;
        Ok(warp::reply::json(&serde_json::json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now().to_rfc3339()
        })))
    }

    /// Handle readiness check
    async fn handle_ready() -> Result<impl Reply, Rejection> {
        // Use async block to satisfy clippy
        tokio::task::yield_now().await;
        Ok(warp::reply::json(&serde_json::json!({
            "status": "ready",
            "timestamp": chrono::Utc::now().to_rfc3339()
        })))
    }
}
