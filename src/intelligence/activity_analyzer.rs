//! Activity analysis engine for detailed activity insights

use super::*;
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
    async fn compare_to_history(&self, activity: &Activity, historical_activities: &[Activity]) -> Result<Vec<AdvancedInsight>>;
}

/// Advanced activity analyzer implementation
pub struct AdvancedActivityAnalyzer {
    metrics_calculator: MetricsCalculator,
}

impl AdvancedActivityAnalyzer {
    /// Create a new activity analyzer
    pub fn new() -> Self {
        Self {
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
        Self {
            metrics_calculator: MetricsCalculator::new()
                .with_user_data(ftp, lthr, max_hr, resting_hr, weight_kg),
        }
    }

    /// Generate overall activity score (0-10)
    fn calculate_overall_score(&self, activity: &Activity, metrics: &AdvancedMetrics) -> f64 {
        let mut score: f64 = 5.0; // Base score
        
        // Adjust based on completion
        if activity.distance_meters.unwrap_or(0.0) > 0.0 {
            score += 1.0;
        }
        
        // Adjust based on effort level
        if let Some(avg_hr) = activity.average_heart_rate {
            if avg_hr > 140 {
                score += 1.0;
            }
            if avg_hr > 160 {
                score += 0.5;
            }
        }
        
        // Adjust based on metrics quality
        if metrics.trimp.is_some() {
            score += 0.5;
        }
        if metrics.power_to_weight_ratio.is_some() {
            score += 0.5;
        }
        
        // Adjust based on duration
        let duration = activity.duration_seconds;
        if duration > 1800 { // 30+ minutes
            score += 0.5;
        }
        if duration > 3600 { // 60+ minutes
            score += 0.5;
        }
        
        score.min(10.0).max(0.0)
    }

    /// Generate insights for activity performance
    fn generate_performance_insights(&self, activity: &Activity, metrics: &AdvancedMetrics) -> Vec<AdvancedInsight> {
        let mut insights = Vec::new();
        
        // Heart rate insights
        if let Some(avg_hr) = activity.average_heart_rate {
            if let Some(max_hr) = activity.max_heart_rate {
                let hr_reserve_used = (avg_hr as f32 / max_hr as f32) * 100.0;
                
                let (message, confidence) = if hr_reserve_used > 85.0 {
                    ("High intensity effort - excellent cardiovascular challenge".to_string(), Confidence::High)
                } else if hr_reserve_used > 70.0 {
                    ("Moderate to high intensity - good aerobic stimulus".to_string(), Confidence::Medium)
                } else {
                    ("Low to moderate intensity - great for base building".to_string(), Confidence::Medium)
                };
                
                let mut metadata = HashMap::new();
                metadata.insert("hr_reserve_percentage".to_string(), serde_json::Value::from(hr_reserve_used));
                
                insights.push(AdvancedInsight {
                    insight_type: "heart_rate_analysis".to_string(),
                    message,
                    confidence,
                    severity: InsightSeverity::Info,
                    metadata,
                });
            }
        }
        
        // Power insights
        if let Some(power_to_weight) = metrics.power_to_weight_ratio {
            let (message, severity) = if power_to_weight > 4.0 {
                ("Excellent power-to-weight ratio - elite level performance".to_string(), InsightSeverity::Info)
            } else if power_to_weight > 3.0 {
                ("Good power-to-weight ratio - competitive level".to_string(), InsightSeverity::Info)
            } else if power_to_weight > 2.0 {
                ("Moderate power-to-weight ratio - room for improvement".to_string(), InsightSeverity::Warning)
            } else {
                ("Consider power training to improve performance".to_string(), InsightSeverity::Warning)
            };
            
            let mut metadata = HashMap::new();
            metadata.insert("power_to_weight_ratio".to_string(), serde_json::Value::from(power_to_weight));
            
            insights.push(AdvancedInsight {
                insight_type: "power_analysis".to_string(),
                message,
                confidence: Confidence::High,
                severity,
                metadata,
            });
        }
        
        // Efficiency insights
        if let Some(efficiency) = metrics.aerobic_efficiency {
            let message = if efficiency > 0.1 {
                "Excellent aerobic efficiency - well-conditioned cardiovascular system".to_string()
            } else if efficiency > 0.08 {
                "Good aerobic efficiency - steady cardiovascular fitness".to_string()
            } else {
                "Consider base training to improve aerobic efficiency".to_string()
            };
            
            let mut metadata = HashMap::new();
            metadata.insert("aerobic_efficiency".to_string(), serde_json::Value::from(efficiency));
            
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
    fn generate_recommendations(&self, activity: &Activity, metrics: &AdvancedMetrics) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        // Duration-based recommendations
        let duration = activity.duration_seconds;
        if duration < 1800 { // Less than 30 minutes
            recommendations.push("Consider extending workout duration for better aerobic benefits".to_string());
        } else if duration > 7200 { // More than 2 hours
            recommendations.push("Great endurance work! Ensure proper recovery and nutrition".to_string());
        }
        
        // Heart rate based recommendations
        if let Some(avg_hr) = activity.average_heart_rate {
            if avg_hr < 120 {
                recommendations.push("Consider increasing intensity for better cardiovascular stimulus".to_string());
            } else if avg_hr > 180 {
                recommendations.push("High intensity session - ensure adequate recovery time".to_string());
            }
        }
        
        // Power-based recommendations
        if let Some(tss) = metrics.training_stress_score {
            if tss > 150.0 {
                recommendations.push("High training stress - plan recovery days to avoid overtraining".to_string());
            } else if tss < 50.0 {
                recommendations.push("Light training load - good for recovery or base building".to_string());
            }
        }
        
        // Sport-specific recommendations
        match activity.sport_type {
            SportType::Run => {
                if let Some(pace) = activity.average_speed {
                    if pace > 4.0 { // ~4 m/s = 6:40 min/mile
                        recommendations.push("Excellent pace! Focus on maintaining form at speed".to_string());
                    }
                }
                recommendations.push("Consider incorporating strength training for injury prevention".to_string());
            },
            SportType::Ride => {
                // Power data not available in current Activity model
                recommendations.push("Remember bike maintenance for optimal performance".to_string());
            },
            SportType::Swim => {
                recommendations.push("Focus on stroke technique and breathing efficiency".to_string());
            },
            _ => {},
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
        
        // Check for unrealistic heart rate values
        if let Some(max_hr) = activity.max_heart_rate {
            if max_hr > 220 {
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
        
        // Check for unrealistic speed values
        if let Some(max_speed) = activity.max_speed {
            let expected_max_speed = match activity.sport_type {
                SportType::Run => 12.0, // ~43 km/h
                SportType::Ride => 25.0, // ~90 km/h  
                SportType::Swim => 3.0,  // ~11 km/h
                _ => 30.0,
            };
            
            if max_speed > expected_max_speed {
                anomalies.push(Anomaly {
                    anomaly_type: "unrealistic_speed".to_string(),
                    description: format!("Maximum speed seems unusually high for {:?}", activity.sport_type),
                    severity: InsightSeverity::Warning,
                    confidence: Confidence::Medium,
                    affected_metric: "max_speed".to_string(),
                    expected_value: Some(expected_max_speed),
                    actual_value: Some(max_speed as f64),
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
        
        // Multiply by intensity factor
        if let Some(avg_hr) = activity.average_heart_rate {
            let intensity_multiplier = if avg_hr > 160 {
                2.0
            } else if avg_hr > 140 {
                1.5
            } else {
                1.0
            };
            load *= intensity_multiplier;
        }
        
        // Power data not available in current Activity model
        
        Ok(load)
    }
    
    async fn compare_to_history(&self, activity: &Activity, historical_activities: &[Activity]) -> Result<Vec<AdvancedInsight>> {
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
                .filter_map(|a| a.average_speed)
                .collect();
            
            if !historical_speeds.is_empty() {
                let avg_historical_speed = historical_speeds.iter().sum::<f32>() / historical_speeds.len() as f32;
                let improvement = ((current_speed - avg_historical_speed) / avg_historical_speed) * 100.0;
                
                if improvement > 5.0 {
                    let mut metadata = HashMap::new();
                    metadata.insert("improvement_percentage".to_string(), serde_json::Value::from(improvement));
                    metadata.insert("current_speed".to_string(), serde_json::Value::from(current_speed));
                    metadata.insert("historical_average".to_string(), serde_json::Value::from(avg_historical_speed));
                    
                    insights.push(AdvancedInsight {
                        insight_type: "pace_improvement".to_string(),
                        message: format!("Pace improved by {:.1}% compared to recent activities", improvement),
                        confidence: Confidence::High,
                        severity: InsightSeverity::Info,
                        metadata,
                    });
                } else if improvement < -5.0 {
                    let mut metadata = HashMap::new();
                    metadata.insert("decline_percentage".to_string(), serde_json::Value::from(-improvement));
                    
                    insights.push(AdvancedInsight {
                        insight_type: "pace_decline".to_string(),
                        message: format!("Pace was {:.1}% slower than recent average", -improvement),
                        confidence: Confidence::Medium,
                        severity: InsightSeverity::Warning,
                        metadata,
                    });
                }
            }
        }
        
        // Compare heart rate efficiency
        if let (Some(current_hr), Some(current_speed)) = (activity.average_heartrate, activity.average_speed) {
            let current_efficiency = current_speed / current_hr;
            
            let historical_efficiencies: Vec<f32> = same_sport_activities
                .iter()
                .filter_map(|a| {
                    if let (Some(hr), Some(speed)) = (a.average_heartrate, a.average_speed) {
                        Some(speed / hr)
                    } else {
                        None
                    }
                })
                .collect();
            
            if !historical_efficiencies.is_empty() {
                let avg_efficiency = historical_efficiencies.iter().sum::<f32>() / historical_efficiencies.len() as f32;
                let efficiency_change = ((current_efficiency - avg_efficiency) / avg_efficiency) * 100.0;
                
                if efficiency_change > 3.0 {
                    let mut metadata = HashMap::new();
                    metadata.insert("efficiency_improvement".to_string(), serde_json::Value::from(efficiency_change));
                    
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
        
        let mut activity = Activity::default();
        activity.sport_type = "Run".to_string();
        activity.distance = Some(5000.0); // 5km
        activity.moving_time = Some(1800); // 30 minutes
        activity.average_heartrate = Some(150.0);
        activity.average_speed = Some(2.78); // ~6 min/km pace
        
        let result = analyzer.analyze_activity(&activity).await;
        assert!(result.is_ok());
        
        let insights = result.unwrap();
        assert!(insights.overall_score > 0.0);
        assert!(!insights.insights.is_empty());
    }

    #[tokio::test]
    async fn test_anomaly_detection() {
        let analyzer = AdvancedActivityAnalyzer::new();
        
        let mut activity = Activity::default();
        activity.max_heartrate = Some(250.0); // Unrealistic HR
        activity.max_watts = Some(3000.0); // Unrealistic power
        
        let anomalies = analyzer.detect_anomalies(&activity).await.unwrap();
        assert_eq!(anomalies.len(), 2); // Should detect both anomalies
    }
}