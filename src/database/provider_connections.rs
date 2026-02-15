// ABOUTME: Database operations for provider connections (unified connection tracking)
// ABOUTME: CRUD methods for the provider_connections table, the single source of truth for provider connectivity
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::database::Database;
use crate::errors::AppResult;
use crate::models::{ConnectionType, ProviderConnection};
use chrono::{DateTime, Utc};
use pierre_core::models::TenantId;
use sqlx::Row;
use uuid::Uuid;

impl Database {
    /// Register a provider connection (upsert)
    ///
    /// Creates or updates a record in `provider_connections` for the given user/tenant/provider.
    /// Uses `ON CONFLICT` to update the `connection_type`, `connected_at`, and `metadata` if already exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub async fn register_provider_connection_impl(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        provider: &str,
        connection_type: &ConnectionType,
        metadata: Option<&str>,
    ) -> AppResult<()> {
        let id = Uuid::new_v4().to_string();
        let user_id_str = user_id.to_string();
        let now = Utc::now().to_rfc3339();
        let conn_type_str = connection_type.as_str();

        sqlx::query(
            r"
            INSERT INTO provider_connections (id, user_id, tenant_id, provider, connection_type, connected_at, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(user_id, tenant_id, provider) DO UPDATE SET
                connection_type = excluded.connection_type,
                connected_at = excluded.connected_at,
                metadata = excluded.metadata
            ",
        )
        .bind(&id)
        .bind(&user_id_str)
        .bind(tenant_id.to_string())
        .bind(provider)
        .bind(conn_type_str)
        .bind(&now)
        .bind(metadata)
        .execute(self.pool())
        .await?;

        Ok(())
    }

    /// Remove a provider connection
    ///
    /// Deletes the `provider_connections` record for the given user/tenant/provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub async fn remove_provider_connection_impl(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        provider: &str,
    ) -> AppResult<()> {
        let user_id_str = user_id.to_string();

        sqlx::query(
            "DELETE FROM provider_connections WHERE user_id = ? AND tenant_id = ? AND provider = ?",
        )
        .bind(&user_id_str)
        .bind(tenant_id.to_string())
        .bind(provider)
        .execute(self.pool())
        .await?;

        Ok(())
    }

    /// Get all provider connections for a user
    ///
    /// Returns all connected providers across tenants (cross-tenant view) when `tenant_id` is None,
    /// or scoped to a specific tenant when `tenant_id` is provided.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub async fn get_user_provider_connections_impl(
        &self,
        user_id: Uuid,
        tenant_id: Option<TenantId>,
    ) -> AppResult<Vec<ProviderConnection>> {
        let user_id_str = user_id.to_string();

        let rows = if let Some(tid) = tenant_id {
            sqlx::query(
                r"
                SELECT id, user_id, tenant_id, provider, connection_type, connected_at, metadata
                FROM provider_connections
                WHERE user_id = ? AND tenant_id = ?
                ORDER BY connected_at DESC
                ",
            )
            .bind(&user_id_str)
            .bind(tid)
            .fetch_all(self.pool())
            .await?
        } else {
            sqlx::query(
                r"
                SELECT id, user_id, tenant_id, provider, connection_type, connected_at, metadata
                FROM provider_connections
                WHERE user_id = ?
                ORDER BY connected_at DESC
                ",
            )
            .bind(&user_id_str)
            .fetch_all(self.pool())
            .await?
        };

        let mut connections = Vec::with_capacity(rows.len());
        for row in rows {
            let conn_type_str: String = row.get("connection_type");
            let connected_at_str: String = row.get("connected_at");
            let connected_at = DateTime::parse_from_rfc3339(&connected_at_str)
                .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc));

            let user_id_from_db: String = row.get("user_id");
            let parsed_user_id = Uuid::parse_str(&user_id_from_db).unwrap_or_else(|_| Uuid::nil());

            connections.push(ProviderConnection {
                id: row.get("id"),
                user_id: parsed_user_id,
                tenant_id: row.get("tenant_id"),
                provider: row.get("provider"),
                connection_type: ConnectionType::from_str_value(&conn_type_str)
                    .unwrap_or(ConnectionType::Manual),
                connected_at,
                metadata: row.get("metadata"),
            });
        }

        Ok(connections)
    }

    /// Check if a specific provider is connected for a user
    ///
    /// Cross-tenant check: returns true if the provider is connected in any tenant.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub async fn is_provider_connected_impl(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> AppResult<bool> {
        let user_id_str = user_id.to_string();

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM provider_connections WHERE user_id = ? AND provider = ?",
        )
        .bind(&user_id_str)
        .bind(provider)
        .fetch_one(self.pool())
        .await?;

        Ok(count > 0)
    }
}
