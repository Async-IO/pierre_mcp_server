// ABOUTME: Transport coordination for MCP server with stdio, HTTP, and SSE transports
// ABOUTME: Manages notification channels and coordinates multiple transport methods
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - Arc resource clones for parallel transport protocol handling
// - Shared resource distribution across stdio, SSE, and HTTP transports

use super::resources::ServerResources;
use crate::errors::{AppError, AppResult};
use crate::mcp::schema::OAuthCompletedNotification;
use std::sync::Arc;
#[cfg(feature = "transport-stdio")]
use tokio::io::{stdin, stdout, AsyncBufReadExt, BufReader};
use tokio::sync::broadcast;
#[cfg(feature = "transport-sse")]
use tokio::sync::broadcast::error::RecvError;
#[cfg(feature = "transport-stdio")]
use tokio::sync::mpsc;
#[cfg(feature = "transport-stdio")]
use tokio::sync::Mutex;
#[cfg(feature = "transport-http")]
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};
#[cfg(feature = "transport-sse")]
use uuid::Uuid;

/// Log the status of a transport feature
fn log_transport_status(name: &str, enabled: bool, extra: Option<String>) {
    let status = if enabled { "ENABLED" } else { "DISABLED" };
    match extra {
        Some(details) if enabled => info!("  - {name} transport: {status} ({details})"),
        _ => info!("  - {name} transport: {status}"),
    }
}

