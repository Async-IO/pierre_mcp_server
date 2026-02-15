// ABOUTME: UserOAuthToken database operations for per-user, per-tenant OAuth credential storage
// ABOUTME: Handles tenant-aware OAuth token management for multi-tenant architecture
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::Database;
use crate::errors::{AppError, AppResult};
use crate::models::UserOAuthToken;
use chrono::{DateTime, Utc};
use pierre_core::models::TenantId;
use sqlx::sqlite::SqliteRow;
use sqlx::Row;
use uuid::Uuid;

/// OAuth token data for database operations
pub struct OAuthTokenData<'a> {
    /// Unique token identifier
    pub id: &'a str,
    /// User ID this token belongs to
    pub user_id: Uuid,
    /// Tenant ID for multi-tenant isolation
    pub tenant_id: TenantId,
    /// OAuth provider (e.g., "strava", "fitbit")
    pub provider: &'a str,
    /// OAuth access token
    pub access_token: &'a str,
    /// Optional OAuth refresh token
    pub refresh_token: Option<&'a str>,
    /// Token type (usually "Bearer")
    pub token_type: &'a str,
    /// When the access token expires
    pub expires_at: Option<DateTime<Utc>>,
    /// OAuth scope string
    pub scope: &'a str,
}

impl Database {
    /// Upsert a user OAuth token using structured data
    ///
    /// Provider tokens are encrypted at rest using AES-256-GCM with AAD binding
    /// to prevent cross-tenant or cross-user token reuse.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Encryption fails
    /// - Database operation fails
    pub async fn upsert_user_oauth_token(&self, token_data: &OAuthTokenData<'_>) -> AppResult<()> {
        // Create AAD context: tenant_id|user_id|provider|table
        let aad_context = format!(
            "{}|{}|{}|user_oauth_tokens",
            token_data.tenant_id, token_data.user_id, token_data.provider
        );

        // Encrypt access token with AAD binding
        let encrypted_access_token =
            self.encrypt_data_with_aad(token_data.access_token, &aad_context)?;

        // Encrypt refresh token if present
        let encrypted_refresh_token = token_data
            .refresh_token
            .map(|rt| self.encrypt_data_with_aad(rt, &aad_context))
            .transpose()?;

        sqlx::query(
            r"
            INSERT INTO user_oauth_tokens (
                id, user_id, tenant_id, provider, access_token, refresh_token,
                token_type, expires_at, scope, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (user_id, tenant_id, provider)
            DO UPDATE SET
                id = EXCLUDED.id,
                access_token = EXCLUDED.access_token,
                refresh_token = EXCLUDED.refresh_token,
                token_type = EXCLUDED.token_type,
                expires_at = EXCLUDED.expires_at,
                scope = EXCLUDED.scope,
                updated_at = EXCLUDED.updated_at
            ",
        )
        .bind(token_data.id)
        .bind(token_data.user_id.to_string())
        .bind(token_data.tenant_id.to_string())
        .bind(token_data.provider)
        .bind(&encrypted_access_token)
        .bind(encrypted_refresh_token.as_deref())
        .bind(token_data.token_type)
        .bind(token_data.expires_at)
        .bind(token_data.scope)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to upsert user OAuth token: {e}")))?;

        Ok(())
    }

    /// Get a user OAuth token
    ///
    /// Decrypts provider tokens using AAD binding for security.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query fails
    /// - Decryption fails (possibly due to tampered data or AAD mismatch)
    pub async fn get_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        provider: &str,
    ) -> AppResult<Option<UserOAuthToken>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, provider, access_token, refresh_token,
                   token_type, expires_at, scope, created_at, updated_at
            FROM user_oauth_tokens
            WHERE user_id = $1 AND tenant_id = $2 AND provider = $3
            ",
        )
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .bind(provider)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to query user OAuth token: {e}")))?;

        row.map_or_else(
            || Ok(None),
            |row| Ok(Some(self.row_to_user_oauth_token(&row)?)),
        )
    }

    /// Get all OAuth tokens for a user, optionally scoped to a specific tenant
    ///
    /// When `tenant_id` is `Some`, only tokens for that tenant are returned.
    /// When `None`, returns tokens across all tenants (intentional cross-tenant view
    /// for OAuth status checks, e.g. admin dashboards).
    ///
    /// Decrypts provider tokens using AAD binding for security.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query fails
    /// - Decryption fails for any token
    pub async fn get_user_oauth_tokens_impl(
        &self,
        user_id: Uuid,
        tenant_id: Option<TenantId>,
    ) -> AppResult<Vec<UserOAuthToken>> {
        let rows = if let Some(tid) = tenant_id {
            sqlx::query(
                r"
                SELECT id, user_id, tenant_id, provider, access_token, refresh_token,
                       token_type, expires_at, scope, created_at, updated_at
                FROM user_oauth_tokens
                WHERE user_id = $1 AND tenant_id = $2
                ORDER BY created_at DESC
                ",
            )
            .bind(user_id.to_string())
            .bind(tid.to_string())
            .fetch_all(&self.pool)
            .await
        } else {
            // Intentional cross-tenant view for OAuth status checks (e.g. admin dashboards)
            sqlx::query(
                r"
                SELECT id, user_id, tenant_id, provider, access_token, refresh_token,
                       token_type, expires_at, scope, created_at, updated_at
                FROM user_oauth_tokens
                WHERE user_id = $1
                ORDER BY created_at DESC
                ",
            )
            .bind(user_id.to_string())
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| AppError::database(format!("Failed to query user OAuth tokens: {e}")))?;

        let mut tokens = Vec::with_capacity(rows.len());
        for row in rows {
            tokens.push(self.row_to_user_oauth_token(&row)?);
        }
        Ok(tokens)
    }

    /// Get OAuth tokens for a tenant and provider
    ///
    /// Decrypts provider tokens using AAD binding for security.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query fails
    /// - Decryption fails for any token
    pub async fn get_tenant_provider_tokens(
        &self,
        tenant_id: TenantId,
        provider: &str,
    ) -> AppResult<Vec<UserOAuthToken>> {
        let rows = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, provider, access_token, refresh_token,
                   token_type, expires_at, scope, created_at, updated_at
            FROM user_oauth_tokens
            WHERE tenant_id = $1 AND provider = $2
            ORDER BY created_at DESC
            ",
        )
        .bind(tenant_id.to_string())
        .bind(provider)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to query tenant provider tokens: {e}")))?;

        let mut tokens = Vec::with_capacity(rows.len());
        for row in rows {
            tokens.push(self.row_to_user_oauth_token(&row)?);
        }
        Ok(tokens)
    }

    /// Delete a specific user OAuth token
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn delete_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        provider: &str,
    ) -> AppResult<()> {
        sqlx::query(
            r"
            DELETE FROM user_oauth_tokens
            WHERE user_id = $1 AND tenant_id = $2 AND provider = $3
            ",
        )
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .bind(provider)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to delete user OAuth token: {e}")))?;

        Ok(())
    }

    /// Delete all OAuth tokens for a user within a specific tenant scope
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn delete_user_oauth_tokens_impl(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> AppResult<()> {
        sqlx::query(
            r"
            DELETE FROM user_oauth_tokens
            WHERE user_id = $1 AND tenant_id = $2
            ",
        )
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to delete user OAuth tokens: {e}")))?;

        Ok(())
    }

    /// Refresh a user OAuth token
    ///
    /// Encrypts new tokens using AES-256-GCM with AAD binding.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Encryption fails
    /// - Database query fails
    pub async fn refresh_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        provider: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> AppResult<()> {
        // Create AAD context: tenant_id|user_id|provider|table
        let aad_context = format!("{tenant_id}|{user_id}|{provider}|user_oauth_tokens");

        // Encrypt new access token
        let encrypted_access_token = self.encrypt_data_with_aad(access_token, &aad_context)?;

        // Encrypt new refresh token if present
        let encrypted_refresh_token = refresh_token
            .map(|rt| self.encrypt_data_with_aad(rt, &aad_context))
            .transpose()?;

        sqlx::query(
            r"
            UPDATE user_oauth_tokens
            SET access_token = $4,
                refresh_token = $5,
                expires_at = $6,
                updated_at = $7
            WHERE user_id = $1 AND tenant_id = $2 AND provider = $3
            ",
        )
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .bind(provider)
        .bind(&encrypted_access_token)
        .bind(encrypted_refresh_token.as_deref())
        .bind(expires_at)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to refresh user OAuth token: {e}")))?;

        Ok(())
    }

    /// Convert a database row to a `UserOAuthToken`
    ///
    /// Decrypts provider tokens using AAD binding.
    ///
    /// # Errors
    ///
    /// Returns an error if decryption fails (possibly due to tampered data or AAD mismatch)
    fn row_to_user_oauth_token(&self, row: &SqliteRow) -> AppResult<UserOAuthToken> {
        let user_id_str: String = row.get("user_id");
        let user_id = Uuid::parse_str(&user_id_str)?;
        let tenant_id: String = row.get("tenant_id");
        let provider: String = row.get("provider");

        // Create AAD context: tenant_id|user_id|provider|table
        let aad_context = format!("{tenant_id}|{user_id}|{provider}|user_oauth_tokens");

        // Decrypt access token
        let encrypted_access_token: String = row.get("access_token");
        let access_token = self.decrypt_data_with_aad(&encrypted_access_token, &aad_context)?;

        // Decrypt refresh token if present
        let encrypted_refresh_token: Option<String> = row.get("refresh_token");
        let refresh_token = encrypted_refresh_token
            .as_deref()
            .map(|ert| self.decrypt_data_with_aad(ert, &aad_context))
            .transpose()?;

        Ok(UserOAuthToken {
            id: row.get("id"),
            user_id,
            tenant_id,
            provider,
            access_token,
            refresh_token,
            token_type: row.get("token_type"),
            expires_at: row.get("expires_at"),
            scope: row.get::<Option<String>, _>("scope"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
    // Public wrapper methods (delegate to _impl versions)

    /// Get user OAuth tokens, optionally scoped to a tenant (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_user_oauth_tokens(
        &self,
        user_id: Uuid,
        tenant_id: Option<TenantId>,
    ) -> AppResult<Vec<UserOAuthToken>> {
        self.get_user_oauth_tokens_impl(user_id, tenant_id).await
    }

    /// Delete user OAuth tokens within a tenant scope (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn delete_user_oauth_tokens(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> AppResult<()> {
        self.delete_user_oauth_tokens_impl(user_id, tenant_id).await
    }
}
