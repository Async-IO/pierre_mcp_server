// ABOUTME: Configuration-driven constants for intelligence analysis replacing magic numbers
// ABOUTME: Provides type-safe, environment-configurable parameters for all analysis algorithms

use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// Analysis configuration errors
#[derive(Debug, Error)]
pub enum AnalysisConfigError {
    #[error("Invalid timeframe: {0}")]
    InvalidTimeframe(String),

    #[error("Invalid threshold: {0}")]
    InvalidThreshold(String),

    #[error("Configuration validation failed: {0}")]
    ValidationFailed(String),
}

/// Time periods for various analysis windows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisTimeframes {
    /// Number of weeks to look back for fitness score calculation
    pub fitness_score_weeks: u32,

    /// Number of weeks to analyze for trend detection
    pub trend_analysis_weeks: u32,

    /// Number of weeks to consider for training load analysis
    pub training_load_weeks: u32,

    /// Number of weeks of history needed for performance prediction
    pub prediction_history_weeks: u32,

    /// Number of days without activity before flagging as a gap
    pub training_gap_days: i64,

    /// Maximum consecutive training days before recommending rest
    pub max_consecutive_training_days: i64,

    /// Number of days to analyze for recovery recommendations
    pub recovery_analysis_days: i64,
}

/// Confidence calculation thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceThresholds {
    /// Number of data points needed for high confidence
    pub high_data_points: usize,

    /// Number of data points needed for medium confidence
    pub medium_data_points: usize,

    /// R-squared threshold for high confidence trends
    pub high_r_squared: f64,

    /// R-squared threshold for medium confidence trends
    pub medium_r_squared: f64,

    /// Statistical significance threshold (p-value)
    pub significance_threshold: f64,

    /// Minimum correlation for meaningful trends
    pub min_correlation_threshold: f64,
}

/// Statistical analysis parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalConfig {
    /// Window size for moving average smoothing
    pub smoothing_window_size: usize,

    /// Alpha parameter for exponential smoothing (0.0 to 1.0)
    pub exponential_smoothing_alpha: f64,

    /// Z-score threshold for outlier detection
    pub outlier_z_score_threshold: f64,

    /// Minimum slope magnitude to consider a trend significant
    pub trend_slope_threshold: f64,

    /// Stability threshold - changes below this are considered stable
    pub stability_threshold: f64,
}

/// Performance analysis thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThresholds {
    /// Minimum weekly training volume (hours)
    pub min_weekly_volume_hours: f64,

    /// High weekly training volume threshold (hours)
    pub high_weekly_volume_hours: f64,

    /// Maximum safe weekly training load (seconds)
    pub max_weekly_load_seconds: u64,

    /// Maximum recommended high-intensity sessions per week
    pub max_high_intensity_sessions_per_week: usize,

    /// Heart rate threshold for high intensity (% of max)
    pub high_intensity_hr_percentage: f64,

    /// Heart rate threshold for recovery (% of max)
    pub recovery_hr_percentage: f64,

    /// Minimum duration for aerobic benefit (seconds)
    pub min_aerobic_duration_seconds: u64,
}

/// Fitness scoring parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessScoring {
    /// Weight for aerobic fitness component
    pub aerobic_weight: f64,

    /// Weight for strength/power component
    pub strength_weight: f64,

    /// Weight for consistency component
    pub consistency_weight: f64,

    /// Target weekly activity frequency
    pub target_weekly_activities: f64,

    /// Threshold for considering fitness improving
    pub fitness_improving_threshold: f64,

    /// Threshold for considering fitness stable
    pub fitness_stable_threshold: f64,

    /// Divisor for strength endurance calculation
    pub strength_endurance_divisor: f64,
}

