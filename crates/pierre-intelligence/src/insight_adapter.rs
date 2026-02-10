// ABOUTME: Adapts shared insights to a user's personal training context
// ABOUTME: Transforms friend insights into personalized recommendations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Insight Adapter
//!
//! This module adapts shared insights from friends to a user's personal training context.
//! When a user taps "Adapt to My Training" on a friend's insight, this adapter
//! transforms the generic insight into a personalized recommendation considering
//! the user's fitness level, training phase, and goals.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{AdaptedInsight, InsightType, SharedInsight, TrainingPhase};

// ============================================================================
// Constants
// ============================================================================

/// Maximum context length for adaptation
const MAX_CONTEXT_LENGTH: usize = 500;

/// Minimum fitness level for intensity suggestions
const MIN_FITNESS_FOR_INTENSITY: f64 = 40.0;

/// High fitness threshold for advanced adaptations
const HIGH_FITNESS_THRESHOLD: f64 = 70.0;

// ============================================================================
// User Training Context
// ============================================================================

/// Context about the user requesting the adaptation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserTrainingContext {
    /// User's current fitness score (0-100)
    pub fitness_score: Option<f64>,
    /// Current training phase
    pub training_phase: Option<TrainingPhase>,
    /// Weekly training volume (hours)
    pub weekly_volume_hours: Option<f64>,
    /// Primary sport type
    pub primary_sport: Option<String>,
    /// Current training goal
    pub training_goal: Option<String>,
    /// Recent activity count (last 7 days)
    pub recent_activity_count: u32,
    /// Days since last workout
    pub days_since_last_workout: u32,
}

impl UserTrainingContext {
    /// Create a new context with fitness score
    #[must_use]
    pub const fn with_fitness_score(mut self, score: f64) -> Self {
        self.fitness_score = Some(score);
        self
    }

    /// Set the training phase
    #[must_use]
    pub const fn with_training_phase(mut self, phase: TrainingPhase) -> Self {
        self.training_phase = Some(phase);
        self
    }

    /// Set weekly volume
    #[must_use]
    pub const fn with_weekly_volume(mut self, hours: f64) -> Self {
        self.weekly_volume_hours = Some(hours);
        self
    }

    /// Set primary sport (cannot be const due to String)
    #[must_use]
    pub fn with_primary_sport(mut self, sport: String) -> Self {
        self.primary_sport = Some(sport);
        self
    }

    /// Set training goal (cannot be const due to String)
    #[must_use]
    pub fn with_training_goal(mut self, goal: String) -> Self {
        self.training_goal = Some(goal);
        self
    }

    /// Get fitness level category
    #[must_use]
    pub fn fitness_level(&self) -> FitnessLevel {
        match self.fitness_score {
            Some(score) if score >= HIGH_FITNESS_THRESHOLD => FitnessLevel::Advanced,
            Some(score) if score >= MIN_FITNESS_FOR_INTENSITY => FitnessLevel::Intermediate,
            Some(_) => FitnessLevel::Beginner,
            None => FitnessLevel::Unknown,
        }
    }
}

/// Fitness level category for adaptation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitnessLevel {
    /// Unknown fitness level
    Unknown,
    /// Beginner fitness level
    Beginner,
    /// Intermediate fitness level
    Intermediate,
    /// Advanced fitness level
    Advanced,
}

// ============================================================================
// Adaptation Result
// ============================================================================

/// Result of adapting an insight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationResult {
    /// The adapted insight content
    pub adapted_content: String,
    /// Explanation of the adaptation
    pub adaptation_notes: String,
    /// Relevance to the user's context (0-100)
    pub relevance_score: u8,
    /// Suggested actions based on the insight
    pub suggested_actions: Vec<String>,
    /// Context used for adaptation (JSON serialized)
    pub context_summary: String,
}

// ============================================================================
// Insight Adapter
// ============================================================================

/// Adapts shared insights to a user's personal training context
///
/// This adapter transforms friend insights into personalized recommendations.
/// It considers the user's fitness level, training phase, and goals to provide
/// contextually relevant advice.
pub struct InsightAdapter {
    /// Whether to include detailed notes
    include_notes: bool,
}

impl Default for InsightAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl InsightAdapter {
    /// Create a new insight adapter
    #[must_use]
    pub const fn new() -> Self {
        Self {
            include_notes: true,
        }
    }

    /// Set whether to include adaptation notes
    #[must_use]
    pub const fn with_notes(mut self, include: bool) -> Self {
        self.include_notes = include;
        self
    }

    /// Adapt a shared insight to a user's context
    ///
    /// Returns an `AdaptationResult` with personalized content and suggestions.
    #[must_use]
    pub fn adapt(
        &self,
        insight: &SharedInsight,
        user_context: &UserTrainingContext,
        additional_context: Option<&str>,
    ) -> AdaptationResult {
        let adapted_content = Self::generate_adapted_content(insight, user_context);
        let suggested_actions = Self::generate_suggested_actions(insight, user_context);
        let relevance_score = Self::calculate_relevance(insight, user_context);

        let adaptation_notes = if self.include_notes {
            Self::generate_adaptation_notes(insight, user_context)
        } else {
            String::new()
        };

        let context_summary = Self::build_context_summary(user_context, additional_context);

        AdaptationResult {
            adapted_content,
            adaptation_notes,
            relevance_score,
            suggested_actions,
            context_summary,
        }
    }

