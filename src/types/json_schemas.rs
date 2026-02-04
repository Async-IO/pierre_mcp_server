// ABOUTME: Type-safe JSON schema definitions for API request/response parameters
// ABOUTME: Replaces dynamic serde_json::Value usage with compile-time validated structs
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # JSON Schema Types
//!
//! This module provides strongly-typed definitions for JSON parameters that were
//! previously handled using dynamic `serde_json::Value` types.
//!
//! ## Design Principles
//!
//! 1. **Type Safety**: Use structs instead of dynamic `Value` for known schemas
//! 2. **Fail Fast**: Leverage serde's validation instead of manual `.as_*()` chains
//! 3. **Clear Errors**: Provide context about what failed to parse
//! 4. **Backwards Compatibility**: Support field aliases for API evolution
//!
//! ## When to Use These Types
//!
//! - Request parameters with known structure
//! - Configuration values that need validation
//! - API responses that clients depend on
//!
//! ## When NOT to Use
//!
//! - Plugin metadata (unknown schema)
//! - User-defined custom fields
//! - Pass-through JSON-RPC parameters

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::config::fitness::FitnessConfig;
use crate::config::runtime::ConfigValue;
use crate::errors::AppResult;
use crate::intelligence::{FitnessLevel, TimeAvailability, UserFitnessProfile, UserPreferences};

// ============================================================================
// Configuration Types
// ============================================================================

/// Configuration parameter value with type discrimination
///
/// This enum automatically tries each variant during deserialization,
/// allowing natural JSON values to be parsed correctly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigValueInput {
    /// Floating point number
    Float(f64),
    /// Integer number
    Integer(i64),
    /// Boolean value
    Boolean(bool),
    /// String value
    String(String),
}

impl ConfigValueInput {
    /// Convert to internal `ConfigValue` type
    ///
    /// This is a helper for migrating from the old `HashMap`<String, Value> pattern
    #[must_use]
    pub fn to_config_value(self) -> ConfigValue {
        match self {
            Self::Float(v) => ConfigValue::Float(v),
            Self::Integer(v) => ConfigValue::Integer(v),
            Self::Boolean(v) => ConfigValue::Boolean(v),
            Self::String(v) => ConfigValue::String(v),
        }
    }
}

/// Request to update user configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateConfigurationRequest {
    /// Optional profile name to apply
    pub profile: Option<String>,

    /// Parameter overrides as key-value pairs
    #[serde(default)]
    pub parameter_overrides: HashMap<String, ConfigValueInput>,
}

// ============================================================================
// A2A Protocol Types
// ============================================================================

/// Parameters for creating an A2A task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ATaskCreateParams {
    /// Client identifier (optional, defaults to "unknown")
    #[serde(default = "default_client_id")]
    pub client_id: String,

    /// Task type (accepts both `task_type` and `type` JSON keys)
    #[serde(alias = "type")]
    pub task_type: String,

    /// Optional metadata about the task
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

fn default_client_id() -> String {
    "unknown".to_owned()
}

/// Parameters for retrieving an A2A task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ATaskGetParams {
    /// Task ID to retrieve
    pub task_id: String,
}

/// Parameters for listing A2A tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ATaskListParams {
    /// Optional client ID filter
    #[serde(default)]
    pub client_id: Option<String>,

    /// Optional task status filter
    #[serde(default)]
    pub status: Option<String>,

    /// Maximum number of results
    #[serde(default = "default_limit")]
    pub limit: u32,

    /// Offset for pagination
    #[serde(default)]
    pub offset: Option<u32>,
}

const fn default_limit() -> u32 {
    20
}

// ============================================================================
// MCP Tool Parameter Types
// ============================================================================

/// Parameters for OAuth notification check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckOAuthNotificationsParams {
    /// Optional notification ID to check
    #[serde(default)]
    pub notification_id: Option<String>,
}

/// Parameters for provider connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectProviderParams {
    /// Provider name (e.g., "strava", "fitbit")
    pub provider: String,

    /// Optional Strava client ID
    #[serde(default)]
    pub strava_client_id: Option<String>,

    /// Optional Strava client secret
    #[serde(default)]
    pub strava_client_secret: Option<String>,

    /// Optional Fitbit client ID
    #[serde(default)]
    pub fitbit_client_id: Option<String>,

    /// Optional Fitbit client secret
    #[serde(default)]
    pub fitbit_client_secret: Option<String>,
}

/// Parameters for disconnecting a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisconnectProviderParams {
    /// Provider name to disconnect
    pub provider: String,
}

/// Parameters for getting connection status
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GetConnectionStatusParams {
    /// Optional provider name (if not specified, returns all)
    #[serde(default)]
    pub provider: Option<String>,

    /// Optional Strava client ID (for credentials)
    #[serde(default)]
    pub strava_client_id: Option<String>,

    /// Optional Strava client secret (for credentials)
    #[serde(default)]
    pub strava_client_secret: Option<String>,

    /// Optional Fitbit client ID (for credentials)
    #[serde(default)]
    pub fitbit_client_id: Option<String>,

    /// Optional Fitbit client secret (for credentials)
    #[serde(default)]
    pub fitbit_client_secret: Option<String>,
}

