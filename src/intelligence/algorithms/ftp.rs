// ABOUTME: FTP (Functional Threshold Power) estimation algorithms for cycling performance
// ABOUTME: Implements 20-min, 8-min, ramp test, 60-min, and Critical Power models for FTP calculation

use crate::errors::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// FTP estimation algorithm selection
///
/// Different algorithms for estimating Functional Threshold Power (FTP) from various test protocols:
///
/// - `From20MinTest`: 20-minute all-out test with 0.95 multiplier (Coggan standard)
/// - `From8MinTest`: 8-minute test with 0.90 multiplier (shorter alternative)
/// - `FromRampTest`: Ramp test with 0.75 multiplier of max 1-minute power
/// - `From60MinPower`: True FTP from 1-hour sustained power
/// - `CriticalPower`: Critical Power model from multiple time trials
/// - `Hybrid`: Auto-select based on available test data
///
/// # Scientific References
///
/// - Coggan, A. (2003). "Training and Racing Using a Power Meter." *Peaksware LLC*.
/// - Allen, H., & Coggan, A. (2010). "Training and Racing with a Power Meter" (2nd ed.). `VeloPress`.
/// - Monod, H., & Scherrer, J. (1965). "The work capacity of a synergic muscular group." *Ergonomics*, 8(3), 329-338.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FtpAlgorithm {
    /// 20-Minute Test Protocol (Coggan Standard)
    ///
    /// Formula: `FTP = avg_power_20min x 0.95`
    ///
    /// Most common FTP test. Ride all-out for 20 minutes after proper warmup.
    /// The 0.95 multiplier accounts for the fact that 20-minute power is slightly
    /// higher than true 1-hour sustainable power.
    ///
    /// Pros: Well-established, reproducible, manageable duration
    /// Cons: Requires fresh legs, proper pacing critical
    From20MinTest {
        /// Average power for 20-minute test (watts)
        avg_power_20min: f64,
    },

    /// 8-Minute Test Protocol
    ///
    /// Formula: `FTP = avg_power_8min x 0.90`
    ///
    /// Shorter alternative for time-constrained athletes or indoor training.
    /// Two 8-minute intervals with 10-minute recovery, use best effort.
    ///
    /// Pros: Shorter duration, less mental fatigue
    /// Cons: Less accurate, higher variability, less validated
    From8MinTest {
        /// Average power for 8-minute test (watts)
        avg_power_8min: f64,
    },

    /// Ramp Test Protocol
    ///
    /// Formula: `FTP = max_1min_power x 0.75`
    ///
    /// Progressive ramp to failure, typically 1-minute steps at increasing power.
    /// FTP estimated from peak 1-minute power achieved.
    ///
    /// Pros: Simple, quick (~20 minutes), used by `Zwift` and `TrainerRoad`
    /// Cons: Less accurate for athletes with strong anaerobic capacity
    FromRampTest {
        /// Maximum 1-minute average power achieved (watts)
        max_1min_power: f64,
    },

    /// 60-Minute Power (True FTP)
    ///
    /// Formula: `FTP = avg_power_60min x 1.0`
    ///
    /// Gold standard: best average power for 1 hour all-out effort.
    /// No multiplier needed as this IS the definition of FTP.
    ///
    /// Pros: Most accurate, true definition of FTP
    /// Cons: Very demanding, requires race or ideal conditions
    From60MinPower {
        /// Average power for 60-minute effort (watts)
        avg_power_60min: f64,
    },

    /// Critical Power Model (2-parameter)
    ///
    /// Formula: `W' = (P - CP) x t` where CP = Critical Power (≈ FTP)
    ///
    /// Uses multiple time trials at different durations to model the power-duration
    /// relationship. Critical Power (CP) represents sustainable aerobic power,
    /// while W' represents anaerobic work capacity.
    ///
    /// Pros: Most physiologically accurate, accounts for W'
    /// Cons: Requires multiple tests, complex calculation
    CriticalPower {
        /// Time trial 1: duration in seconds
        tt1_duration_seconds: f64,
        /// Time trial 1: average power in watts
        tt1_avg_power: f64,
        /// Time trial 2: duration in seconds
        tt2_duration_seconds: f64,
        /// Time trial 2: average power in watts
        tt2_avg_power: f64,
    },

    /// `VO2max`-based FTP Estimation
    ///
    /// Formula: `FTP = VO2max x 13.5 x fitness_factor`
    ///
    /// Estimates FTP from `VO2max` using physiological relationships.
    /// The 13.5 coefficient converts ml/kg/min to watts, and the fitness
    /// factor adjusts based on training level (0.75-0.85).
    ///
    /// Pros: Useful when power meter unavailable, based on lab test
    /// Cons: Less accurate than direct power testing, requires accurate `VO2max`
    FromVo2Max {
        /// `VO2max` in ml/kg/min
        vo2_max: f64,
        /// Power coefficient (typically 13.5 W per ml/kg/min)
        power_coefficient: f64,
    },

    /// Hybrid: Auto-select best method based on available data
    ///
    /// Priority:
    /// 1. Critical Power model if multiple time trials available
    /// 2. 60-minute power if available (true FTP)
    /// 3. 20-minute test (most common)
    /// 4. Ramp test (if only short data available)
    Hybrid,
}

