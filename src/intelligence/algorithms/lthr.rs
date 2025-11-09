// ABOUTME: LTHR (Lactate Threshold Heart Rate) estimation algorithms for endurance training
// ABOUTME: Implements MaxHR-based, 30-min test, ramp test, and Friel method for LTHR calculation

use crate::errors::AppError;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// LTHR estimation algorithm selection
///
/// Different algorithms for estimating Lactate Threshold Heart Rate (LTHR) from various test protocols:
///
/// - `FromMaxHR`: Percentage of maximum heart rate (85-91%)
/// - `From30MinTest`: 30-minute time trial with 1.03 multiplier
/// - `FromRampTest`: Average HR from last 20 minutes of ramp test
/// - `FrielMethod`: HR drift analysis in sustained efforts
/// - `Hybrid`: Auto-select based on available test data
///
/// # Scientific References
///
/// - Friel, J. (2009). "The Cyclist's Training Bible" (4th ed.). `VeloPress`.
/// - Seiler, S., & Tønnessen, E. (2009). "Intervals, thresholds, and long slow distance." *Sportscience*, 13, 32-53.
/// - Billat, V.L. (1996). "Use of blood lactate measurements for prediction of exercise performance." *Sports Medicine*, 22(3), 157-175.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LthrAlgorithm {
    /// Percentage of Maximum Heart Rate
    ///
    /// Formula: `LTHR = MaxHR x percentage`
    ///
    /// Typical percentages:
    /// - 0.85 (85%): Untrained individuals
    /// - 0.88 (88%): Recreationally trained
    /// - 0.91 (91%): Well-trained endurance athletes
    ///
    /// Simple estimation but individual variation can be high (±5-10 bpm).
    ///
    /// Pros: Simple, no test required if `MaxHR` known
    /// Cons: High individual variability, less accurate than field tests
    FromMaxHR {
        /// Maximum heart rate (bpm)
        max_hr: f64,
        /// Percentage of `MaxHR` (0.85-0.91, default 0.88)
        percentage: f64,
    },

    /// 30-Minute Time Trial Test
    ///
    /// Formula: `LTHR = avg_hr_30min x 1.03`
    ///
    /// Perform a 30-minute all-out time trial after proper warmup.
    /// Average HR for the entire 30 minutes, then apply 1.03 multiplier.
    ///
    /// The multiplier accounts for cardiac drift and slightly elevated HR
    /// at lactate threshold vs. 30-minute sustainable pace.
    ///
    /// Pros: Accurate, practical, well-validated
    /// Cons: Requires sustained maximal effort, proper pacing critical
    From30MinTest {
        /// Average heart rate for 30-minute effort (bpm)
        avg_hr_30min: f64,
    },

    /// Ramp Test Protocol
    ///
    /// Formula: `LTHR = avg_hr_last_20min`
    ///
    /// Progressive ramp to failure, typically 1-minute steps at increasing intensity.
    /// LTHR estimated as average HR during last 20 minutes before failure.
    ///
    /// Pros: Same test can estimate both FTP and LTHR
    /// Cons: Less validated than 30-minute test, can overestimate LTHR
    FromRampTest {
        /// Heart rate samples from last 20 minutes (bpm)
        hr_samples: Vec<f64>,
    },

    /// Friel Method (HR Drift Analysis)
    ///
    /// Formula: `LTHR = HR at which drift begins`
    ///
    /// Perform steady-state ride at moderate intensity. Monitor HR drift:
    /// - First half average HR
    /// - Second half average HR
    /// - If drift > 5%, intensity is above LTHR
    ///
    /// Repeat at different intensities to find HR where drift ≤ 5%.
    ///
    /// Pros: Precise, accounts for individual response
    /// Cons: Requires multiple tests, time-consuming
    FrielMethod {
        /// Average HR for first half of test (bpm)
        first_half_avg_hr: f64,
        /// Average HR for second half of test (bpm)
        second_half_avg_hr: f64,
        /// Duration of each half (seconds)
        half_duration_seconds: f64,
    },

    /// Hybrid: Auto-select best method based on available data
    ///
    /// Priority:
    /// 1. Friel method if HR drift data available (most accurate)
    /// 2. 30-minute test if available (gold standard)
    /// 3. Ramp test if HR samples available
    /// 4. `MaxHR` percentage (fallback)
    Hybrid,
}