/// Main analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub timeframes: AnalysisTimeframes,
    pub confidence: ConfidenceThresholds,
    pub statistical: StatisticalConfig,
    pub performance: PerformanceThresholds,
    pub fitness_scoring: FitnessScoring,
    pub min_activities_for_prediction: usize,
    pub max_prediction_days: i64,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            timeframes: AnalysisTimeframes {
                fitness_score_weeks: 6,
                trend_analysis_weeks: 12,
                training_load_weeks: 4,
                prediction_history_weeks: 8,
                training_gap_days: 7,
                max_consecutive_training_days: 6,
                recovery_analysis_days: 14,
            },
            confidence: ConfidenceThresholds {
                high_data_points: 20,
                medium_data_points: 10,
                high_r_squared: 0.7,
                medium_r_squared: 0.4,
                significance_threshold: 0.05,
                min_correlation_threshold: 0.3,
            },
            statistical: StatisticalConfig {
                smoothing_window_size: 3,
                exponential_smoothing_alpha: 0.3,
                outlier_z_score_threshold: 2.5,
                trend_slope_threshold: 0.01,
                stability_threshold: 0.05,
            },
            performance: PerformanceThresholds {
                min_weekly_volume_hours: 2.0,
                high_weekly_volume_hours: 12.0,
                max_weekly_load_seconds: 18000, // 5 hours
                max_high_intensity_sessions_per_week: 3,
                high_intensity_hr_percentage: 0.85,
                recovery_hr_percentage: 0.65,
                min_aerobic_duration_seconds: 1200, // 20 minutes
            },
            fitness_scoring: FitnessScoring {
                aerobic_weight: 0.5,
                strength_weight: 0.3,
                consistency_weight: 0.2,
                target_weekly_activities: 4.0,
                fitness_improving_threshold: 75.0,
                fitness_stable_threshold: 60.0,
                strength_endurance_divisor: 10.0,
            },
            min_activities_for_prediction: 5,
            max_prediction_days: 365,
        }
    }
}

impl AnalysisConfig {
    /// Load configuration from environment variables with fallback to defaults
    ///
    /// # Errors
    ///
    /// Returns an error if environment variables contain invalid values
    pub fn from_environment() -> Result<Self, AnalysisConfigError> {
        let mut config = Self::default();

        // Apply environment variable overrides
        if let Ok(val) = std::env::var("INTELLIGENCE_FITNESS_SCORE_WEEKS") {
            config.timeframes.fitness_score_weeks = val.parse().map_err(|_| {
                AnalysisConfigError::InvalidTimeframe("INTELLIGENCE_FITNESS_SCORE_WEEKS".into())
            })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_TREND_ANALYSIS_WEEKS") {
            config.timeframes.trend_analysis_weeks = val.parse().map_err(|_| {
                AnalysisConfigError::InvalidTimeframe("INTELLIGENCE_TREND_ANALYSIS_WEEKS".into())
            })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_HIGH_R_SQUARED_THRESHOLD") {
            config.confidence.high_r_squared = val.parse().map_err(|_| {
                AnalysisConfigError::InvalidThreshold(
                    "INTELLIGENCE_HIGH_R_SQUARED_THRESHOLD".into(),
                )
            })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_SIGNIFICANCE_THRESHOLD") {
            config.confidence.significance_threshold = val.parse().map_err(|_| {
                AnalysisConfigError::InvalidThreshold("INTELLIGENCE_SIGNIFICANCE_THRESHOLD".into())
            })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_MIN_WEEKLY_VOLUME") {
            config.performance.min_weekly_volume_hours = val.parse().map_err(|_| {
                AnalysisConfigError::InvalidThreshold("INTELLIGENCE_MIN_WEEKLY_VOLUME".into())
            })?;
        }

        if let Ok(val) = std::env::var("INTELLIGENCE_HIGH_WEEKLY_VOLUME") {
            config.performance.high_weekly_volume_hours = val.parse().map_err(|_| {
                AnalysisConfigError::InvalidThreshold("INTELLIGENCE_HIGH_WEEKLY_VOLUME".into())
            })?;
        }

        config.validate()?;
        Ok(config)
    }

