// ABOUTME: Sleep and recovery configuration for athlete recovery analysis
// ABOUTME: Configures sleep duration thresholds, HRV analysis, TSB ranges, and recovery scoring
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Sleep and Recovery Configuration
//!
//! Provides configuration for sleep tracking and recovery analysis including
//! sleep duration thresholds, HRV analysis, TSB (Training Stress Balance),
//! and recovery scoring weights.
//!
//! # Scientific References
//!
//! - Sleep duration: NSF/AASM guidelines (Watson et al. 2015, Hirshkowitz et al. 2015)
//! - Sleep stages: AASM sleep stage guidelines
//! - HRV: Shaffer & Ginsberg (2017), Plews et al. (2013)
//! - TSB: Banister training load model

use serde::{Deserialize, Serialize};

/// Sleep and Recovery Configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SleepRecoveryConfig {
    /// Sleep duration thresholds and recommendations
    pub sleep_duration: SleepDurationConfig,
    /// Sleep stage distribution percentages
    pub sleep_stages: SleepStagesConfig,
    /// Sleep efficiency thresholds
    pub sleep_efficiency: SleepEfficiencyConfig,
    /// Heart rate variability (HRV) analysis settings
    pub hrv: HrvConfig,
    /// Training Stress Balance (TSB) thresholds
    pub training_stress_balance: TsbConfig,
    /// Recovery score calculation weights
    pub recovery_scoring: RecoveryScoringConfig,
}

/// Configuration for sleep duration thresholds and recommendations
///
/// Based on NSF/AASM guidelines (Watson et al. 2015, Hirshkowitz et al. 2015)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepDurationConfig {
    /// Minimum optimal sleep duration for adults (hours)
    pub adult_min_hours: f64,
    /// Maximum optimal sleep duration for adults (hours)
    pub adult_max_hours: f64,
    /// Optimal sleep duration for athletes (hours)
    pub athlete_optimal_hours: f64,
    /// Minimum optimal sleep for athletes (hours)
    pub athlete_min_hours: f64,
    /// Short sleep threshold (hours)
    pub short_sleep_threshold: f64,
    /// Very short sleep threshold (hours)
    pub very_short_sleep_threshold: f64,
}

/// Sleep stage distribution thresholds for optimal recovery
///
/// Based on AASM sleep stage guidelines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepStagesConfig {
    /// Minimum healthy deep sleep percentage
    pub deep_sleep_min_percent: f64,
    /// Optimal deep sleep percentage
    pub deep_sleep_optimal_percent: f64,
    /// Maximum healthy deep sleep percentage
    pub deep_sleep_max_percent: f64,
    /// Minimum healthy REM sleep percentage
    pub rem_sleep_min_percent: f64,
    /// Optimal REM sleep percentage
    pub rem_sleep_optimal_percent: f64,
    /// Maximum healthy REM sleep percentage
    pub rem_sleep_max_percent: f64,
    /// Minimum healthy light sleep percentage
    pub light_sleep_min_percent: f64,
    /// Maximum healthy light sleep percentage
    pub light_sleep_max_percent: f64,
    /// Healthy awake time threshold (percentage)
    pub awake_time_healthy_percent: f64,
    /// Acceptable awake time threshold (percentage)
    pub awake_time_acceptable_percent: f64,
}

/// Sleep efficiency quality thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepEfficiencyConfig {
    /// Excellent sleep efficiency threshold (percentage)
    pub excellent_threshold: f64,
    /// Good sleep efficiency threshold (percentage)
    pub good_threshold: f64,
    /// Poor sleep efficiency threshold (percentage)
    pub poor_threshold: f64,
}

/// Heart Rate Variability (HRV) analysis configuration
///
/// Based on Shaffer & Ginsberg (2017) and Plews et al. (2013)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HrvConfig {
    /// RMSSD decrease threshold indicating concern (ms, negative value)
    pub rmssd_decrease_concern_threshold: f64,
    /// RMSSD increase threshold indicating good recovery (ms)
    pub rmssd_increase_good_threshold: f64,
    /// Baseline deviation percentage indicating concern
    pub baseline_deviation_concern_percent: f64,
}

