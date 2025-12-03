// ABOUTME: Impersonation system for super admins to act as other users
// ABOUTME: Provides audit logging and session management for impersonation
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Impersonation session record for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpersonationSession {
    /// Unique session identifier
    pub id: String,
    /// Super admin performing the impersonation
    pub impersonator_id: Uuid,
    /// User being impersonated
    pub target_user_id: Uuid,
    /// Reason for impersonation (for audit)
    pub reason: Option<String>,
    /// When impersonation started
    pub started_at: DateTime<Utc>,
    /// When impersonation ended (None if still active)
    pub ended_at: Option<DateTime<Utc>>,
    /// Whether session is currently active
    pub is_active: bool,
    /// Record creation timestamp
    pub created_at: DateTime<Utc>,
}

impl ImpersonationSession {
    /// Create a new impersonation session
    #[must_use]
    pub fn new(impersonator_id: Uuid, target_user_id: Uuid, reason: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            impersonator_id,
            target_user_id,
            reason,
            started_at: now,
            ended_at: None,
            is_active: true,
            created_at: now,
        }
    }

    /// End the impersonation session
    pub fn end(&mut self) {
        self.ended_at = Some(Utc::now());
        self.is_active = false;
    }

    /// Get duration of impersonation in seconds
    #[must_use]
    pub fn duration_seconds(&self) -> i64 {
        let end = self.ended_at.unwrap_or_else(Utc::now);
        (end - self.started_at).num_seconds()
    }
}

/// JWT claims extension for impersonation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpersonationClaims {
    /// Original user ID (the super admin)
    pub original_user_id: Uuid,
    /// User being impersonated
    pub impersonated_user_id: Uuid,
    /// Impersonation session ID for audit
    pub session_id: String,
}

impl ImpersonationClaims {
    /// Create new impersonation claims
    #[must_use]
    pub const fn new(
        original_user_id: Uuid,
        impersonated_user_id: Uuid,
        session_id: String,
    ) -> Self {
        Self {
            original_user_id,
            impersonated_user_id,
            session_id,
        }
    }
}

/// Request to start impersonation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartImpersonationRequest {
    /// User ID to impersonate
    pub target_user_id: Uuid,
    /// Reason for impersonation (optional but recommended)
    pub reason: Option<String>,
}

/// Response after starting impersonation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpersonationResponse {
    /// New JWT token with impersonation claims
    pub token: String,
    /// Session ID for tracking
    pub session_id: String,
    /// Target user email for display
    pub target_user_email: String,
    /// Target user display name
    pub target_user_name: Option<String>,
}

/// Permission delegation record for session sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionDelegation {
    /// Unique delegation identifier
    pub id: String,
    /// User granting permissions
    pub grantor_id: Uuid,
    /// User receiving permissions
    pub grantee_id: Uuid,
    /// Delegated permissions (bitflags value)
    pub permissions: u64,
    /// When delegation expires (None = permanent until revoked)
    pub expires_at: Option<DateTime<Utc>>,
    /// When delegation was revoked (None if active)
    pub revoked_at: Option<DateTime<Utc>>,
    /// Record creation timestamp
    pub created_at: DateTime<Utc>,
}

impl PermissionDelegation {
    /// Create a new permission delegation
    #[must_use]
    pub fn new(
        grantor_id: Uuid,
        grantee_id: Uuid,
        permissions: u64,
        expires_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            grantor_id,
            grantee_id,
            permissions,
            expires_at,
            revoked_at: None,
            created_at: Utc::now(),
        }
    }

    /// Check if delegation is currently active
    #[must_use]
    pub fn is_active(&self) -> bool {
        if self.revoked_at.is_some() {
            return false;
        }
        self.expires_at.is_none_or(|exp| Utc::now() < exp)
    }

    /// Revoke the delegation
    pub fn revoke(&mut self) {
        self.revoked_at = Some(Utc::now());
    }
}
