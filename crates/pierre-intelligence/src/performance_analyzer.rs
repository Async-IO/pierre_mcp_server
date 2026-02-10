// ABOUTME: Performance trend analysis and historical comparison engine for fitness progression
// ABOUTME: Tracks fitness improvements, identifies performance patterns, and provides trend analysis
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//! Performance trend analysis and historical comparison engine
#![allow(clippy::cast_precision_loss)] // Safe: fitness data conversions
#![allow(clippy::cast_possible_truncation)] // Safe: controlled ranges

use std::cmp::min;

use super::{
    AdvancedInsight, Confidence, Deserialize, InsightSeverity, Serialize, TimeFrame, TrendAnalysis,
    TrendDataPoint, TrendDirection, UserFitnessProfile, FITNESS_IMPROVING_THRESHOLD,
    FITNESS_STABLE_THRESHOLD, MIN_STATISTICAL_SIGNIFICANCE_POINTS, SMALL_DATASET_REDUCTION_FACTOR,
    STATISTICAL_SIGNIFICANCE_THRESHOLD, STRENGTH_ENDURANCE_DIVISOR,
};
use crate::config::intelligence::{
    DefaultStrategy, IntelligenceConfig, IntelligenceStrategy, PerformanceAnalyzerConfig,
};
use crate::errors::{AppError, AppResult};
use crate::models::Activity;
use crate::physiological_constants::{
    adaptations::{
        HIGH_VOLUME_IMPROVEMENT_FACTOR, LOW_VOLUME_IMPROVEMENT_FACTOR,
        MODERATE_VOLUME_IMPROVEMENT_FACTOR,
    },
    duration::MIN_AEROBIC_DURATION,
    fitness_weights::{AEROBIC_WEIGHT, CONSISTENCY_WEIGHT, STRENGTH_WEIGHT},
    heart_rate::{HIGH_INTENSITY_HR_THRESHOLD, MODERATE_HR_THRESHOLD, RECOVERY_HR_THRESHOLD},
    performance::TARGET_WEEKLY_ACTIVITIES,
    statistics::STABILITY_THRESHOLD,
    training_load::{RECOVERY_LOAD_MULTIPLIER, TWO_WEEK_RECOVERY_THRESHOLD},
};
use chrono::{DateTime, Utc};
use std::cmp::Ordering;
use std::collections::HashMap;

/// Safe casting helper functions  
#[inline]
fn safe_u32_to_f32(value: u32) -> f32 {
    // Use f64 intermediate and proper conversion
    let as_f64 = f64::from(value);
    safe_f64_to_f32(as_f64)
}

/// Safe cast from f64 to f32 using `total_cmp` for comparison
#[inline]
fn safe_f64_to_f32(value: f64) -> f32 {
    // Handle special cases
    if value.is_nan() {
        return 0.0_f32;
    }

    // Use total_cmp for proper comparison without casting warnings
    if value.total_cmp(&f64::from(f32::MAX)) == Ordering::Greater {
        f32::MAX
    } else if value.total_cmp(&f64::from(f32::MIN)) == Ordering::Less {
        f32::MIN
    } else {
        // Value is within f32 range, use rounding conversion
        let rounded = value.round();
        if rounded > f64::from(f32::MAX) {
            f32::MAX
        } else if rounded < f64::from(f32::MIN) {
            f32::MIN
        } else {
            // Safe conversion using IEEE 754 standard rounding
            {
                rounded as f32
            }
        }
    }
}

/// Safe cast from u64 to f64
#[inline]
const fn safe_u64_to_f64(value: u64) -> f64 {
    // u64 to f64 conversion can lose precision for very large values
    // but for duration/count statistics, this is acceptable
    {
        value as f64
    }
}

/// Trait for analyzing performance trends over time
#[async_trait::async_trait]
pub trait PerformanceAnalyzerTrait {
    /// Analyze performance trends over a given timeframe
    async fn analyze_trends(
        &self,
        activities: &[Activity],
        timeframe: TimeFrame,
        metric: &str,
    ) -> AppResult<TrendAnalysis>;

    /// Calculate fitness score based on recent activities
    async fn calculate_fitness_score(&self, activities: &[Activity]) -> AppResult<FitnessScore>;

