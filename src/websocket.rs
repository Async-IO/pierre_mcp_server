// ABOUTME: WebSocket implementation for real-time communication and live data streaming
// ABOUTME: Handles WebSocket connections, message routing, and real-time fitness data updates
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! WebSocket support for real-time updates
//!
//! Provides real-time updates for API key usage, rate limit status,
//! and system metrics via WebSocket connections.

use crate::auth::{AuthManager, AuthResult, McpAuthMiddleware};
use crate::database_plugins::{factory::Database, DatabaseProvider};
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{interval, Duration};
use uuid::Uuid;
use warp::ws::{Message, WebSocket, Ws};
use warp::Filter;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    #[serde(rename = "auth")]
    Authentication { token: String },
    #[serde(rename = "subscribe")]
    Subscribe { topics: Vec<String> },
    #[serde(rename = "usage_update")]
    UsageUpdate {
        api_key_id: String,
        requests_today: u64,
        requests_this_month: u64,
        rate_limit_status: Value,
    },
    #[serde(rename = "system_stats")]
    SystemStats {
        total_requests_today: u64,
        total_requests_this_month: u64,
        active_connections: usize,
    },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "success")]
    Success { message: String },
}

#[derive(Clone)]
pub struct WebSocketManager {
    database: Database,
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
    pub fn new(database: Database, auth_manager: AuthManager) -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        let database_arc = std::sync::Arc::new(database.clone());
        let auth_middleware = McpAuthMiddleware::new(auth_manager, database_arc);

        Self {
            database,
            auth_middleware,
            clients: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
        }
    }

    /// Get WebSocket filter for warp
    pub fn websocket_filter(
        &self,
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        let manager = self.clone();

        warp::path("ws").and(warp::ws()).map(move |ws: Ws| {
            let manager = manager.clone();
            ws.on_upgrade(move |socket| async move { manager.handle_connection(socket).await })
        })
    }

    /// Handle new WebSocket connection
    async fn handle_connection(&self, ws: WebSocket) {
        let (mut ws_tx, mut ws_rx) = ws.split();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let connection_id = Uuid::new_v4();
        let mut authenticated_user: Option<Uuid> = None;
        let mut subscriptions: Vec<String> = Vec::new();

        // Spawn task to forward messages to WebSocket
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
                Ok(msg) if msg.is_text() => {
                    let text = msg.to_str().unwrap_or("");
                    match serde_json::from_str::<WebSocketMessage>(text) {
                        Ok(WebSocketMessage::Authentication { token }) => {
                            match self.authenticate_user(&token).await {
                                Ok(auth_result) => {
                                    authenticated_user = Some(auth_result.user_id);
                                    let success_msg = WebSocketMessage::Success {
                                        message: "Authentication successful".to_string(),
                                    };
                                    let _ = tx.send(Message::text(
                                        serde_json::to_string(&success_msg).unwrap(),
                                    ));
                                }
                                Err(e) => {
                                    let error_msg = WebSocketMessage::Error {
                                        message: format!("Authentication failed: {}", e),
                                    };
                                    let _ = tx.send(Message::text(
                                        serde_json::to_string(&error_msg).unwrap(),
                                    ));
                                }
                            }
                        }
                        Ok(WebSocketMessage::Subscribe { topics }) => {
                            if authenticated_user.is_some() {
                                subscriptions = topics;
                                let success_msg = WebSocketMessage::Success {
                                    message: format!(
                                        "Subscribed to {} topics",
                                        subscriptions.len()
                                    ),
                                };
                                let _ = tx.send(Message::text(
                                    serde_json::to_string(&success_msg).unwrap(),
                                ));
                            } else {
                                let error_msg = WebSocketMessage::Error {
                                    message: "Authentication required".to_string(),
                                };
                                let _ = tx.send(Message::text(
                                    serde_json::to_string(&error_msg).unwrap(),
                                ));
                            }
                        }
                        Err(e) => {
                            let error_msg = WebSocketMessage::Error {
                                message: format!("Invalid message format: {}", e),
                            };
                            let _ =
                                tx.send(Message::text(serde_json::to_string(&error_msg).unwrap()));
                        }
                        _ => {}
                    }
                }
                Ok(msg) if msg.is_close() => break,
                Err(_) => break,
                _ => {}
            }
        }

        // Store authenticated connection
        if let Some(user_id) = authenticated_user {
            let client = ClientConnection {
                user_id,
                subscriptions,
                tx: tx.clone(),
            };
            self.clients.write().await.insert(connection_id, client);
        }

        // Clean up on disconnect
        ws_send_task.abort();
        self.clients.write().await.remove(&connection_id);
    }

    /// Authenticate WebSocket user with JWT
    async fn authenticate_user(&self, token: &str) -> Result<AuthResult> {
        let auth_header = if token.starts_with("Bearer ") {
            token.to_string()
        } else {
            format!("Bearer {}", token)
        };

        self.auth_middleware
            .authenticate_request(Some(&auth_header))
            .await
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
            api_key_id: api_key_id.to_string(),
            requests_today,
            requests_this_month,
            rate_limit_status,
        };

        self.send_to_user_subscribers(user_id, &message, "usage")
            .await;
    }

    /// Broadcast system statistics
    pub async fn broadcast_system_stats(&self) -> Result<()> {
        let stats = self.get_system_stats().await?;
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
            if client.user_id == *user_id && client.subscriptions.contains(&topic.to_string()) {
                let msg_text = serde_json::to_string(message).unwrap();
                let _ = client.tx.send(Message::text(msg_text));
            }
        }
    }

    /// Broadcast message to all subscribers of a topic
    async fn broadcast_to_all(&self, message: &WebSocketMessage, topic: &str) {
        // Use broadcast channel for efficient message distribution
        if let Err(e) = self.broadcast_tx.send(message.clone()) {
            tracing::warn!("Failed to send broadcast message: {}", e);
        }

        // Also send directly to subscribed clients for immediate delivery
        let clients = self.clients.read().await;
        for (_, client) in clients.iter() {
            if client.subscriptions.contains(&topic.to_string()) {
                let msg_text = serde_json::to_string(message).unwrap();
                let _ = client.tx.send(Message::text(msg_text));
            }
        }
    }

    /// Get current system statistics
    async fn get_system_stats(&self) -> Result<SystemStats> {
        // Query the database for real statistics
        let (today_count, month_count) = self.database.get_system_stats().await?;

        tracing::debug!(
            "System statistics: {} requests today, {} this month",
            today_count,
            month_count
        );

        Ok(SystemStats {
            total_requests_today: today_count,
            total_requests_this_month: month_count,
        })
    }

    /// Start background task for periodic updates
    pub fn start_periodic_updates(&self) {
        let manager = self.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30)); // Update every 30 seconds

            loop {
                interval.tick().await;

                // Broadcast system stats
                if let Err(e) = manager.broadcast_system_stats().await {
                    tracing::warn!("Failed to broadcast system stats: {}", e);
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
