// ABOUTME: Configuration service with caching and hot reload support
// ABOUTME: Provides runtime configuration access with database override resolution
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::manager::AdminConfigManager;
use super::types::{
    AdminConfigCategory, AdminConfigParameter, ConfigAuditFilter, ConfigCatalogResponse,
    ConfigDataType, ConfigOverride, ConfigValidationError, ParameterRange, ResetConfigRequest,
    ResetConfigResponse, UpdateConfigRequest, UpdateConfigResponse, ValidateConfigRequest,
    ValidateConfigResponse,
};
use crate::errors::{AppError, AppResult};
use chrono::Utc;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Default configuration definitions with metadata
/// This struct holds the canonical parameter definitions loaded at startup
#[derive(Debug, Clone)]
pub struct ParameterDefinition {
    /// Unique identifier for the parameter (e.g., `rate_limit.free_tier_burst`)
    pub key: String,
    /// Human-readable name for display in UI
    pub display_name: String,
    /// Detailed description of what this parameter controls
    pub description: String,
    /// Category grouping for organization (e.g., `rate_limiting`, `algorithms`)
    pub category: String,
    /// Data type for validation and UI rendering
    pub data_type: ConfigDataType,
    /// Default value when no override is set
    pub default_value: serde_json::Value,
    /// Optional numeric range constraints for validation
    pub valid_range: Option<ParameterRange>,
    /// Optional list of valid enum values
    pub enum_options: Option<Vec<String>>,
    /// Unit of measurement for display (e.g., "requests", "km", "% max HR")
    pub units: Option<String>,
    /// Scientific or research basis for the default value
    pub scientific_basis: Option<String>,
    /// Environment variable name if this can be set via env
    pub env_variable: Option<String>,
    /// Whether this can be changed at runtime without restart
    pub is_runtime_configurable: bool,
    /// Whether changing this parameter requires a server restart
    pub requires_restart: bool,
}

/// Admin configuration service for managing runtime configuration
pub struct AdminConfigService {
    manager: AdminConfigManager,
    /// Cached parameter definitions (loaded at startup)
    definitions: Arc<RwLock<HashMap<String, ParameterDefinition>>>,
    /// Cached effective values (refreshed on changes)
    cache: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    /// Category metadata
    categories: Arc<RwLock<Vec<AdminConfigCategory>>>,
}

impl AdminConfigService {
    /// Create a new admin config service
    ///
    /// # Errors
    ///
    /// Returns an error if the initial cache refresh fails.
    pub async fn new(pool: SqlitePool) -> AppResult<Self> {
        let manager = AdminConfigManager::new(pool);

        // Load categories from database
        let categories = manager.get_categories().await.unwrap_or_default();

        let service = Self {
            manager,
            definitions: Arc::new(RwLock::new(HashMap::new())),
            cache: Arc::new(RwLock::new(HashMap::new())),
            categories: Arc::new(RwLock::new(categories)),
        };

        // Initialize parameter definitions
        service.initialize_definitions().await;

        // Load overrides into cache
        service.refresh_cache(None).await?;

        Ok(service)
    }

