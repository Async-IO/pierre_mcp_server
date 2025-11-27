// ABOUTME: Training load calculations including TSS, CTL, ATL, and TSB for fitness tracking
// ABOUTME: Implements exponential moving averages to track chronic and acute training loads
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::errors::AppError;
use crate::intelligence::metrics::MetricsCalculator;
use crate::models::Activity;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Standard CTL (Chronic Training Load) window - 42 days for long-term fitness
const CTL_WINDOW_DAYS: i64 = 42;

/// Standard ATL (Acute Training Load) window - 7 days for short-term fatigue
const ATL_WINDOW_DAYS: i64 = 7;

/// Training load metrics for an athlete
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingLoad {
    /// Chronic Training Load (42-day exponential moving average) - represents fitness
    pub ctl: f64,
    /// Acute Training Load (7-day exponential moving average) - represents fatigue
    pub atl: f64,
    /// Training Stress Balance (CTL - ATL) - represents form/freshness
    pub tsb: f64,
    /// Individual TSS values with dates for visualization
    pub tss_history: Vec<TssDataPoint>,
}

/// TSS data point with timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TssDataPoint {
    /// Date of the training session
    pub date: DateTime<Utc>,
    /// Training Stress Score for this session
    pub tss: f64,
}

/// Calculator for training load metrics
pub struct TrainingLoadCalculator {
    ctl_window_days: i64,
    atl_window_days: i64,
}

impl Default for TrainingLoadCalculator {
    fn default() -> Self {
        Self::new()
    }
}

impl TrainingLoadCalculator {
    /// Create a new training load calculator with standard windows
    #[must_use]
    pub const fn new() -> Self {
        Self {
            ctl_window_days: CTL_WINDOW_DAYS,
            atl_window_days: ATL_WINDOW_DAYS,
        }
    }

    /// Create a training load calculator with custom window sizes
    #[must_use]
    pub const fn with_windows(ctl_days: i64, atl_days: i64) -> Self {
        Self {
            ctl_window_days: ctl_days,
            atl_window_days: atl_days,
        }
    }

    /// Calculate TSS for a single activity using existing `MetricsCalculator`
    ///
    /// Returns the TSS value or an error if calculation fails
    ///
    /// # Errors
    /// Returns `AppError` if metrics calculation fails or TSS cannot be determined
    pub fn calculate_tss(
        &self,
        activity: &Activity,
        ftp: Option<f64>,
        lthr: Option<f64>,
        max_hr: Option<f64>,
        resting_hr: Option<f64>,
        weight_kg: Option<f64>,
    ) -> Result<f64, AppError> {
        let calculator = MetricsCalculator {
            ftp,
            lthr,
            max_hr,
            resting_hr,
            weight_kg,
        };

        let metrics = calculator
            .calculate_metrics(activity)
            .map_err(|e| AppError::internal(format!("Failed to calculate metrics: {e}")))?;

        metrics
            .training_stress_score
            .ok_or_else(|| AppError::internal("Unable to calculate TSS for activity".to_owned()))
    }

    /// Calculate complete training load metrics (CTL, ATL, TSB) from activities
    ///
    /// Activities should be sorted by date (oldest first) for accurate EMA calculation
    ///
    /// # Errors
    /// Returns `AppError` if TSS calculation fails for any activity
    pub fn calculate_training_load(
        &self,
        activities: &[Activity],
        ftp: Option<f64>,
        lthr: Option<f64>,
        max_hr: Option<f64>,
        resting_hr: Option<f64>,
        weight_kg: Option<f64>,
    ) -> Result<TrainingLoad, AppError> {
        if activities.is_empty() {
            return Ok(TrainingLoad {
                ctl: 0.0,
                atl: 0.0,
                tsb: 0.0,
                tss_history: Vec::new(),
            });
        }

        // Calculate TSS for each activity
        let mut tss_data: Vec<TssDataPoint> = Vec::with_capacity(activities.len());
        for activity in activities {
            if let Ok(tss) = self.calculate_tss(activity, ftp, lthr, max_hr, resting_hr, weight_kg)
            {
                tss_data.push(TssDataPoint {
                    date: activity.start_date,
                    tss,
                });
            }
        }

        if tss_data.is_empty() {
            return Ok(TrainingLoad {
                ctl: 0.0,
                atl: 0.0,
                tsb: 0.0,
                tss_history: Vec::new(),
            });
        }

        // Calculate CTL and ATL using exponential moving average
        let ctl = Self::calculate_ema(&tss_data, self.ctl_window_days);
        let atl = Self::calculate_ema(&tss_data, self.atl_window_days);
        let tsb = ctl - atl;

        Ok(TrainingLoad {
            ctl,
            atl,
            tsb,
            tss_history: tss_data,
        })
    }

    /// Calculate CTL (Chronic Training Load) - 42-day exponential moving average
    ///
    /// # Errors
    /// Returns `AppError` if training load calculation fails
    pub fn calculate_ctl(
        &self,
        activities: &[Activity],
        ftp: Option<f64>,
        lthr: Option<f64>,
        max_hr: Option<f64>,
        resting_hr: Option<f64>,
        weight_kg: Option<f64>,
    ) -> Result<f64, AppError> {
        let training_load =
            self.calculate_training_load(activities, ftp, lthr, max_hr, resting_hr, weight_kg)?;
        Ok(training_load.ctl)
    }

