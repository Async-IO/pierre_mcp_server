// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Runtime configuration management with session-scoped overrides

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
#[derive(Debug, Clone)]
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
                ConfigValue::Integer(i) => Some(i as f64),
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
    pub fn with_profile(profile: ConfigProfile) -> Self {
        let mut config = Self::new();
        config.active_profile = profile;
        config
    }

    /// Load base constants from physiological constants module
    fn load_base_constants() -> HashMap<String, f64> {
        let mut constants = HashMap::new();

        // Heart rate zones - using default values for now
        constants.insert("heart_rate.anaerobic_threshold".to_string(), 85.0);
        constants.insert("heart_rate.vo2_max_zone".to_string(), 95.0);
        constants.insert("heart_rate.tempo_zone".to_string(), 80.0);
        constants.insert("heart_rate.endurance_zone".to_string(), 70.0);
        constants.insert("heart_rate.recovery_zone".to_string(), 60.0);

        // Performance calculation - using default values for now
        constants.insert("performance.run_distance_divisor".to_string(), 10.0);
        constants.insert("performance.bike_distance_divisor".to_string(), 40.0);
        constants.insert("performance.swim_distance_divisor".to_string(), 2.0);
        constants.insert("performance.elevation_divisor".to_string(), 100.0);

        // Efficiency calculation - using default values for now
        constants.insert("efficiency.base_score".to_string(), 50.0);
        constants.insert("efficiency.hr_factor".to_string(), 1000.0);

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
            "system".to_string(),
            "profile".to_string(),
            Some(ConfigValue::String(self.active_profile.name())),
            ConfigValue::String(profile.name()),
            Some(format!("Applied {} profile", profile.name())),
        );

        self.active_profile = profile;
        self.last_modified = Utc::now();
    }

    /// Get a configuration value
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
    pub fn set_override(&mut self, key: String, value: ConfigValue) -> Result<(), String> {
        let old_value = self.get_value(&key);

        self.log_change(
            "session".to_string(),
            key.clone(),
            old_value,
            value.clone(),
            None,
        );

        self.session_overrides.insert(key, value);
        self.last_modified = Utc::now();

        Ok(())
    }

    /// Get all values for a module
    pub fn get_module_values(&self, module: &str) -> HashMap<String, ConfigValue> {
        let mut values = HashMap::new();
        let prefix = format!("{}.", module);

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
            "system".to_string(),
            "all_overrides".to_string(),
            None,
            ConfigValue::String("reset".to_string()),
            Some("Reset all session overrides".to_string()),
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
    pub fn get_recent_changes(&self, limit: usize) -> Vec<&ConfigChange> {
        self.change_log.iter().rev().take(limit).collect()
    }

    /// Get the active profile
    pub fn get_profile(&self) -> &ConfigProfile {
        &self.active_profile
    }

    /// Get session overrides
    pub fn get_session_overrides(&self) -> &HashMap<String, ConfigValue> {
        &self.session_overrides
    }

    /// Export configuration state
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
    pub async fn update_user_config<F>(&self, user_id: Uuid, updater: F) -> Result<(), String>
    where
        F: FnOnce(&mut RuntimeConfig) -> Result<(), String>,
    {
        let mut configs = self.user_configs.write().await;
        let config = configs.entry(user_id).or_insert_with(RuntimeConfig::new);
        updater(config)
    }
}

impl Default for ConfigurationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_config_creation() {
        let config = RuntimeConfig::new();
        assert_eq!(config.active_profile, ConfigProfile::Default);
        assert!(config.session_overrides.is_empty());
        assert!(config
            .base_constants
            .contains_key("heart_rate.anaerobic_threshold"));
    }

    #[test]
    fn test_config_value_override() {
        let mut config = RuntimeConfig::new();
        let key = "heart_rate.anaerobic_threshold".to_string();

        // Get base value
        let base_value = config.get_value(&key);
        assert!(base_value.is_some());

        // Set override
        config
            .set_override(key.clone(), ConfigValue::Float(90.0))
            .unwrap();

        // Verify override takes precedence
        if let Some(ConfigValue::Float(value)) = config.get_value(&key) {
            assert_eq!(value, 90.0);
        } else {
            panic!("Expected float value");
        }
    }

    #[test]
    fn test_module_values() {
        let mut config = RuntimeConfig::new();

        // Add some overrides
        config
            .set_override(
                "heart_rate.custom_threshold".to_string(),
                ConfigValue::Float(82.5),
            )
            .unwrap();

        let hr_values = config.get_module_values("heart_rate");
        assert!(hr_values.contains_key("heart_rate.anaerobic_threshold"));
        assert!(hr_values.contains_key("heart_rate.custom_threshold"));
    }

    #[test]
    fn test_change_logging() {
        let mut config = RuntimeConfig::new();

        config
            .set_override("test.parameter".to_string(), ConfigValue::Float(50.0))
            .unwrap();

        let changes = config.get_recent_changes(10);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].parameter, "test.parameter");
    }

    #[tokio::test]
    async fn test_configuration_manager() {
        let manager = ConfigurationManager::new();
        let user_id = Uuid::new_v4();

        // Get config (should create new)
        let config1 = manager.get_user_config(user_id).await;
        assert_eq!(config1.active_profile, ConfigProfile::Default);

        // Update config
        manager
            .update_user_config(user_id, |config| {
                config.apply_profile(ConfigProfile::Elite {
                    performance_factor: 1.1,
                    recovery_sensitivity: 1.2,
                });
                Ok(())
            })
            .await
            .unwrap();

        // Verify update
        let config2 = manager.get_user_config(user_id).await;
        assert!(matches!(
            config2.active_profile,
            ConfigProfile::Elite { .. }
        ));
    }
}
