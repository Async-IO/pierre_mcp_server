// ABOUTME: Training Impulse (TRIMP) calculation algorithms with gender-specific implementations
// ABOUTME: Supports Bannister male/female formulas, Edwards zone-based, Lucia banded, and hybrid approaches

use crate::errors::AppError;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// TRIMP calculation algorithm selection
///
/// Different algorithms provide varying levels of accuracy and complexity:
///
/// - `BannisterMale`: Classic Bannister formula for males (exp(1.92))
/// - `BannisterFemale`: Classic Bannister formula for females (exp(1.67))
/// - `EdwardsSimplified`: Zone-based TRIMP using 5 HR zones with linear weighting
/// - `LuciaBanded`: Sport-specific intensity bands with custom weights
/// - `Hybrid`: Auto-select Bannister based on gender, fallback to simplified
///
/// # Scientific References
///
/// - Bannister, E.W. (1991). "Modeling elite athletic performance." *Physiological Testing of Elite Athletes*.
/// - Edwards, S. (1993). "The Heart Rate Monitor Book." Polar Electro Oy.
/// - Lucia, A. et al. (2003). "Tour de France versus Vuelta a Espana." *Br J Sports Med*, 37(1), 50-55.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrimpAlgorithm {
    /// Bannister formula for males
    ///
    /// Formula: `duration_minutes × HR_reserve_fraction × 0.64 × exp(1.92 × HR_reserve_fraction)`
    ///
    /// Where `HR_reserve_fraction = (avg_hr - resting_hr) / (max_hr - resting_hr)`
    ///
    /// Pros: Physiologically validated, accounts for exponential stress
    /// Cons: Requires accurate `max_hr` and `resting_hr` values
    BannisterMale,

    /// Bannister formula for females
    ///
    /// Formula: `duration_minutes × HR_reserve_fraction × 0.64 × exp(1.67 × HR_reserve_fraction)`
    ///
    /// Uses lower exponential factor (1.67 vs 1.92) reflecting gender-specific physiology
    ///
    /// Pros: Gender-specific accuracy
    /// Cons: Same data requirements as male version
    BannisterFemale,

    /// Edwards simplified zone-based TRIMP
    ///
    /// Formula: Sum of (`zone_minutes` × `zone_number`) for zones 1-5
    ///
    /// HR Zones:
    /// - Zone 1: 50-60% `max_hr` (weight: 1)
    /// - Zone 2: 60-70% `max_hr` (weight: 2)
    /// - Zone 3: 70-80% `max_hr` (weight: 3)
    /// - Zone 4: 80-90% `max_hr` (weight: 4)
    /// - Zone 5: 90-100% `max_hr` (weight: 5)
    ///
    /// Pros: Simple, doesn't require `resting_hr`
    /// Cons: Less accurate than Bannister, requires HR stream for zone distribution
    EdwardsSimplified,

    /// Lucia sport-specific banded TRIMP
    ///
    /// Formula: Uses sport-specific intensity bands
    ///
    /// Cycling bands (Lucia 2003):
    /// - Light: <VT1 (~lactate threshold 1)
    /// - Moderate: VT1-VT2
    /// - Heavy: >VT2 (~lactate threshold 2)
    ///
    /// Pros: Sport-specific, validated in elite athletes
    /// Cons: Requires ventilatory threshold data or proxy estimates
    LuciaBanded {
        /// Sport type for band definitions
        sport: String,
    },

    /// Hybrid approach: Auto-select best method based on available data
    ///
    /// Priority:
    /// 1. Bannister (if gender, `max_hr`, `resting_hr` available)
    /// 2. Edwards (if `max_hr` available)
    /// 3. Fallback to simplified estimation
    Hybrid,
}

impl Default for TrimpAlgorithm {
    fn default() -> Self {
        Self::Hybrid
    }
}

