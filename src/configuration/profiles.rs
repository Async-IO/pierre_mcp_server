// ABOUTME: User profile configuration and fitness-specific settings
// ABOUTME: Manages athlete profiles, preferences, and personalized configurations
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Configuration profiles for different user types and use cases

use super::catalog::CatalogBuilder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Predefined configuration profiles
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ConfigProfile {
    /// Default configuration with standard thresholds
    Default,

    /// Research-grade detailed analysis
    Research {
        /// Multiplier for sensitivity in analysis (1.0 = normal, >1.0 = more sensitive)
        sensitivity_multiplier: f64,
        /// Level of zone granularity
        zone_granularity: ZoneGranularity,
        /// Statistical confidence level for analysis
        statistical_confidence: f64,
    },

    /// Elite athlete with strict thresholds
    Elite {
        /// Performance adjustment factor (>1.0 for higher standards)
        performance_factor: f64,
        /// Recovery sensitivity multiplier
        recovery_sensitivity: f64,
    },

    /// Recreational athlete with forgiving analysis
    Recreational {
        /// Positive bias in performance assessment (0.0-1.0)
        motivation_bias: f64,
        /// Threshold tolerance multiplier (>1.0 for more forgiving)
        threshold_tolerance: f64,
    },

    /// Beginner with educational focus
    Beginner {
        /// Reduce thresholds by this factor
        threshold_reduction: f64,
        /// Simplify metrics for easier understanding
        simplified_metrics: bool,
    },

    /// Medical/rehabilitation context
    Medical {
        /// Maximum allowed intensity (0.0-1.0)
        max_intensity: f64,
        /// Use conservative thresholds
        conservative_thresholds: bool,
        /// Additional safety margin
        safety_margin: f64,
    },

    /// Sport-specific optimization
    SportSpecific {
        /// Primary sport
        sport: String,
        /// Sport-specific parameter overrides
        specialization_factors: HashMap<String, f64>,
    },

    /// Custom configuration
    Custom {
        /// Profile name
        name: String,
        /// Profile description
        description: String,
        /// Custom parameter overrides
        overrides: HashMap<String, f64>,
    },
}

impl ConfigProfile {
    /// Get the profile name
    #[must_use]
    pub fn name(&self) -> String {
        match self {
            Self::Default => "default".into(),
            Self::Research { .. } => "research".into(),
            Self::Elite { .. } => "elite".into(),
            Self::Recreational { .. } => "recreational".into(),
            Self::Beginner { .. } => "beginner".into(),
            Self::Medical { .. } => "medical".into(),
            Self::SportSpecific { sport, .. } => format!("sport_{}", sport.to_lowercase()),
            Self::Custom { name, .. } => name.clone(), // Safe: String ownership required for return value
        }
    }

    /// Create an elite profile from VO2 max
    #[must_use]
    pub fn elite_from_vo2_max(vo2_max: f64) -> Self {
        let performance_factor = match vo2_max {
            v if v >= 70.0 => 1.2,  // Professional level
            v if v >= 60.0 => 1.15, // Competitive amateur
            v if v >= 50.0 => 1.1,  // Strong recreational
            _ => 1.05,              // Standard elite
        };

        Self::Elite {
            performance_factor,
            recovery_sensitivity: 1.2,
        }
    }

    /// Get parameter adjustments for this profile
    #[must_use]
    pub fn get_adjustments(&self) -> HashMap<String, f64> {
        let mut adjustments = HashMap::new();

        match self {
            Self::Research {
                sensitivity_multiplier,
                ..
            } => {
                adjustments.insert("sensitivity_multiplier".into(), *sensitivity_multiplier);
                adjustments.insert("analysis_depth".into(), 2.0); // Double analysis depth
            }

            Self::Elite {
                performance_factor,
                recovery_sensitivity,
            } => {
                adjustments.insert("threshold_multiplier".into(), *performance_factor);
                adjustments.insert("recovery_sensitivity".into(), *recovery_sensitivity);
                adjustments.insert("performance_standards".into(), 1.15);
            }

            Self::Recreational {
                threshold_tolerance,
                ..
            } => {
                adjustments.insert("threshold_multiplier".into(), *threshold_tolerance);
                adjustments.insert("effort_scaling".into(), 0.9); // Slightly easier effort scores
            }

            Self::Beginner {
                threshold_reduction,
                ..
            } => {
                adjustments.insert("threshold_multiplier".into(), *threshold_reduction);
                adjustments.insert("zone_buffer".into(), 1.1); // 10% buffer between zones
                adjustments.insert("achievement_sensitivity".into(), 1.2); // More achievements
            }

            Self::Medical {
                max_intensity,
                safety_margin,
                ..
            } => {
                adjustments.insert("max_intensity".into(), *max_intensity);
                adjustments.insert("safety_margin".into(), *safety_margin);
                adjustments.insert("conservative_factor".into(), 0.8);
            }

            Self::SportSpecific {
                specialization_factors,
                ..
            } => {
                for (key, value) in specialization_factors {
                    adjustments.insert(key.clone(), *value); // Safe: HashMap.insert requires key ownership
                }
            }

            Self::Custom { overrides, .. } => {
                for (key, value) in overrides {
                    adjustments.insert(key.clone(), *value); // Safe: HashMap.insert requires key ownership
                }
            }

            Self::Default => {
                // No adjustments for default
            }
        }

        adjustments
    }
}

impl Default for ConfigProfile {
    fn default() -> Self {
        Self::Default
    }
}

