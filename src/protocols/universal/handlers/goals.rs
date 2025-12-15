// ABOUTME: Goal management handlers for fitness objectives
// ABOUTME: Handle goal setting, tracking, and feasibility analysis
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::config::environment::default_provider;
use crate::constants::defaults::DEFAULT_GOAL_TIMEFRAME_DAYS;
use crate::constants::goal_management::MIN_ACTIVITIES_FOR_TRAINING_HISTORY;
use crate::constants::limits::{
    ACTIVITY_CAPACITY_HINT, MAX_TIMEFRAME_DAYS, METERS_PER_KILOMETER, PERCENTAGE_MULTIPLIER,
};
use crate::constants::time_constants::{
    DAYS_PER_MONTH, DAYS_PER_QUARTER, DAYS_PER_WEEK, DAYS_PER_YEAR, SECONDS_PER_HOUR_F64,
};
use crate::database_plugins::factory::Database;
use crate::database_plugins::DatabaseProvider;
use crate::errors::JsonResultExt;
use crate::intelligence::goal_engine::{AdvancedGoalEngine, GoalEngineTrait, GoalSuggestion};
use crate::intelligence::physiological_constants::goal_feasibility::{
    ADEQUATE_FREQUENCY_DATA_THRESHOLD, ASSUMED_TRAINING_HISTORY_WEEKS, DAYS_PER_MONTH_APPROX,
    DEFAULT_TIMEFRAME_DAYS as GOAL_DEFAULT_TIMEFRAME_DAYS, EXCELLENT_CONFIDENCE_THRESHOLD,
    EXCELLENT_DATA_QUALITY_THRESHOLD, EXCESSIVE_IMPROVEMENT_PENALTY_FACTOR,
    GOAL_SUGGESTION_ACTIVITY_LIMIT, GOOD_CONFIDENCE_LEVEL, GOOD_CONFIDENCE_THRESHOLD,
    GOOD_DATA_QUALITY_THRESHOLD, HIGH_CONFIDENCE_LEVEL, LIMITED_CONFIDENCE_LEVEL, MAX_PERCENTAGE,
    MEDIUM_CONFIDENCE_LEVEL, MINIMUM_CONFIDENCE_LEVEL, MIN_ACTIVITIES_FOR_EXCELLENT_CONFIDENCE,
    MIN_ACTIVITIES_FOR_GOOD_CONFIDENCE, MODERATE_FEASIBILITY_THRESHOLD,
    PROGRESS_TRACKING_ACTIVITY_LIMIT, SAFE_MONTHLY_IMPROVEMENT_RATE_PERCENT,
    SAFE_RANGE_PENALTY_FACTOR, SIMPLE_PROGRESS_THRESHOLD, UNSAFE_IMPROVEMENT_PENALTY_BASE,
    VERY_LOW_CONFIDENCE_LEVEL, VOLUME_DOUBLING_THRESHOLD,
};
use crate::intelligence::{FitnessLevel, TimeAvailability, UserFitnessProfile, UserPreferences};
use crate::models::Activity;
use crate::protocols::universal::{UniversalRequest, UniversalResponse, UniversalToolExecutor};
use crate::protocols::ProtocolError;
use crate::types::json_schemas::{AnalyzeGoalFeasibilityParams, SetGoalParams};
use crate::utils::uuid::parse_user_id_for_protocol;
use chrono::{DateTime, FixedOffset, Utc};
use num_traits::ToPrimitive;
use serde_json::{from_value, json, Value as JsonValue};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use tracing::{debug, warn};
use uuid::Uuid;

/// Safe conversion from usize to f64 for activity counts
/// Uses num-traits to avoid clippy cast warnings
#[inline]
fn safe_usize_to_f64(len: usize) -> f64 {
    len.to_f64().unwrap_or_else(|| f64::from(u32::MAX))
}

/// Safe conversion from i64 to f64 for time durations
/// Uses num-traits to avoid clippy cast warnings
#[inline]
fn safe_i64_to_f64(val: i64) -> f64 {
    val.to_f64().unwrap_or_else(|| f64::from(i32::MAX))
}

/// Safe conversion from f64 to u32 with clamping
/// Clamps values to u32 range to prevent overflow
#[inline]
fn safe_f64_to_u32(val: f64) -> u32 {
    if val >= f64::from(u32::MAX) {
        u32::MAX
    } else if val <= 0.0 {
        0
    } else {
        val.to_u32().unwrap_or(u32::MAX)
    }
}

/// Extract and validate goal feasibility parameters from request
fn extract_feasibility_params(
    request: &UniversalRequest,
) -> Result<(String, f64, u32), ProtocolError> {
    let params: AnalyzeGoalFeasibilityParams = from_value(request.parameters.clone())
        .json_context("analyze_goal_feasibility parameters")
        .map_err(|e| ProtocolError::InvalidParameters(e.to_string()))?;

    let timeframe_days = params.timeframe_days.unwrap_or(GOAL_DEFAULT_TIMEFRAME_DAYS);

    let effective_timeframe = if timeframe_days > MAX_TIMEFRAME_DAYS {
        warn!(
            "Timeframe {timeframe_days} days is unusually long, capping at {}",
            MAX_TIMEFRAME_DAYS
        );
        MAX_TIMEFRAME_DAYS
    } else {
        timeframe_days
    };

    Ok((params.goal_type, params.target_value, effective_timeframe))
}

