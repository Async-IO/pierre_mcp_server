// ABOUTME: VO2 max calculation configuration and physiological constants
// ABOUTME: Provides configurable parameters for aerobic capacity calculations
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! VO2 max-based physiological calculations for personalized thresholds

use serde::{Deserialize, Serialize};

/// Helper function to get configuration values with fallback
fn get_config_value(key: &str, fallback: f64) -> f64 {
    use super::catalog::CatalogBuilder;
    use super::runtime::ConfigValue;

    CatalogBuilder::get_parameter(key)
        .and_then(|param| match param.default_value {
            ConfigValue::Float(v) => Some(v),
            _ => None,
        })
        .unwrap_or(fallback)
}

/// VO2 max-based physiological calculator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VO2MaxCalculator {
    /// VO2 max in ml/kg/min
    pub vo2_max: f64,
    /// Resting heart rate in bpm
    pub resting_hr: u16,
    /// Maximum heart rate in bpm
    pub max_hr: u16,
    /// Lactate threshold as percentage of VO2 max (typically 0.65-0.85)
    pub lactate_threshold: f64,
    /// Sport-specific efficiency factor
    pub sport_efficiency: f64,
}

/// Personalized heart rate zones based on VO2 max
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalizedHRZones {
    /// Zone 1: Active Recovery
    pub zone1_lower: u16,
    pub zone1_upper: u16,

    /// Zone 2: Aerobic Base
    pub zone2_lower: u16,
    pub zone2_upper: u16,

    /// Zone 3: Tempo
    pub zone3_lower: u16,
    pub zone3_upper: u16,

    /// Zone 4: Lactate Threshold
    pub zone4_lower: u16,
    pub zone4_upper: u16,

    /// Zone 5: VO2 Max
    pub zone5_lower: u16,
    pub zone5_upper: u16,

    /// Zone 6: Neuromuscular Power (optional)
    pub zone6_lower: Option<u16>,
    pub zone6_upper: Option<u16>,
}

/// Personalized pace zones for running
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalizedPaceZones {
    /// Easy pace range (seconds per km)
    pub easy_pace_range: (f64, f64),

    /// Marathon pace range
    pub marathon_pace_range: (f64, f64),

    /// Threshold pace range
    pub threshold_pace_range: (f64, f64),

    /// VO2 max pace range
    pub vo2max_pace_range: (f64, f64),

    /// Neuromuscular/sprint pace maximum
    pub neuromuscular_pace_max: f64,
}

/// Power zones for cycling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalizedPowerZones {
    /// Zone 1: Active Recovery (% of FTP)
    pub zone1_range: (f64, f64),

    /// Zone 2: Endurance
    pub zone2_range: (f64, f64),

    /// Zone 3: Tempo
    pub zone3_range: (f64, f64),

    /// Zone 4: Threshold
    pub zone4_range: (f64, f64),

    /// Zone 5: VO2 Max
    pub zone5_range: (f64, f64),

    /// Zone 6: Anaerobic
    pub zone6_range: (f64, f64),

    /// Zone 7: Neuromuscular
    pub zone7_range: (f64, f64),
}

impl VO2MaxCalculator {
    /// Helper function to safely convert HR calculations to u16
    fn hr_calc_to_u16(base_hr: u16, reserve: f64, percentage: f64) -> u16 {
        let addition = (reserve * percentage).round();
        // Safe: addition represents small heart rate increment (0-100 bpm)
        #[allow(clippy::cast_possible_truncation)]
        u16::try_from(addition as i32)
            .map(|add| base_hr.saturating_add(add))
            .unwrap_or(base_hr)
    }
    /// Create a new VO2 max calculator
    #[must_use]
    pub fn new(
        vo2_max: f64,
        resting_hr: u16,
        max_hr: u16,
        lactate_threshold: f64,
        sport_efficiency: f64,
    ) -> Self {
        // Get clamping values from configuration
        let lactate_min = get_config_value("hr_zones.lactate_threshold_min", 0.65);
        let lactate_max = get_config_value("hr_zones.lactate_threshold_max", 0.95);
        let efficiency_min = get_config_value("hr_zones.sport_efficiency_min", 0.5);
        let efficiency_max = get_config_value("hr_zones.sport_efficiency_max", 1.5);

        Self {
            vo2_max,
            resting_hr,
            max_hr,
            lactate_threshold: lactate_threshold.clamp(lactate_min, lactate_max),
            sport_efficiency: sport_efficiency.clamp(efficiency_min, efficiency_max),
        }
    }