    /// Predict performance for a target activity
    async fn predict_performance(
        &self,
        activities: &[Activity],
        target: &ActivityGoal,
    ) -> AppResult<PerformancePrediction>;

    /// Calculate training load balance and recovery metrics
    async fn analyze_training_load(
        &self,
        activities: &[Activity],
    ) -> AppResult<TrainingLoadAnalysis>;
}

/// Advanced performance analyzer implementation with configurable strategy
pub struct AdvancedPerformanceAnalyzer<S: IntelligenceStrategy = DefaultStrategy> {
    strategy: S,
    config: PerformanceAnalyzerConfig,
    user_profile: Option<UserFitnessProfile>,
}

impl Default for AdvancedPerformanceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl AdvancedPerformanceAnalyzer {
    /// Create a new performance analyzer with default strategy
    #[must_use]
    pub fn new() -> Self {
        let global_config = IntelligenceConfig::global();
        Self {
            strategy: DefaultStrategy,
            config: global_config.performance_analyzer.clone(),
            user_profile: None,
        }
    }
}

impl<S: IntelligenceStrategy> AdvancedPerformanceAnalyzer<S> {
    /// Create with custom strategy
    #[must_use]
    pub fn with_strategy(strategy: S) -> Self {
        let global_config = IntelligenceConfig::global();
        Self {
            strategy,
            config: global_config.performance_analyzer.clone(),
            user_profile: None,
        }
    }

    /// Create with custom configuration
    #[must_use]
    pub const fn with_config(strategy: S, config: PerformanceAnalyzerConfig) -> Self {
        Self {
            strategy,
            config,
            user_profile: None,
        }
    }

    /// Create analyzer with user profile using default strategy
    #[must_use]
    pub fn with_profile(profile: UserFitnessProfile) -> AdvancedPerformanceAnalyzer {
        let global_config = IntelligenceConfig::global();
        AdvancedPerformanceAnalyzer {
            strategy: DefaultStrategy,
            config: global_config.performance_analyzer.clone(),
            user_profile: Some(profile),
        }
    }

    /// Set user profile for this analyzer
    pub fn set_profile(&mut self, profile: UserFitnessProfile) {
        self.user_profile = Some(profile);
    }