/// Calculate feasibility score based on current level vs target
fn calculate_feasibility_score(
    current_level: f64,
    target_value: f64,
    effective_timeframe: u32,
) -> (f64, f64, f64) {
    let improvement_required = if current_level > 0.0 {
        ((target_value - current_level) / current_level) * MAX_PERCENTAGE
    } else {
        MAX_PERCENTAGE
    };

    let months = f64::from(effective_timeframe) / DAYS_PER_MONTH_APPROX;
    let safe_improvement_capacity = SAFE_MONTHLY_IMPROVEMENT_RATE_PERCENT * months;

    let feasibility_score = if improvement_required <= 0.0 {
        MAX_PERCENTAGE
    } else if improvement_required <= safe_improvement_capacity {
        (improvement_required / safe_improvement_capacity)
            .mul_add(-SAFE_RANGE_PENALTY_FACTOR, MAX_PERCENTAGE)
    } else {
        let excess_improvement = improvement_required - safe_improvement_capacity;
        let penalty =
            (excess_improvement / safe_improvement_capacity) * EXCESSIVE_IMPROVEMENT_PENALTY_FACTOR;
        (UNSAFE_IMPROVEMENT_PENALTY_BASE - penalty).max(0.0)
    };

    (
        feasibility_score,
        improvement_required,
        safe_improvement_capacity,
    )
}

/// Generate recommendations based on feasibility analysis
fn generate_feasibility_recommendations(
    mut recommendations: Vec<String>,
    feasible: bool,
    improvement_required: f64,
    safe_improvement_capacity: f64,
    current_level: f64,
    goal_type: &str,
    activities_count: usize,
) -> Vec<String> {
    if !feasible && improvement_required > safe_improvement_capacity {
        let suggested_days_f64 = (improvement_required / SAFE_MONTHLY_IMPROVEMENT_RATE_PERCENT)
            .mul_add(f64::from(DAYS_PER_MONTH), 0.0)
            .ceil();
        let suggested_timeframe = safe_f64_to_u32(suggested_days_f64);
        recommendations.push(format!(
            "Consider extending timeframe to {suggested_timeframe} days for safer progression"
        ));

        let safer_target = current_level * (1.0 + (safe_improvement_capacity / 100.0));
        recommendations.push(format!(
            "Or reduce target to {safer_target:.1} {} for current timeframe",
            match goal_type {
                "distance" => "km",
                "duration" => "hours",
                "frequency" => "activities",
                _ => "units",
            }
        ));
    }

    if activities_count < GOOD_DATA_QUALITY_THRESHOLD {
        recommendations
            .push("Build consistent training history for better goal planning".to_owned());
    }

    recommendations
}

/// Parameters for building feasibility response
struct FeasibilityResponseParams<'a> {
    feasibility_score: f64,
    feasible: bool,
    confidence_level: f64,
    risk_factors: Vec<String>,
    recommendations: Vec<String>,
    target_value: f64,
    current_level: f64,
    safe_improvement_capacity: f64,
    effective_timeframe: u32,
    improvement_required: f64,
    activities_len: usize,
    goal_type: &'a str,
}

/// Build feasibility analysis response
fn build_feasibility_response(params: &FeasibilityResponseParams) -> UniversalResponse {
    let months = f64::from(params.effective_timeframe) / DAYS_PER_MONTH_APPROX;
    UniversalResponse {
        success: true,
        result: Some(json!({
            "feasible": params.feasible,
            "feasibility_score": params.feasibility_score.min(100.0),
            "confidence_level": params.confidence_level,
            "risk_factors": params.risk_factors,
            "success_probability": (params.feasibility_score / 100.0).min(1.0),
            "recommendations": params.recommendations,
            "adjusted_target": if params.feasible { params.target_value } else { params.current_level * (1.0 + (params.safe_improvement_capacity / 100.0)) },
            "adjusted_timeframe": if params.feasible {
                params.effective_timeframe
            } else {
                let safe_days_f64 = (params.improvement_required / SAFE_MONTHLY_IMPROVEMENT_RATE_PERCENT).mul_add(
                    f64::from(DAYS_PER_MONTH),
                    0.0
                ).ceil();
                safe_f64_to_u32(safe_days_f64)
            },
            "analysis": {
                "current_level": params.current_level,
                "target_value": params.target_value,
                "improvement_required_percent": params.improvement_required,
                "safe_improvement_capacity_percent": params.safe_improvement_capacity,
                "timeframe_months": months
            },
            "historical_context": {
                "activities_analyzed": params.activities_len,
                "goal_type": params.goal_type,
                "data_quality": if params.activities_len >= EXCELLENT_DATA_QUALITY_THRESHOLD { "excellent" } else if params.activities_len >= GOOD_DATA_QUALITY_THRESHOLD { "good" } else { "limited" }
            }
        })),
        error: None,
        metadata: Some({
            let mut map = HashMap::new();
            map.insert(
                "analysis_method".to_owned(),
                JsonValue::String("historical_performance_based".to_owned()),
            );
            map.insert(
                "safe_improvement_rate".to_owned(),
                JsonValue::String("10_percent_per_month".to_owned()),
            );
            map
        }),
    }
}