impl Default for LthrAlgorithm {
    fn default() -> Self {
        // 30-minute test is the gold standard for field testing
        Self::From30MinTest { avg_hr_30min: 0.0 }
    }
}

impl LthrAlgorithm {
    /// Estimate LTHR from test data
    ///
    /// # Returns
    ///
    /// Estimated LTHR in beats per minute (bpm)
    ///
    /// # Errors
    ///
    /// Returns `AppError::InvalidInput` if:
    /// - Heart rate values are outside physiological range (40-220 bpm)
    /// - Test data is insufficient or invalid
    /// - Percentage is outside valid range (0.80-0.95)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let algorithm = LthrAlgorithm::From30MinTest { avg_hr_30min: 165.0 };
    /// let lthr = algorithm.estimate_lthr()?;
    /// // lthr = 169.95 bpm (165 x 1.03)
    /// ```
    pub fn estimate_lthr(&self) -> Result<f64, AppError> {
        match self {
            Self::FromMaxHR { max_hr, percentage } => {
                Self::validate_hr(*max_hr, "Maximum heart rate")?;
                Self::validate_percentage(*percentage)?;
                Ok(max_hr * percentage)
            }
            Self::From30MinTest { avg_hr_30min } => {
                Self::validate_hr(*avg_hr_30min, "30-minute average HR")?;
                Ok(avg_hr_30min * 1.03)
            }
            Self::FromRampTest { hr_samples } => Self::calculate_ramp_test_lthr(hr_samples),
            Self::FrielMethod {
                first_half_avg_hr,
                second_half_avg_hr,
                half_duration_seconds,
            } => Self::calculate_friel_lthr(
                *first_half_avg_hr,
                *second_half_avg_hr,
                *half_duration_seconds,
            ),
            Self::Hybrid => Err(AppError::invalid_input(
                "Hybrid LTHR estimation requires specific test data. Use one of the explicit test protocols.".to_owned(),
            )),
        }
    }

    /// Validate heart rate value
    fn validate_hr(hr: f64, name: &str) -> Result<(), AppError> {
        if !(40.0..=220.0).contains(&hr) {
            return Err(AppError::invalid_input(format!(
                "{name} {hr:.1} bpm is outside physiological range (40-220 bpm)"
            )));
        }
        Ok(())
    }

    /// Validate LTHR percentage
    fn validate_percentage(percentage: f64) -> Result<(), AppError> {
        if !(0.80..=0.95).contains(&percentage) {
            return Err(AppError::invalid_input(format!(
                "LTHR percentage {percentage:.2} is outside valid range (0.80-0.95)"
            )));
        }
        Ok(())
    }

    /// Calculate LTHR from ramp test HR samples
    fn calculate_ramp_test_lthr(hr_samples: &[f64]) -> Result<f64, AppError> {
        if hr_samples.is_empty() {
            return Err(AppError::invalid_input(
                "Ramp test requires HR samples".to_owned(),
            ));
        }

        if hr_samples.len() < 10 {
            return Err(AppError::invalid_input(format!(
                "Ramp test requires at least 10 HR samples, got {}",
                hr_samples.len()
            )));
        }

        // Validate all HR samples
        for (i, &hr) in hr_samples.iter().enumerate() {
            Self::validate_hr(hr, &format!("HR sample {i}"))?;
        }

        // Calculate average of all samples
        #[allow(clippy::cast_precision_loss)]
        let avg_hr = hr_samples.iter().sum::<f64>() / hr_samples.len() as f64;

        Ok(avg_hr)
    }

    /// Calculate LTHR using Friel method (HR drift analysis)
    fn calculate_friel_lthr(
        first_half_avg_hr: f64,
        second_half_avg_hr: f64,
        half_duration_seconds: f64,
    ) -> Result<f64, AppError> {
        Self::validate_hr(first_half_avg_hr, "First half average HR")?;
        Self::validate_hr(second_half_avg_hr, "Second half average HR")?;

        if half_duration_seconds < 600.0 {
            return Err(AppError::invalid_input(
                "Friel method requires at least 10-minute halves (600 seconds)".to_owned(),
            ));
        }

        // Calculate HR drift percentage
        let hr_drift_percent =
            ((second_half_avg_hr - first_half_avg_hr) / first_half_avg_hr) * 100.0;

        // If drift > 5%, intensity is above LTHR
        // If drift ≤ 5%, use average of both halves as LTHR estimate
        if hr_drift_percent > 5.0 {
            return Err(AppError::invalid_input(format!(
                "HR drift {hr_drift_percent:.1}% exceeds 5% threshold. Intensity is above LTHR. Reduce intensity and retest."
            )));
        }

        // Use average of both halves as LTHR estimate
        // Use midpoint calculation that avoids overflow
        let lthr = first_half_avg_hr + (second_half_avg_hr - first_half_avg_hr) / 2.0;
        Ok(lthr)
    }

