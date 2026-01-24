// ABOUTME: Social insight generation for coach-mediated sharing features
// ABOUTME: Generates privacy-safe shareable insights from user activity data
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Social Insight Generation
//!
//! This module generates shareable, privacy-preserving insights from user activity data.
//! It transforms raw fitness data into social-friendly content that can be shared
//! with friends without exposing sensitive information like GPS coordinates,
//! exact paces, or recovery scores.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{Activity, InsightType, ShareVisibility, SharedInsight, TrainingPhase};

// ============================================================================
// Constants
// ============================================================================

/// Minimum activity count to suggest sharing milestones
const MIN_ACTIVITIES_FOR_MILESTONE: u32 = 10;

/// Milestone thresholds for activity counts
const MILESTONE_COUNTS: [u32; 7] = [10, 25, 50, 100, 250, 500, 1000];

/// Milestone thresholds for total distance (in km)
const DISTANCE_MILESTONES_KM: [f64; 8] = [
    100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0, 25000.0,
];

/// Days to look back for streak calculation
const STREAK_LOOKBACK_DAYS: i64 = 90;

/// Minimum streak length to suggest sharing
const MIN_STREAK_FOR_SHARING: u32 = 7;

// ============================================================================
// Shareable Insight Context
// ============================================================================

/// Context used when generating a shareable insight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightGenerationContext {
    /// User's recent activity count
    pub recent_activity_count: u32,
    /// User's total activity count
    pub total_activity_count: u32,
    /// User's total distance (km)
    pub total_distance_km: f64,
    /// Current training streak (days)
    pub current_streak_days: u32,
    /// Longest streak (days)
    pub longest_streak_days: u32,
    /// Primary sport type
    pub primary_sport: Option<String>,
    /// Current training phase
    pub training_phase: Option<TrainingPhase>,
    /// Recent personal records
    pub recent_prs: Vec<PersonalRecord>,
}

/// A personal record achievement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalRecord {
    /// Type of PR (e.g., "5k", "10k", "half marathon")
    pub pr_type: String,
    /// Description without exact times
    pub description: String,
    /// Date achieved
    pub achieved_at: DateTime<Utc>,
    /// Improvement percentage (optional)
    pub improvement_pct: Option<f64>,
}

/// Suggestion for a shareable insight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightSuggestion {
    /// Type of insight
    pub insight_type: InsightType,
    /// Suggested content (privacy-safe)
    pub suggested_content: String,
    /// Suggested title
    pub suggested_title: Option<String>,
    /// Relevance score (0-100)
    pub relevance_score: u8,
    /// Sport type context
    pub sport_type: Option<String>,
    /// Training phase context
    pub training_phase: Option<TrainingPhase>,
}

// ============================================================================
// Shared Insight Generator
// ============================================================================

/// Generates shareable insights from user activity data
///
/// This generator creates privacy-preserving insights suitable for social sharing.
/// It analyzes user activities and generates suggestions for achievements,
/// milestones, training tips, and motivational content.
pub struct SharedInsightGenerator {
    /// Minimum relevance score to include in suggestions
    min_relevance_score: u8,
}

impl Default for SharedInsightGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedInsightGenerator {
    /// Create a new insight generator with default settings
    #[must_use]
    pub const fn new() -> Self {
        Self {
            min_relevance_score: 50,
        }
    }

    /// Create a generator with custom minimum relevance score
    #[must_use]
    pub const fn with_min_relevance(min_score: u8) -> Self {
        Self {
            min_relevance_score: min_score,
        }
    }

    /// Generate insight suggestions based on user context
    ///
    /// Returns a list of suggested insights sorted by relevance score.
    #[must_use]
    pub fn generate_suggestions(
        &self,
        context: &InsightGenerationContext,
    ) -> Vec<InsightSuggestion> {
        let mut suggestions = Vec::new();

        // Check for activity count milestones
        Self::check_activity_milestones(context, &mut suggestions);

        // Check for distance milestones
        Self::check_distance_milestones(context, &mut suggestions);

        // Check for streak achievements
        Self::check_streak_achievements(context, &mut suggestions);

        // Check for personal records
        Self::check_personal_records(context, &mut suggestions);

        // Generate training phase insights
        Self::generate_training_phase_insights(context, &mut suggestions);

        // Filter by minimum relevance and sort by score
        suggestions.retain(|s| s.relevance_score >= self.min_relevance_score);
        suggestions.sort_by(|a, b| b.relevance_score.cmp(&a.relevance_score));

        suggestions
    }

