// ABOUTME: Database operations for tenant-specific fitness configurations
// ABOUTME: Handles CRUD operations for fitness settings with tenant isolation and user-specific overrides
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

use crate::config::fitness_config::FitnessConfig;

/// Database representation of a fitness configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessConfigurationRecord {
    pub id: String,
    pub tenant_id: String,
    pub user_id: Option<String>,
    pub configuration_name: String,
    pub config_data: String, // JSON serialized FitnessConfig
    pub created_at: String,
    pub updated_at: String,
}

/// Fitness configuration database operations
pub struct FitnessConfigurationManager {
    pool: SqlitePool,
}

impl FitnessConfigurationManager {
    /// Create a new fitness configuration manager
    #[must_use]
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Save or update a fitness configuration for a tenant
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails or config serialization fails
    pub async fn save_tenant_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
        config: &FitnessConfig,
    ) -> Result<String> {
        let config_json = serde_json::to_string(config)?;

        let result = sqlx::query(
            r"
            INSERT INTO fitness_configurations (tenant_id, user_id, configuration_name, config_data)
            VALUES ($1, NULL, $2, $3)
            ON CONFLICT (tenant_id, user_id, configuration_name) 
            DO UPDATE SET 
                config_data = EXCLUDED.config_data,
                updated_at = datetime('now')
            RETURNING id
            ",
        )
        .bind(tenant_id)
        .bind(configuration_name)
        .bind(&config_json)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.get("id"))
    }

    /// Save or update a fitness configuration for a specific user
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails or config serialization fails
    pub async fn save_user_config(
        &self,
        tenant_id: &str,
        user_id: &str,
        configuration_name: &str,
        config: &FitnessConfig,
    ) -> Result<String> {
        let config_json = serde_json::to_string(config)?;

        let result = sqlx::query(
            r"
            INSERT INTO fitness_configurations (tenant_id, user_id, configuration_name, config_data)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (tenant_id, user_id, configuration_name) 
            DO UPDATE SET 
                config_data = EXCLUDED.config_data,
                updated_at = datetime('now')
            RETURNING id
            ",
        )
        .bind(tenant_id)
        .bind(user_id)
        .bind(configuration_name)
        .bind(&config_json)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.get("id"))
    }

    /// Get fitness configuration for a specific user (checks user-specific first, then tenant default)
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails or config deserialization fails
    pub async fn get_user_config(
        &self,
        tenant_id: &str,
        user_id: &str,
        configuration_name: &str,
    ) -> Result<Option<FitnessConfig>> {
        // First try to get user-specific configuration
        if let Some(config) = self
            .get_config_internal(tenant_id, Some(user_id), configuration_name)
            .await?
        {
            return Ok(Some(config));
        }

        // Fall back to tenant default configuration
        self.get_config_internal(tenant_id, None, configuration_name)
            .await
    }

    /// Get fitness configuration for a tenant (tenant-level default only)
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails or config deserialization fails
    pub async fn get_tenant_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
    ) -> Result<Option<FitnessConfig>> {
        self.get_config_internal(tenant_id, None, configuration_name)
            .await
    }

    /// Internal method to get configuration from database
    async fn get_config_internal(
        &self,
        tenant_id: &str,
        user_id: Option<&str>,
        configuration_name: &str,
    ) -> Result<Option<FitnessConfig>> {
        let result = if let Some(uid) = user_id {
            sqlx::query(
                r"
                SELECT config_data FROM fitness_configurations
                WHERE tenant_id = $1 AND user_id = $2 AND configuration_name = $3
                ",
            )
            .bind(tenant_id)
            .bind(uid)
            .bind(configuration_name)
            .fetch_optional(&self.pool)
            .await?
        } else {
            sqlx::query(
                r"
                SELECT config_data FROM fitness_configurations
                WHERE tenant_id = $1 AND user_id IS NULL AND configuration_name = $2
                ",
            )
            .bind(tenant_id)
            .bind(configuration_name)
            .fetch_optional(&self.pool)
            .await?
        };

        if let Some(row) = result {
            let config_json: String = row.get("config_data");
            let config: FitnessConfig = serde_json::from_str(&config_json)?;
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }

    /// List all configuration names for a tenant
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn list_tenant_configurations(&self, tenant_id: &str) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r"
            SELECT DISTINCT configuration_name FROM fitness_configurations
            WHERE tenant_id = $1
            ORDER BY configuration_name
            ",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        let configurations = rows
            .into_iter()
            .map(|row| row.get::<String, _>("configuration_name"))
            .collect();

        Ok(configurations)
    }

    /// List all configuration names for a specific user
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn list_user_configurations(
        &self,
        tenant_id: &str,
        user_id: &str,
    ) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r"
            SELECT DISTINCT configuration_name FROM fitness_configurations
            WHERE tenant_id = $1 AND user_id = $2
            ORDER BY configuration_name
            ",
        )
        .bind(tenant_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let configurations = rows
            .into_iter()
            .map(|row| row.get::<String, _>("configuration_name"))
            .collect();

        Ok(configurations)
    }

    /// Delete a fitness configuration
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn delete_config(
        &self,
        tenant_id: &str,
        user_id: Option<&str>,
        configuration_name: &str,
    ) -> Result<bool> {
        let rows_affected = if let Some(uid) = user_id {
            sqlx::query(
                r"
                DELETE FROM fitness_configurations
                WHERE tenant_id = $1 AND user_id = $2 AND configuration_name = $3
                ",
            )
            .bind(tenant_id)
            .bind(uid)
            .bind(configuration_name)
            .execute(&self.pool)
            .await?
        } else {
            sqlx::query(
                r"
                DELETE FROM fitness_configurations
                WHERE tenant_id = $1 AND user_id IS NULL AND configuration_name = $2
                ",
            )
            .bind(tenant_id)
            .bind(configuration_name)
            .execute(&self.pool)
            .await?
        };

        Ok(rows_affected.rows_affected() > 0)
    }

    /// Get all fitness configuration records for a tenant (for admin purposes)
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn get_all_tenant_records(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<FitnessConfigurationRecord>> {
        let rows = sqlx::query(
            r"
            SELECT id, tenant_id, user_id, configuration_name, config_data, created_at, updated_at
            FROM fitness_configurations
            WHERE tenant_id = $1
            ORDER BY user_id, configuration_name
            ",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            records.push(FitnessConfigurationRecord {
                id: row.get("id"),
                tenant_id: row.get("tenant_id"),
                user_id: row.get("user_id"),
                configuration_name: row.get("configuration_name"),
                config_data: row.get("config_data"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(records)
    }
}
