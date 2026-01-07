// ABOUTME: Recommendation engine configuration for training suggestions
// ABOUTME: Configures thresholds, weights, limits, and message templates for recommendations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Recommendation Engine Configuration
//!
//! Provides configuration for the workout recommendation system including
//! thresholds for triggering recommendations, weights for scoring factors,
//! and message templates.

use crate::constants::limits;
use serde::{Deserialize, Serialize};

/// Recommendation Engine Configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecommendationEngineConfig {
    /// Threshold values for triggering recommendations
    pub thresholds: RecommendationThresholds,
    /// Weights for scoring different recommendation factors
    pub weights: RecommendationWeights,
    /// Limits on recommendation generation
    pub limits: RecommendationLimits,
    /// Template messages for recommendations
    pub messages: RecommendationMessages,
}

/// Thresholds for triggering training recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationThresholds {
    /// Minimum weekly distance (km) to trigger low distance warning
    pub low_weekly_distance_km: f64,
    /// Maximum weekly distance (km) to trigger high distance warning
    pub high_weekly_distance_km: f64,
    /// Minimum weekly training frequency to trigger warning
    pub low_weekly_frequency: i32,
    /// Maximum weekly training frequency to trigger overtraining warning
    pub high_weekly_frequency: i32,
    /// Pace improvement percentage required for pace recommendation
    pub pace_improvement_threshold: f64,
    /// Consistency score threshold for consistency recommendations
    pub consistency_threshold: f64,
    /// Days without activity to trigger rest day recommendation
    pub rest_day_threshold: i32,
    /// Volume increase percentage to trigger warning
    pub volume_increase_threshold: f64,
    /// Intensity threshold for high intensity warnings
    pub intensity_threshold: f64,
}

/// Weights for different factors in recommendation scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationWeights {
    /// Weight for distance-based recommendations
    pub distance_weight: f64,
    /// Weight for frequency-based recommendations
    pub frequency_weight: f64,
    /// Weight for pace-based recommendations
    pub pace_weight: f64,
    /// Weight for consistency-based recommendations
    pub consistency_weight: f64,
    /// Weight for recovery-based recommendations
    pub recovery_weight: f64,
}

/// Limits on recommendation generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationLimits {
    /// Maximum recommendations per category to prevent overwhelming users
    pub max_recommendations_per_category: usize,
    /// Maximum total recommendations across all categories
    pub max_total_recommendations: usize,
    /// Minimum confidence score to include a recommendation
    pub min_confidence_threshold: f64,
}

/// Template messages for different recommendation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationMessages {
    /// Message template for low distance warnings
    pub low_distance: String,
    /// Message template for high distance warnings
    pub high_distance: String,
    /// Message template for low frequency warnings
    pub low_frequency: String,
    /// Message template for high frequency warnings
    pub high_frequency: String,
    /// Message template for pace improvement recommendations
    pub pace_improvement: String,
    /// Message template for consistency improvement recommendations
    pub consistency_improvement: String,
    /// Message template for recovery recommendations
    pub recovery_needed: String,
}

impl Default for RecommendationThresholds {
    fn default() -> Self {
        Self {
            low_weekly_distance_km: 20.0,
            high_weekly_distance_km: 80.0,
            low_weekly_frequency: 2,
            high_weekly_frequency: 6,
            pace_improvement_threshold: 0.05,
            consistency_threshold: limits::DEFAULT_CONFIDENCE_THRESHOLD,
            rest_day_threshold: 1,
            volume_increase_threshold: 0.1,
            intensity_threshold: 0.8,
        }
    }
}

impl Default for RecommendationWeights {
    fn default() -> Self {
        Self {
            distance_weight: 0.3,
            frequency_weight: 0.25,
            pace_weight: 0.2,
            consistency_weight: 0.15,
            recovery_weight: 0.1,
        }
    }
}

impl Default for RecommendationLimits {
    fn default() -> Self {
        Self {
            max_recommendations_per_category: 3,
            max_total_recommendations: 10,
            min_confidence_threshold: 0.6,
        }
    }
}

impl Default for RecommendationMessages {
    fn default() -> Self {
        Self {
            low_distance: "Consider gradually increasing your weekly distance".into(),
            high_distance: "You're covering good distance - focus on quality".into(),
            low_frequency: "Try to add one more training session per week".into(),
            high_frequency: "You're training frequently - ensure adequate recovery".to_owned(),
            pace_improvement: "Focus on tempo runs to improve your pace".into(),
            consistency_improvement: "Try to maintain a more consistent training schedule"
                .to_owned(),
            recovery_needed: "Consider adding more recovery time between sessions".to_owned(),
        }
    }
}
