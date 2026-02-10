// ABOUTME: VO2 max calculation configuration and physiological constants
// ABOUTME: Provides configurable parameters for aerobic capacity calculations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! VO2 max-based physiological calculations for personalized thresholds

use crate::algorithms::{FtpAlgorithm, TrimpAlgorithm};
use crate::models::SportType;
use serde::{Deserialize, Serialize};

/// Heart rate zone configuration defaults
mod hr_zone_defaults {
    /// Minimum lactate threshold percentage
    pub const LACTATE_THRESHOLD_MIN: f64 = 0.65;
    /// Maximum lactate threshold percentage
    pub const LACTATE_THRESHOLD_MAX: f64 = 0.95;
    /// Minimum sport efficiency factor
    pub const SPORT_EFFICIENCY_MIN: f64 = 0.5;
    /// Maximum sport efficiency factor
    pub const SPORT_EFFICIENCY_MAX: f64 = 1.5;
    /// VO2 max threshold for Zone 6 eligibility
    pub const ELITE_ZONE6_THRESHOLD: f64 = 50.0;
}

/// VDOT pace calculation coefficients
mod vdot_coefficients {
    /// VDOT velocity constant A
    pub const VDOT_COEFFICIENT_A: f64 = 29.54;
    /// VDOT velocity coefficient B
    pub const VDOT_COEFFICIENT_B: f64 = 5.000_663;
    /// VDOT velocity coefficient C
    pub const VDOT_COEFFICIENT_C: f64 = 0.007_546;
    /// Base threshold velocity as fraction of vVO2max
    pub const THRESHOLD_VELOCITY_BASE: f64 = 0.86;
    /// Adjustment factor for lactate threshold
    pub const THRESHOLD_ADJUSTMENT_FACTOR: f64 = 0.4;
}

/// Pace zone percentages
mod pace_zone_defaults {
    /// Easy zone lower bound (fraction of vVO2max)
    pub const EASY_ZONE_LOW: f64 = 0.59;
    /// Easy zone upper bound (fraction of vVO2max)
    pub const EASY_ZONE_HIGH: f64 = 0.74;
    /// Marathon pace adjustment lower
    pub const MARATHON_ADJUSTMENT_LOW: f64 = 1.06;
    /// Marathon pace adjustment upper
    pub const MARATHON_ADJUSTMENT_HIGH: f64 = 1.02;
    /// Threshold pace adjustment lower
    pub const THRESHOLD_ADJUSTMENT_LOW: f64 = 1.02;
    /// Threshold pace adjustment upper
    pub const THRESHOLD_ADJUSTMENT_HIGH: f64 = 0.98;
    /// `VO2max` zone percentage
    pub const VO2MAX_ZONE_PERCENTAGE: f64 = 0.95;
    /// Neuromuscular zone percentage
    pub const NEUROMUSCULAR_ZONE_PERCENTAGE: f64 = 1.05;
}

/// Power calculation defaults
mod power_defaults {
    /// Power coefficient for FTP estimation from `VO2max`
    pub const POWER_COEFFICIENT: f64 = 13.5;
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
    /// Zone 1: Active Recovery - lower bound (bpm)
    pub zone1_lower: u16,
    /// Zone 1: Active Recovery - upper bound (bpm)
    pub zone1_upper: u16,

    /// Zone 2: Aerobic Base - lower bound (bpm)
    pub zone2_lower: u16,
    /// Zone 2: Aerobic Base - upper bound (bpm)
    pub zone2_upper: u16,

    /// Zone 3: Tempo - lower bound (bpm)
    pub zone3_lower: u16,
    /// Zone 3: Tempo - upper bound (bpm)
    pub zone3_upper: u16,

    /// Zone 4: Lactate Threshold - lower bound (bpm)
    pub zone4_lower: u16,
    /// Zone 4: Lactate Threshold - upper bound (bpm)
    pub zone4_upper: u16,

    /// Zone 5: VO2 Max - lower bound (bpm)
    pub zone5_lower: u16,
    /// Zone 5: VO2 Max - upper bound (bpm)
    pub zone5_upper: u16,

    /// Zone 6: Neuromuscular Power - lower bound (bpm, optional)
    pub zone6_lower: Option<u16>,
    /// Zone 6: Neuromuscular Power - upper bound (bpm, optional)
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
        use hr_zone_defaults::{
            LACTATE_THRESHOLD_MAX, LACTATE_THRESHOLD_MIN, SPORT_EFFICIENCY_MAX,
            SPORT_EFFICIENCY_MIN,
        };

