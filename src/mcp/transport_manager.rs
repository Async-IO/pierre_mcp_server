// ABOUTME: Transport coordination for MCP server with stdio, HTTP, and SSE transports
// ABOUTME: Manages notification channels and coordinates multiple transport methods

// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - Arc resource clones for parallel transport protocol handling
// - Shared resource distribution across stdio, SSE, and HTTP transports

use super::resources::ServerResources;
use crate::mcp::schema::OAuthCompletedNotification;
use anyhow::Result;
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
    pub async fn start_all_transports(&self, port: u16) -> Result<()> {
        info!(
            "Transport manager coordinating all transports on port {}",
            port
        );

        // Delegate to the unified server implementation
        self.start_legacy_unified_server(port).await
    }

    /// Unified server startup using existing transport coordination
    async fn start_legacy_unified_server(&self, port: u16) -> Result<()> {
        info!("Starting MCP server with stdio and HTTP transports");

        // Use the notification sender from the struct instance
        let notification_receiver = self.notification_sender.subscribe();
        let sse_notification_receiver = self.notification_sender.subscribe();

        // Set up notification sender in resources for OAuth callbacks
        let mut resources_clone = (*self.resources).clone(); // Safe: ServerResources clone for notification setup
        resources_clone.set_oauth_notification_sender(self.notification_sender.clone()); // Safe: Sender clone for notification
        let shared_resources = Arc::new(resources_clone);

        // Start stdio transport in background
        let resources_for_stdio = shared_resources.clone();
        let stdio_handle = tokio::spawn(async move {
            let stdio_transport = StdioTransport::new(resources_for_stdio);
            match stdio_transport.run(notification_receiver).await {
                Ok(()) => info!("stdio transport completed successfully"),
                Err(e) => warn!("stdio transport failed: {}", e),
            }
        });

        // Monitor stdio transport in background
        tokio::spawn(async move {
            match stdio_handle.await {
                Ok(()) => info!("stdio transport task completed"),
                Err(e) => warn!("stdio transport task failed: {}", e),
            }
        });

        // Start SSE notification forwarder task
        let resources_for_sse = shared_resources.clone();
        tokio::spawn(async move {
            let sse_forwarder = SseNotificationForwarder::new(resources_for_sse);
            if let Err(e) = sse_forwarder.run(sse_notification_receiver).await {
                error!("SSE notification forwarder failed: {}", e);
            }
        });

        // Run unified HTTP server with all routes (OAuth2, MCP, etc.) - this should run indefinitely
        loop {
            info!("Starting unified HTTP server on port {}", port);

            // Clone shared resources for each iteration since run_http_server_with_resources takes ownership
            let server = super::multitenant::MultiTenantMcpServer::new(shared_resources.clone());
            match server
                .run_http_server_with_resources(port, shared_resources.clone())
                .await
            {
                Ok(()) => {
                    error!("HTTP server unexpectedly completed - this should never happen");
                    error!("HTTP server should run indefinitely. Restarting in 5 seconds...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
                Err(e) => {
                    error!("HTTP server failed: {}", e);
                    error!("Restarting HTTP server in 10 seconds...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                }
            }
        }
    }
}

/// Handles stdio transport for MCP communication
pub struct StdioTransport {
    resources: Arc<ServerResources>,
}

impl StdioTransport {
    #[must_use]
    pub const fn new(resources: Arc<ServerResources>) -> Self {
        Self { resources }
    }

    /// Run stdio transport for MCP communication
    ///
    /// # Errors
    /// Returns an error if stdio processing fails
    pub async fn run(
        &self,
        notification_receiver: broadcast::Receiver<OAuthCompletedNotification>,
    ) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, BufReader};

        info!("MCP stdio transport ready - listening on stdin/stdout");

        let stdin = tokio::io::stdin();
        let mut lines = BufReader::new(stdin).lines();

        // Spawn notification handler for stdio transport
        let resources_for_notifications = self.resources.clone();
        let notification_handle = tokio::spawn(async move {
            Self::handle_stdio_notifications(notification_receiver, resources_for_notifications)
                .await
        });

        // Main stdio loop
        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }

            match Self::process_stdio_line(&line) {
                Ok(response) => {
                    if let Some(resp) = response {
                        println!("{resp}");
                    }
                }
                Err(e) => {
                    warn!("Error processing stdio input: {}", e);
                    let error_response = serde_json::json!({
                        "jsonrpc": "2.0",
                        "error": {
                            "code": -32603,
                            "message": "Internal error"
                        },
                        "id": null
                    });
                    println!("{error_response}");
                }
            }
        }

        // Clean up notification handler
        notification_handle.abort();
        Ok(())
    }

    fn process_stdio_line(line: &str) -> Result<Option<String>> {
        // Parse JSON-RPC request
        let _request: serde_json::Value = serde_json::from_str(line)?;

        // Process MCP request (processed by McpRequestProcessor in actual implementation)
        // For now, return a simple response
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "result": "transport_manager_active",
            "id": 1
        });

        Ok(Some(serde_json::to_string(&response)?))
    }

    async fn handle_stdio_notifications(
        mut receiver: broadcast::Receiver<OAuthCompletedNotification>,
        _resources: Arc<ServerResources>,
    ) -> Result<()> {
        info!("Stdio notification handler ready");

        while let Ok(notification) = receiver.recv().await {
            info!("Received OAuth notification for stdio: {:?}", notification);
            // Send notification to stdio client
            let notification_json = serde_json::to_string(&notification)?;
            println!("{notification_json}");
        }

        Ok(())
    }
}

