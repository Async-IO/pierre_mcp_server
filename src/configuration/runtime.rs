// ABOUTME: Runtime configuration management and dynamic config loading
// ABOUTME: Handles configuration parsing, validation, and runtime updates
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Runtime configuration management with session-scoped overrides
//!
// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - Configuration value ownership transfers for runtime updates and validation
// - Profile data ownership for configuration loading and session management

use super::profiles::ConfigProfile;
use super::vo2_max::{SportEfficiency, VO2MaxCalculator};
use crate::models::UserPhysiologicalProfile;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Session-specific runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Base physiological constants (from static definitions)
    base_constants: HashMap<String, f64>,

    /// User-specific physiological profile
    user_profile: Option<UserPhysiologicalProfile>,

    /// Session-specific overrides
    session_overrides: HashMap<String, ConfigValue>,

    /// Active configuration profile
    active_profile: ConfigProfile,

    /// VO2 max calculator for personalized thresholds
    vo2_calculator: Option<VO2MaxCalculator>,

    /// Audit trail for configuration changes
    change_log: Vec<ConfigChange>,

    /// Last modification timestamp
    last_modified: DateTime<Utc>,
}

/// Configuration value types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum ConfigValue {
    Float(f64),
    Integer(i64),
    Boolean(bool),
    String(String),
    FloatRange { min: f64, max: f64 },
    IntegerRange { min: i64, max: i64 },
}

/// Configuration change audit entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChange {
    pub timestamp: DateTime<Utc>,
    pub module: String,
    pub parameter: String,
    pub old_value: Option<ConfigValue>,
    pub new_value: ConfigValue,
    pub reason: Option<String>,
}

/// Trait for configuration-aware components
pub trait ConfigAware {
    /// Get the runtime configuration
    fn get_runtime_config(&self) -> &RuntimeConfig;

    /// Get a configuration value with fallback to default
    fn get_config_value(&self, key: &str, default: f64) -> f64 {
        self.get_runtime_config()
            .get_value(key)
            .and_then(|v| match v {
                ConfigValue::Float(f) => Some(f),
                #[allow(clippy::cast_precision_loss)]
                ConfigValue::Integer(i) => Some(i as f64), // Cast needed for interface compatibility
                _ => None,
            })
            .unwrap_or(default)
    }

    /// Get a threshold adjusted for athlete level
    fn get_threshold_for_athlete(&self, key: &str, default: f64) -> f64 {
        let config = self.get_runtime_config();
        let base_value = self.get_config_value(key, default);

        // Apply profile-based adjustments
        match &config.active_profile {
            ConfigProfile::Elite {
                performance_factor, ..
            } => base_value * performance_factor,
            ConfigProfile::Recreational {
                threshold_tolerance,
                ..
            } => base_value * threshold_tolerance,
            ConfigProfile::Beginner {
                threshold_reduction,
                ..
            } => base_value * threshold_reduction,
            _ => base_value,
        }
    }
}

impl RuntimeConfig {
    /// Create a new runtime configuration with defaults
    #[must_use]
    pub fn new() -> Self {
        Self {
            base_constants: Self::load_base_constants(),
            user_profile: None,
            session_overrides: HashMap::new(),
            active_profile: ConfigProfile::Default,
            vo2_calculator: None,
            change_log: Vec::new(),
            last_modified: Utc::now(),
        }
    }

    /// Create with a specific profile
    #[must_use]
    pub fn with_profile(profile: ConfigProfile) -> Self {
        let mut config = Self::new();
        config.active_profile = profile;
        config
    }

    /// Load base constants from physiological constants module
    fn load_base_constants() -> HashMap<String, f64> {
        let mut constants = HashMap::new();

        // Heart rate zones - physiological standards
        constants.insert("heart_rate.anaerobic_threshold".into(), 85.0);
        constants.insert("heart_rate.vo2_max_zone".into(), 95.0);
        constants.insert("heart_rate.tempo_zone".into(), 80.0);
        constants.insert("heart_rate.endurance_zone".into(), 70.0);
        constants.insert("heart_rate.recovery_zone".into(), 60.0);

        // Performance calculation coefficients
        constants.insert("performance.run_distance_divisor".into(), 10.0);
        constants.insert("performance.bike_distance_divisor".into(), 40.0);
        constants.insert("performance.swim_distance_divisor".into(), 2.0);
        constants.insert("performance.elevation_divisor".into(), 100.0);

        // Efficiency calculation baseline
        constants.insert("efficiency.base_score".into(), 50.0);
        constants.insert("efficiency.hr_factor".into(), 1000.0);

        constants
    }

    /// Set user physiological profile and update VO2 calculator
    pub fn set_user_profile(&mut self, profile: UserPhysiologicalProfile) {
        // Create VO2 calculator if we have the necessary data
        if let (Some(vo2_max), Some(resting_hr), Some(max_hr)) =
            (profile.vo2_max, profile.resting_hr, profile.max_hr)
        {
            self.vo2_calculator = Some(VO2MaxCalculator::new(
                vo2_max,
                resting_hr,
                max_hr,
                profile.lactate_threshold_percentage.unwrap_or(0.85),
                profile.primary_sport.sport_efficiency_factor(),
            ));
        }

        self.user_profile = Some(profile);
        self.last_modified = Utc::now();
    }

