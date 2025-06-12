//! Goal tracking and progress monitoring engine

use super::*;
use crate::models::Activity;
use anyhow::Result;
use chrono::{DateTime, Utc, Duration};
use std::collections::HashMap;
use uuid::Uuid;

/// Trait for goal management and progress tracking
#[async_trait::async_trait]
pub trait GoalEngineTrait {
    /// Suggest goals based on user profile and activity history
    async fn suggest_goals(&self, user_profile: &UserFitnessProfile, activities: &[Activity]) -> Result<Vec<GoalSuggestion>>;
    
    /// Track progress toward a specific goal
    async fn track_progress(&self, goal: &Goal, activities: &[Activity]) -> Result<ProgressReport>;
    
    /// Adjust goal based on current progress and performance
    async fn adjust_goal(&self, goal: &Goal, progress: &ProgressReport) -> Result<Option<GoalAdjustment>>;
    
    /// Create milestone structure for a goal
    async fn create_milestones(&self, goal: &Goal) -> Result<Vec<Milestone>>;
}

/// Advanced goal engine implementation
pub struct AdvancedGoalEngine {
    user_profile: Option<UserFitnessProfile>,
}

impl AdvancedGoalEngine {
    /// Create a new goal engine
    pub fn new() -> Self {
        Self {
            user_profile: None,
        }
    }

    /// Create engine with user profile
    pub fn with_profile(profile: UserFitnessProfile) -> Self {
        Self {
            user_profile: Some(profile),
        }
    }

    /// Calculate goal difficulty based on user's current performance
    fn calculate_goal_difficulty(&self, goal: &Goal, activities: &[Activity]) -> GoalDifficulty {
        let similar_activities: Vec<_> = activities
            .iter()
            .filter(|a| a.sport_type == goal.goal_type.sport_type())
            .collect();

        if similar_activities.is_empty() {
            return GoalDifficulty::Unknown;
        }

        let current_performance = match &goal.goal_type {
            GoalType::Distance { .. } => {
                let avg_distance = similar_activities
                    .iter()
                    .filter_map(|a| a.distance)
                    .sum::<f32>() / similar_activities.len() as f32;
                avg_distance as f64
            },
            GoalType::Time { distance, .. } => {
                let similar_distance_activities: Vec<_> = similar_activities
                    .iter()
                    .filter(|a| {
                        if let Some(d) = a.distance {
                            (d as f64 - distance).abs() < distance * 0.2 // Within 20%
                        } else {
                            false
                        }
                    })
                    .collect();
                
                if similar_distance_activities.is_empty() {
                    return GoalDifficulty::Unknown;
                }
                
                let avg_time = similar_distance_activities
                    .iter()
                    .filter_map(|a| a.moving_time)
                    .sum::<i32>() / similar_distance_activities.len() as i32;
                avg_time as f64
            },
            GoalType::Performance { .. } => {
                // Use average speed as performance metric
                let avg_speed = similar_activities
                    .iter()
                    .filter_map(|a| a.average_speed)
                    .sum::<f32>() / similar_activities.len() as f32;
                avg_speed as f64
            },
            GoalType::Frequency { sessions_per_week, .. } => {
                // Calculate current weekly frequency
                let weeks = 4;
                let recent_count = similar_activities
                    .iter()
                    .filter(|a| {
                        if let Some(start_time) = &a.start_date_local {
                            if let Ok(activity_date) = DateTime::parse_from_rfc3339(start_time) {
                                let activity_utc = activity_date.with_timezone(&Utc);
                                let weeks_ago = (Utc::now() - activity_utc).num_weeks();
                                return weeks_ago <= weeks;
                            }
                        }
                        false
                    })
                    .count();
                (recent_count as f64) / (weeks as f64)
            },
            GoalType::Custom { .. } => {
                return GoalDifficulty::Unknown;
            },
        };

        let improvement_ratio = goal.target_value / current_performance;
        
        if improvement_ratio < 1.1 {
            GoalDifficulty::Easy
        } else if improvement_ratio < 1.3 {
            GoalDifficulty::Moderate
        } else if improvement_ratio < 1.5 {
            GoalDifficulty::Challenging
        } else {
            GoalDifficulty::Ambitious
        }
    }

