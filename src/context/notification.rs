// ABOUTME: Notification context for dependency injection of WebSocket and SSE services
// ABOUTME: Contains WebSocket manager, SSE manager, and OAuth notification channels
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::mcp::schema::OAuthCompletedNotification;
#[cfg(feature = "transport-sse")]
use crate::sse::SseManager;
#[cfg(feature = "transport-websocket")]
use crate::websocket::WebSocketManager;
#[cfg(any(feature = "transport-sse", feature = "transport-websocket"))]
use std::sync::Arc;
use tokio::sync::broadcast;

/// Notification context containing `WebSocket` and SSE dependencies
///
/// This context provides all notification-related dependencies needed for
/// real-time communication, `WebSocket` management, and Server-Sent Events.
///
/// # Dependencies
/// - `websocket_manager`: `WebSocket` connection management (requires transport-websocket)
/// - `sse_manager`: Server-Sent Events for streaming notifications (requires transport-sse)
/// - `oauth_notification_sender`: Broadcast channel for OAuth completion notifications
#[derive(Clone)]
pub struct NotificationContext {
    #[cfg(feature = "transport-websocket")]
    websocket_manager: Arc<WebSocketManager>,
    #[cfg(feature = "transport-sse")]
    sse_manager: Arc<SseManager>,
    oauth_notification_sender: Option<broadcast::Sender<OAuthCompletedNotification>>,
}

impl NotificationContext {
    /// Create new notification context
    #[must_use]
    pub const fn new(
        #[cfg(feature = "transport-websocket")] websocket_manager: Arc<WebSocketManager>,
        #[cfg(feature = "transport-sse")] sse_manager: Arc<SseManager>,
        oauth_notification_sender: Option<broadcast::Sender<OAuthCompletedNotification>>,
    ) -> Self {
        Self {
            #[cfg(feature = "transport-websocket")]
            websocket_manager,
            #[cfg(feature = "transport-sse")]
            sse_manager,
            oauth_notification_sender,
        }
    }

    /// Get `WebSocket` manager for connection management
    #[cfg(feature = "transport-websocket")]
    #[must_use]
    pub const fn websocket_manager(&self) -> &Arc<WebSocketManager> {
        &self.websocket_manager
    }

    /// Get SSE manager for Server-Sent Events
    #[cfg(feature = "transport-sse")]
    #[must_use]
    pub const fn sse_manager(&self) -> &Arc<SseManager> {
        &self.sse_manager
    }

    /// Get OAuth notification sender for broadcasting completion events
    #[must_use]
    pub const fn oauth_notification_sender(
        &self,
    ) -> &Option<broadcast::Sender<OAuthCompletedNotification>> {
        &self.oauth_notification_sender
    }
}
