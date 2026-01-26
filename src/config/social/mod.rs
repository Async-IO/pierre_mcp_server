// ABOUTME: Social insights configuration for coach-mediated sharing features
// ABOUTME: Configurable thresholds for milestones, streaks, and relevance scoring
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Social Insights Configuration Module
//!
//! Provides configurable thresholds and scoring for social insight generation.
//! All values can be overridden via environment variables with the `SOCIAL_INSIGHTS_` prefix.

use serde::{Deserialize, Serialize};
use std::env;
use std::str::FromStr;
use std::sync::OnceLock;
use tracing::warn;

use super::intelligence::ConfigError;

/// Global configuration singleton
static SOCIAL_INSIGHTS_CONFIG: OnceLock<SocialInsightsConfig> = OnceLock::new();

// ============================================================================
// Main Configuration
// ============================================================================

/// Social insights configuration container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialInsightsConfig {
    /// Activity count milestone thresholds
    pub milestone_thresholds: MilestoneConfig,
    /// Distance milestone thresholds (in kilometers)
    pub distance_milestones: DistanceMilestoneConfig,
    /// Training streak configuration
    pub streak_config: StreakConfig,
    /// Relevance scoring configuration
    pub relevance_scoring: RelevanceConfig,
    /// Activity fetch limits for insight generation
    pub activity_fetch_limits: ActivityFetchLimitsConfig,
    /// Minimum relevance score to include suggestions
    pub min_relevance_score: u8,
}

impl SocialInsightsConfig {
    /// Get the global configuration instance
    pub fn global() -> &'static Self {
        SOCIAL_INSIGHTS_CONFIG.get_or_init(|| {
            Self::load().unwrap_or_else(|e| {
                warn!("Failed to load social insights config: {e}, using defaults");
                Self::default()
            })
        })
    }

    /// Load configuration from environment
    ///
    /// # Errors
    ///
    /// Returns an error if environment variables contain invalid values or validation fails
    pub fn load() -> Result<Self, ConfigError> {
        let mut config = Self::default();
        config = config.apply_env_overrides()?;
        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration
    ///
    /// # Errors
    ///
    /// Returns an error if configuration values are invalid
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate milestone counts are sorted ascending
        let counts = &self.milestone_thresholds.activity_counts;
        for i in 1..counts.len() {
            if counts[i] <= counts[i - 1] {
                return Err(ConfigError::InvalidRange(
                    "milestone activity_counts must be sorted ascending",
                ));
            }
        }

        // Validate distance milestones are sorted ascending
        let distances = &self.distance_milestones.thresholds_km;
        for i in 1..distances.len() {
            if distances[i] <= distances[i - 1] {
                return Err(ConfigError::InvalidRange(
                    "distance_milestones must be sorted ascending",
                ));
            }
        }

        // Validate streak milestones are sorted ascending
        let streaks = &self.streak_config.milestone_days;
        for i in 1..streaks.len() {
            if streaks[i] <= streaks[i - 1] {
                return Err(ConfigError::InvalidRange(
                    "streak milestone_days must be sorted ascending",
                ));
            }
        }

        // Validate min < max where applicable
        if self.milestone_thresholds.min_activities_for_milestone == 0 {
            return Err(ConfigError::ValueOutOfRange(
                "min_activities_for_milestone must be > 0",
            ));
        }

        if self.streak_config.min_for_sharing == 0 {
            return Err(ConfigError::ValueOutOfRange(
                "streak min_for_sharing must be > 0",
            ));
        }

        if self.streak_config.lookback_days == 0 {
            return Err(ConfigError::ValueOutOfRange(
                "streak lookback_days must be > 0",
            ));
        }

        // Validate relevance scores are valid percentages (0-100)
        if self.min_relevance_score > 100 {
            return Err(ConfigError::ValueOutOfRange(
                "min_relevance_score must be <= 100",
            ));
        }

        // Validate activity fetch limits
        if self.activity_fetch_limits.insight_context_limit == 0 {
            return Err(ConfigError::ValueOutOfRange(
                "insight_context_limit must be > 0",
            ));
        }

        if self.activity_fetch_limits.training_context_limit == 0 {
            return Err(ConfigError::ValueOutOfRange(
                "training_context_limit must be > 0",
            ));
        }

        if self.activity_fetch_limits.max_client_limit == 0 {
            return Err(ConfigError::ValueOutOfRange("max_client_limit must be > 0"));
        }

        Ok(())
    }

    /// Helper function to parse and apply an environment variable override
    fn apply_env_var<T: FromStr>(env_var_name: &str, target: &mut T) -> Result<(), ConfigError> {
        if let Ok(val) = env::var(env_var_name) {
            *target = val
                .parse()
                .map_err(|_| ConfigError::Parse(format!("Invalid {env_var_name}")))?;
        }
        Ok(())
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(mut self) -> Result<Self, ConfigError> {
        // Milestone thresholds
        Self::apply_env_var(
            "SOCIAL_INSIGHTS_MIN_ACTIVITIES_FOR_MILESTONE",
            &mut self.milestone_thresholds.min_activities_for_milestone,
        )?;

        // Streak configuration
        Self::apply_env_var(
            "SOCIAL_INSIGHTS_STREAK_LOOKBACK_DAYS",
            &mut self.streak_config.lookback_days,
        )?;
        Self::apply_env_var(
            "SOCIAL_INSIGHTS_STREAK_MIN_FOR_SHARING",
            &mut self.streak_config.min_for_sharing,
        )?;

        // Activity fetch limits
        Self::apply_env_var(
            "SOCIAL_INSIGHTS_ACTIVITY_LIMIT",
            &mut self.activity_fetch_limits.insight_context_limit,
        )?;
        Self::apply_env_var(
            "SOCIAL_INSIGHTS_TRAINING_CONTEXT_LIMIT",
            &mut self.activity_fetch_limits.training_context_limit,
        )?;
        Self::apply_env_var(
            "SOCIAL_INSIGHTS_MAX_CLIENT_LIMIT",
            &mut self.activity_fetch_limits.max_client_limit,
        )?;

        // Min relevance score
        Self::apply_env_var(
            "SOCIAL_INSIGHTS_MIN_RELEVANCE_SCORE",
            &mut self.min_relevance_score,
        )?;

        Ok(self)
    }
}

