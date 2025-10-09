// ABOUTME: Fitness pattern detection and analysis logic
// ABOUTME: Demonstrates autonomous analysis capabilities using A2A data
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, info, warn};

use crate::a2a_client::{Activity, A2AClient};

/// Analysis results structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResults {
    pub timestamp: DateTime<Utc>,
    pub activities_analyzed: usize,
    pub patterns: Vec<Pattern>,
    pub recommendations: Vec<Recommendation>,
    pub risk_indicators: Vec<RiskIndicator>,
    pub performance_trends: PerformanceTrends,
}

/// Detected fitness pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub pattern_type: String,
    pub confidence: f64,
    pub description: String,
    pub supporting_data: HashMap<String, Value>,
}

/// Generated recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub category: String,
    pub priority: String,
    pub title: String,
    pub description: String,
    pub actionable_steps: Vec<String>,
}

/// Risk indicator for injury or overtraining
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskIndicator {
    pub risk_type: String,
    pub severity: String,
    pub probability: f64,
    pub description: String,
    pub mitigation_actions: Vec<String>,
}

/// Performance trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrends {
    pub overall_trend: String,
    pub pace_trend: Option<f64>,
    pub distance_trend: Option<f64>,
    pub frequency_trend: Option<f64>,
    pub heart_rate_trend: Option<f64>,
}

/// Fitness analysis engine
pub struct FitnessAnalyzer {
    pub client: A2AClient,
}

impl FitnessAnalyzer {
    /// Create a new fitness analyzer
    pub fn new(client: A2AClient) -> Self {
        Self { client }
    }

    /// Perform comprehensive fitness analysis
    pub async fn analyze(&mut self, provider: &str, max_activities: u32) -> Result<AnalysisResults> {
        info!("🔬 Starting comprehensive fitness analysis");
        
        // Fetch recent activities via A2A
        let activities = self.client.get_activities(provider, max_activities).await?;
        
        if activities.is_empty() {
            warn!("⚠️ No activities found for analysis");
            return Ok(AnalysisResults {
                timestamp: Utc::now(),
                activities_analyzed: 0,
                patterns: vec![],
                recommendations: vec![],
                risk_indicators: vec![],
                performance_trends: PerformanceTrends {
                    overall_trend: "insufficient_data".to_string(),
                    pace_trend: None,
                    distance_trend: None,
                    frequency_trend: None,
                    heart_rate_trend: None,
                },
            });
        }

        info!("📊 Analyzing {} activities", activities.len());

        // Perform pattern detection
        let patterns = self.detect_patterns(&activities)?;
        info!("🔍 Detected {} patterns", patterns.len());

        // Generate recommendations
        let recommendations = self.generate_recommendations(&activities, &patterns).await?;
        info!("💡 Generated {} recommendations", recommendations.len());

        // Assess risk indicators
        let risk_indicators = self.assess_risks(&activities)?;
        info!("⚠️ Identified {} risk indicators", risk_indicators.len());

        // Analyze performance trends
        let performance_trends = self.analyze_performance_trends(&activities)?;
        info!("📈 Performance trend: {}", performance_trends.overall_trend);

        Ok(AnalysisResults {
            timestamp: Utc::now(),
            activities_analyzed: activities.len(),
            patterns,
            recommendations,
            risk_indicators,
            performance_trends,
        })
    }

    /// Detect patterns in fitness activities
    fn detect_patterns(&self, activities: &[Activity]) -> Result<Vec<Pattern>> {
        let mut patterns = Vec::new();

        // Pattern 1: Training frequency patterns
        let frequency_pattern = self.analyze_training_frequency(activities)?;
        if let Some(pattern) = frequency_pattern {
            patterns.push(pattern);
        }

        // Pattern 2: Sport distribution patterns
        let sport_pattern = self.analyze_sport_distribution(activities)?;
        if let Some(pattern) = sport_pattern {
            patterns.push(pattern);
        }

        // Pattern 3: Distance progression patterns
        let distance_pattern = self.analyze_distance_progression(activities)?;
        if let Some(pattern) = distance_pattern {
            patterns.push(pattern);
        }

        // Pattern 4: Weekly rhythm patterns
        let rhythm_pattern = self.analyze_weekly_rhythm(activities)?;
        if let Some(pattern) = rhythm_pattern {
            patterns.push(pattern);
        }

        debug!("Detected patterns: {:?}", patterns);
        Ok(patterns)
    }

