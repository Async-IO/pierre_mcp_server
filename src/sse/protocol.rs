// ABOUTME: MCP protocol streaming implementation for session-based bidirectional communication
// ABOUTME: Handles SSE streaming of MCP protocol messages with session management
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::errors::AppError;
use crate::mcp::{
    protocol::{McpRequest, McpResponse},
    resources::ServerResources,
    tool_handlers::ToolHandlers,
};
use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// MCP protocol stream for a specific session
pub struct McpProtocolStream {
    resources: Arc<ServerResources>,
    sender: Arc<RwLock<Option<broadcast::Sender<String>>>>,
    session_id: Option<String>,
    buffer_size: usize,
}

impl McpProtocolStream {
    #[must_use]
    pub fn new(resources: Arc<ServerResources>) -> Self {
        let buffer_size = resources.config.sse.max_buffer_size;
        Self {
            resources,
            sender: Arc::new(RwLock::new(None)),
            session_id: None,
            buffer_size,
        }
    }

    /// Subscribe to MCP protocol messages for this session
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

    /// Handle MCP request and stream response
    ///
    /// # Errors
    ///
    /// Returns an error if no active sender is available for this stream
    pub async fn handle_request(&self, request: McpRequest) -> Result<()> {
        // Process the MCP request using existing handlers
        let response =
            ToolHandlers::handle_tools_call_with_resources(request, &self.resources).await;

        // Stream the response
        self.send_response(response).await?;

        Ok(())
    }

    /// Send MCP response through SSE stream
    async fn send_response(&self, response: McpResponse) -> Result<()> {
        let sender_guard = self.sender.read().await;

        if let Some(sender) = sender_guard.as_ref() {
            // Send only the JSON data - Warp will handle SSE formatting
            let json_data = serde_json::to_string(&response)?;

            sender
                .send(json_data)
                .map_err(|e| AppError::internal(format!("Failed to send MCP response: {e}")))?;

            Ok(())
        } else {
            Err(AppError::internal("No active sender for protocol stream").into())
        }
    }

    /// Send error event through SSE stream
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No active sender is available for this stream
    /// - JSON serialization fails
    /// - Sending the error event fails
    pub async fn send_error(&self, error_message: &str) -> Result<()> {
        let error_response =
            McpResponse::error(Some(Value::Null), -32603, error_message.to_string());

        let sender_guard = self.sender.read().await;

        if let Some(sender) = sender_guard.as_ref() {
            // Send only the JSON data - Warp will handle SSE formatting
            let json_data = serde_json::to_string(&error_response)?;

            sender
                .send(json_data)
                .map_err(|e| AppError::internal(format!("Failed to send error event: {e}")))?;

            Ok(())
        } else {
            Err(AppError::internal("No active sender for protocol stream").into())
        }
    }

    /// Check if stream has active subscribers
    pub async fn has_subscribers(&self) -> bool {
        let sender_guard = self.sender.read().await;
        sender_guard
            .as_ref()
            .is_some_and(|sender| sender.receiver_count() > 0)
    }

    /// Set session ID for this stream
    pub fn set_session_id(&mut self, session_id: String) {
        self.session_id = Some(session_id);
    }

    /// Get session ID for this stream
    #[must_use]
    pub const fn get_session_id(&self) -> Option<&String> {
        self.session_id.as_ref()
    }

    /// Send OAuth notification through MCP protocol stream
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No active sender is available for this stream
    /// - JSON serialization fails
    /// - Sending the notification fails
    pub async fn send_oauth_notification(
        &self,
        notification: &crate::database::oauth_notifications::OAuthNotification,
    ) -> Result<()> {
        tracing::debug!(
            "send_oauth_notification called for provider: {}",
            notification.provider
        );

        let sender_guard = self.sender.read().await;

        if let Some(sender) = sender_guard.as_ref() {
            let receiver_count = sender.receiver_count();
            tracing::debug!("Active SSE receivers for this stream: {}", receiver_count);

            // Format as proper JSON-RPC notification (no id field)
            let mcp_notification = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/oauth_completed",
                "params": {
                    "provider": notification.provider,
                    "success": notification.success,
                    "message": notification.message,
                    "user_id": notification.user_id,
                }
            });

            // Send only the JSON data - Warp will handle SSE formatting
            let json_data = serde_json::to_string(&mcp_notification)?;

            tracing::debug!("JSON data to send: {}", json_data);

            let result = sender.send(json_data);

            match result {
                Ok(receiver_count) => {
                    tracing::info!("OAuth notification broadcast succeeded! Reached {} receiver(s) for provider {}", receiver_count, notification.provider);
                }
                Err(e) => {
                    tracing::error!("OAuth notification broadcast FAILED: {}", e);
                    return Err(AppError::internal(format!(
                        "Failed to send OAuth notification: {e}"
                    ))
                    .into());
                }
            }

            Ok(())
        } else {
            tracing::error!("No active sender for protocol stream");
            Err(AppError::internal("No active sender for protocol stream").into())
        }
    }
}