/// Handles SSE notification forwarding
pub struct SseNotificationForwarder {
    resources: Arc<ServerResources>,
}

impl SseNotificationForwarder {
    #[must_use]
    pub const fn new(resources: Arc<ServerResources>) -> Self {
        Self { resources }
    }

    /// Run SSE notification forwarding
    ///
    /// # Errors
    /// Returns an error if notification forwarding fails
    pub async fn run(
        &self,
        mut notification_receiver: broadcast::Receiver<OAuthCompletedNotification>,
    ) -> Result<()> {
        info!("SSE notification forwarder ready - waiting for OAuth notifications");

        loop {
            match notification_receiver.recv().await {
                Ok(notification) => {
                    info!(
                        "Forwarding OAuth notification to SSE clients: {:?}",
                        notification
                    );

                    // Extract user_id from notification
                    if let Some(user_id_str) = &notification.params.user_id {
                        match uuid::Uuid::parse_str(user_id_str) {
                            Ok(user_id) => {
                                // Create OAuthNotification from the received notification
                                let oauth_notification =
                                    crate::database::oauth_notifications::OAuthNotification {
                                        id: uuid::Uuid::new_v4().to_string(),
                                        user_id: user_id.to_string(),
                                        provider: notification.params.provider.clone(),
                                        success: notification.params.success,
                                        message: notification.params.message.clone(),
                                        expires_at: None,
                                        created_at: chrono::Utc::now(),
                                        read_at: None,
                                    };

                                // Send notification to SSE notification streams (for direct clients)
                                match self
                                    .resources
                                    .sse_manager
                                    .send_notification(user_id, &oauth_notification)
                                    .await
                                {
                                    Ok(()) => {
                                        info!("Successfully forwarded OAuth notification to notification stream for user {}", user_id);
                                    }
                                    Err(e) => {
                                        warn!("Failed to forward OAuth notification to notification stream for user {}: {}", user_id, e);
                                    }
                                }

                                // Also send to MCP protocol streams (for bridges like Claude Desktop)
                                match self
                                    .resources
                                    .sse_manager
                                    .send_oauth_notification_to_protocol_streams(
                                        user_id,
                                        &oauth_notification,
                                    )
                                    .await
                                {
                                    Ok(()) => {
                                        info!("Successfully forwarded OAuth notification to protocol streams for user {}", user_id);
                                    }
                                    Err(e) => {
                                        warn!("No active protocol streams to forward OAuth notification for user {}: {}", user_id, e);
                                    }
                                }
                            }
                            Err(e) => {
                                warn!(
                                    "Invalid user_id in OAuth notification: {} - error: {}",
                                    user_id_str, e
                                );
                            }
                        }
                    } else {
                        warn!(
                            "OAuth notification missing user_id field: {:?}",
                            notification
                        );
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    info!("OAuth notification channel closed, shutting down SSE forwarder");
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!(
                        "SSE notification forwarder lagged, skipped {} notifications",
                        skipped
                    );
                }
            }
        }

        Ok(())
    }
}