    /// Calculate statistical trend strength
    fn calculate_trend_strength(data_points: &[TrendDataPoint]) -> f64 {
        if data_points.len() < 2 {
            return 0.0;
        }

        // Simple linear regression to calculate R-squared
        let n = f64::from(u32::try_from(data_points.len()).unwrap_or(u32::MAX));
        let sum_x: f64 = (0..data_points.len())
            .map(|i| f64::from(u32::try_from(i).unwrap_or(u32::MAX)))
            .sum();
        let sum_y: f64 = data_points.iter().map(|p| p.value).sum();
        let sum_x_y: f64 = data_points
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let i_f64 = f64::from(u32::try_from(i).unwrap_or(u32::MAX));
                i_f64 * p.value
            })
            .sum();
        let sum_x_squared: f64 = (0..data_points.len())
            .map(|i| {
                let i_f64 = f64::from(u32::try_from(i).unwrap_or(u32::MAX));
                i_f64.powi(2)
            })
            .sum();
        let sum_values_squared: f64 = data_points.iter().map(|p| p.value.powi(2)).sum();

        let numerator = n.mul_add(sum_x_y, -(sum_x * sum_y));
        let denominator = (n.mul_add(sum_x_squared, -sum_x.powi(2))
            * n.mul_add(sum_values_squared, -sum_y.powi(2)))
        .sqrt();

        if denominator == 0.0 {
            return 0.0;
        }

        let correlation = numerator / denominator;
        correlation.abs() // Return absolute correlation as trend strength
    }

    /// Apply smoothing to data points using moving average
    fn apply_smoothing(data_points: &mut [TrendDataPoint], window_size: usize) {
        if window_size <= 1 || data_points.len() < window_size {
            return;
        }

        for i in 0..data_points.len() {
            let start = i.saturating_sub(window_size / 2);
            let end = min(start + window_size, data_points.len());

            let window_sum: f64 = data_points[start..end].iter().map(|p| p.value).sum();
            let window_avg = window_sum / f64::from(u32::try_from(end - start).unwrap_or(u32::MAX));

            data_points[i].smoothed_value = Some(window_avg);
        }
    }

    /// Generate trend insights based on analysis
    fn generate_trend_insights(&self, analysis: &TrendAnalysis) -> Vec<AdvancedInsight> {
        let mut insights = Vec::new();

        // Trend direction insight using config thresholds and strategy
        // Use outlier_threshold from statistical config as multiplier for strong trend detection
        let strong_trend_multiplier = self.config.statistical.outlier_threshold;
        let strong_trend_threshold =
            self.config.trend_analysis.trend_strength_threshold * strong_trend_multiplier;
        let (message, severity) = match analysis.trend_direction {
            TrendDirection::Improving => {
                if analysis.trend_strength > strong_trend_threshold {
                    (
                        "Strong improvement trend detected - excellent progress!".into(),
                        InsightSeverity::Info,
                    )
                } else {
                    (
                        "Gradual improvement trend - keep up the consistent work".into(),
                        InsightSeverity::Info,
                    )
                }
            }
            TrendDirection::Declining => {
                if analysis.trend_strength > strong_trend_threshold {
                    ("Significant decline in performance - consider recovery or training adjustments".into(), InsightSeverity::Warning)
                } else {
                    (
                        "Slight performance decline - may need attention".into(),
                        InsightSeverity::Warning,
                    )
                }
            }
            TrendDirection::Stable => (
                "Performance is stable - consider progressive overload for improvement".into(),
                InsightSeverity::Info,
            ),
        };

        let mut metadata = HashMap::new();
        metadata.insert(
            "trend_strength".into(),
            serde_json::Value::from(analysis.trend_strength),
        );
        metadata.insert(
            "statistical_significance".into(),
            serde_json::Value::from(analysis.statistical_significance),
        );

        insights.push(AdvancedInsight {
            insight_type: "performance_trend".into(),
            message,
            confidence: {
                let confidence_level = self.config.statistical.confidence_level;
                // Use improvement/decline thresholds for confidence calculation
                let improvement_threshold = self.config.trend_analysis.improvement_threshold;
                let decline_threshold = self.config.trend_analysis.decline_threshold.abs();

                let high_threshold = confidence_level * (confidence_level - improvement_threshold);
                let medium_threshold = confidence_level * (confidence_level - decline_threshold);

                if analysis.statistical_significance > high_threshold {
                    Confidence::High
                } else if analysis.statistical_significance > medium_threshold {
                    Confidence::Medium
                } else {
                    Confidence::Low
                }
            },
            severity,
            metadata,
        });

        // Data quality insight using config min data points
        if analysis.data_points.len() < self.config.trend_analysis.min_data_points {
            insights.push(AdvancedInsight {
                insight_type: "data_quality".into(),
                message: "Limited data points - trends may not be reliable".into(),
                confidence: Confidence::Medium,
                severity: InsightSeverity::Warning,
                metadata: HashMap::new(),
            });
        }

        // Strategy-based insights using the strategy field
        if analysis.trend_direction == TrendDirection::Improving {
            let strategy_thresholds = self.strategy.performance_thresholds();
            if analysis.trend_strength > strategy_thresholds.significant_improvement {
                insights.push(AdvancedInsight {
                    insight_type: "strategy_validation".into(),
                    message: "Your training strategy is producing excellent results".into(),
                    confidence: Confidence::High,
                    severity: InsightSeverity::Info,
                    metadata: HashMap::new(),
                });
            }
        }

        insights
    }
}

