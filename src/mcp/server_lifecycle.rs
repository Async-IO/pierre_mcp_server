// ABOUTME: Server lifecycle management and coordination for multi-tenant MCP server
// ABOUTME: Handles server startup, transport coordination, and notification forwarding
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::multitenant::{McpRequest, McpResponse};
use super::sampling_peer::SamplingPeer;
use super::schema::OAuthCompletedNotification;
use super::{
    mcp_request_processor::McpRequestProcessor, resources::ServerResources,
    transport_manager::TransportManager,
};
use crate::errors::{AppError, AppResult};
use serde_json::Value;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info, warn};

/// Manages server lifecycle, startup, and transport coordination
pub struct ServerLifecycle {
    resources: Arc<ServerResources>,
}

impl ServerLifecycle {
    /// Create a new server lifecycle manager
    #[must_use]
    pub const fn new(resources: Arc<ServerResources>) -> Self {
        Self { resources }
    }

    /// Start unified server (HTTP + MCP) on specified port
    ///
    /// # Errors
    /// Returns an error if server startup or transport coordination fails
    pub async fn run_unified_server(self, port: u16) -> AppResult<()> {
        let transport_manager = TransportManager::new(self.resources);
        transport_manager.start_all_transports(port).await
    }

    /// Run MCP server with only HTTP transport (for testing)
    ///
    /// # Errors
    /// Returns an error if HTTP server startup fails
    pub async fn run_http_only(self, port: u16) -> AppResult<()> {
        info!(
            "Starting MCP server with HTTP transport only on port {}",
            port
        );

        let resources = self.resources.clone();
        self.run_http_server_with_resources(port, resources).await
    }

    /// Run HTTP server with shared resources
    async fn run_http_server_with_resources(
        self,
        port: u16,
        resources: Arc<ServerResources>,
    ) -> AppResult<()> {
        // Delegate to the existing comprehensive HTTP server implementation
        // This ensures we don't lose any existing functionality
        let server = super::multitenant::MultiTenantMcpServer::new(resources.clone());
        server
            .run_http_server_with_resources_axum(port, resources)
            .await
    }

    /// Check if a JSON message is a sampling response
    fn is_sampling_response(message: &Value) -> bool {
        message.get("id").is_some()
            && message.get("method").is_none()
            && (message.get("result").is_some() || message.get("error").is_some())
    }

    /// Route a sampling response to the sampling peer
    async fn route_sampling_response(message: &Value, sampling_peer: &Arc<SamplingPeer>) {
        let id = message.get("id").cloned().unwrap_or(Value::Null);
        let result = message.get("result").cloned();
        let error = message.get("error").cloned();

        match sampling_peer.handle_response(id, result, error).await {
            Ok(handled) if !handled => {
                warn!("Received response for unknown sampling request");
            }
            Ok(_) => {}
            Err(e) => {
                warn!("Failed to handle sampling response: {}", e);
            }
        }
    }

    /// Route an MCP request for processing
    async fn route_mcp_request(
        message: Value,
        resources: &Arc<ServerResources>,
        stdout: &Arc<tokio::sync::Mutex<tokio::io::Stdout>>,
        sampling_peer: &Arc<SamplingPeer>,
    ) {
        match serde_json::from_value::<McpRequest>(message.clone()) {
            Ok(request) => {
                if let Err(e) =
                    Self::process_mcp_request(request, resources, stdout, sampling_peer).await
                {
                    warn!("Failed to process MCP request: {}", e);
                }
            }
            Err(e) => {
                debug!("Failed to parse MCP request: {} - Message: {}", e, message);
            }
        }
    }

    /// Process a single incoming message from stdio
    async fn process_stdio_message(
        message: Value,
        resources: &Arc<ServerResources>,
        stdout: &Arc<tokio::sync::Mutex<tokio::io::Stdout>>,
        sampling_peer: &Arc<SamplingPeer>,
    ) {
        if Self::is_sampling_response(&message) {
            Self::route_sampling_response(&message, sampling_peer).await;
        } else {
            Self::route_mcp_request(message, resources, stdout, sampling_peer).await;
        }
    }

    /// Run stdio transport for MCP communication
    ///
    /// # Errors
    /// Returns an error if stdio processing or I/O operations fail
    pub async fn run_stdio_transport(
        self,
        notification_receiver: tokio::sync::broadcast::Receiver<OAuthCompletedNotification>,
    ) -> AppResult<()> {
        use tokio::io::{AsyncBufReadExt, BufReader};

        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin).lines();
        let stdout = Arc::new(tokio::sync::Mutex::new(tokio::io::stdout()));
        let sampling_peer = Arc::new(SamplingPeer::new(stdout.clone()));

