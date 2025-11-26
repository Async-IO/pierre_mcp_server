// ABOUTME: Sampling peer for server-initiated LLM requests to MCP clients
// ABOUTME: Manages request correlation and response routing for bidirectional MCP sampling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::schema::{CreateMessageRequest, CreateMessageResult};
use crate::errors::{AppError, AppResult};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex, RwLock};
use tokio::time::{timeout, Duration};
use tracing::{debug, warn};

/// Type alias for pending request sender
type ResponseSender = oneshot::Sender<AppResult<Value>>;

/// Manages server-initiated sampling requests to MCP clients
///
/// This struct provides the infrastructure for bidirectional MCP communication,
/// allowing the server to request LLM inference from the client and wait for responses.
#[derive(Clone)]
pub struct SamplingPeer {
    /// Counter for generating unique request IDs
    request_counter: Arc<RwLock<u64>>,
    /// Pending requests awaiting responses from client
    pending_requests: Arc<Mutex<HashMap<Value, ResponseSender>>>,
    /// Stdout channel for sending requests to client
    stdout: Arc<Mutex<tokio::io::Stdout>>,
}

impl SamplingPeer {
    /// Create a new sampling peer with access to stdout
    #[must_use]
    pub fn new(stdout: Arc<Mutex<tokio::io::Stdout>>) -> Self {
        Self {
            request_counter: Arc::new(RwLock::new(0)),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            stdout,
        }
    }

    /// Generate a unique request ID
    async fn next_request_id(&self) -> Value {
        let counter_value = {
            let mut counter = self.request_counter.write().await;
            *counter += 1;
            *counter
        };
        serde_json::json!(format!("sampling-{counter_value}"))
    }

    /// Send a sampling request to the client and wait for response
    ///
    /// # Errors
    /// Returns an error if:
    /// - Request serialization fails
    /// - Writing to stdout fails
    /// - Response timeout occurs (30 seconds)
    /// - Client returns an error response
    pub async fn create_message(
        &self,
        request: CreateMessageRequest,
    ) -> Result<CreateMessageResult> {
        let request_id = self.next_request_id().await;

        // Create oneshot channel for response
        let (tx, rx) = oneshot::channel();

        // Register pending request
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(request_id.clone(), tx);
        }

        // Build MCP sampling request
        let mcp_request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "sampling/createMessage",
            "params": request,
            "id": request_id
        });

        // Send request to client via stdout
        {
            use tokio::io::AsyncWriteExt;

            // Prepare request JSON before locking stdout
            let request_json = serde_json::to_string(&mcp_request)?;

            debug!(
                request_id = %request_id,
                "Sending sampling request to client"
            );

            // Lock stdout only for the write operation
            let mut stdout_lock = self.stdout.lock().await;
            stdout_lock.write_all(request_json.as_bytes()).await?;
            stdout_lock.write_all(b"\n").await?;
            stdout_lock.flush().await?;
            drop(stdout_lock);
        }

        // Wait for response with timeout
        let response = timeout(Duration::from_secs(30), rx)
            .await
            .map_err(|_| AppError::internal("Sampling request timed out after 30 seconds"))??;

        // Parse response as CreateMessageResult
        match response {
            Ok(value) => serde_json::from_value(value)
                .map_err(|e| AppError::internal(format!("Invalid sampling response: {e}"))),
            Err(e) => Err(e),
        }
    }

    /// Handle incoming response from client
    ///
    /// Routes the response to the appropriate pending request handler.
    /// Returns true if the response was handled (matched a pending request).
    ///
    /// # Errors
    /// Returns an error if response processing fails
    pub async fn handle_response(
        &self,
        id: Value,
        result: Option<Value>,
        error: Option<Value>,
    ) -> Result<bool> {
        let mut pending = self.pending_requests.lock().await;

        pending.remove(&id).map_or_else(
            || Ok(false),
            |tx| {
                debug!(request_id = %id, "Routing sampling response to handler");

                if let Some(err) = error {
                    let error_msg = err
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("Unknown error");
                    let _ = tx.send(Err(AppError::external_service(
                        "MCP Client",
                        format!("Sampling error: {error_msg}"),
                    )));
                } else if let Some(res) = result {
                    let _ = tx.send(Ok(res));
                } else {
                    let _ = tx.send(Err(AppError::invalid_input(
                        "Response missing both result and error",
                    )));
                }

                Ok(true)
            },
        )
    }

    /// Cancel all pending requests (for cleanup)
    pub async fn cancel_all_pending(&self) {
        let mut pending = self.pending_requests.lock().await;
        for (id, _tx) in pending.drain() {
            warn!(request_id = %id, "Cancelled pending sampling request");
        }
    }
}
