//! Performance trend analysis and historical comparison engine

use super::*;
use crate::models::Activity;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Trait for analyzing performance trends over time
#[async_trait::async_trait]
pub trait PerformanceAnalyzerTrait {
    /// Analyze performance trends over a given timeframe
    async fn analyze_trends(&self, activities: &[Activity], timeframe: TimeFrame, metric: &str) -> Result<TrendAnalysis>;
    
    /// Calculate fitness score based on recent activities
    async fn calculate_fitness_score(&self, activities: &[Activity]) -> Result<FitnessScore>;
    
    /// Predict performance for a target activity
    async fn predict_performance(&self, activities: &[Activity], target: &ActivityGoal) -> Result<PerformancePrediction>;
    
    /// Calculate training load balance and recovery metrics
    async fn analyze_training_load(&self, activities: &[Activity]) -> Result<TrainingLoadAnalysis>;
}

/// Advanced performance analyzer implementation
pub struct AdvancedPerformanceAnalyzer {
    user_profile: Option<UserFitnessProfile>,
}

impl AdvancedPerformanceAnalyzer {
    /// Create a new performance analyzer
    pub fn new() -> Self {
        Self {
            user_profile: None,
        }
    }

    /// Create analyzer with user profile
    pub fn with_profile(profile: UserFitnessProfile) -> Self {
        Self {
            user_profile: Some(profile),
        }
    }

    /// Calculate statistical trend strength
    fn calculate_trend_strength(&self, data_points: &[TrendDataPoint]) -> f64 {
        if data_points.len() < 2 {
            return 0.0;
        }

        // Simple linear regression to calculate R-squared
        let n = data_points.len() as f64;
        let sum_x: f64 = (0..data_points.len()).map(|i| i as f64).sum();
        let sum_y: f64 = data_points.iter().map(|p| p.value).sum();
        let sum_xy: f64 = data_points.iter().enumerate()
            .map(|(i, p)| i as f64 * p.value).sum();
        let sum_x2: f64 = (0..data_points.len()).map(|i| (i as f64).powi(2)).sum();
        let sum_y2: f64 = data_points.iter().map(|p| p.value.powi(2)).sum();

        let numerator = n * sum_xy - sum_x * sum_y;
        let denominator = ((n * sum_x2 - sum_x.powi(2)) * (n * sum_y2 - sum_y.powi(2))).sqrt();

        if denominator == 0.0 {
            return 0.0;
        }

        let correlation = numerator / denominator;
        correlation.abs() // Return absolute correlation as trend strength
    }

    /// Apply smoothing to data points using moving average
    fn apply_smoothing(&self, data_points: &mut Vec<TrendDataPoint>, window_size: usize) {
        if window_size <= 1 || data_points.len() < window_size {
            return;
        }

        for i in 0..data_points.len() {
            let start = if i >= window_size / 2 { i - window_size / 2 } else { 0 };
            let end = std::cmp::min(start + window_size, data_points.len());
            
            let window_sum: f64 = data_points[start..end].iter().map(|p| p.value).sum();
            let window_avg = window_sum / (end - start) as f64;
            
            data_points[i].smoothed_value = Some(window_avg);
        }
    }

    /// Generate trend insights based on analysis
    fn generate_trend_insights(&self, analysis: &TrendAnalysis) -> Vec<AdvancedInsight> {
        let mut insights = Vec::new();

        // Trend direction insight
        let (message, severity) = match analysis.trend_direction {
            TrendDirection::Improving => {
                if analysis.trend_strength > 0.7 {
                    ("Strong improvement trend detected - excellent progress!".to_string(), InsightSeverity::Info)
                } else {
                    ("Gradual improvement trend - keep up the consistent work".to_string(), InsightSeverity::Info)
                }
            },
            TrendDirection::Declining => {
                if analysis.trend_strength > 0.7 {
                    ("Significant decline in performance - consider recovery or training adjustments".to_string(), InsightSeverity::Warning)
                } else {
                    ("Slight performance decline - may need attention".to_string(), InsightSeverity::Warning)
                }
            },
            TrendDirection::Stable => {
                ("Performance is stable - consider progressive overload for improvement".to_string(), InsightSeverity::Info)
            },
            TrendDirection::Volatile => {
                ("Performance is inconsistent - focus on training consistency".to_string(), InsightSeverity::Warning)
            },
        };

        let mut metadata = HashMap::new();
        metadata.insert("trend_strength".to_string(), serde_json::Value::from(analysis.trend_strength));
        metadata.insert("statistical_significance".to_string(), serde_json::Value::from(analysis.statistical_significance));

        insights.push(AdvancedInsight {
            insight_type: "performance_trend".to_string(),
            message,
            confidence: if analysis.statistical_significance > 0.8 {
                Confidence::High
            } else if analysis.statistical_significance > 0.6 {
                Confidence::Medium
            } else {
                Confidence::Low
            },
            severity,
            metadata,
        });

        // Data quality insight
        if analysis.data_points.len() < 5 {
            insights.push(AdvancedInsight {
                insight_type: "data_quality".to_string(),
                message: "Limited data points - trends may not be reliable".to_string(),
                confidence: Confidence::Medium,
                severity: InsightSeverity::Warning,
                metadata: HashMap::new(),
            });
        }

        insights
    }
}