/// Manages multiple transport methods for MCP communication
///
/// The transport manager coordinates stdio, HTTP, and SSE transports based on
/// enabled feature flags. At least one transport must be enabled.
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

    /// Start stdio transport only (no HTTP/SSE transports)
    ///
    /// This mode is useful for MCP clients that communicate via stdin/stdout
    /// and do not need HTTP endpoints.
    ///
    /// # Errors
    /// Returns an error if stdio transport setup or processing fails
    #[cfg(feature = "transport-stdio")]
    pub async fn start_stdio_only(&self) -> AppResult<()> {
        info!("Starting MCP server in stdio-only mode (HTTP/SSE disabled)");

        let notification_receiver = self.notification_sender.subscribe();

        let mut resources_clone = (*self.resources).clone();
        resources_clone.set_oauth_notification_sender(self.notification_sender.clone());

        let stdout_handle = Arc::new(Mutex::new(stdout()));
        let sampling_peer = Arc::new(super::sampling_peer::SamplingPeer::new(stdout_handle));
        resources_clone.set_sampling_peer(sampling_peer);

        Self::spawn_progress_handler(&mut resources_clone);

        let shared_resources = Arc::new(resources_clone);

        let stdio_transport = StdioTransport::new(shared_resources);
        stdio_transport.run(notification_receiver).await
    }

    /// Start stdio transport only (stub when feature is disabled)
    #[cfg(not(feature = "transport-stdio"))]
    pub async fn start_stdio_only(&self) -> AppResult<()> {
        Err(AppError::config(
            "stdio transport is not available - enable the 'transport-stdio' feature",
        ))
    }

    /// Spawn progress notification handler
    #[cfg(feature = "transport-stdio")]
    fn spawn_progress_handler(resources: &mut ServerResources) {
        let (progress_tx, mut progress_rx) = mpsc::unbounded_channel();
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
    #[cfg(feature = "transport-stdio")]
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
    #[cfg(feature = "transport-sse")]
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
    #[cfg(feature = "transport-http")]
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

    #[cfg(feature = "transport-http")]
    async fn handle_server_restart(result: AppResult<()>) {
        match result {
            Ok(()) => {
                error!("HTTP server unexpectedly completed - restarting in 5 seconds...");
                sleep(Duration::from_secs(5)).await;
            }
            Err(e) => {
                error!("HTTP server failed: {} - restarting in 10 seconds...", e);
                sleep(Duration::from_secs(10)).await;
            }
        }
    }

    /// Unified server startup using existing transport coordination
    ///
    /// Conditionally starts transports based on enabled features:
    /// - `transport-stdio`: stdio transport for MCP communication
    /// - `transport-sse`: SSE notification forwarder
    /// - `transport-http`: HTTP server with Axum
    async fn start_legacy_unified_server(&self, port: u16) -> AppResult<()> {
        info!("Starting MCP server with configured transports (Axum framework)");
        Self::log_enabled_transports(port);

        let shared_resources = self.prepare_resources();

        self.spawn_background_transports(&shared_resources);

        Self::run_primary_transport(shared_resources, port).await
    }

    /// Log which transports are enabled at startup
    fn log_enabled_transports(port: u16) {
        log_transport_status("stdio", cfg!(feature = "transport-stdio"), None);
        log_transport_status("SSE", cfg!(feature = "transport-sse"), None);
        log_transport_status(
            "HTTP",
            cfg!(feature = "transport-http"),
            Some(format!("port {port}")),
        );
        log_transport_status("WebSocket", cfg!(feature = "transport-websocket"), None);
    }

    /// Prepare resources for transport initialization
    fn prepare_resources(&self) -> Arc<ServerResources> {
        let mut resources_clone = (*self.resources).clone();
        resources_clone.set_oauth_notification_sender(self.notification_sender.clone());

        #[cfg(feature = "transport-stdio")]
        {
            use tokio::io::stdout;
            let stdout_handle = Arc::new(Mutex::new(stdout()));
            let sampling_peer = Arc::new(super::sampling_peer::SamplingPeer::new(stdout_handle));
            resources_clone.set_sampling_peer(sampling_peer);
            Self::spawn_progress_handler(&mut resources_clone);
        }

        Arc::new(resources_clone)
    }

    /// Spawn background transports (stdio, SSE)
    fn spawn_background_transports(&self, shared_resources: &Arc<ServerResources>) {
        #[cfg(feature = "transport-stdio")]
        {
            let notification_receiver = self.notification_sender.subscribe();
            Self::spawn_stdio_transport(shared_resources.clone(), notification_receiver);
        }

        #[cfg(feature = "transport-sse")]
        {
            let sse_notification_receiver = self.notification_sender.subscribe();
            Self::spawn_sse_forwarder(shared_resources.clone(), sse_notification_receiver);
        }
    }

    /// Run the primary transport (HTTP or wait for signal)
    #[cfg(feature = "transport-http")]
    async fn run_primary_transport(
        shared_resources: Arc<ServerResources>,
        port: u16,
    ) -> AppResult<()> {
        Self::run_http_server_loop(shared_resources, port).await
    }

    #[cfg(not(feature = "transport-http"))]
    async fn run_primary_transport(
        shared_resources: Arc<ServerResources>,
        port: u16,
    ) -> AppResult<()> {
        let _ = (shared_resources, port); // Suppress unused warnings

        #[cfg(feature = "transport-stdio")]
        {
            info!("Running in non-HTTP mode with stdio transport");
            tokio::signal::ctrl_c()
                .await
                .map_err(|e| AppError::internal(format!("Failed to wait for ctrl-c: {e}")))?;
            info!("Received shutdown signal, exiting...");
            return Ok(());
        }

        #[cfg(not(feature = "transport-stdio"))]
        {
            warn!("No transports enabled - server has nothing to do");
            Err(AppError::config(
                "No transports enabled. Enable at least one of: transport-http, transport-stdio",
            ))
        }
    }
}

/// Handles stdio transport for MCP communication
#[cfg(feature = "transport-stdio")]
pub struct StdioTransport {
    resources: Arc<ServerResources>,
}

#[cfg(feature = "transport-stdio")]
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
    ) -> AppResult<()> {
        info!("MCP stdio transport ready - listening on stdin/stdout with sampling support");

        let stdin_handle = stdin();
        let mut lines = BufReader::new(stdin_handle).lines();
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
#[cfg(feature = "transport-sse")]
pub struct SseNotificationForwarder;

#[cfg(feature = "transport-sse")]
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

        match Uuid::parse_str(user_id_str) {
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
    fn handle_recv_result(result: Result<OAuthCompletedNotification, RecvError>) -> bool {
        match result {
            Ok(notification) => {
                info!(
                    "Forwarding OAuth notification to SSE clients: {:?}",
                    notification
                );
                Self::process_notification(&notification);
                true
            }
            Err(RecvError::Closed) => {
                info!("OAuth notification channel closed, shutting down SSE forwarder");
                false
            }
            Err(RecvError::Lagged(skipped)) => {
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
