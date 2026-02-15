// ABOUTME: Key rotation mechanisms for enhanced security and compliance
// ABOUTME: Provides automated key rotation, version management, and seamless key transitions
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Key Rotation Module
//!
//! Automated key rotation for enhanced security including:
//! - Scheduled key rotation for tenants
//! - Seamless key version transitions
//! - Emergency key rotation procedures
//! - Key lifecycle management

use crate::constants::time;
use crate::database_plugins::factory::Database;
use crate::database_plugins::DatabaseProvider;
use crate::errors::AppResult;
use crate::models::TenantId;
use chrono::{Duration as ChronoDuration, Timelike, Utc};
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    runtime,
    sync::RwLock,
    time::{interval, Duration},
};
use tracing::{error, info, warn};

// Re-export DTOs from pierre-core (canonical definitions)
pub use pierre_core::models::{KeyRotationConfig, KeyVersion, RotationStatus};

/// Key rotation manager
pub struct KeyRotationManager {
    /// Encryption manager for performing key operations
    encryption_manager: Arc<super::TenantEncryptionManager>,
    /// Database for storing key metadata
    database: Arc<Database>,
    /// Audit logger
    auditor: Arc<super::audit::SecurityAuditor>,
    /// Rotation configuration
    config: KeyRotationConfig,
    /// Key version tracking
    key_versions: RwLock<HashMap<Option<TenantId>, Vec<KeyVersion>>>,
    /// Rotation status tracking
    rotation_status: RwLock<HashMap<Option<TenantId>, RotationStatus>>,
}

impl KeyRotationManager {
    /// Create new key rotation manager
    #[must_use]
    pub fn new(
        encryption_manager: Arc<super::TenantEncryptionManager>,
        database: Arc<Database>,
        auditor: Arc<super::audit::SecurityAuditor>,
        config: KeyRotationConfig,
    ) -> Self {
        Self {
            encryption_manager,
            database,
            auditor,
            config,
            key_versions: RwLock::new(HashMap::new()),
            rotation_status: RwLock::new(HashMap::new()),
        }
    }

