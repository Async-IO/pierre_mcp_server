// ABOUTME: Admin token management for SQLite
// ABOUTME: Handles creation, retrieval, and tracking of admin authentication tokens
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::Database;
use crate::errors::{AppError, AppResult};
use chrono::{DateTime, Utc};
use sqlx::Row;
use tracing::debug;

impl Database {
    /// Create a new admin token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - JWT token generation fails
    /// - Database insertion fails
    /// - Permissions JSON serialization fails
    pub async fn create_admin_token(
        &self,
        request: &crate::admin::models::CreateAdminTokenRequest,
        admin_jwt_secret: &str,
        jwks_manager: &crate::admin::jwks::JwksManager,
    ) -> AppResult<crate::admin::models::GeneratedAdminToken> {
        use crate::admin::{
            jwt::AdminJwtManager,
            models::{AdminPermissions, GeneratedAdminToken},
        };
        use uuid::Uuid;

        // Generate unique token ID
        let uuid = Uuid::new_v4().simple();
        let token_id = format!("admin_{uuid}");

        // Debug: Log token creation without exposing secrets
        debug!("Creating admin token with RS256 asymmetric signing");

        // Create JWT manager for RS256 token operations
        let jwt_manager = AdminJwtManager::new();

        // Get permissions
        let permissions = request.permissions.as_ref().map_or_else(
            || {
                if request.is_super_admin {
                    AdminPermissions::super_admin()
                } else {
                    AdminPermissions::default_admin()
                }
            },
            |perms| AdminPermissions::new(perms.clone()),
        );

        // Calculate expiration (0 days means never expires)
        let expires_at = request.expires_in_days.and_then(|days| {
            if days == 0 {
                None // Never expires
            } else {
                Some(
                    chrono::Utc::now() + chrono::Duration::days(i64::try_from(days).unwrap_or(365)),
                )
            }
        });

        // Generate JWT token using RS256
        let jwt_token = jwt_manager
            .generate_token(
                &token_id,
                &request.service_name,
                &permissions,
                request.is_super_admin,
                expires_at,
                jwks_manager,
            )
            .map_err(|e| AppError::internal(format!("Failed to generate JWT token: {e}")))?;

        // Generate token prefix and hash for storage
        let token_prefix = AdminJwtManager::generate_token_prefix(&jwt_token);
        let token_hash = AdminJwtManager::hash_token_for_storage(&jwt_token)
            .map_err(|e| AppError::internal(format!("Failed to hash JWT token: {e}")))?;
        let jwt_secret_hash = AdminJwtManager::hash_secret(admin_jwt_secret);

        // Store in database (SQLite uses ? placeholders)
        let query = r"
            INSERT INTO admin_tokens (
                id, service_name, service_description, token_hash, token_prefix,
                jwt_secret_hash, permissions, is_super_admin, is_active,
                created_at, expires_at, usage_count
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ";

        let permissions_json = permissions.to_json()?;
        let created_at = chrono::Utc::now();

        sqlx::query(query)
            .bind(&token_id)
            .bind(&request.service_name)
            .bind(&request.service_description)
            .bind(&token_hash)
            .bind(&token_prefix)
            .bind(&jwt_secret_hash)
            .bind(&permissions_json)
            .bind(request.is_super_admin)
            .bind(true) // is_active
            .bind(created_at)
            .bind(expires_at)
            .bind(0i64) // usage_count
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to create admin token: {e}")))?;

        Ok(GeneratedAdminToken {
            token_id,
            service_name: request.service_name.clone(),
            jwt_token,
            token_prefix,
            permissions,
            is_super_admin: request.is_super_admin,
            expires_at,
            created_at,
        })
    }

    /// Get admin token by ID
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn get_admin_token_by_id(
        &self,
        token_id: &str,
    ) -> AppResult<Option<crate::admin::models::AdminToken>> {
        let query = r"
            SELECT id, service_name, service_description, token_hash, token_prefix,
                   jwt_secret_hash, permissions, is_super_admin, is_active,
                   created_at, expires_at, last_used_at, last_used_ip, usage_count
            FROM admin_tokens WHERE id = ?
        ";

        let row = sqlx::query(query)
            .bind(token_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to get admin token by ID: {e}")))?;

        if let Some(row) = row {
            Ok(Some(Self::row_to_admin_token(&row)?))
        } else {
            Ok(None)
        }
    }

    /// Get admin token by prefix
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn get_admin_token_by_prefix(
        &self,
        token_prefix: &str,
    ) -> AppResult<Option<crate::admin::models::AdminToken>> {
        let query = r"
            SELECT id, service_name, service_description, token_hash, token_prefix,
                   jwt_secret_hash, permissions, is_super_admin, is_active,
                   created_at, expires_at, last_used_at, last_used_ip, usage_count
            FROM admin_tokens WHERE token_prefix = ?
        ";

        let row = sqlx::query(query)
            .bind(token_prefix)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to get admin token by prefix: {e}")))?;

        if let Some(row) = row {
            Ok(Some(Self::row_to_admin_token(&row)?))
        } else {
            Ok(None)
        }
    }

    /// List all admin tokens
    ///
    /// # Errors
    ///
    /// Returns an error if database query or row parsing fails
    pub async fn list_admin_tokens(
        &self,
        include_inactive: bool,
    ) -> AppResult<Vec<crate::admin::models::AdminToken>> {
        let query = if include_inactive {
            r"
                SELECT id, service_name, service_description, token_hash, token_prefix,
                       jwt_secret_hash, permissions, is_super_admin, is_active,
                       created_at, expires_at, last_used_at, last_used_ip, usage_count
                FROM admin_tokens ORDER BY created_at DESC
            "
        } else {
            r"
                SELECT id, service_name, service_description, token_hash, token_prefix,
                       jwt_secret_hash, permissions, is_super_admin, is_active,
                       created_at, expires_at, last_used_at, last_used_ip, usage_count
                FROM admin_tokens WHERE is_active = 1 ORDER BY created_at DESC
            "
        };

        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to list admin tokens: {e}")))?;

        let mut tokens = Vec::with_capacity(rows.len());
        for row in rows {
            tokens.push(Self::row_to_admin_token(&row)?);
        }

        Ok(tokens)
    }

    /// Deactivate an admin token
    ///
    /// # Errors
    ///
    /// Returns an error if database update fails
    pub async fn deactivate_admin_token_impl(&self, token_id: &str) -> AppResult<()> {
        let query = "UPDATE admin_tokens SET is_active = 0 WHERE id = ?";

        sqlx::query(query)
            .bind(token_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to deactivate admin token: {e}")))?;

        Ok(())
    }

    /// Update admin token last used timestamp
    ///
    /// # Errors
    ///
    /// Returns an error if database update fails
    pub async fn update_admin_token_last_used(
        &self,
        token_id: &str,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
        let query = r"
            UPDATE admin_tokens
            SET last_used_at = CURRENT_TIMESTAMP, last_used_ip = ?, usage_count = usage_count + 1
            WHERE id = ?
        ";

        sqlx::query(query)
            .bind(ip_address)
            .bind(token_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to update admin token last used: {e}"))
            })?;

        Ok(())
    }

    /// Record admin token usage
    ///
    /// # Errors
    ///
    /// Returns an error if database insertion fails
    pub async fn record_admin_token_usage(
        &self,
        usage: &crate::admin::models::AdminTokenUsage,
    ) -> AppResult<()> {
        let query = r"
            INSERT INTO admin_token_usage (
                admin_token_id, timestamp, action, target_resource,
                ip_address, user_agent, request_size_bytes, success,
                response_time_ms
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ";

        sqlx::query(query)
            .bind(&usage.admin_token_id)
            .bind(usage.timestamp)
            .bind(usage.action.to_string())
            .bind(&usage.target_resource)
            .bind(&usage.ip_address)
            .bind(&usage.user_agent)
            .bind(
                usage
                    .request_size_bytes
                    .map(|x| i32::try_from(x).unwrap_or(i32::MAX)),
            )
            .bind(usage.success)
            .bind(
                usage
                    .response_time_ms
                    .map(|x| i32::try_from(x).unwrap_or(i32::MAX)),
            )
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to record admin token usage: {e}")))?;

        Ok(())
    }

    /// Get admin token usage history
    ///
    /// # Errors
    ///
    /// Returns an error if database query or row parsing fails
    pub async fn get_admin_token_usage_history(
        &self,
        token_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> AppResult<Vec<crate::admin::models::AdminTokenUsage>> {
        let query = r"
            SELECT id, admin_token_id, timestamp, action, target_resource,
                   ip_address, user_agent, request_size_bytes, success,
                   response_time_ms
            FROM admin_token_usage
            WHERE admin_token_id = ? AND timestamp BETWEEN ? AND ?
            ORDER BY timestamp DESC
        ";

        let rows = sqlx::query(query)
            .bind(token_id)
            .bind(start_date)
            .bind(end_date)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to get admin token usage history: {e}"))
            })?;

        let mut usage_history = Vec::new();
        for row in rows {
            usage_history.push(Self::row_to_admin_token_usage(&row)?);
        }

        Ok(usage_history)
    }

    /// Record an admin-provisioned API key
    ///
    /// # Errors
    ///
    /// Returns an error if database insertion or token lookup fails
    pub async fn record_admin_provisioned_key(
        &self,
        admin_token_id: &str,
        api_key_id: &str,
        user_email: &str,
        tier: &str,
        rate_limit_requests: u32,
        rate_limit_period: &str,
    ) -> AppResult<()> {
        let query = r"
            INSERT INTO admin_provisioned_keys (
                admin_token_id, api_key_id, user_email, requested_tier,
                provisioned_at, provisioned_by_service, rate_limit_requests,
                rate_limit_period, key_status
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ";

        // Get service name from admin token
        let service_name = if let Some(token) = self.get_admin_token_by_id(admin_token_id).await? {
            token.service_name
        } else {
            "unknown".into()
        };

        sqlx::query(query)
            .bind(admin_token_id)
            .bind(api_key_id)
            .bind(user_email)
            .bind(tier)
            .bind(chrono::Utc::now())
            .bind(service_name)
            .bind(i32::try_from(rate_limit_requests).unwrap_or(i32::MAX))
            .bind(rate_limit_period)
            .bind("active")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to record admin provisioned key: {e}"))
            })?;

        Ok(())
    }

    /// Get admin provisioned keys
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn get_admin_provisioned_keys(
        &self,
        admin_token_id: Option<&str>,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> AppResult<Vec<serde_json::Value>> {
        if let Some(token_id) = admin_token_id {
            let rows = sqlx::query(
                r"
                    SELECT id, admin_token_id, api_key_id, user_email, requested_tier,
                           provisioned_at, provisioned_by_service, rate_limit_requests,
                           rate_limit_period, key_status, revoked_at, revoked_reason
                    FROM admin_provisioned_keys
                    WHERE admin_token_id = ? AND provisioned_at BETWEEN ? AND ?
                    ORDER BY provisioned_at DESC
                ",
            )
            .bind(token_id)
            .bind(start_date)
            .bind(end_date)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to get admin provisioned keys: {e}"))
            })?;

            let mut results = Vec::new();
            for row in rows {
                let result = serde_json::json!({
                    "id": row.get::<i32, _>("id"),
                    "admin_token_id": row.get::<String, _>("admin_token_id"),
                    "api_key_id": row.get::<String, _>("api_key_id"),
                    "user_email": row.get::<String, _>("user_email"),
                    "requested_tier": row.get::<String, _>("requested_tier"),
                    "provisioned_at": row.get::<DateTime<Utc>, _>("provisioned_at"),
                    "provisioned_by_service": row.get::<String, _>("provisioned_by_service"),
                    "rate_limit_requests": row.get::<i32, _>("rate_limit_requests"),
                    "rate_limit_period": row.get::<String, _>("rate_limit_period"),
                    "key_status": row.get::<String, _>("key_status"),
                    "revoked_at": row.get::<Option<DateTime<Utc>>, _>("revoked_at"),
                    "revoked_reason": row.get::<Option<String>, _>("revoked_reason"),
                });
                results.push(result);
            }
            Ok(results)
        } else {
            let rows = sqlx::query(
                r"
                    SELECT id, admin_token_id, api_key_id, user_email, requested_tier,
                           provisioned_at, provisioned_by_service, rate_limit_requests,
                           rate_limit_period, key_status, revoked_at, revoked_reason
                    FROM admin_provisioned_keys
                    WHERE provisioned_at BETWEEN ? AND ?
                    ORDER BY provisioned_at DESC
                ",
            )
            .bind(start_date)
            .bind(end_date)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to get all admin provisioned keys: {e}"))
            })?;

