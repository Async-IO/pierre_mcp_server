// ABOUTME: A2A (Agent-to-Agent) protocol route handlers for inter-agent communication
// ABOUTME: Provides endpoints for agent registration, messaging, and protocol management

//! A2A protocol routes for agent-to-agent communication

use warp::{Filter, Rejection, Reply};

/// A2A routes implementation
pub struct A2ARoutes;

impl A2ARoutes {
    /// Create all A2A routes
    pub fn routes() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        warp::path("a2a")
            .and(warp::path("status"))
            .and(warp::path::end())
            .and(warp::get())
            .and_then(Self::handle_status)
    }

    /// Handle A2A status
    async fn handle_status() -> Result<impl Reply, Rejection> {
        // Use async block to satisfy clippy
        tokio::task::yield_now().await;
        Ok(warp::reply::json(&serde_json::json!({
            "status": "active"
        })))
    }
}