    /// Calculate personalized heart rate zones using Karvonen method
    #[must_use]
    pub fn calculate_hr_zones(&self) -> PersonalizedHRZones {
        let hr_reserve = f64::from(self.max_hr - self.resting_hr);

        // Zone percentages based on VO2 max level
        let (z1_low, z1_high, z2_low, z2_high, z3_low, z3_high, z4_low, z4_high, z5_low, z5_high) =
            if self.vo2_max >= 60.0 {
                // Elite athlete zones (tighter ranges)
                (0.50, 0.58, 0.58, 0.68, 0.68, 0.78, 0.78, 0.88, 0.88, 0.95)
            } else if self.vo2_max >= 50.0 {
                // Advanced athlete zones
                (0.50, 0.60, 0.60, 0.70, 0.70, 0.80, 0.80, 0.90, 0.90, 0.98)
            } else if self.vo2_max >= 40.0 {
                // Intermediate athlete zones
                (0.45, 0.60, 0.60, 0.72, 0.72, 0.82, 0.82, 0.92, 0.92, 1.00)
            } else {
                // Beginner athlete zones (wider ranges)
                (0.40, 0.60, 0.60, 0.75, 0.75, 0.85, 0.85, 0.95, 0.95, 1.00)
            };

        PersonalizedHRZones {
            zone1_lower: Self::hr_calc_to_u16(self.resting_hr, hr_reserve, z1_low),
            zone1_upper: Self::hr_calc_to_u16(self.resting_hr, hr_reserve, z1_high),

            zone2_lower: Self::hr_calc_to_u16(self.resting_hr, hr_reserve, z2_low),
            zone2_upper: Self::hr_calc_to_u16(self.resting_hr, hr_reserve, z2_high),

            zone3_lower: Self::hr_calc_to_u16(self.resting_hr, hr_reserve, z3_low),
            zone3_upper: Self::hr_calc_to_u16(self.resting_hr, hr_reserve, z3_high),

            zone4_lower: Self::hr_calc_to_u16(self.resting_hr, hr_reserve, z4_low),
            zone4_upper: Self::hr_calc_to_u16(self.resting_hr, hr_reserve, z4_high),

            zone5_lower: Self::hr_calc_to_u16(self.resting_hr, hr_reserve, z5_low),
            zone5_upper: Self::hr_calc_to_u16(self.resting_hr, hr_reserve, z5_high)
                .min(self.max_hr),

            // Zone 6 for advanced athletes only (configurable threshold)
            zone6_lower: if self.vo2_max >= get_config_value("hr_zones.elite_zone6_threshold", 50.0)
            {
                Some(Self::hr_calc_to_u16(self.resting_hr, hr_reserve, 0.95))
            } else {
                None
            },
            zone6_upper: if self.vo2_max >= get_config_value("hr_zones.elite_zone6_threshold", 50.0)
            {
                Some(self.max_hr)
            } else {
                None
            },
        }
    }

    /// Calculate personalized running pace zones
    #[must_use]
    pub fn calculate_pace_zones(&self) -> PersonalizedPaceZones {
        // Calculate critical velocity at lactate threshold
        // Using simplified Jack Daniels' VDOT formulas
        let vdot = self.vo2_max;

        // Get coefficients from configuration
        let coeff_a = get_config_value("vo2.vdot_coefficient_a", 29.54);
        let coeff_b = get_config_value("vo2.vdot_coefficient_b", 5.000_663);
        let coeff_c = get_config_value("vo2.vdot_coefficient_c", 0.007_546);

        // Convert VDOT to velocity at VO2max (vVO2max) in m/min
        let v_vo2max = (coeff_c * vdot).mul_add(-vdot, vdot.mul_add(coeff_b, coeff_a));

        // Calculate threshold velocity using configurable parameters
        let threshold_base = get_config_value("vo2.threshold_velocity_base", 0.86);
        let threshold_factor = get_config_value("vo2.threshold_adjustment_factor", 0.4);
        let threshold_velocity =
            v_vo2max * (self.lactate_threshold - 0.75).mul_add(threshold_factor, threshold_base);

        // Convert to pace (seconds per km)
        let threshold_pace = 1000.0 / threshold_velocity * 60.0;

        // Get pace zone parameters from configuration
        let easy_low = get_config_value("pace.easy_zone_low", 0.59);
        let easy_high = get_config_value("pace.easy_zone_high", 0.74);
        let marathon_adj_low = get_config_value("pace.marathon_adjustment_low", 1.06);
        let marathon_adj_high = get_config_value("pace.marathon_adjustment_high", 1.02);
        let threshold_adj_low = get_config_value("pace.threshold_adjustment_low", 1.02);
        let threshold_adj_high = get_config_value("pace.threshold_adjustment_high", 0.98);
        let vo2max_zone_pct = get_config_value("pace.vo2max_zone_percentage", 0.95);
        let neuromuscular_pct = get_config_value("pace.neuromuscular_zone_percentage", 1.05);

        PersonalizedPaceZones {
            // Easy pace: configurable % of vVO2max (slower = higher seconds/km)
            easy_pace_range: (
                1000.0 / (v_vo2max * easy_low) * 60.0, // Slower end (higher seconds/km)
                1000.0 / (v_vo2max * easy_high) * 60.0, // Faster end (lower seconds/km)
            ),

            // Marathon pace: based on threshold pace with configurable adjustments
            marathon_pace_range: (
                threshold_pace * marathon_adj_low,
                threshold_pace * marathon_adj_high,
            ),

            // Threshold pace: configurable adjustments around threshold
            threshold_pace_range: (
                threshold_pace * threshold_adj_low,
                threshold_pace * threshold_adj_high,
            ),

            // VO2 max pace: configurable % of vVO2max
            vo2max_pace_range: (
                1000.0 / v_vo2max * 60.0,
                1000.0 / (v_vo2max * vo2max_zone_pct) * 60.0,
            ),

            // Neuromuscular pace: configurable % of vVO2max
            neuromuscular_pace_max: 1000.0 / (v_vo2max * neuromuscular_pct) * 60.0,
        }
    }

