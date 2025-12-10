// ABOUTME: Detailed activity analysis engine for comprehensive workout breakdowns
// ABOUTME: Analyzes individual activities for pace, power, heart rate patterns and training insights
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//! Activity analysis engine for detailed activity insights

use super::{
    ActivityInsights, AdvancedInsight, AdvancedMetrics, Anomaly, Confidence, InsightSeverity,
    MetricsCalculator,
};
use crate::config::intelligence::{ActivityAnalyzerConfig, IntelligenceConfig};
use crate::errors::{AppError, AppResult};
use crate::intelligence::physiological_constants::{
    activity_scoring::{
        BASE_ACTIVITY_SCORE, COMPLETION_BONUS, DURATION_BONUS, HR_ZONE_BONUS, INTENSITY_BONUS,
        STANDARD_BONUS,
    },
    duration::{ENDURANCE_DURATION_THRESHOLD, LONG_WORKOUT_DURATION, MIN_AEROBIC_DURATION},
    efficiency::{EXCELLENT_AEROBIC_EFFICIENCY, GOOD_AEROBIC_EFFICIENCY},
    heart_rate::{
        AEROBIC_THRESHOLD_PERCENTAGE, ANAEROBIC_THRESHOLD_PERCENTAGE, HIGH_INTENSITY_HR_THRESHOLD,
        MAX_REALISTIC_HEART_RATE, MODERATE_HR_THRESHOLD, RECOVERY_HR_THRESHOLD,
        VERY_HIGH_INTENSITY_HR_THRESHOLD,
    },
    max_speeds::{DEFAULT_MAX_SPEED, MAX_CYCLING_SPEED, MAX_RUNNING_SPEED, MAX_SWIMMING_SPEED},
    performance::{HR_EFFICIENCY_IMPROVEMENT_THRESHOLD, PACE_IMPROVEMENT_THRESHOLD},
    power::{COMPETITIVE_POWER_TO_WEIGHT, ELITE_POWER_TO_WEIGHT, RECREATIONAL_POWER_TO_WEIGHT},
    running::FAST_PACE_THRESHOLD,
    training_load::{HIGH_TSS_THRESHOLD, LOW_TSS_THRESHOLD},
};
use crate::models::{Activity, SportType};
use std::collections::HashMap;

/// Safe casting helper functions to avoid clippy warnings
#[inline]
// Safe: value range validation performed within function
fn safe_f64_to_f32(value: f64) -> f32 {
    #[allow(clippy::cast_possible_truncation)] // Safe: clamped to f32 range
    {
        value.clamp(f64::from(f32::MIN), f64::from(f32::MAX)) as f32
    }
}

#[inline]
// Safe: value clamped to u16 range within function
fn safe_u32_to_u16(value: u32) -> u16 {
    #[allow(clippy::cast_possible_truncation)] // Safe: clamped to u16 range
    {
        value.min(u32::from(u16::MAX)) as u16
    }
}

/// Trait for analyzing individual activities
#[async_trait::async_trait]
pub trait ActivityAnalyzerTrait {
    /// Analyze a single activity and generate insights
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Activity data is invalid or corrupted
    /// - Metrics calculation fails
    /// - Anomaly detection fails
    /// - Data processing errors occur
    async fn analyze_activity(&self, activity: &Activity) -> AppResult<ActivityInsights>;

    /// Detect anomalies in activity data
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Activity data is malformed
    /// - Anomaly detection algorithms fail
    /// - Data validation errors occur
    async fn detect_anomalies(&self, activity: &Activity) -> AppResult<Vec<Anomaly>>;

    /// Calculate training load for an activity
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Activity duration data is invalid
    /// - Heart rate data is corrupted
    /// - Training load calculation fails
    /// - Mathematical operations fail
    async fn calculate_training_load(&self, activity: &Activity) -> AppResult<f64>;

    /// Compare activity against user's historical data
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Historical activity data is invalid
    /// - Activity comparison calculations fail
    /// - Statistical analysis fails
    /// - Data aggregation errors occur
    async fn compare_to_history(
        &self,
        activity: &Activity,
        historical_activities: &[Activity],
    ) -> AppResult<Vec<AdvancedInsight>>;
}

/// Advanced activity analyzer implementation
pub struct AdvancedActivityAnalyzer {
    config: ActivityAnalyzerConfig,
    metrics_calculator: MetricsCalculator,
}

impl Default for AdvancedActivityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl AdvancedActivityAnalyzer {
    /// Create a new activity analyzer
    #[must_use]
    pub fn new() -> Self {
        let global_config = IntelligenceConfig::global();
        Self {
            config: global_config.activity_analyzer.clone(),
            metrics_calculator: MetricsCalculator::new(),
        }
    }