/// Extract goal parameters from request
///
/// Parses and validates required goal parameters from the request.
///
/// # Arguments
/// * `request` - Universal request containing goal parameters
///
/// # Returns
/// `SetGoalParams` struct with validated parameters
fn extract_goal_params(request: &UniversalRequest) -> Result<SetGoalParams, ProtocolError> {
    from_value(request.parameters.clone())
        .json_context("set_goal parameters")
        .map_err(|e| ProtocolError::InvalidParameters(e.to_string()))
}

/// Build goal creation response
///
/// Constructs success response for goal creation.
///
/// # Arguments
/// * `goal_id` - Created goal's ID
/// * `goal_type` - Type of goal
/// * `target_value` - Target value
/// * `timeframe` - Goal timeframe
/// * `title` - Goal title
/// * `created_at` - Creation timestamp
///
/// # Returns
/// Universal response with goal details
fn build_goal_creation_response(
    goal_id: &str,
    goal_type: &str,
    target_value: f64,
    timeframe: &str,
    title: &str,
    created_at: DateTime<Utc>,
) -> UniversalResponse {
    UniversalResponse {
        success: true,
        result: Some(json!({
            "goal_id": goal_id,
            "goal_type": goal_type,
            "target_value": target_value,
            "timeframe": timeframe,
            "title": title,
            "created_at": created_at.to_rfc3339(),
            "status": "created"
        })),
        error: None,
        metadata: None,
    }
}

/// Handle `set_goal` tool - set a new fitness goal
#[must_use]
pub fn handle_set_goal(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use parse_user_id_for_protocol;

        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "handle_set_goal cancelled by user".to_owned(),
                ));
            }
        }

        let params = extract_goal_params(&request)?;
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Save goal to database
        let created_at = Utc::now();
        let goal_data = json!({
            "goal_type": params.goal_type,
            "target_value": params.target_value,
            "timeframe": params.timeframe,
            "title": params.title,
            "created_at": created_at.to_rfc3339()
        });

        let goal_id = (*executor.resources.database)
            .create_goal(user_uuid, goal_data)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Database error: {e}")))?;

        Ok(build_goal_creation_response(
            &goal_id,
            &params.goal_type,
            params.target_value,
            &params.timeframe,
            &params.title,
            created_at,
        ))
    })
}

/// Load user fitness profile from database
///
/// Attempts to load profile from database, falling back to calculated profile from activities.
///
/// # Arguments
/// * `database` - Database provider
/// * `user_uuid` - User's UUID
/// * `user_id` - User ID string for logging
/// * `activities` - Activities for fallback profile calculation
///
/// # Returns
/// `UserFitnessProfile` (either from DB or calculated fallback)
async fn load_user_profile(
    database: &Database,
    user_uuid: Uuid,
    user_id: &str,
    activities: &[Activity],
) -> UserFitnessProfile {
    match database.get_user_profile(user_uuid).await {
        Ok(Some(profile_json)) => from_value(profile_json).unwrap_or_else(|e| {
            warn!(
                user_id = %user_id,
                error = %e,
                "Failed to deserialize user fitness profile, using fallback profile"
            );
            create_fallback_profile(user_id.to_owned(), activities)
        }),
        Ok(None) | Err(_) => create_fallback_profile(user_id.to_owned(), activities),
    }
}

/// Format goal suggestions for response
///
/// Converts goal suggestions into JSON format for API response.
///
/// # Arguments
/// * `suggestions` - Vector of goal suggestions from engine
///
/// # Returns
/// JSON array of formatted goal suggestions
fn format_goal_suggestions(suggestions: Vec<GoalSuggestion>) -> Vec<JsonValue> {
    suggestions
        .into_iter()
        .map(|g| {
            json!({
                "goal_type": format!("{:?}", g.goal_type),
                "target_value": g.suggested_target,
                "difficulty": format!("{:?}", g.difficulty),
                "rationale": g.rationale,
                "estimated_timeline_days": g.estimated_timeline_days,
                "success_probability": g.success_probability
            })
        })
        .collect()
}

/// Create metadata for goal suggestion response
///
/// Builds metadata hashmap with analysis engine information.
///
/// # Returns
/// Metadata hashmap for response
fn create_suggestion_metadata() -> HashMap<String, JsonValue> {
    let mut map = HashMap::with_capacity(2);
    map.insert(
        "analysis_engine".into(),
        JsonValue::String("smart_goal_engine".into()),
    );
    map.insert(
        "suggestion_algorithm".into(),
        JsonValue::String("adaptive_goal_generation".into()),
    );
    map
}

/// Fetch activities for goal suggestions
///
/// Retrieves limited set of recent activities for AI goal suggestion analysis.
///
/// # Arguments
/// * `executor` - Universal tool executor with auth and provider access
/// * `user_uuid` - User's UUID for authentication
/// * `tenant_id` - Optional tenant ID for multi-tenant environments
///
/// # Returns
/// Vector of recent activities (empty if fetch fails)
async fn fetch_suggestion_activities(
    executor: &UniversalToolExecutor,
    provider_name: &str,
    user_uuid: Uuid,
    tenant_id: Option<&str>,
) -> Vec<Activity> {
    let mut activities = Vec::new();
    if let Ok(provider) = executor
        .auth_service
        .create_authenticated_provider(provider_name, user_uuid, tenant_id)
        .await
    {
        if let Ok(provider_activities) = provider
            .get_activities(Some(GOAL_SUGGESTION_ACTIVITY_LIMIT), None)
            .await
        {
            activities = provider_activities;
        }
    }
    activities
}

