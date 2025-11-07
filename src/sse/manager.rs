// ABOUTME: Central SSE manager that coordinates both OAuth notifications and MCP protocol streams
// ABOUTME: Provides unified connection management with clean separation of stream types
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::{notifications::NotificationStream, protocol::McpProtocolStream};
use crate::{
    database::oauth_notifications::OAuthNotification,
    errors::AppError,
    mcp::{protocol::McpRequest, resources::ServerResources},
};
use anyhow::Result;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

/// Connection types for different SSE streams
#[derive(Debug, Clone)]
pub enum ConnectionType {
    /// OAuth notification stream for a specific user
    Notification {
        /// User ID for the notification stream
        user_id: Uuid,
    },
    /// MCP protocol stream for a client session
    Protocol {
        /// Session ID for the protocol stream
        session_id: String,
    },
}

/// SSE connection metadata
#[derive(Debug, Clone)]
pub struct ConnectionMetadata {
    /// Type of SSE connection (notification or protocol)
    pub connection_type: ConnectionType,
    /// When the connection was established
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Timestamp of last activity on this connection
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

/// Unified SSE manager handling both notification and protocol streams
#[derive(Clone)]
pub struct SseManager {
    notification_streams: Arc<RwLock<HashMap<Uuid, NotificationStream>>>,
    protocol_streams: Arc<RwLock<HashMap<String, McpProtocolStream>>>,
    connection_metadata: Arc<RwLock<HashMap<String, ConnectionMetadata>>>,
    /// Maps `user_id` to their active `session_ids` for protocol streams
    user_sessions: Arc<RwLock<HashMap<Uuid, Vec<String>>>>,
    /// Buffer size for SSE channels
    buffer_size: usize,
}

impl SseManager {
    /// Creates a new SSE manager with the specified buffer size
    #[must_use]
    pub fn new(buffer_size: usize) -> Self {
        Self {
            notification_streams: Arc::new(RwLock::new(HashMap::new())),
            protocol_streams: Arc::new(RwLock::new(HashMap::new())),
            connection_metadata: Arc::new(RwLock::new(HashMap::new())),
            user_sessions: Arc::new(RwLock::new(HashMap::new())),
            buffer_size,
        }
    }
}

impl Default for SseManager {
    fn default() -> Self {
        // Use default buffer size from constants
        Self::new(crate::constants::network_config::SSE_BROADCAST_CHANNEL_SIZE)
    }
}

impl SseManager {
    /// Register a new OAuth notification stream for a user
    pub async fn register_notification_stream(&self, user_id: Uuid) -> broadcast::Receiver<String> {
        let stream = NotificationStream::new(self.buffer_size);
        let receiver = stream.subscribe().await;

        {
            let mut streams = self.notification_streams.write().await;
            streams.insert(user_id, stream);
        }

        let connection_id = format!("notification_{user_id}");
        let metadata = ConnectionMetadata {
            connection_type: ConnectionType::Notification { user_id },
            created_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
        };

        {
            let mut metadata_map = self.connection_metadata.write().await;
            metadata_map.insert(connection_id.clone(), metadata);
        }

        tracing::info!("Registered notification stream for user: {}", user_id);
        receiver
    }

