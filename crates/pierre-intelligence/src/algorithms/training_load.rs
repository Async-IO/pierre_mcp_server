// ABOUTME: Training load calculation algorithms (CTL/ATL/TSB) with multiple moving average methods
// ABOUTME: Implements EMA (standard), SMA, WMA, and Kalman Filter for fitness tracking
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use std::collections::HashMap;
use std::str::FromStr;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::errors::{AppError, AppResult};

/// Training load calculation algorithm selection
///
/// Different algorithms for calculating CTL (Chronic Training Load), ATL (Acute Training Load),
/// and TSB (Training Stress Balance):
///
/// - `Ema`: Exponential Moving Average (TrainingPeaks/Coggan standard)
/// - `Sma`: Simple Moving Average (equal weights)
/// - `Wma`: Weighted Moving Average (linear decay)
/// - `KalmanFilter`: State estimation with noise modeling
///
/// # Scientific References
///
/// - Coggan, A. (2003). "Training and Racing Using a Power Meter." *Peaksware LLC*.
/// - Banister, E.W. (1991). "Modeling elite athletic performance." *Physiological Testing of Elite Athletes*.
/// - Kalman, R.E. (1960). "A New Approach to Linear Filtering." *Journal of Basic Engineering*, 82(1), 35-45.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TrainingLoadAlgorithm {
    /// Exponential Moving Average (EMA)
    ///
    /// Formula: `α = 2/(N+1)`, `EMA_t = α x TSS_t + (1-α) x EMA_{t-1}`
    ///
    /// Standard method used by `TrainingPeaks` Performance Manager Chart.
    /// Recent days weighted more heavily with exponential decay.
    ///
    /// Pros: Smooth response, standard in industry
    /// Cons: Requires all historical data for initialization
    Ema {
        /// CTL window in days (default 42 for fitness)
        ctl_days: i64,
        /// ATL window in days (default 7 for fatigue)
        atl_days: i64,
    },

    /// Simple Moving Average (SMA)
    ///
    /// Formula: `SMA = Σ(TSS_i) / N` for i in [t-N+1, t]
    ///
    /// All days in window weighted equally.
    ///
    /// Pros: Simple, intuitive, no historical data needed
    /// Cons: Step changes at window boundaries, less responsive
    Sma {
        /// CTL window in days
        ctl_days: i64,
        /// ATL window in days
        atl_days: i64,
    },

    /// Weighted Moving Average (WMA)
    ///
    /// Formula: `WMA = Σ(w_i x TSS_i) / Σ(w_i)` where `w_i = i` (linear weights)
    ///
    /// Recent days weighted linearly more than older days.
    ///
    /// Pros: More responsive than SMA, simpler than EMA
    /// Cons: Still has boundary effects
    Wma {
        /// CTL window in days
        ctl_days: i64,
        /// ATL window in days
        atl_days: i64,
    },

    /// Kalman Filter
    ///
    /// State-space model with process and measurement noise.
    /// Optimal estimation when data is noisy or has gaps.
    ///
    /// Pros: Optimal for noisy data, handles gaps well
    /// Cons: Complex, requires tuning noise parameters
    KalmanFilter {
        /// Process noise (training load variability)
        process_noise: f64,
        /// Measurement noise (TSS measurement error)
        measurement_noise: f64,
    },
}

impl Default for TrainingLoadAlgorithm {
    fn default() -> Self {
        Self::Ema {
            ctl_days: 42,
            atl_days: 7,
        }
    }
}

/// TSS data point with timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TssDataPoint {
    /// Date of the training session
    pub date: DateTime<Utc>,
    /// Training Stress Score for this session
    pub tss: f64,
}