    /// Generate progress insights based on current status
    fn generate_progress_insights(&self, goal: &Goal, progress: &ProgressReport) -> Vec<AdvancedInsight> {
        let mut insights = Vec::new();
        
        // Progress rate insight
        let days_elapsed = (Utc::now() - goal.created_at).num_days() as f64;
        let days_total = (goal.target_date - goal.created_at).num_days() as f64;
        let time_progress = days_elapsed / days_total;
        
        if progress.progress_percentage > time_progress * 100.0 + 10.0 {
            insights.push(AdvancedInsight {
                insight_type: "ahead_of_schedule".to_string(),
                message: "You're ahead of schedule! Excellent progress.".to_string(),
                confidence: Confidence::High,
                severity: InsightSeverity::Info,
                metadata: HashMap::new(),
            });
        } else if progress.progress_percentage < time_progress * 100.0 - 10.0 {
            insights.push(AdvancedInsight {
                insight_type: "behind_schedule".to_string(),
                message: "Progress is behind schedule - consider adjusting training plan.".to_string(),
                confidence: Confidence::High,
                severity: InsightSeverity::Warning,
                metadata: HashMap::new(),
            });
        }
        
        // Milestone achievement insight
        let achieved_milestones = progress.milestones_achieved.iter().filter(|m| m.achieved).count();
        let total_milestones = progress.milestones_achieved.len();
        
        if achieved_milestones > total_milestones / 2 {
            insights.push(AdvancedInsight {
                insight_type: "milestone_progress".to_string(),
                message: format!("Great milestone progress: {}/{} completed", achieved_milestones, total_milestones),
                confidence: Confidence::Medium,
                severity: InsightSeverity::Info,
                metadata: HashMap::new(),
            });
        }
        
        insights
    }
}