    /// Create a `SharedInsight` from a suggestion
    #[must_use]
    pub fn create_insight(
        &self,
        user_id: Uuid,
        suggestion: &InsightSuggestion,
        visibility: ShareVisibility,
    ) -> SharedInsight {
        let mut insight = SharedInsight::new(
            user_id,
            suggestion.insight_type,
            suggestion.suggested_content.clone(),
            visibility,
        );
        insight.title.clone_from(&suggestion.suggested_title);
        insight.sport_type.clone_from(&suggestion.sport_type);
        insight.training_phase = suggestion.training_phase;
        insight
    }

    /// Check for activity count milestones
    fn check_activity_milestones(
        context: &InsightGenerationContext,
        suggestions: &mut Vec<InsightSuggestion>,
    ) {
        if context.total_activity_count < MIN_ACTIVITIES_FOR_MILESTONE {
            return;
        }

        for &milestone in &MILESTONE_COUNTS {
            if context.total_activity_count >= milestone
                && context.total_activity_count < milestone + 5
            {
                let sport_desc =
                    context
                        .primary_sport
                        .as_ref()
                        .map_or("activities", |s| match s.as_str() {
                            "run" | "running" => "runs",
                            "ride" | "cycling" => "rides",
                            "swim" | "swimming" => "swims",
                            _ => "workouts",
                        });

                suggestions.push(InsightSuggestion {
                    insight_type: InsightType::Milestone,
                    suggested_content: format!(
                        "Just completed my {milestone}th {sport_desc}! Consistency is key 💪"
                    ),
                    suggested_title: Some(format!("{milestone} {}", capitalize_first(sport_desc))),
                    relevance_score: calculate_milestone_relevance(milestone),
                    sport_type: context.primary_sport.clone(),
                    training_phase: context.training_phase,
                });
                break;
            }
        }
    }

    /// Check for distance milestones
    fn check_distance_milestones(
        context: &InsightGenerationContext,
        suggestions: &mut Vec<InsightSuggestion>,
    ) {
        for &milestone in &DISTANCE_MILESTONES_KM {
            // Within 5% of milestone
            let threshold = milestone * 0.05;
            if context.total_distance_km >= milestone
                && context.total_distance_km < milestone + threshold
            {
                let display_distance = if milestone >= 1000.0 {
                    format!("{:.0}k km", milestone / 1000.0)
                } else {
                    format!("{milestone:.0} km")
                };

                suggestions.push(InsightSuggestion {
                    insight_type: InsightType::Milestone,
                    suggested_content: format!(
                        "Crossed the {display_distance} total distance milestone! Every kilometer counts 🏃"
                    ),
                    suggested_title: Some(format!("{display_distance} Club")),
                    relevance_score: calculate_distance_milestone_relevance(milestone),
                    sport_type: context.primary_sport.clone(),
                    training_phase: context.training_phase,
                });
                break;
            }
        }
    }

    /// Check for streak achievements
    fn check_streak_achievements(
        context: &InsightGenerationContext,
        suggestions: &mut Vec<InsightSuggestion>,
    ) {
        if context.current_streak_days >= MIN_STREAK_FOR_SHARING {
            let streak_milestones = [7, 14, 21, 30, 60, 90, 180, 365];

            for &milestone in &streak_milestones {
                if context.current_streak_days >= milestone
                    && context.current_streak_days < milestone + 3
                {
                    let time_desc = if milestone >= 30 {
                        format!("{} months", milestone / 30)
                    } else {
                        format!("{milestone} days")
                    };

                    suggestions.push(InsightSuggestion {
                        insight_type: InsightType::Achievement,
                        suggested_content: format!(
                            "{time_desc} training streak! Building habits one day at a time 🔥"
                        ),
                        suggested_title: Some(format!("{time_desc} Streak")),
                        relevance_score: calculate_streak_relevance(milestone),
                        sport_type: context.primary_sport.clone(),
                        training_phase: context.training_phase,
                    });
                    break;
                }
            }
        }
    }

