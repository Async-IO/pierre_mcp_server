// ABOUTME: Goal management and feasibility configuration types
// ABOUTME: Handles training history analysis and goal feasibility assessment
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::constants::goal_management;
use serde::{Deserialize, Serialize};
use std::env;

/// Goal management and feasibility configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalManagementConfig {
    /// Minimum activities required for training history
    pub min_activities_for_history: usize,
    /// Activities per week for advanced fitness level
    pub advanced_activities_per_week: f64,
    /// Training weeks required for advanced level
    pub advanced_min_weeks: f64,
    /// Activities per week for intermediate fitness level
    pub intermediate_activities_per_week: f64,
    /// Training weeks required for intermediate level
    pub intermediate_min_weeks: f64,
    /// Default training time availability (hours/week)
    pub default_time_availability_hours: f64,
    /// Default preferred activity duration (minutes)
    pub default_preferred_duration_minutes: u32,
    /// Average days per month for calculations
    pub days_per_month_average: f64,
}

impl Default for GoalManagementConfig {
    fn default() -> Self {
        Self {
            min_activities_for_history: goal_management::MIN_ACTIVITIES_FOR_TRAINING_HISTORY,
            advanced_activities_per_week: goal_management::ADVANCED_FITNESS_ACTIVITIES_PER_WEEK,
            advanced_min_weeks: goal_management::ADVANCED_FITNESS_MIN_WEEKS,
            intermediate_activities_per_week:
                goal_management::INTERMEDIATE_FITNESS_ACTIVITIES_PER_WEEK,
            intermediate_min_weeks: goal_management::INTERMEDIATE_FITNESS_MIN_WEEKS,
            default_time_availability_hours: goal_management::DEFAULT_TIME_AVAILABILITY_HOURS,
            default_preferred_duration_minutes: goal_management::DEFAULT_PREFERRED_DURATION_MINUTES,
            days_per_month_average: goal_management::DAYS_PER_MONTH_AVERAGE,
        }
    }
}

impl GoalManagementConfig {
    /// Load goal management configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            min_activities_for_history: env::var("GOAL_MANAGEMENT_MIN_ACTIVITIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(goal_management::MIN_ACTIVITIES_FOR_TRAINING_HISTORY),
            advanced_activities_per_week: env::var("GOAL_MANAGEMENT_ADVANCED_ACTIVITIES_PER_WEEK")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(goal_management::ADVANCED_FITNESS_ACTIVITIES_PER_WEEK),
            advanced_min_weeks: env::var("GOAL_MANAGEMENT_ADVANCED_MIN_WEEKS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(goal_management::ADVANCED_FITNESS_MIN_WEEKS),
            intermediate_activities_per_week: env::var(
                "GOAL_MANAGEMENT_INTERMEDIATE_ACTIVITIES_PER_WEEK",
            )
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(goal_management::INTERMEDIATE_FITNESS_ACTIVITIES_PER_WEEK),
            intermediate_min_weeks: env::var("GOAL_MANAGEMENT_INTERMEDIATE_MIN_WEEKS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(goal_management::INTERMEDIATE_FITNESS_MIN_WEEKS),
            default_time_availability_hours: env::var(
                "GOAL_MANAGEMENT_DEFAULT_TIME_AVAILABILITY_HOURS",
            )
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(goal_management::DEFAULT_TIME_AVAILABILITY_HOURS),
            default_preferred_duration_minutes: env::var(
                "GOAL_MANAGEMENT_DEFAULT_DURATION_MINUTES",
            )
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(goal_management::DEFAULT_PREFERRED_DURATION_MINUTES),
            days_per_month_average: env::var("GOAL_MANAGEMENT_DAYS_PER_MONTH")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(goal_management::DAYS_PER_MONTH_AVERAGE),
        }
    }
}