    /// Calculate HR drift percentage
    ///
    /// # Arguments
    ///
    /// * `first_half_avg_hr` - Average HR for first half of test (bpm)
    /// * `second_half_avg_hr` - Average HR for second half of test (bpm)
    ///
    /// # Returns
    ///
    /// HR drift as percentage (positive = drift up, negative = drift down)
    ///
    /// # Errors
    ///
    /// Returns `AppError::InvalidInput` if HR values are outside physiological range
    pub fn calculate_hr_drift(
        first_half_avg_hr: f64,
        second_half_avg_hr: f64,
    ) -> Result<f64, AppError> {
        Self::validate_hr(first_half_avg_hr, "First half average HR")?;
        Self::validate_hr(second_half_avg_hr, "Second half average HR")?;

        let drift_percent = ((second_half_avg_hr - first_half_avg_hr) / first_half_avg_hr) * 100.0;
        Ok(drift_percent)
    }

    /// Get algorithm name
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::FromMaxHR { .. } => "from_max_hr",
            Self::From30MinTest { .. } => "30min_test",
            Self::FromRampTest { .. } => "ramp_test",
            Self::FrielMethod { .. } => "friel_method",
            Self::Hybrid => "hybrid",
        }
    }

    /// Get algorithm description
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Self::FromMaxHR { max_hr, percentage } => {
                format!("From MaxHR (LTHR = {max_hr:.0} bpm x {percentage:.2})")
            }
            Self::From30MinTest { avg_hr_30min } => {
                format!("30-Minute Test (LTHR = {avg_hr_30min:.0} bpm x 1.03)")
            }
            Self::FromRampTest { hr_samples } => {
                format!("Ramp Test ({} HR samples)", hr_samples.len())
            }
            Self::FrielMethod {
                first_half_avg_hr,
                second_half_avg_hr,
                ..
            } => {
                let drift = ((second_half_avg_hr - first_half_avg_hr) / first_half_avg_hr) * 100.0;
                format!(
                    "Friel Method (HR drift: {drift:.1}%, avg: {:.0} bpm)",
                    (first_half_avg_hr + second_half_avg_hr) / 2.0
                )
            }
            Self::Hybrid => "Hybrid (auto-select best method)".to_owned(),
        }
    }

    /// Get the formula as a string
    #[must_use]
    pub const fn formula(&self) -> &'static str {
        match self {
            Self::FromMaxHR { .. } => "LTHR = MaxHR x percentage (0.85-0.91)",
            Self::From30MinTest { .. } => "LTHR = avg_hr_30min x 1.03",
            Self::FromRampTest { .. } => "LTHR = avg(HR_last_20min)",
            Self::FrielMethod { .. } => "LTHR = avg(HR) where drift ≤ 5%",
            Self::Hybrid => "Auto-select based on available test data",
        }
    }
}

impl FromStr for LthrAlgorithm {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "from_max_hr" | "maxhr" => Ok(Self::FromMaxHR {
                max_hr: 0.0,
                percentage: 0.88,
            }),
            "30min" | "30min_test" => Ok(Self::From30MinTest { avg_hr_30min: 0.0 }),
            "ramp" | "ramp_test" => Ok(Self::FromRampTest {
                hr_samples: Vec::new(),
            }),
            "friel" | "friel_method" => Ok(Self::FrielMethod {
                first_half_avg_hr: 0.0,
                second_half_avg_hr: 0.0,
                half_duration_seconds: 0.0,
            }),
            "hybrid" => Ok(Self::Hybrid),
            other => Err(AppError::invalid_input(format!(
                "Unknown LTHR algorithm: '{other}'. Valid options: from_max_hr, 30min_test, ramp_test, friel_method, hybrid"
            ))),
        }
    }
}