    /// Check for personal records
    fn check_personal_records(
        context: &InsightGenerationContext,
        suggestions: &mut Vec<InsightSuggestion>,
    ) {
        let recent_cutoff = Utc::now() - Duration::days(7);

        for pr in &context.recent_prs {
            if pr.achieved_at >= recent_cutoff {
                let improvement_text = pr
                    .improvement_pct
                    .map_or_else(String::new, |pct| format!(" ({pct:.1}% faster!)"));

                suggestions.push(InsightSuggestion {
                    insight_type: InsightType::Achievement,
                    suggested_content: format!(
                        "New {} personal best{improvement_text} 🎉",
                        pr.description
                    ),
                    suggested_title: Some(format!("{} PR", pr.pr_type.to_uppercase())),
                    relevance_score: 90, // PRs are highly relevant
                    sport_type: context.primary_sport.clone(),
                    training_phase: context.training_phase,
                });
            }
        }
    }

    /// Generate training phase insights
    fn generate_training_phase_insights(
        input: &InsightGenerationContext,
        suggestions: &mut Vec<InsightSuggestion>,
    ) {
        let Some(phase) = input.training_phase else {
            return;
        };

        let (msg_content, title) = match phase {
            TrainingPhase::Base => (
                "Building my aerobic base this month. Easy miles now pay off later! 🏃‍♂️",
                "Base Building",
            ),
            TrainingPhase::Build => (
                "In build phase - adding intensity and volume. The work is getting done! 💪",
                "Build Phase",
            ),
            TrainingPhase::Peak => (
                "Peak phase training! All systems go for race day 🎯",
                "Peak Mode",
            ),
            TrainingPhase::Recovery => (
                "Active recovery week. Rest is part of training too! 😌",
                "Recovery Mode",
            ),
        };

        suggestions.push(InsightSuggestion {
            insight_type: InsightType::TrainingTip,
            suggested_content: msg_content.to_owned(),
            suggested_title: Some(title.to_owned()),
            relevance_score: 60,
            sport_type: input.primary_sport.clone(),
            training_phase: Some(phase),
        });
    }
}

// ============================================================================
// Activity Context Builder
// ============================================================================

/// Builds insight generation context from activities
pub struct InsightContextBuilder {
    activities: Vec<Activity>,
    training_phase: Option<TrainingPhase>,
}

impl InsightContextBuilder {
    /// Create a new context builder
    #[must_use]
    pub const fn new() -> Self {
        Self {
            activities: Vec::new(),
            training_phase: None,
        }
    }

    /// Add activities for analysis
    #[must_use]
    pub fn with_activities(mut self, activities: Vec<Activity>) -> Self {
        self.activities = activities;
        self
    }

    /// Set the current training phase
    #[must_use]
    pub const fn with_training_phase(mut self, phase: TrainingPhase) -> Self {
        self.training_phase = Some(phase);
        self
    }

    /// Build the insight generation context
    #[must_use]
    pub fn build(self) -> InsightGenerationContext {
        let total_activity_count = u32::try_from(self.activities.len()).unwrap_or(u32::MAX);

        // Calculate recent activity count (last 30 days)
        let thirty_days_ago = Utc::now() - Duration::days(30);
        let recent_activity_count = u32::try_from(
            self.activities
                .iter()
                .filter(|a| a.start_date() >= thirty_days_ago)
                .count(),
        )
        .unwrap_or(u32::MAX);

        // Calculate total distance
        let total_distance_km = self
            .activities
            .iter()
            .filter_map(Activity::distance_meters)
            .sum::<f64>()
            / 1000.0;

        // Determine primary sport
        let primary_sport = self.determine_primary_sport();

        // Calculate streaks
        let (current_streak, longest_streak) = self.calculate_streaks();

        InsightGenerationContext {
            recent_activity_count,
            total_activity_count,
            total_distance_km,
            current_streak_days: current_streak,
            longest_streak_days: longest_streak,
            primary_sport,
            training_phase: self.training_phase,
            recent_prs: Vec::new(), // PRs would need separate calculation
        }
    }

