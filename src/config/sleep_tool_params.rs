// ABOUTME: Sleep tool operational parameters for activity fetching and trend analysis
// ABOUTME: Distinct from intelligence/sleep_recovery.rs which handles analytical thresholds
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::constants::sleep_recovery;
use serde::{Deserialize, Serialize};
use std::env;

/// Sleep tool operational parameters for activity fetching and trend analysis
///
/// This config controls operational parameters like activity limits and trend thresholds.
/// For sleep quality analytical thresholds (duration, stages, HRV, TSB), see
/// `config::intelligence::SleepRecoveryConfig`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepToolParamsConfig {
    /// Number of recent activities to fetch for analysis
    pub activity_limit: u32,
    /// Minimum days of sleep history required for trend analysis
    pub trend_min_days: usize,
    /// Sleep trend improving threshold (hours)
    pub trend_improving_threshold: f64,
    /// Sleep trend declining threshold (hours)
    pub trend_declining_threshold: f64,
    /// Additional sleep hours when fatigued
    pub fatigue_bonus_hours: f64,
    /// ATL threshold for high training load
    pub high_load_atl_threshold: f64,
    /// Additional sleep hours for high training load
    pub high_load_bonus_hours: f64,
    /// Wind-down buffer time before sleep (minutes)
    pub wind_down_minutes: i64,
    /// Minutes per day for time calculations
    pub minutes_per_day: i64,
}

impl Default for SleepToolParamsConfig {
    fn default() -> Self {
        Self {
            activity_limit: sleep_recovery::ACTIVITY_LIMIT,
            trend_min_days: sleep_recovery::TREND_MIN_DAYS,
            trend_improving_threshold: sleep_recovery::TREND_IMPROVING_THRESHOLD,
            trend_declining_threshold: sleep_recovery::TREND_DECLINING_THRESHOLD,
            fatigue_bonus_hours: sleep_recovery::FATIGUE_BONUS_HOURS,
            high_load_atl_threshold: sleep_recovery::HIGH_LOAD_ATL_THRESHOLD,
            high_load_bonus_hours: sleep_recovery::HIGH_LOAD_BONUS_HOURS,
            wind_down_minutes: sleep_recovery::WIND_DOWN_MINUTES,
            minutes_per_day: sleep_recovery::MINUTES_PER_DAY,
        }
    }
}

impl SleepToolParamsConfig {
    /// Load sleep recovery configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            activity_limit: env::var("SLEEP_RECOVERY_ACTIVITY_LIMIT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(sleep_recovery::ACTIVITY_LIMIT),
            trend_min_days: env::var("SLEEP_RECOVERY_TREND_MIN_DAYS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(sleep_recovery::TREND_MIN_DAYS),
            trend_improving_threshold: env::var("SLEEP_RECOVERY_TREND_IMPROVING_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(sleep_recovery::TREND_IMPROVING_THRESHOLD),
            trend_declining_threshold: env::var("SLEEP_RECOVERY_TREND_DECLINING_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(sleep_recovery::TREND_DECLINING_THRESHOLD),
            fatigue_bonus_hours: env::var("SLEEP_RECOVERY_FATIGUE_BONUS_HOURS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(sleep_recovery::FATIGUE_BONUS_HOURS),
            high_load_atl_threshold: env::var("SLEEP_RECOVERY_HIGH_LOAD_ATL_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(sleep_recovery::HIGH_LOAD_ATL_THRESHOLD),
            high_load_bonus_hours: env::var("SLEEP_RECOVERY_HIGH_LOAD_BONUS_HOURS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(sleep_recovery::HIGH_LOAD_BONUS_HOURS),
            wind_down_minutes: env::var("SLEEP_RECOVERY_WIND_DOWN_MINUTES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(sleep_recovery::WIND_DOWN_MINUTES),
            minutes_per_day: env::var("SLEEP_RECOVERY_MINUTES_PER_DAY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(sleep_recovery::MINUTES_PER_DAY),
        }
    }
}