    /// Register a new MCP protocol stream for a session
    pub async fn register_protocol_stream(
        &self,
        session_id: String,
        authorization: Option<String>,
        resources: Arc<ServerResources>,
    ) -> broadcast::Receiver<String> {
        let stream = McpProtocolStream::new(resources.clone());
        let receiver = stream.subscribe().await;

        {
            let mut streams = self.protocol_streams.write().await;
            streams.insert(session_id.clone(), stream);
        }

        // Extract user_id from JWT token if provided
        let user_id = if let Some(auth) = authorization {
            if let Some(token) = auth.strip_prefix("Bearer ") {
                if let Ok(jwt_result) = crate::mcp::tenant_isolation::validate_jwt_token_for_mcp(
                    token,
                    &resources.auth_manager,
                    &resources.jwks_manager,
                    &resources.database,
                )
                .await
                {
                    Some(jwt_result.user_id)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Track session for this user
        if let Some(user_id) = user_id {
            self.user_sessions
                .write()
                .await
                .entry(user_id)
                .or_default()
                .push(session_id.clone());
            tracing::info!(
                "Registered protocol stream for session {} belonging to user {}",
                session_id,
                user_id
            );
        } else {
            tracing::info!("Registered protocol stream for session: {}", session_id);
        }

        let connection_id = format!("protocol_{session_id}");
        let metadata = ConnectionMetadata {
            connection_type: ConnectionType::Protocol {
                session_id: session_id.clone(),
            },
            created_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
        };

        {
            let mut metadata_map = self.connection_metadata.write().await;
            metadata_map.insert(connection_id, metadata);
        }

        receiver
    }

    /// Send OAuth notification to a specific user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No notification stream is found for the specified user
    /// - The underlying stream fails to send the notification
    pub async fn send_notification(
        &self,
        user_id: Uuid,
        notification: &OAuthNotification,
    ) -> Result<()> {
        let streams = self.notification_streams.read().await;

        if let Some(stream) = streams.get(&user_id) {
            stream.send_notification(notification).await?;

            // Update last activity
            let connection_id = format!("notification_{user_id}");
            {
                let mut metadata_map = self.connection_metadata.write().await;
                if let Some(metadata) = metadata_map.get_mut(&connection_id) {
                    metadata.last_activity = chrono::Utc::now();
                }
            }

            Ok(())
        } else {
            Err(AppError::not_found(format!("Notification stream for user {user_id}")).into())
        }
    }

    /// Send OAuth notification to all MCP protocol streams for a user
    ///
    /// # Errors
    ///
    /// Returns an error if sending to any stream fails
    pub async fn send_oauth_notification_to_protocol_streams(
        &self,
        user_id: Uuid,
        notification: &OAuthNotification,
    ) -> Result<()> {
        let user_sessions = self.user_sessions.read().await;
        let session_ids = user_sessions.get(&user_id).cloned();
        drop(user_sessions);

        if let Some(sessions) = session_ids {
            let streams = self.protocol_streams.read().await;
            let mut sent_count = 0;

            for session_id in &sessions {
                if let Some(stream) = streams.get(session_id) {
                    if let Err(e) = stream.send_oauth_notification(notification).await {
                        tracing::warn!(
                            "Failed to send OAuth notification to session {}: {}",
                            session_id,
                            e
                        );
                    } else {
                        sent_count += 1;
                    }
                }
            }

            if sent_count > 0 {
                tracing::info!(
                    "Sent OAuth notification to {} protocol stream(s) for user {}",
                    sent_count,
                    user_id
                );
                Ok(())
            } else {
                Err(
                    AppError::not_found(format!("Active protocol streams for user {user_id}"))
                        .into(),
                )
            }
        } else {
            Err(AppError::not_found(format!("Protocol streams for user {user_id}")).into())
        }
    }

    /// Send MCP request to a protocol stream
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No protocol stream is found for the specified session ID
    /// - The underlying stream fails to handle the request
    pub async fn send_mcp_request(&self, session_id: &str, request: McpRequest) -> Result<()> {
        let streams = self.protocol_streams.read().await;

        if let Some(stream) = streams.get(session_id) {
            stream.handle_request(request).await?;

            // Update last activity
            let connection_id = format!("protocol_{session_id}");
            {
                let mut metadata_map = self.connection_metadata.write().await;
                if let Some(metadata) = metadata_map.get_mut(&connection_id) {
                    metadata.last_activity = chrono::Utc::now();
                }
            }

            Ok(())
        } else {
            Err(AppError::not_found(format!("Protocol stream for session {session_id}")).into())
        }
    }

    /// Unregister a notification stream
    pub async fn unregister_notification_stream(&self, user_id: Uuid) {
        {
            let mut streams = self.notification_streams.write().await;
            streams.remove(&user_id);
        }

        let connection_id = format!("notification_{user_id}");
        {
            let mut metadata_map = self.connection_metadata.write().await;
            metadata_map.remove(&connection_id);
        }

        tracing::info!("Unregistered notification stream for user: {}", user_id);
    }

    /// Unregister a protocol stream
    pub async fn unregister_protocol_stream(&self, session_id: &str) {
        {
            let mut streams = self.protocol_streams.write().await;
            streams.remove(session_id);
        }

        let connection_id = format!("protocol_{session_id}");
        {
            let mut metadata_map = self.connection_metadata.write().await;
            metadata_map.remove(&connection_id);
        }

        // Clean up session from user_sessions to prevent memory leak
        {
            let mut user_sessions = self.user_sessions.write().await;
            user_sessions.retain(|_user_id, sessions| {
                sessions.retain(|s| s != session_id);
                // Keep user entry only if they still have active sessions
                !sessions.is_empty()
            });
        }

        tracing::info!("Unregistered protocol stream for session: {}", session_id);
    }

    /// Get count of active notification streams
    pub async fn active_notification_streams(&self) -> usize {
        let streams = self.notification_streams.read().await;
        streams.len()
    }

    /// Get count of active protocol streams
    pub async fn active_protocol_streams(&self) -> usize {
        let streams = self.protocol_streams.read().await;
        streams.len()
    }

    /// Get all connection metadata for monitoring
    pub async fn get_connection_metadata(&self) -> HashMap<String, ConnectionMetadata> {
        let metadata_map = self.connection_metadata.read().await;
        metadata_map.clone()
    }

    /// Clean up inactive connections based on timeout
    pub async fn cleanup_inactive_connections(&self, timeout_seconds: u64) {
        let timeout_seconds = i64::try_from(timeout_seconds).unwrap_or(i64::MAX);
        let cutoff = chrono::Utc::now() - chrono::Duration::seconds(timeout_seconds);
        let mut to_remove = Vec::new();

        {
            let metadata_map = self.connection_metadata.read().await;
            for (connection_id, metadata) in metadata_map.iter() {
                if metadata.last_activity < cutoff {
                    to_remove.push((connection_id.clone(), metadata.connection_type.clone()));
                }
            }
        }

        for (connection_id, connection_type) in to_remove {
            match connection_type {
                ConnectionType::Notification { user_id } => {
                    self.unregister_notification_stream(user_id).await;
                }
                ConnectionType::Protocol { session_id } => {
                    self.unregister_protocol_stream(&session_id).await;
                }
            }
            tracing::info!("Cleaned up inactive connection: {}", connection_id);
        }
    }
}
