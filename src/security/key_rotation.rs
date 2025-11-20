// ABOUTME: Key rotation mechanisms for enhanced security and compliance
// ABOUTME: Provides automated key rotation, version management, and seamless key transitions
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! # Key Rotation Module
//!
//! Automated key rotation for enhanced security including:
//! - Scheduled key rotation for tenants
//! - Seamless key version transitions
//! - Emergency key rotation procedures
//! - Key lifecycle management

use crate::database_plugins::DatabaseProvider;
use crate::errors::AppResult;
use chrono::Timelike;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::time::Duration;
use uuid::Uuid;

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
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When this key version expires
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// Whether this version is currently active
    pub is_active: bool,
    /// Tenant ID (None for global keys)
    pub tenant_id: Option<Uuid>,
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
        scheduled_at: chrono::DateTime<chrono::Utc>,
    },
    /// Rotation in progress
    InProgress {
        /// When the rotation started
        started_at: chrono::DateTime<chrono::Utc>,
    },
    /// Rotation completed
    Completed {
        /// When the rotation completed
        completed_at: chrono::DateTime<chrono::Utc>,
    },
    /// Rotation failed
    Failed {
        /// When the rotation failed
        failed_at: chrono::DateTime<chrono::Utc>,
        /// Error message describing the failure
        error: String,
    },
}

/// Key rotation manager
pub struct KeyRotationManager {
    /// Encryption manager for performing key operations
    encryption_manager: Arc<super::TenantEncryptionManager>,
    /// Database for storing key metadata
    database: Arc<crate::database_plugins::factory::Database>,
    /// Audit logger
    auditor: Arc<super::audit::SecurityAuditor>,
    /// Rotation configuration
    config: KeyRotationConfig,
    /// Key version tracking
    key_versions: tokio::sync::RwLock<HashMap<Option<Uuid>, Vec<KeyVersion>>>,
    /// Rotation status tracking
    rotation_status: tokio::sync::RwLock<HashMap<Option<Uuid>, RotationStatus>>,
}

