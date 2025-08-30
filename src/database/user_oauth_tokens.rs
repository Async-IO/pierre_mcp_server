// ABOUTME: UserOAuthToken database operations for per-user, per-tenant OAuth credential storage
// ABOUTME: Handles tenant-aware OAuth token management for multi-tenant architecture

use super::Database;
use crate::models::UserOAuthToken;
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::Row;
use uuid::Uuid;

/// OAuth token data for database operations
pub struct OAuthTokenData<'a> {
    pub id: &'a str,
    pub user_id: Uuid,
    pub tenant_id: &'a str,
    pub provider: &'a str,
    pub access_token: &'a str,
    pub refresh_token: Option<&'a str>,
    pub token_type: &'a str,
    pub expires_at: Option<DateTime<Utc>>,
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
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn upsert_user_oauth_token(&self, token_data: &OAuthTokenData<'_>) -> Result<()> {
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
        .bind(token_data.access_token)
        .bind(token_data.refresh_token)
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
    /// # Errors
    ///
    /// Returns an error if the database query fails
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
            |row| Ok(Some(Self::row_to_user_oauth_token(&row)?)),
        )
    }

    /// Get all OAuth tokens for a user
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
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
            tokens.push(Self::row_to_user_oauth_token(&row)?);
        }
        Ok(tokens)
    }

    /// Get OAuth tokens for a tenant and provider
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
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
            tokens.push(Self::row_to_user_oauth_token(&row)?);
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
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn refresh_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
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
        .bind(access_token)
        .bind(refresh_token)
        .bind(expires_at)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Convert a database row to a `UserOAuthToken`
    fn row_to_user_oauth_token(row: &sqlx::sqlite::SqliteRow) -> Result<UserOAuthToken> {
        let user_id_str: String = row.get("user_id");

        Ok(UserOAuthToken {
            id: row.get("id"),
            user_id: Uuid::parse_str(&user_id_str)?,
            tenant_id: row.get("tenant_id"),
            provider: row.get("provider"),
            access_token: row.get("access_token"),
            refresh_token: row.get("refresh_token"),
            token_type: row.get("token_type"),
            expires_at: row.get("expires_at"),
            scope: row.get::<Option<String>, _>("scope"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
}