    /// Validate configuration values
    ///
    /// # Errors
    ///
    /// Returns an error if any configuration values are invalid
    pub fn validate(&self) -> Result<(), AnalysisConfigError> {
        // Validate timeframes
        if self.timeframes.fitness_score_weeks == 0 {
            return Err(AnalysisConfigError::ValidationFailed(
                "fitness_score_weeks must be > 0".into(),
            ));
        }

        if self.timeframes.trend_analysis_weeks < self.timeframes.fitness_score_weeks {
            return Err(AnalysisConfigError::ValidationFailed(
                "trend_analysis_weeks should be >= fitness_score_weeks".into(),
            ));
        }

        // Validate confidence thresholds
        if !(0.0..=1.0).contains(&self.confidence.high_r_squared) {
            return Err(AnalysisConfigError::ValidationFailed(
                "high_r_squared must be between 0 and 1".into(),
            ));
        }

        if !(0.0..=1.0).contains(&self.confidence.medium_r_squared) {
            return Err(AnalysisConfigError::ValidationFailed(
                "medium_r_squared must be between 0 and 1".into(),
            ));
        }

        if self.confidence.high_r_squared < self.confidence.medium_r_squared {
            return Err(AnalysisConfigError::ValidationFailed(
                "high_r_squared must be >= medium_r_squared".into(),
            ));
        }

        // Validate statistical config
        if !(0.0..=1.0).contains(&self.statistical.exponential_smoothing_alpha) {
            return Err(AnalysisConfigError::ValidationFailed(
                "exponential_smoothing_alpha must be between 0 and 1".into(),
            ));
        }

        if self.statistical.outlier_z_score_threshold <= 0.0 {
            return Err(AnalysisConfigError::ValidationFailed(
                "outlier_z_score_threshold must be > 0".into(),
            ));
        }

        // Validate performance thresholds
        if self.performance.min_weekly_volume_hours < 0.0 {
            return Err(AnalysisConfigError::ValidationFailed(
                "min_weekly_volume_hours must be >= 0".into(),
            ));
        }

        if self.performance.high_weekly_volume_hours <= self.performance.min_weekly_volume_hours {
            return Err(AnalysisConfigError::ValidationFailed(
                "high_weekly_volume_hours must be > min_weekly_volume_hours".into(),
            ));
        }

        // Validate fitness scoring weights sum to 1.0
        let weight_sum = self.fitness_scoring.aerobic_weight
            + self.fitness_scoring.strength_weight
            + self.fitness_scoring.consistency_weight;

        if (weight_sum - 1.0).abs() > 0.01 {
            return Err(AnalysisConfigError::ValidationFailed(format!(
                "Fitness scoring weights must sum to 1.0, got {weight_sum}"
            )));
        }

        Ok(())
    }

    /// Get training gap threshold as Duration
    #[must_use]
    pub const fn training_gap_duration(&self) -> Duration {
        let seconds = self.timeframes.training_gap_days * 24 * 3600;
        Duration::from_secs(seconds.unsigned_abs())
    }

    /// Get recovery analysis duration as Duration
    #[must_use]
    pub const fn recovery_analysis_duration(&self) -> Duration {
        let seconds = self.timeframes.recovery_analysis_days * 24 * 3600;
        Duration::from_secs(seconds.unsigned_abs())
    }

    /// Check if a given number of data points provides high confidence
    #[must_use]
    pub const fn is_high_confidence_data(&self, data_points: usize) -> bool {
        data_points >= self.confidence.high_data_points
    }

    /// Check if a given number of data points provides medium confidence
    #[must_use]
    pub const fn is_medium_confidence_data(&self, data_points: usize) -> bool {
        data_points >= self.confidence.medium_data_points
    }

    /// Get confidence level based on R-squared and data points
    #[must_use]
    pub fn calculate_confidence_level(
        &self,
        r_squared: f64,
        data_points: usize,
    ) -> ConfidenceLevel {
        if self.is_high_confidence_data(data_points) && r_squared >= self.confidence.high_r_squared
        {
            ConfidenceLevel::High
        } else if self.is_medium_confidence_data(data_points)
            && r_squared >= self.confidence.medium_r_squared
        {
            ConfidenceLevel::Medium
        } else {
            ConfidenceLevel::Low
        }
    }
}

/// Confidence levels for analysis results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    Low,
    Medium,
    High,
    VeryHigh,
}

impl ConfidenceLevel {
    /// Convert to numeric score (0.0 to 1.0)
    #[must_use]
    pub const fn as_score(self) -> f64 {
        match self {
            Self::Low => 0.25,
            Self::Medium => 0.50,
            Self::High => 0.75,
            Self::VeryHigh => 0.95,
        }
    }

    /// Create from numeric score
    #[must_use]
    pub fn from_score(score: f64) -> Self {
        if score >= 0.90 {
            Self::VeryHigh
        } else if score >= 0.70 {
            Self::High
        } else if score >= 0.45 {
            Self::Medium
        } else {
            Self::Low
        }
    }
}
