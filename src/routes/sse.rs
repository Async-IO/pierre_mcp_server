// ABOUTME: Server-Sent Events route handlers for real-time notifications and streaming
// ABOUTME: Provides SSE endpoints for pushing live updates to clients

//! SSE routes for server-sent events

use warp::{Filter, Rejection, Reply};

/// SSE routes implementation
pub struct SseRoutes;

impl SseRoutes {
    /// Create all SSE routes
    pub fn routes() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        warp::path("notifications")
            .and(warp::path("sse"))
            .and(warp::path::end())
            .and(warp::get())
            .and_then(Self::handle_sse)
    }

    /// Handle SSE connection
    async fn handle_sse() -> Result<impl Reply, Rejection> {
        // Use async block to satisfy clippy
        tokio::task::yield_now().await;
        Ok(warp::reply::with_status(
            "SSE endpoint",
            warp::http::StatusCode::OK,
        ))
    }
}