impl Default for SocialInsightsConfig {
    fn default() -> Self {
        Self {
            milestone_thresholds: MilestoneConfig::default(),
            distance_milestones: DistanceMilestoneConfig::default(),
            streak_config: StreakConfig::default(),
            relevance_scoring: RelevanceConfig::default(),
            activity_fetch_limits: ActivityFetchLimitsConfig::default(),
            min_relevance_score: 50,
        }
    }
}

// ============================================================================
// Sub-Configurations
// ============================================================================

/// Activity count milestone configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilestoneConfig {
    /// Minimum activity count before suggesting milestones
    pub min_activities_for_milestone: u32,
    /// Activity count thresholds that trigger milestone suggestions
    pub activity_counts: Vec<u32>,
}

impl Default for MilestoneConfig {
    fn default() -> Self {
        Self {
            min_activities_for_milestone: 10,
            activity_counts: vec![10, 25, 50, 100, 250, 500, 1000],
        }
    }
}

/// Distance milestone configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistanceMilestoneConfig {
    /// Distance thresholds in kilometers
    pub thresholds_km: Vec<f64>,
    /// Percentage threshold for "near milestone" detection (default 5%)
    pub near_milestone_percent: f64,
}

impl Default for DistanceMilestoneConfig {
    fn default() -> Self {
        Self {
            thresholds_km: vec![
                100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0, 25000.0,
            ],
            near_milestone_percent: 5.0,
        }
    }
}

/// Training streak configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreakConfig {
    /// Days to look back for streak calculation
    pub lookback_days: i64,
    /// Minimum streak length to suggest sharing
    pub min_for_sharing: u32,
    /// Streak day thresholds that trigger suggestions
    pub milestone_days: Vec<u32>,
}

impl Default for StreakConfig {
    fn default() -> Self {
        Self {
            lookback_days: 90,
            min_for_sharing: 7,
            milestone_days: vec![7, 14, 21, 30, 60, 90, 180, 365],
        }
    }
}

/// Relevance scoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevanceConfig {
    /// Relevance score thresholds for activity milestones
    pub activity_milestone_scores: MilestoneRelevanceScores,
    /// Relevance score thresholds for distance milestones
    pub distance_milestone_scores: DistanceRelevanceScores,
    /// Relevance score thresholds for streak achievements
    pub streak_scores: StreakRelevanceScores,
    /// Base relevance score for personal records
    pub pr_base_score: u8,
    /// Base relevance score for training phase insights
    pub training_phase_base_score: u8,
}

impl Default for RelevanceConfig {
    fn default() -> Self {
        Self {
            activity_milestone_scores: MilestoneRelevanceScores::default(),
            distance_milestone_scores: DistanceRelevanceScores::default(),
            streak_scores: StreakRelevanceScores::default(),
            pr_base_score: 90,
            training_phase_base_score: 60,
        }
    }
}