    /// Analyze training frequency patterns
    fn analyze_training_frequency(&self, activities: &[Activity]) -> Result<Option<Pattern>> {
        if activities.is_empty() {
            return Ok(None);
        }

        // Calculate activities per week
        let date_range = self.get_date_range(activities)?;
        let weeks = (date_range.num_days() as f64 / 7.0).max(1.0);
        let activities_per_week = activities.len() as f64 / weeks;

        let (pattern_type, confidence, description) = match activities_per_week {
            x if x >= 6.0 => (
                "high_frequency",
                0.9,
                format!("High training frequency: {:.1} activities per week", x)
            ),
            x if x >= 3.0 => (
                "moderate_frequency", 
                0.8,
                format!("Moderate training frequency: {:.1} activities per week", x)
            ),
            x => (
                "low_frequency",
                0.7,
                format!("Low training frequency: {:.1} activities per week", x)
            ),
        };

        let mut supporting_data = HashMap::new();
        supporting_data.insert("activities_per_week".to_string(), 
            Value::Number(serde_json::Number::from_f64(activities_per_week).unwrap()));
        supporting_data.insert("total_activities".to_string(),
            Value::Number(serde_json::Number::from(activities.len())));
        supporting_data.insert("weeks_analyzed".to_string(),
            Value::Number(serde_json::Number::from_f64(weeks).unwrap()));

        Ok(Some(Pattern {
            pattern_type: pattern_type.to_string(),
            confidence,
            description,
            supporting_data,
        }))
    }

    /// Analyze sport distribution patterns
    fn analyze_sport_distribution(&self, activities: &[Activity]) -> Result<Option<Pattern>> {
        if activities.is_empty() {
            return Ok(None);
        }

        let mut sport_counts = HashMap::new();
        for activity in activities {
            *sport_counts.entry(activity.sport_type.clone()).or_insert(0) += 1;
        }

        let dominant_sport = sport_counts.iter()
            .max_by_key(|(_, count)| *count)
            .map(|(sport, count)| (sport.clone(), *count));

        if let Some((sport, count)) = dominant_sport {
            let percentage = (count as f64 / activities.len() as f64) * 100.0;
            
            let (pattern_type, confidence, description) = if percentage >= 80.0 {
                ("sport_specialization", 0.9, 
                 format!("Sport specialization: {:.0}% {} activities", percentage, sport))
            } else if percentage >= 50.0 {
                ("sport_preference", 0.8,
                 format!("Strong sport preference: {:.0}% {} activities", percentage, sport))
            } else {
                ("sport_variety", 0.7,
                 format!("Sport variety: {:.0}% {} (primary), {} total sports", 
                        percentage, sport, sport_counts.len()))
            };

            let mut supporting_data = HashMap::new();
            supporting_data.insert("dominant_sport".to_string(), 
                Value::String(sport));
            supporting_data.insert("dominant_percentage".to_string(),
                Value::Number(serde_json::Number::from_f64(percentage).unwrap()));
            supporting_data.insert("total_sports".to_string(),
                Value::Number(serde_json::Number::from(sport_counts.len())));

            Ok(Some(Pattern {
                pattern_type: pattern_type.to_string(),
                confidence,
                description,
                supporting_data,
            }))
        } else {
            Ok(None)
        }
    }

