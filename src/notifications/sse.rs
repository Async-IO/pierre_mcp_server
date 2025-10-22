// ABOUTME: Server-Sent Events implementation for real-time OAuth notifications
// ABOUTME: Handles SSE connections, message broadcasting, and connection management
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - Arc clone for SSE manager sharing across HTTP handlers
// - Stream state management for concurrent client connections

use crate::database::oauth_notifications::OAuthNotification;
use crate::errors::AppError;
use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use warp::{http::StatusCode, Filter, Rejection, Reply};

/// SSE connection manager for tracking active client connections
#[derive(Clone)]
pub struct SseConnectionManager {
    connections: Arc<RwLock<HashMap<String, broadcast::Sender<String>>>>,
}

impl Default for SseConnectionManager {
    fn default() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl SseConnectionManager {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new SSE connection for a user
    pub async fn register_connection(&self, user_id: String) -> broadcast::Receiver<String> {
        let (tx, rx) =
            broadcast::channel(crate::constants::network_config::SSE_BROADCAST_CHANNEL_SIZE);

        {
            let mut connections = self.connections.write().await;
            connections.insert(user_id.clone(), tx); // Safe: String ownership for HashMap key
        }

        tracing::info!("SSE connection registered for user: {}", user_id);
        rx
    }

    /// Remove an SSE connection when client disconnects
    pub async fn unregister_connection(&self, user_id: &str) {
        self.connections.write().await.remove(user_id);
        tracing::info!("SSE connection unregistered for user: {}", user_id);
    }

    /// Send notification to a specific user via SSE
    ///
    /// # Errors
    /// Returns an error if no SSE connection exists for the user or if sending fails
    pub async fn send_notification(
        &self,
        user_id: &str,
        notification: &OAuthNotification,
    ) -> Result<()> {
        let connections = self.connections.read().await;

        if let Some(sender) = connections.get(user_id) {
            let sse_message = format!(
                "data: {}\n\n",
                json!({
                    "type": "oauth_notification",
                    "id": notification.id,
                    "provider": notification.provider,
                    "message": notification.message,
                    "success": notification.success,
                    "created_at": notification.created_at
                })
            );

            if let Err(e) = sender.send(sse_message) {
                tracing::warn!("Failed to send SSE notification to user {}: {}", user_id, e);
                return Err(AppError::internal("Failed to send SSE notification").into());
            }

            tracing::info!("SSE notification sent successfully to user: {}", user_id);
            Ok(())
        } else {
            tracing::warn!("No SSE connection found for user: {}", user_id);
            Err(AppError::not_found("No SSE connection found for user").into())
        }
    }

    /// Get count of active connections (for monitoring)
    pub async fn active_connections(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }
}

/// Handle SSE connection endpoint
///
/// # Errors
/// Returns a rejection if the SSE stream cannot be created
pub async fn handle_sse_connection(
    user_id: String,
    manager: Arc<SseConnectionManager>,
) -> Result<impl Reply, Rejection> {
    tracing::info!("New SSE connection request for user: {}", user_id);

    let mut receiver = manager.register_connection(user_id.clone()).await; // Safe: String ownership for async call
    let manager_clone = manager.clone(); // Safe: Arc clone for async stream
    let user_id_clone = user_id.clone(); // Safe: String ownership for async stream

    let stream = async_stream::stream! {
        // Send initial connection established event
        yield Ok::<_, warp::Error>(warp::sse::Event::default()
            .data("connected")
            .event("connection"));

        // Listen for notifications
        loop {
            match receiver.recv().await {
                Ok(message) => {
                    yield Ok(warp::sse::Event::default()
                        .data(message)
                        .event("notification"));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    tracing::warn!("SSE receiver lagged for user: {}", user_id);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    tracing::info!("SSE channel closed for user: {}", user_id);
                    break;
                }
            }
        }

        // Clean up connection
        manager_clone.unregister_connection(&user_id_clone).await;
    };

    // Configure keepalive with 15-second interval
    let keep = warp::sse::keep_alive()
        .interval(std::time::Duration::from_secs(15))
        .text(": keepalive\n\n");

    Ok(warp::sse::reply(keep.stream(stream)))
}

/// SSE route filter for user authentication
#[must_use]
pub fn sse_routes(
    manager: Arc<SseConnectionManager>,
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("notifications")
        .and(warp::path("sse"))
        .and(warp::path::end())
        .and(warp::get())
        .and(warp::query::<HashMap<String, String>>())
        .and_then({
            move |params: HashMap<String, String>| {
                let manager = manager.clone(); // Safe: Arc clone for HTTP handler closure
                async move {
                    let user_id = params
                        .get("user_id")
                        .ok_or_else(|| warp::reject::custom(InvalidUserIdError))?
                        .clone(); // Safe: String ownership from HashMap

                    // Accept user_id from query parameter for SSE connection

                    handle_sse_connection(user_id, manager).await
                }
            }
        })
}

/// Custom error for invalid user ID
#[derive(Debug)]
struct InvalidUserIdError;

impl warp::reject::Reject for InvalidUserIdError {}

/// Error handling for SSE endpoints
///
/// # Errors
/// This function cannot fail (returns Infallible) but processes warp rejections
pub fn handle_sse_rejection(err: &Rejection) -> Result<impl Reply, std::convert::Infallible> {
    if err.find::<InvalidUserIdError>().is_some() {
        Ok(warp::reply::with_status(
            "Invalid or missing user_id parameter",
            StatusCode::BAD_REQUEST,
        ))
    } else {
        Ok(warp::reply::with_status(
            "Internal server error",
            StatusCode::INTERNAL_SERVER_ERROR,
        ))
    }
}
