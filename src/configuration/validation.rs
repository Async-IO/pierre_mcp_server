// ABOUTME: Configuration validation and type checking utilities
// ABOUTME: Ensures configuration values are valid and within acceptable ranges
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Configuration validation system for ensuring safe and valid parameter changes

use super::catalog::CatalogBuilder;
use super::runtime::ConfigValue;
use crate::models::UserPhysiologicalProfile;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration validator
pub struct ConfigValidator {
    /// Safety constraints
    safety_rules: Vec<SafetyRule>,
    /// Physiological relationship rules
    relationship_rules: Vec<RelationshipRule>,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether all validations passed
    pub is_valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Impact analysis
    pub impact_analysis: Option<ImpactAnalysis>,
}

/// Impact analysis of configuration changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAnalysis {
    /// Expected change in effort scores
    pub effort_score_change: f64,
    /// Expected change in zone boundaries
    pub zone_boundary_changes: HashMap<String, f64>,
    /// Affected analysis components
    pub affected_components: Vec<String>,
    /// Risk level (low, medium, high)
    pub risk_level: RiskLevel,
}

/// Risk level for configuration changes
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

/// Safety validation rule
#[derive(Debug, Clone)]
pub struct SafetyRule {
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: String,
    /// Validation function
    pub validator: fn(&str, &ConfigValue, Option<&UserPhysiologicalProfile>) -> Result<(), String>,
}

/// Physiological relationship rule
#[derive(Debug, Clone)]
pub struct RelationshipRule {
    /// Rule name
    pub name: String,
    /// Parameters this rule applies to
    pub parameters: Vec<String>,
    /// Validation function
    pub validator: fn(&HashMap<String, ConfigValue>) -> Result<(), String>,
}

impl ConfigValidator {
    /// Create a new configuration validator
    #[must_use]
    pub fn new() -> Self {
        Self {
            safety_rules: Self::build_safety_rules(),
            relationship_rules: Self::build_relationship_rules(),
        }
    }

    /// Validate a set of configuration changes
    #[must_use]
    pub fn validate(
        &self,
        changes: &HashMap<String, ConfigValue>,
        user_profile: Option<&UserPhysiologicalProfile>,
    ) -> ValidationResult {
        let mut result = ValidationResult::new();

        // 1. Validate individual parameters
        for (key, value) in changes {
            if let Err(error) = self.validate_parameter(key, value, user_profile) {
                result.add_error(error);
            }
        }

        // 2. Validate physiological relationships
        for rule in &self.relationship_rules {
            let relevant_params: HashMap<String, ConfigValue> = changes
                .iter()
                .filter(|(key, _)| rule.parameters.contains(key))
                .map(|(k, v)| (k.clone(), v.clone())) // Safe: String ownership for filtered config map
                .collect();

            if !relevant_params.is_empty() {
                if let Err(error) = (rule.validator)(&relevant_params) {
                    result.add_error(format!("{}: {}", rule.name, error));
                }
            }
        }

        // 3. Perform impact analysis
        result.impact_analysis = Some(Self::analyze_impact(changes));

        result
    }