    /// Create with custom configuration
    #[must_use]
    pub const fn with_config(config: ActivityAnalyzerConfig) -> Self {
        Self {
            config,
            metrics_calculator: MetricsCalculator::new(),
        }
    }

    /// Create analyzer with user-specific parameters
    #[must_use]
    pub fn with_user_data(
        ftp: Option<f64>,
        lthr: Option<f64>,
        max_hr: Option<f64>,
        resting_hr: Option<f64>,
        weight_kg: Option<f64>,
    ) -> Self {
        let global_config = IntelligenceConfig::global();
        Self {
            config: global_config.activity_analyzer.clone(),
            metrics_calculator: MetricsCalculator::new()
                .with_user_data(ftp, lthr, max_hr, resting_hr, weight_kg),
        }
    }

    /// Generate overall activity score (0-10)
    fn calculate_overall_score(&self, activity: &Activity, metrics: &AdvancedMetrics) -> f64 {
        let mut score: f64 = BASE_ACTIVITY_SCORE; // Base score

        // Use config weights for scoring components
        let scoring_config = &self.config.scoring;

        // Efficiency component - based on completion and metrics quality
        let mut efficiency_score = 0.0;
        if activity.distance_meters.unwrap_or(0.0) > 0.0 {
            efficiency_score += COMPLETION_BONUS;
        }
        if metrics.trimp.is_some() {
            efficiency_score += STANDARD_BONUS;
        }
        if metrics.power_to_weight_ratio.is_some() {
            efficiency_score += STANDARD_BONUS;
        }
        score += efficiency_score * scoring_config.efficiency_weight;

        // Intensity component - based on effort level using physiological thresholds
        let mut intensity_score = 0.0;
        if let Some(avg_hr) = activity.average_heart_rate {
            if avg_hr > MODERATE_HR_THRESHOLD {
                intensity_score += HR_ZONE_BONUS;
            }
            if avg_hr > HIGH_INTENSITY_HR_THRESHOLD {
                intensity_score += INTENSITY_BONUS;
            }
        }
        score += intensity_score * scoring_config.intensity_weight;

        // Duration component - based on duration thresholds
        let mut duration_score = 0.0;
        let duration = activity.duration_seconds;
        if duration > MIN_AEROBIC_DURATION {
            duration_score += DURATION_BONUS;
        }
        if duration > ENDURANCE_DURATION_THRESHOLD {
            duration_score += DURATION_BONUS;
        }
        score += duration_score * scoring_config.duration_weight;

        // Consistency component - based on data completeness and quality
        let mut consistency_score = 0.0;
        if activity.average_heart_rate.is_some() && activity.max_heart_rate.is_some() {
            consistency_score += STANDARD_BONUS;
        }
        if activity.average_speed.is_some() && activity.max_speed.is_some() {
            consistency_score += STANDARD_BONUS;
        }
        score += consistency_score * scoring_config.consistency_weight;

        score.clamp(0.0, 10.0)
    }