impl TrimpAlgorithm {
    /// Calculate TRIMP for an activity
    ///
    /// # Arguments
    ///
    /// * `avg_hr` - Average heart rate in bpm
    /// * `duration_minutes` - Activity duration in minutes
    /// * `max_hr` - Maximum heart rate in bpm
    /// * `resting_hr` - Resting heart rate in bpm (optional for some methods)
    /// * `gender` - Gender ("male"/"female") for gender-specific formulas
    ///
    /// # Errors
    ///
    /// Returns `AppError::InvalidInput` if:
    /// - Heart rate values are out of valid range (20-220 bpm)
    /// - Duration is negative or zero
    /// - Required parameters missing for selected algorithm
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let algorithm = TrimpAlgorithm::BannisterMale;
    /// let trimp = algorithm.calculate(150, 60.0, 190, Some(60), Some("male"))?;
    /// ```
    pub fn calculate(
        &self,
        avg_hr: u32,
        duration_minutes: f64,
        max_hr: u32,
        resting_hr: Option<u32>,
        gender: Option<&str>,
    ) -> Result<f64, AppError> {
        // Validate heart rate ranges
        if !(20..=220).contains(&avg_hr) {
            return Err(AppError::invalid_input(format!(
                "Average heart rate must be between 20 and 220 bpm, got {avg_hr}"
            )));
        }
        if !(20..=220).contains(&max_hr) {
            return Err(AppError::invalid_input(format!(
                "Maximum heart rate must be between 20 and 220 bpm, got {max_hr}"
            )));
        }
        if let Some(rhr) = resting_hr {
            if !(20..=120).contains(&rhr) {
                return Err(AppError::invalid_input(format!(
                    "Resting heart rate must be between 20 and 120 bpm, got {rhr}"
                )));
            }
        }
        if duration_minutes <= 0.0 {
            return Err(AppError::invalid_input(
                "Duration must be greater than zero".to_string(),
            ));
        }

        match self {
            Self::BannisterMale => Ok(Self::calculate_bannister_male(
                avg_hr,
                duration_minutes,
                max_hr,
                resting_hr.ok_or_else(|| {
                    AppError::invalid_input("Resting HR required for Bannister formula".to_string())
                })?,
            )),
            Self::BannisterFemale => Ok(Self::calculate_bannister_female(
                avg_hr,
                duration_minutes,
                max_hr,
                resting_hr.ok_or_else(|| {
                    AppError::invalid_input("Resting HR required for Bannister formula".to_string())
                })?,
            )),
            Self::EdwardsSimplified => Ok(Self::calculate_edwards_simplified(
                avg_hr,
                duration_minutes,
                max_hr,
            )),
            Self::LuciaBanded { sport } => Ok(Self::calculate_lucia_banded(
                avg_hr,
                duration_minutes,
                max_hr,
                sport,
            )),
            Self::Hybrid => Ok(Self::calculate_hybrid(
                avg_hr,
                duration_minutes,
                max_hr,
                resting_hr,
                gender,
            )),
        }
    }

    /// Calculate TRIMP using Bannister formula for males
    ///
    /// Formula: `duration × HR_reserve_fraction × 0.64 × exp(1.92 × HR_reserve_fraction)`
    fn calculate_bannister_male(
        avg_hr: u32,
        duration_minutes: f64,
        max_hr: u32,
        resting_hr: u32,
    ) -> f64 {
        // Bannister male exponential factor: 1.92
        const MALE_EXPONENTIAL_FACTOR: f64 = 1.92;
        const BASE_MULTIPLIER: f64 = 0.64;

        let hr_reserve = f64::from(max_hr - resting_hr);
        let hr_ratio = (f64::from(avg_hr) - f64::from(resting_hr)) / hr_reserve;

        duration_minutes * hr_ratio * BASE_MULTIPLIER * (MALE_EXPONENTIAL_FACTOR * hr_ratio).exp()
    }

    /// Calculate TRIMP using Bannister formula for females
    ///
    /// Formula: `duration × HR_reserve_fraction × 0.64 × exp(1.67 × HR_reserve_fraction)`
    fn calculate_bannister_female(
        avg_hr: u32,
        duration_minutes: f64,
        max_hr: u32,
        resting_hr: u32,
    ) -> f64 {
        // Bannister female exponential factor: 1.67
        const FEMALE_EXPONENTIAL_FACTOR: f64 = 1.67;
        const BASE_MULTIPLIER: f64 = 0.64;

        let hr_reserve = f64::from(max_hr - resting_hr);
        let hr_ratio = (f64::from(avg_hr) - f64::from(resting_hr)) / hr_reserve;

        duration_minutes * hr_ratio * BASE_MULTIPLIER * (FEMALE_EXPONENTIAL_FACTOR * hr_ratio).exp()
    }

    /// Calculate TRIMP using Edwards simplified zone-based method
    ///
    /// Approximates zone distribution based on average HR percentage
    fn calculate_edwards_simplified(avg_hr: u32, duration_minutes: f64, max_hr: u32) -> f64 {
        let hr_percentage = (f64::from(avg_hr) / f64::from(max_hr)) * 100.0;

        // Determine zone weight based on average HR percentage
        let zone_weight = if hr_percentage < 60.0 {
            1.0
        } else if hr_percentage < 70.0 {
            2.0
        } else if hr_percentage < 80.0 {
            3.0
        } else if hr_percentage < 90.0 {
            4.0
        } else {
            5.0
        };

        duration_minutes * zone_weight
    }