impl TrainingLoadAlgorithm {
    /// Calculate CTL (Chronic Training Load) using selected algorithm
    ///
    /// # Arguments
    ///
    /// * `tss_data` - Time series of TSS values with dates (sorted oldest to newest)
    ///
    /// # Returns
    ///
    /// CTL value representing long-term fitness
    ///
    /// # Errors
    ///
    /// Returns `AppError::InvalidInput` if:
    /// - TSS data is empty
    /// - Dates are not properly ordered
    /// - Window sizes are invalid
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pierre_mcp_server::intelligence::algorithms::training_load::{
    ///     TrainingLoadAlgorithm, TssDataPoint,
    /// };
    /// use chrono::{Duration, Utc};
    /// # fn example() -> Result<(), pierre_mcp_server::errors::AppError> {
    ///
    /// // Create TSS data for the past week
    /// let now = Utc::now();
    /// let tss_data: Vec<TssDataPoint> = (0..7)
    ///     .map(|day| TssDataPoint {
    ///         date: now - Duration::days(6 - day),
    ///         tss: 50.0 + (day as f64 * 10.0), // 50, 60, 70, 80, 90, 100, 110
    ///     })
    ///     .collect();
    ///
    /// // Use default EMA algorithm (42-day CTL, 7-day ATL)
    /// let algorithm = TrainingLoadAlgorithm::default();
    /// let ctl = algorithm.calculate_ctl(&tss_data)?;
    /// println!("CTL (fitness): {:.1}", ctl);
    ///
    /// // Use different algorithm variants
    /// let sma = TrainingLoadAlgorithm::Sma { ctl_days: 42, atl_days: 7 };
    /// let ctl_sma = sma.calculate_ctl(&tss_data)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn calculate_ctl(&self, tss_data: &[TssDataPoint]) -> AppResult<f64> {
        if tss_data.is_empty() {
            return Ok(0.0);
        }

        match self {
            Self::Ema { ctl_days, .. } => Self::calculate_ema(tss_data, *ctl_days),
            Self::Sma { ctl_days, .. } => Self::calculate_sma(tss_data, *ctl_days),
            Self::Wma { ctl_days, .. } => Self::calculate_wma(tss_data, *ctl_days),
            Self::KalmanFilter {
                process_noise,
                measurement_noise,
            } => Self::calculate_kalman(tss_data, *process_noise, *measurement_noise),
        }
    }

    /// Calculate ATL (Acute Training Load) using selected algorithm
    ///
    /// # Arguments
    ///
    /// * `tss_data` - Time series of TSS values with dates (sorted oldest to newest)
    ///
    /// # Returns
    ///
    /// ATL value representing short-term fatigue
    ///
    /// # Errors
    ///
    /// Returns `AppError::InvalidInput` if:
    /// - Dates are not properly ordered
    /// - Window sizes are invalid
    pub fn calculate_atl(&self, tss_data: &[TssDataPoint]) -> AppResult<f64> {
        if tss_data.is_empty() {
            return Ok(0.0);
        }

        match self {
            Self::Ema { atl_days, .. } => Self::calculate_ema(tss_data, *atl_days),
            Self::Sma { atl_days, .. } => Self::calculate_sma(tss_data, *atl_days),
            Self::Wma { atl_days, .. } => Self::calculate_wma(tss_data, *atl_days),
            Self::KalmanFilter {
                process_noise,
                measurement_noise,
            } => Self::calculate_kalman(tss_data, *process_noise, *measurement_noise),
        }
    }

    /// Calculate TSB (Training Stress Balance) = CTL - ATL
    ///
    /// # Interpretation
    ///
    /// - TSB < -10: Overreaching (high fatigue, need recovery)
    /// - TSB -10 to 0: Productive training zone
    /// - TSB 0 to +10: Fresh, ready to perform
    /// - TSB > +10: Risk of detraining
    #[must_use]
    pub const fn calculate_tsb(ctl: f64, atl: f64) -> f64 {
        ctl - atl
    }

