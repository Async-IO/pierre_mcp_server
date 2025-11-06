// ABOUTME: Maximum heart rate estimation algorithms using age-predicted formulas
// ABOUTME: Implements Fox, Tanaka, Nes, and Gulati formulas with scientific validation

use crate::errors::AppError;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Maximum heart rate estimation algorithm
///
/// Different formulas provide varying accuracy across populations:
///
/// - `Fox`: Classic 220-age (±10-12 bpm error, tends to overestimate)
/// - `Tanaka`: 208-0.7xage (±7-8 bpm error, current gold standard)
/// - `Nes`: 211-0.64xage (±6-7 bpm error, validated in large cohort)
/// - `Gulati`: 206-0.88xage (women-specific, ±7-8 bpm error)
///
/// # Scientific References
///
/// - Fox, S.M. et al. (1971). "Physical activity and coronary heart disease." *Ann Clin Res*, 3(6), 404-432.
/// - Tanaka, H. et al. (2001). "Age-predicted maximal heart rate revisited." *J Am Coll Cardiol*, 37(1), 153-156.
/// - Nes, B.M. et al. (2013). "Age-predicted maximal heart rate." *Scand J Med Sci Sports*, 23(6), 697-704.
/// - Gulati, M. et al. (2010). "Heart rate response to exercise stress testing." *Circulation*, 122(2), 130-137.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MaxHrAlgorithm {
    /// Fox formula: 220 - age
    ///
    /// Classic formula, widely known but least accurate
    /// Standard deviation: ±10-12 bpm
    /// Tends to overestimate max HR, especially for older adults
    Fox,

    /// Tanaka formula: 208 - 0.7 x age
    ///
    /// Current gold standard in exercise physiology
    /// Based on meta-analysis of 18,712 subjects
    /// Standard deviation: ±7-8 bpm
    /// More accurate across all age groups
    Tanaka,

    /// Nes formula: 211 - 0.64 x age
    ///
    /// Derived from Norwegian HUNT study
    /// Standard deviation: ±6-7 bpm
    /// Performs well in athletic populations
    Nes,

    /// Gulati formula: 206 - 0.88 x age
    ///
    /// Women-specific formula
    /// Standard deviation: ±7-8 bpm
    /// More accurate for female athletes than generic formulas
    Gulati,
}

impl Default for MaxHrAlgorithm {
    fn default() -> Self {
        // Use Tanaka as default (most accurate, research-backed)
        Self::Tanaka
    }
}

impl MaxHrAlgorithm {
    /// Estimate maximum heart rate from age
    ///
    /// # Arguments
    ///
    /// * `age` - Age in years (must be 1-120)
    /// * `gender` - Optional gender ("male" or "female") for gender-specific formulas
    ///
    /// # Errors
    ///
    /// Returns `AppError::InvalidInput` if age is outside valid range (1-120 years)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let max_hr = MaxHrAlgorithm::Tanaka.estimate(40, None)?;
    /// assert_eq!(max_hr, 180.0); // 208 - 0.7*40 = 180
    /// ```
    pub fn estimate(&self, age: u32, gender: Option<&str>) -> Result<f64, AppError> {
        // Validate age range
        if age == 0 || age > 120 {
            return Err(AppError::invalid_input(format!(
                "Age must be between 1 and 120 years, got {age}"
            )));
        }

        let age_f64 = f64::from(age);

        let max_hr = match self {
            Self::Fox => 220.0 - age_f64,
            Self::Tanaka => 0.7f64.mul_add(-age_f64, 208.0),
            Self::Nes => 0.64f64.mul_add(-age_f64, 211.0),
            Self::Gulati => 0.88f64.mul_add(-age_f64, 206.0),
        };

        // Apply gender-specific formula if Gulati selected for males
        // (Gulati is women-specific, fall back to Tanaka for males)
        if matches!(self, Self::Gulati) {
            if let Some(g) = gender {
                if g.eq_ignore_ascii_case("male") {
                    // Gulati is women-specific, use Tanaka for males
                    return Ok(0.7f64.mul_add(-age_f64, 208.0));
                }
            }
        }

        Ok(max_hr)
    }

    /// Get algorithm name for logging and debugging
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Fox => "fox",
            Self::Tanaka => "tanaka",
            Self::Nes => "nes",
            Self::Gulati => "gulati",
        }
    }

    /// Get algorithm description with formula
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Fox => "Fox: 220 - age (classic, ±10-12 bpm)",
            Self::Tanaka => "Tanaka: 208 - 0.7xage (gold standard, ±7-8 bpm)",
            Self::Nes => "Nes: 211 - 0.64xage (athletic populations, ±6-7 bpm)",
            Self::Gulati => "Gulati: 206 - 0.88xage (women-specific, ±7-8 bpm)",
        }
    }

    /// Get the formula as a string
    #[must_use]
    pub const fn formula(&self) -> &'static str {
        match self {
            Self::Fox => "220 - age",
            Self::Tanaka => "208 - 0.7 x age",
            Self::Nes => "211 - 0.64 x age",
            Self::Gulati => "206 - 0.88 x age",
        }
    }
}

impl FromStr for MaxHrAlgorithm {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "fox" => Ok(Self::Fox),
            "tanaka" => Ok(Self::Tanaka),
            "nes" => Ok(Self::Nes),
            "gulati" => Ok(Self::Gulati),
            other => Err(AppError::invalid_input(format!(
                "Unknown MaxHR algorithm: '{other}'. Valid options: fox, tanaka, nes, gulati"
            ))),
        }
    }
}
