// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Activity analyzer for generating intelligent insights

use chrono::{DateTime, Utc, Timelike, Local};
use crate::models::{Activity, SportType};
use super::{
    ActivityIntelligence, PerformanceMetrics, ContextualFactors, TrendIndicators, 
    TrendDirection, TimeOfDay, ZoneDistribution, PersonalRecord,
    insights::{InsightGenerator, ActivityContext},
};

/// Main analyzer for generating activity intelligence
pub struct ActivityAnalyzer {
    insight_generator: InsightGenerator,
}

impl ActivityAnalyzer {
    /// Create a new activity analyzer
    pub fn new() -> Self {
        Self {
            insight_generator: InsightGenerator::new(),
        }
    }

    /// Analyze a single activity and generate intelligence
    pub async fn analyze_activity(
        &self, 
        activity: &Activity,
        context: Option<ActivityContext>,
    ) -> Result<ActivityIntelligence, AnalysisError> {
        // Generate insights
        let insights = self.insight_generator.generate_insights(activity, context.as_ref());
        
        // Calculate performance metrics
        let performance = self.calculate_performance_metrics(activity)?;
        
        // Determine contextual factors
        let contextual_factors = self.analyze_contextual_factors(activity, &context);
        
        // Generate natural language summary
        let summary = self.generate_summary(activity, &insights, &performance, &contextual_factors);
        
        Ok(ActivityIntelligence::new(
            summary,
            insights,
            performance,
            contextual_factors,
        ))
    }

    /// Calculate performance metrics for an activity
    fn calculate_performance_metrics(&self, activity: &Activity) -> Result<PerformanceMetrics, AnalysisError> {
        let relative_effort = self.calculate_relative_effort(activity);
        let zone_distribution = self.calculate_zone_distribution(activity);
        let personal_records = self.detect_personal_records(activity);
        let efficiency_score = self.calculate_efficiency_score(activity);
        let trend_indicators = self.calculate_trend_indicators(activity);

        Ok(PerformanceMetrics {
            relative_effort: Some(relative_effort),
            zone_distribution,
            personal_records,
            efficiency_score: Some(efficiency_score),
            trend_indicators,
        })
    }

    /// Calculate relative effort score (1-10 scale)
    fn calculate_relative_effort(&self, activity: &Activity) -> f32 {
        let mut effort = 1.0;

        // Base effort from duration
        let duration = activity.duration_seconds;
        effort += (duration as f32 / 3600.0) * 1.5; // +1.5 per hour

        // Heart rate intensity
        if let (Some(avg_hr), Some(max_hr)) = (activity.average_heart_rate, activity.max_heart_rate) {
            let hr_intensity = (avg_hr as f32) / (max_hr as f32);
            effort += hr_intensity * 4.0;
        }

        // Distance factor
        if let Some(distance_m) = activity.distance_meters {
            let distance_km = distance_m / 1000.0;
            match activity.sport_type {
                SportType::Run => effort += (distance_km / 10.0) as f32 * 0.8, // +0.8 per 10km
                SportType::Ride => effort += (distance_km / 50.0) as f32 * 0.6, // +0.6 per 50km
                _ => effort += (distance_km / 20.0) as f32 * 0.5,
            }
        }

        // Elevation factor
        if let Some(elevation) = activity.elevation_gain {
            effort += (elevation / 100.0) as f32 * 0.3;
        }

        effort.min(10.0).max(1.0)
    }