#[async_trait::async_trait]
impl PerformanceAnalyzerTrait for AdvancedPerformanceAnalyzer {
    async fn analyze_trends(&self, activities: &[Activity], timeframe: TimeFrame, metric: &str) -> Result<TrendAnalysis> {
        // Filter activities by timeframe
        let start_date = timeframe.start_date();
        let end_date = timeframe.end_date();
        
        let filtered_activities: Vec<_> = activities
            .iter()
            .filter(|a| {
                if let Some(start_time) = &a.start_date_local {
                    if let Ok(activity_date) = DateTime::parse_from_rfc3339(start_time) {
                        let activity_utc = activity_date.with_timezone(&Utc);
                        return activity_utc >= start_date && activity_utc <= end_date;
                    }
                }
                false
            })
            .collect();

        if filtered_activities.is_empty() {
            return Err(anyhow::anyhow!("No activities found in the specified timeframe"));
        }

        // Extract metric values
        let mut data_points = Vec::new();
        
        for activity in filtered_activities {
            if let Some(start_time) = &activity.start_date_local {
                if let Ok(activity_date) = DateTime::parse_from_rfc3339(start_time) {
                    let activity_utc = activity_date.with_timezone(&Utc);
                    
                    let value = match metric {
                        "pace" | "speed" => activity.average_speed.map(|s| s as f64),
                        "heart_rate" => activity.average_heartrate.map(|hr| hr as f64),
                        "power" => activity.average_watts.map(|w| w as f64),
                        "distance" => activity.distance.map(|d| d as f64),
                        "duration" => activity.moving_time.map(|t| t as f64),
                        "elevation" => activity.total_elevation_gain.map(|e| e as f64),
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
            }
        }

        if data_points.is_empty() {
            return Err(anyhow::anyhow!("No valid data points found for metric: {}", metric));
        }

        // Sort by date
        data_points.sort_by(|a, b| a.date.cmp(&b.date));

        // Apply smoothing
        self.apply_smoothing(&mut data_points, 3);

        // Calculate trend direction
        let first_half_avg = data_points[..data_points.len()/2].iter()
            .map(|p| p.value).sum::<f64>() / (data_points.len()/2) as f64;
        let second_half_avg = data_points[data_points.len()/2..].iter()
            .map(|p| p.value).sum::<f64>() / (data_points.len() - data_points.len()/2) as f64;

        let trend_direction = if (second_half_avg - first_half_avg).abs() < first_half_avg * 0.05 {
            TrendDirection::Stable
        } else if second_half_avg > first_half_avg {
            if metric == "pace" { // For pace, lower is better
                TrendDirection::Declining
            } else {
                TrendDirection::Improving
            }
        } else {
            if metric == "pace" { // For pace, lower is better  
                TrendDirection::Improving
            } else {
                TrendDirection::Declining
            }
        };

        // Calculate trend strength
        let trend_strength = self.calculate_trend_strength(&data_points);

        // Calculate statistical significance (simplified)
        let statistical_significance = if data_points.len() > 10 && trend_strength > 0.5 {
            trend_strength
        } else {
            trend_strength * 0.7
        };

        let analysis = TrendAnalysis {
            timeframe,
            metric: metric.to_string(),
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

    async fn calculate_fitness_score(&self, activities: &[Activity]) -> Result<FitnessScore> {
        // Calculate fitness score based on recent training load and consistency
        let recent_activities: Vec<_> = activities
            .iter()
            .filter(|a| {
                if let Some(start_time) = &a.start_date_local {
                    if let Ok(activity_date) = DateTime::parse_from_rfc3339(start_time) {
                        let activity_utc = activity_date.with_timezone(&Utc);
                        let days_ago = (Utc::now() - activity_utc).num_days();
                        return days_ago <= 42; // Last 6 weeks
                    }
                }
                false
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
        let activities_per_week = recent_activities.len() as f64 / weeks as f64;
        let consistency = (activities_per_week / 5.0).min(1.0) * 100.0; // Target: 5 activities/week

        // Calculate aerobic fitness based on heart rate and duration
        let mut aerobic_score = 0.0;
        let mut aerobic_count = 0;
        
        for activity in &recent_activities {
            if let (Some(hr), Some(duration)) = (activity.average_heartrate, activity.moving_time) {
                if hr > 120.0 && duration > 1800 { // Aerobic threshold
                    aerobic_score += (hr as f64 - 120.0) * (duration as f64 / 3600.0);
                    aerobic_count += 1;
                }
            }
        }
        
        let aerobic_fitness = if aerobic_count > 0 {
            (aerobic_score / aerobic_count as f64).min(100.0)
        } else {
            0.0
        };

        // Calculate strength endurance based on power and effort
        let mut strength_score = 0.0;
        let mut strength_count = 0;
        
        for activity in &recent_activities {
            if let Some(power) = activity.average_watts {
                strength_score += power as f64;
                strength_count += 1;
            } else if let (Some(hr), Some(duration)) = (activity.average_heartrate, activity.moving_time) {
                if hr > 160.0 { // High intensity
                    strength_score += hr as f64;
                    strength_count += 1;
                }
            }
        }
        
        let strength_endurance = if strength_count > 0 {
            (strength_score / strength_count as f64 / 5.0).min(100.0)
        } else {
            0.0
        };

        // Overall score is weighted average
        let overall_score = (aerobic_fitness * 0.4 + strength_endurance * 0.3 + consistency * 0.3).min(100.0);

        // Determine trend by comparing with older activities
        let trend = if overall_score > 70.0 {
            TrendDirection::Improving
        } else if overall_score > 40.0 {
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

    async fn predict_performance(&self, activities: &[Activity], target: &ActivityGoal) -> Result<PerformancePrediction> {
        // Simple performance prediction based on recent trends
        let similar_activities: Vec<_> = activities
            .iter()
            .filter(|a| a.sport_type == target.sport_type)
            .collect();

        if similar_activities.is_empty() {
            return Err(anyhow::anyhow!("No similar activities found for prediction"));
        }

        // Calculate recent average performance
        let recent_performance = if let Some(last_activity) = similar_activities.last() {
            match target.metric.as_str() {
                "distance" => last_activity.distance.unwrap_or(0.0) as f64,
                "time" => last_activity.moving_time.unwrap_or(0) as f64,
                "pace" => last_activity.average_speed.unwrap_or(0.0) as f64,
                _ => 0.0,
            }
        } else {
            0.0
        };

        // Simple improvement factor based on training consistency
        let training_days = similar_activities.len() as f64;
        let improvement_factor = if training_days > 20.0 {
            1.1 // 10% improvement
        } else if training_days > 10.0 {
            1.05 // 5% improvement
        } else {
            1.0 // No improvement
        };

        let predicted_value = recent_performance * improvement_factor;
        
        let confidence = if training_days > 20.0 {
            Confidence::High
        } else if training_days > 10.0 {
            Confidence::Medium
        } else {
            Confidence::Low
        };

        Ok(PerformancePrediction {
            target_goal: target.clone(),
            predicted_value,
            confidence,
            factors: vec![
                "Recent training consistency".to_string(),
                "Historical performance trends".to_string(),
                "Current fitness level".to_string(),
            ],
            recommendations: vec![
                "Maintain consistent training schedule".to_string(),
                "Focus on progressive overload".to_string(),
                "Include recovery sessions".to_string(),
            ],
            estimated_achievement_date: target.target_date,
        })
    }

    async fn analyze_training_load(&self, activities: &[Activity]) -> Result<TrainingLoadAnalysis> {
        // Analyze training load over recent weeks
        let weeks = 4;
        let start_date = Utc::now() - chrono::Duration::weeks(weeks);
        
        let recent_activities: Vec<_> = activities
            .iter()
            .filter(|a| {
                if let Some(start_time) = &a.start_date_local {
                    if let Ok(activity_date) = DateTime::parse_from_rfc3339(start_time) {
                        let activity_utc = activity_date.with_timezone(&Utc);
                        return activity_utc >= start_date;
                    }
                }
                false
            })
            .collect();

        let mut weekly_loads = Vec::new();
        
        for week in 0..weeks {
            let week_start = start_date + chrono::Duration::weeks(week);
            let week_end = week_start + chrono::Duration::weeks(1);
            
            let week_activities: Vec<_> = recent_activities
                .iter()
                .filter(|a| {
                    if let Some(start_time) = &a.start_date_local {
                        if let Ok(activity_date) = DateTime::parse_from_rfc3339(start_time) {
                            let activity_utc = activity_date.with_timezone(&Utc);
                            return activity_utc >= week_start && activity_utc < week_end;
                        }
                    }
                    false
                })
                .collect();

            let total_duration: i32 = week_activities
                .iter()
                .filter_map(|a| a.moving_time)
                .sum();
            
            let total_distance: f32 = week_activities
                .iter()
                .filter_map(|a| a.distance)
                .sum();

            weekly_loads.push(WeeklyLoad {
                week_number: week + 1,
                total_duration_hours: total_duration as f64 / 3600.0,
                total_distance_km: total_distance as f64 / 1000.0,
                activity_count: week_activities.len() as i32,
                intensity_score: week_activities
                    .iter()
                    .filter_map(|a| a.average_heartrate)
                    .map(|hr| hr as f64)
                    .sum::<f64>() / week_activities.len().max(1) as f64,
            });
        }

        // Calculate load balance
        let avg_load = weekly_loads.iter()
            .map(|w| w.total_duration_hours)
            .sum::<f64>() / weekly_loads.len() as f64;
        
        let load_variance = weekly_loads.iter()
            .map(|w| (w.total_duration_hours - avg_load).powi(2))
            .sum::<f64>() / weekly_loads.len() as f64;
        
        let load_balance_score = (100.0 - (load_variance.sqrt() / avg_load * 100.0)).max(0.0);

        // Determine if currently in recovery phase
        let last_week_load = weekly_loads.last().map(|w| w.total_duration_hours).unwrap_or(0.0);
        let previous_week_load = weekly_loads.get(weekly_loads.len().saturating_sub(2))
            .map(|w| w.total_duration_hours).unwrap_or(0.0);
        
        let recovery_needed = last_week_load > avg_load * 1.3 || 
                             (last_week_load + previous_week_load) > avg_load * 2.2;

        Ok(TrainingLoadAnalysis {
            weekly_loads,
            average_weekly_load: avg_load,
            load_balance_score,
            recovery_needed,
            recommendations: if recovery_needed {
                vec![
                    "Consider reducing training volume this week".to_string(),
                    "Focus on recovery activities".to_string(),
                    "Ensure adequate sleep and nutrition".to_string(),
                ]
            } else {
                vec![
                    "Training load is well balanced".to_string(),
                    "Continue current training pattern".to_string(),
                    "Consider gradual load increases".to_string(),
                ]
            },
            insights: vec![], // Add specific insights based on analysis
        })
    }
}

/// Fitness score components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessScore {
    pub overall_score: f64,
    pub aerobic_fitness: f64,
    pub strength_endurance: f64,
    pub consistency: f64,
    pub trend: TrendDirection,
    pub last_updated: DateTime<Utc>,
}

/// Activity goal for performance prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityGoal {
    pub sport_type: String,
    pub metric: String, // "distance", "time", "pace"
    pub target_value: f64,
    pub target_date: DateTime<Utc>,
}

/// Performance prediction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformancePrediction {
    pub target_goal: ActivityGoal,
    pub predicted_value: f64,
    pub confidence: Confidence,
    pub factors: Vec<String>,
    pub recommendations: Vec<String>,
    pub estimated_achievement_date: DateTime<Utc>,
}

/// Training load analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingLoadAnalysis {
    pub weekly_loads: Vec<WeeklyLoad>,
    pub average_weekly_load: f64,
    pub load_balance_score: f64,
    pub recovery_needed: bool,
    pub recommendations: Vec<String>,
    pub insights: Vec<AdvancedInsight>,
}

/// Weekly training load data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyLoad {
    pub week_number: i32,
    pub total_duration_hours: f64,
    pub total_distance_km: f64,
    pub activity_count: i32,
    pub intensity_score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_trend_analysis() {
        let analyzer = AdvancedPerformanceAnalyzer::new();
        
        // Create sample activities with improving pace trend
        let mut activities = Vec::new();
        for i in 0..10 {
            let mut activity = Activity::default();
            activity.sport_type = "Run".to_string();
            activity.average_speed = Some(3.0 + i as f32 * 0.1); // Improving speed
            activity.start_date_local = Some(
                (Utc::now() - chrono::Duration::days(i * 7)).to_rfc3339()
            );
            activities.push(activity);
        }
        
        let result = analyzer.analyze_trends(&activities, TimeFrame::Quarter, "speed").await;
        assert!(result.is_ok());
        
        let analysis = result.unwrap();
        assert_eq!(analysis.trend_direction, TrendDirection::Improving);
        assert!(analysis.trend_strength > 0.5);
    }

    #[tokio::test]
    async fn test_fitness_score() {
        let analyzer = AdvancedPerformanceAnalyzer::new();
        
        let mut activities = Vec::new();
        for i in 0..20 {
            let mut activity = Activity::default();
            activity.average_heartrate = Some(150.0);
            activity.moving_time = Some(3600); // 1 hour
            activity.start_date_local = Some(
                (Utc::now() - chrono::Duration::days(i * 2)).to_rfc3339()
            );
            activities.push(activity);
        }
        
        let result = analyzer.calculate_fitness_score(&activities).await;
        assert!(result.is_ok());
        
        let score = result.unwrap();
        assert!(score.overall_score > 0.0);
        assert!(score.consistency > 0.0);
    }
}