    /// Calculate TRIMP using Lucia sport-specific banded method
    ///
    /// Uses estimated zones based on heart rate reserve (VT1/VT2 thresholds require lab testing)
    fn calculate_lucia_banded(
        avg_hr: u32,
        duration_minutes: f64,
        max_hr: u32,
        _sport: &str,
    ) -> f64 {
        // Simplified Lucia using HR percentage as proxy for intensity bands
        let hr_percentage = (f64::from(avg_hr) / f64::from(max_hr)) * 100.0;

        // Lucia intensity bands (approximate)
        // Light: <75% `max_hr` (weight: 1)
        // Moderate: 75-85% `max_hr` (weight: 2)
        // Heavy: >85% `max_hr` (weight: 3)
        let intensity_weight = if hr_percentage < 75.0 {
            1.0
        } else if hr_percentage < 85.0 {
            2.0
        } else {
            3.0
        };

        duration_minutes * intensity_weight
    }

    /// Hybrid approach: Auto-select best method based on available data
    fn calculate_hybrid(
        avg_hr: u32,
        duration_minutes: f64,
        max_hr: u32,
        resting_hr: Option<u32>,
        gender: Option<&str>,
    ) -> f64 {
        // Priority 1: Bannister if we have all required data
        if let Some(rhr) = resting_hr {
            if let Some(g) = gender {
                if g.eq_ignore_ascii_case("female") || g.eq_ignore_ascii_case("f") {
                    return Self::calculate_bannister_female(avg_hr, duration_minutes, max_hr, rhr);
                }
                return Self::calculate_bannister_male(avg_hr, duration_minutes, max_hr, rhr);
            }
            // Default to male formula if gender not specified
            return Self::calculate_bannister_male(avg_hr, duration_minutes, max_hr, rhr);
        }

        // Priority 2: Edwards simplified (only needs max_hr)
        Self::calculate_edwards_simplified(avg_hr, duration_minutes, max_hr)
    }

    /// Get algorithm name for logging and debugging
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::BannisterMale => "bannister_male",
            Self::BannisterFemale => "bannister_female",
            Self::EdwardsSimplified => "edwards_simplified",
            Self::LuciaBanded { .. } => "lucia_banded",
            Self::Hybrid => "hybrid",
        }
    }

    /// Get algorithm description
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Self::BannisterMale => {
                "Bannister male TRIMP (exp(1.92), requires resting HR)".to_string()
            }
            Self::BannisterFemale => {
                "Bannister female TRIMP (exp(1.67), requires resting HR)".to_string()
            }
            Self::EdwardsSimplified => "Edwards zone-based TRIMP (5 zones, simple)".to_string(),
            Self::LuciaBanded { sport } => {
                format!("Lucia sport-specific TRIMP (sport: {sport})")
            }
            Self::Hybrid => {
                "Hybrid TRIMP (auto-select Bannister or Edwards based on data)".to_string()
            }
        }
    }

    /// Get the formula as a string
    #[must_use]
    pub const fn formula(&self) -> &'static str {
        match self {
            Self::BannisterMale => {
                "duration × HR_reserve_fraction × 0.64 × exp(1.92 × HR_reserve_fraction)"
            }
            Self::BannisterFemale => {
                "duration × HR_reserve_fraction × 0.64 × exp(1.67 × HR_reserve_fraction)"
            }
            Self::EdwardsSimplified => "Σ(zone_minutes × zone_weight) for zones 1-5",
            Self::LuciaBanded { .. } => "Σ(band_minutes × band_weight) for intensity bands",
            Self::Hybrid => "Auto-select best method based on available data",
        }
    }
}

impl FromStr for TrimpAlgorithm {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bannister_male" | "bannister" | "male" => Ok(Self::BannisterMale),
            "bannister_female" | "female" => Ok(Self::BannisterFemale),
            "edwards_simplified" | "edwards" | "zones" => Ok(Self::EdwardsSimplified),
            "lucia_banded" | "lucia" => Ok(Self::LuciaBanded {
                sport: "cycling".to_string(),
            }),
            "hybrid" => Ok(Self::Hybrid),
            other => Err(AppError::invalid_input(format!(
                "Unknown TRIMP algorithm: '{other}'. Valid options: bannister_male, bannister_female, edwards_simplified, lucia_banded, hybrid"
            ))),
        }
    }
}
