// ABOUTME: HTTP REST endpoints for fitness configuration management with tenant isolation
// ABOUTME: Provides API access to tenant-specific fitness configurations with proper authentication

use crate::config::fitness_config::FitnessConfig;
use crate::database_plugins::DatabaseProvider;
use crate::utils::auth::extract_bearer_token_from_option;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

// ================================================================================================
// Request/Response Models
// ================================================================================================

#[derive(Debug, Deserialize)]
pub struct SaveFitnessConfigRequest {
    /// Configuration name (defaults to "default")
    pub configuration_name: Option<String>,
    /// Fitness configuration data
    pub configuration: FitnessConfig,
}

#[derive(Debug, Deserialize)]
pub struct GetFitnessConfigRequest {
    /// Configuration name (defaults to "default")
    pub configuration_name: Option<String>,
}

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

#[derive(Debug, Serialize)]
pub struct FitnessConfigurationListResponse {
    /// List of configuration names
    pub configurations: Vec<String>,
    /// Total count
    pub total_count: usize,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

#[derive(Debug, Serialize)]
pub struct FitnessConfigurationSaveResponse {
    /// Configuration ID
    pub id: String,
    /// Success message
    pub message: String,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

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
    resources: Arc<crate::mcp::resources::ServerResources>,
}

impl FitnessConfigurationRoutes {
    /// Create a new fitness configuration routes handler
    #[must_use]
    pub const fn new(resources: Arc<crate::mcp::resources::ServerResources>) -> Self {
        Self { resources }
    }

    /// Authenticate JWT token and extract user ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The authorization header is missing
    /// - The authorization header format is invalid
    /// - The token validation fails
    /// - The user ID cannot be parsed as a UUID
    fn authenticate_user(&self, auth_header: Option<&str>) -> Result<Uuid> {
        let auth_str =
            auth_header.ok_or_else(|| anyhow::anyhow!("Missing authorization header"))?;

        let token = extract_bearer_token_from_option(Some(auth_str))?;

        let claims = self.resources.auth_manager.validate_token(token)?;
        let user_id = crate::utils::uuid::parse_uuid(&claims.sub)?;
        Ok(user_id)
    }

