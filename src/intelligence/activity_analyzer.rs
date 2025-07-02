// ABOUTME: Detailed activity analysis engine for comprehensive workout breakdowns
// ABOUTME: Analyzes individual activities for pace, power, heart rate patterns and training insights
//! Activity analysis engine for detailed activity insights

use super::*;
use crate::config::intelligence_config::{ActivityAnalyzerConfig, IntelligenceConfig};
use crate::intelligence::physiological_constants::{
    activity_scoring::*, duration::*, efficiency::*, heart_rate::*, max_speeds::*, performance::*,
    power::*, running::*, training_load::*,
};
use crate::models::{Activity, SportType};
use anyhow::Result;
use std::collections::HashMap;

/// Trait for analyzing individual activities
#[async_trait::async_trait]
pub trait ActivityAnalyzerTrait {
    /// Analyze a single activity and generate insights
    async fn analyze_activity(&self, activity: &Activity) -> Result<ActivityInsights>;

    /// Detect anomalies in activity data
    async fn detect_anomalies(&self, activity: &Activity) -> Result<Vec<Anomaly>>;

    /// Calculate training load for an activity
    async fn calculate_training_load(&self, activity: &Activity) -> Result<f64>;

    /// Compare activity against user's historical data
    async fn compare_to_history(
        &self,
        activity: &Activity,
        historical_activities: &[Activity],
    ) -> Result<Vec<AdvancedInsight>>;
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
    pub fn new() -> Self {
        let global_config = IntelligenceConfig::global();
        Self {
            config: global_config.activity_analyzer.clone(),
            metrics_calculator: MetricsCalculator::new(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: ActivityAnalyzerConfig) -> Self {
        Self {
            config,
            metrics_calculator: MetricsCalculator::new(),
        }
    }

    /// Create analyzer with user-specific parameters
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
        &self,
        activity: &Activity,
        metrics: &AdvancedMetrics,
    ) -> Vec<AdvancedInsight> {
        let mut insights = Vec::new();

        // Heart rate insights
        if let Some(avg_hr) = activity.average_heart_rate {
            if let Some(max_hr) = activity.max_heart_rate {
                let hr_reserve_used = (avg_hr as f32 / max_hr as f32) * 100.0;

                let (message, confidence) = if hr_reserve_used > ANAEROBIC_THRESHOLD_PERCENTAGE {
                    (
                        "High intensity effort - excellent cardiovascular challenge".to_string(),
                        Confidence::High,
                    )
                } else if hr_reserve_used > AEROBIC_THRESHOLD_PERCENTAGE {
                    (
                        "Moderate to high intensity - good aerobic stimulus".to_string(),
                        Confidence::Medium,
                    )
                } else {
                    (
                        "Low to moderate intensity - great for base building".to_string(),
                        Confidence::Medium,
                    )
                };

                let mut metadata = HashMap::new();
                metadata.insert(
                    "hr_reserve_percentage".to_string(),
                    serde_json::Value::from(hr_reserve_used),
                );

                insights.push(AdvancedInsight {
                    insight_type: "heart_rate_analysis".to_string(),
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
                    "Excellent power-to-weight ratio - elite level performance".to_string(),
                    InsightSeverity::Info,
                )
            } else if power_to_weight > COMPETITIVE_POWER_TO_WEIGHT {
                (
                    "Good power-to-weight ratio - competitive level".to_string(),
                    InsightSeverity::Info,
                )
            } else if power_to_weight > RECREATIONAL_POWER_TO_WEIGHT {
                (
                    "Moderate power-to-weight ratio - room for improvement".to_string(),
                    InsightSeverity::Warning,
                )
            } else {
                (
                    "Consider power training to improve performance".to_string(),
                    InsightSeverity::Warning,
                )
            };

            let mut metadata = HashMap::new();
            metadata.insert(
                "power_to_weight_ratio".to_string(),
                serde_json::Value::from(power_to_weight),
            );

            insights.push(AdvancedInsight {
                insight_type: "power_analysis".to_string(),
                message,
                confidence: Confidence::High,
                severity,
                metadata,
            });
        }

        // Efficiency insights using research-based thresholds
        if let Some(efficiency) = metrics.aerobic_efficiency {
            let message = if efficiency > EXCELLENT_AEROBIC_EFFICIENCY {
                "Excellent aerobic efficiency - well-conditioned cardiovascular system".to_string()
            } else if efficiency > GOOD_AEROBIC_EFFICIENCY {
                "Good aerobic efficiency - steady cardiovascular fitness".to_string()
            } else {
                "Consider base training to improve aerobic efficiency".to_string()
            };

            let mut metadata = HashMap::new();
            metadata.insert(
                "aerobic_efficiency".to_string(),
                serde_json::Value::from(efficiency),
            );

            insights.push(AdvancedInsight {
                insight_type: "efficiency_analysis".to_string(),
                message,
                confidence: Confidence::Medium,
                severity: InsightSeverity::Info,
                metadata,
            });
        }

        insights
    }

    /// Generate training recommendations based on activity
    fn generate_recommendations(
        &self,
        activity: &Activity,
        metrics: &AdvancedMetrics,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Duration-based recommendations using physiological thresholds
        let duration = activity.duration_seconds;
        if duration < MIN_AEROBIC_DURATION {
            recommendations.push(
                "Consider extending workout duration for better aerobic benefits".to_string(),
            );
        } else if duration > LONG_WORKOUT_DURATION {
            recommendations
                .push("Great endurance work! Ensure proper recovery and nutrition".to_string());
        }

        // Heart rate based recommendations using established zones
        if let Some(avg_hr) = activity.average_heart_rate {
            if avg_hr < RECOVERY_HR_THRESHOLD {
                recommendations.push(
                    "Consider increasing intensity for better cardiovascular stimulus".to_string(),
                );
            } else if avg_hr > VERY_HIGH_INTENSITY_HR_THRESHOLD {
                recommendations
                    .push("High intensity session - ensure adequate recovery time".to_string());
            }
        }

        // Training stress recommendations using established thresholds
        if let Some(tss) = metrics.training_stress_score {
            if tss > HIGH_TSS_THRESHOLD {
                recommendations.push(
                    "High training stress - plan recovery days to avoid overtraining".to_string(),
                );
            } else if tss < LOW_TSS_THRESHOLD {
                recommendations
                    .push("Light training load - good for recovery or base building".to_string());
            }
        }

        // Sport-specific recommendations
        match activity.sport_type {
            SportType::Run => {
                if let Some(pace) = activity.average_speed {
                    if pace > FAST_PACE_THRESHOLD {
                        recommendations
                            .push("Excellent pace! Focus on maintaining form at speed".to_string());
                    }
                }
                recommendations.push(
                    "Consider incorporating strength training for injury prevention".to_string(),
                );
            }
            SportType::Ride => {
                // Power data not available in current Activity model
                recommendations
                    .push("Remember bike maintenance for optimal performance".to_string());
            }
            SportType::Swim => {
                recommendations
                    .push("Focus on stroke technique and breathing efficiency".to_string());
            }
            _ => {}
        }

        recommendations
    }
}

#[async_trait::async_trait]
impl ActivityAnalyzerTrait for AdvancedActivityAnalyzer {
    async fn analyze_activity(&self, activity: &Activity) -> Result<ActivityInsights> {
        // Calculate advanced metrics
        let metrics = self.metrics_calculator.calculate_metrics(activity)?;

        // Generate overall score
        let overall_score = self.calculate_overall_score(activity, &metrics);

        // Generate performance insights
        let insights = self.generate_performance_insights(activity, &metrics);

        // Generate recommendations
        let recommendations = self.generate_recommendations(activity, &metrics);

        // Detect anomalies
        let anomalies = self.detect_anomalies(activity).await?;

        Ok(ActivityInsights {
            activity_id: activity.id.clone(),
            overall_score,
            insights,
            metrics,
            recommendations,
            anomalies,
        })
    }

    async fn detect_anomalies(&self, activity: &Activity) -> Result<Vec<Anomaly>> {
        let mut anomalies = Vec::new();

        // Check for unrealistic heart rate values using physiological limits
        if let Some(max_hr) = activity.max_heart_rate {
            if max_hr > MAX_REALISTIC_HEART_RATE {
                anomalies.push(Anomaly {
                    anomaly_type: "unrealistic_heart_rate".to_string(),
                    description: "Maximum heart rate seems unusually high".to_string(),
                    severity: InsightSeverity::Warning,
                    confidence: Confidence::High,
                    affected_metric: "max_heartrate".to_string(),
                    expected_value: Some(200.0),
                    actual_value: Some(max_hr as f64),
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
                    anomaly_type: "unrealistic_speed".to_string(),
                    description: format!(
                        "Maximum speed seems unusually high for {:?}",
                        activity.sport_type
                    ),
                    severity: InsightSeverity::Warning,
                    confidence: Confidence::Medium,
                    affected_metric: "max_speed".to_string(),
                    expected_value: Some(expected_max_speed),
                    actual_value: Some(max_speed),
                });
            }
        }

        // Check for missing expected data
        if activity.average_heart_rate.is_none() && activity.sport_type != SportType::Swim {
            anomalies.push(Anomaly {
                anomaly_type: "missing_heart_rate".to_string(),
                description: "Heart rate data missing - consider using HR monitor".to_string(),
                severity: InsightSeverity::Info,
                confidence: Confidence::Medium,
                affected_metric: "average_heartrate".to_string(),
                expected_value: None,
                actual_value: None,
            });
        }

        Ok(anomalies)
    }

    async fn calculate_training_load(&self, activity: &Activity) -> Result<f64> {
        // Calculate training load based on available metrics
        let mut load = 0.0;

        // Base load on duration
        let duration = activity.duration_seconds;
        load += duration as f64 / 60.0; // Minutes as base

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

    async fn compare_to_history(
        &self,
        activity: &Activity,
        historical_activities: &[Activity],
    ) -> Result<Vec<AdvancedInsight>> {
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
        if let Some(current_speed) = activity.average_speed {
            let historical_speeds: Vec<f32> = same_sport_activities
                .iter()
                .filter_map(|a| a.average_speed.map(|s| s as f32))
                .collect();

            if !historical_speeds.is_empty() {
                let avg_historical_speed =
                    historical_speeds.iter().sum::<f32>() / historical_speeds.len() as f32;
                let improvement =
                    ((current_speed as f32 - avg_historical_speed) / avg_historical_speed) * 100.0;

                if improvement > PACE_IMPROVEMENT_THRESHOLD as f32 {
                    let mut metadata = HashMap::new();
                    metadata.insert(
                        "improvement_percentage".to_string(),
                        serde_json::Value::from(improvement),
                    );
                    metadata.insert(
                        "current_speed".to_string(),
                        serde_json::Value::from(current_speed),
                    );
                    metadata.insert(
                        "historical_average".to_string(),
                        serde_json::Value::from(avg_historical_speed),
                    );

                    insights.push(AdvancedInsight {
                        insight_type: "pace_improvement".to_string(),
                        message: format!(
                            "Pace improved by {:.1}% compared to recent activities",
                            improvement
                        ),
                        confidence: Confidence::High,
                        severity: InsightSeverity::Info,
                        metadata,
                    });
                } else if improvement < -(PACE_IMPROVEMENT_THRESHOLD as f32) {
                    let mut metadata = HashMap::new();
                    metadata.insert(
                        "decline_percentage".to_string(),
                        serde_json::Value::from(-improvement),
                    );

                    insights.push(AdvancedInsight {
                        insight_type: "pace_decline".to_string(),
                        message: format!(
                            "Pace was {:.1}% slower than recent average",
                            -improvement
                        ),
                        confidence: Confidence::Medium,
                        severity: InsightSeverity::Warning,
                        metadata,
                    });
                }
            }
        }

        // Compare heart rate efficiency
        if let (Some(current_hr), Some(current_speed)) =
            (activity.average_heart_rate, activity.average_speed)
        {
            let current_efficiency = current_speed / current_hr as f64;

            let historical_efficiencies: Vec<f32> = same_sport_activities
                .iter()
                .filter_map(|a| {
                    if let (Some(hr), Some(speed)) = (a.average_heart_rate, a.average_speed) {
                        Some((speed / hr as f64) as f32)
                    } else {
                        None
                    }
                })
                .collect();

            if !historical_efficiencies.is_empty() {
                let avg_efficiency = historical_efficiencies.iter().sum::<f32>()
                    / historical_efficiencies.len() as f32;
                let efficiency_change =
                    ((current_efficiency - avg_efficiency as f64) / avg_efficiency as f64) * 100.0;

                if efficiency_change > HR_EFFICIENCY_IMPROVEMENT_THRESHOLD {
                    let mut metadata = HashMap::new();
                    metadata.insert(
                        "efficiency_improvement".to_string(),
                        serde_json::Value::from(efficiency_change),
                    );

                    insights.push(AdvancedInsight {
                        insight_type: "efficiency_improvement".to_string(),
                        message: "Heart rate efficiency improved - getting fitter!".to_string(),
                        confidence: Confidence::Medium,
                        severity: InsightSeverity::Info,
                        metadata,
                    });
                }
            }
        }

        Ok(insights)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_activity_analysis() {
        let analyzer = AdvancedActivityAnalyzer::new();

        let activity = Activity {
            sport_type: crate::models::SportType::Run,
            distance_meters: Some(5000.0), // 5km
            duration_seconds: 1800,        // 30 minutes
            average_heart_rate: Some(150),
            average_speed: Some(2.78), // ~6 min/km pace
            ..Activity::default()
        };

        let result = analyzer.analyze_activity(&activity).await;
        assert!(result.is_ok());

        let insights = result.unwrap();
        assert!(insights.overall_score > 0.0);
        assert!(!insights.insights.is_empty());
    }

    #[tokio::test]
    async fn test_anomaly_detection() {
        let analyzer = AdvancedActivityAnalyzer::new();

        let activity = Activity {
            max_heart_rate: Some(250), // Unrealistic HR
            ..Activity::default()
        };
        // Power data not available in current Activity model - skip test

        let anomalies = analyzer.detect_anomalies(&activity).await.unwrap();
        assert_eq!(anomalies.len(), 1); // Should detect HR anomaly
    }
}
