// ABOUTME: VO2max estimation algorithms for aerobic fitness assessment
// ABOUTME: Implements VDOT, Cooper, Rockport, Astrand-Ryhming, and pace-based models for VO2max calculation
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::errors::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Rockport 1-mile walk test data
#[derive(Debug, Clone, Copy)]
struct RockportTestData {
    weight_kg: f64,
    age: u8,
    gender: u8,
    time_seconds: f64,
    heart_rate: f64,
}

/// `VO2max` estimation algorithm selection
///
/// Different algorithms for estimating maximal oxygen uptake (`VO2max`) from various tests:
///
/// - `FromVDOT`: Convert Jack Daniels' `VDOT` to `VO2max` (ml/kg/min)
/// - `CooperTest`: 12-minute run distance test
/// - `RockportWalk`: 1-mile walk test with heart rate
/// - `AstrandRyhming`: Submaximal cycle ergometer test
/// - `FromPace`: Speed-based estimation from race performance
/// - `Hybrid`: Auto-select based on available data
///
/// # Scientific References
///
/// - Daniels, J. (2013). "Daniels' Running Formula" (3rd ed.). Human Kinetics.
/// - Cooper, K.H. (1968). "A means of assessing maximal oxygen intake." *JAMA*, 203(3), 201-204.
/// - Kline, G.M., et al. (1987). "Estimation of `VO2max` from a one-mile track walk." *Medicine & Science in Sports & Exercise*, 19(3), 253-259.
/// - Åstrand, P.O., & Ryhming, I. (1954). "A nomogram for calculation of aerobic capacity." *Journal of Applied Physiology*, 7(2), 218-221.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Vo2maxAlgorithm {
    /// From Jack Daniels' VDOT
    ///
    /// Formula: `VO2max = VDOT x 3.5`
    ///
    /// `VDOT` is Jack Daniels' running economy-adjusted `VO2max` measure.
    /// Multiply by 3.5 to convert to standard ml/kg/min units.
    ///
    /// Pros: Accurate for runners, accounts for running economy
    /// Cons: Requires `VDOT` calculation from race performance
    FromVdot {
        /// VDOT value (30-85 for recreational to elite)
        vdot: f64,
    },

    /// Cooper 12-Minute Run Test
    ///
    /// Formula: `VO2max = (distance_meters - 504.9) / 44.73`
    ///
    /// Run as far as possible in 12 minutes on a flat track.
    /// `VO2max` estimated from distance covered.
    ///
    /// Pros: Simple, well-validated, widely used
    /// Cons: Requires maximal effort, pacing can affect results
    CooperTest {
        /// Distance covered in 12 minutes (meters)
        distance_meters: f64,
    },

    /// Rockport 1-Mile Walk Test
    ///
    /// Formula: `VO2max = 132.853 - 0.0769xweight - 0.3877xage + 6.315xgender - 3.2649xtime - 0.1565xHR`
    ///
    /// Walk 1 mile as fast as possible, measure time and heart rate at finish.
    /// Gender: 0 = female, 1 = male
    ///
    /// Pros: Submaximal, suitable for sedentary individuals, well-validated
    /// Cons: Less accurate for trained athletes
    RockportWalk {
        /// Body weight in kg
        weight_kg: f64,
        /// Age in years
        age: u8,
        /// Gender (0 = female, 1 = male)
        gender: u8,
        /// Time to walk 1 mile (seconds)
        time_seconds: f64,
        /// Heart rate immediately after walk (bpm)
        heart_rate: f64,
    },

    /// Åstrand-Ryhming Cycle Ergometer Test
    ///
    /// Formula: `VO2max = (VO2_submaximal x 195) / (HR_submaximal - 60)` (male)
    ///          `VO2max = (VO2_submaximal x 198) / (HR_submaximal - 72)` (female)
    ///
    /// Submaximal cycle test at steady-state heart rate (120-170 bpm).
    /// VO2 at submaximal workload estimated from power output.
    ///
    /// Pros: Submaximal, controlled conditions, good for cycling
    /// Cons: Requires cycle ergometer, HR-based (affected by medications)
    AstrandRyhming {
        /// Gender (0 = female, 1 = male)
        gender: u8,
        /// Steady-state heart rate during test (bpm)
        heart_rate: f64,
        /// Power output during test (watts)
        power_watts: f64,
        /// Body weight in kg
        weight_kg: f64,
    },

    /// From Race Pace (Speed-Based)
    ///
    /// Formula: `VO2max = 15.3 x (MaxSpeed / RecSpeed)`
    ///
    /// Where:
    /// - `MaxSpeed` = velocity at `VO2max` (typically 3-8 min pace)
    /// - `RecSpeed` = easy/recovery pace velocity
    ///
    /// Pros: Simple, based on training paces
    /// Cons: Less validated, requires accurate pace data
    FromPace {
        /// Maximum sustainable speed (m/s) for 3-8 minutes
        max_speed_ms: f64,
        /// Easy/recovery pace speed (m/s)
        recovery_speed_ms: f64,
    },

    /// Hybrid: Auto-select best method based on available data
    ///
    /// Priority:
    /// 1. Cooper test if 12-min run data available
    /// 2. Rockport walk if 1-mile walk data available
    /// 3. From `VDOT` if race performance available
    /// 4. From pace if training pace data available
    Hybrid,
}