    /// Get tenant ID for authenticated user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User is not found in database
    /// - User has no tenant assigned
    async fn get_user_tenant(&self, user_id: Uuid) -> Result<Uuid> {
        let user = self
            .resources
            .database
            .get_user(user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found: {}", user_id))?;

        let tenant_id = user
            .tenant_id
            .as_ref()
            .and_then(|id| Uuid::parse_str(id).ok())
            .ok_or_else(|| anyhow::anyhow!("User has no valid tenant: {}", user_id))?;

        Ok(tenant_id)
    }

    /// Create response metadata
    fn create_metadata(processing_start: std::time::Instant) -> ResponseMetadata {
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
        auth_header: Option<&str>,
    ) -> Result<FitnessConfigurationListResponse> {
        let processing_start = std::time::Instant::now();
        let user_id = self.authenticate_user(auth_header)?;
        let tenant_id = self.get_user_tenant(user_id).await?;

        // Get both user-specific and tenant-level configurations
        let mut configurations = self
            .resources
            .database
            .list_user_fitness_configurations(&tenant_id.to_string(), &user_id.to_string())
            .await?;

        let tenant_configs = self
            .resources
            .database
            .list_tenant_fitness_configurations(&tenant_id.to_string())
            .await?;

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
        auth_header: Option<&str>,
        configuration_name: &str,
    ) -> Result<FitnessConfigurationResponse> {
        let processing_start = std::time::Instant::now();
        let user_id = self.authenticate_user(auth_header)?;
        let tenant_id = self.get_user_tenant(user_id).await?;

        // Try user-specific first, then tenant-level
        let config = match self
            .resources
            .database
            .get_user_fitness_config(
                &tenant_id.to_string(),
                &user_id.to_string(),
                configuration_name,
            )
            .await?
        {
            Some(config) => config,
            None => {
                // If user-specific config not found, try tenant-level
                self.resources
                    .database
                    .get_tenant_fitness_config(&tenant_id.to_string(), configuration_name)
                    .await?
                    .ok_or_else(|| {
                        anyhow::anyhow!("Configuration not found: {}", configuration_name)
                    })?
            }
        };

        // Return response with current timestamp since database schema doesn't store creation/update metadata
        Ok(FitnessConfigurationResponse {
            id: format!("{tenant_id}:{configuration_name}"),
            tenant_id: tenant_id.to_string(),
            user_id: Some(user_id.to_string()),
            configuration_name: configuration_name.to_string(),
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
        auth_header: Option<&str>,
        request: SaveFitnessConfigRequest,
    ) -> Result<FitnessConfigurationSaveResponse> {
        let processing_start = std::time::Instant::now();
        let user_id = self.authenticate_user(auth_header)?;
        let tenant_id = self.get_user_tenant(user_id).await?;

        let configuration_name = request
            .configuration_name
            .unwrap_or_else(|| "default".to_string());

        let config_id = self
            .resources
            .database
            .save_user_fitness_config(
                &tenant_id.to_string(),
                &user_id.to_string(),
                &configuration_name,
                &request.configuration,
            )
            .await?;

        Ok(FitnessConfigurationSaveResponse {
            id: config_id,
            message: "User-specific fitness configuration saved successfully".to_string(),
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
        auth_header: Option<&str>,
        request: SaveFitnessConfigRequest,
    ) -> Result<FitnessConfigurationSaveResponse> {
        let processing_start = std::time::Instant::now();
        let user_id = self.authenticate_user(auth_header)?;
        let tenant_id = self.get_user_tenant(user_id).await?;

        // Check if user is admin (simplified check)
        let user = self
            .resources
            .database
            .get_user(user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;

        if !user.is_admin {
            return Err(anyhow::anyhow!("Admin access required"));
        }

        let configuration_name = request
            .configuration_name
            .unwrap_or_else(|| "default".to_string());

        let config_id = self
            .resources
            .database
            .save_tenant_fitness_config(
                &tenant_id.to_string(),
                &configuration_name,
                &request.configuration,
            )
            .await?;

        Ok(FitnessConfigurationSaveResponse {
            id: config_id,
            message: "Tenant-level fitness configuration saved successfully".to_string(),
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
        auth_header: Option<&str>,
        configuration_name: &str,
    ) -> Result<FitnessConfigurationSaveResponse> {
        let processing_start = std::time::Instant::now();
        let user_id = self.authenticate_user(auth_header)?;
        let tenant_id = self.get_user_tenant(user_id).await?;

        let deleted = self
            .resources
            .database
            .delete_fitness_config(
                &tenant_id.to_string(),
                Some(&user_id.to_string()),
                configuration_name,
            )
            .await?;

        if !deleted {
            return Err(anyhow::anyhow!(
                "Configuration not found: {}",
                configuration_name
            ));
        }

        Ok(FitnessConfigurationSaveResponse {
            id: format!("{tenant_id}:{user_id}:{configuration_name}"),
            message: "User-specific fitness configuration deleted successfully".to_string(),
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
        auth_header: Option<&str>,
        configuration_name: &str,
    ) -> Result<FitnessConfigurationSaveResponse> {
        let processing_start = std::time::Instant::now();
        let user_id = self.authenticate_user(auth_header)?;
        let tenant_id = self.get_user_tenant(user_id).await?;

        // Check if user is admin
        let user = self
            .resources
            .database
            .get_user(user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;

        if !user.is_admin {
            return Err(anyhow::anyhow!("Admin access required"));
        }

        let deleted = self
            .resources
            .database
            .delete_fitness_config(&tenant_id.to_string(), None, configuration_name)
            .await?;

        if !deleted {
            return Err(anyhow::anyhow!(
                "Configuration not found: {}",
                configuration_name
            ));
        }

        Ok(FitnessConfigurationSaveResponse {
            id: format!("{tenant_id}:{configuration_name}"),
            message: "Tenant-level fitness configuration deleted successfully".to_string(),
            metadata: Self::create_metadata(processing_start),
        })
    }
}