    /// Determine the primary sport type from activities
    fn determine_primary_sport(&self) -> Option<String> {
        use std::collections::HashMap;
        let mut sport_counts: HashMap<String, usize> = HashMap::new();

        for activity in &self.activities {
            let sport = activity.sport_type().display_name().to_owned();
            *sport_counts.entry(sport).or_insert(0) += 1;
        }

        sport_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(sport, _)| sport)
    }

    /// Calculate current and longest streaks
    fn calculate_streaks(&self) -> (u32, u32) {
        if self.activities.is_empty() {
            return (0, 0);
        }

        let cutoff = Utc::now() - Duration::days(STREAK_LOOKBACK_DAYS);

        // Get unique activity dates within lookback period
        let mut dates: Vec<chrono::NaiveDate> = self
            .activities
            .iter()
            .filter(|a| a.start_date() >= cutoff)
            .map(|a| a.start_date().date_naive())
            .collect();

        dates.sort();
        dates.dedup();

        if dates.is_empty() {
            return (0, 0);
        }

        let today = Utc::now().date_naive();
        let mut current_streak = 0u32;
        let mut longest_streak = 0u32;
        let mut streak = 1u32;

        // Calculate longest streak
        for window in dates.windows(2) {
            let diff = window[1].signed_duration_since(window[0]);
            if diff.num_days() == 1 {
                streak += 1;
                if streak > longest_streak {
                    longest_streak = streak;
                }
            } else {
                streak = 1;
            }
        }

        // Check if streak is current (last activity was today or yesterday)
        if let Some(last_date) = dates.last() {
            let diff = today.signed_duration_since(*last_date);
            if diff.num_days() <= 1 {
                // Count backwards from today
                current_streak = 1;
                for i in (0..dates.len().saturating_sub(1)).rev() {
                    let day_diff = dates[i + 1].signed_duration_since(dates[i]);
                    if day_diff.num_days() == 1 {
                        current_streak += 1;
                    } else {
                        break;
                    }
                }
            }
        }

        (current_streak, longest_streak.max(1))
    }
}

impl Default for InsightContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate relevance score for activity count milestone
#[must_use]
pub const fn calculate_milestone_relevance(milestone: u32) -> u8 {
    match milestone {
        1000.. => 95,
        500..=999 => 90,
        250..=499 => 85,
        100..=249 => 80,
        50..=99 => 75,
        25..=49 => 70,
        _ => 65,
    }
}

/// Calculate relevance score for distance milestone
fn calculate_distance_milestone_relevance(milestone_km: f64) -> u8 {
    // Note: Can't be const fn due to f64 comparisons
    if milestone_km >= 10000.0 {
        95
    } else if milestone_km >= 5000.0 {
        90
    } else if milestone_km >= 2500.0 {
        85
    } else if milestone_km >= 1000.0 {
        80
    } else if milestone_km >= 500.0 {
        75
    } else {
        70
    }
}

/// Calculate relevance score for streak achievement
const fn calculate_streak_relevance(streak_days: u32) -> u8 {
    match streak_days {
        365.. => 95,
        180..=364 => 90,
        90..=179 => 85,
        60..=89 => 80,
        30..=59 => 75,
        _ => 70,
    }
}

/// Capitalize first letter of a string
#[must_use]
pub fn capitalize_first(s: &str) -> String {
    // Note: Can't use map_or_else because chars iterator is consumed
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    if let Some(first) = chars.next() {
        for c in first.to_uppercase() {
            result.push(c);
        }
        result.extend(chars);
    }
    result
}
