// ABOUTME: Goal tracking and progress monitoring engine for fitness objectives
// ABOUTME: Tracks training goals, milestones, progress metrics, and provides achievement insights
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//! Goal tracking and progress monitoring engine
#![allow(clippy::cast_precision_loss)] // Safe: fitness data conversions
#![allow(clippy::cast_possible_truncation)] // Safe: controlled ranges
#![allow(clippy::cast_sign_loss)] // Safe: positive values only
#![allow(clippy::cast_possible_wrap)] // Safe: bounded values

use std::cmp::Ordering;

use super::{
    AdvancedInsight, Confidence, Deserialize, FitnessLevel, Goal, GoalType, InsightSeverity,
    Milestone, ProgressReport, Serialize, TimeFrame, UserFitnessProfile,
};
use crate::config::intelligence::{
    DefaultStrategy, GoalEngineConfig, IntelligenceConfig, IntelligenceStrategy,
};
use crate::errors::{AppError, AppResult};
use crate::intelligence::physiological_constants::{
    consistency::{
        MILESTONE_ACHIEVEMENT_THRESHOLD, MIN_ACTIVITY_COUNT_FOR_ANALYSIS,
        PROGRESS_TOLERANCE_PERCENTAGE,
    },
    frequency_targets::{MAX_WEEKLY_FREQUENCY, TARGET_PERFORMANCE_IMPROVEMENT},
    goal_difficulty::GOAL_DISTANCE_PRECISION,
    goal_progress::{
        AHEAD_OF_SCHEDULE_THRESHOLD, BEHIND_SCHEDULE_THRESHOLD, TARGET_DECREASE_MULTIPLIER,
        TARGET_INCREASE_MULTIPLIER,
    },
    milestones::{MILESTONE_NAMES, MILESTONE_PERCENTAGES},
    time_periods::{GOAL_ADJUSTMENT_THRESHOLD, GOAL_ANALYSIS_WEEKS, GOAL_DAYS_REMAINING_THRESHOLD},
};
use crate::models::Activity;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Trait for goal management and progress tracking
#[async_trait::async_trait]
pub trait GoalEngineTrait {
    /// Suggest goals based on user profile and activity history
    async fn suggest_goals(
        &self,
        user_profile: &UserFitnessProfile,
        activities: &[Activity],
    ) -> AppResult<Vec<GoalSuggestion>>;

    /// Track progress toward a specific goal
    async fn track_progress(
        &self,
        goal: &Goal,
        activities: &[Activity],
    ) -> AppResult<ProgressReport>;

    /// Adjust goal based on current progress and performance
    async fn adjust_goal(
        &self,
        goal: &Goal,
        progress: &ProgressReport,
    ) -> AppResult<Option<GoalAdjustment>>;

    /// Create milestone structure for a goal
    async fn create_milestones(&self, goal: &Goal) -> AppResult<Vec<Milestone>>;
}