/// Handle `suggest_goals` tool - get AI-suggested fitness goals
#[must_use]
pub fn handle_suggest_goals(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use parse_user_id_for_protocol;

        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "handle_suggest_goals cancelled by user".to_owned(),
                ));
            }
        }

        let provider_name = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .map_or_else(default_provider, String::from);
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Fetch activities and load user profile
        let activities = fetch_suggestion_activities(
            executor,
            &provider_name,
            user_uuid,
            request.tenant_id.as_deref(),
        )
        .await;

        // Generate goal suggestions
        let goal_engine = AdvancedGoalEngine::new();
        let user_profile = load_user_profile(
            &executor.resources.database,
            user_uuid,
            &request.user_id,
            &activities,
        )
        .await;

        match goal_engine.suggest_goals(&user_profile, &activities).await {
            Ok(suggestions) => Ok(UniversalResponse {
                success: true,
                result: Some(json!({
                    "suggested_goals": format_goal_suggestions(suggestions),
                    "activities_analyzed": activities.len()
                })),
                error: None,
                metadata: Some(create_suggestion_metadata()),
            }),
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Failed to suggest goals: {e}")),
                metadata: None,
            }),
        }
    })
}

/// Fetch activities for goal feasibility analysis
///
/// Retrieves user activities from Strava for analyzing goal feasibility.
/// Returns empty vector if authentication fails or activities cannot be fetched.
///
/// # Arguments
/// * `executor` - Universal tool executor with auth and provider access
/// * `user_uuid` - User's UUID for authentication
/// * `tenant_id` - Optional tenant ID for multi-tenant environments
///
/// # Returns
/// Vector of activities for analysis (empty if fetch fails)
async fn fetch_feasibility_activities(
    executor: &UniversalToolExecutor,
    provider_name: &str,
    user_uuid: Uuid,
    tenant_id: Option<&str>,
) -> Vec<Activity> {
    let mut activities: Vec<Activity> = Vec::with_capacity(ACTIVITY_CAPACITY_HINT);

    if let Ok(provider) = executor
        .auth_service
        .create_authenticated_provider(provider_name, user_uuid, tenant_id)
        .await
    {
        if let Ok(provider_activities) = provider
            .get_activities(Some(PROGRESS_TRACKING_ACTIVITY_LIMIT), None)
            .await
        {
            activities = provider_activities;
        }
    }

    activities
}

/// Analyze goal performance based on goal type
///
/// Dispatches to the appropriate goal-specific analysis function based on type.
/// Returns performance metrics, confidence level, risk factors, and recommendations.
///
/// # Arguments
/// * `goal_type` - Type of goal (distance, duration, or frequency)
/// * `activities` - Historical activities for analysis
/// * `target_value` - Target value for the goal
/// * `timeframe_days` - Timeframe for goal completion in days
///
/// # Returns
/// Tuple of (`current_level`, `confidence_level`, `risk_factors`, `recommendations`)
fn analyze_goal_by_type(
    goal_type: &str,
    activities: &[Activity],
    target_value: f64,
    timeframe_days: u32,
) -> (f64, f64, Vec<String>, Vec<String>) {
    match goal_type {
        "distance" => analyze_distance_goal_feasibility(activities, target_value, timeframe_days),
        "duration" => analyze_duration_goal_feasibility(activities, target_value, timeframe_days),
        "frequency" => analyze_frequency_goal_feasibility(activities, target_value, timeframe_days),
        _ => (
            0.0,
            VERY_LOW_CONFIDENCE_LEVEL,
            vec!["Unknown goal type".to_owned()],
            vec!["Specify a valid goal type: distance, duration, or frequency".to_owned()],
        ),
    }
}

/// Handle `analyze_goal_feasibility` tool - analyze if goal is achievable
#[must_use]
pub fn handle_analyze_goal_feasibility(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "handle_analyze_goal_feasibility cancelled by user".to_owned(),
                ));
            }
        }

        let (goal_type, target_value, effective_timeframe) = extract_feasibility_params(&request)?;
        let provider_name = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .map_or_else(default_provider, String::from);
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Get historical activities
        let activities = fetch_feasibility_activities(
            executor,
            &provider_name,
            user_uuid,
            request.tenant_id.as_deref(),
        )
        .await;

        // Analyze current performance
        let (current_level, confidence_level, risk_factors, recommendations) =
            analyze_goal_by_type(&goal_type, &activities, target_value, effective_timeframe);

        let (feasibility_score, improvement_required, safe_improvement_capacity) =
            calculate_feasibility_score(current_level, target_value, effective_timeframe);
        let feasible = feasibility_score >= MODERATE_FEASIBILITY_THRESHOLD;

        let final_recommendations = generate_feasibility_recommendations(
            recommendations,
            feasible,
            improvement_required,
            safe_improvement_capacity,
            current_level,
            &goal_type,
            activities.len(),
        );

        Ok(build_feasibility_response(&FeasibilityResponseParams {
            feasibility_score,
            feasible,
            confidence_level,
            risk_factors,
            recommendations: final_recommendations,
            target_value,
            current_level,
            safe_improvement_capacity,
            effective_timeframe,
            improvement_required,
            activities_len: activities.len(),
            goal_type: &goal_type,
        }))
    })
}

