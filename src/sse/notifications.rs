// ABOUTME: OAuth notification streaming implementation for user-specific real-time updates
// ABOUTME: Handles SSE streaming of OAuth connection status and completion events
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::constants::network_config::SSE_BROADCAST_CHANNEL_SIZE;
use crate::errors::AppError;
use crate::models::OAuthNotification;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// OAuth notification stream for a specific user
pub struct NotificationStream {
    sender: Arc<RwLock<Option<broadcast::Sender<String>>>>,
    buffer_size: usize,
}

impl NotificationStream {
    /// Creates a new notification stream with the specified buffer size
    #[must_use]
    pub fn new(buffer_size: usize) -> Self {
        Self {
            sender: Arc::new(RwLock::new(None)),
            buffer_size,
        }
    }

    /// Subscribe to notifications for this stream
    pub async fn subscribe(&self) -> broadcast::Receiver<String> {
        let mut sender_guard = self.sender.write().await;

        let sender = if let Some(existing_sender) = sender_guard.take() {
            *sender_guard = Some(existing_sender.clone());
            existing_sender
        } else {
            let (tx, _) = broadcast::channel(self.buffer_size);
            *sender_guard = Some(tx.clone());
            tx
        };
        drop(sender_guard);

        sender.subscribe()
    }

    /// Send OAuth notification through this stream
    ///
    /// # Errors
    ///
    /// Returns an error if no active sender is available for this stream
    pub async fn send_notification(
        &self,
        notification: &OAuthNotification,
    ) -> Result<(), AppError> {
        let sender_guard = self.sender.read().await;

        if let Some(sender) = sender_guard.as_ref() {
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

            sender
                .send(sse_message)
                .map_err(|e| AppError::internal(format!("Failed to send notification: {e}")))?;

            Ok(())
        } else {
            Err(AppError::internal(
                "No active sender for notification stream",
            ))
        }
    }

    /// Check if stream has active subscribers
    pub async fn has_subscribers(&self) -> bool {
        let sender_guard = self.sender.read().await;
        sender_guard
            .as_ref()
            .is_some_and(|sender| sender.receiver_count() > 0)
    }
}

impl Default for NotificationStream {
    fn default() -> Self {
        // Use default buffer size from constants
        Self::new(SSE_BROADCAST_CHANNEL_SIZE)
    }
}
