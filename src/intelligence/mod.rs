// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Athlete Intelligence Module
//! 
//! This module provides intelligent analysis and insights for fitness activities,
//! generating natural language summaries and detecting patterns in athletic performance.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub mod analyzer;
pub mod insights;
pub mod weather;

pub use analyzer::ActivityAnalyzer;
pub use insights::Insight;

/// Activity intelligence summary with insights and analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityIntelligence {
    /// Natural language summary of the activity
    pub summary: String,
    
    /// Key insights extracted from the activity
    pub key_insights: Vec<Insight>,
    
    /// Performance metrics and indicators
    pub performance_indicators: PerformanceMetrics,
    
    /// Contextual factors affecting the activity
    pub contextual_factors: ContextualFactors,
    
    /// Timestamp when the analysis was generated
    pub generated_at: DateTime<Utc>,
}

/// Performance metrics derived from activity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Relative effort (1-10 scale)
    pub relative_effort: Option<f32>,
    
    /// Zone distribution (percentage in each zone)
    pub zone_distribution: Option<ZoneDistribution>,
    
    /// Personal records achieved
    pub personal_records: Vec<PersonalRecord>,
    
    /// Efficiency score (0-100)
    pub efficiency_score: Option<f32>,
    
    /// Comparison with recent activities
    pub trend_indicators: TrendIndicators,
}

/// Heart rate or power zone distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneDistribution {
    pub zone1_recovery: f32,
    pub zone2_endurance: f32,
    pub zone3_tempo: f32,
    pub zone4_threshold: f32,
    pub zone5_vo2max: f32,
}

/// Personal record information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalRecord {
    pub record_type: String,
    pub value: f64,
    pub unit: String,
    pub previous_best: Option<f64>,
    pub improvement_percentage: Option<f32>,
}

/// Trend indicators comparing to recent activities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendIndicators {
    pub pace_trend: TrendDirection,
    pub effort_trend: TrendDirection,
    pub distance_trend: TrendDirection,
    pub consistency_score: f32, // 0-100
}

/// Direction of a trend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrendDirection {
    Improving,
    Stable,
    Declining,
}

/// Contextual factors that might affect performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualFactors {
    pub weather: Option<WeatherConditions>,
    pub time_of_day: TimeOfDay,
    pub days_since_last_activity: Option<i32>,
    pub weekly_load: Option<WeeklyLoad>,
}

/// Weather conditions during activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherConditions {
    pub temperature_celsius: f32,
    pub humidity_percentage: Option<f32>,
    pub wind_speed_kmh: Option<f32>,
    pub conditions: String, // "sunny", "rainy", "cloudy", etc.
}

/// Time of day categorization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeOfDay {
    EarlyMorning, // 5-7 AM
    Morning,      // 7-11 AM
    Midday,       // 11 AM - 2 PM
    Afternoon,    // 2-6 PM
    Evening,      // 6-9 PM
    Night,        // 9 PM - 5 AM
}

/// Weekly training load summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyLoad {
    pub total_distance_km: f64,
    pub total_duration_hours: f64,
    pub activity_count: i32,
    pub load_trend: TrendDirection,
}

impl ActivityIntelligence {
    /// Create a new activity intelligence analysis
    pub fn new(
        summary: String,
        insights: Vec<Insight>,
        performance: PerformanceMetrics,
        context: ContextualFactors,
    ) -> Self {
        Self {
            summary,
            key_insights: insights,
            performance_indicators: performance,
            contextual_factors: context,
            generated_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_intelligence_creation() {
        let intelligence = ActivityIntelligence::new(
            "Great morning run!".to_string(),
            vec![],
            PerformanceMetrics {
                relative_effort: Some(5.0),
                zone_distribution: None,
                personal_records: vec![],
                efficiency_score: Some(85.0),
                trend_indicators: TrendIndicators {
                    pace_trend: TrendDirection::Improving,
                    effort_trend: TrendDirection::Stable,
                    distance_trend: TrendDirection::Stable,
                    consistency_score: 90.0,
                },
            },
            ContextualFactors {
                weather: None,
                time_of_day: TimeOfDay::Morning,
                days_since_last_activity: Some(1),
                weekly_load: None,
            },
        );

        assert_eq!(intelligence.summary, "Great morning run!");
        assert_eq!(intelligence.performance_indicators.relative_effort, Some(5.0));
    }

    #[test]
    fn test_zone_distribution() {
        let zones = ZoneDistribution {
            zone1_recovery: 10.0,
            zone2_endurance: 65.0,
            zone3_tempo: 20.0,
            zone4_threshold: 5.0,
            zone5_vo2max: 0.0,
        };

        assert_eq!(zones.zone2_endurance, 65.0);
        
        // Total should be 100%
        let total = zones.zone1_recovery + zones.zone2_endurance + 
                   zones.zone3_tempo + zones.zone4_threshold + zones.zone5_vo2max;
        assert_eq!(total, 100.0);
    }
}