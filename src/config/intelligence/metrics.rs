// ABOUTME: Metrics configuration for fitness data calculation and validation
// ABOUTME: Configures smoothing, outlier detection, validation ranges, and aggregation methods
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Metrics Configuration
//!
//! Provides configuration for metrics calculation, validation, and aggregation
//! including smoothing parameters and outlier detection thresholds.

use serde::{Deserialize, Serialize};

/// Metrics Configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Metrics calculation settings
    pub calculation: MetricsCalculationConfig,
    /// Metrics validation settings
    pub validation: MetricsValidationConfig,
    /// Metrics aggregation settings
    pub aggregation: MetricsAggregationConfig,
}

/// Configuration for metrics calculation algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsCalculationConfig {
    /// Window size for data smoothing
    pub smoothing_window_size: usize,
    /// Z-score threshold for outlier detection
    pub outlier_detection_threshold: f64,
    /// Whether to interpolate missing data points
    pub missing_data_interpolation: bool,
}

/// Validation rules for metrics data quality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsValidationConfig {
    /// Maximum valid heart rate (BPM)
    pub max_heart_rate: u32,
    /// Minimum valid heart rate (BPM)
    pub min_heart_rate: u32,
    /// Maximum valid pace (min/km)
    pub max_pace_min_per_km: f64,
    /// Minimum valid pace (min/km)
    pub min_pace_min_per_km: f64,
}

/// Configuration for aggregating metrics over time periods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsAggregationConfig {
    /// Method for weekly aggregation (mean, median, sum)
    pub weekly_aggregation_method: String,
    /// Method for monthly aggregation (mean, median, sum)
    pub monthly_aggregation_method: String,
    /// Method for trend calculation (linear, exponential)
    pub trend_calculation_method: String,
}

impl Default for MetricsCalculationConfig {
    fn default() -> Self {
        Self {
            smoothing_window_size: 7,
            outlier_detection_threshold: 2.5,
            missing_data_interpolation: true,
        }
    }
}

impl Default for MetricsValidationConfig {
    fn default() -> Self {
        Self {
            max_heart_rate: 220,
            min_heart_rate: 40,
            max_pace_min_per_km: 20.0,
            min_pace_min_per_km: 2.0,
        }
    }
}

impl Default for MetricsAggregationConfig {
    fn default() -> Self {
        Self {
            weekly_aggregation_method: "average".into(),
            monthly_aggregation_method: "weighted_average".into(),
            trend_calculation_method: "linear_regression".into(),
        }
    }
}
