// ABOUTME: VDOT (VO2max running) calculation algorithms with Daniels, Riegel, and hybrid methods
// ABOUTME: Implements Jack Daniels' VDOT methodology and Riegel's power-law race prediction formula

use crate::errors::AppError;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// VDOT calculation algorithm selection
///
/// Different algorithms for calculating running performance metrics:
///
/// - `Daniels`: Jack Daniels' VDOT formula (VO2 = -4.60 + 0.182258xv + 0.000104xv²)
/// - `Riegel`: Power-law model (T2 = T1 x (D2/D1)^1.06)
/// - `Hybrid`: Auto-select based on race distance and conditions
///
/// # Scientific References
///
/// - Daniels, J. (2013). "Daniels' Running Formula" (3rd ed.). Human Kinetics.
/// - Riegel, P.S. (1981). "Athletic records and human endurance." *American Scientist*, 69(3), 285-290.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum VdotAlgorithm {
    /// Jack Daniels' VDOT formula
    ///
    /// Formula: VO2 = -4.60 + 0.182258 x velocity + 0.000104 x velocity²
    ///
    /// Where velocity is in meters per minute
    ///
    /// Pros: Physiologically accurate, accounts for running economy
    /// Cons: Requires velocity calculation, best for 5K-Marathon distances
    #[default]
    Daniels,

    /// Riegel power-law formula
    ///
    /// Formula: T2 = T1 x (D2/D1)^1.06
    ///
    /// Predicts time for distance D2 based on time T1 for distance D1
    ///
    /// Pros: Simple, works across all distances
    /// Cons: Less accurate for very short (<1 mile) or ultra distances
    Riegel {
        /// Exponent for power-law (default 1.06, can vary by athlete: 1.03-1.08)
        exponent: f64,
    },

    /// Hybrid: Auto-select best method based on distance and data
    ///
    /// Priority:
    /// 1. Daniels for 5K-Marathon range (optimal accuracy)
    /// 2. Riegel for ultra distances or when multiple race times available
    Hybrid,
}

/// Minimum velocity for VDOT calculation (m/min)
const MIN_VELOCITY: f64 = 100.0;

/// Maximum velocity for VDOT calculation (m/min)
const MAX_VELOCITY: f64 = 500.0;

/// Jack Daniels' VO2 formula coefficient for velocity squared term
const DANIELS_A: f64 = 0.000_104;

/// Jack Daniels' VO2 formula coefficient for velocity term
const DANIELS_B: f64 = 0.182_258;

/// Jack Daniels' VO2 formula constant term
const DANIELS_C: f64 = -4.60;

impl VdotAlgorithm {
    /// Calculate VDOT from race performance
    ///
    /// # Arguments
    ///
    /// * `distance_meters` - Race distance in meters
    /// * `time_seconds` - Race time in seconds
    ///
    /// # Returns
    ///
    /// VDOT value (typically 30-85 for recreational to elite runners)
    ///
    /// # Errors
    ///
    /// Returns `AppError::InvalidInput` if:
    /// - Time or distance is non-positive
    /// - Velocity is outside valid range (100-500 m/min)
    /// - VDOT is outside typical range (30-85)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use pierre_mcp_server::intelligence::algorithms::VdotAlgorithm;
    ///
    /// let algorithm = VdotAlgorithm::Daniels;
    /// let vdot = algorithm.calculate_vdot(5000.0, 1200.0)?; // 5K in 20:00
    /// ```
    pub fn calculate_vdot(&self, distance_meters: f64, time_seconds: f64) -> Result<f64, AppError> {
        if time_seconds <= 0.0 {
            return Err(AppError::invalid_input("Time must be positive".to_owned()));
        }

        if distance_meters <= 0.0 {
            return Err(AppError::invalid_input(
                "Distance must be positive".to_owned(),
            ));
        }

        match self {
            Self::Daniels => Self::calculate_daniels(distance_meters, time_seconds),
            Self::Riegel { exponent } => {
                Self::calculate_riegel_vdot(distance_meters, time_seconds, *exponent)
            }
            Self::Hybrid => Self::calculate_hybrid(distance_meters, time_seconds),
        }
    }