impl Default for FtpAlgorithm {
    fn default() -> Self {
        // 20-minute test is the gold standard and most commonly used
        Self::From20MinTest {
            avg_power_20min: 0.0,
        }
    }
}

impl FtpAlgorithm {
    /// Estimate FTP from test data
    ///
    /// # Arguments
    ///
    /// * Self contains the test protocol and power data
    ///
    /// # Returns
    ///
    /// Estimated FTP in watts
    ///
    /// # Errors
    ///
    /// Returns `AppError::InvalidInput` if:
    /// - Power values are non-positive
    /// - Test durations are invalid
    /// - Critical Power model inputs are invalid
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let algorithm = FtpAlgorithm::From20MinTest { avg_power_20min: 250.0 };
    /// let ftp = algorithm.estimate_ftp()?;
    /// // ftp = 237.5 watts (250 x 0.95)
    /// ```
    pub fn estimate_ftp(&self) -> AppResult<f64> {
        match self {
            Self::From20MinTest { avg_power_20min } => {
                Self::validate_power(*avg_power_20min, "20-minute test power")?;
                Ok(avg_power_20min * 0.95)
            }
            Self::From8MinTest { avg_power_8min } => {
                Self::validate_power(*avg_power_8min, "8-minute test power")?;
                Ok(avg_power_8min * 0.90)
            }
            Self::FromRampTest { max_1min_power } => {
                Self::validate_power(*max_1min_power, "ramp test max power")?;
                Ok(max_1min_power * 0.75)
            }
            Self::From60MinPower { avg_power_60min } => {
                Self::validate_power(*avg_power_60min, "60-minute power")?;
                Ok(*avg_power_60min)
            }
            Self::CriticalPower {
                tt1_duration_seconds,
                tt1_avg_power,
                tt2_duration_seconds,
                tt2_avg_power,
            } => Self::calculate_critical_power(
                *tt1_duration_seconds,
                *tt1_avg_power,
                *tt2_duration_seconds,
                *tt2_avg_power,
            ),
            Self::FromVo2Max {
                vo2_max,
                power_coefficient,
            } => {
                if *vo2_max <= 0.0 || *vo2_max > 90.0 {
                    return Err(AppError::invalid_input(format!(
                        "VO2max must be between 0 and 90 ml/kg/min, got {vo2_max:.1}"
                    )));
                }

                let power_at_vo2max = vo2_max * power_coefficient;

                // FTP percentage based on fitness level
                let ftp_percentage = match *vo2_max {
                    v if v >= 60.0 => 0.85, // Elite
                    v if v >= 50.0 => 0.82, // Advanced
                    v if v >= 40.0 => 0.78, // Intermediate
                    _ => 0.75,              // Beginner
                };

                Ok(power_at_vo2max * ftp_percentage)
            }
            Self::Hybrid => Err(AppError::invalid_input(
                "Hybrid FTP estimation requires specific test data. Use one of the explicit test protocols.".to_owned(),
            )),
        }
    }