    /// Validate a single parameter
    fn validate_parameter(
        &self,
        key: &str,
        value: &ConfigValue,
        user_profile: Option<&UserPhysiologicalProfile>,
    ) -> Result<(), String> {
        // Check if parameter exists in catalog
        let param_def = CatalogBuilder::get_parameter(key)
            .ok_or_else(|| format!("Unknown parameter: {key}"))?;

        // Validate data type
        match (&param_def.data_type, value) {
            (crate::configuration::catalog::ParameterType::Float, ConfigValue::Float(_))
            | (crate::configuration::catalog::ParameterType::Integer, ConfigValue::Integer(_))
            | (crate::configuration::catalog::ParameterType::Boolean, ConfigValue::Boolean(_))
            | (crate::configuration::catalog::ParameterType::String, ConfigValue::String(_)) => {}
            _ => return Err(format!("Type mismatch for parameter {key}")),
        }

        // Validate range if specified
        if let Some(valid_range) = &param_def.valid_range {
            match (value, valid_range) {
                (ConfigValue::Float(v), ConfigValue::FloatRange { min, max }) => {
                    if v < min || v > max {
                        return Err(format!(
                            "Value {v} is outside valid range [{min}, {max}] for {key}"
                        ));
                    }
                }
                (ConfigValue::Integer(v), ConfigValue::IntegerRange { min, max }) => {
                    if v < min || v > max {
                        return Err(format!(
                            "Value {v} is outside valid range [{min}, {max}] for {key}"
                        ));
                    }
                }
                _ => {}
            }
        }

        // Check VO2 max requirement
        if param_def.requires_vo2_max
            && (user_profile.is_none() || user_profile.as_ref().is_none_or(|p| p.vo2_max.is_none()))
        {
            return Err(format!("Parameter {key} requires VO2 max data"));
        }

        // Apply safety rules
        for rule in &self.safety_rules {
            if let Err(error) = (rule.validator)(key, value, user_profile) {
                return Err(format!("{}: {}", rule.name, error));
            }
        }

        Ok(())
    }

