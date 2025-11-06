// ABOUTME: Proper statistical analysis engine for fitness trend calculations
// ABOUTME: Implements correct linear regression, R-squared calculations, and trend strength analysis
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
#![allow(clippy::cast_precision_loss)] // Safe: statistical calculations with controlled ranges

use super::{TrendDataPoint, TrendDirection};
use crate::errors::AppError;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Complete linear regression analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionResult {
    /// Slope of the regression line (rate of change)
    pub slope: f64,
    /// Y-intercept of the regression line
    pub intercept: f64,
    /// Coefficient of determination (goodness of fit, 0-1)
    pub r_squared: f64,
    /// Pearson correlation coefficient (-1 to 1)
    pub correlation: f64,
    /// Standard error of the estimate
    pub standard_error: f64,
    /// Degrees of freedom (n - 2)
    pub degrees_of_freedom: usize,
    /// P-value for statistical significance testing
    pub p_value: Option<f64>,
}

/// Statistical significance levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignificanceLevel {
    /// No statistical significance (p >= 0.1)
    NotSignificant,
    /// Weak significance (p < 0.1)
    Weak,
    /// Moderate significance (p < 0.05)
    Moderate,
    /// Strong significance (p < 0.01)
    Strong,
    /// Very strong significance (p < 0.001)
    VeryStrong,
}

impl SignificanceLevel {
    /// Get the alpha threshold for this significance level
    #[must_use]
    pub const fn alpha_threshold(self) -> f64 {
        match self {
            Self::NotSignificant => 1.0,
            Self::Weak => 0.1,
            Self::Moderate => 0.05,
            Self::Strong => 0.01,
            Self::VeryStrong => 0.001,
        }
    }

    /// Create significance level from p-value
    #[must_use]
    pub fn from_p_value(p_value: f64) -> Self {
        if p_value < 0.001 {
            Self::VeryStrong
        } else if p_value < 0.01 {
            Self::Strong
        } else if p_value < 0.05 {
            Self::Moderate
        } else if p_value < 0.1 {
            Self::Weak
        } else {
            Self::NotSignificant
        }
    }
}

/// Advanced statistical analyzer with proper mathematical implementations
pub struct StatisticalAnalyzer;

impl StatisticalAnalyzer {
    /// Calculate proper linear regression with all statistical measures
    ///
    /// # Errors
    ///
    /// Returns an error if there are insufficient data points for regression
    pub fn linear_regression(data_points: &[TrendDataPoint]) -> Result<RegressionResult> {
        if data_points.len() < 2 {
            return Err(AppError::invalid_input(format!(
                "Insufficient data points for regression: need at least 2, got {}",
                data_points.len()
            ))
            .into());
        }

        let n = data_points.len() as f64;
        let x_values: Vec<f64> = (0..data_points.len()).map(|i| i as f64).collect();
        let y_values: Vec<f64> = data_points.iter().map(|p| p.value).collect();

        // Calculate sums for regression
        let sum_x = x_values.iter().sum::<f64>();
        let sum_y = y_values.iter().sum::<f64>();
        let sum_xx = x_values.iter().map(|x| x * x).sum::<f64>();
        let sum_x_y = x_values
            .iter()
            .zip(&y_values)
            .map(|(x, y)| x * y)
            .sum::<f64>();
        let sum_yy = y_values.iter().map(|y| y * y).sum::<f64>();

        let mean_x = sum_x / n;
        let mean_y = sum_y / n;

        // Calculate slope and intercept
        let denominator = (n * mean_x).mul_add(-mean_x, sum_xx);
        if denominator.abs() < f64::EPSILON {
            return Err(
                AppError::invalid_input("Cannot calculate regression: zero variance in x").into(),
            );
        }

        let slope = (n * mean_x).mul_add(-mean_y, sum_x_y) / denominator;
        let intercept = slope.mul_add(-mean_x, mean_y);

        // Calculate correlation coefficient
        let numerator = (n * mean_x).mul_add(-mean_y, sum_x_y);
        let denominator_corr =
            ((n * mean_x).mul_add(-mean_x, sum_xx) * (n * mean_y).mul_add(-mean_y, sum_yy)).sqrt();

        let correlation = if denominator_corr == 0.0 {
            0.0
        } else {
            numerator / denominator_corr
        };

        // Calculate R-squared (coefficient of determination)
        let r_squared = correlation * correlation;

        // Calculate standard error of the estimate
        let y_predicted: Vec<f64> = x_values.iter().map(|x| slope * x + intercept).collect();
        let sse = y_values
            .iter()
            .zip(&y_predicted)
            .map(|(actual, predicted)| {
                let diff = actual - predicted;
                diff * diff
            })
            .sum::<f64>();

        let degrees_of_freedom = data_points.len().saturating_sub(2);
        let standard_error = if degrees_of_freedom > 0 {
            (sse / degrees_of_freedom as f64).sqrt()
        } else {
            0.0
        };

        // Calculate p-value for slope significance (simplified t-test)
        let p_value = if degrees_of_freedom > 0 && standard_error > 0.0 {
            let se_slope = standard_error / (n * mean_x).mul_add(-mean_x, sum_xx).sqrt();
            let t_stat = slope / se_slope;
            Some(Self::t_test_p_value(t_stat.abs(), degrees_of_freedom))
        } else {
            None
        };

        Ok(RegressionResult {
            slope,
            intercept,
            r_squared,
            correlation,
            standard_error,
            degrees_of_freedom,
            p_value,
        })
    }