impl Default for Vo2maxAlgorithm {
    fn default() -> Self {
        // Cooper test is the gold standard field test
        Self::CooperTest {
            distance_meters: 0.0,
        }
    }
}

impl Vo2maxAlgorithm {
    /// Estimate `VO2max` from test data
    ///
    /// # Returns
    ///
    /// Estimated `VO2max` in ml/kg/min
    ///
    /// # Errors
    ///
    /// Returns `AppError::InvalidInput` if:
    /// - Test values are outside physiological ranges
    /// - Required parameters are missing or invalid
    /// - Gender values are not 0 (female) or 1 (male)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pierre_mcp_server::intelligence::algorithms::vo2max::Vo2maxAlgorithm;
    /// use pierre_mcp_server::errors::AppResult;
    ///
    /// # fn example() -> AppResult<()> {
    /// let algorithm = Vo2maxAlgorithm::CooperTest { distance_meters: 2800.0 };
    /// let vo2max = algorithm.estimate_vo2max()?;
    /// // vo2max ≈ 51.3 ml/kg/min
    /// # Ok(())
    /// # }
    /// ```
    pub fn estimate_vo2max(&self) -> AppResult<f64> {
        match self {
            Self::FromVdot { vdot } => {
                Self::validate_vdot(*vdot)?;
                Ok(vdot * 3.5)
            }
            Self::CooperTest { distance_meters } => Self::calculate_cooper(*distance_meters),
            Self::RockportWalk {
                weight_kg,
                age,
                gender,
                time_seconds,
                heart_rate,
            } => Self::calculate_rockport(RockportTestData {
                weight_kg: *weight_kg,
                age: *age,
                gender: *gender,
                time_seconds: *time_seconds,
                heart_rate: *heart_rate,
            }),
            Self::AstrandRyhming {
                gender,
                heart_rate,
                power_watts,
                weight_kg,
            } => Self::calculate_astrand(*gender, *heart_rate, *power_watts, *weight_kg),
            Self::FromPace {
                max_speed_ms,
                recovery_speed_ms,
            } => Self::calculate_from_pace(*max_speed_ms, *recovery_speed_ms),
            Self::Hybrid => Err(AppError::invalid_input(
                "Hybrid VO2max estimation requires specific test data. Use one of the explicit test protocols.".to_owned(),
            )),
        }
    }

    /// Validate VDOT value
    fn validate_vdot(vdot: f64) -> AppResult<()> {
        if !(30.0..=85.0).contains(&vdot) {
            return Err(AppError::invalid_input(format!(
                "VDOT {vdot:.1} is outside typical range (30-85)"
            )));
        }
        Ok(())
    }

    /// Validate gender (0 = female, 1 = male)
    fn validate_gender(gender: u8) -> AppResult<()> {
        if gender > 1 {
            return Err(AppError::invalid_input(format!(
                "Gender must be 0 (female) or 1 (male), got {gender}"
            )));
        }
        Ok(())
    }

    /// Calculate `VO2max` from Cooper 12-minute test
    fn calculate_cooper(distance_meters: f64) -> AppResult<f64> {
        if distance_meters < 1000.0 {
            return Err(AppError::invalid_input(format!(
                "Cooper test distance {distance_meters:.0}m seems too low (< 1000m)"
            )));
        }

        if distance_meters > 5000.0 {
            return Err(AppError::invalid_input(format!(
                "Cooper test distance {distance_meters:.0}m seems unrealistically high (> 5000m)"
            )));
        }

        // Cooper formula: VO2max = (distance - 504.9) / 44.73
        let vo2max = (distance_meters - 504.9) / 44.73;
        Ok(vo2max.max(20.0)) // Minimum physiological VO2max
    }

