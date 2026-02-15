// ABOUTME: HTTP REST endpoints for fitness configuration management with tenant isolation
// ABOUTME: Provides API access to tenant-specific fitness configurations with proper authentication
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::auth::AuthResult;
use crate::config::fitness::FitnessConfig;
use crate::database_plugins::DatabaseProvider;
use crate::errors::{AppError, AppResult};
use crate::mcp::resources::ServerResources;
use crate::middleware::require_admin;
use crate::models::TenantId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

// ================================================================================================
// Request/Response Models
// ================================================================================================

/// Request to save fitness configuration
#[derive(Debug, Deserialize)]
pub struct SaveFitnessConfigRequest {
    /// Configuration name (defaults to "default")
    pub configuration_name: Option<String>,
    /// Fitness configuration data
    pub configuration: FitnessConfig,
}

/// Request to retrieve a specific fitness configuration
#[derive(Debug, Deserialize)]
pub struct GetFitnessConfigRequest {
    /// Configuration name (defaults to "default")
    pub configuration_name: Option<String>,
}

/// Response containing fitness configuration details
#[derive(Debug, Serialize)]
pub struct FitnessConfigurationResponse {
    /// Configuration ID
    pub id: String,
    /// Tenant ID
    pub tenant_id: String,
    /// User ID (if user-specific, null for tenant-level)
    pub user_id: Option<String>,
    /// Configuration name
    pub configuration_name: String,
    /// Fitness configuration data
    pub configuration: FitnessConfig,
    /// Creation timestamp
    pub created_at: String,
    /// Last update timestamp
    pub updated_at: String,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

/// Response containing list of available fitness configurations
#[derive(Debug, Serialize)]
pub struct FitnessConfigurationListResponse {
    /// List of configuration names
    pub configurations: Vec<String>,
    /// Total count
    pub total_count: usize,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

/// Response confirming successful configuration save or delete operation
#[derive(Debug, Serialize)]
pub struct FitnessConfigurationSaveResponse {
    /// Configuration ID
    pub id: String,
    /// Success message
    pub message: String,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

/// Standard metadata included in all API responses
#[derive(Debug, Serialize)]
pub struct ResponseMetadata {
    /// Response timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Request processing time in milliseconds
    pub processing_time_ms: Option<u64>,
    /// API version
    pub api_version: String,
}

// ================================================================================================
// Route Handler
// ================================================================================================

/// Fitness configuration routes handler
#[derive(Clone)]
pub struct FitnessConfigurationRoutes {
    resources: Arc<ServerResources>,
}

impl FitnessConfigurationRoutes {
    /// Create a new fitness configuration routes handler
    #[must_use]
    pub const fn new(resources: Arc<ServerResources>) -> Self {
        Self { resources }
    }

    /// Authenticate JWT token and extract user ID
    ///
    /// Get tenant ID for authenticated user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User is not found in database
    /// - User has no tenant assigned
    async fn get_user_tenant(&self, user_id: Uuid) -> AppResult<TenantId> {
        // Verify user exists
        self.resources
            .database
            .get_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user {user_id}: {e}")))?
            .ok_or_else(|| AppError::not_found(format!("User {user_id}")))?;

        // Get tenant from tenant_users junction table
        let tenants = self
            .resources
            .database
            .list_tenants_for_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get tenants for user: {e}")))?;

        tenants
            .first()
            .map(|t| t.id)
            .ok_or_else(|| AppError::invalid_input(format!("User has no valid tenant: {user_id}")))
    }

    /// Create response metadata
    fn create_metadata(processing_start: Instant) -> ResponseMetadata {
        ResponseMetadata {
            timestamp: chrono::Utc::now(),
            processing_time_ms: u64::try_from(processing_start.elapsed().as_millis()).ok(),
            api_version: "1.0.0".into(),
        }
    }

    // ================================================================================================
    // Route Handlers
    // ================================================================================================

    /// GET /api/fitness-configurations - List all configuration names for user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User authentication fails
    /// - Database operations fail
    pub async fn list_configurations(
        &self,
        auth: &AuthResult,
    ) -> AppResult<FitnessConfigurationListResponse> {
        let processing_start = Instant::now();
        let user_id = auth.user_id;
        let tenant_id = self.get_user_tenant(user_id).await?;

        let user_id_str = user_id.to_string();

        // Get both user-specific and tenant-level configurations
        let mut configurations = self
            .resources
            .database
            .list_user_fitness_configurations(tenant_id, &user_id_str)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to list user fitness configurations: {e}"))
            })?;

        let tenant_configs = self
            .resources
            .database
            .list_tenant_fitness_configurations(tenant_id)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to list tenant fitness configurations: {e}"))
            })?;

        // Combine and deduplicate
        configurations.extend(tenant_configs);
        configurations.sort();
        configurations.dedup();