    /// Calculate Exponential Moving Average
    ///
    /// Formula: `α = 2/(N+1)`, `EMA_t = α x TSS_t + (1-α) x EMA_{t-1}`
    fn calculate_ema(tss_data: &[TssDataPoint], window_days: i64) -> AppResult<f64> {
        if tss_data.is_empty() {
            return Ok(0.0);
        }

        if window_days <= 0 {
            return Err(AppError::invalid_input(format!(
                "Window size must be positive, got {window_days}"
            )));
        }

        // Calculate smoothing factor: α = 2 / (N + 1)
        #[allow(clippy::cast_precision_loss)]
        let alpha = 2.0 / (window_days as f64 + 1.0);

        // Fill in missing days with zero TSS to create continuous time series
        let first_date = tss_data[0].date;
        let last_date = tss_data[tss_data.len() - 1].date;

        let days_span = (last_date - first_date).num_days();
        if days_span < 0 {
            return Err(AppError::invalid_input(
                "TSS data not sorted by date".to_owned(),
            ));
        }

        // Create a map of date -> TSS for quick lookup
        let mut tss_map = HashMap::new();
        for point in tss_data {
            let date_key = point.date.date_naive();
            *tss_map.entry(date_key).or_insert(0.0) += point.tss;
        }

        // Calculate EMA day by day
        let mut ema = 0.0;
        for day_offset in 0..=days_span {
            let current_date = first_date + Duration::days(day_offset);
            let date_key = current_date.date_naive();
            let daily_tss = tss_map.get(&date_key).copied().unwrap_or(0.0);

            // Apply EMA formula: EMA_t = α x TSS_t + (1-α) x EMA_{t-1}
            ema = daily_tss.mul_add(alpha, ema * (1.0 - alpha));
        }

        Ok(ema)
    }

    /// Calculate Simple Moving Average
    ///
    /// Formula: `SMA = Σ(TSS_i) / N` for i in [t-N+1, t]
    fn calculate_sma(tss_data: &[TssDataPoint], window_days: i64) -> AppResult<f64> {
        if tss_data.is_empty() {
            return Ok(0.0);
        }

        if window_days <= 0 {
            return Err(AppError::invalid_input(format!(
                "Window size must be positive, got {window_days}"
            )));
        }

        // Get the most recent window_days of data
        let last_date = tss_data[tss_data.len() - 1].date;
        let window_start = last_date - Duration::days(window_days - 1);

        // Sum TSS values in window
        let mut sum = 0.0;
        for point in tss_data {
            if point.date >= window_start {
                sum += point.tss;
            }
        }

        // Average over window (missing days count as zero)
        #[allow(clippy::cast_precision_loss)]
        let average = sum / window_days as f64;

        Ok(average)
    }

    /// Calculate Weighted Moving Average
    ///
    /// Formula: `WMA = Σ(w_i x TSS_i) / Σ(w_i)` where weights are linear (1, 2, 3, ..., N)
    fn calculate_wma(tss_data: &[TssDataPoint], window_days: i64) -> AppResult<f64> {
        if tss_data.is_empty() {
            return Ok(0.0);
        }

        if window_days <= 0 {
            return Err(AppError::invalid_input(format!(
                "Window size must be positive, got {window_days}"
            )));
        }

        let last_date = tss_data[tss_data.len() - 1].date;
        let window_start = last_date - Duration::days(window_days - 1);

        // Create daily TSS map
        let mut tss_map = HashMap::new();
        for point in tss_data {
            if point.date >= window_start {
                let date_key = point.date.date_naive();
                *tss_map.entry(date_key).or_insert(0.0) += point.tss;
            }
        }

        // Calculate WMA with linear weights (older = lower weight)
        let mut weighted_sum = 0.0;
        let mut weight_sum = 0.0;

        for day_offset in 0..window_days {
            let current_date = window_start + Duration::days(day_offset);
            let date_key = current_date.date_naive();
            let daily_tss = tss_map.get(&date_key).copied().unwrap_or(0.0);

            // Weight increases linearly: 1, 2, 3, ..., N
            #[allow(clippy::cast_precision_loss)]
            let weight = (day_offset + 1) as f64;

            weighted_sum += daily_tss * weight;
            weight_sum += weight;
        }

        let wma = if weight_sum > 0.0 {
            weighted_sum / weight_sum
        } else {
            0.0
        };

        Ok(wma)
    }

