// ABOUTME: Defines ToolResult and ToolNotification types for tool execution responses.
// ABOUTME: Provides structured result handling with support for side-effect notifications.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Tool Result Types
//!
//! Defines the result types returned by tool execution:
//! - `ToolResult` - The main result containing content and notifications
//! - `ToolNotification` - Side-effect notifications (OAuth events, progress, etc.)
//!
//! These types bridge tool implementations with the MCP protocol response format.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Result returned by tool execution.
///
/// Contains the tool's output content and optional notifications for
/// side effects like OAuth completion or progress updates.
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// The result value to return to the client
    pub content: Value,
    /// Optional notifications to append (e.g., OAuth refresh notifications)
    pub notifications: Vec<ToolNotification>,
    /// Whether this result represents an error condition
    pub is_error: bool,
}

impl ToolResult {
    /// Create a simple successful result with just content
    #[must_use]
    pub const fn ok(content: Value) -> Self {
        Self {
            content,
            notifications: Vec::new(),
            is_error: false,
        }
    }

    /// Create an error result
    #[must_use]
    pub const fn error(content: Value) -> Self {
        Self {
            content,
            notifications: Vec::new(),
            is_error: true,
        }
    }

    /// Create a result with notifications
    #[must_use]
    pub const fn with_notifications(content: Value, notifications: Vec<ToolNotification>) -> Self {
        Self {
            content,
            notifications,
            is_error: false,
        }
    }

    /// Add a notification to this result
    #[must_use]
    pub fn add_notification(mut self, notification: ToolNotification) -> Self {
        self.notifications.push(notification);
        self
    }

    /// Create a result from a serializable value
    ///
    /// # Errors
    ///
    /// Returns the serialization error if the value cannot be converted to JSON
    pub fn from_serializable<T: Serialize>(value: &T) -> Result<Self, serde_json::Error> {
        Ok(Self::ok(serde_json::to_value(value)?))
    }

    /// Create a text result (convenience method)
    #[must_use]
    pub fn text(message: impl Into<String>) -> Self {
        Self::ok(Value::String(message.into()))
    }

    /// Check if this result has any notifications
    #[must_use]
    pub const fn has_notifications(&self) -> bool {
        !self.notifications.is_empty()
    }
}

impl Default for ToolResult {
    fn default() -> Self {
        Self::ok(Value::Null)
    }
}

/// Notification to send alongside tool result.
///
/// Notifications inform clients of side effects that occurred during
/// tool execution, such as:
/// - OAuth token refresh events
/// - Progress updates for long-running operations
/// - Cache invalidation hints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolNotification {
    /// Type of notification (e.g., `oauth_refresh`, `progress`)
    pub notification_type: NotificationType,
    /// Notification payload data
    pub data: Value,
}

/// Types of notifications that tools can emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    /// OAuth token was refreshed during execution
    OAuthRefresh,
    /// OAuth flow completed
    OAuthCompleted,
    /// Progress update for long-running operations
    Progress,
    /// Cache was invalidated
    CacheInvalidation,
    /// Tool list changed (for dynamic tool registration)
    ToolListChanged,
    /// Resource updated
    ResourceUpdated,
}

impl NotificationType {
    /// Get the MCP method name for this notification type
    #[must_use]
    pub const fn method_name(&self) -> &'static str {
        match self {
            Self::OAuthRefresh => "notifications/oauth/refresh",
            Self::OAuthCompleted => "notifications/oauth/completed",
            Self::Progress => "notifications/progress",
            Self::CacheInvalidation => "notifications/cache/invalidated",
            Self::ToolListChanged => "notifications/tools/list_changed",
            Self::ResourceUpdated => "notifications/resources/updated",
        }
    }
}

impl ToolNotification {
    /// Create a new notification
    #[must_use]
    pub const fn new(notification_type: NotificationType, data: Value) -> Self {
        Self {
            notification_type,
            data,
        }
    }

    /// Create an OAuth refresh notification
    #[must_use]
    pub fn oauth_refresh(provider: &str) -> Self {
        Self::new(
            NotificationType::OAuthRefresh,
            serde_json::json!({
                "provider": provider,
                "refreshed": true
            }),
        )
    }

    /// Create a progress notification
    #[must_use]
    pub fn progress(token: &str, current: f64, total: Option<f64>, message: Option<&str>) -> Self {
        let mut data = serde_json::json!({
            "progressToken": token,
            "progress": current,
        });

        if let Some(t) = total {
            data["total"] = serde_json::json!(t);
        }

        if let Some(msg) = message {
            data["message"] = serde_json::json!(msg);
        }

        Self::new(NotificationType::Progress, data)
    }

    /// Create a cache invalidation notification
    #[must_use]
    pub fn cache_invalidation(pattern: &str) -> Self {
        Self::new(
            NotificationType::CacheInvalidation,
            serde_json::json!({
                "pattern": pattern
            }),
        )
    }
}
