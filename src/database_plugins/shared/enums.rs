// ABOUTME: Enum conversion utilities for database operations
// ABOUTME: Eliminates duplicate enum ↔ string conversions across PostgreSQL and SQLite
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::a2a::protocol::TaskStatus;
use crate::constants::tiers;
use crate::models::{UserStatus, UserTier};

/// Convert `UserTier` enum to database string representation
///
/// # Examples
/// ```
/// use pierre_mcp_server::models::UserTier;
/// use pierre_mcp_server::database_plugins::shared::enums::user_tier_to_str;
///
/// assert_eq!(user_tier_to_str(&UserTier::Starter), "starter");
/// assert_eq!(user_tier_to_str(&UserTier::Professional), "professional");
/// assert_eq!(user_tier_to_str(&UserTier::Enterprise), "enterprise");
/// ```
#[must_use]
#[inline]
pub const fn user_tier_to_str(tier: &UserTier) -> &'static str {
    match tier {
        UserTier::Starter => tiers::STARTER,
        UserTier::Professional => tiers::PROFESSIONAL,
        UserTier::Enterprise => tiers::ENTERPRISE,
    }
}

/// Convert database string to `UserTier` enum
///
/// Unknown values default to `Starter` tier for safety.
///
/// # Examples
/// ```
/// use pierre_mcp_server::models::UserTier;
/// use pierre_mcp_server::database_plugins::shared::enums::str_to_user_tier;
///
/// assert_eq!(str_to_user_tier("professional"), UserTier::Professional);
/// assert_eq!(str_to_user_tier("unknown"), UserTier::Starter); // Default
/// ```
#[must_use]
pub fn str_to_user_tier(s: &str) -> UserTier {
    match s {
        tiers::PROFESSIONAL | "pro" => UserTier::Professional,
        tiers::ENTERPRISE => UserTier::Enterprise,
        _ => UserTier::Starter,
    }
}

/// Convert `UserStatus` enum to database string representation
///
/// # Examples
/// ```
/// use pierre_mcp_server::models::UserStatus;
/// use pierre_mcp_server::database_plugins::shared::enums::user_status_to_str;
///
/// assert_eq!(user_status_to_str(&UserStatus::Active), "active");
/// assert_eq!(user_status_to_str(&UserStatus::Pending), "pending");
/// assert_eq!(user_status_to_str(&UserStatus::Suspended), "suspended");
/// ```
#[must_use]
#[inline]
pub const fn user_status_to_str(status: &UserStatus) -> &'static str {
    match status {
        UserStatus::Active => "active",
        UserStatus::Pending => "pending",
        UserStatus::Suspended => "suspended",
    }
}

/// Convert database string to `UserStatus` enum
///
/// Unknown values default to `Active` for safety.
///
/// # Examples
/// ```
/// use pierre_mcp_server::models::UserStatus;
/// use pierre_mcp_server::database_plugins::shared::enums::str_to_user_status;
///
/// assert_eq!(str_to_user_status("pending"), UserStatus::Pending);
/// assert_eq!(str_to_user_status("suspended"), UserStatus::Suspended);
/// assert_eq!(str_to_user_status("unknown"), UserStatus::Active); // Default
/// ```
#[must_use]
pub fn str_to_user_status(s: &str) -> UserStatus {
    match s {
        "pending" => UserStatus::Pending,
        "suspended" => UserStatus::Suspended,
        _ => UserStatus::Active,
    }
}

/// Convert `TaskStatus` enum to database string representation
///
/// # Examples
/// ```
/// use pierre_mcp_server::a2a::protocol::TaskStatus;
/// use pierre_mcp_server::database_plugins::shared::enums::task_status_to_str;
///
/// assert_eq!(task_status_to_str(&TaskStatus::Pending), "pending");
/// assert_eq!(task_status_to_str(&TaskStatus::Running), "running");
/// assert_eq!(task_status_to_str(&TaskStatus::Completed), "completed");
/// assert_eq!(task_status_to_str(&TaskStatus::Failed), "failed");
/// assert_eq!(task_status_to_str(&TaskStatus::Cancelled), "cancelled");
/// ```
#[must_use]
#[inline]
pub const fn task_status_to_str(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "pending",
        TaskStatus::Running => "running",
        TaskStatus::Completed => "completed",
        TaskStatus::Failed => "failed",
        TaskStatus::Cancelled => "cancelled",
    }
}

/// Convert database string to `TaskStatus` enum
///
/// Unknown values default to `Pending` for safety.
///
/// # Examples
/// ```
/// use pierre_mcp_server::a2a::protocol::TaskStatus;
/// use pierre_mcp_server::database_plugins::shared::enums::str_to_task_status;
///
/// assert_eq!(str_to_task_status("running"), TaskStatus::Running);
/// assert_eq!(str_to_task_status("completed"), TaskStatus::Completed);
/// assert_eq!(str_to_task_status("unknown"), TaskStatus::Pending); // Default
/// ```
#[must_use]
pub fn str_to_task_status(s: &str) -> TaskStatus {
    match s {
        "running" => TaskStatus::Running,
        "completed" => TaskStatus::Completed,
        "failed" => TaskStatus::Failed,
        "cancelled" => TaskStatus::Cancelled,
        _ => TaskStatus::Pending,
    }
}