    /// Generate insights for activity performance
    fn generate_performance_insights(
        activity: &Activity,
        metrics: &AdvancedMetrics,
    ) -> Vec<AdvancedInsight> {
        let mut insights = Vec::new();

        // Heart rate insights
        if let Some(avg_hr) = activity.average_heart_rate {
            if let Some(max_hr) = activity.max_heart_rate {
                // Heart rates are small values (30-220), use safe conversion
                let hr_reserve_used = (f32::from(safe_u32_to_u16(avg_hr))
                    / f32::from(safe_u32_to_u16(max_hr)))
                    * 100.0;

                let (message, confidence) = if hr_reserve_used > ANAEROBIC_THRESHOLD_PERCENTAGE {
                    (
                        "High intensity effort - excellent cardiovascular challenge".into(),
                        Confidence::High,
                    )
                } else if hr_reserve_used > AEROBIC_THRESHOLD_PERCENTAGE {
                    (
                        "Moderate to high intensity - good aerobic stimulus".into(),
                        Confidence::Medium,
                    )
                } else {
                    (
                        "Low to moderate intensity - great for base building".into(),
                        Confidence::Medium,
                    )
                };

                let mut metadata = HashMap::new();
                metadata.insert(
                    "hr_reserve_percentage".into(),
                    serde_json::Value::from(hr_reserve_used),
                );

                insights.push(AdvancedInsight {
                    insight_type: "heart_rate_analysis".into(),
                    message,
                    confidence,
                    severity: InsightSeverity::Info,
                    metadata,
                });
            }
        }

        // Power insights using established performance categories
        if let Some(power_to_weight) = metrics.power_to_weight_ratio {
            let (message, severity) = if power_to_weight > ELITE_POWER_TO_WEIGHT {
                (
                    "Excellent power-to-weight ratio - elite level performance".into(),
                    InsightSeverity::Info,
                )
            } else if power_to_weight > COMPETITIVE_POWER_TO_WEIGHT {
                (
                    "Good power-to-weight ratio - competitive level".into(),
                    InsightSeverity::Info,
                )
            } else if power_to_weight > RECREATIONAL_POWER_TO_WEIGHT {
                (
                    "Moderate power-to-weight ratio - room for improvement".into(),
                    InsightSeverity::Warning,
                )
            } else {
                (
                    "Consider power training to improve performance".into(),
                    InsightSeverity::Warning,
                )
            };

            let mut metadata = HashMap::new();
            metadata.insert(
                "power_to_weight_ratio".into(),
                serde_json::Value::from(power_to_weight),
            );

            insights.push(AdvancedInsight {
                insight_type: "power_analysis".into(),
                message,
                confidence: Confidence::High,
                severity,
                metadata,
            });
        }

        // Efficiency insights using research-based thresholds
        if let Some(efficiency) = metrics.aerobic_efficiency {
            let message = if efficiency > EXCELLENT_AEROBIC_EFFICIENCY {
                "Excellent aerobic efficiency - well-conditioned cardiovascular system".into()
            } else if efficiency > GOOD_AEROBIC_EFFICIENCY {
                "Good aerobic efficiency - steady cardiovascular fitness".into()
            } else {
                "Consider base training to improve aerobic efficiency".into()
            };

            let mut metadata = HashMap::new();
            metadata.insert(
                "aerobic_efficiency".into(),
                serde_json::Value::from(efficiency),
            );

            insights.push(AdvancedInsight {
                insight_type: "efficiency_analysis".into(),
                message,
                confidence: Confidence::Medium,
                severity: InsightSeverity::Info,
                metadata,
            });
        }

        insights
    }

    /// Generate training recommendations based on activity
    fn generate_recommendations(activity: &Activity, metrics: &AdvancedMetrics) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Duration-based recommendations using physiological thresholds
        let duration = activity.duration_seconds;
        if duration < MIN_AEROBIC_DURATION {
            recommendations
                .push("Consider extending workout duration for better aerobic benefits".into());
        } else if duration > LONG_WORKOUT_DURATION {
            recommendations
                .push("Great endurance work! Ensure proper recovery and nutrition".into());
        }

        // Heart rate based recommendations using established zones
        if let Some(avg_hr) = activity.average_heart_rate {
            if avg_hr < RECOVERY_HR_THRESHOLD {
                recommendations.push(
                    "Consider increasing intensity for better cardiovascular stimulus".into(),
                );
            } else if avg_hr > VERY_HIGH_INTENSITY_HR_THRESHOLD {
                recommendations
                    .push("High intensity session - ensure adequate recovery time".into());
            }
        }

        // Training stress recommendations using established thresholds
        if let Some(tss) = metrics.training_stress_score {
            if tss > HIGH_TSS_THRESHOLD {
                recommendations
                    .push("High training stress - plan recovery days to avoid overtraining".into());
            } else if tss < LOW_TSS_THRESHOLD {
                recommendations
                    .push("Light training load - good for recovery or base building".into());
            }
        }

        // Sport-specific recommendations
        match activity.sport_type {
            SportType::Run => {
                if let Some(pace) = activity.average_speed {
                    if pace > FAST_PACE_THRESHOLD {
                        recommendations
                            .push("Excellent pace! Focus on maintaining form at speed".into());
                    }
                }
                recommendations
                    .push("Consider incorporating strength training for injury prevention".into());
            }
            SportType::Ride => {
                // Power data not available in current Activity model
                recommendations.push("Remember bike maintenance for optimal performance".into());
            }
            SportType::Swim => {
                recommendations.push("Focus on stroke technique and breathing efficiency".into());
            }
            _ => {}
        }