#[async_trait::async_trait]
impl PerformanceAnalyzerTrait for AdvancedPerformanceAnalyzer {
    async fn analyze_trends(
        &self,
        activities: &[Activity],
        timeframe: TimeFrame,
        metric: &str,
    ) -> AppResult<TrendAnalysis> {
        // Filter activities by timeframe
        let start_date = timeframe.start_date();
        let end_date = timeframe.end_date();

        let filtered_activities: Vec<_> = activities
            .iter()
            .filter(|a| {
                let activity_utc = a.start_date();
                activity_utc >= start_date && activity_utc <= end_date
            })
            .collect();

        if filtered_activities.is_empty() {
            return Err(AppError::not_found(
                "No activities found in the specified timeframe",
            ));
        }

        // Extract metric values
        let mut data_points = Vec::new();

        for activity in filtered_activities {
            let activity_utc = activity.start_date();

            let value = match metric {
                "pace" | "speed" => activity.average_speed(),
                "heart_rate" => activity.average_heart_rate().map(f64::from),
                "distance" => activity.distance_meters(),
                "duration" => Some(if activity.duration_seconds() > u64::from(u32::MAX) {
                    f64::from(u32::MAX)
                } else {
                    f64::from(u32::try_from(activity.duration_seconds()).unwrap_or(u32::MAX))
                }),
                "elevation" => activity.elevation_gain(),
                _ => None,
            };

            if let Some(v) = value {
                data_points.push(TrendDataPoint {
                    date: activity_utc,
                    value: v,
                    smoothed_value: None,
                });
            }
        }

        if data_points.is_empty() {
            return Err(AppError::not_found(format!(
                "No valid data points found for metric: {metric}"
            )));
        }

        // Sort by date
        data_points.sort_by(|a, b| a.date.cmp(&b.date));

        // Apply smoothing
        Self::apply_smoothing(&mut data_points, 3);

        // Calculate trend direction
        let first_half_avg = data_points[..data_points.len() / 2]
            .iter()
            .map(|p| p.value)
            .sum::<f64>()
            / f64::from(u32::try_from(data_points.len() / 2).unwrap_or(u32::MAX));
        let second_half_avg = data_points[data_points.len() / 2..]
            .iter()
            .map(|p| p.value)
            .sum::<f64>()
            / f64::from(
                u32::try_from(data_points.len() - data_points.len() / 2).unwrap_or(u32::MAX),
            );

        let trend_direction =
            if (second_half_avg - first_half_avg).abs() < first_half_avg * STABILITY_THRESHOLD {
                TrendDirection::Stable
            } else if second_half_avg > first_half_avg {
                if metric == "pace" {
                    // For pace, lower is better
                    TrendDirection::Declining
                } else {
                    TrendDirection::Improving
                }
            } else if metric == "pace" {
                // For pace, lower is better
                TrendDirection::Improving
            } else {
                TrendDirection::Declining
            };

        // Calculate trend strength
        let trend_strength = Self::calculate_trend_strength(&data_points);

        // Calculate statistical significance (simplified)
        let statistical_significance = if data_points.len() > MIN_STATISTICAL_SIGNIFICANCE_POINTS
            && trend_strength > STATISTICAL_SIGNIFICANCE_THRESHOLD
        {
            trend_strength
        } else {
            trend_strength * SMALL_DATASET_REDUCTION_FACTOR
        };

        let analysis = TrendAnalysis {
            timeframe,
            metric: metric.to_owned(),
            trend_direction,
            trend_strength,
            statistical_significance,
            data_points,
            insights: vec![], // Will be filled next
        };

        let mut analysis_with_insights = analysis;
        analysis_with_insights.insights = self.generate_trend_insights(&analysis_with_insights);

        Ok(analysis_with_insights)
    }

