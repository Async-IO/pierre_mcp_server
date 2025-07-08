// ABOUTME: Goal tracking and progress monitoring engine for fitness objectives
// ABOUTME: Tracks training goals, milestones, progress metrics, and provides achievement insights
//! Goal tracking and progress monitoring engine

use super::{
    AdvancedInsight, Confidence, Deserialize, FitnessLevel, Goal, GoalStatus, GoalType,
    InsightSeverity, Milestone, ProgressReport, Serialize, TimeFrame, UserFitnessProfile,
};
use crate::config::intelligence_config::{
    GoalEngineConfig, IntelligenceConfig, IntelligenceStrategy,
};
use crate::intelligence::physiological_constants::{
    consistency::{
        MILESTONE_ACHIEVEMENT_THRESHOLD, MIN_ACTIVITY_COUNT_FOR_ANALYSIS,
        PROGRESS_TOLERANCE_PERCENTAGE,
    },
    frequency_targets::{MAX_WEEKLY_FREQUENCY, TARGET_PERFORMANCE_IMPROVEMENT},
    goal_difficulty::{
        CHALLENGING_GOAL_RATIO, EASY_GOAL_RATIO, GOAL_DISTANCE_PRECISION, GOAL_DISTANCE_TOLERANCE,
        MODERATE_GOAL_RATIO,
    },
    goal_progress::{
        AHEAD_OF_SCHEDULE_THRESHOLD, BEHIND_SCHEDULE_THRESHOLD, TARGET_DECREASE_MULTIPLIER,
        TARGET_INCREASE_MULTIPLIER,
    },
    milestones::{MILESTONE_NAMES, MILESTONE_PERCENTAGES},
    time_periods::{
        GOAL_ADJUSTMENT_THRESHOLD, GOAL_ANALYSIS_WEEKS, GOAL_DAYS_REMAINING_THRESHOLD,
        TRAINING_PATTERN_ANALYSIS_WEEKS,
    },
};
use crate::models::Activity;
use anyhow::Result;
use chrono::{Duration, Utc};
use std::collections::HashMap;

/// Trait for goal management and progress tracking
#[async_trait::async_trait]
pub trait GoalEngineTrait {
    /// Suggest goals based on user profile and activity history
    async fn suggest_goals(
        &self,
        user_profile: &UserFitnessProfile,
        activities: &[Activity],
    ) -> Result<Vec<GoalSuggestion>>;

    /// Track progress toward a specific goal
    async fn track_progress(&self, goal: &Goal, activities: &[Activity]) -> Result<ProgressReport>;

    /// Adjust goal based on current progress and performance
    async fn adjust_goal(
        &self,
        goal: &Goal,
        progress: &ProgressReport,
    ) -> Result<Option<GoalAdjustment>>;

    /// Create milestone structure for a goal
    async fn create_milestones(&self, goal: &Goal) -> Result<Vec<Milestone>>;
}

/// Advanced goal engine implementation with configurable strategy
pub struct AdvancedGoalEngine<
    S: IntelligenceStrategy = crate::config::intelligence_config::DefaultStrategy,
> {
    strategy: S,
    config: GoalEngineConfig,
    user_profile: Option<UserFitnessProfile>,
}

impl Default for AdvancedGoalEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AdvancedGoalEngine {
    /// Create a new goal engine with default strategy
    #[must_use]
    pub fn new() -> Self {
        let global_config = IntelligenceConfig::global();
        Self {
            strategy: crate::config::intelligence_config::DefaultStrategy,
            config: global_config.goal_engine.clone(),
            user_profile: None,
        }
    }
}

impl<S: IntelligenceStrategy> AdvancedGoalEngine<S> {
    /// Create with custom strategy
    #[must_use]
    pub fn with_strategy(strategy: S) -> Self {
        let global_config = IntelligenceConfig::global();
        Self {
            strategy,
            config: global_config.goal_engine.clone(),
            user_profile: None,
        }
    }

    /// Create with custom configuration
    #[must_use]
    pub const fn with_config(strategy: S, config: GoalEngineConfig) -> Self {
        Self {
            strategy,
            config,
            user_profile: None,
        }
    }

    /// Create engine with user profile
    /// Create goal engine with user profile using default strategy
    #[must_use]
    pub fn with_profile(profile: UserFitnessProfile) -> AdvancedGoalEngine {
        let global_config = IntelligenceConfig::global();
        AdvancedGoalEngine {
            strategy: crate::config::intelligence_config::DefaultStrategy,
            config: global_config.goal_engine.clone(),
            user_profile: Some(profile),
        }
    }

