// ABOUTME: Progress tracking utilities for MCP long-running operations
// ABOUTME: Manages progress notifications, tracking tokens, and operation status reporting
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::mcp::schema::ProgressNotification;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, warn};
use uuid::Uuid;

/// Progress tracker for managing long-running operations
#[derive(Debug)]
pub struct ProgressTracker {
    /// Active progress tokens and their current status
    progress_map: Arc<RwLock<HashMap<String, ProgressState>>>,
    /// Channel for sending progress notifications
    notification_sender: Option<mpsc::UnboundedSender<ProgressNotification>>,
}

/// Internal progress state for an operation
#[derive(Debug, Clone)]
struct ProgressState {
    pub operation_name: String,
    pub current: f64,
    pub total: Option<f64>,
    pub message: Option<String>,
    pub completed: bool,
    pub cancelled: bool,
}

impl ProgressTracker {
    /// Create a new progress tracker
    #[must_use]
    pub fn new() -> Self {
        Self {
            progress_map: Arc::new(RwLock::new(HashMap::new())),
            notification_sender: None,
        }
    }

    /// Create a progress tracker with notification channel
    #[must_use]
    pub fn with_notifications(sender: mpsc::UnboundedSender<ProgressNotification>) -> Self {
        Self {
            progress_map: Arc::new(RwLock::new(HashMap::new())),
            notification_sender: Some(sender),
        }
    }

    /// Start tracking progress for a new operation
    pub async fn start_operation(&self, operation_name: &str, total: Option<f64>) -> String {
        let progress_token = Uuid::new_v4().to_string();

        let state = ProgressState {
            operation_name: operation_name.to_owned(),
            current: 0.0,
            total,
            message: Some(format!("Starting {operation_name}...")),
            completed: false,
            cancelled: false,
        };

        {
            let mut progress_map = self.progress_map.write().await;
            progress_map.insert(progress_token.clone(), state);
        }

        debug!(
            "Started tracking progress for operation: {} (token: {})",
            operation_name, progress_token
        );

        // Send initial progress notification
        if let Some(sender) = &self.notification_sender {
            let notification = ProgressNotification::new(
                progress_token.clone(),
                0.0,
                total,
                Some(format!("Started {operation_name}")),
            );
            if let Err(e) = sender.send(notification) {
                warn!("Failed to send progress notification: {}", e);
            }
        }

        progress_token
    }

    /// Update progress for an ongoing operation
    ///
    /// # Errors
    ///
    /// Returns an error if the progress token is not found or operation is already completed
    pub async fn update_progress(
        &self,
        progress_token: &str,
        current: f64,
        message: Option<String>,
    ) -> Result<(), String> {
        let mut progress_map = self.progress_map.write().await;

        if let Some(state) = progress_map.get_mut(progress_token) {
            if state.completed {
                return Err("Operation already completed".to_owned());
            }
            if state.cancelled {
                return Err("Operation was cancelled".to_owned());
            }

            state.current = current;
            if let Some(ref msg) = message {
                state.message = Some(msg.clone()); // Safe: String ownership required for state storage
            }

            debug!(
                "Updated progress for {}: {} / {:?} ({}%)",
                state.operation_name,
                current,
                state.total,
                state.total.map_or(current, |total| {
                    if total > 0.0 {
                        (current / total * 100.0).min(100.0)
                    } else {
                        0.0
                    }
                })
            );

            // Send progress notification
            if let Some(sender) = &self.notification_sender {
                let notification = ProgressNotification::new(
                    progress_token.to_owned(),
                    current,
                    state.total,
                    message,
                );
                if let Err(e) = sender.send(notification) {
                    warn!("Failed to send progress notification: {}", e);
                }
            }

            Ok(())
        } else {
            Err(format!("Progress token not found: {progress_token}"))
        }
    }

    /// Mark an operation as completed
    ///
    /// # Errors
    ///
    /// Returns an error if the progress token is not found
    pub async fn complete_operation(
        &self,
        progress_token: &str,
        final_message: Option<String>,
    ) -> Result<(), String> {
        let mut progress_map = self.progress_map.write().await;

        if let Some(state) = progress_map.get_mut(progress_token) {
            state.completed = true;
            state.current = state.total.unwrap_or(100.0);
            if let Some(ref msg) = final_message {
                state.message = Some(msg.clone()); // Safe: String ownership required for state storage
            }

            debug!(
                "Completed operation: {} (token: {})",
                state.operation_name, progress_token
            );

            // Send completion notification
            if let Some(sender) = &self.notification_sender {
                let notification = ProgressNotification::new(
                    progress_token.to_owned(),
                    state.current,
                    state.total,
                    final_message.or_else(|| Some(format!("Completed {}", state.operation_name))),
                );
                if let Err(e) = sender.send(notification) {
                    warn!("Failed to send completion notification: {}", e);
                }
            }

            Ok(())
        } else {
            Err(format!("Progress token not found: {progress_token}"))
        }
    }