    /// Analyze distance progression patterns
    fn analyze_distance_progression(&self, activities: &[Activity]) -> Result<Option<Pattern>> {
        // Filter activities with distance data and sort by date
        let mut activities_with_distance: Vec<_> = activities
            .iter()
            .filter_map(|a| {
                a.distance_meters.map(|d| (a, d))
            })
            .collect();

        if activities_with_distance.len() < 5 {
            return Ok(None);
        }

        // Sort by start date (newest first)
        activities_with_distance.sort_by(|a, b| b.0.start_date.cmp(&a.0.start_date));

        // Calculate trend using linear regression on recent activities
        let recent_activities = &activities_with_distance[..activities_with_distance.len().min(20)];
        let trend = self.calculate_distance_trend(recent_activities)?;

        let (pattern_type, confidence, description) = if trend > 50.0 {
            ("distance_increase", 0.8,
             format!("Increasing distance trend: +{:.0}m per activity", trend))
        } else if trend < -50.0 {
            ("distance_decrease", 0.8,
             format!("Decreasing distance trend: {:.0}m per activity", trend))
        } else {
            ("distance_stable", 0.7,
             "Stable distance pattern".to_string())
        };

        let mut supporting_data = HashMap::new();
        supporting_data.insert("trend_meters_per_activity".to_string(),
            Value::Number(serde_json::Number::from_f64(trend).unwrap()));
        supporting_data.insert("activities_analyzed".to_string(),
            Value::Number(serde_json::Number::from(recent_activities.len())));

        Ok(Some(Pattern {
            pattern_type: pattern_type.to_string(),
            confidence,
            description,
            supporting_data,
        }))
    }

    /// Analyze weekly rhythm patterns
    fn analyze_weekly_rhythm(&self, activities: &[Activity]) -> Result<Option<Pattern>> {
        if activities.len() < 7 {
            return Ok(None);
        }

        // Parse dates and count activities by day of week
        let mut day_counts = [0; 7]; // Sunday = 0, Monday = 1, etc.
        
        for activity in activities {
            if let Ok(date) = DateTime::parse_from_rfc3339(&activity.start_date) {
                let weekday = date.weekday().num_days_from_sunday() as usize;
                day_counts[weekday] += 1;
            }
        }

        // Find peak days
        let max_count = *day_counts.iter().max().unwrap_or(&0);
        let peak_days: Vec<_> = day_counts
            .iter()
            .enumerate()
            .filter_map(|(day, &count)| {
                if count == max_count && count > 0 {
                    Some(self.weekday_name(day))
                } else {
                    None
                }
            })
            .collect();

        if peak_days.is_empty() {
            return Ok(None);
        }

        let pattern_type = if peak_days.len() == 1 {
            "single_peak_day"
        } else if peak_days.len() == 2 {
            "dual_peak_days"
        } else {
            "distributed_rhythm"
        };

        let description = format!("Weekly rhythm: Peak activity on {}", 
            peak_days.join(" and "));

        let mut supporting_data = HashMap::new();
        supporting_data.insert("peak_days".to_string(),
            Value::Array(peak_days.iter().map(|d| Value::String(d.clone())).collect())); // Safe: String ownership for JSON value
        supporting_data.insert("max_day_count".to_string(),
            Value::Number(serde_json::Number::from(max_count)));

        Ok(Some(Pattern {
            pattern_type: pattern_type.to_string(),
            confidence: 0.7,
            description,
            supporting_data,
        }))
    }