/// Parameters for marking notifications as read
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkNotificationsReadParams {
    /// Notification ID to mark as read
    pub notification_id: String,
}

/// Parameters for getting notifications
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GetNotificationsParams {
    /// Whether to include read notifications
    #[serde(default)]
    pub include_read: bool,

    /// Optional provider filter
    #[serde(default)]
    pub provider: Option<String>,
}

/// Parameters for announcing OAuth success
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnounceOAuthSuccessParams {
    /// Provider name (e.g., "strava", "fitbit")
    pub provider: String,

    /// Success message
    #[serde(default = "default_oauth_message")]
    pub message: String,

    /// Notification ID
    #[serde(default = "default_notification_id")]
    pub notification_id: String,
}

fn default_oauth_message() -> String {
    "OAuth completed successfully".to_owned()
}

fn default_notification_id() -> String {
    "unknown".to_owned()
}

/// Parameters for getting fitness configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GetFitnessConfigParams {
    /// Configuration name
    #[serde(default = "default_config_name")]
    pub configuration_name: String,
}

/// Parameters for setting fitness configuration
///
/// Uses typed `FitnessConfig` instead of `serde_json::Value` for compile-time
/// validation of configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetFitnessConfigParams {
    /// Configuration name
    #[serde(default = "default_config_name")]
    pub configuration_name: String,

    /// Configuration data - typed for validation
    pub configuration: FitnessConfig,
}

/// Parameters for deleting fitness configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteFitnessConfigParams {
    /// Configuration name to delete
    pub configuration_name: String,
}

fn default_config_name() -> String {
    "default".to_owned()
}

// ============================================================================
// Database User Profile Types
// ============================================================================

/// Typed wrapper for user fitness profile storage
///
/// This replaces the generic `serde_json::Value` in database operations
/// with a properly typed struct for better validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFitnessProfileData {
    /// User ID (UUID as string)
    pub user_id: String,

    /// User's age in years
    #[serde(default)]
    pub age: Option<i32>,

    /// User's gender
    #[serde(default)]
    pub gender: Option<String>,

    /// User's weight in kg
    #[serde(default)]
    pub weight: Option<f64>,

    /// User's height in cm
    #[serde(default)]
    pub height: Option<f64>,

    /// Fitness level
    #[serde(default)]
    pub fitness_level: Option<String>,

    /// Primary sports activities
    #[serde(default)]
    pub primary_sports: Vec<String>,

    /// Training history in months
    #[serde(default)]
    pub training_history_months: i32,
}

impl UserFitnessProfileData {
    /// Convert to internal `UserFitnessProfile` type
    ///
    /// # Errors
    /// Returns error if `fitness_level` string cannot be parsed
    pub fn to_user_fitness_profile(self) -> AppResult<UserFitnessProfile> {
        let fitness_level = self
            .fitness_level
            .as_deref()
            .and_then(|s| match s.to_lowercase().as_str() {
                "beginner" => Some(FitnessLevel::Beginner),
                "intermediate" => Some(FitnessLevel::Intermediate),
                "advanced" => Some(FitnessLevel::Advanced),
                "elite" => Some(FitnessLevel::Elite),
                _ => None,
            })
            .unwrap_or(FitnessLevel::Beginner);

        Ok(UserFitnessProfile {
            user_id: self.user_id,
            age: self.age,
            gender: self.gender,
            weight: self.weight,
            height: self.height,
            fitness_level,
            primary_sports: self.primary_sports,
            training_history_months: self.training_history_months,
            preferences: UserPreferences {
                preferred_units: "metric".to_owned(),
                training_focus: vec![],
                injury_history: vec![],
                time_availability: TimeAvailability {
                    hours_per_week: 5.0,
                    preferred_days: vec![],
                    preferred_duration_minutes: None,
                },
            },
        })
    }

    /// Create from internal `UserFitnessProfile` type
    #[must_use]
    pub fn from_user_fitness_profile(profile: &UserFitnessProfile) -> Self {
        Self {
            user_id: profile.user_id.clone(),
            age: profile.age,
            gender: profile.gender.clone(),
            weight: profile.weight,
            height: profile.height,
            fitness_level: Some(format!("{:?}", profile.fitness_level)),
            primary_sports: profile.primary_sports.clone(),
            training_history_months: profile.training_history_months,
        }
    }
}

// ============================================================================
// Intelligence Handler Parameters
// ============================================================================

/// Parameters for `get_activity_intelligence` tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetActivityIntelligenceParams {
    /// Strava activity ID to analyze
    pub activity_id: String,
}

/// Parameters for `analyze_performance_trends` tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzePerformanceTrendsParams {
    /// Metric to analyze (e.g., pace, `heart_rate`, power)
    #[serde(default = "default_metric")]
    pub metric: String,

    /// Timeframe for analysis (e.g., week, month, year)
    #[serde(default = "default_month_timeframe")]
    pub timeframe: String,
}

