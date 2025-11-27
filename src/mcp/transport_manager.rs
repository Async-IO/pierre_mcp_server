// ABOUTME: Transport coordination for MCP server with stdio, HTTP, and SSE transports
// ABOUTME: Manages notification channels and coordinates multiple transport methods
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - Arc resource clones for parallel transport protocol handling
// - Shared resource distribution across stdio, SSE, and HTTP transports

use super::resources::ServerResources;
use crate::errors::{AppError, AppResult};
use crate::mcp::schema::OAuthCompletedNotification;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

/// Manages multiple transport methods for MCP communication
pub struct TransportManager {
    resources: Arc<ServerResources>,
    notification_sender: broadcast::Sender<OAuthCompletedNotification>,
}

impl TransportManager {
    /// Create a new transport manager with shared resources
    #[must_use]
    pub fn new(resources: Arc<ServerResources>) -> Self {
        let (notification_sender, _) = broadcast::channel(100);
        Self {
            resources,
            notification_sender,
        }
    }

    /// Start all transport methods (stdio, HTTP, SSE) in coordinated fashion
    ///
    /// # Errors
    /// Returns an error if transport setup or server startup fails
    pub async fn start_all_transports(&self, port: u16) -> AppResult<()> {
        info!(
            "Transport manager coordinating all transports on port {}",
            port
        );

        // Delegate to the unified server implementation
        self.start_legacy_unified_server(port).await
    }

    /// Spawn progress notification handler
    fn spawn_progress_handler(resources: &mut ServerResources) {
        let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel();
        resources.set_progress_notification_sender(progress_tx);

        tokio::spawn(async move {
            while let Some(progress_notification) = progress_rx.recv().await {
                if let Ok(json) = serde_json::to_string(&progress_notification) {
                    println!("{json}");
                }
            }
        });
    }

    /// Spawn stdio transport task
    fn spawn_stdio_transport(
        resources: Arc<ServerResources>,
        notification_receiver: broadcast::Receiver<OAuthCompletedNotification>,
    ) {
        let stdio_handle = tokio::spawn(async move {
            let stdio_transport = StdioTransport::new(resources);
            match stdio_transport.run(notification_receiver).await {
                Ok(()) => info!("stdio transport completed successfully"),
                Err(e) => warn!("stdio transport failed: {}", e),
            }
        });

        tokio::spawn(async move {
            match stdio_handle.await {
                Ok(()) => info!("stdio transport task completed"),
                Err(e) => warn!("stdio transport task failed: {}", e),
            }
        });
    }

    /// Spawn SSE notification forwarder task
    fn spawn_sse_forwarder(
        resources: Arc<ServerResources>,
        notification_receiver: broadcast::Receiver<OAuthCompletedNotification>,
    ) {
        tokio::spawn(async move {
            let sse_forwarder = SseNotificationForwarder::new(resources);
            if let Err(e) = sse_forwarder.run(notification_receiver).await {
                error!("SSE notification forwarder failed: {}", e);
            }
        });
    }

    /// Run HTTP server with restart on failure
    async fn run_http_server_loop(shared_resources: Arc<ServerResources>, port: u16) -> ! {
        loop {
            info!("Starting unified Axum HTTP server on port {}", port);

            let server = super::multitenant::MultiTenantMcpServer::new(shared_resources.clone());
            let result = server
                .run_http_server_with_resources_axum(port, shared_resources.clone())
                .await;

            Self::handle_server_restart(result).await;
        }
    }

    async fn handle_server_restart(result: AppResult<()>) {
        match result {
            Ok(()) => {
                error!("HTTP server unexpectedly completed - restarting in 5 seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
            Err(e) => {
                error!("HTTP server failed: {} - restarting in 10 seconds...", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            }
        }
    }

    /// Unified server startup using existing transport coordination
    async fn start_legacy_unified_server(&self, port: u16) -> AppResult<()> {
        info!("Starting MCP server with stdio and HTTP transports (Axum framework)");

        let notification_receiver = self.notification_sender.subscribe();
        let sse_notification_receiver = self.notification_sender.subscribe();

        let mut resources_clone = (*self.resources).clone();
        resources_clone.set_oauth_notification_sender(self.notification_sender.clone());

        let stdout = Arc::new(tokio::sync::Mutex::new(tokio::io::stdout()));
        let sampling_peer = Arc::new(super::sampling_peer::SamplingPeer::new(stdout));
        resources_clone.set_sampling_peer(sampling_peer);

        Self::spawn_progress_handler(&mut resources_clone);

        let shared_resources = Arc::new(resources_clone);

        Self::spawn_stdio_transport(shared_resources.clone(), notification_receiver);
        Self::spawn_sse_forwarder(shared_resources.clone(), sse_notification_receiver);

        Self::run_http_server_loop(shared_resources, port).await
    }
}

/// Handles stdio transport for MCP communication
pub struct StdioTransport {
    resources: Arc<ServerResources>,
}

impl StdioTransport {
    /// Creates a new stdio transport instance
    #[must_use]
    pub const fn new(resources: Arc<ServerResources>) -> Self {
        Self { resources }
    }

    /// Check if a JSON message is a sampling response
    fn is_sampling_response(message: &serde_json::Value) -> bool {
        message.get("id").is_some()
            && message.get("method").is_none()
            && (message.get("result").is_some() || message.get("error").is_some())
    }

    /// Route a sampling response to the sampling peer
    async fn route_sampling_response(
        message: &serde_json::Value,
        sampling_peer: Option<&Arc<super::sampling_peer::SamplingPeer>>,
    ) {
        let Some(peer) = sampling_peer else {
            warn!("Received sampling response but no sampling peer available");
            return;
        };

        let id = message
            .get("id")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let result = message.get("result").cloned();
        let error = message.get("error").cloned();

        match peer.handle_response(id, result, error).await {
            Ok(handled) if !handled => {
                warn!("Received response for unknown sampling request");
            }
            Ok(_) => {}
            Err(e) => {
                warn!("Failed to handle sampling response: {}", e);
            }
        }
    }

    /// Create a JSON-RPC parse error response
    fn parse_error_response() -> serde_json::Value {
        serde_json::json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32700,
                "message": "Parse error"
            },
            "id": null
        })
    }