    /// Generate recommendations based on analysis
    async fn generate_recommendations(
        &mut self,
        _activities: &[Activity],
        patterns: &[Pattern],
    ) -> Result<Vec<Recommendation>> {
        let mut recommendations = Vec::new();

        // Add pattern-based recommendations
        for pattern in patterns {
            match pattern.pattern_type.as_str() {
                "low_frequency" => {
                    recommendations.push(Recommendation {
                        category: "training_volume".to_string(),
                        priority: "medium".to_string(),
                        title: "Increase Training Frequency".to_string(),
                        description: "Consider adding 1-2 more activities per week to improve fitness gains".to_string(),
                        actionable_steps: vec![
                            "Start with one additional easy-intensity session per week".to_string(),
                            "Focus on activities you enjoy to maintain consistency".to_string(),
                            "Track your progress to stay motivated".to_string(),
                        ],
                    });
                }
                "high_frequency" => {
                    recommendations.push(Recommendation {
                        category: "recovery".to_string(),
                        priority: "high".to_string(),
                        title: "Prioritize Recovery".to_string(),
                        description: "High training frequency detected. Ensure adequate recovery to prevent overtraining".to_string(),
                        actionable_steps: vec![
                            "Schedule at least 1-2 complete rest days per week".to_string(),
                            "Include easy/recovery activities between intense sessions".to_string(),
                            "Monitor sleep quality and stress levels".to_string(),
                        ],
                    });
                }
                "sport_specialization" => {
                    recommendations.push(Recommendation {
                        category: "training_variety".to_string(),
                        priority: "medium".to_string(),
                        title: "Add Cross-Training".to_string(),
                        description: "Consider adding variety to prevent overuse injuries and improve overall fitness".to_string(),
                        actionable_steps: vec![
                            "Add 1 different sport activity per week".to_string(),
                            "Include strength training to support your primary sport".to_string(),
                            "Try complementary activities (e.g., yoga, swimming)".to_string(),
                        ],
                    });
                }
                _ => {}
            }
        }

        // Try to get A2A-generated recommendations
        match self.client.generate_recommendations("strava").await {
            Ok(a2a_recommendations) => {
                if let Some(recs) = a2a_recommendations.get("training_recommendations") {
                    if let Ok(parsed_recs) = serde_json::from_value::<Vec<HashMap<String, Value>>>(recs.clone()) { // Safe: JSON value ownership for deserialization
                        for rec in parsed_recs {
                            if let (Some(title), Some(description)) = (
                                rec.get("title").and_then(|v| v.as_str()),
                                rec.get("description").and_then(|v| v.as_str())
                            ) {
                                recommendations.push(Recommendation {
                                    category: "a2a_generated".to_string(),
                                    priority: rec.get("priority")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("medium")
                                        .to_string(),
                                    title: title.to_string(),
                                    description: description.to_string(),
                                    actionable_steps: vec!["Follow A2A recommendation".to_string()],
                                });
                            }
                        }
                    }
                }
            }
            Err(e) => {
                debug!("Failed to get A2A recommendations: {}", e);
            }
        }

        Ok(recommendations)
    }

    /// Assess injury and overtraining risks
    fn assess_risks(&self, activities: &[Activity]) -> Result<Vec<RiskIndicator>> {
        let mut risks = Vec::new();

        // Risk 1: Sudden volume increase
        if let Some(risk) = self.assess_volume_spike_risk(activities)? {
            risks.push(risk);
        }

        // Risk 2: Insufficient recovery
        if let Some(risk) = self.assess_recovery_risk(activities)? {
            risks.push(risk);
        }

        // Risk 3: Monotonous training
        if let Some(risk) = self.assess_monotony_risk(activities)? {
            risks.push(risk);
        }

        Ok(risks)
    }