        Self {
            vo2_max,
            resting_hr,
            max_hr,
            lactate_threshold: lactate_threshold
                .clamp(LACTATE_THRESHOLD_MIN, LACTATE_THRESHOLD_MAX),
            sport_efficiency: sport_efficiency.clamp(SPORT_EFFICIENCY_MIN, SPORT_EFFICIENCY_MAX),
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

            // Zone 6 for advanced athletes only
            zone6_lower: if self.vo2_max >= hr_zone_defaults::ELITE_ZONE6_THRESHOLD {
                Some(Self::hr_calc_to_u16(self.resting_hr, hr_reserve, 0.95))
            } else {
                None
            },
            zone6_upper: if self.vo2_max >= hr_zone_defaults::ELITE_ZONE6_THRESHOLD {
                Some(self.max_hr)
            } else {
                None
            },
        }
    }

    /// Calculate personalized running pace zones
    #[must_use]
    pub fn calculate_pace_zones(&self) -> PersonalizedPaceZones {
        use pace_zone_defaults::{
            EASY_ZONE_HIGH, EASY_ZONE_LOW, MARATHON_ADJUSTMENT_HIGH, MARATHON_ADJUSTMENT_LOW,
            NEUROMUSCULAR_ZONE_PERCENTAGE, THRESHOLD_ADJUSTMENT_HIGH, THRESHOLD_ADJUSTMENT_LOW,
            VO2MAX_ZONE_PERCENTAGE,
        };
        use vdot_coefficients::{
            THRESHOLD_ADJUSTMENT_FACTOR, THRESHOLD_VELOCITY_BASE, VDOT_COEFFICIENT_A,
            VDOT_COEFFICIENT_B, VDOT_COEFFICIENT_C,
        };

        // Calculate critical velocity at lactate threshold
        // Using simplified Jack Daniels' VDOT formulas
        let vdot = self.vo2_max;

        // Convert VDOT to velocity at VO2max (vVO2max) in m/min
        let v_vo2max = (VDOT_COEFFICIENT_C * vdot)
            .mul_add(-vdot, vdot.mul_add(VDOT_COEFFICIENT_B, VDOT_COEFFICIENT_A))
            .max(f64::MIN_POSITIVE);

        // Calculate threshold velocity
        let threshold_velocity = (v_vo2max
            * (self.lactate_threshold - 0.75)
                .mul_add(THRESHOLD_ADJUSTMENT_FACTOR, THRESHOLD_VELOCITY_BASE))
        .max(f64::MIN_POSITIVE);

        // Convert to pace (seconds per km)
        let threshold_pace = 1000.0 / threshold_velocity * 60.0;

        PersonalizedPaceZones {
            // Easy pace: % of vVO2max (slower = higher seconds/km)
            easy_pace_range: (
                1000.0 / (v_vo2max * EASY_ZONE_LOW) * 60.0,
                1000.0 / (v_vo2max * EASY_ZONE_HIGH) * 60.0,
            ),

            // Marathon pace: based on threshold pace with adjustments
            marathon_pace_range: (
                threshold_pace * MARATHON_ADJUSTMENT_LOW,
                threshold_pace * MARATHON_ADJUSTMENT_HIGH,
            ),

            // Threshold pace: adjustments around threshold
            threshold_pace_range: (
                threshold_pace * THRESHOLD_ADJUSTMENT_LOW,
                threshold_pace * THRESHOLD_ADJUSTMENT_HIGH,
            ),

            // VO2 max pace
            vo2max_pace_range: (
                1000.0 / v_vo2max * 60.0,
                1000.0 / (v_vo2max * VO2MAX_ZONE_PERCENTAGE) * 60.0,
            ),

            // Neuromuscular pace
            neuromuscular_pace_max: 1000.0 / (v_vo2max * NEUROMUSCULAR_ZONE_PERCENTAGE) * 60.0,
        }
    }

    /// Calculate functional threshold power (FTP) from VO2 max
    #[must_use]
    pub fn estimate_ftp(&self) -> f64 {
        // Use FtpAlgorithm enum for calculation
        let algorithm = FtpAlgorithm::FromVo2Max {
            vo2_max: self.vo2_max,
            power_coefficient: power_defaults::POWER_COEFFICIENT,
        };

        // Unwrap is safe here: FromVo2Max never returns Err unless VO2max is invalid,
        // but this struct ensures valid VO2max via the constructor
        algorithm.estimate_ftp().unwrap_or(0.0)
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

    /// Calculate training impulse (TRIMP) for an activity using enum-based algorithm selection
    #[must_use]
    pub fn calculate_trimp(&self, avg_hr: u16, duration_minutes: f64, gender: &str) -> f64 {
        // Use Hybrid algorithm which auto-selects appropriate formula based on gender
        let algorithm = TrimpAlgorithm::Hybrid;

        algorithm
            .calculate(
                u32::from(avg_hr),
                duration_minutes,
                u32::from(self.max_hr),
                Some(u32::from(self.resting_hr)),
                Some(gender),
            )
            .unwrap_or(0.0) // Return 0.0 if calculation fails (shouldn't happen with valid inputs)
    }
}

/// Sport-specific efficiency factors
pub trait SportEfficiency {
    /// Get the efficiency factor for this sport type
    fn sport_efficiency_factor(&self) -> f64;
}

impl SportEfficiency for SportType {
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