    /// Calculate functional threshold power (FTP) from VO2 max
    #[must_use]
    pub fn estimate_ftp(&self) -> f64 {
        // Get power coefficient from configuration
        let power_coefficient = get_config_value("vo2.power_coefficient", 13.5);
        let power_at_vo2max = self.vo2_max * power_coefficient;

        // FTP percentage based on fitness level using configurable values
        let ftp_percentage = match self.vo2_max {
            v if v >= get_config_value("fitness.vo2_max_threshold_male_elite", 60.0) => {
                get_config_value("ftp.elite_percentage", 0.85)
            }
            v if v >= get_config_value("fitness.vo2_max_threshold_male_advanced", 50.0) => {
                get_config_value("ftp.advanced_percentage", 0.82)
            }
            v if v >= get_config_value("fitness.vo2_max_threshold_male_intermediate", 40.0) => {
                get_config_value("ftp.intermediate_percentage", 0.78)
            }
            _ => get_config_value("ftp.beginner_percentage", 0.75),
        };

        power_at_vo2max * ftp_percentage
    }

    /// Calculate personalized power zones for cycling
    #[must_use]
    pub fn calculate_power_zones(&self, ftp: Option<f64>) -> PersonalizedPowerZones {
        let ftp_value = ftp.unwrap_or_else(|| self.estimate_ftp());

        PersonalizedPowerZones {
            zone1_range: (0.0 * ftp_value, 0.55 * ftp_value), // Active Recovery
            zone2_range: (0.56 * ftp_value, 0.75 * ftp_value), // Endurance
            zone3_range: (0.76 * ftp_value, 0.90 * ftp_value), // Tempo
            zone4_range: (0.91 * ftp_value, 1.05 * ftp_value), // Threshold
            zone5_range: (1.06 * ftp_value, 1.20 * ftp_value), // VO2 Max
            zone6_range: (1.21 * ftp_value, 1.50 * ftp_value), // Anaerobic
            zone7_range: (1.51 * ftp_value, f64::MAX),        // Neuromuscular
        }
    }

    /// Get zone name for a given heart rate
    #[must_use]
    pub fn get_hr_zone_name(&self, heart_rate: u16) -> &'static str {
        let zones = self.calculate_hr_zones();

        match heart_rate {
            hr if hr < zones.zone1_upper => "Recovery",
            hr if hr < zones.zone2_upper => "Aerobic Base",
            hr if hr < zones.zone3_upper => "Tempo",
            hr if hr < zones.zone4_upper => "Threshold",
            hr if hr < zones.zone5_upper => "VO2 Max",
            _ => "Neuromuscular",
        }
    }

    /// Calculate training impulse (TRIMP) for an activity
    #[must_use]
    pub fn calculate_trimp(&self, avg_hr: u16, duration_minutes: f64, gender: &str) -> f64 {
        let hr_reserve = f64::from(self.max_hr - self.resting_hr);
        let hr_ratio = f64::from(avg_hr - self.resting_hr) / hr_reserve;

        // Gender-specific weighting factor
        let gender_factor: f64 = match gender {
            "F" | "female" => 1.67,
            _ => 1.92, // Male or unspecified
        };

        duration_minutes * hr_ratio * 0.64 * gender_factor.powf(hr_ratio)
    }
}

/// Sport-specific efficiency factors
pub trait SportEfficiency {
    fn sport_efficiency_factor(&self) -> f64;
}

impl SportEfficiency for crate::models::SportType {
    fn sport_efficiency_factor(&self) -> f64 {
        match self {
            Self::Run => 1.0,
            Self::Swim => 0.7, // Swimming has lower mechanical efficiency
            Self::Walk => 0.8,
            Self::Hike => 0.85,
            _ => 0.9, // Default including cycling which is mechanically more efficient
        }
    }
}