/// Advanced goal engine implementation with configurable strategy
pub struct AdvancedGoalEngine<S: IntelligenceStrategy = DefaultStrategy> {
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
            strategy: DefaultStrategy,
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
            strategy: DefaultStrategy,
            config: global_config.goal_engine.clone(),
            user_profile: Some(profile),
        }
    }

    /// Set user profile for this engine
    pub fn set_profile(&mut self, profile: UserFitnessProfile) {
        self.user_profile = Some(profile);
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

    fn filter_recent_activities(activities: &[Activity]) -> Vec<&Activity> {
        activities
            .iter()
            .filter(|a| {
                let weeks_ago = (Utc::now() - a.start_date).num_weeks();
                weeks_ago <= GOAL_ANALYSIS_WEEKS
            })
            .collect()
    }

    #[allow(clippy::cast_precision_loss)] // Safe: fitness data conversions
    fn analyze_sport_patterns(activities: &[&Activity]) -> HashMap<String, SportStats> {
        let mut sport_stats = HashMap::new();

        for activity in activities {
            let sport = format!("{:?}", activity.sport_type);
            let stats = sport_stats.entry(sport).or_insert_with(SportStats::new);

            stats.activity_count += 1;
            if let Some(distance) = activity.distance_meters {
                stats.total_distance += distance;
                stats.max_distance = stats.max_distance.max(distance);
            }

            let duration_seconds = if activity.duration_seconds > u64::from(u32::MAX) {
                f64::from(u32::MAX)
            } else {
                activity.duration_seconds as f64
            };
            stats.total_duration += duration_seconds;
            stats.max_duration = stats.max_duration.max(duration_seconds);

            if let Some(speed) = activity.average_speed {
                stats.speeds.push(speed);
            }
        }

        sport_stats
    }

    fn generate_sport_based_suggestions(
        &self,
        sport_stats: &HashMap<String, SportStats>,
        _activities: &[Activity],
    ) -> Vec<GoalSuggestion> {
        let mut suggestions = Vec::new();

        for (sport, stats) in sport_stats {
            if stats.activity_count < MIN_ACTIVITY_COUNT_FOR_ANALYSIS {
                continue;
            }

            suggestions.extend(self.create_distance_suggestions(sport, stats));
            suggestions.extend(Self::create_performance_suggestions(sport, stats));
            suggestions.extend(Self::create_frequency_suggestions(sport, stats));
        }

        suggestions
    }

    fn create_distance_suggestions(&self, sport: &str, stats: &SportStats) -> Vec<GoalSuggestion> {
        let mut suggestions = Vec::new();

        if stats.activity_count == 0 {
            return suggestions;
        }

        let avg_distance = stats.total_distance / stats.activity_count as f64;
        if avg_distance > 0.0 {
            let base_multiplier = self
                .config
                .feasibility
                .conservative_multiplier
                .max(TARGET_INCREASE_MULTIPLIER);

            let weekly_distance = stats.total_distance / GOAL_ANALYSIS_WEEKS as f64;
            let strategy_multiplier = if self
                .strategy
                .should_recommend_volume_increase(weekly_distance / 1000.0)
            {
                base_multiplier * 1.2
            } else {
                base_multiplier
            };

            let target_distance = stats.max_distance * strategy_multiplier;

            suggestions.push(GoalSuggestion {
                goal_type: GoalType::Distance {
                    sport: sport.to_owned(),
                    timeframe: TimeFrame::Month,
                },
                suggested_target: target_distance,
                rationale: format!("Based on your recent {sport} activities, you could challenge yourself with a longer distance"),
                difficulty: GoalDifficulty::Moderate,
                estimated_timeline_days: 30,
                success_probability: self.config.feasibility.min_success_probability,
            });
        }

        suggestions
    }

    fn create_performance_suggestions(sport: &str, stats: &SportStats) -> Vec<GoalSuggestion> {
        let mut suggestions = Vec::new();

        if !stats.speeds.is_empty() {
            let avg_speed = stats.speeds.iter().sum::<f64>() / stats.speeds.len() as f64;
            if avg_speed > 0.0 {
                let target_improvement = TARGET_PERFORMANCE_IMPROVEMENT;
                suggestions.push(GoalSuggestion {
                    goal_type: GoalType::Performance {
                        metric: "speed".into(),
                        improvement_percent: target_improvement,
                    },
                    suggested_target: avg_speed * (1.0 + target_improvement / 100.0),
                    rationale: format!(
                        "Improve your average {sport} pace by {target_improvement}%"
                    ),
                    difficulty: GoalDifficulty::Challenging,
                    estimated_timeline_days: 60,
                    success_probability: 0.65,
                });
            }
        }

        suggestions
    }

    fn create_frequency_suggestions(sport: &str, stats: &SportStats) -> Vec<GoalSuggestion> {
        let mut suggestions = Vec::new();

        let current_frequency = stats.activity_count as f64 / GOAL_ANALYSIS_WEEKS as f64;
        if current_frequency < MAX_WEEKLY_FREQUENCY {
            let target_frequency = ((current_frequency + 1.0).min(MAX_WEEKLY_FREQUENCY)) as u32;
            suggestions.push(GoalSuggestion {
                goal_type: GoalType::Frequency {
                    sport: sport.to_owned(),
                    sessions_per_week: target_frequency as i32,
                },
                suggested_target: f64::from(target_frequency),
                rationale: format!("Increase {sport} training consistency"),
                difficulty: GoalDifficulty::Moderate,
                estimated_timeline_days: 28,
                success_probability: 0.80,
            });
        }

        suggestions
    }

    fn generate_fitness_level_suggestions(
        user_profile: &UserFitnessProfile,
    ) -> Vec<GoalSuggestion> {
        let mut suggestions = Vec::new();

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

        suggestions
    }

    fn prioritize_suggestions(suggestions: &mut [GoalSuggestion]) {
        suggestions.sort_by(|a, b| {
            b.success_probability
                .partial_cmp(&a.success_probability)
                .unwrap_or(Ordering::Equal)
        });
    }

    fn filter_relevant_activities<'a>(
        goal: &Goal,
        activities: &'a [Activity],
    ) -> Vec<&'a Activity> {
        activities
            .iter()
            .filter(|a| {
                format!("{:?}", a.sport_type) == goal.goal_type.sport_type()
                    && a.start_date >= goal.created_at
            })
            .collect()
    }

    fn calculate_current_progress(goal: &Goal, relevant_activities: &[&Activity]) -> f64 {
        match &goal.goal_type {
            GoalType::Distance { timeframe, .. } => {
                let timeframe_start = Self::get_timeframe_start(timeframe, goal.created_at);
                relevant_activities
                    .iter()
                    .filter(|a| a.start_date >= timeframe_start)
                    .filter_map(|a| a.distance_meters)
                    .sum::<f64>()
            }
            GoalType::Time { distance, .. } => relevant_activities
                .iter()
                .filter(|a| {
                    a.distance_meters
                        .is_some_and(|d| (d - distance).abs() < distance * GOAL_DISTANCE_PRECISION)
                })
                .map(|a| a.duration_seconds as f64)
                .fold(f64::MAX, f64::min),
            GoalType::Frequency { .. } => {
                let weeks_elapsed = (Utc::now() - goal.created_at).num_weeks().max(1) as f64;
                relevant_activities.len() as f64 / weeks_elapsed
            }
            GoalType::Performance { metric, .. } => match metric.as_str() {
                "speed" => relevant_activities
                    .last()
                    .and_then(|a| a.average_speed)
                    .unwrap_or(0.0),
                _ => 0.0,
            },
            GoalType::Custom { .. } => goal.current_value,
        }
    }

    fn get_timeframe_start(timeframe: &TimeFrame, goal_created: DateTime<Utc>) -> DateTime<Utc> {
        match timeframe {
            TimeFrame::Week => Utc::now() - chrono::Duration::weeks(1),
            TimeFrame::Month => Utc::now() - chrono::Duration::days(30),
            TimeFrame::Quarter => Utc::now() - chrono::Duration::days(90),
            _ => goal_created,
        }
    }

    fn calculate_progress_percentage(goal: &Goal, current_value: f64) -> f64 {
        if goal.target_value > 0.0 {
            (current_value / goal.target_value * 100.0).min(100.0)
        } else {
            0.0
        }
    }

    fn update_milestone_achievements(
        mut milestones: Vec<Milestone>,
        current_value: f64,
    ) -> Vec<Milestone> {
        for milestone in &mut milestones {
            if current_value >= milestone.target_value {
                milestone.achieved = true;
                milestone.achieved_date = Some(Utc::now());
            }
        }
        milestones
    }

    fn estimate_completion_date(goal: &Goal, progress_percentage: f64) -> Option<DateTime<Utc>> {
        if progress_percentage > 0.0 {
            let days_elapsed = (Utc::now() - goal.created_at).num_days() as f64;
            let estimated_total_days = (days_elapsed / progress_percentage * 100.0) as i64;
            Some(goal.created_at + chrono::Duration::days(estimated_total_days))
        } else {
            None
        }
    }

    fn is_on_track(goal: &Goal, progress_percentage: f64) -> bool {
        let days_elapsed = (Utc::now() - goal.created_at).num_days() as f64;
        let days_total = (goal.target_date - goal.created_at).num_days() as f64;
        let expected_progress = if days_total > 0.0 {
            days_elapsed / days_total * 100.0
        } else {
            0.0
        };
        progress_percentage >= expected_progress - PROGRESS_TOLERANCE_PERCENTAGE
    }

    fn generate_progress_recommendations(on_track: bool) -> Vec<String> {
        if on_track {
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
        }
    }
}

