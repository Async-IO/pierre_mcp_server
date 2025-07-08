// ABOUTME: Configuration catalog defining available config parameters and defaults
// ABOUTME: Centralizes configuration schema and validation rules
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
    /// Parameter key (e.g., `heart_rate.anaerobic_threshold`)
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
    #[must_use]
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
            version: "1.0.0".into(),
        }
    }

    /// Build physiological zones category
    fn build_physiological_zones_category() -> ConfigCategory {
        ConfigCategory {
            name: "physiological_zones".into(),
            description: "Heart rate zones, lactate thresholds, and VO2 max calculations"
                .to_string(),
            modules: vec![
                Self::build_heart_rate_module(),
                Self::build_lactate_module(),
                Self::build_fitness_levels_module(),
                Self::build_heart_rate_zones_module(),
                Self::build_vo2_calculations_module(),
                Self::build_pace_zones_module(),
                Self::build_ftp_calculation_module(),
            ],
        }
    }

    /// Build heart rate module configuration
    fn build_heart_rate_module() -> ConfigModule {
        ConfigModule {
            name: "heart_rate".into(),
            description: "Heart rate zone thresholds and calculations".into(),
            parameters: vec![
                ConfigParameter {
                    key: "heart_rate.anaerobic_threshold".into(),
                    description: "Anaerobic threshold as percentage of max HR".into(),
                    data_type: ParameterType::Float,
                    default_value: ConfigValue::Float(85.0),
                    valid_range: Some(ConfigValue::FloatRange {
                        min: 70.0,
                        max: 95.0,
                    }),
                    units: Some("percentage".into()),
                    scientific_basis: Some("Seiler 2010, Laursen 2002".into()),
                    requires_vo2_max: false,
                },
                ConfigParameter {
                    key: "heart_rate.vo2_max_zone".into(),
                    description: "VO2 max zone as percentage of max HR".into(),
                    data_type: ParameterType::Float,
                    default_value: ConfigValue::Float(95.0),
                    valid_range: Some(ConfigValue::FloatRange {
                        min: 85.0,
                        max: 100.0,
                    }),
                    units: Some("percentage".into()),
                    scientific_basis: Some("Buchheit & Laursen 2013".into()),
                    requires_vo2_max: false,
                },
                ConfigParameter {
                    key: "heart_rate.tempo_zone".into(),
                    description: "Tempo/threshold zone as percentage of max HR".into(),
                    data_type: ParameterType::Float,
                    default_value: ConfigValue::Float(80.0),
                    valid_range: Some(ConfigValue::FloatRange {
                        min: 70.0,
                        max: 90.0,
                    }),
                    units: Some("percentage".into()),
                    scientific_basis: Some("Coggan & Allen 2006".into()),
                    requires_vo2_max: false,
                },
                ConfigParameter {
                    key: "heart_rate.endurance_zone".into(),
                    description: "Aerobic endurance zone as percentage of max HR".to_string(),
                    data_type: ParameterType::Float,
                    default_value: ConfigValue::Float(70.0),
                    valid_range: Some(ConfigValue::FloatRange {
                        min: 60.0,
                        max: 80.0,
                    }),
                    units: Some("percentage".into()),
                    scientific_basis: Some("Maffetone Method".into()),
                    requires_vo2_max: false,
                },
                ConfigParameter {
                    key: "heart_rate.recovery_zone".into(),
                    description: "Active recovery zone as percentage of max HR".into(),
                    data_type: ParameterType::Float,
                    default_value: ConfigValue::Float(60.0),
                    valid_range: Some(ConfigValue::FloatRange {
                        min: 50.0,
                        max: 70.0,
                    }),
                    units: Some("percentage".into()),
                    scientific_basis: Some("Polarized Training Model".into()),
                    requires_vo2_max: false,
                },
            ],
        }
    }

    /// Build lactate module configuration
    fn build_lactate_module() -> ConfigModule {
        ConfigModule {
            name: "lactate".into(),
            description: "Lactate threshold and metabolic parameters".into(),
            parameters: vec![
                ConfigParameter {
                    key: "lactate.threshold_percentage".into(),
                    description: "Lactate threshold as percentage of VO2 max".into(),
                    data_type: ParameterType::Float,
                    default_value: ConfigValue::Float(85.0),
                    valid_range: Some(ConfigValue::FloatRange {
                        min: 65.0,
                        max: 95.0,
                    }),
                    units: Some("percentage".into()),
                    scientific_basis: Some("Faude et al. 2009".into()),
                    requires_vo2_max: true,
                },
                ConfigParameter {
                    key: "lactate.accumulation_rate".into(),
                    description: "Rate of lactate accumulation above threshold".into(),
                    data_type: ParameterType::Float,
                    default_value: ConfigValue::Float(4.0),
                    valid_range: Some(ConfigValue::FloatRange { min: 2.0, max: 8.0 }),
                    units: Some("mmol/L/min".into()),
                    scientific_basis: Some("Beneke 2003".into()),
                    requires_vo2_max: true,
                },
            ],
        }
    }

    /// Build fitness levels module configuration
    fn build_fitness_levels_module() -> ConfigModule {
        let mut parameters = Vec::new();
        parameters.extend(Self::build_male_fitness_parameters());
        parameters.extend(Self::build_female_fitness_parameters());

        ConfigModule {
            name: "fitness_levels".into(),
            description: "VO2 max thresholds for fitness level classification".into(),
            parameters,
        }
    }

    /// Build male fitness level parameters
    fn build_male_fitness_parameters() -> Vec<ConfigParameter> {
        vec![
            ConfigParameter {
                key: "fitness.vo2_max_threshold_male_beginner".into(),
                description: "VO2 max threshold for beginner level (males)".into(),
                data_type: ParameterType::Float,
                default_value: ConfigValue::Float(35.0),
                valid_range: Some(ConfigValue::FloatRange {
                    min: 25.0,
                    max: 45.0,
                }),
                units: Some("ml/kg/min".into()),
                scientific_basis: Some("ACSM Guidelines 2018".into()),
                requires_vo2_max: false,
            },
            ConfigParameter {
                key: "fitness.vo2_max_threshold_male_recreational".into(),
                description: "VO2 max threshold for recreational level (males)".to_string(),
                data_type: ParameterType::Float,
                default_value: ConfigValue::Float(42.0),
                valid_range: Some(ConfigValue::FloatRange {
                    min: 35.0,
                    max: 50.0,
                }),
                units: Some("ml/kg/min".into()),
                scientific_basis: Some("ACSM Guidelines 2018".into()),
                requires_vo2_max: false,
            },
            ConfigParameter {
                key: "fitness.vo2_max_threshold_male_intermediate".into(),
                description: "VO2 max threshold for intermediate level (males)".to_string(),
                data_type: ParameterType::Float,
                default_value: ConfigValue::Float(50.0),
                valid_range: Some(ConfigValue::FloatRange {
                    min: 42.0,
                    max: 58.0,
                }),
                units: Some("ml/kg/min".into()),
                scientific_basis: Some("ACSM Guidelines 2018".into()),
                requires_vo2_max: false,
            },
            ConfigParameter {
                key: "fitness.vo2_max_threshold_male_advanced".into(),
                description: "VO2 max threshold for advanced level (males)".into(),
                data_type: ParameterType::Float,
                default_value: ConfigValue::Float(55.0),
                valid_range: Some(ConfigValue::FloatRange {
                    min: 50.0,
                    max: 65.0,
                }),
                units: Some("ml/kg/min".into()),
                scientific_basis: Some("ACSM Guidelines 2018".into()),
                requires_vo2_max: false,
            },
            ConfigParameter {
                key: "fitness.vo2_max_threshold_male_elite".into(),
                description: "VO2 max threshold for elite level (males)".into(),
                data_type: ParameterType::Float,
                default_value: ConfigValue::Float(60.0),
                valid_range: Some(ConfigValue::FloatRange {
                    min: 55.0,
                    max: 70.0,
                }),
                units: Some("ml/kg/min".into()),
                scientific_basis: Some("ACSM Guidelines 2018".into()),
                requires_vo2_max: false,
            },
        ]
    }

    /// Build female fitness level parameters
    fn build_female_fitness_parameters() -> Vec<ConfigParameter> {
        vec![
            ConfigParameter {
                key: "fitness.vo2_max_threshold_female_beginner".into(),
                description: "VO2 max threshold for beginner level (females)".to_string(),
                data_type: ParameterType::Float,
                default_value: ConfigValue::Float(30.0),
                valid_range: Some(ConfigValue::FloatRange {
                    min: 20.0,
                    max: 40.0,
                }),
                units: Some("ml/kg/min".into()),
                scientific_basis: Some("ACSM Guidelines 2018".into()),
                requires_vo2_max: false,
            },
            ConfigParameter {
                key: "fitness.vo2_max_threshold_female_recreational".into(),
                description: "VO2 max threshold for recreational level (females)".to_string(),
                data_type: ParameterType::Float,
                default_value: ConfigValue::Float(35.0),
                valid_range: Some(ConfigValue::FloatRange {
                    min: 30.0,
                    max: 45.0,
                }),
                units: Some("ml/kg/min".into()),
                scientific_basis: Some("ACSM Guidelines 2018".into()),
                requires_vo2_max: false,
            },
            ConfigParameter {
                key: "fitness.vo2_max_threshold_female_intermediate".into(),
                description: "VO2 max threshold for intermediate level (females)".to_string(),
                data_type: ParameterType::Float,
                default_value: ConfigValue::Float(42.0),
                valid_range: Some(ConfigValue::FloatRange {
                    min: 35.0,
                    max: 50.0,
                }),
                units: Some("ml/kg/min".into()),
                scientific_basis: Some("ACSM Guidelines 2018".into()),
                requires_vo2_max: false,
            },
            ConfigParameter {
                key: "fitness.vo2_max_threshold_female_advanced".into(),
                description: "VO2 max threshold for advanced level (females)".to_string(),
                data_type: ParameterType::Float,
                default_value: ConfigValue::Float(50.0),
                valid_range: Some(ConfigValue::FloatRange {
                    min: 42.0,
                    max: 58.0,
                }),
                units: Some("ml/kg/min".into()),
                scientific_basis: Some("ACSM Guidelines 2018".into()),
                requires_vo2_max: false,
            },
            ConfigParameter {
                key: "fitness.vo2_max_threshold_female_elite".into(),
                description: "VO2 max threshold for elite level (females)".into(),
                data_type: ParameterType::Float,
                default_value: ConfigValue::Float(55.0),
                valid_range: Some(ConfigValue::FloatRange {
                    min: 50.0,
                    max: 65.0,
                }),
                units: Some("ml/kg/min".into()),
                scientific_basis: Some("ACSM Guidelines 2018".into()),
                requires_vo2_max: false,
            },
        ]
    }

    /// Build heart rate zones module configuration
    fn build_heart_rate_zones_module() -> ConfigModule {
        ConfigModule {
            name: "heart_rate_zones".into(),
            description: "Heart rate zone percentages for different fitness levels".to_string(),
            parameters: vec![
                ConfigParameter {
                    key: "hr_zones.elite_zone6_threshold".into(),
                    description: "VO2 max threshold for zone 6 availability (elite athletes)"
                        .to_string(),
                    data_type: ParameterType::Float,
                    default_value: ConfigValue::Float(50.0),
                    valid_range: Some(ConfigValue::FloatRange {
                        min: 45.0,
                        max: 60.0,
                    }),
                    units: Some("ml/kg/min".into()),
                    scientific_basis: Some("Elite training zone research".into()),
                    requires_vo2_max: false,
                },
                ConfigParameter {
                    key: "hr_zones.lactate_threshold_min".into(),
                    description: "Minimum lactate threshold percentage".into(),
                    data_type: ParameterType::Float,
                    default_value: ConfigValue::Float(0.65),
                    valid_range: Some(ConfigValue::FloatRange {
                        min: 0.60,
                        max: 0.70,
                    }),
                    units: Some("percentage".into()),
                    scientific_basis: Some("Lactate threshold research".into()),
                    requires_vo2_max: false,
                },
                ConfigParameter {
                    key: "hr_zones.lactate_threshold_max".into(),
                    description: "Maximum lactate threshold percentage".into(),
                    data_type: ParameterType::Float,
                    default_value: ConfigValue::Float(0.95),
                    valid_range: Some(ConfigValue::FloatRange {
                        min: 0.90,
                        max: 1.00,
                    }),
                    units: Some("percentage".into()),
                    scientific_basis: Some("Lactate threshold research".into()),
                    requires_vo2_max: false,
                },
                ConfigParameter {
                    key: "hr_zones.sport_efficiency_min".into(),
                    description: "Minimum sport efficiency factor".into(),
                    data_type: ParameterType::Float,
                    default_value: ConfigValue::Float(0.5),
                    valid_range: Some(ConfigValue::FloatRange { min: 0.3, max: 0.7 }),
                    units: None,
                    scientific_basis: Some("Sport efficiency research".into()),
                    requires_vo2_max: false,
                },
                ConfigParameter {
                    key: "hr_zones.sport_efficiency_max".into(),
                    description: "Maximum sport efficiency factor".into(),
                    data_type: ParameterType::Float,
                    default_value: ConfigValue::Float(1.5),
                    valid_range: Some(ConfigValue::FloatRange { min: 1.0, max: 2.0 }),
                    units: None,
                    scientific_basis: Some("Sport efficiency research".into()),
                    requires_vo2_max: false,
                },
            ],
        }
    }

    /// Build VO2 calculations module configuration
    fn build_vo2_calculations_module() -> ConfigModule {
        ConfigModule {
            name: "vo2_calculations".into(),
            description: "VO2 max calculation constants and formulas".into(),
            parameters: vec![
                ConfigParameter {
                    key: "vo2.vdot_coefficient_a".into(),
                    description: "VDOT formula coefficient A (velocity calculation)".to_string(),
                    data_type: ParameterType::Float,
                    default_value: ConfigValue::Float(29.54),
                    valid_range: Some(ConfigValue::FloatRange {
                        min: 25.0,
                        max: 35.0,
                    }),
                    units: Some("m/min".into()),
                    scientific_basis: Some("Jack Daniels Running Formula".into()),
                    requires_vo2_max: false,
                },
                ConfigParameter {
                    key: "vo2.vdot_coefficient_b".into(),
                    description: "VDOT formula coefficient B (linear term)".into(),
                    data_type: ParameterType::Float,
                    default_value: ConfigValue::Float(5.000_663),
                    valid_range: Some(ConfigValue::FloatRange { min: 4.0, max: 6.0 }),
                    units: Some("(m/min)/(ml/kg/min)".into()),
                    scientific_basis: Some("Jack Daniels Running Formula".into()),
                    requires_vo2_max: false,
                },
                ConfigParameter {
                    key: "vo2.vdot_coefficient_c".into(),
                    description: "VDOT formula coefficient C (quadratic term)".into(),
                    data_type: ParameterType::Float,
                    default_value: ConfigValue::Float(0.007_546),
                    valid_range: Some(ConfigValue::FloatRange {
                        min: 0.005,
                        max: 0.01,
                    }),
                    units: Some("(m/min)/(ml/kg/min)²".into()),
                    scientific_basis: Some("Jack Daniels Running Formula".into()),
                    requires_vo2_max: false,
                },
                ConfigParameter {
                    key: "vo2.threshold_velocity_base".into(),
                    description: "Base threshold velocity as percentage of vVO2max".to_string(),
                    data_type: ParameterType::Float,
                    default_value: ConfigValue::Float(0.86),
                    valid_range: Some(ConfigValue::FloatRange {
                        min: 0.80,
                        max: 0.95,
                    }),
                    units: Some("percentage".into()),
                    scientific_basis: Some("Lactate threshold studies".into()),
                    requires_vo2_max: false,
                },
                ConfigParameter {
                    key: "vo2.threshold_adjustment_factor".into(),
                    description: "Threshold adjustment factor for lactate threshold".to_string(),
                    data_type: ParameterType::Float,
                    default_value: ConfigValue::Float(0.4),
                    valid_range: Some(ConfigValue::FloatRange { min: 0.2, max: 0.6 }),
                    units: None,
                    scientific_basis: Some("Lactate threshold variability".into()),
                    requires_vo2_max: false,
                },
                ConfigParameter {
                    key: "vo2.power_coefficient".into(),
                    description: "Power at VO2 max coefficient (W per ml/kg/min)".to_string(),
                    data_type: ParameterType::Float,
                    default_value: ConfigValue::Float(13.5),
                    valid_range: Some(ConfigValue::FloatRange {
                        min: 12.0,
                        max: 15.0,
                    }),
                    units: Some("W/(ml/kg/min)".into()),
                    scientific_basis: Some("Power-VO2 relationship studies".into()),
                    requires_vo2_max: false,
                },
            ],
        }
    }

    /// Build pace zones module configuration
    fn build_pace_zones_module() -> ConfigModule {
        let mut parameters = Vec::new();
        parameters.extend(Self::build_easy_pace_parameters());
        parameters.extend(Self::build_pace_adjustment_parameters());
        parameters.extend(Self::build_intensity_pace_parameters());

        ConfigModule {
            name: "pace_zones".into(),
            description: "Running pace zone percentages and calculations".into(),
            parameters,
        }
    }

    /// Build easy pace zone parameters
    fn build_easy_pace_parameters() -> Vec<ConfigParameter> {
        vec![
            ConfigParameter {
                key: "pace.easy_zone_low".into(),
                description: "Easy pace zone lower bound (% of vVO2max)".into(),
                data_type: ParameterType::Float,
                default_value: ConfigValue::Float(0.59),
                valid_range: Some(ConfigValue::FloatRange {
                    min: 0.50,
                    max: 0.65,
                }),
                units: Some("percentage".into()),
                scientific_basis: Some("Training zone research".into()),
                requires_vo2_max: false,
            },
            ConfigParameter {
                key: "pace.easy_zone_high".into(),
                description: "Easy pace zone upper bound (% of vVO2max)".into(),
                data_type: ParameterType::Float,
                default_value: ConfigValue::Float(0.74),
                valid_range: Some(ConfigValue::FloatRange {
                    min: 0.70,
                    max: 0.80,
                }),
                units: Some("percentage".into()),
                scientific_basis: Some("Training zone research".into()),
                requires_vo2_max: false,
            },
        ]
    }

    /// Build pace adjustment parameters
    fn build_pace_adjustment_parameters() -> Vec<ConfigParameter> {
        vec![
            ConfigParameter {
                key: "pace.marathon_adjustment_low".into(),
                description: "Marathon pace adjustment factor (slower)".into(),
                data_type: ParameterType::Float,
                default_value: ConfigValue::Float(1.06),
                valid_range: Some(ConfigValue::FloatRange {
                    min: 1.02,
                    max: 1.10,
                }),
                units: Some("multiplier".into()),
                scientific_basis: Some("Marathon pace studies".into()),
                requires_vo2_max: false,
            },
            ConfigParameter {
                key: "pace.marathon_adjustment_high".into(),
                description: "Marathon pace adjustment factor (faster)".into(),
                data_type: ParameterType::Float,
                default_value: ConfigValue::Float(1.02),
                valid_range: Some(ConfigValue::FloatRange {
                    min: 1.00,
                    max: 1.05,
                }),
                units: Some("multiplier".into()),
                scientific_basis: Some("Marathon pace studies".into()),
                requires_vo2_max: false,
            },
            ConfigParameter {
                key: "pace.threshold_adjustment_low".into(),
                description: "Threshold pace adjustment factor (slower)".into(),
                data_type: ParameterType::Float,
                default_value: ConfigValue::Float(1.02),
                valid_range: Some(ConfigValue::FloatRange {
                    min: 1.00,
                    max: 1.05,
                }),
                units: Some("multiplier".into()),
                scientific_basis: Some("Threshold pace studies".into()),
                requires_vo2_max: false,
            },
            ConfigParameter {
                key: "pace.threshold_adjustment_high".into(),
                description: "Threshold pace adjustment factor (faster)".into(),
                data_type: ParameterType::Float,
                default_value: ConfigValue::Float(0.98),
                valid_range: Some(ConfigValue::FloatRange {
                    min: 0.95,
                    max: 1.00,
                }),
                units: Some("multiplier".into()),
                scientific_basis: Some("Threshold pace studies".into()),
                requires_vo2_max: false,
            },
        ]
    }

    /// Build high intensity pace parameters
    fn build_intensity_pace_parameters() -> Vec<ConfigParameter> {
        vec![
            ConfigParameter {
                key: "pace.vo2max_zone_percentage".into(),
                description: "VO2 max pace zone percentage of vVO2max".into(),
                data_type: ParameterType::Float,
                default_value: ConfigValue::Float(0.95),
                valid_range: Some(ConfigValue::FloatRange {
                    min: 0.90,
                    max: 1.00,
                }),
                units: Some("percentage".into()),
                scientific_basis: Some("VO2 max training studies".into()),
                requires_vo2_max: false,
            },
            ConfigParameter {
                key: "pace.neuromuscular_zone_percentage".into(),
                description: "Neuromuscular pace zone percentage of vVO2max".to_string(),
                data_type: ParameterType::Float,
                default_value: ConfigValue::Float(1.05),
                valid_range: Some(ConfigValue::FloatRange {
                    min: 1.00,
                    max: 1.15,
                }),
                units: Some("percentage".into()),
                scientific_basis: Some("Neuromuscular training studies".into()),
                requires_vo2_max: false,
            },
        ]
    }

    /// Build FTP calculation module configuration
    fn build_ftp_calculation_module() -> ConfigModule {
        ConfigModule {
            name: "ftp_calculation".into(),
            description: "Functional Threshold Power calculation parameters".into(),
            parameters: vec![
                ConfigParameter {
                    key: "ftp.elite_percentage".into(),
                    description: "FTP percentage for elite athletes (VO2 max >= 60)".to_string(),
                    data_type: ParameterType::Float,
                    default_value: ConfigValue::Float(0.85),
                    valid_range: Some(ConfigValue::FloatRange {
                        min: 0.80,
                        max: 0.90,
                    }),
                    units: Some("percentage".into()),
                    scientific_basis: Some("Elite athlete FTP studies".into()),
                    requires_vo2_max: false,
                },
                ConfigParameter {
                    key: "ftp.advanced_percentage".into(),
                    description: "FTP percentage for advanced athletes (VO2 max >= 50)".to_string(),
                    data_type: ParameterType::Float,
                    default_value: ConfigValue::Float(0.82),
                    valid_range: Some(ConfigValue::FloatRange {
                        min: 0.75,
                        max: 0.85,
                    }),
                    units: Some("percentage".into()),
                    scientific_basis: Some("Advanced athlete FTP studies".into()),
                    requires_vo2_max: false,
                },
                ConfigParameter {
                    key: "ftp.intermediate_percentage".into(),
                    description: "FTP percentage for intermediate athletes (VO2 max >= 40)"
                        .to_string(),
                    data_type: ParameterType::Float,
                    default_value: ConfigValue::Float(0.78),
                    valid_range: Some(ConfigValue::FloatRange {
                        min: 0.70,
                        max: 0.82,
                    }),
                    units: Some("percentage".into()),
                    scientific_basis: Some("Intermediate athlete FTP studies".into()),
                    requires_vo2_max: false,
                },
                ConfigParameter {
                    key: "ftp.beginner_percentage".into(),
                    description: "FTP percentage for beginner athletes".into(),
                    data_type: ParameterType::Float,
                    default_value: ConfigValue::Float(0.75),
                    valid_range: Some(ConfigValue::FloatRange {
                        min: 0.65,
                        max: 0.80,
                    }),
                    units: Some("percentage".into()),
                    scientific_basis: Some("Beginner athlete FTP studies".into()),
                    requires_vo2_max: false,
                },
            ],
        }
    }

    /// Build performance calculation category
    #[allow(clippy::too_many_lines)]
    fn build_performance_calculation_category() -> ConfigCategory {
        ConfigCategory {
            name: "performance_calculation".into(),
            description: "Effort scoring, efficiency calculations, and performance metrics"
                .to_string(),
            modules: vec![
                ConfigModule {
                    name: "effort_scoring".into(),
                    description: "Parameters for calculating relative effort scores".into(),
                    parameters: vec![
                        ConfigParameter {
                            key: "performance.run_distance_divisor".into(),
                            description:
                                "Divisor for normalizing running distance in effort calculation"
                                    .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(10.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 5.0,
                                max: 20.0,
                            }),
                            units: Some("km".into()),
                            scientific_basis: Some("ACSM Guidelines 2018".into()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "performance.bike_distance_divisor".into(),
                            description:
                                "Divisor for normalizing cycling distance in effort calculation"
                                    .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(40.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 20.0,
                                max: 60.0,
                            }),
                            units: Some("km".into()),
                            scientific_basis: Some("Coggan Power Training".into()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "performance.swim_distance_divisor".into(),
                            description:
                                "Divisor for normalizing swimming distance in effort calculation"
                                    .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(2.0),
                            valid_range: Some(ConfigValue::FloatRange { min: 1.0, max: 5.0 }),
                            units: Some("km".into()),
                            scientific_basis: Some("Costill et al. 1985".into()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "performance.elevation_divisor".into(),
                            description: "Divisor for elevation gain in effort calculation"
                                .to_string(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(100.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 50.0,
                                max: 200.0,
                            }),
                            units: Some("meters".into()),
                            scientific_basis: Some("Minetti et al. 2002".into()),
                            requires_vo2_max: false,
                        },
                    ],
                },
                ConfigModule {
                    name: "efficiency".into(),
                    description: "Efficiency scoring and economy calculations".into(),
                    parameters: vec![
                        ConfigParameter {
                            key: "efficiency.base_score".into(),
                            description: "Base efficiency score for all activities".into(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(50.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 0.0,
                                max: 100.0,
                            }),
                            units: Some("points".into()),
                            scientific_basis: None,
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "efficiency.hr_factor".into(),
                            description: "Heart rate efficiency calculation factor".into(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(1000.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 500.0,
                                max: 2000.0,
                            }),
                            units: None,
                            scientific_basis: Some("HR:Pace ratio studies".into()),
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
            name: "sport_specific".into(),
            description: "Sport-specific performance calculations and thresholds".into(),
            modules: vec![
                ConfigModule {
                    name: "cycling".into(),
                    description: "Cycling-specific parameters".into(),
                    parameters: vec![
                        ConfigParameter {
                            key: "cycling.ftp_percentage".into(),
                            description: "FTP as percentage of 20-minute power".into(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(95.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 90.0,
                                max: 98.0,
                            }),
                            units: Some("percentage".into()),
                            scientific_basis: Some("Allen & Coggan 2010".into()),
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "cycling.power_weight_importance".into(),
                            description: "Importance of power-to-weight ratio".into(),
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
                    name: "running".into(),
                    description: "Running-specific parameters".into(),
                    parameters: vec![
                        ConfigParameter {
                            key: "running.economy_factor".into(),
                            description: "Running economy adjustment factor".into(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(1.0),
                            valid_range: Some(ConfigValue::FloatRange { min: 0.7, max: 1.3 }),
                            units: None,
                            scientific_basis: Some("Daniels Running Formula".into()),
                            requires_vo2_max: true,
                        },
                        ConfigParameter {
                            key: "running.cadence_target".into(),
                            description: "Target running cadence".into(),
                            data_type: ParameterType::Integer,
                            default_value: ConfigValue::Integer(180),
                            valid_range: Some(ConfigValue::IntegerRange { min: 160, max: 200 }),
                            units: Some("steps/min".into()),
                            scientific_basis: Some("Heiderscheit et al. 2011".into()),
                            requires_vo2_max: false,
                        },
                    ],
                },
                ConfigModule {
                    name: "swimming".into(),
                    description: "Swimming-specific parameters".into(),
                    parameters: vec![ConfigParameter {
                        key: "swimming.stroke_efficiency".into(),
                        description: "Stroke efficiency factor".into(),
                        data_type: ParameterType::Float,
                        default_value: ConfigValue::Float(0.75),
                        valid_range: Some(ConfigValue::FloatRange { min: 0.5, max: 1.0 }),
                        units: None,
                        scientific_basis: Some("Toussaint & Beek 1992".into()),
                        requires_vo2_max: false,
                    }],
                },
            ],
        }
    }

    /// Build analysis settings category
    fn build_analysis_settings_category() -> ConfigCategory {
        ConfigCategory {
            name: "analysis_settings".into(),
            description: "Settings for activity analysis and insights".into(),
            modules: vec![
                ConfigModule {
                    name: "insights".into(),
                    description: "Insight generation parameters".into(),
                    parameters: vec![
                        ConfigParameter {
                            key: "insights.min_confidence".into(),
                            description: "Minimum confidence threshold for insights".into(),
                            data_type: ParameterType::Float,
                            default_value: ConfigValue::Float(70.0),
                            valid_range: Some(ConfigValue::FloatRange {
                                min: 50.0,
                                max: 95.0,
                            }),
                            units: Some("percentage".into()),
                            scientific_basis: None,
                            requires_vo2_max: false,
                        },
                        ConfigParameter {
                            key: "insights.max_per_activity".into(),
                            description: "Maximum insights per activity".into(),
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
                    name: "anomaly_detection".into(),
                    description: "Anomaly detection thresholds".into(),
                    parameters: vec![ConfigParameter {
                        key: "anomaly.hr_spike_threshold".into(),
                        description: "Heart rate spike detection threshold".into(),
                        data_type: ParameterType::Float,
                        default_value: ConfigValue::Float(20.0),
                        valid_range: Some(ConfigValue::FloatRange {
                            min: 10.0,
                            max: 40.0,
                        }),
                        units: Some("bpm/min".into()),
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
            name: "safety_constraints".into(),
            description: "Safety limits and medical constraints".into(),
            modules: vec![ConfigModule {
                name: "intensity_limits".into(),
                description: "Maximum intensity constraints".into(),
                parameters: vec![
                    ConfigParameter {
                        key: "safety.max_hr_percentage".into(),
                        description: "Maximum allowed heart rate percentage".into(),
                        data_type: ParameterType::Float,
                        default_value: ConfigValue::Float(100.0),
                        valid_range: Some(ConfigValue::FloatRange {
                            min: 60.0,
                            max: 100.0,
                        }),
                        units: Some("percentage".into()),
                        scientific_basis: Some("ACSM Exercise Guidelines".into()),
                        requires_vo2_max: false,
                    },
                    ConfigParameter {
                        key: "safety.recovery_multiplier".into(),
                        description: "Recovery time multiplier for safety".into(),
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
    #[must_use]
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
    #[must_use]
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
