// ABOUTME: System settings database operations for admin-configurable options
// ABOUTME: Provides get/set operations for settings like auto-approval
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::Database;
use crate::errors::{AppError, AppResult};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::Row;

/// System setting key constants
pub const SETTING_AUTO_APPROVAL_ENABLED: &str = "auto_approval_enabled";

/// A system setting entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSetting {
    /// Unique key identifier for the setting
    pub key: String,
    /// The current value of the setting
    pub value: String,
    /// Human-readable description of what this setting controls
    pub description: Option<String>,
    /// When the setting was last modified
    pub updated_at: chrono::DateTime<Utc>,
}

impl Database {
    /// Get a system setting by key
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_system_setting(&self, key: &str) -> AppResult<Option<SystemSetting>> {
        let row = sqlx::query(
            r"
            SELECT key, value, description, updated_at
            FROM system_settings
            WHERE key = ?1
            ",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get system setting: {e}")))?;

        row.map_or(Ok(None), |row| {
            let updated_at_str: String = row.get("updated_at");
            let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc));

            Ok(Some(SystemSetting {
                key: row.get("key"),
                value: row.get("value"),
                description: row.get("description"),
                updated_at,
            }))
        })
    }

    /// Set a system setting value
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn set_system_setting(&self, key: &str, value: &str) -> AppResult<()> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r"
            INSERT INTO system_settings (key, value, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?3)
            ON CONFLICT(key) DO UPDATE SET
                value = ?2,
                updated_at = ?3
            ",
        )
        .bind(key)
        .bind(value)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to set system setting: {e}")))?;

        Ok(())
    }

    /// Get all system settings
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_all_system_settings(&self) -> AppResult<Vec<SystemSetting>> {
        let rows = sqlx::query(
            r"
            SELECT key, value, description, updated_at
            FROM system_settings
            ORDER BY key
            ",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get system settings: {e}")))?;

        let mut settings = Vec::with_capacity(rows.len());
        for row in rows {
            let updated_at_str: String = row.get("updated_at");
            let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc));

            settings.push(SystemSetting {
                key: row.get("key"),
                value: row.get("value"),
                description: row.get("description"),
                updated_at,
            });
        }

        Ok(settings)
    }

    /// Check if auto-approval is enabled
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn is_auto_approval_enabled(&self) -> AppResult<bool> {
        match self
            .get_system_setting(SETTING_AUTO_APPROVAL_ENABLED)
            .await?
        {
            Some(setting) => Ok(setting.value.eq_ignore_ascii_case("true")),
            None => Ok(false), // Default to disabled
        }
    }

    /// Set auto-approval enabled state
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn set_auto_approval_enabled(&self, enabled: bool) -> AppResult<()> {
        self.set_system_setting(
            SETTING_AUTO_APPROVAL_ENABLED,
            if enabled { "true" } else { "false" },
        )
        .await
    }
}