        recommendations
    }

    /// Compare pace performance against historical activities
    fn compare_pace_performance(
        activity: &Activity,
        same_sport_activities: &[&Activity],
    ) -> Option<Vec<AdvancedInsight>> {
        let current_speed = activity.average_speed?;
        let historical_speeds: Vec<f32> = same_sport_activities
            .iter()
            .filter_map(|a| a.average_speed.map(safe_f64_to_f32))
            .collect();

        if historical_speeds.is_empty() {
            return None;
        }

        let avg_historical_speed = {
            let len_f32 = f32::from(u16::try_from(historical_speeds.len()).unwrap_or(u16::MAX));
            historical_speeds.iter().sum::<f32>() / len_f32
        };

        let improvement = ((safe_f64_to_f32(current_speed) - avg_historical_speed)
            / avg_historical_speed)
            * 100.0;

        let mut insights = Vec::new();

        if improvement > safe_f64_to_f32(PACE_IMPROVEMENT_THRESHOLD) {
            let mut metadata = HashMap::new();
            metadata.insert(
                "improvement_percentage".into(),
                serde_json::Value::from(improvement),
            );
            metadata.insert(
                "current_speed".into(),
                serde_json::Value::from(current_speed),
            );
            metadata.insert(
                "historical_average".into(),
                serde_json::Value::from(avg_historical_speed),
            );

            insights.push(AdvancedInsight {
                insight_type: "pace_improvement".into(),
                message: format!(
                    "Pace improved by {improvement:.1}% compared to recent activities"
                ),
                confidence: Confidence::High,
                severity: InsightSeverity::Info,
                metadata,
            });
        } else if improvement < -safe_f64_to_f32(PACE_IMPROVEMENT_THRESHOLD) {
            let mut metadata = HashMap::new();
            metadata.insert(
                "decline_percentage".into(),
                serde_json::Value::from(-improvement),
            );

            insights.push(AdvancedInsight {
                insight_type: "pace_decline".into(),
                message: {
                    let decline = -improvement;
                    format!("Pace was {decline:.1}% slower than recent average")
                },
                confidence: Confidence::Medium,
                severity: InsightSeverity::Warning,
                metadata,
            });
        }

        if insights.is_empty() {
            None
        } else {
            Some(insights)
        }
    }

    /// Compare heart rate efficiency against historical activities
    fn compare_heart_rate_efficiency(
        activity: &Activity,
        same_sport_activities: &[&Activity],
    ) -> Option<Vec<AdvancedInsight>> {
        let current_hr = activity.average_heart_rate?;
        let current_speed = activity.average_speed?;
        let current_efficiency = current_speed / f64::from(current_hr);

        let historical_efficiencies: Vec<f32> = same_sport_activities
            .iter()
            .filter_map(|a| {
                if let (Some(hr), Some(speed)) = (a.average_heart_rate, a.average_speed) {
                    Some(safe_f64_to_f32(speed / f64::from(hr)))
                } else {
                    None
                }
            })
            .collect();

        if historical_efficiencies.is_empty() {
            return None;
        }

        let avg_efficiency = {
            let len_f32 =
                f32::from(u16::try_from(historical_efficiencies.len()).unwrap_or(u16::MAX));
            historical_efficiencies.iter().sum::<f32>() / len_f32
        };

        let efficiency_change =
            ((current_efficiency - f64::from(avg_efficiency)) / f64::from(avg_efficiency)) * 100.0;

        if efficiency_change > HR_EFFICIENCY_IMPROVEMENT_THRESHOLD {
            let mut metadata = HashMap::new();
            metadata.insert(
                "efficiency_improvement".into(),
                serde_json::Value::from(efficiency_change),
            );

            Some(vec![AdvancedInsight {
                insight_type: "efficiency_improvement".into(),
                message: "Heart rate efficiency improved - getting fitter!".into(),
                confidence: Confidence::Medium,
                severity: InsightSeverity::Info,
                metadata,
            }])
        } else {
            None
        }
    }
}

#[async_trait::async_trait]
impl ActivityAnalyzerTrait for AdvancedActivityAnalyzer {
    /// Analyze a single activity and generate comprehensive insights
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Activity data is invalid or corrupted
    /// - Metrics calculation fails
    /// - Anomaly detection fails
    /// - Data processing errors occur
    async fn analyze_activity(&self, activity: &Activity) -> AppResult<ActivityInsights> {
        // Calculate advanced metrics
        let metrics = self
            .metrics_calculator
            .calculate_metrics(activity)
            .map_err(|e| AppError::internal(format!("Metrics calculation failed: {e}")))?;

        // Generate overall score
        let overall_score = self.calculate_overall_score(activity, &metrics);

        // Generate performance insights
        let insights = Self::generate_performance_insights(activity, &metrics);

        // Generate recommendations
        let recommendations = Self::generate_recommendations(activity, &metrics);

        // Detect anomalies
        let anomalies = self
            .detect_anomalies(activity)
            .await
            .map_err(|e| AppError::internal(format!("Anomaly detection failed: {e}")))?;

        Ok(ActivityInsights {
            activity_id: activity.id.clone(),
            overall_score,
            insights,
            metrics,
            recommendations,
            anomalies,
        })
    }

