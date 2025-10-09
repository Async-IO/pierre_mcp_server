// ABOUTME: Server-Sent Events implementation for MCP protocol message streaming
// ABOUTME: Handles bidirectional MCP communication over SSE for MCP client compatibility
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::{protocol::McpRequest, resources::ServerResources, tool_handlers::ToolHandlers};
use crate::mcp::protocol::McpResponse;
use anyhow::Result;
use futures_util::{stream::Stream, StreamExt};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

/// SSE message format for MCP protocol
#[derive(Debug)]
pub struct SseMessage {
    pub event_type: String,
    pub data: String,
}

impl SseMessage {
    /// Format message as SSE protocol string
    #[must_use]
    pub fn format(&self) -> String {
        format!("event: {}\ndata: {}\n\n", self.event_type, self.data)
    }
}

/// SSE connection manager for MCP protocol streaming
pub struct McpSseConnection {
    resources: Arc<ServerResources>,
    sender: mpsc::UnboundedSender<SseMessage>,
}

impl McpSseConnection {
    /// Create new MCP SSE connection
    #[must_use]
    pub fn new(resources: Arc<ServerResources>) -> (Self, mpsc::UnboundedReceiver<SseMessage>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let connection = Self { resources, sender };
        (connection, receiver)
    }

    /// Process MCP request and send response via SSE
    ///
    /// # Errors
    /// Returns error if the request processing fails or SSE transmission fails
    pub async fn handle_mcp_request(&self, request: McpRequest) -> Result<()> {
        // Process the MCP request using existing protocol handler
        let response =
            ToolHandlers::handle_tools_call_with_resources(request, &self.resources).await;

        // Convert response to SSE message
        let event_type = match response.id {
            Some(Value::Null) | None => "notification",
            _ => "response",
        };

        let message = SseMessage {
            event_type: event_type.to_string(),
            data: serde_json::to_string(&response)?,
        };

        // Send message through SSE stream
        self.sender
            .send(message)
            .map_err(|_| anyhow::anyhow!("Failed to send SSE message"))?;

        Ok(())
    }

    /// Send connection established event
    ///
    /// # Errors
    /// Returns error if SSE channel is disconnected or message sending fails
    pub fn send_connection_established(&self) -> Result<()> {
        let message = SseMessage {
            event_type: "connected".to_string(),
            data: "MCP SSE transport ready".to_string(),
        };

        self.sender
            .send(message)
            .map_err(|_| anyhow::anyhow!("Failed to send connection event"))?;

        Ok(())
    }

    /// Send error event
    ///
    /// # Errors
    /// Returns error if JSON serialization fails or SSE channel is disconnected
    pub fn send_error(&self, error_message: &str) -> Result<()> {
        let error_response =
            McpResponse::error(Some(Value::Null), -32603, error_message.to_string());

        let message = SseMessage {
            event_type: "error".to_string(),
            data: serde_json::to_string(&error_response)?,
        };

        self.sender
            .send(message)
            .map_err(|_| anyhow::anyhow!("Failed to send error event"))?;

        Ok(())
    }
}

/// Create SSE event stream for MCP protocol communication with sequential event IDs
pub fn create_mcp_sse_stream(
    resources: Arc<ServerResources>,
    _authorization: Option<String>,
) -> impl Stream<Item = Result<warp::sse::Event, warp::Error>> + Send {
    let (connection, receiver) = McpSseConnection::new(resources);

    // Send connection established event
    if let Err(e) = connection.send_connection_established() {
        tracing::error!("Failed to send connection established event: {}", e);
    }

    // Convert receiver to stream of SSE Event objects with sequential IDs
    let mut event_id: u64 = 0;
    UnboundedReceiverStream::new(receiver).map(move |message| {
        event_id += 1;
        Ok(warp::sse::Event::default()
            .id(event_id.to_string())
            .event(&message.event_type)
            .data(&message.data))
    })
}

/// Handle MCP request sent via query parameters or POST data for SSE
///
/// # Errors
/// Returns error if request data is missing, malformed, or request processing fails
pub async fn handle_mcp_sse_request(
    resources: Arc<ServerResources>,
    request_data: Option<String>,
    authorization: Option<String>,
) -> Result<McpResponse> {
    // Parse MCP request from request data
    let request_json: Value = match request_data {
        Some(data) => serde_json::from_str(&data)?,
        None => return Err(anyhow::anyhow!("No request data provided")),
    };

    let mut request: McpRequest = serde_json::from_value(request_json)?;

    // Add authorization if not present in request
    if request.auth_token.is_none() {
        request.auth_token = authorization;
    }

    // Process the request
    Ok(ToolHandlers::handle_tools_call_with_resources(request, &resources).await)
}