    async fn calculate_fitness_score(&self, activities: &[Activity]) -> AppResult<FitnessScore> {
        // Calculate fitness score based on recent training load and consistency
        let recent_activities: Vec<_> = activities
            .iter()
            .filter(|a| {
                let activity_utc = a.start_date();
                let days_ago = (Utc::now() - activity_utc).num_days();
                days_ago <= 42 // Last 6 weeks
            })
            .collect();

        if recent_activities.is_empty() {
            return Ok(FitnessScore {
                overall_score: 0.0,
                aerobic_fitness: 0.0,
                strength_endurance: 0.0,
                consistency: 0.0,
                trend: TrendDirection::Stable,
                last_updated: Utc::now(),
            });
        }

        // Calculate weekly activity frequency
        let weeks = 6;
        let activities_per_week =
            f64::from(u32::try_from(recent_activities.len()).unwrap_or(u32::MAX))
                / f64::from(weeks);
        let consistency = (activities_per_week / TARGET_WEEKLY_ACTIVITIES)
            .min(self.config.statistical.confidence_level)
            * (self.config.statistical.confidence_level * 100.0);

        // Calculate aerobic fitness based on heart rate and duration
        let mut aerobic_score = 0.0;
        let mut aerobic_count = 0;

        for activity in &recent_activities {
            if let Some(hr) = activity.average_heart_rate() {
                let duration = activity.duration_seconds();
                if hr > RECOVERY_HR_THRESHOLD && duration > MIN_AEROBIC_DURATION {
                    // Above aerobic threshold with sufficient duration
                    let duration_hours = if duration > u64::from(u32::MAX) {
                        f64::from(u32::MAX) / 3600.0
                    } else {
                        f64::from(u32::try_from(duration).unwrap_or(u32::MAX)) / 3600.0
                    };
                    aerobic_score +=
                        (f64::from(hr) - f64::from(RECOVERY_HR_THRESHOLD)) * duration_hours;
                    aerobic_count += 1;
                }
            }
        }

        let aerobic_fitness = if aerobic_count > 0 {
            (aerobic_score / f64::from(aerobic_count)).min(100.0)
        } else {
            0.0
        };

        // Calculate strength endurance based on power and effort
        let mut strength_score = 0.0;
        let mut strength_count = 0;

        for activity in &recent_activities {
            // Use heart rate as proxy for intensity since power data is not available
            if let Some(hr) = activity.average_heart_rate() {
                let duration = activity.duration_seconds();

                // High intensity workouts contribute to strength endurance
                if hr > HIGH_INTENSITY_HR_THRESHOLD {
                    // Weight by duration - longer high-intensity efforts indicate better strength endurance
                    let duration_weight = if duration > u64::from(u32::MAX) {
                        (f64::from(u32::MAX) / 3600.0).min(2.0)
                    } else {
                        (f64::from(u32::try_from(duration).unwrap_or(u32::MAX)) / 3600.0).min(2.0)
                    };
                    strength_score += f64::from(hr) * duration_weight;
                    strength_count += 1;
                } else if hr > MODERATE_HR_THRESHOLD {
                    // Moderate intensity also contributes, but less
                    let duration_weight = if duration > u64::from(u32::MAX) {
                        (f64::from(u32::MAX) / 3600.0).min(1.5)
                    } else {
                        (f64::from(u32::try_from(duration).unwrap_or(u32::MAX)) / 3600.0).min(1.5)
                    };
                    strength_score += (f64::from(hr) * 0.6) * duration_weight;
                    strength_count += 1;
                }
            }
        }

        let strength_endurance = if strength_count > 0 {
            (strength_score / f64::from(strength_count) / STRENGTH_ENDURANCE_DIVISOR).min(100.0)
        } else {
            0.0
        };

        // Overall score is weighted average using fitness component weights
        let overall_score = aerobic_fitness
            .mul_add(
                AEROBIC_WEIGHT,
                strength_endurance.mul_add(STRENGTH_WEIGHT, consistency * CONSISTENCY_WEIGHT),
            )
            .min(100.0);

        // Determine trend by comparing with older activities
        let trend = if overall_score > FITNESS_IMPROVING_THRESHOLD {
            TrendDirection::Improving
        } else if overall_score > FITNESS_STABLE_THRESHOLD {
            TrendDirection::Stable
        } else {
            TrendDirection::Declining
        };

        Ok(FitnessScore {
            overall_score,
            aerobic_fitness,
            strength_endurance,
            consistency,
            trend,
            last_updated: Utc::now(),
        })
    }