    /// Validate power value
    fn validate_power(power: f64, name: &str) -> AppResult<()> {
        if power <= 0.0 {
            return Err(AppError::invalid_input(format!(
                "{name} must be positive, got {power:.1}W"
            )));
        }
        if power > 1000.0 {
            return Err(AppError::invalid_input(format!(
                "{name} {power:.1}W seems unrealistically high (>1000W sustained)"
            )));
        }
        Ok(())
    }

    /// Calculate Critical Power from two time trials
    ///
    /// Uses linear regression on power-duration data:
    /// `W = CP x t + W'`
    ///
    /// Where:
    /// - CP = Critical Power (slope, ≈ FTP)
    /// - W' = Anaerobic work capacity (y-intercept)
    /// - W = Total work done (power x time)
    /// - t = Duration
    fn calculate_critical_power(
        tt1_duration: f64,
        tt1_power: f64,
        tt2_duration: f64,
        tt2_power: f64,
    ) -> AppResult<f64> {
        // Validate inputs
        if tt1_duration <= 0.0 || tt2_duration <= 0.0 {
            return Err(AppError::invalid_input(
                "Time trial durations must be positive".to_owned(),
            ));
        }

        Self::validate_power(tt1_power, "Time trial 1 power")?;
        Self::validate_power(tt2_power, "Time trial 2 power")?;

        // Durations should be sufficiently different
        let duration_ratio = tt1_duration.max(tt2_duration) / tt1_duration.min(tt2_duration);
        if duration_ratio < 1.5 {
            return Err(AppError::invalid_input(
                "Time trials should differ by at least 50% in duration for accurate CP estimation"
                    .to_owned(),
            ));
        }

        // Calculate total work for each time trial
        let work1 = tt1_power * tt1_duration;
        let work2 = tt2_power * tt2_duration;

        // Linear regression: CP = (W2 - W1) / (t2 - t1)
        let critical_power = (work2 - work1) / (tt2_duration - tt1_duration);

        // Validate result
        if critical_power <= 0.0 {
            return Err(AppError::invalid_input(
                "Critical Power calculation resulted in non-positive value. Check that longer duration has higher total work.".to_owned(),
            ));
        }

        if critical_power > tt1_power.min(tt2_power) {
            return Err(AppError::invalid_input(
                "Critical Power cannot exceed average power of time trials".to_owned(),
            ));
        }

        Ok(critical_power)
    }

    /// Calculate W' (anaerobic work capacity) from FTP and time trial data
    ///
    /// Formula: `W' = (P - FTP) x t`
    ///
    /// # Arguments
    ///
    /// * `ftp` - Functional Threshold Power (watts)
    /// * `duration_seconds` - Time trial duration (seconds)
    /// * `avg_power` - Average power during time trial (watts)
    ///
    /// # Returns
    ///
    /// W' in joules (watt-seconds)
    ///
    /// # Errors
    ///
    /// Returns `AppError::InvalidInput` if power or duration are non-positive
    pub fn calculate_w_prime(ftp: f64, duration_seconds: f64, avg_power: f64) -> AppResult<f64> {
        Self::validate_power(ftp, "FTP")?;
        Self::validate_power(avg_power, "Average power")?;

        if duration_seconds <= 0.0 {
            return Err(AppError::invalid_input(
                "Duration must be positive".to_owned(),
            ));
        }

        if avg_power <= ftp {
            return Err(AppError::invalid_input(
                "Average power must exceed FTP to calculate W' from supra-threshold effort"
                    .to_owned(),
            ));
        }

        let w_prime = (avg_power - ftp) * duration_seconds;
        Ok(w_prime)
    }