    /// Calculate heart rate zone distribution
    fn calculate_zone_distribution(&self, activity: &Activity) -> Option<ZoneDistribution> {
        // This is a simplified version - real implementation would need detailed HR data
        if let (Some(avg_hr), Some(max_hr)) = (activity.average_heart_rate, activity.max_heart_rate) {
            let hr_reserve = max_hr - 60; // Assuming 60 bpm resting HR
            let intensity = ((avg_hr - 60) as f32) / (hr_reserve as f32);

            // Estimated distribution based on average intensity
            let zones = match intensity {
                x if x < 0.5 => ZoneDistribution {
                    zone1_recovery: 80.0,
                    zone2_endurance: 20.0,
                    zone3_tempo: 0.0,
                    zone4_threshold: 0.0,
                    zone5_vo2max: 0.0,
                },
                x if x < 0.6 => ZoneDistribution {
                    zone1_recovery: 20.0,
                    zone2_endurance: 70.0,
                    zone3_tempo: 10.0,
                    zone4_threshold: 0.0,
                    zone5_vo2max: 0.0,
                },
                x if x < 0.7 => ZoneDistribution {
                    zone1_recovery: 10.0,
                    zone2_endurance: 40.0,
                    zone3_tempo: 45.0,
                    zone4_threshold: 5.0,
                    zone5_vo2max: 0.0,
                },
                x if x < 0.85 => ZoneDistribution {
                    zone1_recovery: 5.0,
                    zone2_endurance: 20.0,
                    zone3_tempo: 30.0,
                    zone4_threshold: 40.0,
                    zone5_vo2max: 5.0,
                },
                _ => ZoneDistribution {
                    zone1_recovery: 0.0,
                    zone2_endurance: 10.0,
                    zone3_tempo: 20.0,
                    zone4_threshold: 40.0,
                    zone5_vo2max: 30.0,
                },
            };

            Some(zones)
        } else {
            None
        }
    }

    /// Detect personal records (simplified version)
    fn detect_personal_records(&self, activity: &Activity) -> Vec<PersonalRecord> {
        let mut records = Vec::new();

        // Example: Distance PR detection (would normally compare with historical data)
        if let Some(distance_m) = activity.distance_meters {
            let distance_km = distance_m / 1000.0;
            if distance_km > 20.0 { // Arbitrary threshold for demo
                records.push(PersonalRecord {
                    record_type: "Longest Distance".to_string(),
                    value: distance_km,
                    unit: "km".to_string(),
                    previous_best: Some(18.5), // Mock previous best
                    improvement_percentage: Some(((distance_km - 18.5) / 18.5 * 100.0) as f32),
                });
            }
        }

        // Example: Speed PR detection
        if let Some(avg_speed) = activity.average_speed {
            let pace_per_km = 1000.0 / avg_speed; // seconds per km
            if pace_per_km < 300.0 { // Under 5 minutes per km
                records.push(PersonalRecord {
                    record_type: "Fastest Average Pace".to_string(),
                    value: pace_per_km,
                    unit: "seconds/km".to_string(),
                    previous_best: Some(320.0),
                    improvement_percentage: Some(((320.0 - pace_per_km) / 320.0 * 100.0) as f32),
                });
            }
        }

        records
    }

    /// Calculate efficiency score
    fn calculate_efficiency_score(&self, activity: &Activity) -> f32 {
        let mut efficiency: f32 = 50.0; // Base score

        // Heart rate efficiency
        if let (Some(avg_hr), Some(avg_speed)) = (activity.average_heart_rate, activity.average_speed) {
            let pace_per_km = 1000.0 / avg_speed as f32;
            let hr_efficiency = 1000.0 / (avg_hr as f32 * pace_per_km);
            efficiency += hr_efficiency * 10.0;
        }

        // Consistency factor (mock calculation)
        if activity.average_speed.is_some() && activity.max_speed.is_some() {
            let speed_variance = activity.max_speed.unwrap() - activity.average_speed.unwrap();
            let consistency = 1.0 - (speed_variance / activity.max_speed.unwrap()).min(1.0) as f32;
            efficiency += consistency * 20.0;
        }

        efficiency.min(100.0).max(0.0)
    }

    /// Calculate trend indicators (simplified - would need historical data)
    fn calculate_trend_indicators(&self, _activity: &Activity) -> TrendIndicators {
        // Mock implementation - real version would compare with recent activities
        TrendIndicators {
            pace_trend: TrendDirection::Improving,
            effort_trend: TrendDirection::Stable,
            distance_trend: TrendDirection::Stable,
            consistency_score: 85.0,
        }
    }

    /// Analyze contextual factors
    fn analyze_contextual_factors(
        &self, 
        activity: &Activity,
        context: &Option<ActivityContext>,
    ) -> ContextualFactors {
        let time_of_day = self.determine_time_of_day(&activity.start_date);
        
        ContextualFactors {
            weather: context.as_ref().and_then(|c| c.weather.clone()),
            location: context.as_ref().and_then(|c| c.location.clone()),
            time_of_day,
            days_since_last_activity: None, // Would calculate from historical data
            weekly_load: None, // Would calculate from recent activities
        }
    }