    /// Apply a configuration profile
    pub fn apply_profile(&mut self, profile: ConfigProfile) {
        self.log_change(
            "system".into(),
            "profile".into(),
            Some(ConfigValue::String(self.active_profile.name())),
            ConfigValue::String(profile.name()),
            Some(format!("Applied {} profile", profile.name())),
        );

        self.active_profile = profile;
        self.last_modified = Utc::now();
    }

    /// Determine profile based on current configuration settings
    #[must_use]
    pub fn determine_profile(&self) -> ConfigProfile {
        self.active_profile.clone()
    }

    /// Get a configuration value
    #[must_use]
    pub fn get_value(&self, key: &str) -> Option<ConfigValue> {
        // Check session overrides first
        if let Some(value) = self.session_overrides.get(key) {
            return Some(value.clone());
        }

        // Check base constants
        if let Some(base_value) = self.base_constants.get(key) {
            return Some(ConfigValue::Float(*base_value));
        }

        None
    }

    /// Set a session override value
    ///
    /// # Errors
    ///
    /// This function currently doesn't return any errors but is designed to validate
    /// configuration changes in the future.
    pub fn set_override(&mut self, key: &str, value: ConfigValue) -> Result<(), String> {
        let old_value = self.get_value(key);

        self.log_change(
            "session".into(),
            key.to_string(),
            old_value,
            value.clone(),
            None,
        );

        self.session_overrides.insert(key.to_string(), value);
        self.last_modified = Utc::now();

        Ok(())
    }

    /// Get all values for a module
    #[must_use]
    pub fn get_module_values(&self, module: &str) -> HashMap<String, ConfigValue> {
        let mut values = HashMap::new();
        let prefix = format!("{module}.");

        // Collect base constants
        for (key, value) in &self.base_constants {
            if key.starts_with(&prefix) {
                values.insert(key.clone(), ConfigValue::Float(*value));
            }
        }

        // Override with session values
        for (key, value) in &self.session_overrides {
            if key.starts_with(&prefix) {
                values.insert(key.clone(), value.clone());
            }
        }

        values
    }

    /// Reset all session overrides
    pub fn reset_overrides(&mut self) {
        self.session_overrides.clear();
        self.last_modified = Utc::now();

        self.log_change(
            "system".into(),
            "all_overrides".into(),
            None,
            ConfigValue::String("reset".into()),
            Some("Reset all session overrides".into()),
        );
    }

    /// Log a configuration change
    fn log_change(
        &mut self,
        module: String,
        parameter: String,
        old_value: Option<ConfigValue>,
        new_value: ConfigValue,
        reason: Option<String>,
    ) {
        self.change_log.push(ConfigChange {
            timestamp: Utc::now(),
            module,
            parameter,
            old_value,
            new_value,
            reason,
        });
    }

    /// Get recent changes
    #[must_use]
    pub fn get_recent_changes(&self, limit: usize) -> Vec<&ConfigChange> {
        self.change_log.iter().rev().take(limit).collect()
    }

    /// Get the active profile
    #[must_use]
    pub const fn get_profile(&self) -> &ConfigProfile {
        &self.active_profile
    }

    /// Get session overrides
    #[must_use]
    pub const fn get_session_overrides(&self) -> &HashMap<String, ConfigValue> {
        &self.session_overrides
    }

    /// Export configuration state
    #[must_use]
    pub fn export(&self) -> ConfigExport {
        ConfigExport {
            profile: self.active_profile.clone(),
            session_overrides: self.session_overrides.clone(),
            user_profile: self.user_profile.clone(),
            vo2_calculator_state: self.vo2_calculator.as_ref().map(|calc| VO2CalculatorState {
                vo2_max: calc.vo2_max,
                resting_hr: calc.resting_hr,
                max_hr: calc.max_hr,
                lactate_threshold: calc.lactate_threshold,
            }),
            last_modified: self.last_modified,
        }
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration export format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigExport {
    pub profile: ConfigProfile,
    pub session_overrides: HashMap<String, ConfigValue>,
    pub user_profile: Option<UserPhysiologicalProfile>,
    pub vo2_calculator_state: Option<VO2CalculatorState>,
    pub last_modified: DateTime<Utc>,
}

/// VO2 calculator state for export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VO2CalculatorState {
    pub vo2_max: f64,
    pub resting_hr: u16,
    pub max_hr: u16,
    pub lactate_threshold: f64,
}

/// Global configuration manager for all user sessions
pub struct ConfigurationManager {
    /// Per-user runtime configurations
    user_configs: Arc<RwLock<HashMap<Uuid, RuntimeConfig>>>,
}

impl ConfigurationManager {
    /// Create a new configuration manager
    #[must_use]
    pub fn new() -> Self {
        Self {
            user_configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create a user's configuration
    pub async fn get_user_config(&self, user_id: Uuid) -> RuntimeConfig {
        let configs = self.user_configs.read().await;
        if let Some(config) = configs.get(&user_id) {
            config.clone()
        } else {
            drop(configs);
            let mut configs = self.user_configs.write().await;
            let config = RuntimeConfig::new();
            configs.insert(user_id, config.clone());
            config
        }
    }

    /// Update a user's configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the updater function fails to apply the configuration changes.
    pub async fn update_user_config<F>(&self, user_id: Uuid, updater: F) -> Result<(), String>
    where
        F: FnOnce(&mut RuntimeConfig) -> Result<(), String>,
    {
        updater(
            self.user_configs
                .write()
                .await
                .entry(user_id)
                .or_insert_with(RuntimeConfig::new),
        )
    }
}

impl Default for ConfigurationManager {
    fn default() -> Self {
        Self::new()
    }
}