#[async_trait::async_trait]
impl GoalEngineTrait for AdvancedGoalEngine {
    async fn suggest_goals(&self, user_profile: &UserFitnessProfile, activities: &[Activity]) -> Result<Vec<GoalSuggestion>> {
        let mut suggestions = Vec::new();
        
        // Analyze current activity patterns
        let recent_activities: Vec<_> = activities
            .iter()
            .filter(|a| {
                if let Some(start_time) = &a.start_date_local {
                    if let Ok(activity_date) = DateTime::parse_from_rfc3339(start_time) {
                        let activity_utc = activity_date.with_timezone(&Utc);
                        let weeks_ago = (Utc::now() - activity_utc).num_weeks();
                        return weeks_ago <= 8; // Last 8 weeks
                    }
                }
                false
            })
            .collect();

        // Group activities by sport
        let mut sport_stats: HashMap<String, SportStats> = HashMap::new();
        
        for activity in &recent_activities {
            let sport = &activity.sport_type;
            let stats = sport_stats.entry(sport.clone()).or_insert(SportStats::new());
            
            stats.activity_count += 1;
            if let Some(distance) = activity.distance {
                stats.total_distance += distance as f64;
                stats.max_distance = stats.max_distance.max(distance as f64);
            }
            if let Some(duration) = activity.moving_time {
                stats.total_duration += duration as f64;
                stats.max_duration = stats.max_duration.max(duration as f64);
            }
            if let Some(speed) = activity.average_speed {
                stats.speeds.push(speed as f64);
            }
        }

        // Generate suggestions for each sport
        for (sport, stats) in sport_stats {
            if stats.activity_count < 3 {
                continue; // Need more data
            }

            let avg_distance = stats.total_distance / stats.activity_count as f64;
            let avg_speed = if !stats.speeds.is_empty() {
                stats.speeds.iter().sum::<f64>() / stats.speeds.len() as f64
            } else {
                0.0
            };

            // Distance goal suggestions
            if avg_distance > 0.0 {
                let target_distance = stats.max_distance * 1.2; // 20% increase
                suggestions.push(GoalSuggestion {
                    goal_type: GoalType::Distance {
                        sport: sport.clone(),
                        timeframe: TimeFrame::Month,
                    },
                    suggested_target: target_distance,
                    rationale: format!("Based on your recent {} activities, you could challenge yourself with a longer distance", sport),
                    difficulty: GoalDifficulty::Moderate,
                    estimated_timeline_days: 30,
                    success_probability: 0.75,
                });
            }

            // Performance goal suggestions  
            if avg_speed > 0.0 {
                let target_improvement = 5.0; // 5% improvement
                suggestions.push(GoalSuggestion {
                    goal_type: GoalType::Performance {
                        metric: "speed".to_string(),
                        improvement_percent: target_improvement,
                    },
                    suggested_target: avg_speed * (1.0 + target_improvement / 100.0),
                    rationale: format!("Improve your average {} pace by {}%", sport, target_improvement),
                    difficulty: GoalDifficulty::Challenging,
                    estimated_timeline_days: 60,
                    success_probability: 0.65,
                });
            }

            // Frequency goal suggestions
            let current_frequency = stats.activity_count as f64 / 8.0; // Per week over 8 weeks
            if current_frequency < 5.0 {
                let target_frequency = (current_frequency + 1.0).min(5.0) as i32;
                suggestions.push(GoalSuggestion {
                    goal_type: GoalType::Frequency {
                        sport: sport.clone(),
                        sessions_per_week: target_frequency,
                    },
                    suggested_target: target_frequency as f64,
                    rationale: format!("Increase {} training consistency", sport),
                    difficulty: GoalDifficulty::Moderate,
                    estimated_timeline_days: 28,
                    success_probability: 0.80,
                });
            }
        }

        // Fitness level specific suggestions
        match user_profile.fitness_level {
            FitnessLevel::Beginner => {
                suggestions.push(GoalSuggestion {
                    goal_type: GoalType::Custom {
                        metric: "consistency".to_string(),
                        unit: "weeks".to_string(),
                    },
                    suggested_target: 4.0,
                    rationale: "Build a consistent exercise habit".to_string(),
                    difficulty: GoalDifficulty::Easy,
                    estimated_timeline_days: 28,
                    success_probability: 0.85,
                });
            },
            FitnessLevel::Advanced | FitnessLevel::Elite => {
                suggestions.push(GoalSuggestion {
                    goal_type: GoalType::Custom {
                        metric: "training_zones".to_string(),
                        unit: "percentage".to_string(),
                    },
                    suggested_target: 80.0,
                    rationale: "Optimize training zone distribution".to_string(),
                    difficulty: GoalDifficulty::Challenging,
                    estimated_timeline_days: 84,
                    success_probability: 0.60,
                });
            },
            _ => {},
        }

        // Sort by success probability and difficulty
        suggestions.sort_by(|a, b| {
            b.success_probability.partial_cmp(&a.success_probability).unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(suggestions.into_iter().take(5).collect()) // Return top 5 suggestions
    }

    async fn track_progress(&self, goal: &Goal, activities: &[Activity]) -> Result<ProgressReport> {
        // Filter relevant activities since goal creation
        let relevant_activities: Vec<_> = activities
            .iter()
            .filter(|a| {
                // Must be same sport type
                if a.sport_type != goal.goal_type.sport_type() {
                    return false;
                }
                
                // Must be after goal creation
                if let Some(start_time) = &a.start_date_local {
                    if let Ok(activity_date) = DateTime::parse_from_rfc3339(start_time) {
                        let activity_utc = activity_date.with_timezone(&Utc);
                        return activity_utc >= goal.created_at;
                    }
                }
                false
            })
            .collect();

        // Calculate current progress based on goal type
        let current_value = match &goal.goal_type {
            GoalType::Distance { timeframe, .. } => {
                let timeframe_start = match timeframe {
                    TimeFrame::Week => Utc::now() - Duration::weeks(1),
                    TimeFrame::Month => Utc::now() - Duration::days(30),
                    TimeFrame::Quarter => Utc::now() - Duration::days(90),
                    _ => goal.created_at,
                };
                
                relevant_activities
                    .iter()
                    .filter(|a| {
                        if let Some(start_time) = &a.start_date_local {
                            if let Ok(activity_date) = DateTime::parse_from_rfc3339(start_time) {
                                let activity_utc = activity_date.with_timezone(&Utc);
                                return activity_utc >= timeframe_start;
                            }
                        }
                        false
                    })
                    .filter_map(|a| a.distance)
                    .sum::<f32>() as f64
            },
            GoalType::Time { distance, .. } => {
                // Find best time for target distance
                relevant_activities
                    .iter()
                    .filter(|a| {
                        if let Some(d) = a.distance {
                            (d as f64 - distance).abs() < distance * 0.05 // Within 5%
                        } else {
                            false
                        }
                    })
                    .filter_map(|a| a.moving_time)
                    .min()
                    .unwrap_or(i32::MAX) as f64
            },
            GoalType::Frequency { sessions_per_week, .. } => {
                let weeks_elapsed = (Utc::now() - goal.created_at).num_weeks().max(1);
                relevant_activities.len() as f64 / weeks_elapsed as f64
            },
            GoalType::Performance { metric, .. } => {
                match metric.as_str() {
                    "speed" => {
                        if let Some(latest_activity) = relevant_activities.last() {
                            latest_activity.average_speed.unwrap_or(0.0) as f64
                        } else {
                            0.0
                        }
                    },
                    _ => 0.0,
                }
            },
            GoalType::Custom { .. } => goal.current_value,
        };

        // Calculate progress percentage
        let progress_percentage = if goal.target_value > 0.0 {
            (current_value / goal.target_value * 100.0).min(100.0)
        } else {
            0.0
        };

        // Create milestones
        let milestones = self.create_milestones(goal).await?;

        // Check milestone achievements
        let mut achieved_milestones = milestones.clone();
        for milestone in &mut achieved_milestones {
            if current_value >= milestone.target_value {
                milestone.achieved = true;
                milestone.achieved_date = Some(Utc::now());
            }
        }

        // Estimate completion date
        let completion_date_estimate = if progress_percentage > 0.0 {
            let days_elapsed = (Utc::now() - goal.created_at).num_days();
            let estimated_total_days = (days_elapsed as f64 / progress_percentage * 100.0) as i64;
            Some(goal.created_at + Duration::days(estimated_total_days))
        } else {
            None
        };

        // Determine if on track
        let days_elapsed = (Utc::now() - goal.created_at).num_days() as f64;
        let days_total = (goal.target_date - goal.created_at).num_days() as f64;
        let expected_progress = if days_total > 0.0 {
            (days_elapsed / days_total) * 100.0
        } else {
            0.0
        };
        let on_track = progress_percentage >= expected_progress - 10.0; // 10% tolerance

        let progress_report = ProgressReport {
            goal_id: goal.id.clone(),
            progress_percentage,
            completion_date_estimate,
            milestones_achieved: achieved_milestones,
            insights: vec![], // Will be filled next
            recommendations: vec![], // Will be filled next  
            on_track,
        };

        let mut final_report = progress_report;
        final_report.insights = self.generate_progress_insights(goal, &final_report);
        
        // Generate recommendations
        final_report.recommendations = if on_track {
            vec![
                "Maintain current training consistency".to_string(),
                "Continue following your current plan".to_string(),
            ]
        } else {
            vec![
                "Consider increasing training frequency".to_string(),
                "Focus on goal-specific activities".to_string(),
                "Review and adjust your training plan".to_string(),
            ]
        };

        Ok(final_report)
    }

    async fn adjust_goal(&self, goal: &Goal, progress: &ProgressReport) -> Result<Option<GoalAdjustment>> {
        let days_elapsed = (Utc::now() - goal.created_at).num_days() as f64;
        let days_total = (goal.target_date - goal.created_at).num_days() as f64;
        let time_progress = days_elapsed / days_total;

        // Only suggest adjustments if we're past 25% of the timeline
        if time_progress < 0.25 {
            return Ok(None);
        }

        let progress_ratio = progress.progress_percentage / (time_progress * 100.0);

        let adjustment = if progress_ratio > 1.3 {
            // Significantly ahead - suggest more ambitious goal
            Some(GoalAdjustment {
                adjustment_type: AdjustmentType::IncreaseTarget,
                new_target_value: goal.target_value * 1.2,
                rationale: "You're making excellent progress! Consider a more ambitious target.".to_string(),
                confidence: Confidence::Medium,
            })
        } else if progress_ratio < 0.7 {
            // Significantly behind - suggest more realistic goal or extended timeline
            if days_total - days_elapsed > 30.0 {
                // Enough time left - reduce target
                Some(GoalAdjustment {
                    adjustment_type: AdjustmentType::DecreaseTarget,
                    new_target_value: goal.target_value * 0.8,
                    rationale: "Consider adjusting to a more achievable target based on current progress.".to_string(),
                    confidence: Confidence::High,
                })
            } else {
                // Extend timeline
                Some(GoalAdjustment {
                    adjustment_type: AdjustmentType::ExtendDeadline,
                    new_target_value: goal.target_value,
                    rationale: "Consider extending the deadline to maintain motivation.".to_string(),
                    confidence: Confidence::Medium,
                })
            }
        } else {
            None // Progress is reasonable
        };

        Ok(adjustment)
    }

    async fn create_milestones(&self, goal: &Goal) -> Result<Vec<Milestone>> {
        let mut milestones = Vec::new();
        
        // Create 4 milestones at 25%, 50%, 75%, and 100%
        let percentages = [25.0, 50.0, 75.0, 100.0];
        let names = ["First Quarter", "Halfway Point", "Three Quarters", "Goal Complete"];
        
        for (i, &percentage) in percentages.iter().enumerate() {
            milestones.push(Milestone {
                name: names[i].to_string(),
                target_value: goal.target_value * (percentage / 100.0),
                achieved_date: None,
                achieved: false,
            });
        }
        
        Ok(milestones)
    }
}

/// Goal suggestion with rationale
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalSuggestion {
    pub goal_type: GoalType,
    pub suggested_target: f64,
    pub rationale: String,
    pub difficulty: GoalDifficulty,
    pub estimated_timeline_days: i32,
    pub success_probability: f64,
}

/// Goal difficulty levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalDifficulty {
    Easy,
    Moderate,
    Challenging,
    Ambitious,
    Unknown,
}

/// Goal adjustment suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalAdjustment {
    pub adjustment_type: AdjustmentType,
    pub new_target_value: f64,
    pub rationale: String,
    pub confidence: Confidence,
}