    /// Analyze impact of configuration changes
    fn analyze_impact(changes: &HashMap<String, ConfigValue>) -> ImpactAnalysis {
        let mut impact = ImpactAnalysis {
            effort_score_change: 0.0,
            zone_boundary_changes: HashMap::new(),
            affected_components: Vec::new(),
            risk_level: RiskLevel::Low,
        };

        // Analyze effort score impact
        for (key, value) in changes {
            if key.contains("distance_divisor") || key.contains("elevation_divisor") {
                if let ConfigValue::Float(new_value) = value {
                    let default_value = match key.as_str() {
                        "performance.run_distance_divisor" => 10.0,
                        "performance.bike_distance_divisor" => 40.0,
                        "performance.swim_distance_divisor" => 2.0,
                        "performance.elevation_divisor" => 100.0,
                        _ => continue,
                    };

                    // Calculate relative change
                    let relative_change = (new_value - default_value) / default_value;
                    impact.effort_score_change += relative_change * 10.0; // Scale for visibility
                }
            }
        }

        // Analyze zone boundary changes
        for (key, value) in changes {
            if key.contains("threshold") || key.contains("zone") {
                if let ConfigValue::Float(new_value) = value {
                    let zone_name = key.split('.').next_back().unwrap_or(key);
                    impact
                        .zone_boundary_changes
                        .insert(zone_name.to_string(), *new_value);
                }
            }
        }

        // Identify affected components
        for key in changes.keys() {
            if key.starts_with("heart_rate") {
                impact
                    .affected_components
                    .push("Heart Rate Analysis".into());
            }
            if key.starts_with("performance") {
                impact.affected_components.push("Effort Scoring".into());
            }
            if key.starts_with("efficiency") {
                impact
                    .affected_components
                    .push("Efficiency Calculation".into());
            }
            if key.contains("lactate") {
                impact
                    .affected_components
                    .push("Lactate Threshold Analysis".into());
            }
        }

        // Determine risk level
        impact.risk_level = if impact.effort_score_change.abs() > 20.0
            || impact.zone_boundary_changes.len() > 3
        {
            RiskLevel::High
        } else if impact.effort_score_change.abs() > 10.0 || impact.zone_boundary_changes.len() > 1
        {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        impact
    }

    /// Build safety validation rules
    fn build_safety_rules() -> Vec<SafetyRule> {
        vec![
            SafetyRule {
                name: "Heart Rate Safety".into(),
                description: "Ensure heart rate thresholds are physiologically safe".into(),
                validator: |key, value, profile| {
                    if key.contains("heart_rate") && key.contains("percentage") {
                        if let ConfigValue::Float(percentage) = value {
                            if *percentage > 100.0 {
                                return Err("Heart rate percentage cannot exceed 100%".into());
                            }
                            if *percentage < 30.0 {
                                return Err(
                                    "Heart rate percentage too low for meaningful training"
                                        .to_string(),
                                );
                            }

                            // Age-based safety checks
                            if let Some(profile) = profile {
                                if let Some(age) = profile.age {
                                    let estimated_max_hr = 220.0 - f64::from(age);
                                    let actual_hr = estimated_max_hr * percentage / 100.0;

                                    if age > 65 && actual_hr > 160.0 {
                                        return Err(
                                            "Heart rate target may be too high for age group"
                                                .to_string(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Ok(())
                },
            },
            SafetyRule {
                name: "Intensity Limits".into(),
                description: "Prevent dangerously high intensity settings".into(),
                validator: |key, value, _profile| {
                    if key.contains("max_intensity") || key.contains("safety") {
                        if let ConfigValue::Float(intensity) = value {
                            if *intensity > 1.0 {
                                return Err("Maximum intensity cannot exceed 100%".into());
                            }
                            if *intensity < 0.3 {
                                return Err(
                                    "Maximum intensity too low for effective training".into()
                                );
                            }
                        }
                    }
                    Ok(())
                },
            },
            SafetyRule {
                name: "Divisor Sanity Check".into(),
                description: "Ensure divisors are within reasonable ranges".into(),
                validator: |key, value, _profile| {
                    if key.contains("divisor") {
                        if let ConfigValue::Float(divisor) = value {
                            if *divisor <= 0.0 {
                                return Err("Divisor must be positive".into());
                            }
                            if *divisor > 1000.0 {
                                return Err("Divisor value unreasonably high".into());
                            }
                        }
                    }
                    Ok(())
                },
            },
        ]
    }

    /// Build physiological relationship rules
    fn build_relationship_rules() -> Vec<RelationshipRule> {
        vec![
            RelationshipRule {
                name: "Heart Rate Zone Order".into(),
                parameters: vec![
                    "heart_rate.recovery_zone".into(),
                    "heart_rate.endurance_zone".into(),
                    "heart_rate.tempo_zone".into(),
                    "heart_rate.anaerobic_threshold".into(),
                    "heart_rate.vo2_max_zone".into(),
                ],
                validator: |params| {
                    let zones = [
                        (
                            "heart_rate.recovery_zone",
                            params.get("heart_rate.recovery_zone"),
                        ),
                        (
                            "heart_rate.endurance_zone",
                            params.get("heart_rate.endurance_zone"),
                        ),
                        ("heart_rate.tempo_zone", params.get("heart_rate.tempo_zone")),
                        (
                            "heart_rate.anaerobic_threshold",
                            params.get("heart_rate.anaerobic_threshold"),
                        ),
                        (
                            "heart_rate.vo2_max_zone",
                            params.get("heart_rate.vo2_max_zone"),
                        ),
                    ];

                    let mut prev_value = 0.0;
                    for (name, value_opt) in zones {
                        if let Some(ConfigValue::Float(value)) = value_opt {
                            if *value <= prev_value {
                                return Err(format!("{name} must be higher than previous zone"));
                            }
                            prev_value = *value;
                        }
                    }

                    Ok(())
                },
            },
            RelationshipRule {
                name: "Lactate Threshold Consistency".into(),
                parameters: vec![
                    "lactate.threshold_percentage".into(),
                    "heart_rate.anaerobic_threshold".into(),
                ],
                validator: |params| {
                    if let (
                        Some(ConfigValue::Float(lactate_pct)),
                        Some(ConfigValue::Float(hr_pct)),
                    ) = (
                        params.get("lactate.threshold_percentage"),
                        params.get("heart_rate.anaerobic_threshold"),
                    ) {
                        // Lactate threshold and anaerobic threshold should be similar
                        if (lactate_pct - hr_pct).abs() > 10.0 {
                            return Err(
                                "Lactate threshold and HR anaerobic threshold should be within 10%"
                                    .to_string(),
                            );
                        }
                    }
                    Ok(())
                },
            },
        ]
    }
}

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationResult {
    /// Create a new validation result
    #[must_use]
    pub const fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            impact_analysis: None,
        }
    }

    /// Add an error
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
        self.is_valid = false;
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    /// Set impact analysis
    pub fn set_impact_analysis(&mut self, analysis: ImpactAnalysis) {
        self.impact_analysis = Some(analysis);
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}