    /// Start the key rotation scheduler
    ///
    /// # Errors
    ///
    /// Returns an error if the scheduler cannot be started
    pub fn start_scheduler(self: Arc<Self>) -> AppResult<()> {
        if !self.config.auto_rotation_enabled {
            info!("Key rotation scheduler disabled");
            return Ok(());
        }

        info!(
            "Starting key rotation scheduler - checking every {} days at {}:00 UTC",
            self.config.rotation_interval_days, self.config.rotation_hour
        );

        let manager = Arc::clone(&self);
        tokio::spawn(async move {
            let mut interval_timer = interval(Duration::from_secs(time::HOUR_SECONDS as u64)); // Check every hour

            loop {
                interval_timer.tick().await;

                let now = Utc::now();
                if u8::try_from(now.hour()).unwrap_or(0) == manager.config.rotation_hour {
                    if let Err(e) = manager.check_and_rotate_keys().await {
                        error!("Key rotation check failed: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Check all tenants and rotate keys as needed
    async fn check_and_rotate_keys(&self) -> AppResult<()> {
        info!("Checking for keys that need rotation");

        // Get all tenants from database
        let tenants = self.database.get_all_tenants().await?;

        // Check global keys first
        self.check_key_rotation(None).await?;

        // Check each tenant's keys
        for tenant in tenants {
            if let Err(e) = self.check_key_rotation(Some(tenant.id)).await {
                error!(
                    "Failed to check key rotation for tenant {}: {}",
                    tenant.id, e
                );
            }
        }

        Ok(())
    }

    /// Check if a specific tenant/global key needs rotation
    async fn check_key_rotation(&self, tenant_id: Option<TenantId>) -> AppResult<()> {
        let current_version = self.get_current_key_version(tenant_id).await?;

        if let Some(version) = current_version {
            let age_days = (Utc::now() - version.created_at).num_days();

            if age_days >= i64::from(self.config.rotation_interval_days) {
                info!(
                    "Key for tenant {:?} is {} days old, scheduling rotation",
                    tenant_id, age_days
                );
                self.schedule_key_rotation(tenant_id).await?;
            }
        } else {
            // No key version found, create initial version
            info!(
                "No key version found for tenant {:?}, creating initial version",
                tenant_id
            );
            self.initialize_key_version(tenant_id).await?;
        }

        Ok(())
    }

    /// Schedule a key rotation
    async fn schedule_key_rotation(&self, tenant_id: Option<TenantId>) -> AppResult<()> {
        let scheduled_at = Utc::now() + ChronoDuration::hours(1); // Schedule for 1 hour from now

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
            error!("Failed to log key rotation audit event: {}", e);
        }

        // Perform the rotation
        self.perform_key_rotation(tenant_id).await?;

        Ok(())
    }

    /// Update rotation status to in-progress
    async fn set_rotation_in_progress(&self, tenant_id: Option<TenantId>) {
        let mut status = self.rotation_status.write().await;
        status.insert(
            tenant_id,
            RotationStatus::InProgress {
                started_at: Utc::now(),
            },
        );
    }

    /// Update rotation status after completion or failure
    async fn update_rotation_status(&self, tenant_id: Option<TenantId>, result: &AppResult<()>) {
        match result {
            Ok(()) => {
                self.rotation_status.write().await.insert(
                    tenant_id,
                    RotationStatus::Completed {
                        completed_at: Utc::now(),
                    },
                );
                info!(
                    "Key rotation completed successfully for tenant {:?}",
                    tenant_id
                );
            }
            Err(e) => {
                self.rotation_status.write().await.insert(
                    tenant_id,
                    RotationStatus::Failed {
                        failed_at: Utc::now(),
                        error: e.to_string(),
                    },
                );
                error!("Key rotation failed for tenant {:?}: {}", tenant_id, e);
            }
        }
    }

    /// Perform actual key rotation
    async fn perform_key_rotation(&self, tenant_id: Option<TenantId>) -> AppResult<()> {
        info!("Starting key rotation for tenant {:?}", tenant_id);

        self.set_rotation_in_progress(tenant_id).await;

        let result = self.execute_key_rotation(tenant_id).await;
        self.update_rotation_status(tenant_id, &result).await;

        result
    }

    /// Execute the actual key rotation process
    async fn execute_key_rotation(&self, tenant_id: Option<TenantId>) -> AppResult<()> {
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
    async fn create_new_key_version(&self, tenant_id: Option<TenantId>) -> AppResult<KeyVersion> {
        let current_versions = self.get_key_versions(tenant_id).await?;
        let next_version = current_versions
            .iter()
            .map(|v| v.version)
            .max()
            .unwrap_or(0)
            + 1;

        let new_version = KeyVersion {
            version: next_version,
            created_at: Utc::now(),
            expires_at: Utc::now() + ChronoDuration::days(i64::from(self.config.max_key_age_days)),
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
    async fn activate_key_version(
        &self,
        tenant_id: Option<TenantId>,
        version: u32,
    ) -> AppResult<()> {
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
    async fn cleanup_old_key_versions(&self, tenant_id: Option<TenantId>) -> AppResult<()> {
        // Delete old key versions from database
        let deleted_count = self
            .database
            .delete_old_key_versions(tenant_id, self.config.key_versions_to_retain)
            .await?;

        if deleted_count > 0 {
            info!(
                "Cleaned up {} old key versions for tenant {:?}",
                deleted_count, tenant_id
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
    async fn initialize_key_version(&self, tenant_id: Option<TenantId>) -> AppResult<()> {
        let initial_version = KeyVersion {
            version: 1,
            created_at: Utc::now(),
            expires_at: Utc::now() + ChronoDuration::days(i64::from(self.config.max_key_age_days)),
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
        tenant_id: Option<TenantId>,
    ) -> AppResult<Option<KeyVersion>> {
        let versions = self.get_key_versions(tenant_id).await?;
        Ok(versions.into_iter().find(|v| v.is_active))
    }

    /// Get all key versions for a tenant
    async fn get_key_versions(&self, tenant_id: Option<TenantId>) -> AppResult<Vec<KeyVersion>> {
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
        let rt = runtime::Handle::current();
        rt.block_on(self.database.store_key_version(version))
    }

    /// Get rotation status for a tenant
    pub async fn get_rotation_status(&self, tenant_id: Option<TenantId>) -> RotationStatus {
        let status = self.rotation_status.read().await;
        status
            .get(&tenant_id)
            .cloned()
            .unwrap_or(RotationStatus::Current)
    }

    /// Build emergency key rotation audit event
    fn build_emergency_rotation_audit_event(
        tenant_id: Option<TenantId>,
        reason: &str,
    ) -> super::audit::AuditEvent {
        let event = super::audit::AuditEvent::new(
            super::audit::AuditEventType::KeyRotated,
            super::audit::AuditSeverity::Critical,
            format!("Emergency key rotation: {reason}"),
            "emergency_rotation".to_owned(),
            "initiated".to_owned(),
        );

        if let Some(tid) = tenant_id {
            event.with_tenant_id(tid)
        } else {
            event
        }
    }

    /// Force immediate key rotation (for emergency scenarios)
    ///
    /// # Errors
    ///
    /// Returns an error if emergency rotation fails
    pub async fn emergency_key_rotation(
        &self,
        tenant_id: Option<TenantId>,
        reason: &str,
    ) -> AppResult<()> {
        warn!(
            "Emergency key rotation initiated for tenant {:?}. Reason: {}",
            tenant_id, reason
        );

        // Log critical audit event
        let event = Self::build_emergency_rotation_audit_event(tenant_id, reason);
        if let Err(e) = self.auditor.log_event(event).await {
            error!("Failed to log emergency key rotation audit: {}", e);
        }

        // Perform immediate rotation
        self.perform_key_rotation(tenant_id).await?;

        info!(
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
