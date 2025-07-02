// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Configuration catalog for discovering available parameters

use super::runtime::ConfigValue;
use serde::{Deserialize, Serialize};

/// Complete configuration catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigCatalog {
    /// Configuration categories
    pub categories: Vec<ConfigCategory>,
    /// Total number of configurable parameters
    pub total_parameters: usize,
    /// Catalog version
    pub version: String,
}

/// Configuration category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigCategory {
    /// Category name
    pub name: String,
    /// Category description
    pub description: String,
    /// Modules in this category
    pub modules: Vec<ConfigModule>,
}

/// Configuration module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigModule {
    /// Module name
    pub name: String,
    /// Module description
    pub description: String,
    /// Parameters in this module
    pub parameters: Vec<ConfigParameter>,
}

/// Configuration parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigParameter {
    /// Parameter key (e.g., "heart_rate.anaerobic_threshold")
    pub key: String,
    /// Human-readable description
    pub description: String,
    /// Data type
    pub data_type: ParameterType,
    /// Default value
    pub default_value: ConfigValue,
    /// Valid range (if applicable)
    pub valid_range: Option<ConfigValue>,
    /// Units (e.g., "percentage", "bpm", "seconds")
    pub units: Option<String>,
    /// Scientific basis or reference
    pub scientific_basis: Option<String>,
    /// Whether this parameter requires VO2 max data
    pub requires_vo2_max: bool,
}

/// Parameter data types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    Float,
    Integer,
    Boolean,
    String,
}

/// Catalog builder for creating the configuration catalog
pub struct CatalogBuilder;

impl CatalogBuilder {
    /// Build the complete configuration catalog
    pub fn build() -> ConfigCatalog {
        let categories = vec![
            Self::build_physiological_zones_category(),
            Self::build_performance_calculation_category(),
            Self::build_sport_specific_category(),
            Self::build_analysis_settings_category(),
            Self::build_safety_constraints_category(),
        ];

        let total_parameters = categories
            .iter()
            .flat_map(|cat| &cat.modules)
            .map(|module| module.parameters.len())
            .sum();

        ConfigCatalog {
            categories,
            total_parameters,
            version: "1.0.0".to_string(),
        }
    }