/// Zone analysis granularity
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ZoneGranularity {
    /// Standard 5-zone model
    Standard,
    /// Fine 7-zone model
    Fine,
    /// Ultra-fine 10-zone model for research
    UltraFine,
}

/// Athlete fitness level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FitnessLevel {
    Beginner,
    Recreational,
    Intermediate,
    Advanced,
    Elite,
    Professional,
}

impl FitnessLevel {
    /// Get threshold adjustment factor for fitness level
    #[must_use]
    pub const fn threshold_factor(&self) -> f64 {
        match self {
            Self::Beginner => 0.85,
            Self::Recreational => 0.90,
            Self::Intermediate => 0.95,
            Self::Advanced => 1.0,
            Self::Elite => 1.05,
            Self::Professional => 1.1,
        }
    }

    /// Create from VO2 max value
    #[must_use]
    pub fn from_vo2_max(vo2_max: f64, age: Option<u16>, gender: Option<&str>) -> Self {
        // Adjust for age if provided
        let age_adjusted_vo2 = age.map_or(vo2_max, |age| {
            // VO2 max declines approximately 1% per year after age 25
            let age_factor = if age > 25 {
                f64::from(age - 25).mul_add(-0.01, 1.0)
            } else {
                1.0
            };
            vo2_max / age_factor.max(0.7)
        });

        // Get thresholds from configuration catalog
        let gender_prefix = match gender {
            Some("F" | "female") => "female",
            _ => "male", // Male or unspecified
        };

        let get_threshold = |level: &str| -> f64 {
            let key = format!("fitness.vo2_max_threshold_{gender_prefix}_{level}");
            CatalogBuilder::get_parameter(&key)
                .and_then(|param| match param.default_value {
                    super::runtime::ConfigValue::Float(v) => Some(v),
                    _ => None,
                })
                .unwrap_or({
                    // Fallback values if parameter not found
                    match (gender_prefix, level) {
                        ("female", "beginner") => 30.0,
                        ("female", "recreational") | ("male", "beginner") => 35.0,
                        ("female", "intermediate") => 42.0,
                        ("female", "advanced") | ("male", "recreational") => {
                            if gender_prefix == "female" {
                                50.0
                            } else {
                                42.0
                            }
                        }
                        ("female", "elite") | ("male", "intermediate") => {
                            if gender_prefix == "female" {
                                55.0
                            } else {
                                50.0
                            }
                        }
                        ("male", "advanced") => 55.0,
                        ("male", "elite") => 60.0,
                        _ => 50.0,
                    }
                })
        };

        let beginner_threshold = get_threshold("beginner");
        let recreational_threshold = get_threshold("recreational");
        let intermediate_threshold = get_threshold("intermediate");
        let advanced_threshold = get_threshold("advanced");
        let elite_threshold = get_threshold("elite");

        match age_adjusted_vo2 {
            v if v < beginner_threshold => Self::Beginner,
            v if v < recreational_threshold => Self::Recreational,
            v if v < intermediate_threshold => Self::Intermediate,
            v if v < advanced_threshold => Self::Advanced,
            v if v < elite_threshold => Self::Elite,
            _ => Self::Professional,
        }
    }
}

/// Profile templates for quick setup
pub struct ProfileTemplates;

impl ProfileTemplates {
    /// Get all available profile templates
    #[must_use]
    pub fn all() -> Vec<(String, ConfigProfile)> {
        vec![
            ("Default".into(), ConfigProfile::Default),
            (
                "Research".into(),
                ConfigProfile::Research {
                    sensitivity_multiplier: 1.5,
                    zone_granularity: ZoneGranularity::Fine,
                    statistical_confidence: 0.95,
                },
            ),
            (
                "Elite Athlete".into(),
                ConfigProfile::Elite {
                    performance_factor: 1.15,
                    recovery_sensitivity: 1.2,
                },
            ),
            (
                "Recreational Athlete".into(),
                ConfigProfile::Recreational {
                    motivation_bias: 0.1,
                    threshold_tolerance: 1.1,
                },
            ),
            (
                "Beginner".into(),
                ConfigProfile::Beginner {
                    threshold_reduction: 0.85,
                    simplified_metrics: true,
                },
            ),
            (
                "Medical/Rehab".into(),
                ConfigProfile::Medical {
                    max_intensity: 0.75,
                    conservative_thresholds: true,
                    safety_margin: 1.2,
                },
            ),
            (
                "Cycling Specialist".into(),
                ConfigProfile::SportSpecific {
                    sport: "cycling".into(),
                    specialization_factors: HashMap::from([
                        ("power_weight_importance".into(), 1.2),
                        ("aerodynamic_factor".into(), 1.1),
                        ("ftp_calculation_method".into(), 0.95),
                    ]),
                },
            ),
            (
                "Running Specialist".into(),
                ConfigProfile::SportSpecific {
                    sport: "running".into(),
                    specialization_factors: HashMap::from([
                        ("running_economy_factor".into(), 1.15),
                        ("cadence_importance".into(), 1.1),
                        ("vertical_oscillation_penalty".into(), 1.2),
                    ]),
                },
            ),
            (
                "Swimming Specialist".into(),
                ConfigProfile::SportSpecific {
                    sport: "swimming".into(),
                    specialization_factors: HashMap::from([
                        ("stroke_efficiency_weight".into(), 1.3),
                        ("breathing_pattern_factor".into(), 1.1),
                        ("turn_efficiency".into(), 1.05),
                    ]),
                },
            ),
        ]
    }

    /// Get a profile template by name
    #[must_use]
    pub fn get(name: &str) -> Option<ConfigProfile> {
        Self::all()
            .into_iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, profile)| profile)
    }
}
