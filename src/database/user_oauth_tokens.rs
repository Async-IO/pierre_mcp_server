// ABOUTME: UserOAuthToken database operations for per-user, per-tenant OAuth credential storage
// ABOUTME: Handles tenant-aware OAuth token management for multi-tenant architecture
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::Database;
use crate::models::UserOAuthToken;
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::Row;
use uuid::Uuid;

/// OAuth token data for database operations
pub struct OAuthTokenData<'a> {
    /// Unique token identifier
    pub id: &'a str,
    /// User ID this token belongs to
    pub user_id: Uuid,
    /// Tenant ID for multi-tenant isolation
    pub tenant_id: &'a str,
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
    /// Create `user_oauth_tokens` table
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The database schema migration fails
    /// - Table creation fails
    /// - Index creation fails
    pub(super) async fn migrate_user_oauth_tokens(&self) -> Result<()> {
        // Create user_oauth_tokens table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS user_oauth_tokens (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                tenant_id TEXT NOT NULL,
                provider TEXT NOT NULL,
                access_token TEXT NOT NULL,
                refresh_token TEXT,
                token_type TEXT NOT NULL DEFAULT 'bearer',
                expires_at DATETIME,
                scope TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(user_id, tenant_id, provider)
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create indexes
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_user_oauth_tokens_user ON user_oauth_tokens(user_id)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_user_oauth_tokens_tenant_provider ON user_oauth_tokens(tenant_id, provider)")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

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
    pub async fn upsert_user_oauth_token(&self, token_data: &OAuthTokenData<'_>) -> Result<()> {
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
        .bind(token_data.tenant_id)
        .bind(token_data.provider)
        .bind(&encrypted_access_token)
        .bind(encrypted_refresh_token.as_deref())
        .bind(token_data.token_type)
        .bind(token_data.expires_at)
        .bind(token_data.scope)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;

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
        tenant_id: &str,
        provider: &str,
    ) -> Result<Option<UserOAuthToken>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, provider, access_token, refresh_token,
                   token_type, expires_at, scope, created_at, updated_at
            FROM user_oauth_tokens
            WHERE user_id = $1 AND tenant_id = $2 AND provider = $3
            ",
        )
        .bind(user_id.to_string())
        .bind(tenant_id)
        .bind(provider)
        .fetch_optional(&self.pool)
        .await?;

        row.map_or_else(
            || Ok(None),
            |row| Ok(Some(self.row_to_user_oauth_token(&row)?)),
        )
    }

    /// Get all OAuth tokens for a user
    ///
    /// Decrypts provider tokens using AAD binding for security.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query fails
    /// - Decryption fails for any token
    pub async fn get_user_oauth_tokens(&self, user_id: Uuid) -> Result<Vec<UserOAuthToken>> {
        let rows = sqlx::query(
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
        .await?;

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
        tenant_id: &str,
        provider: &str,
    ) -> Result<Vec<UserOAuthToken>> {
        let rows = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, provider, access_token, refresh_token,
                   token_type, expires_at, scope, created_at, updated_at
            FROM user_oauth_tokens
            WHERE tenant_id = $1 AND provider = $2
            ORDER BY created_at DESC
            ",
        )
        .bind(tenant_id)
        .bind(provider)
        .fetch_all(&self.pool)
        .await?;

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
        tenant_id: &str,
        provider: &str,
    ) -> Result<()> {
        sqlx::query(
            r"
            DELETE FROM user_oauth_tokens
            WHERE user_id = $1 AND tenant_id = $2 AND provider = $3
            ",
        )
        .bind(user_id.to_string())
        .bind(tenant_id)
        .bind(provider)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete all OAuth tokens for a user
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn delete_user_oauth_tokens(&self, user_id: Uuid) -> Result<()> {
        sqlx::query(
            r"
            DELETE FROM user_oauth_tokens
            WHERE user_id = $1
            ",
        )
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;

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
        tenant_id: &str,
        provider: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
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
        .bind(tenant_id)
        .bind(provider)
        .bind(&encrypted_access_token)
        .bind(encrypted_refresh_token.as_deref())
        .bind(expires_at)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Convert a database row to a `UserOAuthToken`
    ///
    /// Decrypts provider tokens using AAD binding.
    ///
    /// # Errors
    ///
    /// Returns an error if decryption fails (possibly due to tampered data or AAD mismatch)
    fn row_to_user_oauth_token(&self, row: &sqlx::sqlite::SqliteRow) -> Result<UserOAuthToken> {
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
}