/// Calculate actual training history weeks from activity date range
///
/// Calculates the number of weeks covered by activities based on their `start_date` timestamps.
/// Falls back to `ASSUMED_TRAINING_HISTORY_WEEKS` if fewer than 2 activities (cannot calculate range).
///
/// # Arguments
/// * `activities` - Slice of activities to analyze
///
/// # Returns
/// Number of weeks covered by activities, minimum 1.0 week
fn calculate_training_history_weeks(activities: &[Activity], min_activities: usize) -> f64 {
    if activities.len() < min_activities {
        return ASSUMED_TRAINING_HISTORY_WEEKS;
    }

    // Find earliest and latest activity dates
    let mut dates: Vec<DateTime<Utc>> = activities.iter().map(|a| a.start_date).collect();
    dates.sort();

    if let (Some(first), Some(last)) = (dates.first(), dates.last()) {
        let days = (*last - *first).num_days();
        let weeks = safe_i64_to_f64(days.max(1)) / 7.0;
        // Return at least 1 week, or the actual range
        weeks.max(1.0)
    } else {
        ASSUMED_TRAINING_HISTORY_WEEKS
    }
}

/// Analyze feasibility of distance goal
fn analyze_distance_goal_feasibility(
    activities: &[Activity],
    target_km: f64,
    timeframe_days: u32,
) -> (f64, f64, Vec<String>, Vec<String>) {
    if activities.is_empty() {
        return (
            0.0,
            MINIMUM_CONFIDENCE_LEVEL,
            vec!["No historical data available".to_owned()],
            vec!["Start with smaller distance goals to build baseline".to_owned()],
        );
    }

    // Calculate average distance per activity in last 30 days
    let recent_total_distance: f64 = activities
        .iter()
        .filter_map(|a| a.distance_meters)
        .sum::<f64>()
        / METERS_PER_KILOMETER;

    // Convert activity count to f64 with safe conversion helper
    let activity_count = safe_usize_to_f64(activities.len());
    let avg_distance_per_activity = recent_total_distance / activity_count;

    // Calculate actual training history from activity dates
    let training_weeks =
        calculate_training_history_weeks(activities, MIN_ACTIVITIES_FOR_TRAINING_HISTORY);
    let weeks_in_timeframe = f64::from(timeframe_days) / 7.0;
    let estimated_activities = (activity_count / training_weeks) * weeks_in_timeframe;

    let projected_distance = avg_distance_per_activity * estimated_activities;

    let mut risk_factors = Vec::new();
    let mut recommendations = Vec::new();

    if projected_distance < target_km * VOLUME_DOUBLING_THRESHOLD {
        risk_factors.push("Target requires more than doubling current volume".to_owned());
        recommendations.push("Increase training frequency gradually".to_owned());
    }

    if activity_count < MIN_ACTIVITIES_FOR_GOOD_CONFIDENCE {
        risk_factors.push("Limited training history".to_owned());
    }

    let confidence = if activity_count >= MIN_ACTIVITIES_FOR_EXCELLENT_CONFIDENCE {
        EXCELLENT_CONFIDENCE_THRESHOLD
    } else if activity_count >= MIN_ACTIVITIES_FOR_GOOD_CONFIDENCE {
        GOOD_CONFIDENCE_THRESHOLD
    } else {
        LIMITED_CONFIDENCE_LEVEL
    };

    (
        projected_distance,
        confidence,
        risk_factors,
        recommendations,
    )
}

/// Analyze feasibility of duration goal
fn analyze_duration_goal_feasibility(
    activities: &[Activity],
    _target_hours: f64,
    timeframe_days: u32,
) -> (f64, f64, Vec<String>, Vec<String>) {
    if activities.is_empty() {
        return (
            0.0,
            MINIMUM_CONFIDENCE_LEVEL,
            vec!["No historical data available".to_owned()],
            vec!["Start tracking activity duration".to_owned()],
        );
    }

    let total_duration: u64 = activities.iter().map(|a| a.duration_seconds).sum();
    let current_hours = match u32::try_from(total_duration.min(u64::from(u32::MAX))) {
        Ok(duration_u32) => f64::from(duration_u32) / SECONDS_PER_HOUR_F64,
        Err(e) => {
            warn!(
                total_duration = total_duration,
                error = %e,
                "Duration conversion failed (should not happen after min() with u32::MAX), using u32::MAX"
            );
            f64::from(u32::MAX) / SECONDS_PER_HOUR_F64
        }
    };

    let training_weeks =
        calculate_training_history_weeks(activities, MIN_ACTIVITIES_FOR_TRAINING_HISTORY);
    let weeks_in_timeframe = f64::from(timeframe_days) / 7.0;
    let projected_hours = (current_hours / training_weeks) * weeks_in_timeframe;

    let confidence = if activities.len() >= EXCELLENT_DATA_QUALITY_THRESHOLD {
        HIGH_CONFIDENCE_LEVEL
    } else {
        MEDIUM_CONFIDENCE_LEVEL
    };

    (
        projected_hours,
        confidence,
        Vec::new(),
        vec!["Maintain consistent training schedule".to_owned()],
    )
}