    /// Process an MCP request and send the response
    async fn process_mcp_request(message: serde_json::Value, resources: Arc<ServerResources>) {
        match serde_json::from_value::<super::multitenant::McpRequest>(message) {
            Ok(request) => {
                let processor = super::mcp_request_processor::McpRequestProcessor::new(resources);
                if let Some(response) = processor.handle_request(request).await {
                    if let Ok(json) = serde_json::to_string(&response) {
                        println!("{json}");
                    }
                }
            }
            Err(e) => {
                warn!("Failed to parse MCP request: {}", e);
                println!("{}", Self::parse_error_response());
            }
        }
    }

    /// Process a single incoming message from stdio
    async fn process_stdio_message(
        message: serde_json::Value,
        resources: Arc<ServerResources>,
        sampling_peer: Option<&Arc<super::sampling_peer::SamplingPeer>>,
    ) {
        if Self::is_sampling_response(&message) {
            Self::route_sampling_response(&message, sampling_peer).await;
        } else {
            Self::process_mcp_request(message, resources).await;
        }
    }

    /// Run stdio transport for MCP communication
    ///
    /// # Errors
    /// Returns an error if stdio processing fails
    pub async fn run(
        &self,
        notification_receiver: broadcast::Receiver<OAuthCompletedNotification>,
    ) -> crate::errors::AppResult<()> {
        use tokio::io::{AsyncBufReadExt, BufReader};

        info!("MCP stdio transport ready - listening on stdin/stdout with sampling support");

        let stdin = tokio::io::stdin();
        let mut lines = BufReader::new(stdin).lines();
        let sampling_peer = self.resources.sampling_peer.clone();

        let resources_for_notifications = self.resources.clone();
        let notification_handle = tokio::spawn(async move {
            Self::handle_stdio_notifications(notification_receiver, resources_for_notifications)
                .await
        });

        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<serde_json::Value>(&line) {
                Ok(message) => {
                    Self::process_stdio_message(
                        message,
                        self.resources.clone(),
                        sampling_peer.as_ref(),
                    )
                    .await;
                }
                Err(e) => {
                    warn!("Invalid JSON-RPC message: {}", e);
                    println!("{}", Self::parse_error_response());
                }
            }
        }

        if let Some(peer) = &sampling_peer {
            peer.cancel_all_pending().await;
        }
        notification_handle.abort();
        Ok(())
    }

    async fn handle_stdio_notifications(
        mut receiver: broadcast::Receiver<OAuthCompletedNotification>,
        _resources: Arc<ServerResources>,
    ) -> AppResult<()> {
        info!("Stdio notification handler ready");

        while let Ok(notification) = receiver.recv().await {
            info!("Received OAuth notification for stdio: {:?}", notification);
            // Send notification to stdio client
            let notification_json = serde_json::to_string(&notification)
                .map_err(|e| AppError::internal(format!("JSON serialization failed: {e}")))?;
            println!("{notification_json}");
        }

        Ok(())
    }
}

/// Handles SSE notification forwarding
pub struct SseNotificationForwarder;

impl SseNotificationForwarder {
    /// Creates a new SSE notification forwarder instance
    #[must_use]
    pub fn new(_resources: Arc<ServerResources>) -> Self {
        Self
    }

    /// Process a single OAuth notification
    fn process_notification(notification: &OAuthCompletedNotification) {
        let Some(user_id_str) = &notification.params.user_id else {
            warn!(
                "OAuth notification missing user_id field: {:?}",
                notification
            );
            return;
        };

        match uuid::Uuid::parse_str(user_id_str) {
            Ok(user_id) => {
                info!(
                    user_id = %user_id,
                    provider = %notification.params.provider,
                    success = notification.params.success,
                    "OAuth notification processed (SSE disabled)"
                );
            }
            Err(e) => {
                warn!(
                    "Invalid user_id in OAuth notification: {} - error: {}",
                    user_id_str, e
                );
            }
        }
    }

    /// Handle the result of receiving a notification
    fn handle_recv_result(
        result: Result<OAuthCompletedNotification, broadcast::error::RecvError>,
    ) -> bool {
        match result {
            Ok(notification) => {
                info!(
                    "Forwarding OAuth notification to SSE clients: {:?}",
                    notification
                );
                Self::process_notification(&notification);
                true
            }
            Err(broadcast::error::RecvError::Closed) => {
                info!("OAuth notification channel closed, shutting down SSE forwarder");
                false
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                warn!(
                    "SSE notification forwarder lagged, skipped {} notifications",
                    skipped
                );
                true
            }
        }
    }

    /// Run SSE notification forwarding
    ///
    /// # Errors
    /// Returns an error if notification forwarding fails
    pub async fn run(
        &self,
        mut notification_receiver: broadcast::Receiver<OAuthCompletedNotification>,
    ) -> AppResult<()> {
        info!("SSE notification forwarder ready - waiting for OAuth notifications");

        loop {
            let result = notification_receiver.recv().await;
            if !Self::handle_recv_result(result) {
                break;
            }
        }

        Ok(())
    }
}