#[async_trait::async_trait]
impl<S: IntelligenceStrategy> GoalEngineTrait for AdvancedGoalEngine<S> {
    async fn suggest_goals(
        &self,
        user_profile: &UserFitnessProfile,
        activities: &[Activity],
    ) -> AppResult<Vec<GoalSuggestion>> {
        let recent_activities = Self::filter_recent_activities(activities);
        let sport_stats = Self::analyze_sport_patterns(&recent_activities);
        let mut suggestions = self.generate_sport_based_suggestions(&sport_stats, activities);

        suggestions.extend(Self::generate_fitness_level_suggestions(user_profile));
        Self::prioritize_suggestions(&mut suggestions);

        Ok(suggestions.into_iter().take(5).collect())
    }

    async fn track_progress(
        &self,
        goal: &Goal,
        activities: &[Activity],
    ) -> AppResult<ProgressReport> {
        let relevant_activities = Self::filter_relevant_activities(goal, activities);
        let current_value = Self::calculate_current_progress(goal, &relevant_activities);
        let progress_percentage = Self::calculate_progress_percentage(goal, current_value);

        let milestones = self
            .create_milestones(goal)
            .await
            .map_err(|e| AppError::internal(format!("Milestone creation failed: {e}")))?;
        let achieved_milestones = Self::update_milestone_achievements(milestones, current_value);

        let completion_date_estimate = Self::estimate_completion_date(goal, progress_percentage);
        let on_track = Self::is_on_track(goal, progress_percentage);

        let mut progress_report = ProgressReport {
            goal_id: goal.id.clone(), // Safe: String ownership for progress report
            progress_percentage,
            completion_date_estimate,
            milestones_achieved: achieved_milestones,
            insights: vec![],
            recommendations: vec![],
            on_track,
        };

        progress_report.insights = Self::generate_progress_insights(goal, &progress_report);
        progress_report.recommendations = Self::generate_progress_recommendations(on_track);

        Ok(progress_report)
    }

