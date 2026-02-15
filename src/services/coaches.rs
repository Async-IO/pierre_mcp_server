// ABOUTME: Coach business logic extracted from route handlers
// ABOUTME: Prerequisites checking, bulk assignment, rejection formatting, and provider display
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use std::collections::HashSet;
use std::hash::BuildHasher;

use crate::coaches::CoachPrerequisites;
use crate::database::coaches::CoachesManager;
use crate::database_plugins::DatabaseProvider;
use crate::errors::{AppError, AppResult};
use crate::models::TenantId;
use uuid::Uuid;

/// A missing prerequisite for a coach, protocol-agnostic
#[derive(Debug)]
pub struct MissingPrerequisite {
    /// Type of prerequisite (provider, `activity_count`, `activity_type`)
    pub prerequisite_type: String,
    /// The specific requirement (e.g., "strava", "50 activities", "Run")
    pub requirement: String,
    /// Human-readable message explaining what's missing
    pub message: String,
}

/// Result of a prerequisites check
#[derive(Debug)]
pub struct PrerequisiteCheckResult {
    /// Whether all prerequisites are met
    pub met: bool,
    /// List of missing prerequisites (empty if all met)
    pub missing: Vec<MissingPrerequisite>,
}

/// Check if prerequisites are met given user's connected providers
///
/// Validates that the user has connected all required fitness providers
/// specified in the coach's prerequisite configuration.
#[must_use]
pub fn check_prerequisites<S: BuildHasher>(
    prerequisites: &CoachPrerequisites,
    user_providers: &HashSet<String, S>,
) -> PrerequisiteCheckResult {
    let mut missing = Vec::new();

    for provider in &prerequisites.providers {
        let provider_lower = provider.to_lowercase();
        if !user_providers.contains(&provider_lower) {
            missing.push(MissingPrerequisite {
                prerequisite_type: "provider".to_owned(),
                requirement: provider.clone(),
                message: format!(
                    "Connect {} to unlock this coach",
                    capitalize_provider(provider)
                ),
            });
        }
    }

    let met = missing.is_empty();
    PrerequisiteCheckResult { met, missing }
}

/// Format a rejection reason by combining the base reason with optional notes
///
/// Returns just the reason if notes are empty or absent,
/// otherwise combines them as "reason: notes".
#[must_use]
pub fn format_rejection_reason(reason: &str, notes: Option<&str>) -> String {
    match notes {
        Some(n) if !n.trim().is_empty() => format!("{reason}: {}", n.trim()),
        _ => reason.to_owned(),
    }
}

/// Result of a bulk coach assignment operation
#[derive(Debug)]
pub struct BulkAssignmentResult {
    /// Number of users successfully assigned/unassigned
    pub affected_count: usize,
    /// Total number of users requested
    pub total_requested: usize,
}

/// Assign a coach to multiple users after verifying tenant membership
///
/// Each target user is validated to belong to the specified tenant before
/// the assignment is made. Fails on the first tenant membership violation.
///
/// # Errors
///
/// Returns error if any user ID is invalid, any user doesn't belong to the tenant,
/// or any database operation fails
pub async fn bulk_assign_coach<DB: DatabaseProvider>(
    manager: &CoachesManager,
    database: &DB,
    coach_id: &str,
    tenant_id: TenantId,
    admin_user_id: Uuid,
    user_ids: &[String],
) -> AppResult<BulkAssignmentResult> {
    let mut assigned_count = 0;

    for user_id_str in user_ids {
        let user_id = Uuid::parse_str(user_id_str)
            .map_err(|_| AppError::invalid_input(format!("Invalid user ID: {user_id_str}")))?;

        verify_tenant_membership(database, user_id, tenant_id).await?;

        if manager
            .assign_coach(coach_id, user_id, admin_user_id)
            .await?
        {
            assigned_count += 1;
        }
    }

    Ok(BulkAssignmentResult {
        affected_count: assigned_count,
        total_requested: user_ids.len(),
    })
}

/// Unassign a coach from multiple users after verifying tenant membership
///
/// Each target user is validated to belong to the specified tenant before
/// the unassignment. Fails on the first tenant membership violation.
///
/// # Errors
///
/// Returns error if any user ID is invalid, any user doesn't belong to the tenant,
/// or any database operation fails
pub async fn bulk_unassign_coach<DB: DatabaseProvider>(
    manager: &CoachesManager,
    database: &DB,
    coach_id: &str,
    tenant_id: TenantId,
    user_ids: &[String],
) -> AppResult<BulkAssignmentResult> {
    let mut removed_count = 0;

    for user_id_str in user_ids {
        let user_id = Uuid::parse_str(user_id_str)
            .map_err(|_| AppError::invalid_input(format!("Invalid user ID: {user_id_str}")))?;

        verify_tenant_membership(database, user_id, tenant_id).await?;

        if manager.unassign_coach(coach_id, user_id).await? {
            removed_count += 1;
        }
    }

    Ok(BulkAssignmentResult {
        affected_count: removed_count,
        total_requested: user_ids.len(),
    })
}

/// Verify that a user belongs to a specific tenant
///
/// # Errors
///
/// Returns error if the user is not a member of the specified tenant
async fn verify_tenant_membership<DB: DatabaseProvider>(
    database: &DB,
    user_id: Uuid,
    tenant_id: TenantId,
) -> AppResult<()> {
    let user_tenants = database.list_tenants_for_user(user_id).await.map_err(|e| {
        AppError::database(format!(
            "Failed to verify tenant membership for user {user_id}: {e}"
        ))
    })?;

    if !user_tenants.iter().any(|t| t.id == tenant_id) {
        return Err(AppError::auth_invalid(format!(
            "User {user_id} does not belong to this tenant"
        )));
    }

    Ok(())
}

/// Capitalize provider name for user-friendly display
#[must_use]
pub fn capitalize_provider(provider: &str) -> String {
    let provider_lower = provider.to_lowercase();
    match provider_lower.as_str() {
        "strava" => "Strava".to_owned(),
        "garmin" => "Garmin".to_owned(),
        "fitbit" => "Fitbit".to_owned(),
        "terra" => "Terra".to_owned(),
        _ => {
            let mut chars = provider.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + chars.as_str()
            })
        }
    }
}