    /// Calculate Kalman Filter estimate
    ///
    /// Simplified 1D Kalman filter for training load estimation
    fn calculate_kalman(
        tss_data: &[TssDataPoint],
        process_noise: f64,
        measurement_noise: f64,
    ) -> AppResult<f64> {
        if tss_data.is_empty() {
            return Ok(0.0);
        }

        if process_noise <= 0.0 || measurement_noise <= 0.0 {
            return Err(AppError::invalid_input(
                "Noise parameters must be positive".to_owned(),
            ));
        }

        let first_date = tss_data[0].date;
        let last_date = tss_data[tss_data.len() - 1].date;
        let days_span = (last_date - first_date).num_days();

        if days_span < 0 {
            return Err(AppError::invalid_input(
                "TSS data not sorted by date".to_owned(),
            ));
        }

        // Create daily TSS map
        let mut tss_map = HashMap::new();
        for point in tss_data {
            let date_key = point.date.date_naive();
            *tss_map.entry(date_key).or_insert(0.0) += point.tss;
        }

        // Initialize Kalman filter state
        let mut estimate = tss_data[0].tss; // Initial estimate
        let mut error_covariance = 1.0; // Initial error covariance

        // Process each day
        for day_offset in 0..=days_span {
            let current_date = first_date + Duration::days(day_offset);
            let date_key = current_date.date_naive();
            let measurement = tss_map.get(&date_key).copied().unwrap_or(0.0);

            // Prediction step
            let predicted_estimate = estimate;
            let predicted_covariance = error_covariance + process_noise;

            // Update step (only if we have a measurement)
            if measurement > 0.0 {
                // Kalman gain
                let kalman_gain = predicted_covariance / (predicted_covariance + measurement_noise);

                // Update estimate
                estimate =
                    kalman_gain.mul_add(measurement - predicted_estimate, predicted_estimate);

                // Update error covariance
                error_covariance = (1.0 - kalman_gain) * predicted_covariance;
            } else {
                // No measurement, just propagate prediction
                estimate = predicted_estimate;
                error_covariance = predicted_covariance;
            }
        }

        Ok(estimate)
    }

    /// Get algorithm name
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Ema { .. } => "ema",
            Self::Sma { .. } => "sma",
            Self::Wma { .. } => "wma",
            Self::KalmanFilter { .. } => "kalman",
        }
    }

    /// Get algorithm description
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Self::Ema { ctl_days, atl_days } => {
                format!("Exponential Moving Average (CTL={ctl_days}d, ATL={atl_days}d, α=2/(N+1))")
            }
            Self::Sma { ctl_days, atl_days } => {
                format!("Simple Moving Average (CTL={ctl_days}d, ATL={atl_days}d)")
            }
            Self::Wma { ctl_days, atl_days } => {
                format!(
                    "Weighted Moving Average (CTL={ctl_days}d, ATL={atl_days}d, linear weights)"
                )
            }
            Self::KalmanFilter {
                process_noise,
                measurement_noise,
            } => {
                format!("Kalman Filter (Q={process_noise:.3}, R={measurement_noise:.3})")
            }
        }
    }

    /// Get the formula as a string
    #[must_use]
    pub const fn formula(&self) -> &'static str {
        match self {
            Self::Ema { .. } => "EMA_t = α x TSS_t + (1-α) x EMA_{t-1}, α = 2/(N+1)",
            Self::Sma { .. } => "SMA = Σ(TSS_i) / N",
            Self::Wma { .. } => "WMA = Σ(i x TSS_i) / Σ(i)",
            Self::KalmanFilter { .. } => {
                "x̂_t = x̂_{t-1} + K_t(z_t - x̂_{t-1}), K_t = P_{t|t-1}/(P_{t|t-1} + R)"
            }
        }
    }
}

impl FromStr for TrainingLoadAlgorithm {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ema" => Ok(Self::Ema {
                ctl_days: 42,
                atl_days: 7,
            }),
            "sma" => Ok(Self::Sma {
                ctl_days: 42,
                atl_days: 7,
            }),
            "wma" => Ok(Self::Wma {
                ctl_days: 42,
                atl_days: 7,
            }),
            "kalman" | "kalman_filter" => Ok(Self::KalmanFilter {
                process_noise: 1.0,
                measurement_noise: 10.0,
            }),
            other => Err(AppError::invalid_input(format!(
                "Unknown training load algorithm: '{other}'. Valid options: ema, sma, wma, kalman"
            ))),
        }
    }
}
