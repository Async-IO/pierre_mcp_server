// ABOUTME: Goal engine configuration for training goal management
// ABOUTME: Configures feasibility assessment, goal suggestions, and progression tracking
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Goal Engine Configuration
//!
//! Provides configuration for the goal tracking and achievement engine including
//! feasibility assessment, goal suggestion generation, and progression tracking.

use serde::{Deserialize, Serialize};

/// Goal Engine Configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GoalEngineConfig {
    /// Configuration for goal feasibility assessment
    pub feasibility: FeasibilityConfig,
    /// Configuration for goal suggestion generation
    pub suggestion: SuggestionConfig,
    /// Configuration for goal progression tracking
    pub progression: ProgressionConfig,
}

/// Configuration for goal feasibility assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeasibilityConfig {
    /// Minimum success probability for accepting a goal
    pub min_success_probability: f64,
    /// Multiplier for conservative goal calculations
    pub conservative_multiplier: f64,
    /// Multiplier for aggressive goal calculations
    pub aggressive_multiplier: f64,
    /// Threshold for injury risk warnings
    pub injury_risk_threshold: f64,
}

/// Configuration for goal suggestion generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionConfig {
    /// Maximum number of goals to suggest per goal type
    pub max_goals_per_type: usize,
    /// Distribution of easy/moderate/hard goals
    pub difficulty_distribution: DifficultyDistribution,
    /// Preferred timeframes for goal suggestions
    pub timeframe_preferences: TimeframePreferences,
}

/// Distribution of goal difficulties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyDistribution {
    /// Percentage of easy goals (0-1)
    pub easy_percentage: f64,
    /// Percentage of moderate goals (0-1)
    pub moderate_percentage: f64,
    /// Percentage of hard goals (0-1)
    pub hard_percentage: f64,
}

/// Timeframe preferences for goal suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeframePreferences {
    /// Duration for short-term goals (in weeks)
    pub short_term_weeks: u32,
    /// Duration for medium-term goals (in weeks)
    pub medium_term_weeks: u32,
    /// Duration for long-term goals (in weeks)
    pub long_term_weeks: u32,
}

/// Configuration for training progression limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressionConfig {
    /// Maximum weekly training volume increase (as percentage)
    pub weekly_increase_limit: f64,
    /// Maximum monthly training volume increase (as percentage)
    pub monthly_increase_limit: f64,
    /// Recommended frequency of deload weeks
    pub deload_frequency_weeks: u32,
}

impl Default for FeasibilityConfig {
    fn default() -> Self {
        Self {
            min_success_probability: 0.6,
            conservative_multiplier: 0.8,
            aggressive_multiplier: 1.3,
            injury_risk_threshold: 0.3,
        }
    }
}

impl Default for SuggestionConfig {
    fn default() -> Self {
        Self {
            max_goals_per_type: 3,
            difficulty_distribution: DifficultyDistribution::default(),
            timeframe_preferences: TimeframePreferences::default(),
        }
    }
}

impl Default for DifficultyDistribution {
    fn default() -> Self {
        Self {
            easy_percentage: 0.4,
            moderate_percentage: 0.4,
            hard_percentage: 0.2,
        }
    }
}

impl Default for TimeframePreferences {
    fn default() -> Self {
        Self {
            short_term_weeks: 4,
            medium_term_weeks: 12,
            long_term_weeks: 26,
        }
    }
}

impl Default for ProgressionConfig {
    fn default() -> Self {
        Self {
            weekly_increase_limit: 0.1,
            monthly_increase_limit: 0.2,
            deload_frequency_weeks: 4,
        }
    }
}
