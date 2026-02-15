// ABOUTME: Key rotation configuration and version tracking types
// ABOUTME: DTOs for key lifecycle management and rotation status tracking
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::TenantId;

/// Key rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRotationConfig {
    /// How often to rotate keys (in days)
    pub rotation_interval_days: u32,
    /// Maximum age of a key before forced rotation (in days)
    pub max_key_age_days: u32,
    /// Whether to enable automatic rotation
    pub auto_rotation_enabled: bool,
    /// Hour of day to perform rotations (0-23)
    pub rotation_hour: u8,
    /// Number of old key versions to retain
    pub key_versions_to_retain: u32,
}

impl Default for KeyRotationConfig {
    fn default() -> Self {
        Self {
            rotation_interval_days: 90, // Rotate every 90 days
            max_key_age_days: 365,      // Maximum 1 year
            auto_rotation_enabled: true,
            rotation_hour: 2,          // 2 AM UTC
            key_versions_to_retain: 3, // Keep last 3 versions
        }
    }
}

/// Key version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyVersion {
    /// Version number (incremental)
    pub version: u32,
    /// When this key version was created
    pub created_at: DateTime<Utc>,
    /// When this key version expires
    pub expires_at: DateTime<Utc>,
    /// Whether this version is currently active
    pub is_active: bool,
    /// Tenant ID (None for global keys)
    pub tenant_id: Option<TenantId>,
    /// Algorithm used for this key
    pub algorithm: String,
}

/// Key rotation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RotationStatus {
    /// No rotation needed
    Current,
    /// Rotation scheduled
    Scheduled {
        /// When the rotation is scheduled to occur
        scheduled_at: DateTime<Utc>,
    },
    /// Rotation in progress
    InProgress {
        /// When the rotation started
        started_at: DateTime<Utc>,
    },
    /// Rotation completed
    Completed {
        /// When the rotation completed
        completed_at: DateTime<Utc>,
    },
    /// Rotation failed
    Failed {
        /// When the rotation failed
        failed_at: DateTime<Utc>,
        /// Error message describing the failure
        error: String,
    },
}