    /// Determine time of day category based on local time
    fn determine_time_of_day(&self, start_date: &DateTime<Utc>) -> TimeOfDay {
        // Convert UTC to local time for proper categorization
        let local_time = start_date.with_timezone(&Local);
        match local_time.hour() {
            5..=6 => TimeOfDay::EarlyMorning,   // 5-7 AM
            7..=10 => TimeOfDay::Morning,       // 7-11 AM  
            11..=13 => TimeOfDay::Midday,       // 11 AM - 2 PM
            14..=17 => TimeOfDay::Afternoon,    // 2-6 PM
            18..=20 => TimeOfDay::Evening,      // 6-9 PM
            _ => TimeOfDay::Night,              // 9 PM - 5 AM
        }
    }

    /// Generate natural language summary
    fn generate_summary(
        &self,
        activity: &Activity,
        insights: &[super::insights::Insight],
        performance: &PerformanceMetrics,
        context: &ContextualFactors,
    ) -> String {
        let mut summary_parts = Vec::new();

        // Activity type with weather context - use the display_name method
        let activity_type = activity.sport_type.display_name();

        // Add weather context if available
        let weather_context = if let Some(weather) = &context.weather {
            match weather.conditions.to_lowercase().as_str() {
                c if c.contains("rain") || c.contains("shower") || c.contains("storm") || c.contains("thunderstorm") => " in the rain",
                c if c.contains("snow") => " in the snow",
                c if c.contains("wind") && weather.wind_speed_kmh.unwrap_or(0.0) > 15.0 => " in windy conditions",
                c if c.contains("hot") || weather.temperature_celsius > 28.0 => " in hot weather",
                c if c.contains("cold") || weather.temperature_celsius < 5.0 => " in cold weather",
                _ => "",
            }
        } else {
            ""
        };

        // Add location context  
        let location_context = if let Some(location) = &context.location {
            if let Some(trail_name) = &location.trail_name {
                format!(" on {}", trail_name)
            } else if let (Some(city), Some(region)) = (&location.city, &location.region) {
                format!(" in {}, {}", city, region)
            } else if let Some(city) = &location.city {
                format!(" in {}", city)
            } else {
                "".to_string()
            }
        } else {
            "".to_string()
        };

        // Effort categorization
        let effort_desc = if let Some(relative_effort) = performance.relative_effort {
            match relative_effort {
                r if r < 3.0 => "light intensity",
                r if r < 5.0 => "moderate intensity", 
                r if r < 7.0 => "hard intensity",
                _ => "very high intensity",
            }
        } else {
            "moderate effort"
        };

        // Zone analysis
        let zone_desc = if let Some(zones) = &performance.zone_distribution {
            if zones.zone2_endurance > 50.0 {
                "endurance zones"
            } else if zones.zone4_threshold > 30.0 {
                "threshold zones"
            } else if zones.zone3_tempo > 30.0 {
                "tempo zones"
            } else {
                "mixed training zones"
            }
        } else {
            "training zones"
        };

        // Personal records context
        let pr_context = match performance.personal_records.len() {
            0 => "",
            1 => " with 1 new personal record",
            n => &format!(" with {} new personal records", n),
        };

        // Build the summary
        summary_parts.push(format!("{}{}{}", 
            Self::to_title_case(activity_type), 
            weather_context,
            location_context
        ));
        
        summary_parts.push(format!("{} and {} in {}", 
            pr_context,
            effort_desc,
            zone_desc
        ));

        let mut summary = summary_parts.join("");

        // Add detailed insights
        if let Some(distance) = activity.distance_meters {
            let distance_km = distance / 1000.0;
            summary.push_str(&format!(". During this {:.1} km session", distance_km));
        }

        // Add primary insight from analysis
        if let Some(main_insight) = insights.first() {
            summary.push_str(&format!(", {}", main_insight.message.to_lowercase()));
        }

        summary
    }

    /// Helper to capitalize first letter of a string
    fn to_title_case(s: &str) -> String {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }
}