    /// Create an `AdaptedInsight` model from the adaptation result
    #[must_use]
    pub fn create_adapted_insight(
        source_insight_id: Uuid,
        user_id: Uuid,
        result: &AdaptationResult,
    ) -> AdaptedInsight {
        let mut adapted =
            AdaptedInsight::new(source_insight_id, user_id, result.adapted_content.clone());
        adapted.adaptation_context = Some(result.context_summary.clone());
        adapted
    }

    /// Generate adapted content based on insight type and user context
    fn generate_adapted_content(insight: &SharedInsight, context: &UserTrainingContext) -> String {
        match insight.insight_type {
            InsightType::Achievement => Self::adapt_achievement(insight, context),
            InsightType::Milestone => Self::adapt_milestone(insight, context),
            InsightType::TrainingTip => Self::adapt_training_tip(insight, context),
            InsightType::Recovery => Self::adapt_recovery_insight(insight, context),
            InsightType::Motivation => Self::adapt_motivation(insight, context),
            InsightType::CoachingInsight => Self::adapt_coaching_insight(insight, context),
        }
    }

    /// Adapt an achievement insight
    fn adapt_achievement(insight: &SharedInsight, context: &UserTrainingContext) -> String {
        let base = &insight.content;
        let fitness_level = context.fitness_level();

        let adaptation = match fitness_level {
            FitnessLevel::Beginner => {
                "This achievement shows what's possible with consistent training. \
                Focus on building your base first - your time will come!"
            }
            FitnessLevel::Intermediate => {
                "Seeing others achieve can inspire your own goals. \
                Consider setting a similar target for your training plan."
            }
            FitnessLevel::Advanced => {
                "Nice to see others crushing it! \
                You might be ready for a similar challenge in your next training block."
            }
            FitnessLevel::Unknown => {
                "Achievements like this come from consistent effort. \
                Keep showing up and building your fitness foundation."
            }
        };

        format!("From a friend: {base}\n\nFor you: {adaptation}")
    }

    /// Adapt a milestone insight
    fn adapt_milestone(insight: &SharedInsight, context: &UserTrainingContext) -> String {
        let base = &insight.content;

        let phase_note = context.training_phase.map_or_else(
            || "Track your own progress to celebrate your milestones!".to_owned(),
            |phase| match phase {
                TrainingPhase::Base => {
                    "You're building your base - milestones will follow naturally!".to_owned()
                }
                TrainingPhase::Build => {
                    "In your build phase, you're working toward your own milestones!".to_owned()
                }
                TrainingPhase::Peak => {
                    "Peak phase focus is on performance, not volume milestones.".to_owned()
                }
                TrainingPhase::Recovery => {
                    "Recovery time - your next milestone is coming after rest!".to_owned()
                }
            },
        );

        format!("From a friend: {base}\n\nFor you: {phase_note}")
    }

    /// Adapt a training tip insight
    fn adapt_training_tip(insight: &SharedInsight, context: &UserTrainingContext) -> String {
        let base = &insight.content;
        let fitness_level = context.fitness_level();

        let intensity_note = match fitness_level {
            FitnessLevel::Beginner => "Start with lower intensity versions of these suggestions.",
            FitnessLevel::Intermediate => "This tip aligns well with your current fitness level.",
            FitnessLevel::Advanced => {
                "You might push this tip even further given your fitness level."
            }
            FitnessLevel::Unknown => "Adapt intensity to how you're feeling today.",
        };

        let volume_note = context
            .weekly_volume_hours
            .map_or_else(String::new, |hours| {
                if hours < 3.0 {
                    " Consider adding this gradually to your routine.".to_owned()
                } else if hours > 8.0 {
                    " Be mindful of total volume when adding new elements.".to_owned()
                } else {
                    String::new()
                }
            });

        format!("From a friend: {base}\n\nFor you: {intensity_note}{volume_note}")
    }

    /// Adapt a recovery insight
    fn adapt_recovery_insight(insight: &SharedInsight, context: &UserTrainingContext) -> String {
        let base = &insight.content;

        let recovery_note = if context.days_since_last_workout > 3 {
            "You've had good rest recently - might be time to get moving!"
        } else if context.days_since_last_workout == 0 {
            "If you trained today, this recovery advice is timely!"
        } else {
            "Listen to your body and balance training with recovery."
        };

        format!("From a friend: {base}\n\nFor you: {recovery_note}")
    }

