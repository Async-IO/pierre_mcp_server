// ABOUTME: Sleep tracking models for recovery analysis
// ABOUTME: SleepSession, SleepStage, and SleepStageType definitions
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Sleep stage data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepStage {
    /// Stage type (awake, light, deep, rem)
    pub stage_type: SleepStageType,
    /// Start time of this stage
    pub start_time: DateTime<Utc>,
    /// Duration of this stage in minutes
    pub duration_minutes: u32,
}

/// Types of sleep stages
#[non_exhaustive]
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SleepStageType {
    /// Awake stage - user is conscious and alert
    Awake,
    /// Light sleep stage - easy to wake from, body relaxing
    Light,
    /// Deep sleep stage - restorative, hard to wake from
    Deep,
    /// REM (Rapid Eye Movement) sleep stage - dreaming, memory consolidation
    Rem,
}

/// Sleep session data for recovery analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepSession {
    /// Unique identifier for the sleep session
    pub id: String,
    /// When sleep started
    pub start_time: DateTime<Utc>,
    /// When sleep ended
    pub end_time: DateTime<Utc>,
    /// Total time spent in bed (minutes)
    pub time_in_bed: u32,
    /// Actual sleep time (minutes)
    pub total_sleep_time: u32,
    /// Sleep efficiency percentage (sleep time / time in bed)
    pub sleep_efficiency: f32,
    /// Sleep quality score (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sleep_score: Option<f32>,
    /// Sleep stages breakdown
    pub stages: Vec<SleepStage>,
    /// Heart rate variability during sleep
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hrv_during_sleep: Option<f64>,
    /// Average respiratory rate during sleep
    #[serde(skip_serializing_if = "Option::is_none")]
    pub respiratory_rate: Option<f32>,
    /// Temperature variation during sleep
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature_variation: Option<f32>,
    /// Number of times awakened
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wake_count: Option<u32>,
    /// Time to fall asleep (minutes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sleep_onset_latency: Option<u32>,
    /// Provider of this sleep data
    pub provider: String,
}

impl SleepSession {
    /// Calculate sleep stages summary
    #[must_use]
    pub fn stage_summary(&self) -> HashMap<SleepStageType, u32> {
        let mut summary = HashMap::new();
        for stage in &self.stages {
            *summary.entry(stage.stage_type).or_insert(0) += stage.duration_minutes;
        }
        summary
    }

    /// Get deep sleep percentage
    #[must_use]
    pub fn deep_sleep_percentage(&self) -> f32 {
        let deep_sleep_total = self
            .stages
            .iter()
            .filter(|s| matches!(s.stage_type, SleepStageType::Deep))
            .map(|s| s.duration_minutes)
            .sum::<u32>();
        let deep_sleep_minutes =
            f32::from(u16::try_from(deep_sleep_total.min(u32::from(u16::MAX))).unwrap_or(u16::MAX));

        if self.total_sleep_time > 0 {
            let total_sleep_f32 = f32::from(
                u16::try_from(self.total_sleep_time.min(u32::from(u16::MAX))).unwrap_or(u16::MAX),
            );
            (deep_sleep_minutes / total_sleep_f32) * 100.0
        } else {
            0.0
        }
    }

    /// Get REM sleep percentage
    #[must_use]
    pub fn rem_sleep_percentage(&self) -> f32 {
        let rem_sleep_total = self
            .stages
            .iter()
            .filter(|s| matches!(s.stage_type, SleepStageType::Rem))
            .map(|s| s.duration_minutes)
            .sum::<u32>();
        let rem_sleep_minutes =
            f32::from(u16::try_from(rem_sleep_total.min(u32::from(u16::MAX))).unwrap_or(u16::MAX));

        if self.total_sleep_time > 0 {
            let total_sleep_f32 = f32::from(
                u16::try_from(self.total_sleep_time.min(u32::from(u16::MAX))).unwrap_or(u16::MAX),
            );
            (rem_sleep_minutes / total_sleep_f32) * 100.0
        } else {
            0.0
        }
    }
}
