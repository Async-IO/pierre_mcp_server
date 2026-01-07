// ABOUTME: Activity analyzer configuration for fitness activity classification
// ABOUTME: Configures heart rate zones, power zones, scoring weights, and insight generation
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Activity Analyzer Configuration
//!
//! Provides configuration for activity analysis including zone definitions,
//! scoring weights, and insight generation thresholds.

use crate::constants::limits;
use serde::{Deserialize, Serialize};

/// Activity Analyzer Configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActivityAnalyzerConfig {
    /// Activity analysis settings
    pub analysis: ActivityAnalysisConfig,
    /// Activity scoring settings
    pub scoring: ActivityScoringConfig,
    /// Activity insights generation settings
    pub insights: ActivityInsightsConfig,
}

/// Configuration for activity analysis algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityAnalysisConfig {
    /// Minimum activity duration to analyze (seconds)
    pub min_duration_seconds: u64,
    /// Maximum reasonable pace in min/km
    pub max_reasonable_pace: f64,
    /// Heart rate zone definitions
    pub heart_rate_zones: HeartRateZonesConfig,
    /// Power zone definitions
    pub power_zones: PowerZonesConfig,
}

/// Heart rate zone percentage thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartRateZonesConfig {
    /// Maximum percentage of max HR for zone 1
    pub zone1_max_percentage: f32,
    /// Maximum percentage of max HR for zone 2
    pub zone2_max_percentage: f32,
    /// Maximum percentage of max HR for zone 3
    pub zone3_max_percentage: f32,
    /// Maximum percentage of max HR for zone 4
    pub zone4_max_percentage: f32,
    /// Maximum percentage of max HR for zone 5
    pub zone5_max_percentage: f32,
}

/// Power zone percentage thresholds for cyclists
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerZonesConfig {
    /// Maximum percentage of FTP for zone 1
    pub zone1_max_percentage: f32,
    /// Maximum percentage of FTP for zone 2
    pub zone2_max_percentage: f32,
    /// Maximum percentage of FTP for zone 3
    pub zone3_max_percentage: f32,
    /// Maximum percentage of FTP for zone 4
    pub zone4_max_percentage: f32,
    /// Maximum percentage of FTP for zone 5
    pub zone5_max_percentage: f32,
}

/// Weights for activity quality scoring components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityScoringConfig {
    /// Weight for efficiency score (0.0-1.0)
    pub efficiency_weight: f64,
    /// Weight for intensity score (0.0-1.0)
    pub intensity_weight: f64,
    /// Weight for duration score (0.0-1.0)
    pub duration_weight: f64,
    /// Weight for consistency score (0.0-1.0)
    pub consistency_weight: f64,
}

/// Configuration for generating activity insights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityInsightsConfig {
    /// Minimum confidence to include an insight (0-100)
    pub min_confidence_threshold: f64,
    /// Maximum number of insights per activity
    pub max_insights_per_activity: usize,
    /// Severity threshold configuration
    pub severity_thresholds: SeverityThresholds,
}

/// Thresholds for insight severity classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeverityThresholds {
    /// Threshold for info-level insights (0-100)
    pub info_threshold: f64,
    /// Threshold for warning-level insights (0-100)
    pub warning_threshold: f64,
    /// Threshold for critical-level insights (0-100)
    pub critical_threshold: f64,
}

impl Default for ActivityAnalysisConfig {
    fn default() -> Self {
        Self {
            min_duration_seconds: 300, // 5 minutes
            max_reasonable_pace: 15.0, // 15 min/km
            heart_rate_zones: HeartRateZonesConfig::default(),
            power_zones: PowerZonesConfig::default(),
        }
    }
}

impl Default for HeartRateZonesConfig {
    fn default() -> Self {
        Self {
            zone1_max_percentage: 60.0,
            zone2_max_percentage: 70.0,
            zone3_max_percentage: 80.0,
            zone4_max_percentage: 90.0,
            zone5_max_percentage: 100.0,
        }
    }
}

impl Default for PowerZonesConfig {
    fn default() -> Self {
        Self {
            zone1_max_percentage: 55.0,
            zone2_max_percentage: 75.0,
            zone3_max_percentage: 90.0,
            zone4_max_percentage: 105.0,
            zone5_max_percentage: 150.0,
        }
    }
}

impl Default for ActivityScoringConfig {
    fn default() -> Self {
        Self {
            efficiency_weight: 0.3,
            intensity_weight: 0.3,
            duration_weight: 0.2,
            consistency_weight: 0.2,
        }
    }
}

impl Default for ActivityInsightsConfig {
    fn default() -> Self {
        Self {
            min_confidence_threshold: limits::DEFAULT_CONFIDENCE_THRESHOLD,
            max_insights_per_activity: 5,
            severity_thresholds: SeverityThresholds::default(),
        }
    }
}

impl Default for SeverityThresholds {
    fn default() -> Self {
        Self {
            info_threshold: 0.3,
            warning_threshold: limits::DEFAULT_CONFIDENCE_THRESHOLD,
            critical_threshold: 0.9,
        }
    }
}