    /// Set user profile for this engine
    pub fn set_profile(&mut self, profile: UserFitnessProfile) {
        self.user_profile = Some(profile);
    }

    /// Calculate goal difficulty based on user's current performance
    fn calculate_goal_difficulty(goal: &Goal, activities: &[Activity]) -> GoalDifficulty {
        let similar_activities: Vec<_> = activities
            .iter()
            .filter(|a| format!("{:?}", a.sport_type) == goal.goal_type.sport_type())
            .collect();

        if similar_activities.is_empty() {
            return GoalDifficulty::Unknown;
        }

        let current_performance = match &goal.goal_type {
            GoalType::Distance { .. } => {
                let avg_distance = similar_activities
                    .iter()
                    .filter_map(|a| a.distance_meters)
                    .sum::<f64>()
                    / f64::from(u32::try_from(similar_activities.len()).unwrap_or(u32::MAX));
                avg_distance
            }
            GoalType::Time { distance, .. } => {
                let similar_distance_activities: Vec<_> = similar_activities
                    .iter()
                    .filter(|a| {
                        a.distance_meters.is_some_and(|d| {
                            (d - distance).abs() < distance * GOAL_DISTANCE_TOLERANCE
                        })
                    })
                    .collect();

                if similar_distance_activities.is_empty() {
                    return GoalDifficulty::Unknown;
                }

                let avg_time = similar_distance_activities
                    .iter()
                    .map(|a| a.duration_seconds)
                    .sum::<u64>()
                    / u64::try_from(similar_distance_activities.len()).unwrap_or(1);
                // Safe conversion for time values
                if avg_time > u64::from(u32::MAX) {
                    f64::from(u32::MAX)
                } else {
                    f64::from(u32::try_from(avg_time).unwrap_or(u32::MAX))
                }
            }
            GoalType::Performance { .. } => {
                // Use average speed as performance metric
                let avg_speed = similar_activities
                    .iter()
                    .filter_map(|a| a.average_speed)
                    .sum::<f64>()
                    / f64::from(u32::try_from(similar_activities.len()).unwrap_or(u32::MAX));
                avg_speed
            }
            GoalType::Frequency { .. } => {
                // Calculate current weekly frequency
                let weeks = 4;
                let recent_count = similar_activities
                    .iter()
                    .filter(|a| {
                        let activity_utc = a.start_date;
                        let weeks_ago = (Utc::now() - activity_utc).num_weeks();
                        weeks_ago <= TRAINING_PATTERN_ANALYSIS_WEEKS
                    })
                    .count();
                f64::from(u32::try_from(recent_count).unwrap_or(u32::MAX)) / f64::from(weeks)
            }
            GoalType::Custom { .. } => {
                return GoalDifficulty::Unknown;
            }
        };

        let improvement_ratio = goal.target_value / current_performance;

        if improvement_ratio < EASY_GOAL_RATIO {
            GoalDifficulty::Easy
        } else if improvement_ratio < MODERATE_GOAL_RATIO {
            GoalDifficulty::Moderate
        } else if improvement_ratio < CHALLENGING_GOAL_RATIO {
            GoalDifficulty::Challenging
        } else {
            GoalDifficulty::Ambitious
        }
    }

    /// Generate progress insights based on current status
    fn generate_progress_insights(goal: &Goal, progress: &ProgressReport) -> Vec<AdvancedInsight> {
        let mut insights = Vec::new();

        // Progress rate insight
        let days_elapsed =
            f64::from(i32::try_from((Utc::now() - goal.created_at).num_days()).unwrap_or(0));
        let days_total =
            f64::from(i32::try_from((goal.target_date - goal.created_at).num_days()).unwrap_or(1));
        let time_progress = days_elapsed / days_total;

        if progress.progress_percentage
            > time_progress.mul_add(100.0, PROGRESS_TOLERANCE_PERCENTAGE)
        {
            insights.push(AdvancedInsight {
                insight_type: "ahead_of_schedule".into(),
                message: "You're ahead of schedule! Excellent progress.".into(),
                confidence: Confidence::High,
                severity: InsightSeverity::Info,
                metadata: HashMap::new(),
            });
        } else if progress.progress_percentage
            < time_progress.mul_add(100.0, -PROGRESS_TOLERANCE_PERCENTAGE)
        {
            insights.push(AdvancedInsight {
                insight_type: "behind_schedule".into(),
                message: "Progress is behind schedule - consider adjusting training plan.".into(),
                confidence: Confidence::High,
                severity: InsightSeverity::Warning,
                metadata: HashMap::new(),
            });
        }

        // Milestone achievement insight
        let achieved_milestones = progress
            .milestones_achieved
            .iter()
            .filter(|m| m.achieved)
            .count();
        let total_milestones = progress.milestones_achieved.len();

        if f64::from(u32::try_from(achieved_milestones).unwrap_or(u32::MAX))
            > f64::from(u32::try_from(total_milestones).unwrap_or(u32::MAX))
                * MILESTONE_ACHIEVEMENT_THRESHOLD
        {
            insights.push(AdvancedInsight {
                insight_type: "milestone_progress".into(),
                message: format!(
                    "Great milestone progress: {achieved_milestones}/{total_milestones} completed"
                ),
                confidence: Confidence::Medium,
                severity: InsightSeverity::Info,
                metadata: HashMap::new(),
            });
        }

        insights
    }
}