        Self::spawn_notification_handler(notification_receiver, stdout.clone());

        info!("MCP stdio transport started with sampling support");

        while let Some(line) = reader.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<Value>(&line) {
                Ok(message) => {
                    Self::process_stdio_message(message, &self.resources, &stdout, &sampling_peer)
                        .await;
                }
                Err(e) => {
                    debug!("Failed to parse JSON-RPC message: {} - Line: {}", e, line);
                }
            }
        }

        sampling_peer.cancel_all_pending().await;
        Ok(())
    }

    /// Spawn notification handler for OAuth completion events
    fn spawn_notification_handler(
        notification_receiver: tokio::sync::broadcast::Receiver<OAuthCompletedNotification>,
        stdout: Arc<tokio::sync::Mutex<tokio::io::Stdout>>,
    ) {
        let mut notification_rx = notification_receiver;
        tokio::spawn(async move {
            while let Ok(notification) = notification_rx.recv().await {
                Self::handle_oauth_notification(notification, &stdout).await;
            }
        });
    }

    /// Run SSE notification forwarder
    ///
    /// # Errors
    /// Returns an error if notification forwarding fails
    pub async fn run_sse_notification_forwarder(
        &self,
        mut notification_receiver: tokio::sync::broadcast::Receiver<OAuthCompletedNotification>,
    ) -> AppResult<()> {
        info!("Starting SSE notification forwarder");

        while let Ok(_notification) = notification_receiver.recv().await {
            // Forward OAuth notifications to SSE connections
            // This would be implemented based on the actual SSE notification system
        }

        Ok(())
    }

    /// Handle OAuth completion notification
    async fn handle_oauth_notification(
        notification: OAuthCompletedNotification,
        stdout: &Arc<tokio::sync::Mutex<tokio::io::Stdout>>,
    ) {
        let notification_msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/oauth_completed",
            "params": {
                "provider": notification.params.provider,
                "user_id": notification.params.user_id,
                "success": notification.params.success,
                "message": notification.params.message
            }
        });

        if let Ok(json) = serde_json::to_string(&notification_msg) {
            let mut stdout_lock = stdout.lock().await;
            if let Err(e) = stdout_lock.write_all(json.as_bytes()).await {
                tracing::error!(error = ?e, "Failed to write OAuth notification to stdout");
            }
            if let Err(e) = stdout_lock.write_all(b"\n").await {
                tracing::error!(error = ?e, "Failed to write newline to stdout");
            }
            if let Err(e) = stdout_lock.flush().await {
                tracing::error!(error = ?e, "Failed to flush stdout");
            }
            drop(stdout_lock);
        }
    }

    /// Write MCP response to stdout
    async fn write_response_to_stdout(
        response: &McpResponse,
        stdout: &Arc<tokio::sync::Mutex<tokio::io::Stdout>>,
    ) -> AppResult<()> {
        let response_json = serde_json::to_string(response)
            .map_err(|e| AppError::internal(format!("JSON serialization failed: {e}")))?;
        debug!("Sending MCP response: {}", response_json);

        let mut stdout_lock = stdout.lock().await;
        stdout_lock
            .write_all(response_json.as_bytes())
            .await
            .map_err(|e| AppError::internal(format!("Transport error: {e}")))?;
        stdout_lock
            .write_all(b"\n")
            .await
            .map_err(|e| AppError::internal(format!("Transport error: {e}")))?;
        stdout_lock
            .flush()
            .await
            .map_err(|e| AppError::internal(format!("Transport error: {e}")))?;
        drop(stdout_lock);

        Ok(())
    }

    /// Process MCP request and send response
    async fn process_mcp_request(
        request: McpRequest,
        resources: &Arc<ServerResources>,
        stdout: &Arc<tokio::sync::Mutex<tokio::io::Stdout>>,
        _sampling_peer: &Arc<SamplingPeer>,
    ) -> anyhow::Result<()> {
        debug!(
            "Processing MCP request: method={}, id={:?}",
            request.method, request.id
        );

        // Sampling peer is available via resources.sampling_peer in the request processor
        let processor = McpRequestProcessor::new(resources.clone());
        if let Some(response) = processor.handle_request(request).await {
            Self::write_response_to_stdout(&response, stdout).await?;
        }

        Ok(())
    }
}

/// Server error types
#[derive(Debug)]
pub enum ServerError {
    /// Request is malformed or invalid
    InvalidRequest(String),
    /// JSON serialization/deserialization failed
    SerializationError(serde_json::Error),
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRequest(msg) => write!(f, "Invalid request: {msg}"),
            Self::SerializationError(e) => write!(f, "Serialization error: {e}"),
        }
    }
}

impl std::error::Error for ServerError {}
