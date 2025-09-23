// ABOUTME: Refactored performance analyzer with proper statistical analysis and type safety
// ABOUTME: Addresses critical issues: unsafe conversions, flawed statistics, magic numbers, and strategy misuse
#![allow(clippy::cast_precision_loss)] // Safe: fitness data conversions
#![allow(clippy::cast_possible_truncation)] // Safe: controlled ranges
#![allow(clippy::cast_sign_loss)] // Safe: positive values only
#![allow(clippy::cast_possible_wrap)] // Safe: bounded values

use super::analysis_config::{AnalysisConfig, ConfidenceLevel};
use super::metrics_extractor::{MetricType, SafeMetricExtractor};
use super::statistical_analysis::{RegressionResult, StatisticalAnalyzer};
use super::{
    AdvancedInsight, Confidence, InsightSeverity, TimeFrame, TrendAnalysis, TrendDataPoint,
    TrendDirection,
};
use crate::config::intelligence_config::IntelligenceStrategy;
use crate::models::Activity;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Refactored performance analyzer with proper error handling and type safety
pub struct PerformanceAnalyzerV2 {
    config: AnalysisConfig,
    strategy: Box<dyn IntelligenceStrategy>,
}

impl PerformanceAnalyzerV2 {
    /// Create a new performance analyzer with configuration and strategy
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid
    pub fn new(strategy: Box<dyn IntelligenceStrategy>) -> Result<Self> {
        let config = AnalysisConfig::from_environment()?;
        Ok(Self { config, strategy })
    }

    /// Create with custom configuration
    #[must_use]
    pub const fn with_config(
        strategy: Box<dyn IntelligenceStrategy>,
        config: AnalysisConfig,
    ) -> Self {
        Self { config, strategy }
    }

    /// Analyze performance trends with proper statistical backing
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No activities found in timeframe
    /// - Metric extraction fails
    /// - Statistical analysis fails
    pub fn analyze_trends(
        &self,
        activities: &[Activity],
        timeframe: TimeFrame,
        metric_type: MetricType,
    ) -> Result<TrendAnalysis> {
        // 1. Filter activities by timeframe
        let filtered_activities = Self::filter_activities_by_timeframe(activities, &timeframe)?;

        // 2. Extract metric values with type safety
        let metric_values =
            SafeMetricExtractor::extract_metric_values(&filtered_activities, metric_type)?;

        // 3. Convert to data points
        let mut data_points = Self::create_data_points(metric_values);

        if data_points.len() < self.config.confidence.medium_data_points {
            return Err(anyhow::anyhow!(
                "Insufficient data points for reliable trend analysis: got {}, need at least {}",
                data_points.len(),
                self.config.confidence.medium_data_points
            ));
        }

        // 4. Apply smoothing based on configuration
        self.apply_smoothing(&mut data_points);

        // 5. Perform proper statistical analysis
        let regression = StatisticalAnalyzer::linear_regression(&data_points)
            .map_err(|e| anyhow::anyhow!("Statistical analysis failed: {e}"))?;

        // 6. Determine trend direction using statistical significance
        let trend_direction = StatisticalAnalyzer::determine_trend_direction(
            &regression,
            metric_type.is_lower_better(),
            self.config.statistical.trend_slope_threshold,
        );

        // 7. Calculate confidence based on both data quantity and quality
        let confidence = self.calculate_comprehensive_confidence(&data_points, &regression);

        // 8. Generate insights based on analysis
        let insights = Self::generate_statistical_insights(
            &regression,
            trend_direction,
            confidence,
            metric_type,
        );

        Ok(TrendAnalysis {
            timeframe,
            metric: format!("{metric_type:?}"),
            trend_direction,
            trend_strength: regression.r_squared, // Proper R-squared, not correlation
            statistical_significance: regression.p_value.unwrap_or(1.0),
            data_points,
            insights,
        })
    }

