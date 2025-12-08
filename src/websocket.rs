// ABOUTME: WebSocket implementation for real-time communication and live data streaming
// ABOUTME: Handles WebSocket connections, message routing, and real-time fitness data updates
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - Arc resource clones for multi-tenant concurrent access
// - String ownership transfers for WebSocket message construction

//! `WebSocket` support for real-time updates
//!
//! Provides real-time updates for API key usage, rate limit status,
//! and system metrics via `WebSocket` connections.

use crate::auth::{AuthManager, AuthResult};
use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::errors::{AppError, AppResult};
use crate::middleware::McpAuthMiddleware;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{interval, Duration};
use tracing::{debug, trace, warn};
use uuid::Uuid;

// WebSocket message type alias for Axum
type Message = axum::extract::ws::Message;

/// WebSocket message types for real-time communication
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    /// Client authentication message
    #[serde(rename = "auth")]
    Authentication {
        /// JWT authentication token
        token: String,
    },
    /// Subscribe to specific topics
    #[serde(rename = "subscribe")]
    Subscribe {
        /// List of topics to subscribe to
        topics: Vec<String>,
    },
    /// API key usage update notification
    #[serde(rename = "usage_update")]
    UsageUpdate {
        /// API key identifier
        api_key_id: String,
        /// Number of requests made today
        requests_today: u64,
        /// Number of requests made this month
        requests_this_month: u64,
        /// Current rate limit status
        rate_limit_status: Value,
    },
    /// System-wide statistics update
    #[serde(rename = "system_stats")]
    SystemStats {
        /// Total requests across all keys today
        total_requests_today: u64,
        /// Total requests across all keys this month
        total_requests_this_month: u64,
        /// Number of active WebSocket connections
        active_connections: usize,
    },
    /// Error message to client
    #[serde(rename = "error")]
    Error {
        /// Error description
        message: String,
    },
    /// Success confirmation message
    #[serde(rename = "success")]
    Success {
        /// Success message
        message: String,
    },
}

/// Manages WebSocket connections and message broadcasting
#[derive(Clone)]
pub struct WebSocketManager {
    database: Arc<Database>,
    auth_middleware: McpAuthMiddleware,
    clients: Arc<RwLock<HashMap<Uuid, ClientConnection>>>,
    broadcast_tx: broadcast::Sender<WebSocketMessage>,
}

#[derive(Debug)]
struct ClientConnection {
    user_id: Uuid,
    subscriptions: Vec<String>,
    tx: tokio::sync::mpsc::UnboundedSender<Message>,
}

impl WebSocketManager {
    /// Creates a new WebSocket manager instance
    #[must_use]
    pub fn new(
        database: Arc<Database>,
        auth_manager: &Arc<AuthManager>,
        jwks_manager: &Arc<crate::admin::jwks::JwksManager>,
        rate_limit_config: crate::config::environment::RateLimitConfig,
    ) -> Self {
        let (broadcast_tx, _) =
            broadcast::channel(crate::constants::rate_limits::WEBSOCKET_CHANNEL_CAPACITY);
        let auth_middleware = McpAuthMiddleware::new(
            (**auth_manager).clone(),
            database.clone(),
            jwks_manager.clone(),
            rate_limit_config,
        ); // Safe: Arc clones for middleware creation

        Self {
            database,
            auth_middleware,
            clients: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
        }
    }

    /// Handle authentication message and return authenticated user ID
    async fn handle_auth_message(
        &self,
        token: &str,
        tx: &tokio::sync::mpsc::UnboundedSender<Message>,
    ) -> Option<Uuid> {
        match self.authenticate_user(token).await {
            Ok(auth_result) => {
                let success_msg = WebSocketMessage::Success {
                    message: "Authentication successful".into(),
                };
                if let Ok(json) = serde_json::to_string(&success_msg) {
                    if let Err(e) = tx.send(Message::Text(json)) {
                        warn!(
                            user_id = %auth_result.user_id,
                            error = ?e,
                            "Failed to send authentication success message over WebSocket"
                        );
                    }
                }
                Some(auth_result.user_id)
            }
            Err(e) => {
                let error_msg = WebSocketMessage::Error {
                    message: format!("Authentication failed: {e}"),
                };
                if let Ok(json) = serde_json::to_string(&error_msg) {
                    if let Err(send_err) = tx.send(Message::Text(json)) {
                        warn!(
                            auth_error = %e,
                            send_error = ?send_err,
                            "Failed to send authentication error message over WebSocket"
                        );
                    }
                }
                None
            }
        }
    }

