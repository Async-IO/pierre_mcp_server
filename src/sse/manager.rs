// ABOUTME: Central SSE manager that coordinates both OAuth notifications and MCP protocol streams
// ABOUTME: Provides unified connection management with clean separation of stream types
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::{
    a2a_task_stream::A2ATaskStream, notifications::NotificationStream, protocol::McpProtocolStream,
};
use crate::{
    database::oauth_notifications::OAuthNotification,
    errors::AppError,
    mcp::{protocol::McpRequest, resources::ServerResources},
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};
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
    /// A2A task stream for tracking task progress
    A2ATask {
        /// Task ID being streamed
        task_id: String,
        /// Client ID that owns the task
        client_id: String,
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

/// Unified SSE manager handling notification, protocol, and A2A task streams
#[derive(Clone)]
pub struct SseManager {
    notification_streams: Arc<RwLock<HashMap<Uuid, NotificationStream>>>,
    protocol_streams: Arc<RwLock<HashMap<String, McpProtocolStream>>>,
    a2a_task_streams: Arc<RwLock<HashMap<String, A2ATaskStream>>>,
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
            a2a_task_streams: Arc::new(RwLock::new(HashMap::new())),
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

        info!("Registered notification stream for user: {}", user_id);
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
            info!(
                "Registered protocol stream for session {} belonging to user {}",
                session_id, user_id
            );
        } else {
            info!("Registered protocol stream for session: {}", session_id);
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
    ) -> Result<(), AppError> {
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
            Err(AppError::not_found(format!(
                "Notification stream for user {user_id}"
            )))
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
    ) -> Result<(), AppError> {
        let user_sessions = self.user_sessions.read().await;
        let session_ids = user_sessions.get(&user_id).cloned();
        drop(user_sessions);

        if let Some(sessions) = session_ids {
            let streams = self.protocol_streams.read().await;
            let mut sent_count = 0;

            for session_id in &sessions {
                if let Some(stream) = streams.get(session_id) {
                    if let Err(e) = stream.send_oauth_notification(notification).await {
                        warn!(
                            "Failed to send OAuth notification to session {}: {}",
                            session_id, e
                        );
                    } else {
                        sent_count += 1;
                    }
                }
            }

            if sent_count > 0 {
                info!(
                    "Sent OAuth notification to {} protocol stream(s) for user {}",
                    sent_count, user_id
                );
                Ok(())
            } else {
                Err(AppError::not_found(format!(
                    "Active protocol streams for user {user_id}"
                )))
            }
        } else {
            Err(AppError::not_found(format!(
                "Protocol streams for user {user_id}"
            )))
        }
    }

    /// Send MCP request to a protocol stream
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No protocol stream is found for the specified session ID
    /// - The underlying stream fails to handle the request
    pub async fn send_mcp_request(
        &self,
        session_id: &str,
        request: McpRequest,
    ) -> Result<(), AppError> {
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
            Err(AppError::not_found(format!(
                "Protocol stream for session {session_id}"
            )))
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

        info!("Unregistered notification stream for user: {}", user_id);
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

        info!("Unregistered protocol stream for session: {}", session_id);
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
                ConnectionType::A2ATask { task_id, .. } => {
                    self.unregister_a2a_task_stream(&task_id).await;
                }
            }
            info!("Cleaned up inactive connection: {}", connection_id);
        }
    }

    /// Register a new A2A task stream for a task
    pub async fn register_a2a_task_stream(
        &self,
        task_id: String,
        client_id: String,
    ) -> broadcast::Receiver<String> {
        let stream = A2ATaskStream::new(self.buffer_size);
        let receiver = stream.subscribe();

        {
            let mut streams = self.a2a_task_streams.write().await;
            streams.insert(task_id.clone(), stream);
        }

        let connection_id = format!("a2a_task_{task_id}");
        let metadata = ConnectionMetadata {
            connection_type: ConnectionType::A2ATask {
                task_id: task_id.clone(),
                client_id,
            },
            created_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
        };

        {
            let mut metadata_map = self.connection_metadata.write().await;
            metadata_map.insert(connection_id.clone(), metadata);
        }

        info!("Registered A2A task stream for task: {}", task_id);
        receiver
    }

    /// Send task status update to A2A task stream
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No task stream is found for the specified `task_id`
    /// - The underlying stream fails to send the update
    pub async fn send_a2a_task_update(
        &self,
        task_id: &str,
        event_data: String,
    ) -> Result<(), AppError> {
        let streams = self.a2a_task_streams.read().await;

        if let Some(stream) = streams.get(task_id) {
            stream
                .send_update(event_data)
                .map_err(|e| AppError::internal(format!("Failed to send task update: {e}")))?;

            // Update last activity
            let connection_id = format!("a2a_task_{task_id}");
            {
                let mut metadata_map = self.connection_metadata.write().await;
                if let Some(metadata) = metadata_map.get_mut(&connection_id) {
                    metadata.last_activity = chrono::Utc::now();
                }
            }

            Ok(())
        } else {
            Err(AppError::not_found(format!(
                "A2A task stream for task {task_id}"
            )))
        }
    }

    /// Unregister an A2A task stream
    pub async fn unregister_a2a_task_stream(&self, task_id: &str) {
        {
            let mut streams = self.a2a_task_streams.write().await;
            streams.remove(task_id);
        }

        let connection_id = format!("a2a_task_{task_id}");
        {
            let mut metadata_map = self.connection_metadata.write().await;
            metadata_map.remove(&connection_id);
        }

        info!("Unregistered A2A task stream for task: {}", task_id);
    }

    /// Get count of active A2A task streams
    pub async fn active_a2a_task_streams(&self) -> usize {
        let streams = self.a2a_task_streams.read().await;
        streams.len()
    }
}
