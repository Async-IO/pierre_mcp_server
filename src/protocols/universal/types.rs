// ABOUTME: Core types for universal protocol system
// ABOUTME: Request, response, and executor types used across the universal protocol
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::protocols::universal::executor::UniversalExecutor;
use crate::protocols::ProtocolError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Type alias for progress callback function
type ProgressCallback = Arc<dyn Fn(f64, Option<f64>, Option<String>) + Send + Sync>;

/// Cancellation token for long-running operations
#[derive(Debug, Clone)]
pub struct CancellationToken {
    /// Flag indicating if the operation has been cancelled
    cancelled: Arc<RwLock<bool>>,
}

impl CancellationToken {
    /// Create a new cancellation token
    #[must_use]
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(RwLock::new(false)),
        }
    }

    /// Check if the operation has been cancelled
    pub async fn is_cancelled(&self) -> bool {
        *self.cancelled.read().await
    }

    /// Cancel the operation
    pub async fn cancel(&self) {
        *self.cancelled.write().await = true;
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Progress reporter for long-running operations
#[derive(Clone)]
pub struct ProgressReporter {
    /// Progress token identifying the operation
    pub progress_token: String,
    /// Callback for reporting progress
    report_fn: Option<ProgressCallback>,
}

impl Debug for ProgressReporter {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("ProgressReporter")
            .field("progress_token", &self.progress_token)
            .field("report_fn", &self.report_fn.as_ref().map(|_| "<callback>"))
            .finish()
    }
}

impl ProgressReporter {
    /// Create a new progress reporter
    #[must_use]
    pub fn new(progress_token: String) -> Self {
        Self {
            progress_token,
            report_fn: None,
        }
    }

    /// Report progress with optional total and message
    pub fn report(&self, progress: f64, total: Option<f64>, message: Option<String>) {
        if let Some(ref report_fn) = self.report_fn {
            report_fn(progress, total, message);
        }
    }

    /// Set the progress reporting callback
    pub fn set_callback<F>(&mut self, callback: F)
    where
        F: Fn(f64, Option<f64>, Option<String>) + Send + Sync + 'static,
    {
        self.report_fn = Some(Arc::new(callback));
    }
}

/// Universal request structure for protocol-agnostic tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalRequest {
    /// Name of the tool to execute
    pub tool_name: String,
    /// Tool-specific parameters as JSON
    pub parameters: Value,
    /// User ID making the request
    pub user_id: String,
    /// Protocol identifier (e.g., "mcp", "a2a")
    pub protocol: String,
    /// Optional tenant ID for multi-tenant isolation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    /// Optional progress token for long-running operations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_token: Option<String>,
    /// Cancellation token (not serialized)
    #[serde(skip)]
    pub cancellation_token: Option<CancellationToken>,
    /// Progress reporter (not serialized)
    #[serde(skip)]
    pub progress_reporter: Option<ProgressReporter>,
}

/// Universal response structure for tool execution results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalResponse {
    /// Whether the tool execution succeeded
    pub success: bool,
    /// Tool execution result as JSON
    pub result: Option<Value>,
    /// Error message if execution failed
    pub error: Option<String>,
    /// Additional metadata about the execution
    pub metadata: Option<HashMap<String, Value>>,
}

/// Universal tool definition with handler function
#[derive(Debug, Clone)]
pub struct UniversalTool {
    /// Tool name identifier
    pub name: String,
    /// Human-readable tool description
    pub description: String,
    /// Handler function for tool execution
    pub handler:
        fn(&UniversalToolExecutor, UniversalRequest) -> Result<UniversalResponse, ProtocolError>,
}

/// Type alias for `UniversalExecutor` used in tool handler signatures
pub type UniversalToolExecutor = UniversalExecutor;
