// ABOUTME: Server lifecycle management and coordination for multi-tenant MCP server
// ABOUTME: Handles server startup, transport coordination, and notification forwarding

use super::multitenant::{McpRequest, McpResponse};
use super::schema::OAuthCompletedNotification;
use super::{
    mcp_request_processor::McpRequestProcessor, resources::ServerResources,
    transport_manager::TransportManager,
};
use anyhow::Result;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info};

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
    pub async fn run_unified_server(self, port: u16) -> Result<()> {
        let transport_manager = TransportManager::new(self.resources);
        transport_manager.start_all_transports(port).await
    }

    /// Run MCP server with only HTTP transport (for testing)
    ///
    /// # Errors
    /// Returns an error if HTTP server startup fails
    pub async fn run_http_only(self, port: u16) -> Result<()> {
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
    ) -> Result<()> {
        // Delegate to the existing comprehensive HTTP server implementation
        // This ensures we don't lose any existing functionality
        super::multitenant::MultiTenantMcpServer::run_http_server_with_resources(port, resources)
            .await
    }

    /// Run stdio transport for MCP communication
    ///
    /// # Errors
    /// Returns an error if stdio processing or I/O operations fail
    pub async fn run_stdio_transport(
        self,
        notification_receiver: tokio::sync::broadcast::Receiver<OAuthCompletedNotification>,
    ) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, BufReader};

        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin).lines();
        let stdout = Arc::new(tokio::sync::Mutex::new(tokio::io::stdout()));

        // Spawn notification handler
        let notification_stdout = stdout.clone();
        let mut notification_rx = notification_receiver;
        tokio::spawn(async move {
            while let Ok(notification) = notification_rx.recv().await {
                Self::handle_oauth_notification(notification, &notification_stdout).await;
            }
        });

        info!("MCP stdio transport started");

        // Process stdin requests
        while let Some(line) = reader.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<McpRequest>(&line) {
                Ok(request) => {
                    Self::process_mcp_request(request, &self.resources, &stdout).await?;
                }
                Err(e) => {
                    debug!("Failed to parse MCP request: {} - Line: {}", e, line);
                }
            }
        }

        Ok(())
    }

    /// Run SSE notification forwarder
    ///
    /// # Errors
    /// Returns an error if notification forwarding fails
    pub async fn run_sse_notification_forwarder(
        &self,
        mut notification_receiver: tokio::sync::broadcast::Receiver<OAuthCompletedNotification>,
    ) -> Result<()> {
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
            let _ = stdout_lock.write_all(json.as_bytes()).await;
            let _ = stdout_lock.write_all(b"\n").await;
            let _ = stdout_lock.flush().await;
            drop(stdout_lock);
        }
    }

    /// Write MCP response to stdout
    async fn write_response_to_stdout(
        response: &McpResponse,
        stdout: &Arc<tokio::sync::Mutex<tokio::io::Stdout>>,
    ) -> Result<()> {
        let response_json = serde_json::to_string(response)?;
        debug!("Sending MCP response: {}", response_json);

        let mut stdout_lock = stdout.lock().await;
        stdout_lock.write_all(response_json.as_bytes()).await?;
        stdout_lock.write_all(b"\n").await?;
        stdout_lock.flush().await?;
        drop(stdout_lock);

        Ok(())
    }

    /// Process MCP request and send response
    async fn process_mcp_request(
        request: McpRequest,
        resources: &Arc<ServerResources>,
        stdout: &Arc<tokio::sync::Mutex<tokio::io::Stdout>>,
    ) -> Result<()> {
        debug!(
            "Processing MCP request: method={}, id={:?}",
            request.method, request.id
        );

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
    InvalidRequest(String),
    SerializationError(serde_json::Error),
}

impl warp::reject::Reject for ServerError {}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRequest(msg) => write!(f, "Invalid request: {msg}"),
            Self::SerializationError(e) => write!(f, "Serialization error: {e}"),
        }
    }
}

impl std::error::Error for ServerError {}