    /// Calculate ATL (Acute Training Load) - 7-day exponential moving average
    ///
    /// # Errors
    /// Returns `AppError` if training load calculation fails
    pub fn calculate_atl(
        &self,
        activities: &[Activity],
        ftp: Option<f64>,
        lthr: Option<f64>,
        max_hr: Option<f64>,
        resting_hr: Option<f64>,
        weight_kg: Option<f64>,
    ) -> Result<f64, AppError> {
        let training_load =
            self.calculate_training_load(activities, ftp, lthr, max_hr, resting_hr, weight_kg)?;
        Ok(training_load.atl)
    }

    /// Calculate TSB (Training Stress Balance) = CTL - ATL
    ///
    /// Interpretation:
    /// - TSB < -10: Overreaching (high fatigue, need recovery)
    /// - TSB -10 to 0: Productive training zone
    /// - TSB 0 to +10: Fresh, ready to perform
    /// - TSB > +10: Risk of detraining
    #[must_use]
    pub const fn calculate_tsb(ctl: f64, atl: f64) -> f64 {
        ctl - atl
    }

    /// Calculate exponential moving average for TSS values
    ///
    /// EMA formula: `EMA_today` = (`TSS_today` x α) + (`EMA_yesterday` x (1 - α))
    /// where α = 2 / (N + 1) and N is the window size in days
    fn calculate_ema(tss_data: &[TssDataPoint], window_days: i64) -> f64 {
        if tss_data.is_empty() {
            return 0.0;
        }

        // Sort data by date (oldest first) - required for correct EMA calculation
        // Activity APIs typically return newest-first, so we must sort here
        let mut sorted_data: Vec<&TssDataPoint> = tss_data.iter().collect();
        sorted_data.sort_by_key(|p| p.date);

        // Calculate smoothing factor: α = 2 / (N + 1)
        #[allow(clippy::cast_precision_loss)]
        let alpha = 2.0 / (window_days as f64 + 1.0);

        // Fill in missing days with zero TSS to create continuous time series
        let first_date = sorted_data[0].date;
        let last_date = sorted_data[sorted_data.len() - 1].date;

        let days_span = (last_date - first_date).num_days();

        // Create a map of date -> TSS for quick lookup
        let mut tss_map = std::collections::HashMap::new();
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

            // Apply EMA formula
            ema = daily_tss.mul_add(alpha, ema * (1.0 - alpha));
        }

        ema
    }

    /// Interpret TSB value and provide status
    #[must_use]
    pub fn interpret_tsb(tsb: f64) -> TrainingStatus {
        if tsb < -10.0 {
            TrainingStatus::Overreaching
        } else if tsb < 0.0 {
            TrainingStatus::Productive
        } else if tsb <= 10.0 {
            TrainingStatus::Fresh
        } else {
            TrainingStatus::Detraining
        }
    }

    /// Check if athlete is at risk of overtraining
    ///
    /// Warning conditions:
    /// - ATL > CTL x 1.3: Acute load spike
    /// - ATL > 150: Very high acute load
    /// - TSB < -10: Deep fatigue
    #[must_use]
    pub fn check_overtraining_risk(training_load: &TrainingLoad) -> OvertrainingRisk {
        let mut risk_factors = Vec::new();

        // Check for acute load spike
        if training_load.ctl > 0.0 && training_load.atl > training_load.ctl * 1.3 {
            risk_factors
                .push("Acute training load spike detected (>30% above chronic load)".to_owned());
        }

        // Check for very high acute load
        if training_load.atl > 150.0 {
            risk_factors.push("Very high acute training load (>150 TSS/day)".to_owned());
        }

        // Check for deep fatigue
        if training_load.tsb < -10.0 {
            risk_factors.push("Deep fatigue detected (TSB < -10) - recovery needed".to_owned());
        }

        let risk_level = if risk_factors.len() >= 2 {
            RiskLevel::High
        } else if risk_factors.len() == 1 {
            RiskLevel::Moderate
        } else {
            RiskLevel::Low
        };

        OvertrainingRisk {
            risk_level,
            risk_factors,
        }
    }

    /// Calculate recommended recovery days based on TSB
    #[must_use]
    pub fn recommend_recovery_days(tsb: f64) -> u32 {
        // Multi-level threshold function for recovery recommendations
        const VERY_DEEP_FATIGUE: f64 = -20.0;
        const DEEP_FATIGUE: f64 = -15.0;
        const MODERATE_FATIGUE: f64 = -10.0;
        const LIGHT_FATIGUE: f64 = 0.0;

        if tsb < VERY_DEEP_FATIGUE {
            return 5;
        }
        if tsb < DEEP_FATIGUE {
            return 3;
        }
        if tsb < MODERATE_FATIGUE {
            return 2;
        }
        if tsb < LIGHT_FATIGUE {
            return 1;
        }
        0
    }
}

/// Training status based on TSB
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainingStatus {
    /// TSB < -10: Overreaching, high fatigue
    Overreaching,
    /// TSB -10 to 0: Productive training zone
    Productive,
    /// TSB 0 to +10: Fresh, ready to perform
    Fresh,
    /// TSB > +10: Risk of detraining
    Detraining,
}

/// Risk level for overtraining
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Low risk of overtraining
    Low,
    /// Moderate risk - monitor closely
    Moderate,
    /// High risk - rest recommended
    High,
}

/// Overtraining risk assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OvertrainingRisk {
    /// Overall risk level
    pub risk_level: RiskLevel,
    /// Specific risk factors identified
    pub risk_factors: Vec<String>,
}