    async fn predict_performance(
        &self,
        activities: &[Activity],
        target: &ActivityGoal,
    ) -> AppResult<PerformancePrediction> {
        // Simple performance prediction based on recent trends
        let similar_activities: Vec<_> = activities
            .iter()
            .filter(|a| format!("{:?}", a.sport_type()) == target.sport_type)
            .collect();

        if similar_activities.is_empty() {
            return Err(AppError::not_found(
                "No similar activities found for prediction",
            ));
        }

        // Calculate recent average performance
        let recent_performance =
            similar_activities
                .last()
                .map_or(0.0, |last_activity| match target.metric.as_str() {
                    "distance" => last_activity.distance_meters().unwrap_or(0.0),
                    "time" => {
                        if last_activity.duration_seconds() > u64::from(u32::MAX) {
                            f64::from(u32::MAX)
                        } else {
                            f64::from(
                                u32::try_from(last_activity.duration_seconds()).unwrap_or(u32::MAX),
                            )
                        }
                    }
                    "pace" => last_activity.average_speed().unwrap_or(0.0),
                    _ => 0.0,
                });

        // Training adaptation factors based on volume
        let training_days = f64::from(u32::try_from(similar_activities.len()).unwrap_or(u32::MAX));
        let min_data_points_f64 = f64::from(
            u32::try_from(self.config.trend_analysis.min_data_points * 2).unwrap_or(u32::MAX),
        );

        let improvement_factor = if training_days > 20.0 {
            HIGH_VOLUME_IMPROVEMENT_FACTOR
        } else if training_days > min_data_points_f64 {
            MODERATE_VOLUME_IMPROVEMENT_FACTOR
        } else {
            LOW_VOLUME_IMPROVEMENT_FACTOR
        };

        let predicted_value = recent_performance * improvement_factor;

        let confidence = if training_days > 20.0 {
            Confidence::High
        } else if training_days > min_data_points_f64 {
            Confidence::Medium
        } else {
            Confidence::Low
        };

        Ok(PerformancePrediction {
            target_goal: target.clone(),
            predicted_value,
            confidence,
            factors: vec![
                "Recent training consistency".into(),
                "Historical performance trends".into(),
                "Current fitness level".into(),
            ],
            recommendations: vec![
                "Maintain consistent training schedule".into(),
                "Focus on progressive overload".into(),
                "Include recovery sessions".into(),
            ],
            estimated_achievement_date: target.target_date,
        })
    }

    async fn analyze_training_load(
        &self,
        activities: &[Activity],
    ) -> AppResult<TrainingLoadAnalysis> {
        // Analyze training load over recent weeks
        let weeks = 4;
        let start_date = Utc::now() - chrono::Duration::weeks(weeks);

        let recent_activities: Vec<_> = activities
            .iter()
            .filter(|a| {
                let activity_utc = a.start_date();
                activity_utc >= start_date
            })
            .collect();

        let mut weekly_loads = Vec::new();

        for week in 0..weeks {
            let week_start = start_date + chrono::Duration::weeks(week);
            let week_end = week_start + chrono::Duration::weeks(1);

            let week_activities: Vec<_> = recent_activities
                .iter()
                .filter(|a| {
                    let activity_utc = a.start_date();
                    activity_utc >= week_start && activity_utc < week_end
                })
                .collect();

            let total_duration: u64 = week_activities.iter().map(|a| a.duration_seconds()).sum();

            let total_distance: f64 = week_activities
                .iter()
                .filter_map(|a| a.distance_meters())
                .sum();

            weekly_loads.push(WeeklyLoad {
                week_number: i32::try_from(week + 1).unwrap_or(i32::MAX),
                total_duration_hours: safe_u64_to_f64(total_duration) / 3600.0,
                total_distance_km: total_distance / 1000.0,
                activity_count: i32::try_from(week_activities.len()).unwrap_or(i32::MAX),
                intensity_score: week_activities
                    .iter()
                    // Heart rates are small values (30-220), safe to cast to f32
                    .filter_map(|a| a.average_heart_rate().map(safe_u32_to_f32))
                    .map(f64::from)
                    .sum::<f64>()
                    / f64::from(u32::try_from(week_activities.len().max(1)).unwrap_or(u32::MAX)),
            });
        }

        // Calculate load balance
        let avg_load = weekly_loads
            .iter()
            .map(|w| w.total_duration_hours)
            .sum::<f64>()
            / f64::from(u32::try_from(weekly_loads.len()).unwrap_or(u32::MAX));

        let load_variance = weekly_loads
            .iter()
            .map(|w| (w.total_duration_hours - avg_load).powi(2))
            .sum::<f64>()
            / f64::from(u32::try_from(weekly_loads.len()).unwrap_or(u32::MAX));

        let load_balance_score = (load_variance.sqrt() / avg_load)
            .mul_add(-100.0, 100.0)
            .max(0.0);

        // Determine if currently in recovery phase
        let last_week_load = weekly_loads.last().map_or(0.0, |w| w.total_duration_hours);
        let previous_week_load = weekly_loads
            .get(weekly_loads.len().saturating_sub(2))
            .map_or(0.0, |w| w.total_duration_hours);

        let recovery_needed = last_week_load > avg_load * RECOVERY_LOAD_MULTIPLIER
            || (last_week_load + previous_week_load) > avg_load * TWO_WEEK_RECOVERY_THRESHOLD;

        Ok(TrainingLoadAnalysis {
            weekly_loads,
            average_weekly_load: avg_load,
            load_balance_score,
            recovery_needed,
            recommendations: if recovery_needed {
                let mut recs = vec![
                    "Consider reducing training volume this week".into(),
                    "Focus on recovery activities".into(),
                    "Ensure adequate sleep and nutrition".into(),
                ];

                // Add strategy-based recovery recommendations using the strategy field
                // Safe: weeks count is small integer value in training period ranges (0-52)
                if self.strategy.should_recommend_recovery(weeks as i32) {
                    recs.push("Your training strategy recommends prioritizing recovery at this load level".into());
                }

                recs
            } else {
                let mut recs = vec![
                    "Training load is well balanced".into(),
                    "Continue current training pattern".into(),
                    "Consider gradual load increases".into(),
                ];

                // Use strategy to determine if volume increase is appropriate
                // Use smoothing factor as conversion multiplier
                let base_multiplier = self.config.statistical.confidence_level
                    / self.config.statistical.smoothing_factor;
                let conversion_multiplier =
                    self.config.statistical.smoothing_factor * base_multiplier;
                let current_avg_km = avg_load * conversion_multiplier;
                if self
                    .strategy
                    .should_recommend_volume_increase(current_avg_km)
                {
                    recs.push(
                        "Your strategy suggests this is a good time to increase training volume"
                            .to_owned(),
                    );
                }

                recs
            },
            insights: vec![], // Add specific insights based on analysis
        })
    }
}