    /// Predict race time for target distance given VDOT
    ///
    /// # Arguments
    ///
    /// * `vdot` - VDOT value
    /// * `target_distance_meters` - Target race distance
    ///
    /// # Returns
    ///
    /// Predicted race time in seconds
    ///
    /// # Errors
    ///
    /// Returns `AppError::InvalidInput` if VDOT is outside typical range (30-85)
    pub fn predict_time(&self, vdot: f64, target_distance_meters: f64) -> Result<f64, AppError> {
        if !(30.0..=85.0).contains(&vdot) {
            return Err(AppError::invalid_input(format!(
                "VDOT {vdot:.1} is outside typical range (30-85)"
            )));
        }

        match self {
            Self::Daniels | Self::Hybrid => {
                Self::predict_time_daniels(vdot, target_distance_meters)
            }
            Self::Riegel { exponent } => {
                Self::predict_time_riegel(vdot, target_distance_meters, *exponent)
            }
        }
    }

    /// Calculate VDOT using Daniels formula
    fn calculate_daniels(distance_meters: f64, time_seconds: f64) -> Result<f64, AppError> {
        // Convert to velocity in meters per minute
        let velocity = (distance_meters / time_seconds) * 60.0;

        if !(MIN_VELOCITY..=MAX_VELOCITY).contains(&velocity) {
            return Err(AppError::invalid_input(format!(
                "Velocity {velocity:.1} m/min is outside valid range ({MIN_VELOCITY}-{MAX_VELOCITY})"
            )));
        }

        // VO2 = -4.60 + 0.182258xv + 0.000104xv²
        let vo2 = (DANIELS_A * velocity).mul_add(velocity, DANIELS_B.mul_add(velocity, DANIELS_C));

        // Calculate percent-max adjustment based on race duration
        let percent_used = Self::calculate_percent_max_adjustment(time_seconds);
        let vdot = vo2 / percent_used;

        Ok(vdot)
    }

    /// Calculate percent-max adjustment based on race duration
    ///
    /// Shorter races use less of VO2 max due to oxygen deficit
    /// Longer races use less due to accumulated fatigue
    fn calculate_percent_max_adjustment(time_seconds: f64) -> f64 {
        let time_minutes = time_seconds / 60.0;

        if time_minutes < 5.0 {
            0.97 // Very short race - oxygen deficit
        } else if time_minutes < 15.0 {
            0.99 // 5K range
        } else if time_minutes < 30.0 {
            1.00 // 10K-15K range - optimal
        } else if time_minutes < 90.0 {
            0.98 // Half marathon range
        } else {
            0.95 // Marathon+ range - fatigue accumulation
        }
    }

    /// Calculate VDOT using Riegel power-law formula
    ///
    /// Uses reference distance (10K) to compute equivalent `VO2max`
    fn calculate_riegel_vdot(
        distance_meters: f64,
        time_seconds: f64,
        exponent: f64,
    ) -> Result<f64, AppError> {
        // Convert to 10K equivalent time
        const REFERENCE_DISTANCE: f64 = 10_000.0;
        let time_10k_equivalent =
            time_seconds * (REFERENCE_DISTANCE / distance_meters).powf(exponent);

        // Use Daniels formula for 10K to get VDOT
        Self::calculate_daniels(REFERENCE_DISTANCE, time_10k_equivalent)
    }

    /// Predict race time using Daniels VDOT tables
    fn predict_time_daniels(vdot: f64, target_distance_meters: f64) -> Result<f64, AppError> {
        // Calculate velocity at VO2 max (reverse of VDOT formula)
        // vo2 = -4.60 + 0.182258 x v + 0.000104 x v²
        // Solve quadratic: 0.000104v² + 0.182258v - (vo2 + 4.60) = 0

        let c: f64 = -(vdot + 4.60);

        let discriminant = DANIELS_B.mul_add(DANIELS_B, -(4.0 * DANIELS_A * c));
        if discriminant < 0.0 {
            return Err(AppError::internal("Invalid VDOT calculation".to_owned()));
        }

        let velocity_max = (-DANIELS_B + discriminant.sqrt()) / (2.0 * DANIELS_A);

        // Calculate race-specific velocity based on distance
        let race_velocity = Self::calculate_race_velocity(velocity_max, target_distance_meters);

        // Calculate time from velocity
        let time_seconds = (target_distance_meters / race_velocity) * 60.0;

        Ok(time_seconds)
    }

