// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Insight generation and management for athlete intelligence

use serde::{Deserialize, Serialize};
use crate::models::Activity;

/// An insight extracted from activity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    /// Type of insight
    pub insight_type: InsightType,
    
    /// Human-readable insight message
    pub message: String,
    
    /// Confidence level (0-100)
    pub confidence: f32,
    
    /// Supporting data for the insight
    pub data: Option<serde_json::Value>,
}

/// Categories of insights that can be generated
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsightType {
    /// Performance achievement (PR, improvement)
    Achievement,
    
    /// Training zone analysis
    ZoneAnalysis,
    
    /// Effort and recovery insights
    EffortAnalysis,
    
    /// Weather impact analysis
    WeatherImpact,
    
    /// Trend and progression
    TrendAnalysis,
    
    /// Recovery and fatigue
    RecoveryInsight,
    
    /// Goal progression
    GoalProgress,
    
    /// Anomaly detection
    Anomaly,
}

/// Insight generator for creating intelligent analysis
pub struct InsightGenerator {
    /// Configuration for insight generation
    config: InsightConfig,
}

/// Configuration for insight generation
#[derive(Debug, Clone)]
pub struct InsightConfig {
    pub min_confidence_threshold: f32,
    pub max_insights_per_activity: usize,
    #[allow(dead_code)]
    pub enable_weather_analysis: bool,
    #[allow(dead_code)]
    pub enable_trend_analysis: bool,
}

impl Default for InsightConfig {
    fn default() -> Self {
        Self {
            min_confidence_threshold: 70.0,
            max_insights_per_activity: 5,
            enable_weather_analysis: true,
            enable_trend_analysis: true,
        }
    }
}

impl InsightGenerator {
    /// Create a new insight generator with default config
    pub fn new() -> Self {
        Self {
            config: InsightConfig::default(),
        }
    }

    /// Create a new insight generator with custom config
    #[allow(dead_code)]
    pub fn with_config(config: InsightConfig) -> Self {
        Self { config }
    }

    /// Generate insights for a single activity
    pub fn generate_insights(&self, activity: &Activity, context: Option<&ActivityContext>) -> Vec<Insight> {
        let mut insights = Vec::new();

        // Generate different types of insights
        insights.extend(self.generate_achievement_insights(activity));
        insights.extend(self.generate_zone_insights(activity));
        insights.extend(self.generate_effort_insights(activity));
        
        if let Some(ctx) = context {
            insights.extend(self.generate_weather_insights(activity, ctx));
            insights.extend(self.generate_trend_insights(activity, ctx));
        }

        // Filter by confidence and limit count
        insights.retain(|insight| insight.confidence >= self.config.min_confidence_threshold);
        insights.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        insights.truncate(self.config.max_insights_per_activity);

        insights
    }

    /// Generate achievement-related insights
    fn generate_achievement_insights(&self, activity: &Activity) -> Vec<Insight> {
        let mut insights = Vec::new();

        // Example: Distance PR detection
        if let Some(distance_m) = activity.distance_meters {
            let distance_km = distance_m / 1000.0;
            if distance_km > 10.0 { // Arbitrary threshold for demo
                insights.push(Insight {
                    insight_type: InsightType::Achievement,
                    message: format!("Impressive distance! You completed {:.2} km, showing great endurance.", distance_km),
                    confidence: 85.0,
                    data: Some(serde_json::json!({
                        "distance_km": distance_km,
                        "achievement_type": "distance_milestone"
                    })),
                });
            }
        }

        insights
    }

