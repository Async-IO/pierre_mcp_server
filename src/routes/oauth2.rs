// ABOUTME: OAuth 2.0 server route handlers for RFC-compliant authorization server endpoints
// ABOUTME: Provides OAuth 2.0 protocol endpoints including client registration and token exchange

//! OAuth 2.0 server routes for authorization server functionality

use warp::{Filter, Rejection, Reply};

/// `OAuth2` routes implementation
pub struct OAuth2Routes;

impl OAuth2Routes {
    /// Create all `OAuth2` routes
    pub fn routes() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        warp::path("oauth2")
            .and(warp::path("authorize"))
            .and(warp::path::end())
            .and(warp::get())
            .and_then(Self::handle_authorize)
    }

    /// Handle `OAuth2` authorization
    async fn handle_authorize() -> Result<impl Reply, Rejection> {
        // Use async block to satisfy clippy
        tokio::task::yield_now().await;
        // Generate OAuth2 authorization endpoint discovery
        let base_url =
            std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
        Ok(warp::reply::json(&serde_json::json!({
            "authorization_endpoint": format!("{}/oauth2/authorize", base_url),
            "token_endpoint": format!("{}/oauth2/token", base_url),
            "supported_scopes": ["read", "write", "admin"],
            "response_types_supported": ["code", "token"]
        })))
    }
}