    /// Calculate `VO2max` from Rockport 1-mile walk test
    fn calculate_rockport(data: RockportTestData) -> AppResult<f64> {
        Self::validate_gender(data.gender)?;

        if !(40.0..=150.0).contains(&data.weight_kg) {
            return Err(AppError::invalid_input(format!(
                "Weight {:.1}kg is outside typical range (40-150 kg)",
                data.weight_kg
            )));
        }

        if !(20..=80).contains(&data.age) {
            return Err(AppError::invalid_input(format!(
                "Age {} is outside validated range (20-80 years)",
                data.age
            )));
        }

        if !(300.0..=1800.0).contains(&data.time_seconds) {
            return Err(AppError::invalid_input(format!(
                "1-mile walk time {:.0}s is outside typical range (5-30 minutes)",
                data.time_seconds
            )));
        }

        if !(60.0..=200.0).contains(&data.heart_rate) {
            return Err(AppError::invalid_input(format!(
                "Heart rate {:.0} bpm is outside physiological range (60-200 bpm)",
                data.heart_rate
            )));
        }

        // Rockport formula
        let time_minutes = data.time_seconds / 60.0;
        #[allow(clippy::cast_precision_loss)]
        let age_f64 = f64::from(data.age);
        #[allow(clippy::cast_precision_loss)]
        let gender_f64 = f64::from(data.gender);

        let vo2max = 132.853
            - 0.0769_f64.mul_add(
                data.weight_kg,
                0.3877_f64.mul_add(
                    age_f64,
                    -(6.315_f64.mul_add(
                        gender_f64,
                        3.2649_f64.mul_add(time_minutes, 0.1565 * data.heart_rate),
                    )),
                ),
            );

        Ok(vo2max.max(20.0))
    }

    /// Calculate `VO2max` from Åstrand-Ryhming cycle test
    fn calculate_astrand(
        gender: u8,
        heart_rate: f64,
        power_watts: f64,
        weight_kg: f64,
    ) -> AppResult<f64> {
        Self::validate_gender(gender)?;

        if !(120.0..=170.0).contains(&heart_rate) {
            return Err(AppError::invalid_input(format!(
                "Submaximal heart rate {heart_rate:.0} bpm should be 120-170 bpm for accurate estimation"
            )));
        }

        if !(50.0..=300.0).contains(&power_watts) {
            return Err(AppError::invalid_input(format!(
                "Power output {power_watts:.0}W is outside typical range (50-300W)"
            )));
        }

        if !(40.0..=150.0).contains(&weight_kg) {
            return Err(AppError::invalid_input(format!(
                "Weight {weight_kg:.1}kg is outside typical range (40-150 kg)"
            )));
        }

        // Estimate VO2 at submaximal workload (approximately 10-12 ml/kg/min per watt)
        // Using cycling economy: ~10.8 ml O2 per watt
        let vo2_submaximal = (power_watts * 10.8) / weight_kg;

        // Åstrand-Ryhming formula
        let vo2max = if gender == 1 {
            // Male
            (vo2_submaximal * 195.0) / (heart_rate - 60.0)
        } else {
            // Female
            (vo2_submaximal * 198.0) / (heart_rate - 72.0)
        };

        Ok(vo2max.clamp(20.0, 90.0)) // Clamp to physiological range
    }

    /// Calculate `VO2max` from pace relationship
    fn calculate_from_pace(max_speed_ms: f64, recovery_speed_ms: f64) -> AppResult<f64> {
        if max_speed_ms <= 0.0 || recovery_speed_ms <= 0.0 {
            return Err(AppError::invalid_input(
                "Speeds must be positive".to_owned(),
            ));
        }

        if max_speed_ms <= recovery_speed_ms {
            return Err(AppError::invalid_input(
                "Max speed must be greater than recovery speed".to_owned(),
            ));
        }

        // Typical ranges for validation
        if !(3.0..=8.0).contains(&max_speed_ms) {
            return Err(AppError::invalid_input(format!(
                "Max speed {max_speed_ms:.2} m/s is outside typical range (3-8 m/s = 5:33-2:05 min/km)"
            )));
        }

        if !(2.0..=5.0).contains(&recovery_speed_ms) {
            return Err(AppError::invalid_input(format!(
                "Recovery speed {recovery_speed_ms:.2} m/s is outside typical range (2-5 m/s = 8:20-3:20 min/km)"
            )));
        }

        // Pace-based formula
        let vo2max = 15.3 * (max_speed_ms / recovery_speed_ms);
        Ok(vo2max.clamp(20.0, 90.0))
    }