    /// Generate zone analysis insights
    fn generate_zone_insights(&self, activity: &Activity) -> Vec<Insight> {
        let mut insights = Vec::new();

        // Analyze heart rate zones if available
        if let (Some(avg_hr), Some(max_hr)) = (activity.average_heart_rate, activity.max_heart_rate) {
            let hr_intensity = (avg_hr as f32) / (max_hr as f32);
            
            let (zone_description, confidence) = match hr_intensity {
                x if x < 0.6 => ("recovery zone", 90.0),
                x if x < 0.7 => ("endurance zone", 95.0),
                x if x < 0.8 => ("tempo zone", 92.0),
                x if x < 0.9 => ("threshold zone", 88.0),
                _ => ("VO2 max zone", 85.0),
            };

            insights.push(Insight {
                insight_type: InsightType::ZoneAnalysis,
                message: format!("Your average heart rate of {} bpm indicates most time was spent in the {}. This is excellent for building aerobic capacity.",
                               avg_hr, zone_description),
                confidence,
                data: Some(serde_json::json!({
                    "avg_heartrate": avg_hr,
                    "max_heartrate": max_hr,
                    "zone": zone_description,
                    "intensity_ratio": hr_intensity
                })),
            });
        }

        insights
    }

    /// Generate effort analysis insights
    fn generate_effort_insights(&self, activity: &Activity) -> Vec<Insight> {
        let mut insights = Vec::new();

        // Analyze effort based on duration and intensity
        let duration = activity.duration_seconds;
        let effort_score = self.calculate_relative_effort(activity);
        
        let effort_description = match effort_score {
            x if x < 3.0 => ("light", "perfect for recovery"),
            x if x < 5.0 => ("moderate", "good training stimulus"),
            x if x < 7.0 => ("hard", "excellent workout intensity"),
            x if x < 9.0 => ("very hard", "high training load"),
            _ => ("maximum", "peak effort achieved"),
        };

        insights.push(Insight {
            insight_type: InsightType::EffortAnalysis,
            message: format!("With a {} effort level, this {} session was {} for your training goals.",
                           effort_description.0, 
                           Self::format_duration(duration as i32),
                           effort_description.1),
            confidence: 80.0,
            data: Some(serde_json::json!({
                "effort_score": effort_score,
                "duration_seconds": duration,
                "effort_category": effort_description.0
            })),
        });

        insights
    }

    /// Generate weather-related insights
    fn generate_weather_insights(&self, _activity: &Activity, context: &ActivityContext) -> Vec<Insight> {
        let mut insights = Vec::new();

        if let Some(weather) = &context.weather {
            // Example weather impact analysis
            if weather.temperature_celsius < 5.0 {
                insights.push(Insight {
                    insight_type: InsightType::WeatherImpact,
                    message: format!("Cold weather conditions ({:.1}°C) likely made this workout more challenging. Great job adapting to the conditions!",
                                   weather.temperature_celsius),
                    confidence: 75.0,
                    data: Some(serde_json::json!({
                        "temperature": weather.temperature_celsius,
                        "impact": "challenging_conditions"
                    })),
                });
            } else if weather.conditions.contains("rain") {
                insights.push(Insight {
                    insight_type: InsightType::WeatherImpact,
                    message: "Training in rainy conditions shows excellent dedication and mental toughness!".to_string(),
                    confidence: 85.0,
                    data: Some(serde_json::json!({
                        "conditions": weather.conditions,
                        "impact": "mental_toughness"
                    })),
                });
            }
        }

        insights
    }

    /// Generate trend analysis insights
    fn generate_trend_insights(&self, _activity: &Activity, context: &ActivityContext) -> Vec<Insight> {
        let mut insights = Vec::new();

        // Example trend analysis (would normally use historical data)
        if let Some(recent_activities) = &context.recent_activities {
            if recent_activities.len() >= 3 {
                insights.push(Insight {
                    insight_type: InsightType::TrendAnalysis,
                    message: "Your consistency has been excellent this week with multiple quality sessions!".to_string(),
                    confidence: 80.0,
                    data: Some(serde_json::json!({
                        "recent_activity_count": recent_activities.len(),
                        "trend": "consistent_training"
                    })),
                });
            }
        }

        insights
    }

