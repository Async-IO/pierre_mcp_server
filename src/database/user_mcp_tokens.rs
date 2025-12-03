// ABOUTME: User MCP token database operations for AI client authentication
// ABOUTME: Handles token creation, validation, listing, and revocation
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::Database;
use crate::errors::{AppError, AppResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

/// User MCP token for AI client authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMcpToken {
    /// Unique token ID
    pub id: String,
    /// Owner user ID
    pub user_id: Uuid,
    /// Human-readable name for the token
    pub name: String,
    /// SHA-256 hash of the full token
    pub token_hash: String,
    /// First 8 characters of the token for identification
    pub token_prefix: String,
    /// Optional expiration timestamp
    pub expires_at: Option<DateTime<Utc>>,
    /// Last time the token was used
    pub last_used_at: Option<DateTime<Utc>>,
    /// Number of times the token has been used
    pub usage_count: u32,
    /// Whether the token has been revoked
    pub is_revoked: bool,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Response when creating a new token (includes the actual token value)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMcpTokenCreated {
    /// Token metadata
    pub token: UserMcpToken,
    /// The actual token value (only returned once at creation)
    pub token_value: String,
}

/// Request to create a new MCP token
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserMcpTokenRequest {
    /// Human-readable name for the token
    pub name: String,
    /// Days until expiration (None for never expires)
    pub expires_in_days: Option<u32>,
}

/// Response for listing tokens (excludes sensitive data)
#[derive(Debug, Clone, Serialize)]
pub struct UserMcpTokenInfo {
    /// Unique token ID
    pub id: String,
    /// Human-readable name for the token
    pub name: String,
    /// First 8 characters of the token for identification
    pub token_prefix: String,
    /// Optional expiration timestamp
    pub expires_at: Option<DateTime<Utc>>,
    /// Last time the token was used
    pub last_used_at: Option<DateTime<Utc>>,
    /// Number of times the token has been used
    pub usage_count: u32,
    /// Whether the token has been revoked
    pub is_revoked: bool,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl From<UserMcpToken> for UserMcpTokenInfo {
    fn from(token: UserMcpToken) -> Self {
        Self {
            id: token.id,
            name: token.name,
            token_prefix: token.token_prefix,
            expires_at: token.expires_at,
            last_used_at: token.last_used_at,
            usage_count: token.usage_count,
            is_revoked: token.is_revoked,
            created_at: token.created_at,
        }
    }
}

impl Database {
    /// Generate a new MCP token with secure random bytes
    fn generate_mcp_token() -> String {
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 32];
        rng.fill_bytes(&mut bytes);
        format!(
            "pmcp_{}",
            base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
        )
    }

    /// Hash a token for storage
    fn hash_token(token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Create a new user MCP token
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn create_user_mcp_token(
        &self,
        user_id: Uuid,
        request: &CreateUserMcpTokenRequest,
    ) -> AppResult<UserMcpTokenCreated> {
        let token_value = Self::generate_mcp_token();
        let token_hash = Self::hash_token(&token_value);
        let token_prefix = token_value.chars().take(12).collect::<String>();
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let expires_at = request
            .expires_in_days
            .map(|days| now + chrono::Duration::days(i64::from(days)));

        sqlx::query(
            r"
            INSERT INTO user_mcp_tokens (
                id, user_id, name, token_hash, token_prefix,
                expires_at, last_used_at, usage_count, is_revoked, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, NULL, 0, 0, $7)
            ",
        )
        .bind(&id)
        .bind(user_id.to_string())
        .bind(&request.name)
        .bind(&token_hash)
        .bind(&token_prefix)
        .bind(expires_at)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create user MCP token: {e}")))?;

        let token = UserMcpToken {
            id,
            user_id,
            name: request.name.clone(),
            token_hash,
            token_prefix,
            expires_at,
            last_used_at: None,
            usage_count: 0,
            is_revoked: false,
            created_at: now,
        };

        Ok(UserMcpTokenCreated { token, token_value })
    }

    /// Validate a user MCP token and return the associated user ID
    ///
    /// # Errors
    /// Returns an error if the token is invalid, expired, or revoked
    pub async fn validate_user_mcp_token(&self, token_value: &str) -> AppResult<Uuid> {
        let token_hash = Self::hash_token(token_value);
        let token_prefix = token_value.chars().take(12).collect::<String>();

        let row = sqlx::query(
            r"
            SELECT id, user_id, expires_at, is_revoked
            FROM user_mcp_tokens
            WHERE token_prefix = $1 AND token_hash = $2
            ",
        )
        .bind(&token_prefix)
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to validate user MCP token: {e}")))?;

        let row = row.ok_or_else(|| AppError::auth_invalid("Invalid MCP token"))?;

        let is_revoked: bool = row.get("is_revoked");
        if is_revoked {
            return Err(AppError::auth_invalid("MCP token has been revoked"));
        }

        let expires_at: Option<DateTime<Utc>> = row.get("expires_at");
        if let Some(exp) = expires_at {
            if exp < Utc::now() {
                return Err(AppError::auth_invalid("MCP token has expired"));
            }
        }

        let token_id: String = row.get("id");
        self.update_user_mcp_token_usage(&token_id).await?;

        let user_id_str: String = row.get("user_id");
        Uuid::parse_str(&user_id_str)
            .map_err(|e| AppError::internal(format!("Failed to parse user_id UUID: {e}")))
    }

    /// Update token usage statistics
    async fn update_user_mcp_token_usage(&self, token_id: &str) -> AppResult<()> {
        sqlx::query(
            r"
            UPDATE user_mcp_tokens
            SET last_used_at = $1, usage_count = usage_count + 1
            WHERE id = $2
            ",
        )
        .bind(Utc::now())
        .bind(token_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update user MCP token usage: {e}")))?;

        Ok(())
    }

    /// List all MCP tokens for a user
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn list_user_mcp_tokens(&self, user_id: Uuid) -> AppResult<Vec<UserMcpTokenInfo>> {
        let rows = sqlx::query(
            r"
            SELECT id, name, token_prefix, expires_at, last_used_at,
                   usage_count, is_revoked, created_at
            FROM user_mcp_tokens
            WHERE user_id = $1
            ORDER BY created_at DESC
            ",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to list user MCP tokens: {e}")))?;

        rows.iter()
            .map(|row| {
                Ok(UserMcpTokenInfo {
                    id: row.get("id"),
                    name: row.get("name"),
                    token_prefix: row.get("token_prefix"),
                    expires_at: row.get("expires_at"),
                    last_used_at: row.get("last_used_at"),
                    usage_count: u32::try_from(row.get::<i32, _>("usage_count")).map_err(|e| {
                        AppError::internal(format!(
                            "Integer conversion failed for usage_count: {e}"
                        ))
                    })?,
                    is_revoked: row.get("is_revoked"),
                    created_at: row.get("created_at"),
                })
            })
            .collect()
    }

    /// Revoke a user MCP token
    ///
    /// # Errors
    /// Returns an error if the token doesn't exist or database operation fails
    pub async fn revoke_user_mcp_token(&self, token_id: &str, user_id: Uuid) -> AppResult<()> {
        let result = sqlx::query(
            r"
            UPDATE user_mcp_tokens
            SET is_revoked = 1
            WHERE id = $1 AND user_id = $2
            ",
        )
        .bind(token_id)
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to revoke user MCP token: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found("MCP token not found or unauthorized"));
        }

        Ok(())
    }

    /// Get a user MCP token by ID
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn get_user_mcp_token(
        &self,
        token_id: &str,
        user_id: Uuid,
    ) -> AppResult<Option<UserMcpToken>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, name, token_hash, token_prefix,
                   expires_at, last_used_at, usage_count, is_revoked, created_at
            FROM user_mcp_tokens
            WHERE id = $1 AND user_id = $2
            ",
        )
        .bind(token_id)
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get user MCP token: {e}")))?;

        row.map(|r| Self::row_to_user_mcp_token(&r)).transpose()
    }

    /// Convert database row to `UserMcpToken`
    fn row_to_user_mcp_token(row: &sqlx::sqlite::SqliteRow) -> AppResult<UserMcpToken> {
        Ok(UserMcpToken {
            id: row.get("id"),
            user_id: Uuid::parse_str(row.get::<String, _>("user_id").as_str())
                .map_err(|e| AppError::internal(format!("Failed to parse user_id UUID: {e}")))?,
            name: row.get("name"),
            token_hash: row.get("token_hash"),
            token_prefix: row.get("token_prefix"),
            expires_at: row.get("expires_at"),
            last_used_at: row.get("last_used_at"),
            usage_count: u32::try_from(row.get::<i32, _>("usage_count")).map_err(|e| {
                AppError::internal(format!("Integer conversion failed for usage_count: {e}"))
            })?,
            is_revoked: row.get("is_revoked"),
            created_at: row.get("created_at"),
        })
    }

    /// Delete expired tokens (for cleanup)
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn cleanup_expired_user_mcp_tokens(&self) -> AppResult<u64> {
        let result = sqlx::query(
            r"
            UPDATE user_mcp_tokens
            SET is_revoked = 1
            WHERE expires_at IS NOT NULL
            AND expires_at < $1
            AND is_revoked = 0
            ",
        )
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::database(format!("Failed to cleanup expired user MCP tokens: {e}"))
        })?;

        Ok(result.rows_affected())
    }
}
