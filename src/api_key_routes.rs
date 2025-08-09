// ABOUTME: HTTP route handlers for API key management and user self-service operations
// ABOUTME: Provides endpoints for trial key requests, API key status, and user API key management
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! HTTP routes for API key management

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    api_keys::{
        ApiKeyManager, ApiKeyTier, ApiKeyUsageStats, CreateApiKeyRequest, CreateApiKeyRequestSimple,
    },
    auth::AuthManager,
    database_plugins::{factory::Database, DatabaseProvider},
};

#[derive(Debug, Serialize)]
pub struct ApiKeyListResponse {
    pub api_keys: Vec<ApiKeyInfo>,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tier: ApiKeyTier,
    pub key_prefix: String,
    pub is_active: bool,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyCreateResponse {
    pub api_key: String,
    pub key_info: ApiKeyInfo,
    pub warning: String,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyUsageResponse {
    pub stats: ApiKeyUsageStats,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyDeactivateResponse {
    pub message: String,
    pub deactivated_at: DateTime<Utc>,
}

/// API Key management routes
#[derive(Clone)]
pub struct ApiKeyRoutes {
    database: Database,
    auth_manager: AuthManager,
    api_key_manager: ApiKeyManager,
}

impl ApiKeyRoutes {
    /// Create a new API key routes handler
    pub const fn new(database: Database, auth_manager: AuthManager) -> Self {
        Self {
            database,
            auth_manager,
            api_key_manager: ApiKeyManager::new(),
        }
    }

    /// Authenticate JWT token and extract user ID
    fn authenticate_user(&self, auth_header: Option<&str>) -> Result<Uuid> {
        let auth_str =
            auth_header.ok_or_else(|| anyhow::anyhow!("Missing authorization header"))?;

        let token = auth_str
            .strip_prefix("Bearer ")
            .ok_or_else(|| anyhow::anyhow!("Invalid authorization header format"))?;

        let claims = self.auth_manager.validate_token(token)?;
        let user_id = crate::utils::uuid::parse_uuid(&claims.sub)?;
        Ok(user_id)
    }

    /// Create a new API key with simplified rate limit approach
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication fails
    /// - Database operations fail
    /// - API key creation fails
    pub async fn create_api_key_simple(
        &self,
        auth_header: Option<&str>,
        request: CreateApiKeyRequestSimple,
    ) -> Result<ApiKeyCreateResponse> {
        let user_id = self.authenticate_user(auth_header)?;

        // Create the API key
        let (api_key, full_key) = self
            .api_key_manager
            .create_api_key_simple(user_id, request)?;

        // Store in database
        self.database.create_api_key(&api_key).await?;

        let key_info = ApiKeyInfo {
            id: api_key.id,
            name: api_key.name,
            description: api_key.description,
            tier: api_key.tier,
            key_prefix: api_key.key_prefix,
            is_active: api_key.is_active,
            last_used_at: api_key.last_used_at,
            expires_at: api_key.expires_at,
            created_at: api_key.created_at,
        };

        Ok(ApiKeyCreateResponse {
            api_key: full_key,
            key_info,
            warning: "Store this API key securely. It will not be shown again.".into(),
        })
    }

    /// Create a new API key (legacy method with tier)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication fails
    /// - Database operations fail
    /// - API key creation fails
    pub async fn create_api_key(
        &self,
        auth_header: Option<&str>,
        request: CreateApiKeyRequest,
    ) -> Result<ApiKeyCreateResponse> {
        let user_id = self.authenticate_user(auth_header)?;

        // Create the API key
        let (api_key, full_key) = self.api_key_manager.create_api_key(user_id, request)?;

        // Store in database
        self.database.create_api_key(&api_key).await?;

        let key_info = ApiKeyInfo {
            id: api_key.id,
            name: api_key.name,
            description: api_key.description,
            tier: api_key.tier,
            key_prefix: api_key.key_prefix,
            is_active: api_key.is_active,
            last_used_at: api_key.last_used_at,
            expires_at: api_key.expires_at,
            created_at: api_key.created_at,
        };

        Ok(ApiKeyCreateResponse {
            api_key: full_key,
            key_info,
            warning: "Store this API key securely. It will not be shown again.".into(),
        })
    }

    /// List user's API keys
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication fails
    /// - Database operations fail
    pub async fn list_api_keys(&self, auth_header: Option<&str>) -> Result<ApiKeyListResponse> {
        let user_id = self.authenticate_user(auth_header)?;

        let api_keys = self.database.get_user_api_keys(user_id).await?;

        let api_key_infos = api_keys
            .into_iter()
            .map(|key| ApiKeyInfo {
                id: key.id,
                name: key.name,
                description: key.description,
                tier: key.tier,
                key_prefix: key.key_prefix,
                is_active: key.is_active,
                last_used_at: key.last_used_at,
                expires_at: key.expires_at,
                created_at: key.created_at,
            })
            .collect();

        Ok(ApiKeyListResponse {
            api_keys: api_key_infos,
        })
    }

    /// Deactivate an API key
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication fails
    /// - Database operations fail
    /// - API key not found or not owned by user
    pub async fn deactivate_api_key(
        &self,
        auth_header: Option<&str>,
        api_key_id: &str,
    ) -> Result<ApiKeyDeactivateResponse> {
        let user_id = self.authenticate_user(auth_header)?;

        self.database
            .deactivate_api_key(api_key_id, user_id)
            .await?;

        Ok(ApiKeyDeactivateResponse {
            message: format!("API key {api_key_id} has been deactivated"),
            deactivated_at: Utc::now(),
        })
    }

    /// Get API key usage statistics
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication fails
    /// - Database operations fail
    /// - API key not found or not owned by user
    pub async fn get_api_key_usage(
        &self,
        auth_header: Option<&str>,
        api_key_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<ApiKeyUsageResponse> {
        let user_id = self.authenticate_user(auth_header)?;

        // Verify the API key belongs to the user
        let user_keys = self.database.get_user_api_keys(user_id).await?;
        if !user_keys.iter().any(|key| key.id == api_key_id) {
            return Err(anyhow::anyhow!("API key not found or access denied"));
        }

        let stats = self
            .database
            .get_api_key_usage_stats(api_key_id, start_date, end_date)
            .await?;

        Ok(ApiKeyUsageResponse { stats })
    }

    /// Create a trial API key with default settings
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication fails
    /// - User already has a trial key
    /// - Database operations fail
    /// - API key creation fails
    pub async fn create_trial_key(
        &self,
        auth_header: Option<&str>,
        name: String,
        description: Option<String>,
    ) -> Result<ApiKeyCreateResponse> {
        let user_id = self.authenticate_user(auth_header)?;

        // Check if user already has a trial key
        let existing_keys = self.database.get_user_api_keys(user_id).await?;
        let has_trial_key = existing_keys.iter().any(|k| k.tier == ApiKeyTier::Trial);

        if has_trial_key {
            return Err(anyhow::anyhow!("User already has a trial API key"));
        }

        // Create the trial key
        let (api_key, full_key) =
            self.api_key_manager
                .create_trial_key(user_id, name, description)?;

        // Store in database
        self.database.create_api_key(&api_key).await?;

        Ok(ApiKeyCreateResponse {
            api_key: full_key,
            key_info: ApiKeyInfo {
                id: api_key.id.clone(),
                name: api_key.name,
                description: api_key.description,
                tier: api_key.tier,
                key_prefix: api_key.key_prefix,
                is_active: api_key.is_active,
                last_used_at: api_key.last_used_at,
                expires_at: api_key.expires_at,
                created_at: api_key.created_at,
            },
            warning: format!(
                "This is a trial API key that will expire on {}. Store it securely - it cannot be recovered once lost.",
                api_key.expires_at.map_or_else(|| "N/A".into(), |d| d.format("%Y-%m-%d").to_string())
            ),
        })
    }
}