#[async_trait::async_trait]
impl<S: IntelligenceStrategy> GoalEngineTrait for AdvancedGoalEngine<S> {
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::too_many_lines
    )]
    async fn suggest_goals(
        &self,
        user_profile: &UserFitnessProfile,
        activities: &[Activity],
    ) -> Result<Vec<GoalSuggestion>> {
        let mut suggestions = Vec::new();

        // Analyze current activity patterns
        let recent_activities: Vec<_> = activities
            .iter()
            .filter(|a| {
                let activity_utc = a.start_date;
                let weeks_ago = (Utc::now() - activity_utc).num_weeks();
                weeks_ago <= GOAL_ANALYSIS_WEEKS
            })
            .collect();

        // Group activities by sport
        let mut sport_stats: HashMap<String, SportStats> = HashMap::new();

        for activity in &recent_activities {
            let sport = format!("{:?}", activity.sport_type);
            let stats = sport_stats.entry(sport).or_insert_with(SportStats::new);

            stats.activity_count += 1;
            if let Some(distance) = activity.distance_meters {
                stats.total_distance += distance;
                stats.max_distance = stats.max_distance.max(distance);
            }
            let duration = activity.duration_seconds;
            stats.total_duration += if duration > u64::from(u32::MAX) {
                f64::from(u32::MAX)
            } else {
                f64::from(u32::try_from(duration).unwrap_or(u32::MAX))
            };
            stats.max_duration = stats.max_duration.max(if duration > u64::from(u32::MAX) {
                f64::from(u32::MAX)
            } else {
                f64::from(u32::try_from(duration).unwrap_or(u32::MAX))
            });

            if let Some(speed) = activity.average_speed {
                stats.speeds.push(speed);
            }
        }

        // Generate suggestions for each sport
        for (sport, stats) in sport_stats {
            if stats.activity_count < MIN_ACTIVITY_COUNT_FOR_ANALYSIS {
                continue; // Need more data
            }

            let avg_distance = stats.total_distance
                / f64::from(u32::try_from(stats.activity_count).unwrap_or(u32::MAX));
            let avg_speed = if stats.speeds.is_empty() {
                0.0
            } else {
                stats.speeds.iter().sum::<f64>()
                    / f64::from(u32::try_from(stats.speeds.len()).unwrap_or(u32::MAX))
            };

            // Distance goal suggestions
            if avg_distance > 0.0 {
                // Use config multiplier and strategy thresholds
                let base_multiplier = self
                    .config
                    .feasibility
                    .conservative_multiplier
                    .max(TARGET_INCREASE_MULTIPLIER);

                // Apply strategy-based adjustments
                let weekly_distance = stats.total_distance
                    / f64::from(i32::try_from(GOAL_ANALYSIS_WEEKS).unwrap_or(i32::MAX));
                let strategy_multiplier = if self
                    .strategy
                    .should_recommend_volume_increase(weekly_distance / 1000.0)
                {
                    base_multiplier * 1.2 // More aggressive for low-volume athletes
                } else {
                    base_multiplier
                };

                let target_distance = stats.max_distance * strategy_multiplier;

                let distance_goal = Goal {
                    id: format!("dist_{sport}_{}", Utc::now().timestamp()),
                    user_id: "system".into(), // Will be set by caller
                    title: format!("Increase {sport} Distance"),
                    description: format!(
                        "Target distance of {:.1} km for {sport}",
                        target_distance / 1000.0
                    ),
                    goal_type: GoalType::Distance {
                        sport: sport.clone(),
                        timeframe: TimeFrame::Month,
                    },
                    target_value: target_distance,
                    target_date: Utc::now() + chrono::Duration::days(30),
                    current_value: 0.0,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    status: GoalStatus::Active,
                };

                // Calculate actual difficulty using strategy and user data
                let difficulty = Self::calculate_goal_difficulty(&distance_goal, activities);

                suggestions.push(GoalSuggestion {
                    goal_type: distance_goal.goal_type,
                    suggested_target: target_distance,
                    rationale: format!("Based on your recent {sport} activities, you could challenge yourself with a longer distance"),
                    difficulty,
                    estimated_timeline_days: 30,
                    success_probability: self.config.feasibility.min_success_probability,
                });
            }

            // Performance goal suggestions
            if avg_speed > 0.0 {
                let target_improvement = TARGET_PERFORMANCE_IMPROVEMENT;
                suggestions.push(GoalSuggestion {
                    goal_type: GoalType::Performance {
                        metric: "speed".into(),
                        improvement_percent: target_improvement,
                    },
                    suggested_target: avg_speed.mul_add(target_improvement / 100.0, avg_speed),
                    rationale: format!(
                        "Improve your average {sport} pace by {target_improvement}%"
                    ),
                    difficulty: GoalDifficulty::Challenging,
                    estimated_timeline_days: 60,
                    success_probability: 0.65,
                });
            }

            // Frequency goal suggestions
            let current_frequency =
                f64::from(u32::try_from(stats.activity_count).unwrap_or(u32::MAX))
                    / f64::from(i32::try_from(GOAL_ANALYSIS_WEEKS).unwrap_or(i32::MAX));
            if current_frequency < MAX_WEEKLY_FREQUENCY {
                let target_frequency = (current_frequency + 1.0).min(MAX_WEEKLY_FREQUENCY) as i32;
                suggestions.push(GoalSuggestion {
                    goal_type: GoalType::Frequency {
                        sport: sport.clone(),
                        sessions_per_week: target_frequency,
                    },
                    suggested_target: f64::from(target_frequency),
                    rationale: format!("Increase {sport} training consistency"),
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
                        metric: "consistency".into(),
                        unit: "weeks".into(),
                    },
                    suggested_target: 4.0,
                    rationale: "Build a consistent exercise habit".into(),
                    difficulty: GoalDifficulty::Easy,
                    estimated_timeline_days: 28,
                    success_probability: 0.85,
                });
            }
            FitnessLevel::Advanced | FitnessLevel::Elite => {
                suggestions.push(GoalSuggestion {
                    goal_type: GoalType::Custom {
                        metric: "training_zones".into(),
                        unit: "percentage".into(),
                    },
                    suggested_target: 80.0,
                    rationale: "Optimize training zone distribution".into(),
                    difficulty: GoalDifficulty::Challenging,
                    estimated_timeline_days: 84,
                    success_probability: 0.60,
                });
            }
            FitnessLevel::Intermediate => {}
        }

        // Sort by success probability and difficulty
        suggestions.sort_by(|a, b| {
            b.success_probability
                .partial_cmp(&a.success_probability)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(suggestions.into_iter().take(5).collect()) // Return top 5 suggestions
    }

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::too_many_lines
    )]
    async fn track_progress(&self, goal: &Goal, activities: &[Activity]) -> Result<ProgressReport> {
        // Filter relevant activities since goal creation
        let relevant_activities: Vec<_> = activities
            .iter()
            .filter(|a| {
                // Must be same sport type
                if format!("{:?}", a.sport_type) != goal.goal_type.sport_type() {
                    return false;
                }

                // Must be after goal creation
                let activity_utc = a.start_date;
                activity_utc >= goal.created_at
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
                        let activity_utc = a.start_date;
                        activity_utc >= timeframe_start
                    })
                    .filter_map(|a| a.distance_meters)
                    .sum::<f64>()
            }
            GoalType::Time { distance, .. } => {
                // Find best time for target distance
                relevant_activities
                    .iter()
                    .filter(|a| {
                        a.distance_meters.is_some_and(|d| {
                            (d - distance).abs() < distance * GOAL_DISTANCE_PRECISION
                        })
                    })
                    .map(|a| a.duration_seconds)
                    .min()
                    .map_or(f64::MAX, |v| {
                        if v > u64::from(u32::MAX) {
                            f64::from(u32::MAX)
                        } else {
                            f64::from(u32::try_from(v).unwrap_or(u32::MAX))
                        }
                    })
            }
            GoalType::Frequency { .. } => {
                let weeks_elapsed = (Utc::now() - goal.created_at).num_weeks().max(1);
                f64::from(u32::try_from(relevant_activities.len()).unwrap_or(u32::MAX))
                    / f64::from(i32::try_from(weeks_elapsed).unwrap_or(i32::MAX))
            }
            GoalType::Performance { metric, .. } => match metric.as_str() {
                "speed" => relevant_activities
                    .last()
                    .and_then(|a| a.average_speed)
                    .unwrap_or(0.0),
                _ => 0.0,
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
        let mut achieved_milestones = milestones;
        for milestone in &mut achieved_milestones {
            if current_value >= milestone.target_value {
                milestone.achieved = true;
                milestone.achieved_date = Some(Utc::now());
            }
        }

        // Estimate completion date
        let completion_date_estimate = if progress_percentage > 0.0 {
            let days_elapsed = (Utc::now() - goal.created_at).num_days();
            let estimated_total_days = (f64::from(i32::try_from(days_elapsed).unwrap_or(i32::MAX))
                / progress_percentage
                * 100.0) as i64;
            Some(goal.created_at + Duration::days(estimated_total_days))
        } else {
            None
        };

        // Determine if on track
        let days_elapsed =
            f64::from(i32::try_from((Utc::now() - goal.created_at).num_days()).unwrap_or(i32::MAX));
        let days_total = f64::from(
            i32::try_from((goal.target_date - goal.created_at).num_days()).unwrap_or(i32::MAX),
        );
        let expected_progress = if days_total > 0.0 {
            days_elapsed.mul_add(100.0 / days_total, 0.0)
        } else {
            0.0
        };
        let on_track = progress_percentage >= expected_progress - PROGRESS_TOLERANCE_PERCENTAGE;

        let progress_report = ProgressReport {
            goal_id: goal.id.clone(),
            progress_percentage,
            completion_date_estimate,
            milestones_achieved: achieved_milestones,
            insights: vec![],        // Will be filled next
            recommendations: vec![], // Will be filled next
            on_track,
        };

        let mut final_report = progress_report;
        final_report.insights = Self::generate_progress_insights(goal, &final_report);

        // Generate recommendations
        final_report.recommendations = if on_track {
            vec![
                "Maintain current training consistency".into(),
                "Continue following your current plan".into(),
            ]
        } else {
            vec![
                "Consider increasing training frequency".into(),
                "Focus on goal-specific activities".into(),
                "Review and adjust your training plan".into(),
            ]
        };

        Ok(final_report)
    }

    async fn adjust_goal(
        &self,
        goal: &Goal,
        progress: &ProgressReport,
    ) -> Result<Option<GoalAdjustment>> {
        let days_elapsed =
            f64::from(i32::try_from((Utc::now() - goal.created_at).num_days()).unwrap_or(i32::MAX));
        let days_total = f64::from(
            i32::try_from((goal.target_date - goal.created_at).num_days()).unwrap_or(i32::MAX),
        );
        let time_progress = days_elapsed / days_total;

        // Only suggest adjustments if we're past threshold of the timeline
        if time_progress < GOAL_ADJUSTMENT_THRESHOLD {
            return Ok(None);
        }

        let progress_ratio = progress.progress_percentage / time_progress.mul_add(100.0, 0.0);

        let adjustment = if progress_ratio > AHEAD_OF_SCHEDULE_THRESHOLD {
            // Significantly ahead - suggest more ambitious goal
            Some(GoalAdjustment {
                adjustment_type: AdjustmentType::IncreaseTarget,
                new_target_value: goal.target_value * TARGET_INCREASE_MULTIPLIER,
                rationale: "You're making excellent progress! Consider a more ambitious target."
                    .into(),
                confidence: Confidence::Medium,
            })
        } else if progress_ratio < BEHIND_SCHEDULE_THRESHOLD {
            // Significantly behind - suggest more realistic goal or extended timeline
            if days_total - days_elapsed > GOAL_DAYS_REMAINING_THRESHOLD {
                // Enough time left - reduce target
                Some(GoalAdjustment {
                    adjustment_type: AdjustmentType::DecreaseTarget,
                    new_target_value: goal.target_value * TARGET_DECREASE_MULTIPLIER,
                    rationale:
                        "Consider adjusting to a more achievable target based on current progress."
                            .into(),
                    confidence: Confidence::High,
                })
            } else {
                // Extend timeline
                Some(GoalAdjustment {
                    adjustment_type: AdjustmentType::ExtendDeadline,
                    new_target_value: goal.target_value,
                    rationale: "Consider extending the deadline to maintain motivation.".into(),
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

        // Create milestones using predefined percentages and names
        let percentages = MILESTONE_PERCENTAGES;
        let names = MILESTONE_NAMES;

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
    const fn new() -> Self {
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
    #[must_use]
    pub fn sport_type(&self) -> String {
        match self {
            Self::Distance { sport, .. }
            | Self::Time { sport, .. }
            | Self::Frequency { sport, .. } => sport.clone(),
            Self::Performance { .. } | Self::Custom { .. } => "Any".into(),
        }
    }
}