    /// Get algorithm name
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::From20MinTest { .. } => "20min_test",
            Self::From8MinTest { .. } => "8min_test",
            Self::FromRampTest { .. } => "ramp_test",
            Self::From60MinPower { .. } => "60min_power",
            Self::CriticalPower { .. } => "critical_power",
            Self::FromVo2Max { .. } => "from_vo2max",
            Self::Hybrid => "hybrid",
        }
    }

    /// Get algorithm description
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Self::From20MinTest { avg_power_20min } => {
                format!("20-Minute Test (FTP = {avg_power_20min:.1}W x 0.95)")
            }
            Self::From8MinTest { avg_power_8min } => {
                format!("8-Minute Test (FTP = {avg_power_8min:.1}W x 0.90)")
            }
            Self::FromRampTest { max_1min_power } => {
                format!("Ramp Test (FTP = {max_1min_power:.1}W x 0.75)")
            }
            Self::From60MinPower { avg_power_60min } => {
                format!("60-Minute Power (FTP = {avg_power_60min:.1}W)")
            }
            Self::CriticalPower {
                tt1_duration_seconds,
                tt1_avg_power,
                tt2_duration_seconds,
                tt2_avg_power,
            } => {
                format!(
                    "Critical Power (TT1: {tt1_avg_power:.0}W x {tt1_duration_seconds:.0}s, TT2: {tt2_avg_power:.0}W x {tt2_duration_seconds:.0}s)"
                )
            }
            Self::FromVo2Max {
                vo2_max,
                power_coefficient,
            } => {
                format!("VO2max-based FTP (VO2max={vo2_max:.1} ml/kg/min, coeff={power_coefficient:.1})")
            }
            Self::Hybrid => "Hybrid (auto-select best method)".to_owned(),
        }
    }

    /// Get the formula as a string
    #[must_use]
    pub const fn formula(&self) -> &'static str {
        match self {
            Self::From20MinTest { .. } => "FTP = avg_power_20min x 0.95",
            Self::From8MinTest { .. } => "FTP = avg_power_8min x 0.90",
            Self::FromRampTest { .. } => "FTP = max_1min_power x 0.75",
            Self::From60MinPower { .. } => "FTP = avg_power_60min",
            Self::CriticalPower { .. } => "CP = (W2 - W1) / (t2 - t1)",
            Self::FromVo2Max { .. } => "FTP = VO2max x power_coefficient x fitness_factor",
            Self::Hybrid => "Auto-select based on available test data",
        }
    }
}

impl FromStr for FtpAlgorithm {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "20min" | "20min_test" => Ok(Self::From20MinTest {
                avg_power_20min: 0.0,
            }),
            "8min" | "8min_test" => Ok(Self::From8MinTest {
                avg_power_8min: 0.0,
            }),
            "ramp" | "ramp_test" => Ok(Self::FromRampTest {
                max_1min_power: 0.0,
            }),
            "60min" | "60min_power" => Ok(Self::From60MinPower {
                avg_power_60min: 0.0,
            }),
            "cp" | "critical_power" => Ok(Self::CriticalPower {
                tt1_duration_seconds: 0.0,
                tt1_avg_power: 0.0,
                tt2_duration_seconds: 0.0,
                tt2_avg_power: 0.0,
            }),
            "vo2max" | "from_vo2max" => Ok(Self::FromVo2Max {
                vo2_max: 0.0,
                power_coefficient: 13.5,
            }),
            "hybrid" => Ok(Self::Hybrid),
            other => Err(AppError::invalid_input(format!(
                "Unknown FTP algorithm: '{other}'. Valid options: 20min_test, 8min_test, ramp_test, 60min_power, critical_power, from_vo2max, hybrid"
            ))),
        }
    }
}