    /// Assess risk from sudden training volume increases
    fn assess_volume_spike_risk(&self, activities: &[Activity]) -> Result<Option<RiskIndicator>> {
        if activities.len() < 14 {
            return Ok(None);
        }

        // Compare recent 2 weeks vs previous 2 weeks
        let mut sorted_activities = activities.to_vec();
        sorted_activities.sort_by(|a, b| b.start_date.cmp(&a.start_date));

        let recent_14 = &sorted_activities[..14.min(sorted_activities.len())];
        let previous_14 = if sorted_activities.len() >= 28 {
            &sorted_activities[14..28]
        } else {
            return Ok(None);
        };

        let recent_volume: u32 = recent_14.iter()
            .map(|a| a.duration_seconds.unwrap_or(0))
            .sum();
        let previous_volume: u32 = previous_14.iter()
            .map(|a| a.duration_seconds.unwrap_or(0))
            .sum();

        if previous_volume == 0 {
            return Ok(None);
        }

        let volume_increase = (recent_volume as f64 / previous_volume as f64 - 1.0) * 100.0;

        if volume_increase > 30.0 {
            let severity = if volume_increase > 50.0 { "high" } else { "medium" };
            let probability = (volume_increase / 100.0).min(1.0);

            Ok(Some(RiskIndicator {
                risk_type: "volume_spike".to_string(),
                severity: severity.to_string(),
                probability,
                description: format!("Training volume increased by {:.0}% in recent 2 weeks", volume_increase),
                mitigation_actions: vec![
                    "Reduce training intensity for 1-2 weeks".to_string(),
                    "Focus on recovery and sleep quality".to_string(),
                    "Monitor for signs of fatigue or injury".to_string(),
                ],
            }))
        } else {
            Ok(None)
        }
    }

    /// Assess insufficient recovery risk
    fn assess_recovery_risk(&self, activities: &[Activity]) -> Result<Option<RiskIndicator>> {
        if activities.len() < 7 {
            return Ok(None);
        }

        // Check for consecutive high-intensity days
        let mut sorted_activities = activities.to_vec();
        sorted_activities.sort_by(|a, b| a.start_date.cmp(&b.start_date));

        let mut consecutive_days = 0;
        let mut max_consecutive = 0;
        let mut prev_date: Option<DateTime<Utc>> = None;

        for activity in &sorted_activities {
            if let Ok(date) = DateTime::parse_from_rfc3339(&activity.start_date) {
                let date = date.with_timezone(&Utc);
                
                if let Some(prev) = prev_date {
                    let days_diff = (date - prev).num_days();
                    if days_diff <= 1 {
                        consecutive_days += 1;
                    } else {
                        max_consecutive = max_consecutive.max(consecutive_days);
                        consecutive_days = 1;
                    }
                } else {
                    consecutive_days = 1;
                }
                prev_date = Some(date);
            }
        }
        max_consecutive = max_consecutive.max(consecutive_days);

        if max_consecutive >= 7 {
            let severity = if max_consecutive >= 10 { "high" } else { "medium" };
            let probability = (max_consecutive as f64 / 14.0).min(1.0);

            Ok(Some(RiskIndicator {
                risk_type: "insufficient_recovery".to_string(),
                severity: severity.to_string(),
                probability,
                description: format!("Up to {} consecutive training days detected", max_consecutive),
                mitigation_actions: vec![
                    "Schedule at least 1 complete rest day per week".to_string(),
                    "Include easy recovery sessions between intense workouts".to_string(),
                    "Listen to your body and take extra rest when needed".to_string(),
                ],
            }))
        } else {
            Ok(None)
        }
    }

    /// Assess monotonous training risk
    fn assess_monotony_risk(&self, activities: &[Activity]) -> Result<Option<RiskIndicator>> {
        if activities.len() < 10 {
            return Ok(None);
        }

        // Check sport variety
        let unique_sports: std::collections::HashSet<_> = activities
            .iter()
            .map(|a| &a.sport_type)
            .collect();

        let sport_variety_ratio = unique_sports.len() as f64 / activities.len() as f64;

        // Check distance variety (for activities with distance)
        let distances: Vec<f64> = activities
            .iter()
            .filter_map(|a| a.distance_meters)
            .collect();

        let distance_coefficient_of_variation = if distances.len() > 3 {
            let mean = distances.iter().sum::<f64>() / distances.len() as f64;
            let variance = distances.iter()
                .map(|d| (d - mean).powi(2))
                .sum::<f64>() / distances.len() as f64;
            (variance.sqrt() / mean).max(0.0)
        } else {
            1.0
        };

        let is_monotonous = sport_variety_ratio < 0.1 && distance_coefficient_of_variation < 0.2;

        if is_monotonous {
            Ok(Some(RiskIndicator {
                risk_type: "training_monotony".to_string(),
                severity: "medium".to_string(),
                probability: 0.7,
                description: "Training lacks variety in sports and distances".to_string(),
                mitigation_actions: vec![
                    "Try different sports or activities".to_string(),
                    "Vary workout distances and intensities".to_string(),
                    "Include different training environments (trails, tracks, etc.)".to_string(),
                ],
            }))
        } else {
            Ok(None)
        }
    }