/// Fitness score components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessScore {
    /// Overall fitness score (0-100)
    pub overall_score: f64,
    /// Aerobic fitness component score (0-100)
    pub aerobic_fitness: f64,
    /// Strength and endurance component score (0-100)
    pub strength_endurance: f64,
    /// Training consistency score (0-100)
    pub consistency: f64,
    /// Current fitness trend direction
    pub trend: TrendDirection,
    /// When this score was last calculated
    pub last_updated: DateTime<Utc>,
}

/// Activity goal for performance prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityGoal {
    /// Type of sport/activity (e.g., "running", "cycling")
    pub sport_type: String,
    /// Metric being targeted ("distance", "time", "pace")
    pub metric: String,
    /// Target value for the metric
    pub target_value: f64,
    /// Target date to achieve the goal
    pub target_date: DateTime<Utc>,
}

/// Performance prediction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformancePrediction {
    /// The goal being predicted
    pub target_goal: ActivityGoal,
    /// Predicted performance value for the metric
    pub predicted_value: f64,
    /// Confidence level in the prediction
    pub confidence: Confidence,
    /// Factors influencing the prediction
    pub factors: Vec<String>,
    /// Recommendations to achieve the goal
    pub recommendations: Vec<String>,
    /// Estimated date when goal will be achieved
    pub estimated_achievement_date: DateTime<Utc>,
}

/// Training load analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingLoadAnalysis {
    /// Weekly training load data points
    pub weekly_loads: Vec<WeeklyLoad>,
    /// Average weekly training load
    pub average_weekly_load: f64,
    /// Balance score between hard and easy weeks (0-100)
    pub load_balance_score: f64,
    /// Whether recovery is currently needed
    pub recovery_needed: bool,
    /// Training recommendations based on load analysis
    pub recommendations: Vec<String>,
    /// Advanced insights about training patterns
    pub insights: Vec<AdvancedInsight>,
}

/// Weekly training load data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyLoad {
    /// Week number (1-52)
    pub week_number: i32,
    /// Total training duration for the week (hours)
    pub total_duration_hours: f64,
    /// Total distance covered in the week (km)
    pub total_distance_km: f64,
    /// Number of activities in the week
    pub activity_count: i32,
    /// Weighted intensity score for the week
    pub intensity_score: f64,
}
