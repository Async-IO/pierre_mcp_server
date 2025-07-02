// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Configuration profiles for different user types and use cases

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
    pub fn name(&self) -> String {
        match self {
            Self::Default => "default".to_string(),
            Self::Research { .. } => "research".to_string(),
            Self::Elite { .. } => "elite".to_string(),
            Self::Recreational { .. } => "recreational".to_string(),
            Self::Beginner { .. } => "beginner".to_string(),
            Self::Medical { .. } => "medical".to_string(),
            Self::SportSpecific { sport, .. } => format!("sport_{}", sport.to_lowercase()),
            Self::Custom { name, .. } => name.clone(),
        }
    }

    /// Create an elite profile from VO2 max
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
    pub fn get_adjustments(&self) -> HashMap<String, f64> {
        let mut adjustments = HashMap::new();

        match self {
            Self::Research {
                sensitivity_multiplier,
                ..
            } => {
                adjustments.insert(
                    "sensitivity_multiplier".to_string(),
                    *sensitivity_multiplier,
                );
                adjustments.insert("analysis_depth".to_string(), 2.0); // Double analysis depth
            }

            Self::Elite {
                performance_factor,
                recovery_sensitivity,
            } => {
                adjustments.insert("threshold_multiplier".to_string(), *performance_factor);
                adjustments.insert("recovery_sensitivity".to_string(), *recovery_sensitivity);
                adjustments.insert("performance_standards".to_string(), 1.15);
            }

            Self::Recreational {
                threshold_tolerance,
                ..
            } => {
                adjustments.insert("threshold_multiplier".to_string(), *threshold_tolerance);
                adjustments.insert("effort_scaling".to_string(), 0.9); // Slightly easier effort scores
            }

            Self::Beginner {
                threshold_reduction,
                ..
            } => {
                adjustments.insert("threshold_multiplier".to_string(), *threshold_reduction);
                adjustments.insert("zone_buffer".to_string(), 1.1); // 10% buffer between zones
                adjustments.insert("achievement_sensitivity".to_string(), 1.2); // More achievements
            }

            Self::Medical {
                max_intensity,
                safety_margin,
                ..
            } => {
                adjustments.insert("max_intensity".to_string(), *max_intensity);
                adjustments.insert("safety_margin".to_string(), *safety_margin);
                adjustments.insert("conservative_factor".to_string(), 0.8);
            }

            Self::SportSpecific {
                specialization_factors,
                ..
            } => {
                for (key, value) in specialization_factors {
                    adjustments.insert(key.clone(), *value);
                }
            }

            Self::Custom { overrides, .. } => {
                for (key, value) in overrides {
                    adjustments.insert(key.clone(), *value);
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ZoneGranularity {
    /// Standard 5-zone model
    Standard,
    /// Fine 7-zone model
    Fine,
    /// Ultra-fine 10-zone model for research
    UltraFine,
}

/// Athlete fitness level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
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
    pub fn threshold_factor(&self) -> f64 {
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
    pub fn from_vo2_max(vo2_max: f64, age: Option<u16>, gender: Option<&str>) -> Self {
        // Adjust for age if provided
        let age_adjusted_vo2 = if let Some(age) = age {
            // VO2 max declines approximately 1% per year after age 25
            let age_factor = if age > 25 {
                1.0 - ((age - 25) as f64 * 0.01)
            } else {
                1.0
            };
            vo2_max / age_factor.max(0.7)
        } else {
            vo2_max
        };

        // Get thresholds from configuration catalog
        use super::catalog::CatalogBuilder;
        let gender_prefix = match gender {
            Some("F") | Some("female") => "female",
            _ => "male", // Male or unspecified
        };

        let get_threshold = |level: &str| -> f64 {
            let key = format!("fitness.vo2_max_threshold_{}_{}", gender_prefix, level);
            CatalogBuilder::get_parameter(&key)
                .and_then(|param| match param.default_value {
                    super::runtime::ConfigValue::Float(v) => Some(v),
                    _ => None,
                })
                .unwrap_or({
                    // Fallback values if parameter not found
                    match (gender_prefix, level) {
                        ("female", "beginner") => 30.0,
                        ("female", "recreational") => 35.0,
                        ("female", "intermediate") => 42.0,
                        ("female", "advanced") => 50.0,
                        ("female", "elite") => 55.0,
                        ("male", "beginner") => 35.0,
                        ("male", "recreational") => 42.0,
                        ("male", "intermediate") => 50.0,
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
    pub fn all() -> Vec<(String, ConfigProfile)> {
        vec![
            ("Default".to_string(), ConfigProfile::Default),
            (
                "Research".to_string(),
                ConfigProfile::Research {
                    sensitivity_multiplier: 1.5,
                    zone_granularity: ZoneGranularity::Fine,
                    statistical_confidence: 0.95,
                },
            ),
            (
                "Elite Athlete".to_string(),
                ConfigProfile::Elite {
                    performance_factor: 1.15,
                    recovery_sensitivity: 1.2,
                },
            ),
            (
                "Recreational Athlete".to_string(),
                ConfigProfile::Recreational {
                    motivation_bias: 0.1,
                    threshold_tolerance: 1.1,
                },
            ),
            (
                "Beginner".to_string(),
                ConfigProfile::Beginner {
                    threshold_reduction: 0.85,
                    simplified_metrics: true,
                },
            ),
            (
                "Medical/Rehab".to_string(),
                ConfigProfile::Medical {
                    max_intensity: 0.75,
                    conservative_thresholds: true,
                    safety_margin: 1.2,
                },
            ),
            (
                "Cycling Specialist".to_string(),
                ConfigProfile::SportSpecific {
                    sport: "cycling".to_string(),
                    specialization_factors: HashMap::from([
                        ("power_weight_importance".to_string(), 1.2),
                        ("aerodynamic_factor".to_string(), 1.1),
                        ("ftp_calculation_method".to_string(), 0.95),
                    ]),
                },
            ),
            (
                "Running Specialist".to_string(),
                ConfigProfile::SportSpecific {
                    sport: "running".to_string(),
                    specialization_factors: HashMap::from([
                        ("running_economy_factor".to_string(), 1.15),
                        ("cadence_importance".to_string(), 1.1),
                        ("vertical_oscillation_penalty".to_string(), 1.2),
                    ]),
                },
            ),
            (
                "Swimming Specialist".to_string(),
                ConfigProfile::SportSpecific {
                    sport: "swimming".to_string(),
                    specialization_factors: HashMap::from([
                        ("stroke_efficiency_weight".to_string(), 1.3),
                        ("breathing_pattern_factor".to_string(), 1.1),
                        ("turn_efficiency".to_string(), 1.05),
                    ]),
                },
            ),
        ]
    }

    /// Get a profile template by name
    pub fn get(name: &str) -> Option<ConfigProfile> {
        Self::all()
            .into_iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, profile)| profile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_names() {
        assert_eq!(ConfigProfile::Default.name(), "default");
        assert_eq!(
            ConfigProfile::Elite {
                performance_factor: 1.1,
                recovery_sensitivity: 1.2
            }
            .name(),
            "elite"
        );
    }

    #[test]
    fn test_elite_from_vo2_max() {
        let profile = ConfigProfile::elite_from_vo2_max(65.0);
        if let ConfigProfile::Elite {
            performance_factor, ..
        } = profile
        {
            assert_eq!(performance_factor, 1.15);
        } else {
            panic!("Expected Elite profile");
        }
    }

    #[test]
    fn test_fitness_level_from_vo2_max() {
        assert_eq!(
            FitnessLevel::from_vo2_max(35.0, None, Some("M")),
            FitnessLevel::Recreational
        );
        assert_eq!(
            FitnessLevel::from_vo2_max(55.0, None, Some("M")),
            FitnessLevel::Elite
        );
        assert_eq!(
            FitnessLevel::from_vo2_max(45.0, None, Some("F")),
            FitnessLevel::Advanced
        );
    }

    #[test]
    fn test_profile_adjustments() {
        let profile = ConfigProfile::Beginner {
            threshold_reduction: 0.85,
            simplified_metrics: true,
        };

        let adjustments = profile.get_adjustments();
        assert_eq!(adjustments.get("threshold_multiplier"), Some(&0.85));
        assert_eq!(adjustments.get("achievement_sensitivity"), Some(&1.2));
    }

    #[test]
    fn test_profile_templates() {
        let templates = ProfileTemplates::all();
        assert!(templates.len() >= 9);

        let research = ProfileTemplates::get("research");
        assert!(research.is_some());
        assert!(matches!(research.unwrap(), ConfigProfile::Research { .. }));
    }
}