            let mut results = Vec::new();
            for row in rows {
                let result = serde_json::json!({
                    "id": row.get::<i32, _>("id"),
                    "admin_token_id": row.get::<String, _>("admin_token_id"),
                    "api_key_id": row.get::<String, _>("api_key_id"),
                    "user_email": row.get::<String, _>("user_email"),
                    "requested_tier": row.get::<String, _>("requested_tier"),
                    "provisioned_at": row.get::<DateTime<Utc>, _>("provisioned_at"),
                    "provisioned_by_service": row.get::<String, _>("provisioned_by_service"),
                    "rate_limit_requests": row.get::<i32, _>("rate_limit_requests"),
                    "rate_limit_period": row.get::<String, _>("rate_limit_period"),
                    "key_status": row.get::<String, _>("key_status"),
                    "revoked_at": row.get::<Option<DateTime<Utc>>, _>("revoked_at"),
                    "revoked_reason": row.get::<Option<String>, _>("revoked_reason"),
                });
                results.push(result);
            }
            Ok(results)
        }
    }

    /// Helper: Convert row to `AdminToken`
    ///
    /// # Errors
    ///
    /// Returns an error if row parsing or JSON deserialization fails
    fn row_to_admin_token(
        row: &sqlx::sqlite::SqliteRow,
    ) -> AppResult<crate::admin::models::AdminToken> {
        use crate::admin::models::{AdminPermissions, AdminToken};
        use sqlx::Row;

        let permissions_json: String = row
            .try_get("permissions")
            .map_err(|e| AppError::database(format!("Failed to get permissions from row: {e}")))?;
        let permissions = AdminPermissions::from_json(&permissions_json)?;

        Ok(AdminToken {
            id: row
                .try_get("id")
                .map_err(|e| AppError::database(format!("Failed to get id from row: {e}")))?,
            service_name: row.try_get("service_name").map_err(|e| {
                AppError::database(format!("Failed to get service_name from row: {e}"))
            })?,
            service_description: row.try_get("service_description").map_err(|e| {
                AppError::database(format!("Failed to get service_description from row: {e}"))
            })?,
            token_hash: row.try_get("token_hash").map_err(|e| {
                AppError::database(format!("Failed to get token_hash from row: {e}"))
            })?,
            token_prefix: row.try_get("token_prefix").map_err(|e| {
                AppError::database(format!("Failed to get token_prefix from row: {e}"))
            })?,
            jwt_secret_hash: row.try_get("jwt_secret_hash").map_err(|e| {
                AppError::database(format!("Failed to get jwt_secret_hash from row: {e}"))
            })?,
            permissions,
            is_super_admin: row.try_get("is_super_admin").map_err(|e| {
                AppError::database(format!("Failed to get is_super_admin from row: {e}"))
            })?,
            is_active: row.try_get("is_active").map_err(|e| {
                AppError::database(format!("Failed to get is_active from row: {e}"))
            })?,
            created_at: row.try_get("created_at").map_err(|e| {
                AppError::database(format!("Failed to get created_at from row: {e}"))
            })?,
            expires_at: row.try_get("expires_at").map_err(|e| {
                AppError::database(format!("Failed to get expires_at from row: {e}"))
            })?,
            last_used_at: row.try_get("last_used_at").map_err(|e| {
                AppError::database(format!("Failed to get last_used_at from row: {e}"))
            })?,
            last_used_ip: row.try_get("last_used_ip").map_err(|e| {
                AppError::database(format!("Failed to get last_used_ip from row: {e}"))
            })?,
            usage_count: row.try_get("usage_count").map_err(|e| {
                AppError::database(format!("Failed to get usage_count from row: {e}"))
            })?,
        })
    }

    /// Helper: Convert row to `AdminTokenUsage`
    ///
    /// # Errors
    ///
    /// Returns an error if row parsing fails
    fn row_to_admin_token_usage(
        row: &sqlx::sqlite::SqliteRow,
    ) -> AppResult<crate::admin::models::AdminTokenUsage> {
        use crate::admin::models::{AdminAction, AdminTokenUsage};
        use sqlx::Row;

        let action_str: String = row
            .try_get("action")
            .map_err(|e| AppError::database(format!("Failed to get action from row: {e}")))?;
        let action = action_str
            .parse::<AdminAction>()
            .unwrap_or(AdminAction::ProvisionKey);

        Ok(AdminTokenUsage {
            id: Some(
                row.try_get("id")
                    .map_err(|e| AppError::database(format!("Failed to get id from row: {e}")))?,
            ),
            admin_token_id: row.try_get("admin_token_id").map_err(|e| {
                AppError::database(format!("Failed to get admin_token_id from row: {e}"))
            })?,
            timestamp: row.try_get("timestamp").map_err(|e| {
                AppError::database(format!("Failed to get timestamp from row: {e}"))
            })?,
            action,
            target_resource: row.try_get("target_resource").map_err(|e| {
                AppError::database(format!("Failed to get target_resource from row: {e}"))
            })?,
            ip_address: row.try_get("ip_address").map_err(|e| {
                AppError::database(format!("Failed to get ip_address from row: {e}"))
            })?,
            user_agent: row.try_get("user_agent").map_err(|e| {
                AppError::database(format!("Failed to get user_agent from row: {e}"))
            })?,
            request_size_bytes: row
                .try_get::<Option<i32>, _>("request_size_bytes")
                .map_err(|e| {
                    AppError::database(format!("Failed to get request_size_bytes from row: {e}"))
                })?
                .map(|x| u32::try_from(x).unwrap_or(0)),
            success: row
                .try_get("success")
                .map_err(|e| AppError::database(format!("Failed to get success from row: {e}")))?,
            error_message: None,
            response_time_ms: row
                .try_get::<Option<i32>, _>("response_time_ms")
                .map_err(|e| {
                    AppError::database(format!("Failed to get response_time_ms from row: {e}"))
                })?
                .map(|x| u32::try_from(x).unwrap_or(0)),
        })
    }
    // Public wrapper methods (delegate to _impl versions)

    /// Deactivate admin token (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn deactivate_admin_token(&self, token_id: &str) -> AppResult<()> {
        self.deactivate_admin_token_impl(token_id).await
    }
}
