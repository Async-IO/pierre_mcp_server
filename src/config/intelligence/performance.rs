// ABOUTME: Performance analyzer configuration for trend and statistical analysis
// ABOUTME: Configures thresholds for detecting improvements, declines, and statistical significance
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Performance Analyzer Configuration
//!
//! Provides configuration for performance analysis algorithms including
//! trend detection, statistical analysis, and performance thresholds.

use serde::{Deserialize, Serialize};

/// Performance Analyzer Configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceAnalyzerConfig {
    /// Trend analysis algorithm configuration
    pub trend_analysis: TrendAnalysisConfig,
    /// Statistical analysis configuration
    pub statistical: StatisticalConfig,
    /// Performance threshold values
    pub thresholds: PerformanceThresholds,
}

/// Configuration for trend analysis algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysisConfig {
    /// Minimum number of data points required for trend analysis
    pub min_data_points: usize,
    /// Threshold for determining trend strength
    pub trend_strength_threshold: f64,
    /// Statistical significance threshold
    pub significance_threshold: f64,
    /// Threshold for detecting performance improvement
    pub improvement_threshold: f64,
    /// Threshold for detecting performance decline
    pub decline_threshold: f64,
}

/// Configuration for statistical analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalConfig {
    /// Confidence level for statistical tests (e.g., 0.95 for 95%)
    pub confidence_level: f64,
    /// Threshold for identifying outliers
    pub outlier_threshold: f64,
    /// Smoothing factor for moving averages
    pub smoothing_factor: f64,
}

/// Performance thresholds for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThresholds {
    /// Percentage change considered significant improvement
    pub significant_improvement: f64,
    /// Percentage change considered significant decline
    pub significant_decline: f64,
    /// Acceptable variance in pace
    pub pace_variance_threshold: f64,
    /// Threshold for endurance assessment
    pub endurance_threshold: f64,
}

impl Default for TrendAnalysisConfig {
    fn default() -> Self {
        Self {
            min_data_points: 5,
            trend_strength_threshold: 0.3,
            significance_threshold: 0.05,
            improvement_threshold: 0.02,
            decline_threshold: -0.02,
        }
    }
}

impl Default for StatisticalConfig {
    fn default() -> Self {
        Self {
            confidence_level: 0.95,
            outlier_threshold: 2.0,
            smoothing_factor: 0.3,
        }
    }
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            significant_improvement: 0.05,
            significant_decline: -0.05,
            pace_variance_threshold: 0.2,
            endurance_threshold: 0.8,
        }
    }
}