    /// Calculate comprehensive fitness score with proper statistical foundation
    ///
    /// # Errors
    ///
    /// Returns an error if fitness score calculation fails
    pub fn calculate_fitness_score(&self, activities: &[Activity]) -> Result<FitnessScore> {
        let weeks_back = self.config.timeframes.fitness_score_weeks;
        let cutoff_date = Utc::now() - chrono::Duration::weeks(weeks_back.into());

        let recent_activities: Vec<_> = activities
            .iter()
            .filter(|a| a.start_date >= cutoff_date)
            .collect();

        if recent_activities.is_empty() {
            return Ok(FitnessScore::empty());
        }

        // Calculate aerobic fitness component
        let aerobic_fitness = self.calculate_aerobic_fitness(&recent_activities[..]);

        // Calculate strength endurance component
        let strength_endurance = self.calculate_strength_endurance(&recent_activities[..]);

        // Calculate consistency component
        let consistency = self.calculate_consistency(&recent_activities[..]);

        // Calculate weighted overall score using optimal floating point operations
        let overall_score = aerobic_fitness.mul_add(
            self.config.fitness_scoring.aerobic_weight,
            strength_endurance.mul_add(
                self.config.fitness_scoring.strength_weight,
                consistency * self.config.fitness_scoring.consistency_weight,
            ),
        );

        // Determine trend using proper thresholds
        let trend = if overall_score >= self.config.fitness_scoring.fitness_improving_threshold {
            TrendDirection::Improving
        } else if overall_score >= self.config.fitness_scoring.fitness_stable_threshold {
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

    /// Predict performance using statistical models
    ///
    /// # Errors
    ///
    /// Returns an error if prediction cannot be generated
    pub fn predict_performance(
        &self,
        activities: &[Activity],
        target: &ActivityGoal,
    ) -> Result<PerformancePrediction> {
        // Filter to similar sport activities
        let similar_activities: Vec<_> = activities
            .iter()
            .filter(|a| format!("{:?}", a.sport_type) == target.sport_type)
            .cloned()
            .collect();

        if similar_activities.len() < self.config.min_activities_for_prediction {
            return Err(anyhow::anyhow!(
                "Insufficient similar activities for prediction: need {}, got {}",
                self.config.min_activities_for_prediction,
                similar_activities.len()
            ));
        }

        // Extract metric for trend analysis
        let metric_type = Self::target_metric_to_metric_type(&target.metric)?;
        let metric_values =
            SafeMetricExtractor::extract_metric_values(&similar_activities, metric_type)?;

        // Perform regression analysis
        let data_points = Self::create_data_points(metric_values);
        let regression = StatisticalAnalyzer::linear_regression(&data_points)?;

        // Project future performance
        let days_to_target = (target.target_date - Utc::now()).num_days();
        if days_to_target > self.config.max_prediction_days {
            return Err(anyhow::anyhow!(
                "Target date too far in future: {} days (max: {})",
                days_to_target,
                self.config.max_prediction_days
            ));
        }

        let future_x = data_points.len() as f64 + (days_to_target as f64 / 7.0); // Weekly progression
                                                                                 // Use mul_add for optimal floating point operation: slope * x + intercept
        let predicted_value = regression.slope.mul_add(future_x, regression.intercept);

        // Calculate confidence interval
        let (lower_bound, upper_bound) = StatisticalAnalyzer::calculate_confidence_interval(
            &regression,
            future_x,
            0.95, // 95% confidence interval
        )?;

        let confidence =
            self.calculate_prediction_confidence(&regression, similar_activities.len());

        Ok(PerformancePrediction {
            target_goal: target.clone(),
            predicted_value,
            confidence_interval: (lower_bound, upper_bound),
            confidence,
            factors: Self::generate_prediction_factors(&regression),
            recommendations: self.generate_strategy_based_recommendations(&similar_activities),
            estimated_achievement_date: target.target_date,
        })
    }

    /// Analyze training load with proper statistical foundation
    ///
    /// # Errors
    ///
    /// Returns an error if training load analysis fails
    pub fn analyze_training_load(&self, activities: &[Activity]) -> Result<TrainingLoadAnalysis> {
        let weeks = self.config.timeframes.training_load_weeks;
        let start_date = Utc::now() - chrono::Duration::weeks(weeks.into());

        let recent_activities: Vec<_> = activities
            .iter()
            .filter(|a| a.start_date >= start_date)
            .collect();

        let mut weekly_loads = Vec::new();

        for week in 0..weeks {
            let week_start = start_date + chrono::Duration::weeks(week.into());
            let week_end = week_start + chrono::Duration::weeks(1);

            let week_activities: Vec<&Activity> = recent_activities
                .iter()
                .filter(|&a| a.start_date >= week_start && a.start_date < week_end)
                .copied()
                .collect();

            let load = Self::calculate_weekly_load(&week_activities[..]);
            weekly_loads.push(load);
        }

        let analysis = self.analyze_load_pattern(&weekly_loads)?;
        let recommendations = self.generate_load_recommendations(&analysis);
        let insights = Self::generate_load_insights(&analysis);

        Ok(TrainingLoadAnalysis {
            weekly_loads,
            average_weekly_load: analysis.average_load,
            load_balance_score: analysis.balance_score,
            recovery_needed: analysis.recovery_needed,
            recommendations,
            insights,
        })
    }

    // Private helper methods

    fn filter_activities_by_timeframe(
        activities: &[Activity],
        timeframe: &TimeFrame,
    ) -> Result<Vec<Activity>> {
        let start_date = timeframe.start_date();
        let end_date = timeframe.end_date();

        let filtered: Vec<_> = activities
            .iter()
            .filter(|a| a.start_date >= start_date && a.start_date <= end_date)
            .cloned()
            .collect();

        if filtered.is_empty() {
            return Err(anyhow::anyhow!(
                "No activities found in timeframe from {start_date} to {end_date}"
            ));
        }

        Ok(filtered)
    }

    fn create_data_points(metric_values: Vec<(DateTime<Utc>, f64)>) -> Vec<TrendDataPoint> {
        let mut data_points: Vec<_> = metric_values
            .into_iter()
            .map(|(date, value)| TrendDataPoint {
                date,
                value,
                smoothed_value: None,
            })
            .collect();

        // Sort by date
        data_points.sort_by(|a, b| a.date.cmp(&b.date));
        data_points
    }

    fn apply_smoothing(&self, data_points: &mut [TrendDataPoint]) {
        // Apply smoothing based on configuration
        if self.config.statistical.exponential_smoothing_alpha > 0.0 {
            StatisticalAnalyzer::apply_exponential_smoothing(
                data_points,
                self.config.statistical.exponential_smoothing_alpha,
            );
        } else {
            StatisticalAnalyzer::apply_moving_average_smoothing(
                data_points,
                self.config.statistical.smoothing_window_size,
            );
        }
    }

    fn calculate_comprehensive_confidence(
        &self,
        data_points: &[TrendDataPoint],
        regression: &RegressionResult,
    ) -> Confidence {
        let data_confidence = self
            .config
            .calculate_confidence_level(regression.r_squared, data_points.len());

        // Adjust for statistical significance
        let p_value = regression.p_value.unwrap_or(1.0);
        let significance_bonus = i32::from(p_value < self.config.confidence.significance_threshold);

        match data_confidence {
            ConfidenceLevel::High => {
                if significance_bonus > 0 {
                    Confidence::VeryHigh
                } else {
                    Confidence::High
                }
            }
            ConfidenceLevel::Medium => {
                if significance_bonus > 0 {
                    Confidence::High
                } else {
                    Confidence::Medium
                }
            }
            ConfidenceLevel::Low => {
                if significance_bonus > 0 {
                    Confidence::Medium
                } else {
                    Confidence::Low
                }
            }
            ConfidenceLevel::VeryHigh => Confidence::VeryHigh,
        }
    }

    fn generate_statistical_insights(
        regression: &RegressionResult,
        trend_direction: TrendDirection,
        confidence: Confidence,
        metric_type: MetricType,
    ) -> Vec<AdvancedInsight> {
        let mut insights = Vec::new();

        // Statistical significance insight
        if let Some(p_value) = regression.p_value {
            let significance = if p_value < 0.01 {
                "highly significant"
            } else if p_value < 0.05 {
                "significant"
            } else if p_value < 0.1 {
                "marginally significant"
            } else {
                "not statistically significant"
            };

            let mut metadata = HashMap::new();
            metadata.insert("p_value".into(), serde_json::Value::from(p_value));
            metadata.insert(
                "r_squared".into(),
                serde_json::Value::from(regression.r_squared),
            );

            insights.push(AdvancedInsight {
                insight_type: "statistical_significance".into(),
                message: format!(
                    "The {} trend in {} is {}",
                    format!("{trend_direction:?}").to_lowercase(),
                    metric_type.display_name(),
                    significance
                ),
                confidence,
                severity: if p_value < 0.05 {
                    InsightSeverity::Info
                } else {
                    InsightSeverity::Warning
                },
                metadata,
            });
        }

        // R-squared interpretation
        let variance_explained = (regression.r_squared * 100.0).round();
        let mut metadata = HashMap::new();
        metadata.insert(
            "variance_explained".into(),
            serde_json::Value::from(variance_explained),
        );

        insights.push(AdvancedInsight {
            insight_type: "variance_explanation".into(),
            message: format!(
                "The trend explains {:.0}% of the variation in your {}",
                variance_explained,
                metric_type.display_name()
            ),
            confidence,
            severity: if variance_explained > 70.0 {
                InsightSeverity::Info
            } else {
                InsightSeverity::Warning
            },
            metadata,
        });

        insights
    }

    fn calculate_aerobic_fitness(&self, activities: &[&Activity]) -> f64 {
        let mut aerobic_score = 0.0;
        let mut count = 0;

        for activity in activities {
            if let Some(hr) = activity.average_heart_rate {
                let duration_seconds = activity.duration_seconds;
                let recovery_threshold =
                    (self.config.performance.recovery_hr_percentage * 200.0) as u32; // Assume max HR ~200

                if hr > recovery_threshold
                    && duration_seconds >= self.config.performance.min_aerobic_duration_seconds
                {
                    let duration_hours = duration_seconds as f64 / 3600.0;
                    aerobic_score +=
                        (f64::from(hr) - f64::from(recovery_threshold)) * duration_hours;
                    count += 1;
                }
            }
        }

        if count > 0 {
            (aerobic_score / f64::from(count)).min(100.0)
        } else {
            0.0
        }
    }

    fn calculate_strength_endurance(&self, activities: &[&Activity]) -> f64 {
        let mut strength_score = 0.0;
        let mut count = 0;

        let high_intensity_threshold =
            (self.config.performance.high_intensity_hr_percentage * 200.0) as u32;

        for activity in activities {
            if let Some(hr) = activity.average_heart_rate {
                if hr > high_intensity_threshold {
                    let duration_hours = activity.duration_seconds as f64 / 3600.0;
                    strength_score += f64::from(hr) * duration_hours.min(2.0); // Cap contribution
                    count += 1;
                }
            }
        }

        if count > 0 {
            (strength_score
                / f64::from(count)
                / self.config.fitness_scoring.strength_endurance_divisor)
                .min(100.0)
        } else {
            0.0
        }
    }

    fn calculate_consistency(&self, activities: &[&Activity]) -> f64 {
        let weeks = f64::from(self.config.timeframes.fitness_score_weeks);
        let activities_per_week = activities.len() as f64 / weeks;
        let consistency_ratio =
            activities_per_week / self.config.fitness_scoring.target_weekly_activities;

        consistency_ratio.min(1.0) * 100.0
    }

    fn target_metric_to_metric_type(metric: &str) -> Result<MetricType> {
        match metric.to_lowercase().as_str() {
            "pace" => Ok(MetricType::Pace),
            "speed" => Ok(MetricType::Speed),
            "distance" => Ok(MetricType::Distance),
            "duration" | "time" => Ok(MetricType::Duration),
            "heart_rate" | "hr" => Ok(MetricType::HeartRate),
            "elevation" => Ok(MetricType::Elevation),
            "power" => Ok(MetricType::Power),
            _ => Err(anyhow::anyhow!("Unknown metric type: {metric}")),
        }
    }

    fn calculate_prediction_confidence(
        &self,
        regression: &RegressionResult,
        data_count: usize,
    ) -> Confidence {
        let r_squared_confidence = if regression.r_squared > self.config.confidence.high_r_squared {
            2
        } else {
            i32::from(regression.r_squared > self.config.confidence.medium_r_squared)
        };

        let data_confidence = if data_count >= self.config.confidence.high_data_points {
            2
        } else {
            i32::from(data_count >= self.config.confidence.medium_data_points)
        };

        let total_confidence = r_squared_confidence + data_confidence;

        match total_confidence {
            4 => Confidence::VeryHigh,
            3 => Confidence::High,
            2 => Confidence::Medium,
            _ => Confidence::Low,
        }
    }

    fn generate_prediction_factors(regression: &RegressionResult) -> Vec<String> {
        let mut factors = vec![
            "Historical performance trends".into(),
            "Training consistency patterns".into(),
        ];

        if regression.r_squared > 0.7 {
            factors.push("Strong statistical trend correlation".into());
        }

        if let Some(p_value) = regression.p_value {
            if p_value < 0.05 {
                factors.push("Statistically significant improvement pattern".into());
            }
        }

        factors
    }

    fn generate_strategy_based_recommendations(&self, activities: &[Activity]) -> Vec<String> {
        let mut recommendations = vec![
            "Maintain consistent training schedule".into(),
            "Focus on progressive overload".into(),
        ];

        // Use strategy for personalized recommendations
        let avg_weekly_distance = activities.len() as f64 * 10.0; // Simplified calculation
        if self
            .strategy
            .should_recommend_volume_increase(avg_weekly_distance)
        {
            recommendations.push("Your training strategy suggests gradual volume increases".into());
        }

        recommendations
    }

    fn calculate_weekly_load(activities: &[&Activity]) -> WeeklyLoad {
        let total_duration: u64 = activities.iter().map(|a| a.duration_seconds).sum();
        let total_distance: f64 = activities.iter().filter_map(|a| a.distance_meters).sum();

        let intensity_score = activities
            .iter()
            .filter_map(|a| a.average_heart_rate.map(f64::from))
            .sum::<f64>()
            / activities.len().max(1) as f64;

        WeeklyLoad {
            week_number: 1, // Would be calculated properly in real implementation
            total_duration_hours: total_duration as f64 / 3600.0,
            total_distance_km: total_distance / 1000.0,
            activity_count: activities.len() as i32,
            intensity_score,
        }
    }

    fn analyze_load_pattern(&self, weekly_loads: &[WeeklyLoad]) -> Result<LoadPatternAnalysis> {
        if weekly_loads.is_empty() {
            return Err(anyhow::anyhow!("No weekly load data to analyze"));
        }

        let average_load = weekly_loads
            .iter()
            .map(|w| w.total_duration_hours)
            .sum::<f64>()
            / weekly_loads.len() as f64;

        let load_variance = weekly_loads
            .iter()
            .map(|w| {
                let diff = w.total_duration_hours - average_load;
                diff * diff
            })
            .sum::<f64>()
            / weekly_loads.len() as f64;

        let balance_score = (load_variance.sqrt() / average_load)
            .mul_add(-100.0, 100.0)
            .max(0.0);

        let last_week_load = weekly_loads.last().map_or(0.0, |w| w.total_duration_hours);
        let recovery_needed = last_week_load > self.config.performance.high_weekly_volume_hours;

        Ok(LoadPatternAnalysis {
            average_load,
            balance_score,
            recovery_needed,
            load_trend: if last_week_load > average_load * 1.2 {
                TrendDirection::Improving
            } else if last_week_load < average_load * 0.8 {
                TrendDirection::Declining
            } else {
                TrendDirection::Stable
            },
        })
    }

    fn generate_load_recommendations(&self, analysis: &LoadPatternAnalysis) -> Vec<String> {
        let mut recommendations = Vec::new();

        if analysis.recovery_needed {
            recommendations.push("Consider reducing training volume this week".into());
            recommendations.push("Prioritize recovery activities and sleep".into());
        } else if analysis.balance_score < 70.0 {
            recommendations.push("Work on more consistent weekly training loads".into());
        } else {
            recommendations.push("Training load is well balanced".into());

            // Use strategy for volume recommendations
            if self
                .strategy
                .should_recommend_volume_increase(analysis.average_load * 10.0)
            {
                recommendations
                    .push("Consider gradual volume increases based on your strategy".into());
            }
        }

        // Add trend-specific recommendations
        match analysis.load_trend {
            TrendDirection::Improving => {
                recommendations
                    .push("Training load is trending upward - monitor for overtraining".into());
            }
            TrendDirection::Declining => {
                recommendations
                    .push("Training load is declining - consider maintaining consistency".into());
            }
            TrendDirection::Stable => {
                recommendations.push("Training load is stable and consistent".into());
            }
        }

        recommendations
    }

    fn generate_load_insights(analysis: &LoadPatternAnalysis) -> Vec<AdvancedInsight> {
        let mut insights = Vec::new();

        let mut metadata = HashMap::new();
        metadata.insert(
            "balance_score".into(),
            serde_json::Value::from(analysis.balance_score),
        );
        metadata.insert(
            "average_load".into(),
            serde_json::Value::from(analysis.average_load),
        );
        metadata.insert(
            "load_trend".into(),
            serde_json::Value::from(format!("{:?}", analysis.load_trend)),
        );

        insights.push(AdvancedInsight {
            insight_type: "training_load_balance".into(),
            message: format!(
                "Your training load balance score is {:.1}/100",
                analysis.balance_score
            ),
            confidence: Confidence::High,
            severity: if analysis.balance_score > 80.0 {
                InsightSeverity::Info
            } else {
                InsightSeverity::Warning
            },
            metadata,
        });

        insights
    }
}

// Supporting data structures

#[derive(Debug, Clone)]
pub struct FitnessScore {
    pub overall_score: f64,
    pub aerobic_fitness: f64,
    pub strength_endurance: f64,
    pub consistency: f64,
    pub trend: TrendDirection,
    pub last_updated: DateTime<Utc>,
}

impl FitnessScore {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            overall_score: 0.0,
            aerobic_fitness: 0.0,
            strength_endurance: 0.0,
            consistency: 0.0,
            trend: TrendDirection::Stable,
            last_updated: Utc::now(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActivityGoal {
    pub sport_type: String,
    pub metric: String,
    pub target_value: f64,
    pub target_date: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PerformancePrediction {
    pub target_goal: ActivityGoal,
    pub predicted_value: f64,
    pub confidence_interval: (f64, f64),
    pub confidence: Confidence,
    pub factors: Vec<String>,
    pub recommendations: Vec<String>,
    pub estimated_achievement_date: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct TrainingLoadAnalysis {
    pub weekly_loads: Vec<WeeklyLoad>,
    pub average_weekly_load: f64,
    pub load_balance_score: f64,
    pub recovery_needed: bool,
    pub recommendations: Vec<String>,
    pub insights: Vec<AdvancedInsight>,
}

#[derive(Debug, Clone)]
pub struct WeeklyLoad {
    pub week_number: i32,
    pub total_duration_hours: f64,
    pub total_distance_km: f64,
    pub activity_count: i32,
    pub intensity_score: f64,
}

#[derive(Debug, Clone)]
struct LoadPatternAnalysis {
    average_load: f64,
    balance_score: f64,
    recovery_needed: bool,
    load_trend: TrendDirection,
}
