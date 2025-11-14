// ABOUTME: Enum conversion utilities for database operations
// ABOUTME: Eliminates duplicate enum ↔ string conversions across PostgreSQL and SQLite
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_tier_round_trip() {
        let tiers = vec![UserTier::Starter, UserTier::Professional, UserTier::Enterprise];
        for tier in tiers {
            let s = user_tier_to_str(&tier);
            let parsed = str_to_user_tier(s);
            assert_eq!(parsed, tier, "Round-trip failed for tier: {tier:?}");
        }
    }

    #[test]
    fn test_user_tier_unknown() {
        assert_eq!(str_to_user_tier("unknown"), UserTier::Starter);
        assert_eq!(str_to_user_tier(""), UserTier::Starter);
    }

    #[test]
    fn test_user_tier_pro_alias() {
        // "pro" should map to Professional
        assert_eq!(str_to_user_tier("pro"), UserTier::Professional);
    }

    #[test]
    fn test_user_status_round_trip() {
        let statuses = vec![UserStatus::Active, UserStatus::Pending, UserStatus::Suspended];
        for status in statuses {
            let s = user_status_to_str(&status);
            let parsed = str_to_user_status(s);
            assert_eq!(parsed, status, "Round-trip failed for status: {status:?}");
        }
    }

    #[test]
    fn test_user_status_unknown() {
        assert_eq!(str_to_user_status("unknown"), UserStatus::Active);
        assert_eq!(str_to_user_status(""), UserStatus::Active);
    }

    #[test]
    fn test_task_status_round_trip() {
        let statuses = vec![
            TaskStatus::Pending,
            TaskStatus::Running,
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Cancelled,
        ];
        for status in statuses {
            let s = task_status_to_str(&status);
            let parsed = str_to_task_status(s);
            assert_eq!(parsed, status, "Round-trip failed for task status: {status:?}");
        }
    }

    #[test]
    fn test_task_status_unknown() {
        assert_eq!(str_to_task_status("unknown"), TaskStatus::Pending);
        assert_eq!(str_to_task_status(""), TaskStatus::Pending);
    }
}