    /// Detect anomalies in activity data using physiological thresholds
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Activity data is malformed
    /// - Anomaly detection algorithms fail
    /// - Data validation errors occur
    async fn detect_anomalies(&self, activity: &Activity) -> AppResult<Vec<Anomaly>> {
        let mut anomalies = Vec::new();

        // Check for unrealistic heart rate values using physiological limits
        if let Some(max_hr) = activity.max_heart_rate {
            if max_hr > MAX_REALISTIC_HEART_RATE {
                anomalies.push(Anomaly {
                    anomaly_type: "unrealistic_heart_rate".into(),
                    description: "Maximum heart rate seems unusually high".into(),
                    severity: InsightSeverity::Warning,
                    confidence: Confidence::High,
                    affected_metric: "max_heartrate".into(),
                    expected_value: Some(200.0),
                    actual_value: Some(f64::from(max_hr)),
                });
            }
        }

        // Power data not available in current Activity model
        // Skip power anomaly detection

        // Check for unrealistic speed values using sport-specific limits
        if let Some(max_speed) = activity.max_speed {
            let expected_max_speed = match activity.sport_type {
                SportType::Run => MAX_RUNNING_SPEED,
                SportType::Ride => MAX_CYCLING_SPEED,
                SportType::Swim => MAX_SWIMMING_SPEED,
                _ => DEFAULT_MAX_SPEED,
            };

            if max_speed > expected_max_speed {
                anomalies.push(Anomaly {
                    anomaly_type: "unrealistic_speed".into(),
                    description: format!(
                        "Maximum speed seems unusually high for {sport_type:?}",
                        sport_type = activity.sport_type
                    ),
                    severity: InsightSeverity::Warning,
                    confidence: Confidence::Medium,
                    affected_metric: "max_speed".into(),
                    expected_value: Some(expected_max_speed),
                    actual_value: Some(max_speed),
                });
            }
        }

        // Check for missing expected data
        if activity.average_heart_rate.is_none() && activity.sport_type != SportType::Swim {
            anomalies.push(Anomaly {
                anomaly_type: "missing_heart_rate".into(),
                description: "Heart rate data missing - consider using HR monitor".into(),
                severity: InsightSeverity::Info,
                confidence: Confidence::Medium,
                affected_metric: "average_heartrate".into(),
                expected_value: None,
                actual_value: None,
            });
        }

        Ok(anomalies)
    }

    /// Calculate training load based on duration and intensity
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Activity duration data is invalid
    /// - Heart rate data is corrupted
    /// - Training load calculation fails
    /// - Mathematical operations fail
    async fn calculate_training_load(&self, activity: &Activity) -> AppResult<f64> {
        // Calculate training load based on available metrics
        let mut load = 0.0;

        // Base load on duration
        let duration = activity.duration_seconds;
        {
            // Safe conversion for duration to f64
            let duration_f64 = if duration > u64::from(u32::MAX) {
                f64::from(u32::MAX)
            } else {
                f64::from(u32::try_from(duration).unwrap_or(u32::MAX))
            };
            load += duration_f64 / 60.0; // Minutes as base
        }

        // Multiply by intensity factor using heart rate zones
        if let Some(avg_hr) = activity.average_heart_rate {
            let intensity_multiplier = if avg_hr > HIGH_INTENSITY_HR_THRESHOLD {
                2.0
            } else if avg_hr > MODERATE_HR_THRESHOLD {
                1.5
            } else {
                1.0
            };
            load *= intensity_multiplier;
        }

        // Power data not available in current Activity model

        Ok(load)
    }

    /// Compare activity against historical performance data
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Historical activity data is invalid
    /// - Activity comparison calculations fail
    /// - Statistical analysis fails
    /// - Data aggregation errors occur
    async fn compare_to_history(
        &self,
        activity: &Activity,
        historical_activities: &[Activity],
    ) -> AppResult<Vec<AdvancedInsight>> {
        let mut insights = Vec::new();

        // Filter historical activities by sport type
        let same_sport_activities: Vec<_> = historical_activities
            .iter()
            .filter(|a| a.sport_type == activity.sport_type)
            .collect();

        if same_sport_activities.is_empty() {
            return Ok(insights);
        }

        // Compare average speed/pace
        if let Some(pace_insights) =
            Self::compare_pace_performance(activity, &same_sport_activities)
        {
            insights.extend(pace_insights);
        }

        // Compare heart rate efficiency
        if let Some(efficiency_insights) =
            Self::compare_heart_rate_efficiency(activity, &same_sport_activities)
        {
            insights.extend(efficiency_insights);
        }

        Ok(insights)
    }
}