/// Analyze feasibility of frequency goal
fn analyze_frequency_goal_feasibility(
    activities: &[Activity],
    _target_count: f64,
    timeframe_days: u32,
) -> (f64, f64, Vec<String>, Vec<String>) {
    // Convert activity count to f64 with safe conversion helper
    let current_count = safe_usize_to_f64(activities.len());
    let training_weeks =
        calculate_training_history_weeks(activities, MIN_ACTIVITIES_FOR_TRAINING_HISTORY);
    let weeks_in_timeframe = f64::from(timeframe_days) / 7.0;
    let current_weekly_frequency = current_count / training_weeks;
    let projected_count = current_weekly_frequency * weeks_in_timeframe;

    let confidence = if current_count >= f64::from(ADEQUATE_FREQUENCY_DATA_THRESHOLD) {
        HIGH_CONFIDENCE_LEVEL
    } else {
        GOOD_CONFIDENCE_LEVEL
    };

    (
        projected_count,
        confidence,
        Vec::new(),
        vec!["Schedule training days in advance".to_owned()],
    )
}

/// Calculate training history in months from activity dates
fn calculate_training_history_months(activities: &[Activity]) -> i32 {
    if activities.is_empty() {
        return 0;
    }

    // Find earliest activity date
    let Some(earliest_date) = activities.iter().map(|a| a.start_date).min() else {
        warn!("No activities found for training history calculation, returning 0 months");
        return 0;
    };

    // Calculate months since earliest activity
    let now = Utc::now();
    let duration = now.signed_duration_since(earliest_date);
    let days = duration.num_days();

    // Convert days to months (using 30.44 days per month average)
    // Cast is safe: human activity history in days fits well within f64 precision (±10^15)
    // Result truncated to i32 is sufficient for months count (max realistic ~1200 months)
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    ((days as f64 / 30.44).round() as i32).max(0)
}

/// Detect primary sport from activity frequency
fn detect_primary_sport(activities: &[Activity]) -> Vec<String> {
    use HashMap;

    if activities.is_empty() {
        return vec![];
    }

    // Count activities by sport type
    let mut sport_counts: HashMap<String, usize> = HashMap::new();
    for activity in activities {
        let sport_name = format!("{:?}", activity.sport_type);
        *sport_counts.entry(sport_name).or_insert(0) += 1;
    }

    // Find sport with most activities
    sport_counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(sport, _)| vec![sport])
        .unwrap_or_default()
}

/// Infer fitness level from training consistency
fn infer_fitness_level(activities: &[Activity]) -> FitnessLevel {
    if activities.is_empty() {
        return FitnessLevel::Beginner;
    }

    let training_weeks =
        calculate_training_history_weeks(activities, MIN_ACTIVITIES_FOR_TRAINING_HISTORY);
    // Cast is safe: activity count (usize) far below f64 precision limit (2^53)
    #[allow(clippy::cast_precision_loss)] // Safe: realistic activity counts
    let activities_per_week = activities.len() as f64 / training_weeks;

    // Classify based on training volume and consistency
    if activities_per_week >= 5.0 && training_weeks >= 26.0 {
        FitnessLevel::Advanced
    } else if activities_per_week >= 3.0 && training_weeks >= 12.0 {
        FitnessLevel::Intermediate
    } else {
        FitnessLevel::Beginner
    }
}

/// Create a fallback user profile when database profile is unavailable
///
/// Calculates real values from activity data instead of using hardcoded defaults:
/// - `training_history_months`: calculated from earliest activity date
/// - `primary_sports`: detected from activity frequency
/// - `fitness_level`: inferred from training consistency and volume
fn create_fallback_profile(user_id: String, activities: &[Activity]) -> UserFitnessProfile {
    let training_history_months = calculate_training_history_months(activities);
    let primary_sports = detect_primary_sport(activities);
    let fitness_level = infer_fitness_level(activities);

    UserFitnessProfile {
        user_id,
        age: None,
        gender: None,
        weight: None,
        height: None,
        fitness_level,
        primary_sports,
        training_history_months,
        preferences: UserPreferences {
            preferred_units: "metric".into(),
            training_focus: vec![],
            injury_history: vec![],
            time_availability: TimeAvailability {
                hours_per_week: 3.0,
                preferred_days: vec![],
                preferred_duration_minutes: Some(30),
            },
        },
    }
}

/// Goal details extracted from database
struct GoalDetails {
    goal_type: String,
    goal_target: f64,
    timeframe: String,
    created_at: Option<DateTime<FixedOffset>>,
}

/// Extract goal details from JSON map
fn extract_goal_details(goal: &serde_json::Map<String, JsonValue>) -> Option<GoalDetails> {
    let goal_type = goal
        .get("goal_type")
        .and_then(|v| v.as_str())
        .unwrap_or("distance")
        .to_owned();

    let goal_target = goal.get("target_value").and_then(JsonValue::as_f64)?;

    let timeframe = goal
        .get("timeframe")
        .and_then(|v| v.as_str())
        .unwrap_or("month")
        .to_owned();

    let created_at = goal
        .get("created_at")
        .and_then(|v| v.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok());

    Some(GoalDetails {
        goal_type,
        goal_target,
        timeframe,
        created_at,
    })
}

