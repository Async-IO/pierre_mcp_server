// ABOUTME: Multi-tenant organization models for OAuth apps
// ABOUTME: Tenant, OAuthApp, OAuthAppParams, and AuthorizationCode definitions
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Tenant organization in multi-tenant setup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    /// Unique tenant identifier
    pub id: Uuid,
    /// Tenant organization name
    pub name: String,
    /// URL-safe slug for tenant
    pub slug: String,
    /// Custom domain for tenant (optional)
    pub domain: Option<String>,
    /// Subscription plan (basic, pro, enterprise)
    pub plan: String,
    /// User ID of the tenant owner
    pub owner_user_id: Uuid,
    /// When tenant was created
    pub created_at: DateTime<Utc>,
    /// When tenant was last updated
    pub updated_at: DateTime<Utc>,
}

impl Tenant {
    /// Creates a new tenant with the given details
    #[must_use]
    pub fn new(
        name: String,
        slug: String,
        domain: Option<String>,
        plan: String,
        owner_user_id: Uuid,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            slug,
            domain,
            plan,
            owner_user_id,
            created_at: now,
            updated_at: now,
        }
    }
}

/// OAuth application registration for MCP clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthApp {
    /// Unique app identifier
    pub id: Uuid,
    /// OAuth client ID
    pub client_id: String,
    /// OAuth client secret
    pub client_secret: String,
    /// Application name
    pub name: String,
    /// Application description
    pub description: Option<String>,
    /// Allowed redirect URIs
    pub redirect_uris: Vec<String>,
    /// Permitted scopes
    pub scopes: Vec<String>,
    /// Application type (desktop, web, mobile, server)
    pub app_type: String,
    /// User ID of the app owner
    pub owner_user_id: Uuid,
    /// When app was registered
    pub created_at: DateTime<Utc>,
    /// When app was last updated
    pub updated_at: DateTime<Utc>,
}

/// OAuth app creation parameters
pub struct OAuthAppParams {
    /// OAuth 2.0 client identifier
    pub client_id: String,
    /// OAuth 2.0 client secret for authentication
    pub client_secret: String,
    /// Human-readable name of the OAuth application
    pub name: String,
    /// Optional description of the application's purpose
    pub description: Option<String>,
    /// List of authorized redirect URIs for OAuth flow
    pub redirect_uris: Vec<String>,
    /// List of OAuth scopes the app can request
    pub scopes: Vec<String>,
    /// Type of OAuth application (e.g., "web", "native", "service")
    pub app_type: String,
    /// UUID of the user who owns this OAuth app
    pub owner_user_id: Uuid,
}

impl OAuthApp {
    /// Create new OAuth app from parameters
    #[must_use]
    pub fn new(params: OAuthAppParams) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            client_id: params.client_id,
            client_secret: params.client_secret,
            name: params.name,
            description: params.description,
            redirect_uris: params.redirect_uris,
            scopes: params.scopes,
            app_type: params.app_type,
            owner_user_id: params.owner_user_id,
            created_at: now,
            updated_at: now,
        }
    }
}

/// OAuth authorization code for token exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationCode {
    /// The authorization code
    pub code: String,
    /// Client ID that requested the code
    pub client_id: String,
    /// Redirect URI used in the request
    pub redirect_uri: String,
    /// Requested scopes
    pub scope: String,
    /// User ID that authorized the request
    pub user_id: Option<Uuid>,
    /// When the code expires
    pub expires_at: DateTime<Utc>,
    /// When the code was created
    pub created_at: DateTime<Utc>,
    /// Whether the code has been used
    pub is_used: bool,
}

impl AuthorizationCode {
    /// Creates a new authorization code with 10-minute expiration
    #[must_use]
    pub fn new(
        code: String,
        client_id: String,
        redirect_uri: String,
        scope: String,
        user_id: Option<Uuid>,
    ) -> Self {
        let now = Utc::now();
        Self {
            code,
            client_id,
            redirect_uri,
            scope,
            user_id,
            expires_at: now + chrono::Duration::minutes(10), // 10 minute expiration
            created_at: now,
            is_used: false,
        }
    }

    /// Check if the authorization code is expired
    #[must_use]
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if the authorization code is valid for use
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.is_used && !self.is_expired()
    }

    /// Mark the authorization code as used
    pub const fn mark_used(&mut self) {
        self.is_used = true;
    }
}