    /// Adapt a motivation insight
    fn adapt_motivation(insight: &SharedInsight, context: &UserTrainingContext) -> String {
        let base = &insight.content;

        let personal_touch = context.training_goal.as_ref().map_or_else(
            || "Let this inspire your training journey!".to_owned(),
            |goal| format!("Keep this energy as you work toward {goal}!"),
        );

        format!("From a friend: {base}\n\n{personal_touch}")
    }

    /// Adapt a coaching insight (coach chat message shared by a friend)
    fn adapt_coaching_insight(insight: &SharedInsight, context: &UserTrainingContext) -> String {
        let base = &insight.content;

        let personalization = context.training_goal.as_ref().map_or_else(
            || "Consider how this advice might apply to your own training!".to_owned(),
            |goal| format!("Think about how this coaching insight relates to your goal of {goal}."),
        );

        format!("From a friend's coach: {base}\n\nFor you: {personalization}")
    }

    /// Generate suggested actions based on insight and context
    fn generate_suggested_actions(
        insight: &SharedInsight,
        context: &UserTrainingContext,
    ) -> Vec<String> {
        let mut actions = Vec::new();

        match insight.insight_type {
            InsightType::Achievement => {
                actions.push("Set a similar goal for yourself".to_owned());
                if context.training_phase == Some(TrainingPhase::Base) {
                    actions.push("Focus on consistent training first".to_owned());
                }
            }
            InsightType::Milestone => {
                actions.push("Track your own progress toward milestones".to_owned());
                actions.push("Celebrate your small wins along the way".to_owned());
            }
            InsightType::TrainingTip => {
                actions.push("Try incorporating this into your next workout".to_owned());
                if context.fitness_level() == FitnessLevel::Beginner {
                    actions.push("Start with a modified version".to_owned());
                }
            }
            InsightType::Recovery => {
                actions.push("Review your current recovery practices".to_owned());
                actions.push("Consider a recovery-focused session".to_owned());
            }
            InsightType::Motivation => {
                actions.push("Save this for motivation on tough days".to_owned());
                actions.push("Share your own wins with friends".to_owned());
            }
            InsightType::CoachingInsight => {
                actions.push("Discuss this advice with your own coach".to_owned());
                actions.push("Consider how this applies to your training".to_owned());
            }
        }

        actions
    }

    /// Calculate relevance score for this adaptation
    fn calculate_relevance(insight: &SharedInsight, context: &UserTrainingContext) -> u8 {
        let mut score = 50u8; // Base score

        // Sport type match bonus
        if let (Some(insight_sport), Some(user_sport)) =
            (&insight.sport_type, &context.primary_sport)
        {
            if insight_sport.eq_ignore_ascii_case(user_sport) {
                score = score.saturating_add(20);
            }
        }

        // Training phase match bonus
        if insight.training_phase == context.training_phase && context.training_phase.is_some() {
            score = score.saturating_add(15);
        }

        // Insight type relevance
        match insight.insight_type {
            InsightType::TrainingTip => score = score.saturating_add(10),
            InsightType::Recovery if context.days_since_last_workout == 0 => {
                score = score.saturating_add(15);
            }
            InsightType::Motivation if context.recent_activity_count < 3 => {
                score = score.saturating_add(10);
            }
            _ => {}
        }

        score.min(100)
    }

    /// Generate adaptation notes
    fn generate_adaptation_notes(insight: &SharedInsight, context: &UserTrainingContext) -> String {
        let mut notes = Vec::new();

        // Note about fitness level adaptation
        match context.fitness_level() {
            FitnessLevel::Beginner => {
                notes.push("Adapted for beginner fitness level".to_owned());
            }
            FitnessLevel::Intermediate => {
                notes.push("Adapted for intermediate fitness level".to_owned());
            }
            FitnessLevel::Advanced => {
                notes.push("Adapted for advanced fitness level".to_owned());
            }
            FitnessLevel::Unknown => {}
        }

        // Note about training phase
        if let Some(phase) = context.training_phase {
            notes.push(format!("Considered your {phase} training phase"));
        }

        // Note about sport type match
        if let (Some(insight_sport), Some(user_sport)) =
            (&insight.sport_type, &context.primary_sport)
        {
            if insight_sport.eq_ignore_ascii_case(user_sport) {
                notes.push("Sport type matches your primary activity".to_owned());
            } else {
                notes.push(format!(
                    "Originally for {insight_sport}, adapted for your {user_sport} focus"
                ));
            }
        }

        notes.join(". ")
    }

    /// Build context summary for storage
    fn build_context_summary(context: &UserTrainingContext, additional: Option<&str>) -> String {
        let summary = serde_json::json!({
            "fitness_level": format!("{:?}", context.fitness_level()),
            "training_phase": context.training_phase.map(|p| p.to_string()),
            "primary_sport": context.primary_sport,
            "weekly_volume_hours": context.weekly_volume_hours,
            "additional_context": additional.map(|s| truncate_string(s, MAX_CONTEXT_LENGTH)),
        });

        summary.to_string()
    }
}

/// Truncate a string to maximum length
#[must_use]
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_owned()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