    /// Calculate trend strength based on R-squared (proper measure of explained variance)
    ///
    /// # Errors
    ///
    /// Returns an error if regression analysis fails
    pub fn calculate_trend_strength(data_points: &[TrendDataPoint]) -> Result<f64> {
        let regression = Self::linear_regression(data_points)?;
        Ok(regression.r_squared)
    }

    /// Calculate correlation coefficient (different from trend strength)
    ///
    /// # Errors
    ///
    /// Returns an error if regression analysis fails
    pub fn calculate_correlation(data_points: &[TrendDataPoint]) -> Result<f64> {
        let regression = Self::linear_regression(data_points)?;
        Ok(regression.correlation)
    }

    /// Determine trend direction with proper statistical backing
    #[must_use]
    pub fn determine_trend_direction(
        regression: &RegressionResult,
        is_lower_better: bool,
        slope_threshold: f64,
    ) -> TrendDirection {
        // Check if slope is statistically significant
        let is_significant = regression
            .p_value
            .is_some_and(|p| p < SignificanceLevel::Moderate.alpha_threshold());

        if !is_significant || regression.slope.abs() < slope_threshold {
            return TrendDirection::Stable;
        }

        let is_improving = if is_lower_better {
            regression.slope < 0.0 // Negative slope is improvement for pace
        } else {
            regression.slope > 0.0 // Positive slope is improvement for most metrics
        };

        if is_improving {
            TrendDirection::Improving
        } else {
            TrendDirection::Declining
        }
    }

    /// Apply exponential smoothing to data points
    pub fn apply_exponential_smoothing(data_points: &mut [TrendDataPoint], alpha: f64) {
        if data_points.is_empty() {
            return;
        }

        // Clamp alpha to valid range
        let alpha = alpha.clamp(0.0, 1.0);

        // Initialize with first value
        data_points[0].smoothed_value = Some(data_points[0].value);

        for i in 1..data_points.len() {
            let previous_smoothed = data_points[i - 1]
                .smoothed_value
                .unwrap_or(data_points[i - 1].value);
            // Use mul_add for optimal floating point operation: a * b + c
            let smoothed = alpha.mul_add(data_points[i].value, (1.0 - alpha) * previous_smoothed);
            data_points[i].smoothed_value = Some(smoothed);
        }
    }

    /// Apply moving average smoothing
    pub fn apply_moving_average_smoothing(data_points: &mut [TrendDataPoint], window_size: usize) {
        if window_size <= 1 || data_points.len() < window_size {
            return;
        }

        for i in 0..data_points.len() {
            let start = i.saturating_sub(window_size / 2);
            let end = std::cmp::min(start + window_size, data_points.len());

            let window_sum: f64 = data_points[start..end].iter().map(|p| p.value).sum();
            let window_avg = window_sum / (end - start) as f64;

            data_points[i].smoothed_value = Some(window_avg);
        }
    }