    /// Analyze performance trends
    fn analyze_performance_trends(&self, activities: &[Activity]) -> Result<PerformanceTrends> {
        if activities.is_empty() {
            return Ok(PerformanceTrends {
                overall_trend: "insufficient_data".to_string(),
                pace_trend: None,
                distance_trend: None,
                frequency_trend: None,
                heart_rate_trend: None,
            });
        }

        // Calculate various trends
        let pace_trend = self.calculate_pace_trend(activities)?;
        let distance_trend = self.calculate_distance_trend_value(activities)?;
        let frequency_trend = self.calculate_frequency_trend(activities)?;
        let heart_rate_trend = self.calculate_heart_rate_trend(activities)?;

        // Determine overall trend
        let overall_trend = self.determine_overall_trend(
            pace_trend,
            distance_trend,
            frequency_trend,
        );

        Ok(PerformanceTrends {
            overall_trend,
            pace_trend,
            distance_trend,
            frequency_trend,
            heart_rate_trend,
        })
    }

    // Helper methods for calculations

    fn get_date_range(&self, activities: &[Activity]) -> Result<Duration> {
        let dates: Result<Vec<_>, _> = activities
            .iter()
            .map(|a| DateTime::parse_from_rfc3339(&a.start_date))
            .collect();

        let dates = dates?;
        if let (Some(earliest), Some(latest)) = (dates.iter().min(), dates.iter().max()) {
            Ok(*latest - *earliest)
        } else {
            Ok(Duration::days(1))
        }
    }

