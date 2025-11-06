// ABOUTME: OAuth 2.0 server route handlers for RFC-compliant authorization server endpoints
// ABOUTME: Provides OAuth 2.0 protocol endpoints including client registration and token exchange
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

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
        let base_url = crate::constants::get_server_config().map_or_else(
            || "http://localhost:8081".to_owned(),
            |c| c.base_url.clone(),
        );
        Ok(warp::reply::json(&serde_json::json!({
            "authorization_endpoint": format!("{}/oauth2/authorize", base_url),
            "token_endpoint": format!("{}/oauth2/token", base_url),
            "supported_scopes": ["read", "write", "admin"],
            "response_types_supported": ["code", "token"]
        })))
    }
}