/// Calculate days remaining in goal timeframe
fn calculate_days_remaining(created_at: Option<DateTime<FixedOffset>>, timeframe: &str) -> u32 {
    created_at.map_or(DEFAULT_GOAL_TIMEFRAME_DAYS, |created| {
        let timeframe_days = match timeframe {
            "week" => DAYS_PER_WEEK,
            "month" => DAYS_PER_MONTH,
            "quarter" => DAYS_PER_QUARTER,
            "year" => DAYS_PER_YEAR,
            _ => DEFAULT_GOAL_TIMEFRAME_DAYS,
        };
        let elapsed = (Utc::now() - created.with_timezone(&chrono::Utc)).num_days();
        let elapsed_u32 = match u32::try_from(elapsed.max(0)) {
            Ok(val) => val,
            Err(e) => {
                warn!(
                    elapsed = elapsed,
                    error = %e,
                    "Elapsed days conversion failed (negative or too large), using 0"
                );
                0
            }
        };
        timeframe_days.saturating_sub(elapsed_u32)
    })
}

/// Calculate current progress value based on goal type
fn calculate_current_progress(goal_type: &str, activities: &[&Activity]) -> (f64, &'static str) {
    match goal_type {
        "distance" => {
            let total_distance: f64 = activities
                .iter()
                .filter_map(|a| a.distance_meters)
                .sum::<f64>()
                / METERS_PER_KILOMETER;
            (total_distance, "km")
        }
        "duration" => {
            let total_duration: u64 = activities.iter().map(|a| a.duration_seconds).sum();
            let hours = match u32::try_from(total_duration.min(u64::from(u32::MAX))) {
                Ok(duration_u32) => f64::from(duration_u32) / SECONDS_PER_HOUR_F64,
                Err(e) => {
                    warn!(
                        total_duration = total_duration,
                        error = %e,
                        "Duration conversion failed in progress calculation, using u32::MAX"
                    );
                    f64::from(u32::MAX) / SECONDS_PER_HOUR_F64
                }
            };
            (hours, "hours")
        }
        "frequency" => {
            let count = safe_usize_to_f64(activities.len());
            (count, "activities")
        }
        _ => (0.0, "unknown"),
    }
}

/// Calculate projected completion days
fn calculate_projected_completion(
    current_value: f64,
    goal_target: f64,
    created_at: Option<DateTime<FixedOffset>>,
) -> Option<f64> {
    if current_value > 0.0 {
        let days_elapsed = created_at.map_or(1, |c| {
            (Utc::now() - c.with_timezone(&chrono::Utc))
                .num_days()
                .max(1)
        });
        let days_elapsed_f64 = safe_i64_to_f64(days_elapsed);
        let daily_rate = current_value / days_elapsed_f64;
        let remaining_value = goal_target - current_value;
        let days_needed = (remaining_value / daily_rate).ceil();
        Some(days_needed)
    } else {
        None
    }
}

/// Parameters for building progress tracking response
struct ProgressResponseParams<'a> {
    goal_id: &'a str,
    details: &'a GoalDetails,
    current_value: f64,
    unit: &'a str,
    progress_percentage: f64,
    on_track: bool,
    days_remaining: u32,
    projected_completion: Option<f64>,
    relevant_activities: &'a [&'a Activity],
    total_duration: u64,
}

/// Build progress tracking response
fn build_progress_response(params: &ProgressResponseParams) -> UniversalResponse {
    UniversalResponse {
        success: true,
        result: Some(json!({
            "goal_id": params.goal_id,
            "goal_type": params.details.goal_type,
            "current_value": params.current_value,
            "target_value": params.details.goal_target,
            "unit": params.unit,
            "progress_percentage": params.progress_percentage.min(100.0),
            "on_track": params.on_track,
            "days_remaining": params.days_remaining,
            "projected_completion_days": params.projected_completion,
            "timeframe": params.details.timeframe,
            "summary": {
                "total_activities": params.relevant_activities.len(),
                "total_distance_km": params.relevant_activities.iter().filter_map(|a| a.distance_meters).sum::<f64>() / METERS_PER_KILOMETER,
                "total_duration_hours": match u32::try_from(params.total_duration.min(u64::from(u32::MAX))) {
                    Ok(duration_u32) => f64::from(duration_u32) / SECONDS_PER_HOUR_F64,
                    Err(e) => {
                        warn!(
                            total_duration = params.total_duration,
                            error = %e,
                            "Duration conversion failed in response summary, using u32::MAX"
                        );
                        f64::from(u32::MAX) / SECONDS_PER_HOUR_F64
                    }
                }
            }
        })),
        error: None,
        metadata: Some({
            let mut map = HashMap::new();
            map.insert("goal_loaded_from_db".to_owned(), JsonValue::Bool(true));
            map.insert(
                "activities_since_goal_created".to_owned(),
                JsonValue::Number(params.relevant_activities.len().into()),
            );
            map
        }),
    }
}