    fn calculate_distance_trend(&self, activities_with_distance: &[(&Activity, f64)]) -> Result<f64> {
        if activities_with_distance.len() < 3 {
            return Ok(0.0);
        }

        // Simple linear regression
        let n = activities_with_distance.len() as f64;
        let sum_x: f64 = (0..activities_with_distance.len()).map(|i| i as f64).sum();
        let sum_y: f64 = activities_with_distance.iter().map(|(_, d)| *d).sum();
        let sum_xy: f64 = activities_with_distance
            .iter()
            .enumerate()
            .map(|(i, (_, d))| i as f64 * d)
            .sum();
        let sum_x_squared: f64 = (0..activities_with_distance.len())
            .map(|i| (i as f64).powi(2))
            .sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x_squared - sum_x.powi(2));
        Ok(slope)
    }

    fn calculate_distance_trend_value(&self, activities: &[Activity]) -> Result<Option<f64>> {
        let activities_with_distance: Vec<_> = activities
            .iter()
            .filter_map(|a| a.distance_meters.map(|d| (a, d)))
            .collect();

        if activities_with_distance.len() < 3 {
            return Ok(None);
        }

        Ok(Some(self.calculate_distance_trend(&activities_with_distance)?))
    }

    fn calculate_pace_trend(&self, activities: &[Activity]) -> Result<Option<f64>> {
        // Calculate pace trend for running activities
        let running_activities: Vec<_> = activities
            .iter()
            .filter(|a| a.sport_type.to_lowercase().contains("run"))
            .filter_map(|a| {
                if let (Some(distance), Some(duration)) = (a.distance_meters, a.duration_seconds) {
                    if distance > 0.0 && duration > 0 {
                        Some((a, duration as f64 / distance)) // seconds per meter
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        if running_activities.len() < 3 {
            return Ok(None);
        }

        let trend = self.calculate_distance_trend(&running_activities)?;
        Ok(Some(trend))
    }

    fn calculate_frequency_trend(&self, activities: &[Activity]) -> Result<Option<f64>> {
        if activities.len() < 14 {
            return Ok(None);
        }

        // Compare recent vs earlier frequency
        let mut sorted = activities.to_vec();
        sorted.sort_by(|a, b| b.start_date.cmp(&a.start_date));

        let recent_half = &sorted[..sorted.len() / 2];
        let earlier_half = &sorted[sorted.len() / 2..];

        let recent_days = self.get_date_range(recent_half)?.num_days() as f64;
        let earlier_days = self.get_date_range(earlier_half)?.num_days() as f64;

        if recent_days > 0.0 && earlier_days > 0.0 {
            let recent_freq = recent_half.len() as f64 / recent_days * 7.0;
            let earlier_freq = earlier_half.len() as f64 / earlier_days * 7.0;
            Ok(Some(recent_freq - earlier_freq))
        } else {
            Ok(None)
        }
    }

    fn calculate_heart_rate_trend(&self, activities: &[Activity]) -> Result<Option<f64>> {
        let hr_activities: Vec<_> = activities
            .iter()
            .filter_map(|a| a.average_heart_rate.map(|hr| (a, hr as f64)))
            .collect();

        if hr_activities.len() < 3 {
            return Ok(None);
        }

        let trend = self.calculate_distance_trend(&hr_activities)?;
        Ok(Some(trend))
    }

    fn determine_overall_trend(
        &self,
        pace_trend: Option<f64>,
        distance_trend: Option<f64>,
        frequency_trend: Option<f64>,
    ) -> String {
        let mut positive_indicators = 0;
        let mut negative_indicators = 0;

        // Improving pace (negative trend is good for pace)
        if let Some(pace) = pace_trend {
            if pace < -0.001 {
                positive_indicators += 1;
            } else if pace > 0.001 {
                negative_indicators += 1;
            }
        }

        // Increasing distance
        if let Some(distance) = distance_trend {
            if distance > 50.0 {
                positive_indicators += 1;
            } else if distance < -50.0 {
                negative_indicators += 1;
            }
        }

        // Increasing frequency
        if let Some(frequency) = frequency_trend {
            if frequency > 0.5 {
                positive_indicators += 1;
            } else if frequency < -0.5 {
                negative_indicators += 1;
            }
        }

        match (positive_indicators, negative_indicators) {
            (p, n) if p > n => "improving".to_string(),
            (p, n) if n > p => "declining".to_string(),
            _ => "stable".to_string(),
        }
    }

    fn weekday_name(&self, day: usize) -> String {
        match day {
            0 => "Sunday",
            1 => "Monday",
            2 => "Tuesday",
            3 => "Wednesday",
            4 => "Thursday",
            5 => "Friday",
            6 => "Saturday",
            _ => "Unknown",
        }
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_creation() {
        let pattern = Pattern {
            pattern_type: "test_pattern".to_string(),
            confidence: 0.8,
            description: "Test description".to_string(),
            supporting_data: HashMap::new(),
        };

        assert_eq!(pattern.pattern_type, "test_pattern");
        assert_eq!(pattern.confidence, 0.8);
    }

    #[test]
    fn test_analysis_results_serialization() {
        let results = AnalysisResults {
            timestamp: Utc::now(),
            activities_analyzed: 10,
            patterns: vec![],
            recommendations: vec![],
            risk_indicators: vec![],
            performance_trends: PerformanceTrends {
                overall_trend: "stable".to_string(),
                pace_trend: None,
                distance_trend: None,
                frequency_trend: None,
                heart_rate_trend: None,
            },
        };

        let serialized = serde_json::to_string(&results).unwrap();
        assert!(serialized.contains("stable"));
    }
}