/// Types of goal adjustments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdjustmentType {
    IncreaseTarget,
    DecreaseTarget,
    ExtendDeadline,
    ChangeApproach,
}

/// Statistics for a sport type
#[derive(Debug)]
struct SportStats {
    activity_count: usize,
    total_distance: f64,
    max_distance: f64,
    total_duration: f64,
    max_duration: f64,
    speeds: Vec<f64>,
}

impl SportStats {
    fn new() -> Self {
        Self {
            activity_count: 0,
            total_distance: 0.0,
            max_distance: 0.0,
            total_duration: 0.0,
            max_duration: 0.0,
            speeds: Vec::new(),
        }
    }
}

impl GoalType {
    /// Get the sport type for this goal
    pub fn sport_type(&self) -> String {
        match self {
            GoalType::Distance { sport, .. } => sport.clone(),
            GoalType::Time { sport, .. } => sport.clone(),
            GoalType::Frequency { sport, .. } => sport.clone(),
            GoalType::Performance { .. } => "Any".to_string(),
            GoalType::Custom { .. } => "Any".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_goal_suggestions() {
        let profile = UserFitnessProfile {
            user_id: "test_user".to_string(),
            age: Some(30),
            gender: Some("M".to_string()),
            weight: Some(70.0),
            height: Some(175.0),
            fitness_level: FitnessLevel::Intermediate,
            primary_sports: vec!["Run".to_string()],
            training_history_months: 12,
            preferences: UserPreferences {
                preferred_units: "metric".to_string(),
                training_focus: vec!["endurance".to_string()],
                injury_history: vec![],
                time_availability: TimeAvailability {
                    hours_per_week: 5.0,
                    preferred_days: vec!["Monday".to_string(), "Wednesday".to_string(), "Friday".to_string()],
                    preferred_duration_minutes: Some(60),
                },
            },
        };
        
        let engine = AdvancedGoalEngine::with_profile(profile.clone());
        
        // Create sample activities
        let mut activities = Vec::new();
        for i in 0..10 {
            let mut activity = Activity::default();
            activity.sport_type = "Run".to_string();
            activity.distance = Some(5000.0); // 5km runs
            activity.moving_time = Some(1800); // 30 minutes
            activity.start_date_local = Some(
                (Utc::now() - chrono::Duration::days(i * 3)).to_rfc3339()
            );
            activities.push(activity);
        }
        
        let result = engine.suggest_goals(&profile, &activities).await;
        assert!(result.is_ok());
        
        let suggestions = result.unwrap();
        assert!(!suggestions.is_empty());
    }

    #[tokio::test]
    async fn test_progress_tracking() {
        let goal = Goal {
            id: Uuid::new_v4().to_string(),
            user_id: "test_user".to_string(),
            title: "Run 100km this month".to_string(),
            description: "Monthly distance goal".to_string(),
            goal_type: GoalType::Distance {
                sport: "Run".to_string(),
                timeframe: TimeFrame::Month,
            },
            target_value: 100000.0, // 100km in meters
            target_date: Utc::now() + Duration::days(30),
            current_value: 0.0,
            created_at: Utc::now() - Duration::days(10),
            updated_at: Utc::now(),
            status: GoalStatus::Active,
        };
        
        let engine = AdvancedGoalEngine::new();
        
        // Create activities that add up to 30km
        let mut activities = Vec::new();
        for i in 0..6 {
            let mut activity = Activity::default();
            activity.sport_type = "Run".to_string();
            activity.distance = Some(5000.0); // 5km each
            activity.start_date_local = Some(
                (Utc::now() - Duration::days(5 - i)).to_rfc3339()
            );
            activities.push(activity);
        }
        
        let result = engine.track_progress(&goal, &activities).await;
        assert!(result.is_ok());
        
        let progress = result.unwrap();
        assert_eq!(progress.progress_percentage, 30.0); // 30km out of 100km
        assert_eq!(progress.milestones_achieved[0].achieved, true); // 25% milestone
    }
}