/// Training Stress Balance (TSB) thresholds for fatigue management
///
/// Based on Banister training load model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsbConfig {
    /// Highly fatigued TSB threshold
    pub highly_fatigued_tsb: f64,
    /// Fatigued TSB threshold
    pub fatigued_tsb: f64,
    /// Fresh TSB minimum (optimal range start)
    pub fresh_tsb_min: f64,
    /// Fresh TSB maximum (optimal range end)
    pub fresh_tsb_max: f64,
    /// Detraining TSB threshold (too much rest)
    pub detraining_tsb: f64,
}

/// Recovery score calculation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryScoringConfig {
    /// Excellent recovery threshold (score 0-100)
    pub excellent_threshold: f64,
    /// Good recovery threshold (score 0-100)
    pub good_threshold: f64,
    /// Fair recovery threshold (score 0-100)
    pub fair_threshold: f64,
    /// TSB weight when all components available
    pub tsb_weight_full: f64,
    /// Sleep weight when all components available
    pub sleep_weight_full: f64,
    /// HRV weight when all components available
    pub hrv_weight_full: f64,
    /// TSB weight when HRV not available
    pub tsb_weight_no_hrv: f64,
    /// Sleep weight when HRV not available
    pub sleep_weight_no_hrv: f64,
}

impl Default for SleepDurationConfig {
    fn default() -> Self {
        Self {
            adult_min_hours: 7.0,
            adult_max_hours: 9.0,
            athlete_optimal_hours: 8.0,
            athlete_min_hours: 7.5,
            short_sleep_threshold: 6.0,
            very_short_sleep_threshold: 5.0,
        }
    }
}

impl Default for SleepStagesConfig {
    fn default() -> Self {
        Self {
            deep_sleep_min_percent: 15.0,
            deep_sleep_optimal_percent: 20.0,
            deep_sleep_max_percent: 25.0,
            rem_sleep_min_percent: 20.0,
            rem_sleep_optimal_percent: 25.0,
            rem_sleep_max_percent: 30.0,
            light_sleep_min_percent: 45.0,
            light_sleep_max_percent: 55.0,
            awake_time_healthy_percent: 5.0,
            awake_time_acceptable_percent: 10.0,
        }
    }
}

impl Default for SleepEfficiencyConfig {
    fn default() -> Self {
        Self {
            excellent_threshold: 90.0,
            good_threshold: 85.0,
            poor_threshold: 70.0,
        }
    }
}

impl Default for HrvConfig {
    fn default() -> Self {
        Self {
            rmssd_decrease_concern_threshold: -10.0, // -10ms indicates poor recovery
            rmssd_increase_good_threshold: 5.0,      // +5ms indicates good recovery
            baseline_deviation_concern_percent: 15.0, // >15% below baseline = concern
        }
    }
}

impl Default for TsbConfig {
    fn default() -> Self {
        Self {
            highly_fatigued_tsb: -15.0,
            fatigued_tsb: -10.0,
            fresh_tsb_min: 5.0,
            fresh_tsb_max: 15.0,
            detraining_tsb: 25.0,
        }
    }
}

impl Default for RecoveryScoringConfig {
    fn default() -> Self {
        Self {
            excellent_threshold: 85.0,
            good_threshold: 70.0,
            fair_threshold: 50.0,
            // When all components available: TSB 40%, Sleep 40%, HRV 20%
            tsb_weight_full: 0.4,
            sleep_weight_full: 0.4,
            hrv_weight_full: 0.2,
            // When HRV not available: TSB 50%, Sleep 50%
            tsb_weight_no_hrv: 0.5,
            sleep_weight_no_hrv: 0.5,
        }
    }
}