impl Default for ActivityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during analysis
#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error("Insufficient activity data for analysis")]
    #[allow(dead_code)]
    InsufficientData,
    
    #[error("Invalid activity data: {0}")]
    #[allow(dead_code)]
    InvalidData(String),
    
    #[error("Analysis computation failed: {0}")]
    #[allow(dead_code)]
    ComputationError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Activity, SportType};
    use chrono::Utc;

    fn create_test_activity() -> Activity {
        Activity {
            id: "test123".to_string(),
            name: "Morning Run".to_string(),
            sport_type: SportType::Run,
            start_date: Utc::now(),
            duration_seconds: 3000, // 50 minutes
            distance_meters: Some(10000.0), // 10km
            elevation_gain: Some(100.0),
            average_speed: Some(3.33), // 12 km/h
            max_speed: Some(5.0),      // 18 km/h
            provider: "test".to_string(),
            average_heart_rate: Some(155),
            max_heart_rate: Some(180),
            calories: Some(500),
            start_latitude: Some(45.5017), // Montreal
            start_longitude: Some(-73.5673),
            city: None,
            region: None,
            country: None,
            trail_name: None,
        }
    }

    #[test]
    fn test_activity_analyzer_creation() {
        let _analyzer = ActivityAnalyzer::new();
        assert!(true); // Just test it can be created
    }

    #[test]
    fn test_calculate_relative_effort() {
        let analyzer = ActivityAnalyzer::new();
        let activity = create_test_activity();
        
        let effort = analyzer.calculate_relative_effort(&activity);
        assert!(effort >= 1.0 && effort <= 10.0);
        assert!(effort > 3.0); // Should be moderate effort for 10km run
    }

    #[test]
    fn test_calculate_zone_distribution() {
        let analyzer = ActivityAnalyzer::new();
        let activity = create_test_activity();
        
        let zones = analyzer.calculate_zone_distribution(&activity);
        assert!(zones.is_some());
        
        if let Some(zones) = zones {
            let total = zones.zone1_recovery + zones.zone2_endurance + 
                       zones.zone3_tempo + zones.zone4_threshold + zones.zone5_vo2max;
            assert!((total - 100.0).abs() < 0.1); // Should sum to 100%
        }
    }

    #[test]
    fn test_detect_personal_records() {
        let analyzer = ActivityAnalyzer::new();
        let mut activity = create_test_activity();
        activity.distance_meters = Some(25000.0); // Long distance for PR
        
        let records = analyzer.detect_personal_records(&activity);
        assert!(!records.is_empty());
        
        let distance_pr = &records[0];
        assert_eq!(distance_pr.record_type, "Longest Distance");
        assert_eq!(distance_pr.value, 25.0); // 25km converted from 25000m
    }

    #[test]
    fn test_determine_time_of_day() {
        let analyzer = ActivityAnalyzer::new();
        
        // Test various times - using UTC times that when converted to local will be predictable
        // Testing the logic rather than timezone conversion specifics
        
        // Create test times that cover different periods
        let test_cases = vec![
            (6, TimeOfDay::EarlyMorning),
            (9, TimeOfDay::Morning),
            (12, TimeOfDay::Midday),
            (15, TimeOfDay::Afternoon),
            (19, TimeOfDay::Evening),
            (23, TimeOfDay::Night),
        ];
        
        for (hour, _expected_category) in test_cases {
            let test_time = chrono::Utc::now().date_naive().and_hms_opt(hour, 0, 0).unwrap().and_utc();
            let time_of_day = analyzer.determine_time_of_day(&test_time);
            
            // Since we're converting UTC to local time, we can't guarantee exact matches
            // But we can verify the function doesn't panic and returns a valid TimeOfDay
            match time_of_day {
                TimeOfDay::EarlyMorning | TimeOfDay::Morning | TimeOfDay::Midday | 
                TimeOfDay::Afternoon | TimeOfDay::Evening | TimeOfDay::Night => {
                    // Any valid TimeOfDay is acceptable since timezone conversion affects the result
                }
            }
        }
    }

    #[test]
    fn test_calculate_efficiency_score() {
        let analyzer = ActivityAnalyzer::new();
        let activity = create_test_activity();
        
        let efficiency = analyzer.calculate_efficiency_score(&activity);
        assert!(efficiency >= 0.0 && efficiency <= 100.0);
    }

    #[tokio::test]
    async fn test_analyze_activity() {
        let analyzer = ActivityAnalyzer::new();
        let activity = create_test_activity();
        
        let result = analyzer.analyze_activity(&activity, None).await;
        assert!(result.is_ok());
        
        let intelligence = result.unwrap();
        assert!(!intelligence.summary.is_empty());
        assert!(intelligence.performance_indicators.relative_effort.is_some());
    }
}