    /// Calculate relative effort score (1-10 scale)
    fn calculate_relative_effort(&self, activity: &Activity) -> f32 {
        let mut effort_score = 1.0;

        // Factor in duration
        let duration = activity.duration_seconds;
        effort_score += (duration as f32 / 3600.0) * 2.0; // +2 per hour

        // Factor in heart rate intensity
        if let (Some(avg_hr), Some(max_hr)) = (activity.average_heart_rate, activity.max_heart_rate) {
            let hr_intensity = (avg_hr as f32) / (max_hr as f32);
            effort_score += hr_intensity * 5.0;
        }

        // Factor in elevation gain
        if let Some(elevation) = activity.elevation_gain {
            effort_score += (elevation / 100.0) as f32 * 0.5; // +0.5 per 100m
        }

        effort_score.min(10.0)
    }

    /// Format duration in human-readable form
    fn format_duration(seconds: i32) -> String {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        
        if hours > 0 {
            format!("{} hour{} {} minute{}", 
                   hours, if hours == 1 { "" } else { "s" },
                   minutes, if minutes == 1 { "" } else { "s" })
        } else {
            format!("{} minute{}", minutes, if minutes == 1 { "" } else { "s" })
        }
    }
}

/// Context information for generating insights
#[derive(Debug, Clone)]
pub struct ActivityContext {
    pub weather: Option<super::WeatherConditions>,
    pub recent_activities: Option<Vec<Activity>>,
    #[allow(dead_code)]
    pub athlete_goals: Option<Vec<String>>,
    #[allow(dead_code)]
    pub historical_data: Option<HistoricalData>,
}

/// Historical performance data for trend analysis
#[derive(Debug, Clone)]
pub struct HistoricalData {
    #[allow(dead_code)]
    pub personal_records: Vec<super::PersonalRecord>,
    #[allow(dead_code)]
    pub average_performance: PerformanceBaseline,
}

/// Baseline performance metrics
#[derive(Debug, Clone)]
pub struct PerformanceBaseline {
    #[allow(dead_code)]
    pub average_pace_per_km: Option<f32>,
    #[allow(dead_code)]
    pub average_heartrate: Option<f32>,
    #[allow(dead_code)]
    pub typical_distance: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Activity, SportType};
    use chrono::Utc;

    fn create_test_activity() -> Activity {
        Activity {
            id: "test123".to_string(),
            name: "Test Run".to_string(),
            sport_type: SportType::Run,
            start_date: Utc::now(),
            duration_seconds: 1800, // 30 minutes
            distance_meters: Some(15000.0), // 15km
            elevation_gain: Some(50.0),
            average_speed: Some(2.78), // 10 km/h
            max_speed: Some(4.17),     // 15 km/h
            provider: "test".to_string(),
            average_heart_rate: Some(150),
            max_heart_rate: Some(180),
            calories: Some(300),
        }
    }

    #[test]
    fn test_insight_generator_creation() {
        let generator = InsightGenerator::new();
        assert_eq!(generator.config.min_confidence_threshold, 70.0);
        assert_eq!(generator.config.max_insights_per_activity, 5);
    }

    #[test]
    fn test_generate_achievement_insights() {
        let generator = InsightGenerator::new();
        let activity = create_test_activity();
        
        let insights = generator.generate_achievement_insights(&activity);
        assert!(!insights.is_empty());
        
        let first_insight = &insights[0];
        assert!(matches!(first_insight.insight_type, InsightType::Achievement));
        assert!(first_insight.confidence > 0.0);
    }

    #[test]
    fn test_generate_zone_insights() {
        let generator = InsightGenerator::new();
        let activity = create_test_activity();
        
        let insights = generator.generate_zone_insights(&activity);
        assert!(!insights.is_empty());
        
        let zone_insight = &insights[0];
        assert!(matches!(zone_insight.insight_type, InsightType::ZoneAnalysis));
        assert!(zone_insight.message.contains("heart rate"));
    }

    #[test]
    fn test_calculate_relative_effort() {
        let generator = InsightGenerator::new();
        let activity = create_test_activity();
        
        let effort = generator.calculate_relative_effort(&activity);
        assert!(effort >= 1.0 && effort <= 10.0);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(InsightGenerator::format_duration(1800), "30 minutes");
        assert_eq!(InsightGenerator::format_duration(3661), "1 hour 1 minute");
        assert_eq!(InsightGenerator::format_duration(7200), "2 hours 0 minutes");
    }
}