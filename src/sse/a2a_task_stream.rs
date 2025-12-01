// ABOUTME: SSE stream implementation for A2A task progress updates
// ABOUTME: Provides real-time task status changes and completion notifications
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use tokio::sync::broadcast;

/// SSE stream for A2A task status updates
#[derive(Clone)]
pub struct A2ATaskStream {
    sender: broadcast::Sender<String>,
}

impl A2ATaskStream {
    /// Create a new A2A task stream with the specified buffer size
    #[must_use]
    pub fn new(buffer_size: usize) -> Self {
        let (sender, _) = broadcast::channel(buffer_size);
        Self { sender }
    }

    /// Subscribe to the task stream
    ///
    /// Returns a receiver that will get all task status updates
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.sender.subscribe()
    }

    /// Send a task update event
    ///
    /// # Errors
    ///
    /// Returns error if no active subscribers (all receivers dropped)
    pub fn send_update(
        &self,
        event_data: String,
    ) -> Result<usize, broadcast::error::SendError<String>> {
        self.sender.send(event_data)
    }

    /// Get count of active subscribers
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}
