// ABOUTME: HTTP route handlers for API key management and user self-service operations
// ABOUTME: Provides endpoints for trial key requests, API key status, and user API key management
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! HTTP routes for API key management

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    api_keys::{
        ApiKeyManager, ApiKeyTier, ApiKeyUsageStats, CreateApiKeyRequest, CreateApiKeyRequestSimple,
    },
    auth::AuthResult,
    database::repositories::{ApiKeyRepository, UsageRepository},
    errors::AppError,
    mcp::resources::ServerResources,
};

/// Response containing list of API keys for a user
#[derive(Debug, Serialize)]
pub struct ApiKeyListResponse {
    /// Array of API key information objects
    pub api_keys: Vec<ApiKeyInfo>,
}

/// API key information excluding the secret key value
#[derive(Debug, Serialize)]
pub struct ApiKeyInfo {
    /// Unique API key identifier
    pub id: String,
    /// User-provided name for the key
    pub name: String,
    /// Optional description of the key's purpose
    pub description: Option<String>,
    /// API key tier (trial, pro, enterprise)
    pub tier: ApiKeyTier,
    /// First 8 characters of the key for identification
    pub key_prefix: String,
    /// Whether the key is active
    pub is_active: bool,
    /// When the key was last used
    pub last_used_at: Option<DateTime<Utc>>,
    /// When the key expires (if applicable)
    pub expires_at: Option<DateTime<Utc>>,
    /// When the key was created
    pub created_at: DateTime<Utc>,
}

/// Response after creating a new API key
#[derive(Debug, Serialize)]
pub struct ApiKeyCreateResponse {
    /// The full API key (only shown once)
    pub api_key: String,
    /// Metadata about the created key
    pub key_info: ApiKeyInfo,
    /// Security warning to store the key safely
    pub warning: String,
}

/// Response containing API key usage statistics
#[derive(Debug, Serialize)]
pub struct ApiKeyUsageResponse {
    /// Usage statistics for the API key
    pub stats: ApiKeyUsageStats,
}

/// Response after deactivating an API key
#[derive(Debug, Serialize)]
pub struct ApiKeyDeactivateResponse {
    /// Success message
    pub message: String,
    /// When the key was deactivated
    pub deactivated_at: DateTime<Utc>,
}

/// API Key management routes
#[derive(Clone)]
pub struct ApiKeyRoutes {
    /// Server resources including database
    resources: std::sync::Arc<ServerResources>,
    /// API key management logic
    api_key_manager: ApiKeyManager,
}

impl ApiKeyRoutes {
    /// Create a new API key routes handler
    #[must_use]
    pub const fn new(resources: std::sync::Arc<ServerResources>) -> Self {
        Self {
            api_key_manager: ApiKeyManager::new(),
            resources,
        }
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
        auth: &AuthResult,
        request: CreateApiKeyRequestSimple,
    ) -> Result<ApiKeyCreateResponse> {
        let user_id = auth.user_id;

        // Create the API key
        let (api_key, full_key) = self
            .api_key_manager
            .create_api_key_simple(user_id, request)?;

        // Store in database
        self.resources.database.api_keys().create(&api_key).await?;

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
        auth: &AuthResult,
        request: CreateApiKeyRequest,
    ) -> Result<ApiKeyCreateResponse> {
        let user_id = auth.user_id;

        // Create the API key
        let (api_key, full_key) = self.api_key_manager.create_api_key(user_id, request)?;

        // Store in database
        self.resources.database.api_keys().create(&api_key).await?;

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
    pub async fn list_api_keys(&self, auth: &AuthResult) -> Result<ApiKeyListResponse> {
        let user_id = auth.user_id;

        let api_keys = self
            .resources
            .database
            .api_keys()
            .list_by_user(user_id)
            .await?;

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
        auth: &AuthResult,
        api_key_id: &str,
    ) -> Result<ApiKeyDeactivateResponse> {
        let user_id = auth.user_id;

        self.resources
            .database
            .api_keys()
            .deactivate(api_key_id, user_id)
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
        auth: &AuthResult,
        api_key_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<ApiKeyUsageResponse> {
        let user_id = auth.user_id;

        // Verify the API key belongs to the user
        let user_keys = self
            .resources
            .database
            .api_keys()
            .list_by_user(user_id)
            .await?;
        if !user_keys.iter().any(|key| key.id == api_key_id) {
            return Err(AppError::not_found("API key not found or access denied").into());
        }

        let stats = self
            .resources
            .database
            .usage()
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
        auth: &AuthResult,
        name: String,
        description: Option<String>,
    ) -> Result<ApiKeyCreateResponse> {
        let user_id = auth.user_id;

        // Check if user already has a trial key
        let existing_keys = self
            .resources
            .database
            .api_keys()
            .list_by_user(user_id)
            .await?;
        let has_trial_key = existing_keys.iter().any(|k| k.tier == ApiKeyTier::Trial);

        if has_trial_key {
            return Err(AppError::invalid_input("User already has a trial API key").into());
        }

        // Create the trial key
        let (api_key, full_key) =
            self.api_key_manager
                .create_trial_key(user_id, name, description)?;

        // Store in database
        self.resources.database.api_keys().create(&api_key).await?;

        Ok(ApiKeyCreateResponse {
            api_key: full_key,
            key_info: ApiKeyInfo {
                id: api_key.id.clone(), // Safe: String ownership for API key info struct
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