impl KeyRotationManager {
    /// Create new key rotation manager
    #[must_use]
    pub fn new(
        encryption_manager: Arc<super::TenantEncryptionManager>,
        database: Arc<crate::database_plugins::factory::Database>,
        auditor: Arc<super::audit::SecurityAuditor>,
        config: KeyRotationConfig,
    ) -> Self {
        Self {
            encryption_manager,
            database,
            auditor,
            config,
            key_versions: tokio::sync::RwLock::new(HashMap::new()),
            rotation_status: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Start the key rotation scheduler
    ///
    /// # Errors
    ///
    /// Returns an error if the scheduler cannot be started
    pub fn start_scheduler(self: Arc<Self>) -> AppResult<()> {
        if !self.config.auto_rotation_enabled {
            tracing::info!("Key rotation scheduler disabled");
            return Ok(());
        }

        tracing::info!(
            "Starting key rotation scheduler - checking every {} days at {}:00 UTC",
            self.config.rotation_interval_days,
            self.config.rotation_hour
        );

        let manager = Arc::clone(&self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(
                crate::constants::time::HOUR_SECONDS as u64,
            )); // Check every hour

            loop {
                interval.tick().await;

                let now = chrono::Utc::now();
                if u8::try_from(now.hour()).unwrap_or(0) == manager.config.rotation_hour {
                    if let Err(e) = manager.check_and_rotate_keys().await {
                        tracing::error!("Key rotation check failed: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Check all tenants and rotate keys as needed
    async fn check_and_rotate_keys(&self) -> AppResult<()> {
        tracing::info!("Checking for keys that need rotation");

        // Get all tenants from database
        let tenants = self.database.get_all_tenants().await?;

        // Check global keys first
        self.check_key_rotation(None).await?;

        // Check each tenant's keys
        for tenant in tenants {
            if let Err(e) = self.check_key_rotation(Some(tenant.id)).await {
                tracing::error!(
                    "Failed to check key rotation for tenant {}: {}",
                    tenant.id,
                    e
                );
            }
        }

        Ok(())
    }

    /// Check if a specific tenant/global key needs rotation
    async fn check_key_rotation(&self, tenant_id: Option<Uuid>) -> AppResult<()> {
        let current_version = self.get_current_key_version(tenant_id).await?;

        if let Some(version) = current_version {
            let age_days = (chrono::Utc::now() - version.created_at).num_days();

            if age_days >= i64::from(self.config.rotation_interval_days) {
                tracing::info!(
                    "Key for tenant {:?} is {} days old, scheduling rotation",
                    tenant_id,
                    age_days
                );
                self.schedule_key_rotation(tenant_id).await?;
            }
        } else {
            // No key version found, create initial version
            tracing::info!(
                "No key version found for tenant {:?}, creating initial version",
                tenant_id
            );
            self.initialize_key_version(tenant_id).await?;
        }

        Ok(())
    }

    /// Schedule a key rotation
    async fn schedule_key_rotation(&self, tenant_id: Option<Uuid>) -> AppResult<()> {
        let scheduled_at = chrono::Utc::now() + chrono::Duration::hours(1); // Schedule for 1 hour from now

        {
            let mut status = self.rotation_status.write().await;
            status.insert(tenant_id, RotationStatus::Scheduled { scheduled_at });
        }

        // Log audit event
        let event = super::audit::AuditEvent::new(
            super::audit::AuditEventType::KeyRotated,
            super::audit::AuditSeverity::Info,
            format!("Key rotation scheduled for tenant {tenant_id:?}"),
            "schedule_rotation".to_owned(),
            "success".to_owned(),
        );

        let event = if let Some(tid) = tenant_id {
            event.with_tenant_id(tid)
        } else {
            event
        };

        if let Err(e) = self.auditor.log_event(event).await {
            tracing::error!("Failed to log key rotation audit event: {}", e);
        }

        // Perform the rotation
        self.perform_key_rotation(tenant_id).await?;

        Ok(())
    }

    /// Perform actual key rotation
    async fn perform_key_rotation(&self, tenant_id: Option<Uuid>) -> AppResult<()> {
        tracing::info!("Starting key rotation for tenant {:?}", tenant_id);

        {
            let mut status = self.rotation_status.write().await;
            status.insert(
                tenant_id,
                RotationStatus::InProgress {
                    started_at: chrono::Utc::now(),
                },
            );
        }

        let result = self.execute_key_rotation(tenant_id).await;

        match &result {
            Ok(()) => {
                self.rotation_status.write().await.insert(
                    tenant_id,
                    RotationStatus::Completed {
                        completed_at: chrono::Utc::now(),
                    },
                );

                tracing::info!(
                    "Key rotation completed successfully for tenant {:?}",
                    tenant_id
                );
            }
            Err(e) => {
                self.rotation_status.write().await.insert(
                    tenant_id,
                    RotationStatus::Failed {
                        failed_at: chrono::Utc::now(),
                        error: e.to_string(),
                    },
                );

                tracing::error!("Key rotation failed for tenant {:?}: {}", tenant_id, e);
            }
        }

        result
    }

    /// Execute the actual key rotation process
    async fn execute_key_rotation(&self, tenant_id: Option<Uuid>) -> AppResult<()> {
        // 1. Create new key version
        let new_version = self.create_new_key_version(tenant_id).await?;

        // 2. Re-encrypt existing data with new key (this would be a complex process)
        // For now, we'll just mark the new version as active
        // In a real implementation, this would involve:
        // - Reading all encrypted data for this tenant
        // - Decrypting with old key
        // - Re-encrypting with new key
        // - Updating database records

        // 3. Rotate the key in the encryption manager
        if let Some(tid) = tenant_id {
            self.encryption_manager.rotate_tenant_key(tid).await?;
        }

        // 4. Update key version status
        self.activate_key_version(tenant_id, new_version.version)
            .await?;

        // 5. Clean up old key versions
        self.cleanup_old_key_versions(tenant_id).await?;

        Ok(())
    }

    /// Create a new key version
    async fn create_new_key_version(&self, tenant_id: Option<Uuid>) -> AppResult<KeyVersion> {
        let current_versions = self.get_key_versions(tenant_id).await?;
        let next_version = current_versions
            .iter()
            .map(|v| v.version)
            .max()
            .unwrap_or(0)
            + 1;

        let new_version = KeyVersion {
            version: next_version,
            created_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now()
                + chrono::Duration::days(i64::from(self.config.max_key_age_days)),
            is_active: false, // Will be activated after rotation
            tenant_id,
            algorithm: "AES-256-GCM".to_owned(),
        };

        // Store in database
        self.store_key_version(&new_version)?;

        // Update in-memory cache
        {
            let mut versions = self.key_versions.write().await;
            versions
                .entry(tenant_id)
                .or_default()
                .push(new_version.clone());
        }

        Ok(new_version)
    }

    /// Activate a specific key version
    async fn activate_key_version(&self, tenant_id: Option<Uuid>, version: u32) -> AppResult<()> {
        // Update database first
        self.database
            .update_key_version_status(tenant_id, version, true)
            .await?;

        // Update in-memory cache
        if let Some(tenant_versions) = self.key_versions.write().await.get_mut(&tenant_id) {
            // Deactivate all versions
            for v in tenant_versions.iter_mut() {
                v.is_active = false;
            }

            // Activate the specified version
            if let Some(v) = tenant_versions.iter_mut().find(|v| v.version == version) {
                v.is_active = true;
            }
        }

        Ok(())
    }

    /// Clean up old key versions
    async fn cleanup_old_key_versions(&self, tenant_id: Option<Uuid>) -> AppResult<()> {
        // Delete old key versions from database
        let deleted_count = self
            .database
            .delete_old_key_versions(tenant_id, self.config.key_versions_to_retain)
            .await?;

        if deleted_count > 0 {
            tracing::info!(
                "Cleaned up {} old key versions for tenant {:?}",
                deleted_count,
                tenant_id
            );

            // Update in-memory cache by reloading from database
            let updated_versions = self.database.get_key_versions(tenant_id).await?;
            {
                let mut cache = self.key_versions.write().await;
                cache.insert(tenant_id, updated_versions);
            }
        }

        Ok(())
    }

    /// Initialize key version for new tenant
    async fn initialize_key_version(&self, tenant_id: Option<Uuid>) -> AppResult<()> {
        let initial_version = KeyVersion {
            version: 1,
            created_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now()
                + chrono::Duration::days(i64::from(self.config.max_key_age_days)),
            is_active: true,
            tenant_id,
            algorithm: "AES-256-GCM".to_owned(),
        };

        self.store_key_version(&initial_version)?;

        {
            let mut versions = self.key_versions.write().await;
            versions.entry(tenant_id).or_default().push(initial_version);
        }

        Ok(())
    }

    /// Get current active key version
    async fn get_current_key_version(
        &self,
        tenant_id: Option<Uuid>,
    ) -> AppResult<Option<KeyVersion>> {
        let versions = self.get_key_versions(tenant_id).await?;
        Ok(versions.into_iter().find(|v| v.is_active))
    }

    /// Get all key versions for a tenant
    async fn get_key_versions(&self, tenant_id: Option<Uuid>) -> AppResult<Vec<KeyVersion>> {
        // First try to get from database
        if let Ok(versions) = self.database.get_key_versions(tenant_id).await {
            // Update in-memory cache
            {
                let mut cache = self.key_versions.write().await;
                cache.insert(tenant_id, versions.clone());
            }
            Ok(versions)
        } else {
            // Fallback to cache if database fails
            let versions = self.key_versions.read().await;
            Ok(versions.get(&tenant_id).cloned().unwrap_or_default())
        }
    }

    /// Store key version in database
    fn store_key_version(&self, version: &KeyVersion) -> AppResult<()> {
        // Use async runtime to call the database method
        let rt = tokio::runtime::Handle::current();
        rt.block_on(self.database.store_key_version(version))
    }

    /// Get rotation status for a tenant
    pub async fn get_rotation_status(&self, tenant_id: Option<Uuid>) -> RotationStatus {
        let status = self.rotation_status.read().await;
        status
            .get(&tenant_id)
            .cloned()
            .unwrap_or(RotationStatus::Current)
    }

    /// Force immediate key rotation (for emergency scenarios)
    ///
    /// # Errors
    ///
    /// Returns an error if emergency rotation fails
    pub async fn emergency_key_rotation(
        &self,
        tenant_id: Option<Uuid>,
        reason: &str,
    ) -> AppResult<()> {
        tracing::warn!(
            "Emergency key rotation initiated for tenant {:?}. Reason: {}",
            tenant_id,
            reason
        );

        // Log critical audit event
        let event = super::audit::AuditEvent::new(
            super::audit::AuditEventType::KeyRotated,
            super::audit::AuditSeverity::Critical,
            format!("Emergency key rotation: {reason}"),
            "emergency_rotation".to_owned(),
            "initiated".to_owned(),
        );

        let event = if let Some(tid) = tenant_id {
            event.with_tenant_id(tid)
        } else {
            event
        };

        if let Err(e) = self.auditor.log_event(event).await {
            tracing::error!("Failed to log emergency key rotation audit: {}", e);
        }

        // Perform immediate rotation
        self.perform_key_rotation(tenant_id).await?;

        tracing::info!(
            "Emergency key rotation completed for tenant {:?}",
            tenant_id
        );
        Ok(())
    }

    /// Get key rotation statistics
    pub async fn get_rotation_stats(&self) -> KeyRotationStats {
        let total_tenants = self.key_versions.read().await.len();
        let status = self.rotation_status.read().await;
        let active_rotations = status
            .values()
            .filter(|s| matches!(s, RotationStatus::InProgress { .. }))
            .count();
        let failed_rotations = status
            .values()
            .filter(|s| matches!(s, RotationStatus::Failed { .. }))
            .count();
        drop(status);

        KeyRotationStats {
            total_tenants,
            active_rotations,
            failed_rotations,
            auto_rotation_enabled: self.config.auto_rotation_enabled,
            rotation_interval_days: self.config.rotation_interval_days,
        }
    }
}

/// Key rotation statistics
#[derive(Debug, Serialize)]
pub struct KeyRotationStats {
    /// Total number of tenants being tracked
    pub total_tenants: usize,
    /// Number of rotations currently in progress
    pub active_rotations: usize,
    /// Number of rotations that failed
    pub failed_rotations: usize,
    /// Whether automatic rotation is enabled
    pub auto_rotation_enabled: bool,
    /// Rotation interval in days
    pub rotation_interval_days: u32,
}