/// Relevance scores for activity count milestones
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilestoneRelevanceScores {
    /// Score for 1000+ activities
    pub score_1000_plus: u8,
    /// Score for 500-999 activities
    pub score_500_999: u8,
    /// Score for 250-499 activities
    pub score_250_499: u8,
    /// Score for 100-249 activities
    pub score_100_249: u8,
    /// Score for 50-99 activities
    pub score_50_99: u8,
    /// Score for 25-49 activities
    pub score_25_49: u8,
    /// Default score for lower milestones
    pub score_default: u8,
}

impl Default for MilestoneRelevanceScores {
    fn default() -> Self {
        Self {
            score_1000_plus: 95,
            score_500_999: 90,
            score_250_499: 85,
            score_100_249: 80,
            score_50_99: 75,
            score_25_49: 70,
            score_default: 65,
        }
    }
}

impl MilestoneRelevanceScores {
    /// Calculate relevance score for a given milestone count
    #[must_use]
    pub const fn score_for_milestone(&self, milestone: u32) -> u8 {
        match milestone {
            1000.. => self.score_1000_plus,
            500..=999 => self.score_500_999,
            250..=499 => self.score_250_499,
            100..=249 => self.score_100_249,
            50..=99 => self.score_50_99,
            25..=49 => self.score_25_49,
            _ => self.score_default,
        }
    }
}

/// Relevance scores for distance milestones
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistanceRelevanceScores {
    /// Score for 10000+ km
    pub score_10000_plus: u8,
    /// Score for 5000-9999 km
    pub score_5000_9999: u8,
    /// Score for 2500-4999 km
    pub score_2500_4999: u8,
    /// Score for 1000-2499 km
    pub score_1000_2499: u8,
    /// Score for 500-999 km
    pub score_500_999: u8,
    /// Default score for lower distances
    pub score_default: u8,
}

impl Default for DistanceRelevanceScores {
    fn default() -> Self {
        Self {
            score_10000_plus: 95,
            score_5000_9999: 90,
            score_2500_4999: 85,
            score_1000_2499: 80,
            score_500_999: 75,
            score_default: 70,
        }
    }
}

impl DistanceRelevanceScores {
    /// Calculate relevance score for a given distance milestone
    #[must_use]
    pub fn score_for_distance(&self, distance_km: f64) -> u8 {
        if distance_km >= 10000.0 {
            self.score_10000_plus
        } else if distance_km >= 5000.0 {
            self.score_5000_9999
        } else if distance_km >= 2500.0 {
            self.score_2500_4999
        } else if distance_km >= 1000.0 {
            self.score_1000_2499
        } else if distance_km >= 500.0 {
            self.score_500_999
        } else {
            self.score_default
        }
    }
}

/// Relevance scores for streak achievements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreakRelevanceScores {
    /// Score for 365+ day streak
    pub score_365_plus: u8,
    /// Score for 180-364 day streak
    pub score_180_364: u8,
    /// Score for 90-179 day streak
    pub score_90_179: u8,
    /// Score for 60-89 day streak
    pub score_60_89: u8,
    /// Score for 30-59 day streak
    pub score_30_59: u8,
    /// Default score for shorter streaks
    pub score_default: u8,
}

impl Default for StreakRelevanceScores {
    fn default() -> Self {
        Self {
            score_365_plus: 95,
            score_180_364: 90,
            score_90_179: 85,
            score_60_89: 80,
            score_30_59: 75,
            score_default: 70,
        }
    }
}

impl StreakRelevanceScores {
    /// Calculate relevance score for a given streak length
    #[must_use]
    pub const fn score_for_streak(&self, streak_days: u32) -> u8 {
        match streak_days {
            365.. => self.score_365_plus,
            180..=364 => self.score_180_364,
            90..=179 => self.score_90_179,
            60..=89 => self.score_60_89,
            30..=59 => self.score_30_59,
            _ => self.score_default,
        }
    }
}

/// Activity fetch limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityFetchLimitsConfig {
    /// Default limit for insight context generation
    pub insight_context_limit: usize,
    /// Default limit for training context generation
    pub training_context_limit: usize,
    /// Maximum limit a client can request via query parameter
    pub max_client_limit: usize,
}

impl Default for ActivityFetchLimitsConfig {
    fn default() -> Self {
        Self {
            insight_context_limit: 100,
            training_context_limit: 30,
            max_client_limit: 500,
        }
    }
}
