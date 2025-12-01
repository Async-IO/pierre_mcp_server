// ABOUTME: Type-safe metric extraction system for fitness data analysis
// ABOUTME: Provides unified interface for extracting specific metrics from activities with proper error handling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
#![allow(clippy::cast_precision_loss)] // Safe: fitness data conversions

use crate::errors::{AppError, AppResult};
use crate::models::Activity;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Type-safe metric enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricType {
    /// Pace metric (min/km, lower is better)
    Pace,
    /// Speed metric (m/s)
    Speed,
    /// Heart rate metric (bpm)
    HeartRate,
    /// Distance metric (meters)
    Distance,
    /// Duration metric (seconds)
    Duration,
    /// Elevation gain metric (meters)
    Elevation,
    /// Power output metric (watts)
    Power,
}

impl MetricType {
    /// Extract metric value from an activity
    pub fn extract_value(self, activity: &Activity) -> Option<f64> {
        match self {
            Self::Pace | Self::Speed => activity.average_speed,
            Self::HeartRate => activity.average_heart_rate.map(f64::from),
            Self::Distance => activity.distance_meters,
            Self::Duration => Some(activity.duration_seconds as f64), // Safe: fitness duration fits in f64
            Self::Elevation => activity.elevation_gain,
            Self::Power => activity.average_power.map(f64::from),
        }
    }

    /// Check if lower values are better for this metric (e.g., pace)
    #[must_use]
    pub const fn is_lower_better(self) -> bool {
        matches!(self, Self::Pace)
    }

    /// Get the unit string for this metric
    #[must_use]
    pub const fn unit(self) -> &'static str {
        match self {
            Self::Pace => "min/km",
            Self::Speed => "m/s",
            Self::HeartRate => "bpm",
            Self::Distance | Self::Elevation => "meters",
            Self::Duration => "seconds",
            Self::Power => "watts",
        }
    }

    /// Get display name for this metric
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Pace => "Pace",
            Self::Speed => "Speed",
            Self::HeartRate => "Heart Rate",
            Self::Distance => "Distance",
            Self::Duration => "Duration",
            Self::Elevation => "Elevation",
            Self::Power => "Power",
        }
    }
}

/// Safe metric extractor with proper error handling
pub struct SafeMetricExtractor;

impl SafeMetricExtractor {
    /// Extract metric values from activities with timestamps
    ///
    /// # Errors
    ///
    /// Returns an error if no valid metric values are found
    pub fn extract_metric_values(
        activities: &[Activity],
        metric_type: MetricType,
    ) -> AppResult<Vec<(DateTime<Utc>, f64)>> {
        let values: Vec<_> = activities
            .iter()
            .filter_map(|activity| {
                metric_type
                    .extract_value(activity)
                    .map(|value| (activity.start_date, value))
            })
            .collect();

        if values.is_empty() {
            return Err(AppError::not_found(format!(
                "No valid {} values found in {} activities",
                metric_type.display_name(),
                activities.len()
            )));
        }

        Ok(values)
    }

    /// Extract metric values for a specific time period
    ///
    /// # Errors
    ///
    /// Returns an error if no valid metric values are found in the time period
    pub fn extract_metric_values_in_period(
        activities: &[Activity],
        metric_type: MetricType,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> AppResult<Vec<(DateTime<Utc>, f64)>> {
        let filtered_activities: Vec<_> = activities
            .iter()
            .filter(|activity| activity.start_date >= start_date && activity.start_date <= end_date)
            .cloned()
            .collect();

        Self::extract_metric_values(&filtered_activities, metric_type)
    }

    /// Get summary statistics for a metric
    ///
    /// # Errors
    ///
    /// Returns an error if no valid metric values are found in the activities
    pub fn calculate_metric_summary(
        activities: &[Activity],
        metric_type: MetricType,
    ) -> AppResult<MetricSummary> {
        let values = Self::extract_metric_values(activities, metric_type)
            .map_err(|e| AppError::internal(format!("Metric extraction failed: {e}")))?;
        let metric_values: Vec<f64> = values.into_iter().map(|(_, value)| value).collect();

        if metric_values.is_empty() {
            return Err(AppError::not_found("No values to summarize"));
        }

        let count = metric_values.len();
        let sum = metric_values.iter().sum::<f64>();
        let mean = sum / count as f64;

        let min = metric_values
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .copied()
            .unwrap_or(0.0);

        let max = metric_values
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .copied()
            .unwrap_or(0.0);

        // Calculate standard deviation
        let variance = metric_values
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / count as f64;
        let std_dev = variance.sqrt();

        // Calculate median
        let mut sorted_values = metric_values;
        sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = if count.is_multiple_of(2) {
            let left = sorted_values[count / 2 - 1];
            let right = sorted_values[count / 2];
            left + (right - left) / 2.0
        } else {
            sorted_values[count / 2]
        };

        Ok(MetricSummary {
            metric_type,
            count,
            mean,
            median,
            std_dev,
            min,
            max,
        })
    }
}

/// Summary statistics for a metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSummary {
    /// Type of metric being summarized
    pub metric_type: MetricType,
    /// Number of data points
    pub count: usize,
    /// Mean (average) value
    pub mean: f64,
    /// Median value
    pub median: f64,
    /// Standard deviation
    pub std_dev: f64,
    /// Minimum value
    pub min: f64,
    /// Maximum value
    pub max: f64,
}

impl MetricSummary {
    /// Get coefficient of variation (`std_dev` / mean)
    #[must_use]
    pub fn coefficient_of_variation(&self) -> f64 {
        if self.mean == 0.0 {
            0.0
        } else {
            self.std_dev / self.mean
        }
    }

    /// Check if the metric shows high variability (CV > 0.2)
    #[must_use]
    pub fn is_highly_variable(&self) -> bool {
        self.coefficient_of_variation() > 0.2
    }
}