        Ok(FitnessConfigurationListResponse {
            total_count: configurations.len(),
            configurations,
            metadata: Self::create_metadata(processing_start),
        })
    }

    /// GET /api/fitness-configurations/{name} - Get specific configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User authentication fails
    /// - Configuration not found
    /// - Database operations fail
    pub async fn get_configuration(
        &self,
        auth: &AuthResult,
        configuration_name: &str,
    ) -> AppResult<FitnessConfigurationResponse> {
        let processing_start = Instant::now();
        let user_id = auth.user_id;
        let tenant_id = self.get_user_tenant(user_id).await?;

        let user_id_str = user_id.to_string();

        // Try user-specific first, then tenant-level, then default
        let config = match self
            .resources
            .database
            .get_user_fitness_config(tenant_id, &user_id_str, configuration_name)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user fitness config: {e}")))?
        {
            Some(config) => config,
            None => {
                // If user-specific config not found, try tenant-level
                self.resources
                    .database
                    .get_tenant_fitness_config(tenant_id, configuration_name)
                    .await
                    .map_err(|e| {
                        AppError::database(format!("Failed to get tenant fitness config: {e}"))
                    })?
                    .unwrap_or_default()
            }
        };

        // Return response with current timestamp since database schema doesn't store creation/update metadata
        Ok(FitnessConfigurationResponse {
            id: format!("{tenant_id}:{configuration_name}"),
            tenant_id: tenant_id.to_string(),
            user_id: Some(user_id.to_string()),
            configuration_name: configuration_name.to_owned(),
            configuration: config,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            metadata: Self::create_metadata(processing_start),
        })
    }

    /// POST /api/fitness-configurations - Save user-specific configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User authentication fails
    /// - Database operations fail
    /// - Configuration validation fails
    pub async fn save_user_configuration(
        &self,
        auth: &AuthResult,
        request: SaveFitnessConfigRequest,
    ) -> AppResult<FitnessConfigurationSaveResponse> {
        let processing_start = Instant::now();
        let user_id = auth.user_id;
        let tenant_id = self.get_user_tenant(user_id).await?;

        let configuration_name = request
            .configuration_name
            .unwrap_or_else(|| "default".to_owned());

        let user_id_str = user_id.to_string();

        let config_id = self
            .resources
            .database
            .save_user_fitness_config(
                tenant_id,
                &user_id_str,
                &configuration_name,
                &request.configuration,
            )
            .await
            .map_err(|e| AppError::database(format!("Failed to save user fitness config: {e}")))?;

        Ok(FitnessConfigurationSaveResponse {
            id: config_id,
            message: "User-specific fitness configuration saved successfully".to_owned(),
            metadata: Self::create_metadata(processing_start),
        })
    }

    /// POST /api/fitness-configurations/tenant - Save tenant-level configuration (admin only)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User authentication fails
    /// - User is not admin
    /// - Database operations fail
    /// - Configuration validation fails
    pub async fn save_tenant_configuration(
        &self,
        auth: &AuthResult,
        request: SaveFitnessConfigRequest,
    ) -> AppResult<FitnessConfigurationSaveResponse> {
        let processing_start = Instant::now();
        let user_id = auth.user_id;
        let tenant_id = self.get_user_tenant(user_id).await?;

        // Verify admin privileges using centralized guard
        require_admin(user_id, &self.resources.database).await?;

        let configuration_name = request
            .configuration_name
            .unwrap_or_else(|| "default".to_owned());

        let config_id = self
            .resources
            .database
            .save_tenant_fitness_config(tenant_id, &configuration_name, &request.configuration)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to save tenant fitness config: {e}"))
            })?;

        Ok(FitnessConfigurationSaveResponse {
            id: config_id,
            message: "Tenant-level fitness configuration saved successfully".to_owned(),
            metadata: Self::create_metadata(processing_start),
        })
    }

    /// DELETE /api/fitness-configurations/{name} - Delete user-specific configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User authentication fails
    /// - Database operations fail
    pub async fn delete_user_configuration(
        &self,
        auth: &AuthResult,
        configuration_name: &str,
    ) -> AppResult<FitnessConfigurationSaveResponse> {
        let processing_start = Instant::now();
        let user_id = auth.user_id;
        let tenant_id = self.get_user_tenant(user_id).await?;

        let user_id_str = user_id.to_string();

        let deleted = self
            .resources
            .database
            .delete_fitness_config(tenant_id, Some(&user_id_str), configuration_name)
            .await
            .map_err(|e| AppError::database(format!("Failed to delete fitness config: {e}")))?;

        if !deleted {
            return Err(AppError::not_found(format!(
                "Configuration {configuration_name}"
            )));
        }

        Ok(FitnessConfigurationSaveResponse {
            id: format!("{tenant_id}:{user_id}:{configuration_name}"),
            message: "User-specific fitness configuration deleted successfully".to_owned(),
            metadata: Self::create_metadata(processing_start),
        })
    }

    /// DELETE /api/fitness-configurations/tenant/{name} - Delete tenant-level configuration (admin only)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User authentication fails
    /// - User is not admin
    /// - Database operations fail
    pub async fn delete_tenant_configuration(
        &self,
        auth: &AuthResult,
        configuration_name: &str,
    ) -> AppResult<FitnessConfigurationSaveResponse> {
        let processing_start = Instant::now();
        let user_id = auth.user_id;
        let tenant_id = self.get_user_tenant(user_id).await?;

        // Verify admin privileges using centralized guard
        require_admin(user_id, &self.resources.database).await?;

        let deleted = self
            .resources
            .database
            .delete_fitness_config(tenant_id, None, configuration_name)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to delete tenant fitness config: {e}"))
            })?;

        if !deleted {
            return Err(AppError::not_found(format!(
                "Configuration {configuration_name}"
            )));
        }

        Ok(FitnessConfigurationSaveResponse {
            id: format!("{tenant_id}:{configuration_name}"),
            message: "Tenant-level fitness configuration deleted successfully".to_owned(),
            metadata: Self::create_metadata(processing_start),
        })
    }
}