    /// Initialize parameter definitions with all configurable parameters
    #[allow(clippy::too_many_lines)]
    async fn initialize_definitions(&self) {
        // Build definitions locally first, then acquire lock briefly at the end
        let mut defs = HashMap::new();

        // Rate Limiting Parameters
        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "rate_limit.free_tier_burst".to_owned(),
                display_name: "Free Tier Burst Limit".to_owned(),
                description: "Maximum burst requests for free tier users".to_owned(),
                category: "rate_limiting".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(10),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(1),
                    max: serde_json::json!(100),
                    step: Some(1.0),
                }),
                enum_options: None,
                units: Some("requests".to_owned()),
                scientific_basis: None,
                env_variable: Some("RATE_LIMIT_FREE_TIER_BURST".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "rate_limit.professional_burst".to_owned(),
                display_name: "Professional Tier Burst Limit".to_owned(),
                description: "Maximum burst requests for professional tier users".to_owned(),
                category: "rate_limiting".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(50),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(10),
                    max: serde_json::json!(500),
                    step: Some(1.0),
                }),
                enum_options: None,
                units: Some("requests".to_owned()),
                scientific_basis: None,
                env_variable: Some("RATE_LIMIT_PROFESSIONAL_BURST".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "rate_limit.enterprise_burst".to_owned(),
                display_name: "Enterprise Tier Burst Limit".to_owned(),
                description: "Maximum burst requests for enterprise tier users".to_owned(),
                category: "rate_limiting".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(100),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(50),
                    max: serde_json::json!(10000),
                    step: Some(1.0),
                }),
                enum_options: None,
                units: Some("requests".to_owned()),
                scientific_basis: None,
                env_variable: Some("RATE_LIMIT_ENTERPRISE_BURST".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        // Feature Flags
        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "feature.auto_approval_enabled".to_owned(),
                display_name: "Auto-Approve New Users".to_owned(),
                description:
                    "Automatically approve new user registrations without admin intervention"
                        .to_owned(),
                category: "feature_flags".to_owned(),
                data_type: ConfigDataType::Boolean,
                default_value: serde_json::json!(false),
                valid_range: None,
                enum_options: None,
                units: None,
                scientific_basis: None,
                env_variable: None,
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "feature.weather_api_enabled".to_owned(),
                display_name: "Weather API Integration".to_owned(),
                description: "Enable weather data integration for activity analysis".to_owned(),
                category: "feature_flags".to_owned(),
                data_type: ConfigDataType::Boolean,
                default_value: serde_json::json!(true),
                valid_range: None,
                enum_options: None,
                units: None,
                scientific_basis: None,
                env_variable: Some("WEATHER_SERVICE_ENABLED".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        // Heart Rate Zones
        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "heart_rate.recovery_zone".to_owned(),
                display_name: "Recovery Zone Max".to_owned(),
                description: "Maximum heart rate percentage for recovery zone (Zone 1)".to_owned(),
                category: "heart_rate_zones".to_owned(),
                data_type: ConfigDataType::Float,
                default_value: serde_json::json!(60.0),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(50.0),
                    max: serde_json::json!(70.0),
                    step: Some(0.5),
                }),
                enum_options: None,
                units: Some("% max HR".to_owned()),
                scientific_basis: Some("Polarized Training Model".to_owned()),
                env_variable: Some("FITNESS_ZONE_RECOVERY_MAX".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "heart_rate.endurance_zone".to_owned(),
                display_name: "Endurance Zone Max".to_owned(),
                description: "Maximum heart rate percentage for endurance zone (Zone 2)".to_owned(),
                category: "heart_rate_zones".to_owned(),
                data_type: ConfigDataType::Float,
                default_value: serde_json::json!(70.0),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(60.0),
                    max: serde_json::json!(80.0),
                    step: Some(0.5),
                }),
                enum_options: None,
                units: Some("% max HR".to_owned()),
                scientific_basis: Some("Maffetone Method".to_owned()),
                env_variable: Some("FITNESS_ZONE_ENDURANCE_MAX".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "heart_rate.tempo_zone".to_owned(),
                display_name: "Tempo Zone Max".to_owned(),
                description: "Maximum heart rate percentage for tempo zone (Zone 3)".to_owned(),
                category: "heart_rate_zones".to_owned(),
                data_type: ConfigDataType::Float,
                default_value: serde_json::json!(80.0),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(70.0),
                    max: serde_json::json!(90.0),
                    step: Some(0.5),
                }),
                enum_options: None,
                units: Some("% max HR".to_owned()),
                scientific_basis: Some("Coggan & Allen 2006".to_owned()),
                env_variable: Some("FITNESS_ZONE_TEMPO_MAX".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "heart_rate.threshold_zone".to_owned(),
                display_name: "Threshold Zone Max".to_owned(),
                description: "Maximum heart rate percentage for threshold zone (Zone 4)".to_owned(),
                category: "heart_rate_zones".to_owned(),
                data_type: ConfigDataType::Float,
                default_value: serde_json::json!(90.0),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(80.0),
                    max: serde_json::json!(95.0),
                    step: Some(0.5),
                }),
                enum_options: None,
                units: Some("% max HR".to_owned()),
                scientific_basis: Some("Seiler 2010".to_owned()),
                env_variable: Some("FITNESS_ZONE_THRESHOLD_MAX".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        // Algorithm Selection
        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "algorithm.tss".to_owned(),
                display_name: "TSS Calculation Method".to_owned(),
                description: "Algorithm for Training Stress Score calculation".to_owned(),
                category: "algorithms".to_owned(),
                data_type: ConfigDataType::Enum,
                default_value: serde_json::json!("avg_power"),
                valid_range: None,
                enum_options: Some(vec![
                    "avg_power".to_owned(),
                    "normalized_power".to_owned(),
                    "hybrid".to_owned(),
                ]),
                units: None,
                scientific_basis: Some("Coggan's TSS methodology".to_owned()),
                env_variable: Some("PIERRE_TSS_ALGORITHM".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "algorithm.maxhr".to_owned(),
                display_name: "Max HR Estimation".to_owned(),
                description: "Algorithm for maximum heart rate estimation".to_owned(),
                category: "algorithms".to_owned(),
                data_type: ConfigDataType::Enum,
                default_value: serde_json::json!("tanaka"),
                valid_range: None,
                enum_options: Some(vec![
                    "fox".to_owned(),
                    "tanaka".to_owned(),
                    "nes".to_owned(),
                    "gulati".to_owned(),
                ]),
                units: None,
                scientific_basis: Some("Tanaka et al. 2001: 208 - 0.7 × age".to_owned()),
                env_variable: Some("PIERRE_MAXHR_ALGORITHM".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        // Recommendation Engine
        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "recommendation.low_weekly_distance_km".to_owned(),
                display_name: "Low Weekly Distance".to_owned(),
                description: "Distance threshold below which a low volume warning is triggered"
                    .to_owned(),
                category: "recommendation_engine".to_owned(),
                data_type: ConfigDataType::Float,
                default_value: serde_json::json!(20.0),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(5.0),
                    max: serde_json::json!(50.0),
                    step: Some(1.0),
                }),
                enum_options: None,
                units: Some("km".to_owned()),
                scientific_basis: None,
                env_variable: None,
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "recommendation.high_weekly_distance_km".to_owned(),
                display_name: "High Weekly Distance".to_owned(),
                description: "Distance threshold above which overtraining warnings are triggered"
                    .to_owned(),
                category: "recommendation_engine".to_owned(),
                data_type: ConfigDataType::Float,
                default_value: serde_json::json!(80.0),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(30.0),
                    max: serde_json::json!(200.0),
                    step: Some(5.0),
                }),
                enum_options: None,
                units: Some("km".to_owned()),
                scientific_basis: None,
                env_variable: None,
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "recommendation.max_per_category".to_owned(),
                display_name: "Max Recommendations Per Category".to_owned(),
                description: "Maximum number of recommendations to show per category".to_owned(),
                category: "recommendation_engine".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(3),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(1),
                    max: serde_json::json!(10),
                    step: Some(1.0),
                }),
                enum_options: None,
                units: None,
                scientific_basis: None,
                env_variable: None,
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        // Sleep & Recovery
        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "sleep.adult_min_hours".to_owned(),
                display_name: "Minimum Sleep Hours".to_owned(),
                description: "Minimum recommended sleep hours for adults".to_owned(),
                category: "sleep_recovery".to_owned(),
                data_type: ConfigDataType::Float,
                default_value: serde_json::json!(7.0),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(5.0),
                    max: serde_json::json!(8.0),
                    step: Some(0.5),
                }),
                enum_options: None,
                units: Some("hours".to_owned()),
                scientific_basis: Some("National Sleep Foundation Guidelines".to_owned()),
                env_variable: None,
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "sleep.deep_sleep_min_percent".to_owned(),
                display_name: "Minimum Deep Sleep".to_owned(),
                description: "Minimum percentage of deep sleep for quality rest".to_owned(),
                category: "sleep_recovery".to_owned(),
                data_type: ConfigDataType::Float,
                default_value: serde_json::json!(15.0),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(10.0),
                    max: serde_json::json!(20.0),
                    step: Some(1.0),
                }),
                enum_options: None,
                units: Some("%".to_owned()),
                scientific_basis: Some("AASM Sleep Guidelines".to_owned()),
                env_variable: None,
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        // Training Stress Balance
        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "tsb.fatigued_threshold".to_owned(),
                display_name: "Fatigued TSB Threshold".to_owned(),
                description: "TSB value below which athlete is considered fatigued".to_owned(),
                category: "training_stress".to_owned(),
                data_type: ConfigDataType::Float,
                default_value: serde_json::json!(-10.0),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(-30.0),
                    max: serde_json::json!(-5.0),
                    step: Some(1.0),
                }),
                enum_options: None,
                units: Some("TSB".to_owned()),
                scientific_basis: Some("Banister's Impulse-Response Model".to_owned()),
                env_variable: None,
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "tsb.fresh_min".to_owned(),
                display_name: "Fresh Range Minimum".to_owned(),
                description: "Minimum TSB value for optimal performance readiness".to_owned(),
                category: "training_stress".to_owned(),
                data_type: ConfigDataType::Float,
                default_value: serde_json::json!(5.0),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(0.0),
                    max: serde_json::json!(15.0),
                    step: Some(1.0),
                }),
                enum_options: None,
                units: Some("TSB".to_owned()),
                scientific_basis: Some("Banister's Impulse-Response Model".to_owned()),
                env_variable: None,
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        // Weather Analysis
        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "weather.ideal_min_celsius".to_owned(),
                display_name: "Ideal Min Temperature".to_owned(),
                description: "Minimum temperature for ideal training conditions".to_owned(),
                category: "weather_analysis".to_owned(),
                data_type: ConfigDataType::Float,
                default_value: serde_json::json!(10.0),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(0.0),
                    max: serde_json::json!(15.0),
                    step: Some(1.0),
                }),
                enum_options: None,
                units: Some("°C".to_owned()),
                scientific_basis: None,
                env_variable: None,
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "weather.ideal_max_celsius".to_owned(),
                display_name: "Ideal Max Temperature".to_owned(),
                description: "Maximum temperature for ideal training conditions".to_owned(),
                category: "weather_analysis".to_owned(),
                data_type: ConfigDataType::Float,
                default_value: serde_json::json!(20.0),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(15.0),
                    max: serde_json::json!(30.0),
                    step: Some(1.0),
                }),
                enum_options: None,
                units: Some("°C".to_owned()),
                scientific_basis: None,
                env_variable: None,
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        // Nutrition
        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "nutrition.protein_athlete_g_per_kg".to_owned(),
                display_name: "Athlete Protein Target".to_owned(),
                description: "Recommended protein intake for athletes per kg body weight"
                    .to_owned(),
                category: "nutrition".to_owned(),
                data_type: ConfigDataType::Float,
                default_value: serde_json::json!(1.8),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(1.4),
                    max: serde_json::json!(2.5),
                    step: Some(0.1),
                }),
                enum_options: None,
                units: Some("g/kg".to_owned()),
                scientific_basis: Some("Phillips 2011, ISSN Position Stand".to_owned()),
                env_variable: None,
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        // Tokio Runtime Configuration
        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "tokio_runtime.worker_threads".to_owned(),
                display_name: "Worker Threads".to_owned(),
                description: "Number of Tokio runtime worker threads. Default: CPU core count"
                    .to_owned(),
                category: "tokio_runtime".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(null),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(1),
                    max: serde_json::json!(256),
                    step: Some(1.0),
                }),
                enum_options: None,
                units: Some("threads".to_owned()),
                scientific_basis: None,
                env_variable: Some("TOKIO_WORKER_THREADS".to_owned()),
                is_runtime_configurable: false,
                requires_restart: true,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "tokio_runtime.thread_stack_size".to_owned(),
                display_name: "Thread Stack Size".to_owned(),
                description: "Stack size for worker threads in bytes. Default: ~2MB".to_owned(),
                category: "tokio_runtime".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(null),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(524_288),
                    max: serde_json::json!(16_777_216),
                    step: Some(524_288.0),
                }),
                enum_options: None,
                units: Some("bytes".to_owned()),
                scientific_basis: None,
                env_variable: Some("TOKIO_THREAD_STACK_SIZE".to_owned()),
                is_runtime_configurable: false,
                requires_restart: true,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "tokio_runtime.thread_name".to_owned(),
                display_name: "Thread Name Prefix".to_owned(),
                description: "Name prefix for worker threads".to_owned(),
                category: "tokio_runtime".to_owned(),
                data_type: ConfigDataType::String,
                default_value: serde_json::json!("pierre-worker"),
                valid_range: None,
                enum_options: None,
                units: None,
                scientific_basis: None,
                env_variable: Some("TOKIO_THREAD_NAME".to_owned()),
                is_runtime_configurable: false,
                requires_restart: true,
            },
        );

        // SQLx Connection Pool Configuration
        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "sqlx.idle_timeout_secs".to_owned(),
                display_name: "Idle Timeout".to_owned(),
                description:
                    "Maximum time a connection can sit idle before being closed. Default: 10 min"
                        .to_owned(),
                category: "sqlx_config".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(null),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(30),
                    max: serde_json::json!(3600),
                    step: Some(30.0),
                }),
                enum_options: None,
                units: Some("seconds".to_owned()),
                scientific_basis: None,
                env_variable: Some("SQLX_IDLE_TIMEOUT_SECS".to_owned()),
                is_runtime_configurable: false,
                requires_restart: true,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "sqlx.max_lifetime_secs".to_owned(),
                display_name: "Max Lifetime".to_owned(),
                description:
                    "Maximum lifetime of a connection before it is closed. Default: 30 min"
                        .to_owned(),
                category: "sqlx_config".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(null),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(60),
                    max: serde_json::json!(7200),
                    step: Some(60.0),
                }),
                enum_options: None,
                units: Some("seconds".to_owned()),
                scientific_basis: None,
                env_variable: Some("SQLX_MAX_LIFETIME_SECS".to_owned()),
                is_runtime_configurable: false,
                requires_restart: true,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "sqlx.test_before_acquire".to_owned(),
                display_name: "Test Before Acquire".to_owned(),
                description: "Whether to test connections before acquiring from pool".to_owned(),
                category: "sqlx_config".to_owned(),
                data_type: ConfigDataType::Boolean,
                default_value: serde_json::json!(true),
                valid_range: None,
                enum_options: None,
                units: None,
                scientific_basis: None,
                env_variable: Some("SQLX_TEST_BEFORE_ACQUIRE".to_owned()),
                is_runtime_configurable: false,
                requires_restart: true,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "sqlx.statement_cache_capacity".to_owned(),
                display_name: "Statement Cache Capacity".to_owned(),
                description: "Number of prepared statements to cache per connection. Default: 100"
                    .to_owned(),
                category: "sqlx_config".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(null),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(0),
                    max: serde_json::json!(1000),
                    step: Some(10.0),
                }),
                enum_options: None,
                units: Some("statements".to_owned()),
                scientific_basis: None,
                env_variable: Some("SQLX_STATEMENT_CACHE_CAPACITY".to_owned()),
                is_runtime_configurable: false,
                requires_restart: true,
            },
        );

        // Cache TTL Configuration
        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "cache.profile_ttl_secs".to_owned(),
                display_name: "Profile Cache TTL".to_owned(),
                description: "Time-to-live for cached athlete profiles".to_owned(),
                category: "cache_ttl".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(3600),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(60),
                    max: serde_json::json!(86400),
                    step: Some(60.0),
                }),
                enum_options: None,
                units: Some("seconds".to_owned()),
                scientific_basis: None,
                env_variable: Some("CACHE_PROFILE_TTL_SECS".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "cache.activity_list_ttl_secs".to_owned(),
                display_name: "Activity List Cache TTL".to_owned(),
                description: "Time-to-live for cached activity lists".to_owned(),
                category: "cache_ttl".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(300),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(30),
                    max: serde_json::json!(3600),
                    step: Some(30.0),
                }),
                enum_options: None,
                units: Some("seconds".to_owned()),
                scientific_basis: None,
                env_variable: Some("CACHE_ACTIVITY_LIST_TTL_SECS".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "cache.activity_ttl_secs".to_owned(),
                display_name: "Activity Cache TTL".to_owned(),
                description: "Time-to-live for cached individual activities".to_owned(),
                category: "cache_ttl".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(1800),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(60),
                    max: serde_json::json!(7200),
                    step: Some(60.0),
                }),
                enum_options: None,
                units: Some("seconds".to_owned()),
                scientific_basis: None,
                env_variable: Some("CACHE_ACTIVITY_TTL_SECS".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "cache.stats_ttl_secs".to_owned(),
                display_name: "Stats Cache TTL".to_owned(),
                description: "Time-to-live for cached athlete statistics".to_owned(),
                category: "cache_ttl".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(3600),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(300),
                    max: serde_json::json!(86400),
                    step: Some(300.0),
                }),
                enum_options: None,
                units: Some("seconds".to_owned()),
                scientific_basis: None,
                env_variable: Some("CACHE_STATS_TTL_SECS".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        // Strava Provider Settings
        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "provider.strava_rate_limit_15min".to_owned(),
                display_name: "Strava 15-Min Rate Limit".to_owned(),
                description: "Maximum Strava API requests per 15 minutes".to_owned(),
                category: "provider_strava".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(100),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(10),
                    max: serde_json::json!(1000),
                    step: Some(10.0),
                }),
                enum_options: None,
                units: Some("requests".to_owned()),
                scientific_basis: Some("Strava API documentation".to_owned()),
                env_variable: Some("STRAVA_RATE_LIMIT_15MIN".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "provider.strava_rate_limit_daily".to_owned(),
                display_name: "Strava Daily Rate Limit".to_owned(),
                description: "Maximum Strava API requests per day".to_owned(),
                category: "provider_strava".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(1000),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(100),
                    max: serde_json::json!(10000),
                    step: Some(100.0),
                }),
                enum_options: None,
                units: Some("requests".to_owned()),
                scientific_basis: Some("Strava API documentation".to_owned()),
                env_variable: Some("STRAVA_RATE_LIMIT_DAILY".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "provider.strava_default_activities_per_page".to_owned(),
                display_name: "Strava Default Page Size".to_owned(),
                description: "Default number of activities per API page request".to_owned(),
                category: "provider_strava".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(30),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(1),
                    max: serde_json::json!(200),
                    step: Some(1.0),
                }),
                enum_options: None,
                units: Some("activities".to_owned()),
                scientific_basis: None,
                env_variable: Some("STRAVA_DEFAULT_ACTIVITIES_PER_PAGE".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "provider.strava_max_activities_per_request".to_owned(),
                display_name: "Strava Max Activities Per Request".to_owned(),
                description: "Maximum activities allowed in a single request".to_owned(),
                category: "provider_strava".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(200),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(10),
                    max: serde_json::json!(500),
                    step: Some(10.0),
                }),
                enum_options: None,
                units: Some("activities".to_owned()),
                scientific_basis: None,
                env_variable: Some("STRAVA_MAX_ACTIVITIES_PER_REQUEST".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        // Fitbit Provider Settings
        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "provider.fitbit_rate_limit_hourly".to_owned(),
                display_name: "Fitbit Hourly Rate Limit".to_owned(),
                description: "Maximum Fitbit API requests per hour".to_owned(),
                category: "provider_fitbit".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(150),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(10),
                    max: serde_json::json!(500),
                    step: Some(10.0),
                }),
                enum_options: None,
                units: Some("requests".to_owned()),
                scientific_basis: Some("Fitbit API documentation".to_owned()),
                env_variable: Some("FITBIT_RATE_LIMIT_HOURLY".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "provider.fitbit_rate_limit_daily".to_owned(),
                display_name: "Fitbit Daily Rate Limit".to_owned(),
                description: "Maximum Fitbit API requests per day".to_owned(),
                category: "provider_fitbit".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(2000),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(100),
                    max: serde_json::json!(10000),
                    step: Some(100.0),
                }),
                enum_options: None,
                units: Some("requests".to_owned()),
                scientific_basis: Some("Fitbit API documentation".to_owned()),
                env_variable: Some("FITBIT_RATE_LIMIT_DAILY".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        // Garmin Provider Settings
        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "provider.garmin_rate_limit_hourly".to_owned(),
                display_name: "Garmin Hourly Rate Limit".to_owned(),
                description: "Maximum Garmin API requests per hour".to_owned(),
                category: "provider_garmin".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(100),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(10),
                    max: serde_json::json!(500),
                    step: Some(10.0),
                }),
                enum_options: None,
                units: Some("requests".to_owned()),
                scientific_basis: Some("Garmin API documentation".to_owned()),
                env_variable: Some("GARMIN_RATE_LIMIT_HOURLY".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "provider.garmin_rate_limit_daily".to_owned(),
                display_name: "Garmin Daily Rate Limit".to_owned(),
                description: "Maximum Garmin API requests per day".to_owned(),
                category: "provider_garmin".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(1000),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(100),
                    max: serde_json::json!(5000),
                    step: Some(100.0),
                }),
                enum_options: None,
                units: Some("requests".to_owned()),
                scientific_basis: Some("Garmin API documentation".to_owned()),
                env_variable: Some("GARMIN_RATE_LIMIT_DAILY".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "provider.garmin_default_activities_per_page".to_owned(),
                display_name: "Garmin Default Page Size".to_owned(),
                description: "Default number of activities per API page request".to_owned(),
                category: "provider_garmin".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(20),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(1),
                    max: serde_json::json!(100),
                    step: Some(1.0),
                }),
                enum_options: None,
                units: Some("activities".to_owned()),
                scientific_basis: None,
                env_variable: Some("GARMIN_DEFAULT_ACTIVITIES_PER_PAGE".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "provider.garmin_max_activities_per_request".to_owned(),
                display_name: "Garmin Max Activities Per Request".to_owned(),
                description: "Maximum activities allowed in a single request".to_owned(),
                category: "provider_garmin".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(100),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(10),
                    max: serde_json::json!(200),
                    step: Some(10.0),
                }),
                enum_options: None,
                units: Some("activities".to_owned()),
                scientific_basis: None,
                env_variable: Some("GARMIN_MAX_ACTIVITIES_PER_REQUEST".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "provider.garmin_rate_limit_block_secs".to_owned(),
                display_name: "Garmin Rate Limit Block Duration".to_owned(),
                description: "Estimated block duration when rate limited".to_owned(),
                category: "provider_garmin".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(3600),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(60),
                    max: serde_json::json!(86400),
                    step: Some(60.0),
                }),
                enum_options: None,
                units: Some("seconds".to_owned()),
                scientific_basis: None,
                env_variable: Some("GARMIN_RATE_LIMIT_BLOCK_SECS".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        // MCP Network Settings
        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "mcp.max_request_size".to_owned(),
                display_name: "Max Request Size".to_owned(),
                description: "Maximum size for incoming MCP requests".to_owned(),
                category: "mcp_network".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(16_777_216),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(1_048_576),
                    max: serde_json::json!(104_857_600),
                    step: Some(1_048_576.0),
                }),
                enum_options: None,
                units: Some("bytes".to_owned()),
                scientific_basis: None,
                env_variable: Some("MCP_MAX_REQUEST_SIZE".to_owned()),
                is_runtime_configurable: false,
                requires_restart: true,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "mcp.max_response_size".to_owned(),
                display_name: "Max Response Size".to_owned(),
                description: "Maximum size for outgoing MCP responses".to_owned(),
                category: "mcp_network".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(67_108_864),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(1_048_576),
                    max: serde_json::json!(268_435_456),
                    step: Some(1_048_576.0),
                }),
                enum_options: None,
                units: Some("bytes".to_owned()),
                scientific_basis: None,
                env_variable: Some("MCP_MAX_RESPONSE_SIZE".to_owned()),
                is_runtime_configurable: false,
                requires_restart: true,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "mcp.notification_channel_size".to_owned(),
                display_name: "Notification Channel Size".to_owned(),
                description: "Buffer size for notification channels".to_owned(),
                category: "mcp_network".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(100),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(10),
                    max: serde_json::json!(1000),
                    step: Some(10.0),
                }),
                enum_options: None,
                units: Some("messages".to_owned()),
                scientific_basis: None,
                env_variable: Some("MCP_NOTIFICATION_CHANNEL_SIZE".to_owned()),
                is_runtime_configurable: false,
                requires_restart: true,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "mcp.websocket_channel_capacity".to_owned(),
                display_name: "WebSocket Channel Capacity".to_owned(),
                description: "Buffer capacity for WebSocket message channels".to_owned(),
                category: "mcp_network".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(256),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(32),
                    max: serde_json::json!(2048),
                    step: Some(32.0),
                }),
                enum_options: None,
                units: Some("messages".to_owned()),
                scientific_basis: None,
                env_variable: Some("MCP_WEBSOCKET_CHANNEL_CAPACITY".to_owned()),
                is_runtime_configurable: false,
                requires_restart: true,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "mcp.tcp_keep_alive_secs".to_owned(),
                display_name: "TCP Keep-Alive Interval".to_owned(),
                description: "TCP keep-alive interval for connections".to_owned(),
                category: "mcp_network".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(30),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(10),
                    max: serde_json::json!(300),
                    step: Some(10.0),
                }),
                enum_options: None,
                units: Some("seconds".to_owned()),
                scientific_basis: None,
                env_variable: Some("MCP_TCP_KEEP_ALIVE_SECS".to_owned()),
                is_runtime_configurable: false,
                requires_restart: true,
            },
        );

        // Monitoring Thresholds
        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "monitoring.memory_warn_mb".to_owned(),
                display_name: "Memory Warning Threshold".to_owned(),
                description: "Memory usage threshold for warning alerts".to_owned(),
                category: "monitoring".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(512),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(128),
                    max: serde_json::json!(4096),
                    step: Some(64.0),
                }),
                enum_options: None,
                units: Some("MB".to_owned()),
                scientific_basis: None,
                env_variable: Some("MONITORING_MEMORY_WARN_MB".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "monitoring.memory_critical_mb".to_owned(),
                display_name: "Memory Critical Threshold".to_owned(),
                description: "Memory usage threshold for critical alerts".to_owned(),
                category: "monitoring".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(1024),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(256),
                    max: serde_json::json!(8192),
                    step: Some(128.0),
                }),
                enum_options: None,
                units: Some("MB".to_owned()),
                scientific_basis: None,
                env_variable: Some("MONITORING_MEMORY_CRITICAL_MB".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "monitoring.latency_warn_ms".to_owned(),
                display_name: "Latency Warning Threshold".to_owned(),
                description: "Request latency threshold for warning alerts".to_owned(),
                category: "monitoring".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(500),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(100),
                    max: serde_json::json!(5000),
                    step: Some(100.0),
                }),
                enum_options: None,
                units: Some("ms".to_owned()),
                scientific_basis: None,
                env_variable: Some("MONITORING_LATENCY_WARN_MS".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "monitoring.latency_critical_ms".to_owned(),
                display_name: "Latency Critical Threshold".to_owned(),
                description: "Request latency threshold for critical alerts".to_owned(),
                category: "monitoring".to_owned(),
                data_type: ConfigDataType::Integer,
                default_value: serde_json::json!(2000),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(500),
                    max: serde_json::json!(30000),
                    step: Some(500.0),
                }),
                enum_options: None,
                units: Some("ms".to_owned()),
                scientific_basis: None,
                env_variable: Some("MONITORING_LATENCY_CRITICAL_MS".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "monitoring.error_rate_warn_pct".to_owned(),
                display_name: "Error Rate Warning Threshold".to_owned(),
                description: "Error rate percentage for warning alerts".to_owned(),
                category: "monitoring".to_owned(),
                data_type: ConfigDataType::Float,
                default_value: serde_json::json!(1.0),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(0.1),
                    max: serde_json::json!(10.0),
                    step: Some(0.1),
                }),
                enum_options: None,
                units: Some("%".to_owned()),
                scientific_basis: None,
                env_variable: Some("MONITORING_ERROR_RATE_WARN_PCT".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "monitoring.error_rate_critical_pct".to_owned(),
                display_name: "Error Rate Critical Threshold".to_owned(),
                description: "Error rate percentage for critical alerts".to_owned(),
                category: "monitoring".to_owned(),
                data_type: ConfigDataType::Float,
                default_value: serde_json::json!(5.0),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(1.0),
                    max: serde_json::json!(25.0),
                    step: Some(0.5),
                }),
                enum_options: None,
                units: Some("%".to_owned()),
                scientific_basis: None,
                env_variable: Some("MONITORING_ERROR_RATE_CRITICAL_PCT".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "monitoring.pool_usage_warn_pct".to_owned(),
                display_name: "Connection Pool Warning Threshold".to_owned(),
                description: "Connection pool usage percentage for warning alerts".to_owned(),
                category: "monitoring".to_owned(),
                data_type: ConfigDataType::Float,
                default_value: serde_json::json!(70.0),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(50.0),
                    max: serde_json::json!(90.0),
                    step: Some(5.0),
                }),
                enum_options: None,
                units: Some("%".to_owned()),
                scientific_basis: None,
                env_variable: Some("MONITORING_POOL_USAGE_WARN_PCT".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "monitoring.pool_usage_critical_pct".to_owned(),
                display_name: "Connection Pool Critical Threshold".to_owned(),
                description: "Connection pool usage percentage for critical alerts".to_owned(),
                category: "monitoring".to_owned(),
                data_type: ConfigDataType::Float,
                default_value: serde_json::json!(90.0),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(70.0),
                    max: serde_json::json!(99.0),
                    step: Some(5.0),
                }),
                enum_options: None,
                units: Some("%".to_owned()),
                scientific_basis: None,
                env_variable: Some("MONITORING_POOL_USAGE_CRITICAL_PCT".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "monitoring.cache_hit_rate_warn_pct".to_owned(),
                display_name: "Cache Hit Rate Warning Threshold".to_owned(),
                description: "Cache hit rate percentage below which warning alerts are triggered"
                    .to_owned(),
                category: "monitoring".to_owned(),
                data_type: ConfigDataType::Float,
                default_value: serde_json::json!(80.0),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(50.0),
                    max: serde_json::json!(95.0),
                    step: Some(5.0),
                }),
                enum_options: None,
                units: Some("%".to_owned()),
                scientific_basis: None,
                env_variable: Some("MONITORING_CACHE_HIT_RATE_WARN_PCT".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        Self::add_definition(
            &mut defs,
            ParameterDefinition {
                key: "monitoring.cache_miss_rate_critical_pct".to_owned(),
                display_name: "Cache Miss Rate Critical Threshold".to_owned(),
                description: "Cache miss rate percentage above which critical alerts are triggered"
                    .to_owned(),
                category: "monitoring".to_owned(),
                data_type: ConfigDataType::Float,
                default_value: serde_json::json!(50.0),
                valid_range: Some(ParameterRange {
                    min: serde_json::json!(20.0),
                    max: serde_json::json!(80.0),
                    step: Some(5.0),
                }),
                enum_options: None,
                units: Some("%".to_owned()),
                scientific_basis: None,
                env_variable: Some("MONITORING_CACHE_MISS_RATE_CRITICAL_PCT".to_owned()),
                is_runtime_configurable: true,
                requires_restart: false,
            },
        );

        // Acquire lock briefly and insert all definitions at once
        let def_count = defs.len();
        self.definitions.write().await.extend(defs);

        info!("Initialized {def_count} admin configuration parameter definitions");
    }

    fn add_definition(defs: &mut HashMap<String, ParameterDefinition>, def: ParameterDefinition) {
        defs.insert(def.key.clone(), def);
    }

    /// Refresh the cache from database overrides
    ///
    /// # Errors
    ///
    /// Returns an error if reading overrides from the database fails.
    pub async fn refresh_cache(&self, tenant_id: Option<&str>) -> AppResult<()> {
        let overrides = self.manager.get_overrides(tenant_id).await?;

        // Build the new cache entries
        let new_entries: HashMap<String, serde_json::Value> = overrides
            .into_iter()
            .map(|o| {
                let key = format!("{}.{}", o.category, o.config_key);
                (key, o.config_value)
            })
            .collect();

        let entry_count = new_entries.len();

        // Update the cache with new entries
        {
            let mut cache = self.cache.write().await;
            cache.clear();
            cache.extend(new_entries);
        }

        debug!("Refreshed config cache with {entry_count} overrides");
        Ok(())
    }

    /// Get the full configuration catalog
    ///
    /// # Errors
    ///
    /// Returns an error if reading from the database fails.
    pub async fn get_catalog(&self, tenant_id: Option<&str>) -> AppResult<ConfigCatalogResponse> {
        // Clone categories and definitions before await to avoid holding locks across await
        let categories = self.categories.read().await.clone();
        let definitions = self.definitions.read().await.clone();
        let overrides = self.manager.get_overrides(tenant_id).await?;

        // Build override lookup
        let override_map: HashMap<String, &ConfigOverride> = overrides
            .iter()
            .map(|o| (format!("{}.{}", o.category, o.config_key), o))
            .collect();

        let mut result_categories = Vec::new();
        let mut total_params = 0;
        let mut runtime_count = 0;
        let mut static_count = 0;

        for mut category in categories {
            let params: Vec<AdminConfigParameter> = definitions
                .values()
                .filter(|d| d.category == category.name)
                .map(|def| {
                    let full_key = format!("{}.{}", def.category, def.key);
                    let override_val = override_map.get(&full_key);
                    let current_value = override_val
                        .map_or_else(|| def.default_value.clone(), |o| o.config_value.clone());
                    let is_modified = override_val.is_some();

                    total_params += 1;
                    if def.is_runtime_configurable {
                        runtime_count += 1;
                    } else {
                        static_count += 1;
                    }

                    AdminConfigParameter {
                        key: def.key.clone(),
                        display_name: def.display_name.clone(),
                        description: def.description.clone(),
                        category: def.category.clone(),
                        data_type: def.data_type,
                        current_value,
                        default_value: def.default_value.clone(),
                        is_modified,
                        valid_range: def.valid_range.clone(),
                        enum_options: def.enum_options.clone(),
                        units: def.units.clone(),
                        scientific_basis: def.scientific_basis.clone(),
                        env_variable: def.env_variable.clone(),
                        is_runtime_configurable: def.is_runtime_configurable,
                        requires_restart: def.requires_restart,
                    }
                })
                .collect();

            category.parameters = params;
            result_categories.push(category);
        }

        Ok(ConfigCatalogResponse {
            categories: result_categories,
            total_parameters: total_params,
            runtime_configurable_count: runtime_count,
            static_count,
            version: "1.0.0".to_owned(),
        })
    }

    /// Validate configuration values
    pub async fn validate(&self, request: &ValidateConfigRequest) -> ValidateConfigResponse {
        let definitions = self.definitions.read().await;
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        for (key, value) in &request.parameters {
            if let Some(def) = definitions.get(key) {
                if let Err(error) = Self::validate_value(def, value) {
                    errors.push(*error);
                }

                // Add warnings for non-standard values
                if !def.is_runtime_configurable {
                    warnings.push(format!(
                        "Parameter '{key}' requires server restart to take effect"
                    ));
                }
            } else {
                errors.push(ConfigValidationError {
                    parameter: key.clone(),
                    message: "Unknown configuration parameter".to_owned(),
                    provided_value: value.clone(),
                    valid_range: None,
                });
            }
        }

        ValidateConfigResponse {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    fn validate_value(
        def: &ParameterDefinition,
        value: &serde_json::Value,
    ) -> Result<(), Box<ConfigValidationError>> {
        match def.data_type {
            ConfigDataType::Float => {
                let num = value.as_f64().ok_or_else(|| {
                    Box::new(ConfigValidationError {
                        parameter: def.key.clone(),
                        message: "Expected a floating point number".to_owned(),
                        provided_value: value.clone(),
                        valid_range: def.valid_range.clone(),
                    })
                })?;

                if let Some(range) = &def.valid_range {
                    let min = range.min.as_f64().unwrap_or(f64::MIN);
                    let max = range.max.as_f64().unwrap_or(f64::MAX);
                    if num < min || num > max {
                        return Err(Box::new(ConfigValidationError {
                            parameter: def.key.clone(),
                            message: format!("Value must be between {min} and {max}"),
                            provided_value: value.clone(),
                            valid_range: Some(range.clone()),
                        }));
                    }
                }
            }
            ConfigDataType::Integer => {
                let num = value.as_i64().ok_or_else(|| {
                    Box::new(ConfigValidationError {
                        parameter: def.key.clone(),
                        message: "Expected an integer".to_owned(),
                        provided_value: value.clone(),
                        valid_range: def.valid_range.clone(),
                    })
                })?;

                if let Some(range) = &def.valid_range {
                    let min = range.min.as_i64().unwrap_or(i64::MIN);
                    let max = range.max.as_i64().unwrap_or(i64::MAX);
                    if num < min || num > max {
                        return Err(Box::new(ConfigValidationError {
                            parameter: def.key.clone(),
                            message: format!("Value must be between {min} and {max}"),
                            provided_value: value.clone(),
                            valid_range: Some(range.clone()),
                        }));
                    }
                }
            }
            ConfigDataType::Boolean => {
                if !value.is_boolean() {
                    return Err(Box::new(ConfigValidationError {
                        parameter: def.key.clone(),
                        message: "Expected a boolean (true/false)".to_owned(),
                        provided_value: value.clone(),
                        valid_range: None,
                    }));
                }
            }
            ConfigDataType::String => {
                if !value.is_string() {
                    return Err(Box::new(ConfigValidationError {
                        parameter: def.key.clone(),
                        message: "Expected a string".to_owned(),
                        provided_value: value.clone(),
                        valid_range: None,
                    }));
                }
            }
            ConfigDataType::Enum => {
                let str_val = value.as_str().ok_or_else(|| {
                    Box::new(ConfigValidationError {
                        parameter: def.key.clone(),
                        message: "Expected a string value for enum".to_owned(),
                        provided_value: value.clone(),
                        valid_range: None,
                    })
                })?;

                if let Some(options) = &def.enum_options {
                    if !options.contains(&str_val.to_owned()) {
                        return Err(Box::new(ConfigValidationError {
                            parameter: def.key.clone(),
                            message: format!("Value must be one of: {}", options.join(", ")),
                            provided_value: value.clone(),
                            valid_range: None,
                        }));
                    }
                }
            }
        }

        Ok(())
    }

    /// Update configuration values
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail during update.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_config(
        &self,
        request: &UpdateConfigRequest,
        admin_user_id: &str,
        admin_email: &str,
        tenant_id: Option<&str>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> AppResult<UpdateConfigResponse> {
        // First validate
        let validation = self
            .validate(&ValidateConfigRequest {
                parameters: request.parameters.clone(),
            })
            .await;

        if !validation.is_valid {
            return Ok(UpdateConfigResponse {
                success: false,
                updated_count: 0,
                validation_errors: validation.errors,
                requires_restart: false,
                effective_at: Utc::now(),
            });
        }

        // Clone definitions to avoid holding lock across awaits in the loop
        let definitions = self.definitions.read().await.clone();
        let mut updated_count = 0;
        let mut requires_restart = false;

        for (key, value) in &request.parameters {
            if let Some(def) = definitions.get(key) {
                // Get old value for audit
                let old_override = self
                    .manager
                    .get_override(&def.category, key, tenant_id)
                    .await?;
                let old_value = old_override.map(|o| o.config_value);

                // Set the new override
                self.manager
                    .set_override(
                        &def.category,
                        key,
                        value,
                        def.data_type,
                        admin_user_id,
                        tenant_id,
                        request.reason.as_deref(),
                    )
                    .await?;

                // Log the change
                self.manager
                    .log_change(
                        admin_user_id,
                        admin_email,
                        &def.category,
                        key,
                        old_value.as_ref(),
                        value,
                        def.data_type,
                        request.reason.as_deref(),
                        tenant_id,
                        ip_address,
                        user_agent,
                    )
                    .await?;

                updated_count += 1;
                if def.requires_restart {
                    requires_restart = true;
                }
            }
        }

        // Refresh cache
        self.refresh_cache(tenant_id).await?;

        info!(
            "Admin {} updated {} configuration parameters",
            admin_email, updated_count
        );

        Ok(UpdateConfigResponse {
            success: true,
            updated_count,
            validation_errors: Vec::new(),
            requires_restart,
            effective_at: Utc::now(),
        })
    }

    /// Reset configuration to defaults
    ///
    /// # Errors
    ///
    /// Returns an error if no category is specified or database operations fail.
    #[allow(clippy::cognitive_complexity)]
    pub async fn reset_config(
        &self,
        request: &ResetConfigRequest,
        admin_user_id: &str,
        admin_email: &str,
        tenant_id: Option<&str>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> AppResult<ResetConfigResponse> {
        // Clone definitions to avoid holding lock across awaits in the loop
        let definitions = self.definitions.read().await.clone();
        let mut reset_count = 0;
        let mut reset_keys = Vec::new();

        if let Some(category) = &request.category {
            // Validate that the category exists
            let category_exists = definitions.values().any(|def| def.category == *category);
            if !category_exists {
                return Err(AppError::not_found(format!(
                    "Category '{category}' not found"
                )));
            }

            if let Some(keys) = &request.keys {
                // Reset specific keys in category
                for key in keys {
                    if let Some(def) = definitions.get(key) {
                        if def.category == *category {
                            let old_override =
                                self.manager.get_override(category, key, tenant_id).await?;

                            if self
                                .manager
                                .delete_override(category, key, tenant_id)
                                .await?
                            {
                                // Log the reset
                                if let Some(old) = old_override {
                                    self.manager
                                        .log_change(
                                            admin_user_id,
                                            admin_email,
                                            category,
                                            key,
                                            Some(&old.config_value),
                                            &def.default_value,
                                            def.data_type,
                                            request.reason.as_deref(),
                                            tenant_id,
                                            ip_address,
                                            user_agent,
                                        )
                                        .await?;
                                }
                                reset_count += 1;
                                reset_keys.push(key.clone());
                            }
                        }
                    }
                }
            } else {
                // Reset entire category
                reset_count = self
                    .manager
                    .delete_category_overrides(category, tenant_id)
                    .await?;

                // Get all keys in category for response
                for def in definitions.values() {
                    if def.category == *category {
                        reset_keys.push(def.key.clone());
                    }
                }
            }
        } else {
            warn!("Reset all configurations requested - this is a destructive operation");
            // Reset all would require iterating all categories
            // For safety, we don't implement "reset everything" without explicit category
            return Err(AppError::invalid_input(
                "Must specify a category to reset. Full reset not supported.",
            ));
        }

        // Refresh cache
        self.refresh_cache(tenant_id).await?;

        info!(
            "Admin {} reset {} configuration parameters",
            admin_email, reset_count
        );

        Ok(ResetConfigResponse {
            success: true,
            reset_count,
            reset_keys,
        })
    }

    /// Get audit log
    ///
    /// # Errors
    ///
    /// Returns an error if reading the audit log from the database fails.
    pub async fn get_audit_log(
        &self,
        filter: &ConfigAuditFilter,
        limit: usize,
        offset: usize,
    ) -> AppResult<(Vec<super::types::ConfigAuditEntry>, usize)> {
        self.manager.get_audit_log(filter, limit, offset).await
    }

    /// Get a specific configuration value
    ///
    /// # Errors
    ///
    /// Returns an error if reading from the database fails.
    pub async fn get_value(
        &self,
        key: &str,
        tenant_id: Option<&str>,
    ) -> AppResult<Option<serde_json::Value>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(val) = cache.get(key) {
                return Ok(Some(val.clone()));
            }
        }

        // Get definition to find category and default
        let definitions = self.definitions.read().await;
        if let Some(def) = definitions.get(key) {
            let category = def.category.clone();
            let default_value = def.default_value.clone();
            drop(definitions); // Release lock before await

            // Check database
            if let Some(override_val) = self
                .manager
                .get_effective_override(&category, key, tenant_id)
                .await?
            {
                return Ok(Some(override_val.config_value));
            }

            // Return default
            return Ok(Some(default_value));
        }

        Ok(None)
    }
}