fn default_metric() -> String {
    "pace".to_owned()
}

fn default_month_timeframe() -> String {
    "month".to_owned()
}

/// Parameters for `compare_activities` tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareActivitiesParams {
    /// Primary activity ID to compare
    pub activity_id: String,

    /// Type of comparison (`similar_activities`, `personal_best`, etc.)
    #[serde(default = "default_comparison_type")]
    pub comparison_type: String,

    /// Specific activity ID to compare against (optional)
    #[serde(default)]
    pub compare_activity_id: Option<String>,
}

fn default_comparison_type() -> String {
    "similar_activities".to_owned()
}

/// Parameters for `detect_patterns` tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectPatternsParams {
    /// Pattern type to detect (consistency, progression, plateaus, etc.)
    pub pattern_type: String,
}

/// Parameters for `generate_recommendations` tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRecommendationsParams {
    /// Type of recommendations (training, recovery, nutrition, all)
    #[serde(default = "default_recommendation_type")]
    pub recommendation_type: String,
}

fn default_recommendation_type() -> String {
    "all".to_owned()
}

/// Parameters for `calculate_fitness_score` tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculateFitnessScoreParams {
    /// Timeframe for fitness calculation (week, month, quarter)
    #[serde(default = "default_month_timeframe")]
    pub timeframe: String,
}

/// Parameters for `predict_performance` tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictPerformanceParams {
    /// Target sport for prediction (Run, Ride, Swim, etc.)
    #[serde(default = "default_target_sport")]
    pub target_sport: String,
}

fn default_target_sport() -> String {
    "Run".to_owned()
}

/// Parameters for `analyze_training_load` tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeTrainingLoadParams {
    /// Timeframe for training load analysis (day, week, month)
    #[serde(default = "default_week_timeframe")]
    pub timeframe: String,
}

fn default_week_timeframe() -> String {
    "week".to_owned()
}

// ============================================================================
// Goals Handler Parameters
// ============================================================================

/// Parameters for `analyze_goal_feasibility` tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeGoalFeasibilityParams {
    /// Type of goal (distance, duration, frequency)
    pub goal_type: String,

    /// Target value for the goal
    pub target_value: f64,

    /// Timeframe in days for goal completion
    #[serde(default)]
    pub timeframe_days: Option<u32>,
}

/// Parameters for `set_goal` tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetGoalParams {
    /// Type of goal (distance, duration, frequency)
    pub goal_type: String,

    /// Target value to achieve
    pub target_value: f64,

    /// Timeframe description (week, month, quarter, year)
    pub timeframe: String,

    /// Human-readable goal title
    #[serde(default = "default_goal_title")]
    pub title: String,
}

fn default_goal_title() -> String {
    "Fitness Goal".to_owned()
}

/// Parameters for `track_progress` tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackProgressParams {
    /// ID of the goal to track
    pub goal_id: String,
}

// ============================================================================
// MCP Protocol Handler Types
// ============================================================================

/// Parameters for `tools/call` requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallParams {
    /// Name of the tool to execute
    pub name: String,
    /// Tool-specific arguments
    pub arguments: serde_json::Value,
}

/// Parameters for `resources/read` requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReadParams {
    /// URI of the resource to read
    pub uri: String,
}

/// Parameters for provider-based tool calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderParams {
    /// Provider name (e.g., "strava", "fitbit")
    /// Optional because not all tools require a provider parameter
    #[serde(default)]
    pub provider: Option<String>,
}

// ============================================================================
// MCP Response Types
// ============================================================================

/// Response for disconnecting a fitness provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisconnectProviderResponse {
    /// Whether the disconnect was successful
    pub success: bool,
    /// Human-readable message
    pub message: String,
    /// Provider that was disconnected
    pub provider: String,
}

/// Details of a created goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalCreatedDetails {
    /// ID of the created goal
    pub goal_id: String,
    /// Goal status
    pub status: String,
    /// Human-readable message
    pub message: String,
}

/// Response for goal creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalCreatedResponse {
    /// Details about the created goal
    pub goal_created: GoalCreatedDetails,
}

/// Progress report details for a goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressReportDetails {
    /// Goal identifier
    pub goal_id: String,
    /// Goal data as JSON value
    pub goal: serde_json::Value,
    /// Progress percentage (0-100)
    pub progress_percentage: f64,
    /// Whether the goal is on track
    pub on_track: bool,
    /// Insights about the progress
    pub insights: Vec<String>,
}

/// Response for progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressReportResponse {
    /// Progress report details
    pub progress_report: ProgressReportDetails,
}

/// Individual notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationItem {
    /// Notification identifier
    pub id: String,
    /// Provider name
    pub provider: String,
    /// Whether the notification indicates success
    pub success: bool,
    /// Notification message
    pub message: String,
    /// When the notification was created
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Connection help information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionHelp {
    /// Help message
    pub message: String,
    /// List of supported providers
    pub supported_providers: Vec<String>,
    /// Additional note
    pub note: String,
}