    /// Calculate race velocity based on max velocity and distance
    ///
    /// Applies fatigue factors for longer distances
    fn calculate_race_velocity(velocity_max: f64, distance_meters: f64) -> f64 {
        // Velocity percentages based on distance (Daniels' tables)
        let velocity_percent = if distance_meters <= 1_500.0 {
            1.00 // Mile/1500m - approximately VO2max pace
        } else if distance_meters <= 5_000.0 {
            0.975 // 5K pace
        } else if distance_meters <= 10_000.0 {
            0.95 // 10K pace
        } else if distance_meters <= 21_097.5 {
            0.90 // Half marathon pace
        } else if distance_meters <= 42_195.0 {
            0.85 // Marathon pace
        } else {
            // Ultra distances - further reduction
            0.80
        };

        velocity_max * velocity_percent
    }

    /// Predict time using Riegel power-law formula
    fn predict_time_riegel(
        vdot: f64,
        target_distance_meters: f64,
        exponent: f64,
    ) -> Result<f64, AppError> {
        // Use 10K as reference
        const REFERENCE_DISTANCE: f64 = 10_000.0;

        // Get 10K time from VDOT
        let time_10k = Self::predict_time_daniels(vdot, REFERENCE_DISTANCE)?;

        // Apply Riegel formula: T2 = T1 x (D2/D1)^exponent
        let predicted_time =
            time_10k * (target_distance_meters / REFERENCE_DISTANCE).powf(exponent);

        Ok(predicted_time)
    }

    /// Hybrid: Auto-select best method
    fn calculate_hybrid(distance_meters: f64, time_seconds: f64) -> Result<f64, AppError> {
        // Use Daniels for typical race distances (5K-Marathon)
        if (5_000.0..=42_195.0).contains(&distance_meters) {
            Self::calculate_daniels(distance_meters, time_seconds)
        } else {
            // Use Riegel for ultra distances
            Self::calculate_riegel_vdot(distance_meters, time_seconds, 1.06)
        }
    }

    /// Get algorithm name
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Daniels => "daniels",
            Self::Riegel { .. } => "riegel",
            Self::Hybrid => "hybrid",
        }
    }

    /// Get algorithm description
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Self::Daniels => {
                "Jack Daniels VDOT (VO2 = -4.60 + 0.182258xv + 0.000104xv²)".to_owned()
            }
            Self::Riegel { exponent } => {
                format!("Riegel power-law (T2 = T1 x (D2/D1)^{exponent:.2})")
            }
            Self::Hybrid => "Hybrid VDOT (Daniels for 5K-Marathon, Riegel for ultra)".to_owned(),
        }
    }

    /// Get the formula as a string
    #[must_use]
    pub const fn formula(&self) -> &'static str {
        match self {
            Self::Daniels => "VO2 = -4.60 + 0.182258xv + 0.000104xv²",
            Self::Riegel { .. } => "T2 = T1 x (D2/D1)^exponent",
            Self::Hybrid => "Auto-select: Daniels (5K-Marathon) or Riegel (ultra)",
        }
    }
}

impl FromStr for VdotAlgorithm {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "daniels" => Ok(Self::Daniels),
            "riegel" => Ok(Self::Riegel { exponent: 1.06 }),
            "hybrid" => Ok(Self::Hybrid),
            other => Err(AppError::invalid_input(format!(
                "Unknown VDOT algorithm: '{other}'. Valid options: daniels, riegel, hybrid"
            ))),
        }
    }
}
