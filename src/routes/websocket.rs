// ABOUTME: WebSocket route handlers for real-time bidirectional communication
// ABOUTME: Provides WebSocket endpoints for live notifications and streaming data
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::websocket::WebSocketManager;
use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use std::sync::Arc;
use tokio::task;
use tracing::{debug, info};

/// WebSocket routes implementation
pub struct WebSocketRoutes;

impl WebSocketRoutes {
    /// Create all WebSocket routes with injected `WebSocketManager`
    ///
    /// # Arguments
    /// * `manager` - Shared `WebSocketManager` for handling connections
    ///
    /// # Returns
    /// Configured Axum Router with WebSocket endpoint
    pub fn routes(manager: Arc<WebSocketManager>) -> Router {
        Router::new()
            .route("/ws", get(Self::handle_websocket))
            .with_state(manager)
    }

    /// Handle WebSocket upgrade and connection
    ///
    /// Upgrades HTTP connection to WebSocket protocol and delegates
    /// to `WebSocketManager` for authentication, subscriptions, and broadcasting.
    ///
    /// # Arguments
    /// * `ws` - `WebSocketUpgrade` extractor from Axum
    /// * `manager` - `WebSocketManager` state injected via Router
    ///
    /// # Returns
    /// Response that upgrades the connection to WebSocket
    async fn handle_websocket(
        ws: WebSocketUpgrade,
        State(manager): State<Arc<WebSocketManager>>,
    ) -> impl IntoResponse {
        info!("New WebSocket connection request");

        // Yield to scheduler to allow other tasks to progress during upgrade
        task::yield_now().await;

        // Upgrade HTTP connection to WebSocket
        ws.on_upgrade(move |socket: WebSocket| async move {
            debug!("WebSocket upgraded, delegating to manager");
            manager.handle_connection(socket).await;
        })
    }
}