    /// Handle subscribe message and update subscriptions
    fn handle_subscribe_message(
        topics: Vec<String>,
        authenticated_user: Option<Uuid>,
        tx: &tokio::sync::mpsc::UnboundedSender<Message>,
    ) -> Vec<String> {
        if authenticated_user.is_some() {
            let success_msg = WebSocketMessage::Success {
                message: format!("Subscribed to {} topics", topics.len()),
            };
            if let Ok(json) = serde_json::to_string(&success_msg) {
                if let Err(e) = tx.send(Message::Text(json)) {
                    warn!(
                        user_id = ?authenticated_user,
                        topic_count = topics.len(),
                        error = ?e,
                        "Failed to send subscription confirmation over WebSocket"
                    );
                }
            }
            topics
        } else {
            let error_msg = WebSocketMessage::Error {
                message: "Authentication required".into(),
            };
            if let Ok(json) = serde_json::to_string(&error_msg) {
                if let Err(e) = tx.send(Message::Text(json)) {
                    warn!(
                        error = ?e,
                        "Failed to send authentication required error message over WebSocket"
                    );
                }
            }
            Vec::new()
        }
    }

    /// Handle incoming WebSocket connection
    pub async fn handle_connection(&self, ws: axum::extract::ws::WebSocket) {
        let (mut ws_tx, mut ws_rx) = ws.split();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let connection_id = Uuid::new_v4();
        let mut authenticated_user: Option<Uuid> = None;
        let mut subscriptions: Vec<String> = Vec::new();

        // Spawn task to forward messages to `WebSocket`
        let ws_send_task = tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if ws_tx.send(message).await.is_err() {
                    break;
                }
            }
        });

        // Handle incoming messages
        while let Some(msg) = ws_rx.next().await {
            match msg {
                Ok(Message::Text(text)) => match serde_json::from_str::<WebSocketMessage>(&text) {
                    Ok(WebSocketMessage::Authentication { token }) => {
                        authenticated_user = self.handle_auth_message(&token, &tx).await;
                    }
                    Ok(WebSocketMessage::Subscribe { topics }) => {
                        subscriptions =
                            Self::handle_subscribe_message(topics, authenticated_user, &tx);
                    }
                    Err(e) => {
                        let error_msg = WebSocketMessage::Error {
                            message: format!("Invalid message format: {e}"),
                        };
                        if let Ok(json) = serde_json::to_string(&error_msg) {
                            if let Err(send_err) = tx.send(Message::Text(json)) {
                                warn!(
                                    parse_error = %e,
                                    send_error = ?send_err,
                                    "Failed to send invalid message format error over WebSocket"
                                );
                            }
                        }
                    }
                    _ => {}
                },
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }

        // Store authenticated connection
        if let Some(user_id) = authenticated_user {
            let client = ClientConnection {
                user_id,
                subscriptions,
                tx: tx.clone(), // Safe: mpsc::Sender clone for client storage
            };
            self.clients.write().await.insert(connection_id, client);
        }

        // Clean up on disconnect
        ws_send_task.abort();
        self.clients.write().await.remove(&connection_id);
    }

    /// Authenticate `WebSocket` user with JWT
    async fn authenticate_user(&self, token: &str) -> AppResult<AuthResult> {
        let auth_header = if token.starts_with("Bearer ") {
            token.to_owned()
        } else {
            format!("Bearer {token}")
        };

        self.auth_middleware
            .authenticate_request(Some(&auth_header))
            .await
            .map_err(|e| AppError::internal(format!("WebSocket authentication failed: {e}")))
    }

    /// Broadcast usage update to subscribed clients
    pub async fn broadcast_usage_update(
        &self,
        api_key_id: &str,
        user_id: &Uuid,
        requests_today: u64,
        requests_this_month: u64,
        rate_limit_status: Value,
    ) {
        let message = WebSocketMessage::UsageUpdate {
            api_key_id: api_key_id.to_owned(),
            requests_today,
            requests_this_month,
            rate_limit_status,
        };

        self.send_to_user_subscribers(user_id, &message, "usage")
            .await;
    }

    /// Broadcast system statistics
    ///
    /// # Errors
    ///
    /// Returns an error if:\n    /// - System statistics retrieval fails\n    /// - Message serialization fails\n    /// - Broadcasting to clients fails
    pub async fn broadcast_system_stats(&self) -> AppResult<()> {
        let stats = self
            .get_system_stats()
            .await
            .map_err(|e| AppError::internal(format!("Failed to get system stats: {e}")))?;
        let message = WebSocketMessage::SystemStats {
            total_requests_today: stats.total_requests_today,
            total_requests_this_month: stats.total_requests_this_month,
            active_connections: self.clients.read().await.len(),
        };

        self.broadcast_to_all(&message, "system").await;
        Ok(())
    }

    /// Send message to specific user's subscribers
    async fn send_to_user_subscribers(
        &self,
        user_id: &Uuid,
        message: &WebSocketMessage,
        topic: &str,
    ) {
        let clients = self.clients.read().await;
        for (_, client) in clients.iter() {
            if client.user_id == *user_id && client.subscriptions.contains(&topic.to_owned()) {
                if let Ok(msg_text) = serde_json::to_string(message) {
                    if let Err(e) = client.tx.send(Message::Text(msg_text)) {
                        warn!(
                            user_id = %user_id,
                            topic = %topic,
                            error = ?e,
                            "Failed to send message to user subscriber over WebSocket"
                        );
                    }
                }
            }
        }
    }

    /// Broadcast message to all subscribers of a topic
    async fn broadcast_to_all(&self, message: &WebSocketMessage, topic: &str) {
        // Use broadcast channel for efficient message distribution
        if let Err(e) = self.broadcast_tx.send(message.clone()) {
            // Safe: broadcast channel needs ownership while we reuse message below
            trace!("Failed to send broadcast message: {}", e);
        }

        // Also send directly to subscribed clients for immediate delivery
        let clients = self.clients.read().await;
        for (_, client) in clients.iter() {
            if client.subscriptions.contains(&topic.to_owned()) {
                if let Ok(msg_text) = serde_json::to_string(message) {
                    if let Err(e) = client.tx.send(Message::Text(msg_text)) {
                        warn!(
                            topic = %topic,
                            error = ?e,
                            "Failed to broadcast message to client over WebSocket"
                        );
                    }
                }
            }
        }
    }

    /// Get current system statistics
    async fn get_system_stats(&self) -> AppResult<SystemStats> {
        // Query the database for real statistics
        let (today_count, month_count) = self
            .database
            .get_system_stats()
            .await
            .map_err(|e| AppError::database(e.to_string()))?;

        debug!(
            "System statistics: {} requests today, {} this month",
            today_count, month_count
        );

        Ok(SystemStats {
            total_requests_today: today_count,
            total_requests_this_month: month_count,
        })
    }

    /// Start background task for periodic updates
    pub fn start_periodic_updates(&self) {
        let manager = self.clone(); // Safe: Arc clone for background task
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30)); // Update every 30 seconds

            loop {
                interval.tick().await;

                // Broadcast system stats
                if let Err(e) = manager.broadcast_system_stats().await {
                    warn!("Failed to broadcast system stats: {}", e);
                }
            }
        });
    }
}

#[derive(Debug)]
struct SystemStats {
    total_requests_today: u64,
    total_requests_this_month: u64,
}
