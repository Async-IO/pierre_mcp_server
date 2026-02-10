// ABOUTME: User MCP token types for AI client authentication
// ABOUTME: Token metadata, creation request, and info types used by DatabaseProvider
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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