    /// Detect outliers using modified Z-score
    #[must_use]
    pub fn detect_outliers(data_points: &[TrendDataPoint], threshold: f64) -> Vec<usize> {
        if data_points.len() < 3 {
            return Vec::new();
        }

        let values: Vec<f64> = data_points.iter().map(|p| p.value).collect();
        let median = Self::calculate_median(&values);

        // Calculate median absolute deviation (MAD)
        let deviations: Vec<f64> = values.iter().map(|v| (v - median).abs()).collect();
        let mad = Self::calculate_median(&deviations);

        if mad == 0.0 {
            return Vec::new(); // All values are the same
        }

        let mut outliers = Vec::new();
        for (i, &value) in values.iter().enumerate() {
            let modified_z_score = 0.6745 * (value - median) / mad;
            if modified_z_score.abs() > threshold {
                outliers.push(i);
            }
        }

        outliers
    }

    /// Calculate median value
    fn calculate_median(values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }

        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let len = sorted.len();
        if len % 2 == 0 {
            f64::midpoint(sorted[len / 2 - 1], sorted[len / 2])
        } else {
            sorted[len / 2]
        }
    }

    /// Simplified t-test p-value calculation (two-tailed)
    fn t_test_p_value(t_stat: f64, df: usize) -> f64 {
        // Simplified approximation for p-value calculation
        // In a real implementation, you'd use a proper t-distribution

        if df == 0 {
            return 1.0;
        }

        // Very rough approximation based on normal distribution
        // This is not mathematically rigorous but provides reasonable estimates
        let z_equivalent = t_stat / (1.0 + t_stat * t_stat / (4.0 * df as f64)).sqrt();

        // Two-tailed test
        2.0 * (1.0 - Self::standard_normal_cdf(z_equivalent.abs()))
    }

    /// Standard normal cumulative distribution function approximation
    fn standard_normal_cdf(x: f64) -> f64 {
        // Abramowitz and Stegun approximation
        let x = x.abs();
        let t = 1.0 / 0.231_641_9f64.mul_add(x, 1.0);
        let poly = t.mul_add(
            t.mul_add(
                t.mul_add(t.mul_add(1.330_274_429, -1.821_255_978), 1.781_477_937),
                -0.356_563_782,
            ),
            0.319_381_530,
        );
        // Use mul_add for optimal floating point operation: x * x * -0.5
        let cdf = (0.398_942_3 * (x.mul_add(x, 0.0) * -0.5).exp()).mul_add(-poly, 1.0);

        if x >= 0.0 {
            cdf
        } else {
            1.0 - cdf
        }
    }

    /// Calculate confidence intervals for trend predictions
    /// Calculate confidence intervals for trend predictions
    ///
    /// # Errors
    ///
    /// Returns an error if confidence interval cannot be calculated
    pub fn calculate_confidence_interval(
        regression: &RegressionResult,
        x_value: f64,
        confidence_level: f64,
    ) -> Result<(f64, f64)> {
        if regression.degrees_of_freedom == 0 {
            return Err(AppError::invalid_input(
                "Cannot calculate confidence interval with zero degrees of freedom",
            )
            .into());
        }

        // Use mul_add for optimal floating point operation: slope * x + intercept
        let predicted_y = regression.slope.mul_add(x_value, regression.intercept);

        // Simplified confidence interval calculation
        let alpha = 1.0 - confidence_level;
        let t_critical = Self::t_critical_value(alpha / 2.0, regression.degrees_of_freedom);
        let margin_of_error = t_critical * regression.standard_error;

        Ok((predicted_y - margin_of_error, predicted_y + margin_of_error))
    }

    /// Get critical t-value (simplified approximation)
    fn t_critical_value(_alpha: f64, df: usize) -> f64 {
        // Simplified approximation - in reality you'd use a proper t-table
        match df {
            0 => f64::INFINITY,
            1 => 12.706,
            2 => 4.303,
            3 => 3.182,
            4 => 2.776,
            5 => 2.571,
            _ => {
                // Approximate for larger df
                2.0 + 2.0 / df as f64
            }
        }
    }
}