    /// Cancel an ongoing operation
    ///
    /// # Errors
    ///
    /// Returns an error if the progress token is not found or operation is already completed
    pub async fn cancel_operation(
        &self,
        progress_token: &str,
        cancellation_message: Option<String>,
    ) -> Result<(), String> {
        let mut progress_map = self.progress_map.write().await;

        if let Some(state) = progress_map.get_mut(progress_token) {
            if state.completed {
                return Err("Cannot cancel completed operation".to_owned());
            }
            if state.cancelled {
                return Err("Operation already cancelled".to_owned());
            }

            state.cancelled = true;
            let final_message = cancellation_message
                .unwrap_or_else(|| format!("Operation '{}' was cancelled", state.operation_name));
            state.message = Some(final_message.clone());

            debug!(
                "Cancelled operation: {} (token: {})",
                state.operation_name, progress_token
            );

            // Send cancellation notification
            if let Some(sender) = &self.notification_sender {
                let notification = ProgressNotification::new(
                    progress_token.to_owned(),
                    state.current,
                    state.total,
                    Some(final_message),
                );
                if let Err(e) = sender.send(notification) {
                    warn!("Failed to send cancellation notification: {}", e);
                }
            }

            Ok(())
        } else {
            Err(format!("Progress token not found: {progress_token}"))
        }
    }

    /// Get current progress for an operation
    pub async fn get_progress(
        &self,
        progress_token: &str,
    ) -> Option<(f64, Option<f64>, bool, bool)> {
        let progress_map = self.progress_map.read().await;
        progress_map
            .get(progress_token)
            .map(|state| (state.current, state.total, state.completed, state.cancelled))
    }

    /// Check if an operation is cancelled
    pub async fn is_cancelled(&self, progress_token: &str) -> bool {
        let progress_map = self.progress_map.read().await;
        progress_map
            .get(progress_token)
            .is_some_and(|state| state.cancelled)
    }

    /// Clean up completed or cancelled operations
    pub async fn cleanup_completed(&self) {
        let mut progress_map = self.progress_map.write().await;
        let initial_count = progress_map.len();

        progress_map.retain(|_, state| !state.completed && !state.cancelled);

        let cleaned_count = initial_count - progress_map.len();
        drop(progress_map);
        if cleaned_count > 0 {
            debug!(
                "Cleaned up {} completed/cancelled progress operations",
                cleaned_count
            );
        }
    }

    /// Get all active operations (for debugging)
    pub async fn get_active_operations(&self) -> Vec<String> {
        let progress_map = self.progress_map.read().await;
        progress_map.keys().cloned().collect()
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper macro for updating progress in tool handlers
#[macro_export]
macro_rules! update_progress {
    ($tracker:expr, $token:expr, $current:expr, $message:expr) => {
        if let Some(tracker) = $tracker {
            if let Err(e) = tracker
                .update_progress($token, $current, Some($message.to_string()))
                .await
            {
                tracing::warn!("Failed to update progress: {}", e);
            }
        }
    };
}

/// Helper macro for completing operations
#[macro_export]
macro_rules! complete_operation {
    ($tracker:expr, $token:expr, $message:expr) => {
        if let Some(tracker) = $tracker {
            if let Err(e) = tracker
                .complete_operation($token, Some($message.to_string()))
                .await
            {
                tracing::warn!("Failed to complete operation: {}", e);
            }
        }
    };
}

/// Helper macro for cancelling operations
#[macro_export]
macro_rules! cancel_operation {
    ($tracker:expr, $token:expr, $message:expr) => {
        if let Some(tracker) = $tracker {
            if let Err(e) = tracker
                .cancel_operation($token, Some($message.to_string()))
                .await
            {
                tracing::warn!("Failed to cancel operation: {}", e);
            }
        }
    };
}

/// Helper macro for checking cancellation status
#[macro_export]
macro_rules! check_cancelled {
    ($tracker:expr, $token:expr) => {
        if let Some(tracker) = $tracker {
            if tracker.is_cancelled($token).await {
                return Err("Operation was cancelled".to_owned().into());
            }
        }
    };
}
