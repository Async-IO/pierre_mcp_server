// ABOUTME: Extension context for dependency injection of plugin and protocol services
// ABOUTME: Contains plugin executor, sampling peer, and progress notification channels for MCP extensions
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::mcp::sampling_peer::SamplingPeer;
use crate::mcp::schema::ProgressNotification;
use crate::plugins::executor::PluginToolExecutor;
use crate::protocols::universal::types::CancellationToken;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Extension context containing plugin and protocol dependencies
///
/// This context provides all extension-related dependencies needed for
/// plugin execution, server-initiated LLM requests, and progress tracking.
///
/// # Dependencies
/// - `plugin_executor`: Optional plugin executor for custom tool implementations
/// - `sampling_peer`: Optional sampling peer for server-initiated LLM requests (stdio transport only)
/// - `progress_notification_sender`: Optional channel for progress notifications (stdio transport only)
/// - `cancellation_registry`: Registry mapping progress tokens to cancellation tokens
#[derive(Clone)]
pub struct ExtensionContext {
    plugin_executor: Option<Arc<PluginToolExecutor>>,
    sampling_peer: Option<Arc<SamplingPeer>>,
    progress_notification_sender: Option<mpsc::UnboundedSender<ProgressNotification>>,
    cancellation_registry: Arc<RwLock<HashMap<String, CancellationToken>>>,
}

impl ExtensionContext {
    /// Create new extension context
    #[must_use]
    pub const fn new(
        plugin_executor: Option<Arc<PluginToolExecutor>>,
        sampling_peer: Option<Arc<SamplingPeer>>,
        progress_notification_sender: Option<mpsc::UnboundedSender<ProgressNotification>>,
        cancellation_registry: Arc<RwLock<HashMap<String, CancellationToken>>>,
    ) -> Self {
        Self {
            plugin_executor,
            sampling_peer,
            progress_notification_sender,
            cancellation_registry,
        }
    }

    /// Get plugin executor for custom tool implementations
    #[must_use]
    pub const fn plugin_executor(&self) -> &Option<Arc<PluginToolExecutor>> {
        &self.plugin_executor
    }

    /// Get sampling peer for server-initiated LLM requests
    #[must_use]
    pub const fn sampling_peer(&self) -> &Option<Arc<SamplingPeer>> {
        &self.sampling_peer
    }

    /// Get progress notification sender for progress updates
    #[must_use]
    pub const fn progress_notification_sender(
        &self,
    ) -> &Option<mpsc::UnboundedSender<ProgressNotification>> {
        &self.progress_notification_sender
    }

    /// Get cancellation registry for progress token -> cancellation token mapping
    #[must_use]
    pub const fn cancellation_registry(&self) -> &Arc<RwLock<HashMap<String, CancellationToken>>> {
        &self.cancellation_registry
    }

    /// Register a cancellation token for a progress token
    pub async fn register_cancellation_token(
        &self,
        progress_token: String,
        cancellation_token: CancellationToken,
    ) {
        let mut registry = self.cancellation_registry.write().await;
        registry.insert(progress_token, cancellation_token);
    }

    /// Cancel an operation by progress token
    pub async fn cancel_by_progress_token(&self, progress_token: &str) {
        let registry = self.cancellation_registry.read().await;
        if let Some(token) = registry.get(progress_token) {
            token.cancel().await;
        }
    }

    /// Cleanup a cancellation token after operation completes
    pub async fn cleanup_cancellation_token(&self, progress_token: &str) {
        let mut registry = self.cancellation_registry.write().await;
        registry.remove(progress_token);
    }
}