/// Fetch activities for progress tracking
async fn fetch_progress_activities(
    executor: &UniversalToolExecutor,
    provider_name: &str,
    user_uuid: Uuid,
    tenant_id: Option<&str>,
) -> Result<Vec<Activity>, UniversalResponse> {
    match executor
        .auth_service
        .create_authenticated_provider(provider_name, user_uuid, tenant_id)
        .await
    {
        Ok(provider) => {
            debug!("Provider authenticated for progress tracking");
            Ok(provider
                .get_activities(Some(PROGRESS_TRACKING_ACTIVITY_LIMIT), None)
                .await
                .unwrap_or_default())
        }
        Err(response) => {
            debug!("Authentication failed for progress tracking");
            Err(response)
        }
    }
}

/// Fetch and validate goal from database
///
/// Retrieves user goals, finds the specified goal, and validates its structure.
///
/// # Arguments
/// * `database` - Database provider for fetching goals
/// * `user_uuid` - User's UUID
/// * `goal_id` - ID of the goal to find
///
/// # Returns
/// Result containing validated `GoalDetails` or error response
async fn fetch_and_validate_goal(
    database: &Database,
    user_uuid: Uuid,
    goal_id: &str,
) -> Result<GoalDetails, UniversalResponse> {
    let goals = match database.get_user_goals(user_uuid).await {
        Ok(goals) => goals,
        Err(e) => {
            return Err(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Failed to load goals from database: {e}")),
                metadata: None,
            });
        }
    };

    let Some(goal) = goals
        .iter()
        .find(|g| g.get("goal_id").and_then(|v| v.as_str()) == Some(goal_id))
    else {
        return Err(UniversalResponse {
            success: false,
            result: None,
            error: Some(format!("Goal {goal_id} not found")),
            metadata: None,
        });
    };

    let Some(goal_object) = goal.as_object() else {
        return Err(UniversalResponse {
            success: false,
            result: None,
            error: Some("Goal data is not a valid object".to_owned()),
            metadata: None,
        });
    };

    let Some(details) = extract_goal_details(goal_object) else {
        return Err(UniversalResponse {
            success: false,
            result: None,
            error: Some("Goal is missing required target_value field".to_owned()),
            metadata: None,
        });
    };

    Ok(details)
}

/// Filter activities relevant to goal timeframe
///
/// Returns activities that occurred after goal creation, or all activities if no creation date.
///
/// # Arguments
/// * `activities` - All available activities
/// * `created_at` - Optional goal creation timestamp
///
/// # Returns
/// Vector of references to relevant activities
fn filter_relevant_activities(
    activities: &[Activity],
    created_at: Option<DateTime<FixedOffset>>,
) -> Vec<&Activity> {
    created_at.map_or_else(
        || activities.iter().collect(),
        |created| {
            activities
                .iter()
                .filter(|a| a.start_date > created)
                .collect::<Vec<_>>()
        },
    )
}

/// Calculate progress metrics for goal tracking
///
/// Computes current progress value, percentage, and on-track status.
///
/// # Arguments
/// * `goal_type` - Type of goal being tracked
/// * `relevant_activities` - Activities to analyze
/// * `goal_target` - Target value for the goal
///
/// # Returns
/// Tuple of (`current_value`, `unit`, `progress_percentage`, `on_track`)
fn calculate_progress_metrics(
    goal_type: &str,
    relevant_activities: &[&Activity],
    goal_target: f64,
) -> (f64, &'static str, f64, bool) {
    let (current_value, unit) = calculate_current_progress(goal_type, relevant_activities);

    let progress_percentage = (current_value / goal_target) * PERCENTAGE_MULTIPLIER;
    let on_track = progress_percentage >= SIMPLE_PROGRESS_THRESHOLD;

    (current_value, unit, progress_percentage, on_track)
}

/// Handle `track_progress` tool - track progress towards goals
#[must_use]
pub fn handle_track_progress(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "handle_track_progress cancelled by user".to_owned(),
                ));
            }
        }

        let goal_id = request
            .parameters
            .get("goal_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::InvalidParameters("goal_id is required".into()))?
            .to_owned();

        let provider_name = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .map_or_else(default_provider, String::from);
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Fetch and validate goal
        let details = match fetch_and_validate_goal(
            &executor.resources.database,
            user_uuid,
            &goal_id,
        )
        .await
        {
            Ok(d) => d,
            Err(err_response) => return Ok(err_response),
        };

        let days_remaining = calculate_days_remaining(details.created_at, &details.timeframe);

        // Fetch activities
        let activities = match fetch_progress_activities(
            executor,
            &provider_name,
            user_uuid,
            request.tenant_id.as_deref(),
        )
        .await
        {
            Ok(acts) => acts,
            Err(err_response) => return Ok(err_response),
        };

        // Filter and calculate progress
        let relevant_activities = filter_relevant_activities(&activities, details.created_at);
        let (current_value, unit, progress_percentage, on_track) = calculate_progress_metrics(
            &details.goal_type,
            &relevant_activities,
            details.goal_target,
        );

        let total_duration: u64 = relevant_activities.iter().map(|a| a.duration_seconds).sum();
        let projected_completion =
            calculate_projected_completion(current_value, details.goal_target, details.created_at);

        Ok(build_progress_response(&ProgressResponseParams {
            goal_id: &goal_id,
            details: &details,
            current_value,
            unit,
            progress_percentage,
            on_track,
            days_remaining,
            projected_completion,
            relevant_activities: &relevant_activities,
            total_duration,
        }))
    })
}