    /// Get algorithm name
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::FromVdot { .. } => "from_vdot",
            Self::CooperTest { .. } => "cooper_test",
            Self::RockportWalk { .. } => "rockport_walk",
            Self::AstrandRyhming { .. } => "astrand_ryhming",
            Self::FromPace { .. } => "from_pace",
            Self::Hybrid => "hybrid",
        }
    }

    /// Get algorithm description
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Self::FromVdot { vdot } => {
                format!("From VDOT (VO2max = {vdot:.1} x 3.5)")
            }
            Self::CooperTest { distance_meters } => {
                format!("Cooper 12-Min Test ({distance_meters:.0}m)")
            }
            Self::RockportWalk {
                weight_kg,
                age,
                gender,
                time_seconds,
                heart_rate,
            } => {
                let gender_str = if *gender == 1 { "M" } else { "F" };
                let time_min = time_seconds / 60.0;
                format!(
                    "Rockport Walk ({gender_str}, {age}y, {weight_kg:.0}kg, {time_min:.1}min, {heart_rate:.0}bpm)"
                )
            }
            Self::AstrandRyhming {
                gender,
                heart_rate,
                power_watts,
                weight_kg,
            } => {
                let gender_str = if *gender == 1 { "M" } else { "F" };
                format!("Åstrand-Ryhming ({gender_str}, {power_watts:.0}W, {heart_rate:.0}bpm, {weight_kg:.0}kg)")
            }
            Self::FromPace {
                max_speed_ms,
                recovery_speed_ms,
            } => {
                format!(
                    "From Pace (max: {max_speed_ms:.2} m/s, recovery: {recovery_speed_ms:.2} m/s)"
                )
            }
            Self::Hybrid => "Hybrid (auto-select best method)".to_owned(),
        }
    }

    /// Get the formula as a string
    #[must_use]
    pub const fn formula(&self) -> &'static str {
        match self {
            Self::FromVdot { .. } => "VO2max = VDOT x 3.5",
            Self::CooperTest { .. } => "VO2max = (distance - 504.9) / 44.73",
            Self::RockportWalk { .. } => {
                "VO2max = 132.853 - 0.0769xweight - 0.3877xage + 6.315xgender - 3.2649xtime - 0.1565xHR"
            }
            Self::AstrandRyhming { .. } => {
                "VO2max = (VO2_sub x HRmax) / (HR_sub - HRrest)"
            }
            Self::FromPace { .. } => "VO2max = 15.3 x (MaxSpeed / RecSpeed)",
            Self::Hybrid => "Auto-select based on available test data",
        }
    }
}

impl FromStr for Vo2maxAlgorithm {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "from_vdot" | "vdot" => Err(AppError::invalid_input(
                "FromVdot algorithm requires VDOT parameter. Use Vo2maxAlgorithm::FromVdot { vdot: value }".to_owned()
            )),
            "cooper" | "cooper_test" => Err(AppError::invalid_input(
                "Cooper test algorithm requires distance_meters parameter. Use Vo2maxAlgorithm::CooperTest { distance_meters: value }".to_owned()
            )),
            "rockport" | "rockport_walk" => Err(AppError::invalid_input(
                "Rockport walk algorithm requires test parameters (weight_kg, age, gender, time_seconds, heart_rate). Use Vo2maxAlgorithm::RockportWalk { ... }".to_owned()
            )),
            "astrand" | "astrand_ryhming" => Err(AppError::invalid_input(
                "Astrand-Ryhming algorithm requires test parameters (gender, heart_rate, power_watts, weight_kg). Use Vo2maxAlgorithm::AstrandRyhming { ... }".to_owned()
            )),
            "from_pace" | "pace" => Err(AppError::invalid_input(
                "FromPace algorithm requires speed parameters (max_speed_ms, recovery_speed_ms). Use Vo2maxAlgorithm::FromPace { ... }".to_owned()
            )),
            "hybrid" => Ok(Self::Hybrid),
            other => Err(AppError::invalid_input(format!(
                "Unknown VO2max algorithm: '{other}'. Valid options: from_vdot, cooper_test, rockport_walk, astrand_ryhming, from_pace, hybrid"
            ))),
        }
    }
}