    /// Build physiological zones category
    fn build_physiological_zones_category() -> ConfigCategory {
        ConfigCategory {
            name: "physiological_zones".to_string(),
            description: "Heart rate zones, lactate thresholds, and VO2 max calculations"
                .to_string(),
            modules: vec![
                ConfigModule {
                    name: "heart_rate".to_string(),
                    description: "Heart rate zone thresholds and calculations".to_string(),
                    parameters: vec![
                        ConfigParameter {
                            key: "heart_rate.anaerobic_threshold".to_string(),
                            description: "Anaerobic threshold as percentage of max HR".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(85.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 70.0,
                                max: 95.0,
                            }),
                            units: Some("percentage".to_string()),
                            scientific_basis: Some("Seiler 2010, Laursen 2002".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "heart_rate.vo2_max_zone".to_string(),
                            description: "VO2 max zone as percentage of max HR".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(95.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 85.0,
                                max: 100.0,
                            }),
                            units: Some("percentage".to_string()),
                            scientific_basis: Some("Buchheit & Laursen 2013".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "heart_rate.tempo_zone".to_string(),
                            description: "Tempo/threshold zone as percentage of max HR".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(80.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 70.0,
                                max: 90.0,
                            }),
                            units: Some("percentage".to_string()),
                            scientific_basis: Some("Coggan & Allen 2006".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "heart_rate.endurance_zone".to_string(),
                            description: "Aerobic endurance zone as percentage of max HR"
                                .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(70.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 60.0,
                                max: 80.0,
                            }),
                            units: Some("percentage".to_string()),
                            scientific_basis: Some("Maffetone Method".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "heart_rate.recovery_zone".to_string(),
                            description: "Active recovery zone as percentage of max HR".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(60.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 50.0,
                                max: 70.0,
                            }),
                            units: Some("percentage".to_string()),
                            scientific_basis: Some("Polarized Training Model".to_string()),
                            requires_vo2_max: false,
                        },
                    ],
                },
                ConfigModule {
                    name: "lactate".to_string(),
                    description: "Lactate threshold and metabolic parameters".to_string(),
                    parameters: vec![
                        ConfigParameter {
                            key: "lactate.threshold_percentage".to_string(),
                            description: "Lactate threshold as percentage of VO2 max".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(85.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 65.0,
                                max: 95.0,
                            }),
                            units: Some("percentage".to_string()),
                            scientific_basis: Some("Faude et al. 2009".to_string()),
                            requires_vo2_max: true,
                        },
                        ConfigParameter {
                            key: "lactate.accumulation_rate".to_string(),
                            description: "Rate of lactate accumulation above threshold".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(4.0),
                            valid_range: Some(ConfigValue::FloatRange { min: 2.0, max: 8.0 }),
                            units: Some("mmol/L/min".to_string()),
                            scientific_basis: Some("Beneke 2003".to_string()),
                            requires_vo2_max: true,
                        },
                    ],
                },
                ConfigModule {
                    name: "fitness_levels".to_string(),
                    description: "VO2 max thresholds for fitness level classification".to_string(),
                    parameters: vec![
                        ConfigParameter {
                            key: "fitness.vo2_max_threshold_male_beginner".to_string(),
                            description: "VO2 max threshold for beginner level (males)".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(35.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 25.0,
                                max: 45.0,
                            }),
                            units: Some("ml/kg/min".to_string()),
                            scientific_basis: Some("ACSM Guidelines 2018".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "fitness.vo2_max_threshold_male_recreational".to_string(),
                            description: "VO2 max threshold for recreational level (males)"
                                .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(42.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 35.0,
                                max: 50.0,
                            }),
                            units: Some("ml/kg/min".to_string()),
                            scientific_basis: Some("ACSM Guidelines 2018".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "fitness.vo2_max_threshold_male_intermediate".to_string(),
                            description: "VO2 max threshold for intermediate level (males)"
                                .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(50.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 42.0,
                                max: 58.0,
                            }),
                            units: Some("ml/kg/min".to_string()),
                            scientific_basis: Some("ACSM Guidelines 2018".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "fitness.vo2_max_threshold_male_advanced".to_string(),
                            description: "VO2 max threshold for advanced level (males)".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(55.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 50.0,
                                max: 65.0,
                            }),
                            units: Some("ml/kg/min".to_string()),
                            scientific_basis: Some("ACSM Guidelines 2018".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "fitness.vo2_max_threshold_male_elite".to_string(),
                            description: "VO2 max threshold for elite level (males)".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(60.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 55.0,
                                max: 70.0,
                            }),
                            units: Some("ml/kg/min".to_string()),
                            scientific_basis: Some("ACSM Guidelines 2018".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "fitness.vo2_max_threshold_female_beginner".to_string(),
                            description: "VO2 max threshold for beginner level (females)"
                                .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(30.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 20.0,
                                max: 40.0,
                            }),
                            units: Some("ml/kg/min".to_string()),
                            scientific_basis: Some("ACSM Guidelines 2018".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "fitness.vo2_max_threshold_female_recreational".to_string(),
                            description: "VO2 max threshold for recreational level (females)"
                                .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(35.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 30.0,
                                max: 45.0,
                            }),
                            units: Some("ml/kg/min".to_string()),
                            scientific_basis: Some("ACSM Guidelines 2018".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "fitness.vo2_max_threshold_female_intermediate".to_string(),
                            description: "VO2 max threshold for intermediate level (females)"
                                .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(42.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 35.0,
                                max: 50.0,
                            }),
                            units: Some("ml/kg/min".to_string()),
                            scientific_basis: Some("ACSM Guidelines 2018".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "fitness.vo2_max_threshold_female_advanced".to_string(),
                            description: "VO2 max threshold for advanced level (females)"
                                .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(50.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 42.0,
                                max: 58.0,
                            }),
                            units: Some("ml/kg/min".to_string()),
                            scientific_basis: Some("ACSM Guidelines 2018".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "fitness.vo2_max_threshold_female_elite".to_string(),
                            description: "VO2 max threshold for elite level (females)".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(55.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 50.0,
                                max: 65.0,
                            }),
                            units: Some("ml/kg/min".to_string()),
                            scientific_basis: Some("ACSM Guidelines 2018".to_string()),
                            requires_vo2_max: false,
                        },
                    ],
                },
                ConfigModule {
                    name: "heart_rate_zones".to_string(),
                    description: "Heart rate zone percentages for different fitness levels"
                        .to_string(),
                    parameters: vec![
                        ConfigParameter {
                            key: "hr_zones.elite_zone6_threshold".to_string(),
                            description:
                                "VO2 max threshold for zone 6 availability (elite athletes)"
                                    .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(50.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 45.0,
                                max: 60.0,
                            }),
                            units: Some("ml/kg/min".to_string()),
                            scientific_basis: Some("Elite training zone research".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "hr_zones.lactate_threshold_min".to_string(),
                            description: "Minimum lactate threshold percentage".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(0.65),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 0.60,
                                max: 0.70,
                            }),
                            units: Some("percentage".to_string()),
                            scientific_basis: Some("Lactate threshold research".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "hr_zones.lactate_threshold_max".to_string(),
                            description: "Maximum lactate threshold percentage".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(0.95),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 0.90,
                                max: 1.00,
                            }),
                            units: Some("percentage".to_string()),
                            scientific_basis: Some("Lactate threshold research".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "hr_zones.sport_efficiency_min".to_string(),
                            description: "Minimum sport efficiency factor".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(0.5),
                            valid_range: Some(ConfigValue::FloatRange { min: 0.3, max: 0.7 }),
                            units: None,
                            scientific_basis: Some("Sport efficiency research".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "hr_zones.sport_efficiency_max".to_string(),
                            description: "Maximum sport efficiency factor".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(1.5),
                            valid_range: Some(ConfigValue::FloatRange { min: 1.0, max: 2.0 }),
                            units: None,
                            scientific_basis: Some("Sport efficiency research".to_string()),
                            requires_vo2_max: false,
                        },
                    ],
                },
                ConfigModule {
                    name: "vo2_calculations".to_string(),
                    description: "VO2 max calculation constants and formulas".to_string(),
                    parameters: vec![
                        ConfigParameter {
                            key: "vo2.vdot_coefficient_a".to_string(),
                            description: "VDOT formula coefficient A (velocity calculation)"
                                .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(29.54),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 25.0,
                                max: 35.0,
                            }),
                            units: Some("m/min".to_string()),
                            scientific_basis: Some("Jack Daniels Running Formula".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "vo2.vdot_coefficient_b".to_string(),
                            description: "VDOT formula coefficient B (linear term)".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(5.000663),
                            valid_range: Some(ConfigValue::FloatRange { min: 4.0, max: 6.0 }),
                            units: Some("(m/min)/(ml/kg/min)".to_string()),
                            scientific_basis: Some("Jack Daniels Running Formula".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "vo2.vdot_coefficient_c".to_string(),
                            description: "VDOT formula coefficient C (quadratic term)".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(0.007546),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 0.005,
                                max: 0.01,
                            }),
                            units: Some("(m/min)/(ml/kg/min)²".to_string()),
                            scientific_basis: Some("Jack Daniels Running Formula".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "vo2.threshold_velocity_base".to_string(),
                            description: "Base threshold velocity as percentage of vVO2max"
                                .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(0.86),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 0.80,
                                max: 0.95,
                            }),
                            units: Some("percentage".to_string()),
                            scientific_basis: Some("Lactate threshold studies".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "vo2.threshold_adjustment_factor".to_string(),
                            description: "Threshold adjustment factor for lactate threshold"
                                .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(0.4),
                            valid_range: Some(ConfigValue::FloatRange { min: 0.2, max: 0.6 }),
                            units: None,
                            scientific_basis: Some("Lactate threshold variability".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "vo2.power_coefficient".to_string(),
                            description: "Power at VO2 max coefficient (W per ml/kg/min)"
                                .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(13.5),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 12.0,
                                max: 15.0,
                            }),
                            units: Some("W/(ml/kg/min)".to_string()),
                            scientific_basis: Some("Power-VO2 relationship studies".to_string()),
                            requires_vo2_max: false,
                        },
                    ],
                },
                ConfigModule {
                    name: "pace_zones".to_string(),
                    description: "Running pace zone percentages and calculations".to_string(),
                    parameters: vec![
                        ConfigParameter {
                            key: "pace.easy_zone_low".to_string(),
                            description: "Easy pace zone lower bound (% of vVO2max)".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(0.59),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 0.50,
                                max: 0.65,
                            }),
                            units: Some("percentage".to_string()),
                            scientific_basis: Some("Training zone research".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "pace.easy_zone_high".to_string(),
                            description: "Easy pace zone upper bound (% of vVO2max)".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(0.74),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 0.70,
                                max: 0.80,
                            }),
                            units: Some("percentage".to_string()),
                            scientific_basis: Some("Training zone research".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "pace.marathon_adjustment_low".to_string(),
                            description: "Marathon pace adjustment factor (slower)".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(1.06),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 1.02,
                                max: 1.10,
                            }),
                            units: Some("multiplier".to_string()),
                            scientific_basis: Some("Marathon pace studies".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "pace.marathon_adjustment_high".to_string(),
                            description: "Marathon pace adjustment factor (faster)".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(1.02),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 1.00,
                                max: 1.05,
                            }),
                            units: Some("multiplier".to_string()),
                            scientific_basis: Some("Marathon pace studies".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "pace.threshold_adjustment_low".to_string(),
                            description: "Threshold pace adjustment factor (slower)".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(1.02),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 1.00,
                                max: 1.05,
                            }),
                            units: Some("multiplier".to_string()),
                            scientific_basis: Some("Threshold pace studies".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "pace.threshold_adjustment_high".to_string(),
                            description: "Threshold pace adjustment factor (faster)".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(0.98),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 0.95,
                                max: 1.00,
                            }),
                            units: Some("multiplier".to_string()),
                            scientific_basis: Some("Threshold pace studies".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "pace.vo2max_zone_percentage".to_string(),
                            description: "VO2 max pace zone percentage of vVO2max".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(0.95),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 0.90,
                                max: 1.00,
                            }),
                            units: Some("percentage".to_string()),
                            scientific_basis: Some("VO2 max training studies".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "pace.neuromuscular_zone_percentage".to_string(),
                            description: "Neuromuscular pace zone percentage of vVO2max"
                                .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(1.05),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 1.00,
                                max: 1.15,
                            }),
                            units: Some("percentage".to_string()),
                            scientific_basis: Some("Neuromuscular training studies".to_string()),
                            requires_vo2_max: false,
                        },
                    ],
                },
                ConfigModule {
                    name: "ftp_calculation".to_string(),
                    description: "Functional Threshold Power calculation parameters".to_string(),
                    parameters: vec![
                        ConfigParameter {
                            key: "ftp.elite_percentage".to_string(),
                            description: "FTP percentage for elite athletes (VO2 max >= 60)"
                                .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(0.85),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 0.80,
                                max: 0.90,
                            }),
                            units: Some("percentage".to_string()),
                            scientific_basis: Some("Elite athlete FTP studies".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "ftp.advanced_percentage".to_string(),
                            description: "FTP percentage for advanced athletes (VO2 max >= 50)"
                                .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(0.82),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 0.75,
                                max: 0.85,
                            }),
                            units: Some("percentage".to_string()),
                            scientific_basis: Some("Advanced athlete FTP studies".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "ftp.intermediate_percentage".to_string(),
                            description: "FTP percentage for intermediate athletes (VO2 max >= 40)"
                                .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(0.78),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 0.70,
                                max: 0.82,
                            }),
                            units: Some("percentage".to_string()),
                            scientific_basis: Some("Intermediate athlete FTP studies".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "ftp.beginner_percentage".to_string(),
                            description: "FTP percentage for beginner athletes".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(0.75),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 0.65,
                                max: 0.80,
                            }),
                            units: Some("percentage".to_string()),
                            scientific_basis: Some("Beginner athlete FTP studies".to_string()),
                            requires_vo2_max: false,
                        },
                    ],
                },
            ],
        }
    }

    /// Build performance calculation category
    fn build_performance_calculation_category() -> ConfigCategory {
        ConfigCategory {
            name: "performance_calculation".to_string(),
            description: "Effort scoring, efficiency calculations, and performance metrics"
                .to_string(),
            modules: vec![
                ConfigModule {
                    name: "effort_scoring".to_string(),
                    description: "Parameters for calculating relative effort scores".to_string(),
                    parameters: vec![
                        ConfigParameter {
                            key: "performance.run_distance_divisor".to_string(),
                            description:
                                "Divisor for normalizing running distance in effort calculation"
                                    .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(10.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 5.0,
                                max: 20.0,
                            }),
                            units: Some("km".to_string()),
                            scientific_basis: Some("ACSM Guidelines 2018".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "performance.bike_distance_divisor".to_string(),
                            description:
                                "Divisor for normalizing cycling distance in effort calculation"
                                    .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(40.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 20.0,
                                max: 60.0,
                            }),
                            units: Some("km".to_string()),
                            scientific_basis: Some("Coggan Power Training".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "performance.swim_distance_divisor".to_string(),
                            description:
                                "Divisor for normalizing swimming distance in effort calculation"
                                    .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(2.0),
                            valid_range: Some(ConfigValue::FloatRange { min: 1.0, max: 5.0 }),
                            units: Some("km".to_string()),
                            scientific_basis: Some("Costill et al. 1985".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "performance.elevation_divisor".to_string(),
                            description: "Divisor for elevation gain in effort calculation"
                                .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(100.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 50.0,
                                max: 200.0,
                            }),
                            units: Some("meters".to_string()),
                            scientific_basis: Some("Minetti et al. 2002".to_string()),
                            requires_vo2_max: false,
                        },
                    ],
                },
                ConfigModule {
                    name: "efficiency".to_string(),
                    description: "Efficiency scoring and economy calculations".to_string(),
                    parameters: vec![
                        ConfigParameter {
                            key: "efficiency.base_score".to_string(),
                            description: "Base efficiency score for all activities".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(50.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 0.0,
                                max: 100.0,
                            }),
                            units: Some("points".to_string()),
                            scientific_basis: None,
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "efficiency.hr_factor".to_string(),
                            description: "Heart rate efficiency calculation factor".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(1000.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 500.0,
                                max: 2000.0,
                            }),
                            units: None,
                            scientific_basis: Some("HR:Pace ratio studies".to_string()),
                            requires_vo2_max: false,
                        },
                    ],
                },
            ],
        }
    }

    /// Build sport-specific category
    fn build_sport_specific_category() -> ConfigCategory {
        ConfigCategory {
            name: "sport_specific".to_string(),
            description: "Sport-specific performance calculations and thresholds".to_string(),
            modules: vec![
                ConfigModule {
                    name: "cycling".to_string(),
                    description: "Cycling-specific parameters".to_string(),
                    parameters: vec![
                        ConfigParameter {
                            key: "cycling.ftp_percentage".to_string(),
                            description: "FTP as percentage of 20-minute power".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(95.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 90.0,
                                max: 98.0,
                            }),
                            units: Some("percentage".to_string()),
                            scientific_basis: Some("Allen & Coggan 2010".to_string()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "cycling.power_weight_importance".to_string(),
                            description: "Importance of power-to-weight ratio".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(1.0),
                            valid_range: Some(ConfigValue::FloatRange { min: 0.5, max: 2.0 }),
                            units: None,
                            scientific_basis: None,
                            requires_vo2_max: false,
                        },
                    ],
                },
                ConfigModule {
                    name: "running".to_string(),
                    description: "Running-specific parameters".to_string(),
                    parameters: vec![
                        ConfigParameter {
                            key: "running.economy_factor".to_string(),
                            description: "Running economy adjustment factor".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(1.0),
                            valid_range: Some(ConfigValue::FloatRange { min: 0.7, max: 1.3 }),
                            units: None,
                            scientific_basis: Some("Daniels Running Formula".to_string()),
                            requires_vo2_max: true,
                        },
                        ConfigParameter {
                            key: "running.cadence_target".to_string(),
                            description: "Target running cadence".to_string(),
                            data_type: ParameterType::Integer,
                            default_value: ConfigValue::Integer(180),
                            valid_range: Some(ConfigValue::IntegerRange { min: 160, max: 200 }),
                            units: Some("steps/min".to_string()),
                            scientific_basis: Some("Heiderscheit et al. 2011".to_string()),
                            requires_vo2_max: false,
                        },
                    ],
                },
                ConfigModule {
                    name: "swimming".to_string(),
                    description: "Swimming-specific parameters".to_string(),
                    parameters: vec![ConfigParameter {
                        key: "swimming.stroke_efficiency".to_string(),
                        description: "Stroke efficiency factor".to_string(),
                        data_type: ParameterType::Float,
                        default_value: ConfigValue::Float(0.75),
                        valid_range: Some(ConfigValue::FloatRange { min: 0.5, max: 1.0 }),
                        units: None,
                        scientific_basis: Some("Toussaint & Beek 1992".to_string()),
                        requires_vo2_max: false,
                    }],
                },
            ],
        }
    }

    /// Build analysis settings category
    fn build_analysis_settings_category() -> ConfigCategory {
        ConfigCategory {
            name: "analysis_settings".to_string(),
            description: "Settings for activity analysis and insights".to_string(),
            modules: vec![
                ConfigModule {
                    name: "insights".to_string(),
                    description: "Insight generation parameters".to_string(),
                    parameters: vec![
                        ConfigParameter {
                            key: "insights.min_confidence".to_string(),
                            description: "Minimum confidence threshold for insights".to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(70.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 50.0,
                                max: 95.0,
                            }),
                            units: Some("percentage".to_string()),
                            scientific_basis: None,
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "insights.max_per_activity".to_string(),
                            description: "Maximum insights per activity".to_string(),
                            data_type: ParameterType::Integer,
                            default_value: ConfigValue::Integer(5),
                            valid_range: Some(ConfigValue::IntegerRange { min: 1, max: 10 }),
                            units: None,
                            scientific_basis: None,
                            requires_vo2_max: false,
                        },
                    ],
                },
                ConfigModule {
                    name: "anomaly_detection".to_string(),
                    description: "Anomaly detection thresholds".to_string(),
                    parameters: vec![ConfigParameter {
                        key: "anomaly.hr_spike_threshold".to_string(),
                        description: "Heart rate spike detection threshold".to_string(),
                        data_type: ParameterType::Float,
                        default_value: ConfigValue::Float(20.0),
                        valid_range: Some(ConfigValue::FloatRange {
                            min: 10.0,
                            max: 40.0,
                        }),
                        units: Some("bpm/min".to_string()),
                        scientific_basis: None,
                        requires_vo2_max: false,
                    }],
                },
            ],
        }
    }

    /// Build safety constraints category
    fn build_safety_constraints_category() -> ConfigCategory {
        ConfigCategory {
            name: "safety_constraints".to_string(),
            description: "Safety limits and medical constraints".to_string(),
            modules: vec![ConfigModule {
                name: "intensity_limits".to_string(),
                description: "Maximum intensity constraints".to_string(),
                parameters: vec![
                    ConfigParameter {
                        key: "safety.max_hr_percentage".to_string(),
                        description: "Maximum allowed heart rate percentage".to_string(),
                        data_type: ParameterType::Float,
                        default_value: ConfigValue::Float(100.0),
                        valid_range: Some(ConfigValue::FloatRange {
                            min: 60.0,
                            max: 100.0,
                        }),
                        units: Some("percentage".to_string()),
                        scientific_basis: Some("ACSM Exercise Guidelines".to_string()),
                        requires_vo2_max: false,
                    },
                    ConfigParameter {
                        key: "safety.recovery_multiplier".to_string(),
                        description: "Recovery time multiplier for safety".to_string(),
                        data_type: ParameterType::Float,
                        default_value: ConfigValue::Float(1.0),
                        valid_range: Some(ConfigValue::FloatRange { min: 1.0, max: 3.0 }),
                        units: None,
                        scientific_basis: None,
                        requires_vo2_max: false,
                    },
                ],
            }],
        }
    }

    /// Get a specific parameter by key
    pub fn get_parameter(key: &str) -> Option<ConfigParameter> {
        let catalog = Self::build();

        catalog
            .categories
            .into_iter()
            .flat_map(|cat| cat.modules)
            .flat_map(|module| module.parameters)
            .find(|param| param.key == key)
    }

    /// Get all parameters for a module
    pub fn get_module_parameters(module_name: &str) -> Vec<ConfigParameter> {
        let catalog = Self::build();

        catalog
            .categories
            .into_iter()
            .flat_map(|cat| cat.modules)
            .filter(|module| module.name == module_name)
            .flat_map(|module| module.parameters)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_build() {
        let catalog = CatalogBuilder::build();

        assert!(!catalog.categories.is_empty());
        assert!(catalog.total_parameters > 0);
        assert_eq!(catalog.version, "1.0.0");
    }

    #[test]
    fn test_parameter_lookup() {
        let param = CatalogBuilder::get_parameter("heart_rate.anaerobic_threshold");
        assert!(param.is_some());

        let param = param.unwrap();
        assert_eq!(param.key, "heart_rate.anaerobic_threshold");
        assert!(matches!(param.default_value, ConfigValue::Float(85.0)));
    }

    #[test]
    fn test_module_parameters() {
        let params = CatalogBuilder::get_module_parameters("heart_rate");
        assert!(!params.is_empty());
        assert!(params.iter().all(|p| p.key.starts_with("heart_rate.")));
    }

    #[test]
    fn test_valid_ranges() {
        let catalog = CatalogBuilder::build();

        for category in &catalog.categories {
            for module in &category.modules {
                for param in &module.parameters {
                    if let Some(range) = &param.valid_range {
                        match (&param.data_type, range) {
                            (ParameterType::Float, ConfigValue::FloatRange { min, max }) => {
                                assert!(min < max, "Invalid range for {}", param.key);
                            }
                            (ParameterType::Integer, ConfigValue::IntegerRange { min, max }) => {
                                assert!(min < max, "Invalid range for {}", param.key);
                            }
                            _ => panic!("Mismatched range type for {}", param.key),
                        }
                    }
                }
            }
        }
    }
}
