// ABOUTME: Health and recovery metrics models for wellness tracking
// ABOUTME: RecoveryMetrics and HealthMetrics for comprehensive health data
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Daily recovery and readiness metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryMetrics {
    /// Date for these recovery metrics
    pub date: DateTime<Utc>,
    /// Overall recovery score (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_score: Option<f32>,
    /// Readiness score for training (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readiness_score: Option<f32>,
    /// HRV status or trend
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hrv_status: Option<String>,
    /// Sleep contribution to recovery (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sleep_score: Option<f32>,
    /// Stress level indicator (0-100, higher = more stress)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stress_level: Option<f32>,
    /// Current training load
    #[serde(skip_serializing_if = "Option::is_none")]
    pub training_load: Option<f32>,
    /// Resting heart rate for the day
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resting_heart_rate: Option<u32>,
    /// Body temperature deviation from baseline
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_temperature: Option<f32>,
    /// Respiratory rate while resting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resting_respiratory_rate: Option<f32>,
    /// Provider of this recovery data
    pub provider: String,
}

impl RecoveryMetrics {
    /// Check if recovery metrics indicate good readiness for training
    #[must_use]
    pub fn is_ready_for_training(&self) -> bool {
        // Consider ready if recovery score > 70 and readiness score > 70
        match (self.recovery_score, self.readiness_score) {
            (Some(recovery), Some(readiness)) => recovery > 70.0 && readiness > 70.0,
            (Some(recovery), None) => recovery > 70.0,
            (None, Some(readiness)) => readiness > 70.0,
            (None, None) => false,
        }
    }

    /// Get overall wellness score combining all available metrics
    #[must_use]
    pub fn wellness_score(&self) -> Option<f32> {
        let mut total_score = 0.0;
        let mut factor_count = 0;

        if let Some(recovery) = self.recovery_score {
            total_score += recovery;
            factor_count += 1;
        }

        if let Some(sleep) = self.sleep_score {
            total_score += sleep;
            factor_count += 1;
        }

        // Invert stress level (lower stress = better wellness)
        if let Some(stress) = self.stress_level {
            total_score += 100.0 - stress;
            factor_count += 1;
        }

        if factor_count > 0 {
            Some(total_score / f32::from(u8::try_from(factor_count).unwrap_or(u8::MAX)))
        } else {
            None
        }
    }
}

/// Health metrics for comprehensive wellness tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    /// Date for these health metrics
    pub date: DateTime<Utc>,
    /// Weight in kilograms
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    /// Body fat percentage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_fat_percentage: Option<f32>,
    /// Muscle mass in kilograms
    #[serde(skip_serializing_if = "Option::is_none")]
    pub muscle_mass: Option<f64>,
    /// Bone mass in kilograms
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bone_mass: Option<f64>,
    /// Body water percentage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_water_percentage: Option<f32>,
    /// Basal metabolic rate (calories/day)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bmr: Option<u32>,
    /// Blood pressure (systolic, diastolic)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blood_pressure: Option<(u32, u32)>,
    /// Blood glucose level (mg/dL)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blood_glucose: Option<f32>,
    /// VO2 max estimate (ml/kg/min)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vo2_max: Option<f32>,
    /// Provider of this health data
    pub provider: String,
}