    async fn adjust_goal(
        &self,
        goal: &Goal,
        progress: &ProgressReport,
    ) -> AppResult<Option<GoalAdjustment>> {
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

    async fn create_milestones(&self, goal: &Goal) -> AppResult<Vec<Milestone>> {
        let mut milestones = Vec::new();

        // Create milestones using predefined percentages and names
        let percentages = MILESTONE_PERCENTAGES;
        let names = MILESTONE_NAMES;

        for (i, &percentage) in percentages.iter().enumerate() {
            milestones.push(Milestone {
                name: names[i].to_owned(),
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
    /// Type of goal being suggested (distance, time, frequency, etc.)
    pub goal_type: GoalType,
    /// Target value for the goal
    pub suggested_target: f64,
    /// Explanation for why this goal is suggested
    pub rationale: String,
    /// Difficulty level of achieving this goal
    pub difficulty: GoalDifficulty,
    /// Estimated days needed to achieve this goal
    pub estimated_timeline_days: i32,
    /// Probability of successfully achieving this goal (0.0 - 1.0)
    pub success_probability: f64,
}

/// Goal difficulty levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalDifficulty {
    /// Easy goal (high success rate, minimal challenge)
    Easy,
    /// Moderate goal (balanced difficulty and achievability)
    Moderate,
    /// Challenging goal (requires significant effort)
    Challenging,
    /// Ambitious goal (stretch goal with lower success probability)
    Ambitious,
    /// Unknown difficulty level
    Unknown,
}

/// Goal adjustment suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalAdjustment {
    /// Type of adjustment being suggested
    pub adjustment_type: AdjustmentType,
    /// New target value after adjustment
    pub new_target_value: f64,
    /// Explanation for why this adjustment is recommended
    pub rationale: String,
    /// Confidence level in this adjustment recommendation
    pub confidence: Confidence,
}

/// Types of goal adjustments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdjustmentType {
    /// Increase the target value (for ahead-of-schedule progress)
    IncreaseTarget,
    /// Decrease the target value (for behind-schedule progress)
    DecreaseTarget,
    /// Extend the goal deadline to allow more time
    ExtendDeadline,
    /// Change the approach or strategy for achieving the goal
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
            | Self::Frequency { sport, .. } => sport.clone(), // Safe: String ownership for goal sport
            Self::Performance { .. } | Self::Custom { .. } => "Any".into(),
        }
    }
}
