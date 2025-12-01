// ABOUTME: Notification context for dependency injection of WebSocket services
// ABOUTME: Contains WebSocket manager and OAuth notification channels
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::mcp::schema::OAuthCompletedNotification;
use crate::websocket::WebSocketManager;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Notification context containing `WebSocket` dependencies
///
/// This context provides all notification-related dependencies needed for
/// real-time communication and `WebSocket` management.
///
/// # Dependencies
/// - `websocket_manager`: `WebSocket` connection management
/// - `oauth_notification_sender`: Broadcast channel for OAuth completion notifications
#[derive(Clone)]
pub struct NotificationContext {
    websocket_manager: Arc<WebSocketManager>,
    oauth_notification_sender: Option<broadcast::Sender<OAuthCompletedNotification>>,
}

impl NotificationContext {
    /// Create new notification context
    #[must_use]
    pub const fn new(
        websocket_manager: Arc<WebSocketManager>,
        oauth_notification_sender: Option<broadcast::Sender<OAuthCompletedNotification>>,
    ) -> Self {
        Self {
            websocket_manager,
            oauth_notification_sender,
        }
    }

    /// Get `WebSocket` manager for connection management
    #[must_use]
    pub const fn websocket_manager(&self) -> &Arc<WebSocketManager> {
        &self.websocket_manager
    }

    /// Get OAuth notification sender for broadcasting completion events
    #[must_use]
    pub const fn oauth_notification_sender(
        &self,
    ) -> &Option<broadcast::Sender<OAuthCompletedNotification>> {
        &self.oauth_notification_sender
    }
}
