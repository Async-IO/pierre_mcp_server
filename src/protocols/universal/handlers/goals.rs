// ABOUTME: Goal management handlers for fitness objectives
// ABOUTME: Handle goal setting, tracking, and feasibility analysis
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::database_plugins::DatabaseProvider;
use crate::intelligence::goal_engine::GoalEngineTrait;
use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use crate::protocols::ProtocolError;
use num_traits::ToPrimitive;
use std::future::Future;
use std::pin::Pin;

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
    let goal_type = request
        .parameters
        .get("goal_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ProtocolError::InvalidParameters("goal_type is required".into()))?
        .to_string();

    let target_value = request
        .parameters
        .get("target_value")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| ProtocolError::InvalidParameters("target_value is required".into()))?;

    let timeframe_days = request
        .parameters
        .get("timeframe_days")
        .and_then(serde_json::Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(
            crate::intelligence::physiological_constants::goal_feasibility::DEFAULT_TIMEFRAME_DAYS,
        );

    let effective_timeframe = if timeframe_days > crate::constants::limits::MAX_TIMEFRAME_DAYS {
        tracing::warn!(
            "Timeframe {timeframe_days} days is unusually long, capping at {}",
            crate::constants::limits::MAX_TIMEFRAME_DAYS
        );
        crate::constants::limits::MAX_TIMEFRAME_DAYS
    } else {
        timeframe_days
    };

    Ok((goal_type, target_value, effective_timeframe))
}

/// Calculate feasibility score based on current level vs target
fn calculate_feasibility_score(
    current_level: f64,
    target_value: f64,
    effective_timeframe: u32,
) -> (f64, f64, f64) {
    let improvement_required = if current_level > 0.0 {
        ((target_value - current_level) / current_level)
            * crate::intelligence::physiological_constants::goal_feasibility::MAX_PERCENTAGE
    } else {
        crate::intelligence::physiological_constants::goal_feasibility::MAX_PERCENTAGE
    };

    let months = f64::from(effective_timeframe)
        / crate::intelligence::physiological_constants::goal_feasibility::DAYS_PER_MONTH_APPROX;
    let safe_improvement_capacity =
        crate::intelligence::physiological_constants::goal_feasibility::SAFE_MONTHLY_IMPROVEMENT_RATE_PERCENT * months;

    let feasibility_score = if improvement_required <= 0.0 {
        crate::intelligence::physiological_constants::goal_feasibility::MAX_PERCENTAGE
    } else if improvement_required <= safe_improvement_capacity {
        (improvement_required / safe_improvement_capacity).mul_add(
            -crate::intelligence::physiological_constants::goal_feasibility::SAFE_RANGE_PENALTY_FACTOR,
            crate::intelligence::physiological_constants::goal_feasibility::MAX_PERCENTAGE,
        )
    } else {
        let excess_improvement = improvement_required - safe_improvement_capacity;
        let penalty = (excess_improvement / safe_improvement_capacity)
            * crate::intelligence::physiological_constants::goal_feasibility::EXCESSIVE_IMPROVEMENT_PENALTY_FACTOR;
        (crate::intelligence::physiological_constants::goal_feasibility::UNSAFE_IMPROVEMENT_PENALTY_BASE - penalty).max(0.0)
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
        let suggested_days_f64 = (improvement_required
            / crate::intelligence::physiological_constants::goal_feasibility::SAFE_MONTHLY_IMPROVEMENT_RATE_PERCENT)
            .mul_add(f64::from(crate::constants::time_constants::DAYS_PER_MONTH), 0.0)
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

    if activities_count < crate::intelligence::physiological_constants::goal_feasibility::GOOD_DATA_QUALITY_THRESHOLD {
        recommendations.push("Build consistent training history for better goal planning".to_string());
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
    let months = f64::from(params.effective_timeframe)
        / crate::intelligence::physiological_constants::goal_feasibility::DAYS_PER_MONTH_APPROX;
    UniversalResponse {
        success: true,
        result: Some(serde_json::json!({
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
                let safe_days_f64 = (params.improvement_required / crate::intelligence::physiological_constants::goal_feasibility::SAFE_MONTHLY_IMPROVEMENT_RATE_PERCENT).mul_add(
                    f64::from(crate::constants::time_constants::DAYS_PER_MONTH),
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
                "data_quality": if params.activities_len >= crate::intelligence::physiological_constants::goal_feasibility::EXCELLENT_DATA_QUALITY_THRESHOLD { "excellent" } else if params.activities_len >= crate::intelligence::physiological_constants::goal_feasibility::GOOD_DATA_QUALITY_THRESHOLD { "good" } else { "limited" }
            }
        })),
        error: None,
        metadata: Some({
            let mut map = std::collections::HashMap::new();
            map.insert(
                "analysis_method".to_string(),
                serde_json::Value::String("historical_performance_based".to_string()),
            );
            map.insert(
                "safe_improvement_rate".to_string(),
                serde_json::Value::String("10_percent_per_month".to_string()),
            );
            map
        }),
    }
}

/// Handle `set_goal` tool - set a new fitness goal
#[must_use]
pub fn handle_set_goal(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use crate::utils::uuid::parse_user_id_for_protocol;

        let goal_type = request
            .parameters
            .get("goal_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::InvalidRequest("goal_type is required".to_string()))?;

        let target_value = request
            .parameters
            .get("target_value")
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| ProtocolError::InvalidRequest("target_value is required".to_string()))?;

        let timeframe = request
            .parameters
            .get("timeframe")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::InvalidRequest("timeframe is required".to_string()))?;

        let title = request
            .parameters
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Fitness Goal");

        // Parse user ID
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Save goal to database
        let created_at = chrono::Utc::now();
        let goal_data = serde_json::json!({
            "goal_type": goal_type,
            "target_value": target_value,
            "timeframe": timeframe,
            "title": title,
            "created_at": created_at.to_rfc3339()
        });

        let goal_id = (*executor.resources.database)
            .create_goal(user_uuid, goal_data)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Database error: {e}")))?;

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
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
        })
    })
}

/// Handle `suggest_goals` tool - get AI-suggested fitness goals
#[must_use]
pub fn handle_suggest_goals(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use crate::utils::uuid::parse_user_id_for_protocol;

        // Parse user ID
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Get recent activities using token-based approach
        let mut activities: Vec<crate::models::Activity> = Vec::new();
        if let Ok(Some(_token_data)) = executor
            .auth_service
            .get_valid_token(
                user_uuid,
                crate::constants::oauth_providers::STRAVA,
                request.tenant_id.as_deref(),
            )
            .await
        {
            // Create provider and get activities (simplified approach)
            if let Ok(provider) = executor
                .resources
                .provider_registry
                .create_provider(crate::constants::oauth_providers::STRAVA)
            {
                if let Ok(provider_activities) = provider.get_activities(Some(crate::intelligence::physiological_constants::goal_feasibility::GOAL_SUGGESTION_ACTIVITY_LIMIT), None).await {
                    activities = provider_activities;
                }
            }
        }

        // Use the goal engine from intelligence module
        let goal_engine = crate::intelligence::goal_engine::AdvancedGoalEngine::new();

        // Load user profile from database (falls back to sensible defaults if not found)
        let user_profile = match (*executor.resources.database)
            .get_user_profile(user_uuid)
            .await
        {
            Ok(Some(profile_json)) => {
                // Try to deserialize as UserFitnessProfile
                serde_json::from_value(profile_json).unwrap_or_else(|e| {
                    tracing::warn!(
                        user_id = %request.user_id,
                        error = %e,
                        "Failed to deserialize user fitness profile, using fallback profile"
                    );
                    // Fallback if profile doesn't match structure
                    create_fallback_profile(request.user_id.clone(), &activities)
                })
            }
            Ok(None) | Err(_) => create_fallback_profile(request.user_id.clone(), &activities),
        };

        match goal_engine.suggest_goals(&user_profile, &activities).await {
            Ok(suggestions) => Ok(UniversalResponse {
                success: true,
                result: Some(serde_json::json!({
                    "suggested_goals": suggestions.into_iter().map(|g| {
                        serde_json::json!({
                            "goal_type": format!("{:?}", g.goal_type),
                            "target_value": g.suggested_target,
                            "difficulty": format!("{:?}", g.difficulty),
                            "rationale": g.rationale,
                            "estimated_timeline_days": g.estimated_timeline_days,
                            "success_probability": g.success_probability
                        })
                    }).collect::<Vec<_>>(),
                    "activities_analyzed": activities.len()
                })),
                error: None,
                metadata: Some({
                    let mut map = std::collections::HashMap::with_capacity(4);
                    map.insert(
                        "analysis_engine".into(),
                        serde_json::Value::String("smart_goal_engine".into()),
                    );
                    map.insert(
                        "suggestion_algorithm".into(),
                        serde_json::Value::String("adaptive_goal_generation".into()),
                    );
                    map
                }),
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

/// Handle `analyze_goal_feasibility` tool - analyze if goal is achievable
#[must_use]
pub fn handle_analyze_goal_feasibility(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        let (goal_type, target_value, effective_timeframe) = extract_feasibility_params(&request)?;
        let user_uuid = crate::utils::uuid::parse_user_id_for_protocol(&request.user_id)?;

        // Get historical activities
        let mut activities: Vec<crate::models::Activity> =
            Vec::with_capacity(crate::constants::limits::ACTIVITY_CAPACITY_HINT);
        if let Ok(Some(_token_data)) = executor
            .auth_service
            .get_valid_token(
                user_uuid,
                crate::constants::oauth_providers::STRAVA,
                request.tenant_id.as_deref(),
            )
            .await
        {
            if let Ok(provider) = executor
                .resources
                .provider_registry
                .create_provider(crate::constants::oauth_providers::STRAVA)
            {
                if let Ok(provider_activities) = provider
                    .get_activities(
                        Some(crate::intelligence::physiological_constants::goal_feasibility::PROGRESS_TRACKING_ACTIVITY_LIMIT),
                        None,
                    )
                    .await
                {
                    activities = provider_activities;
                }
            }
        }

        // Analyze current performance
        let (current_level, confidence_level, risk_factors, recommendations) = match goal_type.as_str() {
            "distance" => analyze_distance_goal_feasibility(&activities, target_value, effective_timeframe),
            "duration" => analyze_duration_goal_feasibility(&activities, target_value, effective_timeframe),
            "frequency" => analyze_frequency_goal_feasibility(&activities, target_value, effective_timeframe),
            _ => (
                0.0,
                crate::intelligence::physiological_constants::goal_feasibility::VERY_LOW_CONFIDENCE_LEVEL,
                vec!["Unknown goal type".to_string()],
                vec!["Specify a valid goal type: distance, duration, or frequency".to_string()],
            ),
        };

        let (feasibility_score, improvement_required, safe_improvement_capacity) =
            calculate_feasibility_score(current_level, target_value, effective_timeframe);
        let feasible = feasibility_score >= crate::intelligence::physiological_constants::goal_feasibility::MODERATE_FEASIBILITY_THRESHOLD;

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
fn calculate_training_history_weeks(activities: &[crate::models::Activity]) -> f64 {
    if activities.len() < 2 {
        return crate::intelligence::physiological_constants::goal_feasibility::ASSUMED_TRAINING_HISTORY_WEEKS;
    }

    // Find earliest and latest activity dates
    let mut dates: Vec<chrono::DateTime<chrono::Utc>> =
        activities.iter().map(|a| a.start_date).collect();
    dates.sort();

    if let (Some(first), Some(last)) = (dates.first(), dates.last()) {
        let days = (*last - *first).num_days();
        let weeks = safe_i64_to_f64(days.max(1)) / 7.0;
        // Return at least 1 week, or the actual range
        weeks.max(1.0)
    } else {
        crate::intelligence::physiological_constants::goal_feasibility::ASSUMED_TRAINING_HISTORY_WEEKS
    }
}

/// Analyze feasibility of distance goal
fn analyze_distance_goal_feasibility(
    activities: &[crate::models::Activity],
    target_km: f64,
    timeframe_days: u32,
) -> (f64, f64, Vec<String>, Vec<String>) {
    if activities.is_empty() {
        return (
            0.0,
            crate::intelligence::physiological_constants::goal_feasibility::MINIMUM_CONFIDENCE_LEVEL,
            vec!["No historical data available".to_string()],
            vec!["Start with smaller distance goals to build baseline".to_string()],
        );
    }

    // Calculate average distance per activity in last 30 days
    let recent_total_distance: f64 = activities
        .iter()
        .filter_map(|a| a.distance_meters)
        .sum::<f64>()
        / crate::constants::limits::METERS_PER_KILOMETER;

    // Convert activity count to f64 with safe conversion helper
    let activity_count = safe_usize_to_f64(activities.len());
    let avg_distance_per_activity = recent_total_distance / activity_count;

    // Calculate actual training history from activity dates
    let training_weeks = calculate_training_history_weeks(activities);
    let weeks_in_timeframe = f64::from(timeframe_days) / 7.0;
    let estimated_activities = (activity_count / training_weeks) * weeks_in_timeframe;

    let projected_distance = avg_distance_per_activity * estimated_activities;

    let mut risk_factors = Vec::new();
    let mut recommendations = Vec::new();

    if projected_distance < target_km * crate::intelligence::physiological_constants::goal_feasibility::VOLUME_DOUBLING_THRESHOLD {
        risk_factors.push("Target requires more than doubling current volume".to_string());
        recommendations.push("Increase training frequency gradually".to_string());
    }

    if activity_count < crate::intelligence::physiological_constants::goal_feasibility::MIN_ACTIVITIES_FOR_GOOD_CONFIDENCE {
        risk_factors.push("Limited training history".to_string());
    }

    let confidence = if activity_count >= crate::intelligence::physiological_constants::goal_feasibility::MIN_ACTIVITIES_FOR_EXCELLENT_CONFIDENCE {
        crate::intelligence::physiological_constants::goal_feasibility::EXCELLENT_CONFIDENCE_THRESHOLD
    } else if activity_count >= crate::intelligence::physiological_constants::goal_feasibility::MIN_ACTIVITIES_FOR_GOOD_CONFIDENCE {
        crate::intelligence::physiological_constants::goal_feasibility::GOOD_CONFIDENCE_THRESHOLD
    } else {
        crate::intelligence::physiological_constants::goal_feasibility::LIMITED_CONFIDENCE_LEVEL
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
    activities: &[crate::models::Activity],
    _target_hours: f64,
    timeframe_days: u32,
) -> (f64, f64, Vec<String>, Vec<String>) {
    if activities.is_empty() {
        return (
            0.0,
            crate::intelligence::physiological_constants::goal_feasibility::MINIMUM_CONFIDENCE_LEVEL,
            vec!["No historical data available".to_string()],
            vec!["Start tracking activity duration".to_string()],
        );
    }

    let total_duration: u64 = activities.iter().map(|a| a.duration_seconds).sum();
    let current_hours =
        f64::from(u32::try_from(total_duration.min(u64::from(u32::MAX))).unwrap_or(u32::MAX))
            / crate::constants::time_constants::SECONDS_PER_HOUR_F64;

    let training_weeks = calculate_training_history_weeks(activities);
    let weeks_in_timeframe = f64::from(timeframe_days) / 7.0;
    let projected_hours = (current_hours / training_weeks) * weeks_in_timeframe;

    let confidence = if activities.len() >= crate::intelligence::physiological_constants::goal_feasibility::EXCELLENT_DATA_QUALITY_THRESHOLD { crate::intelligence::physiological_constants::goal_feasibility::HIGH_CONFIDENCE_LEVEL } else { crate::intelligence::physiological_constants::goal_feasibility::MEDIUM_CONFIDENCE_LEVEL };

    (
        projected_hours,
        confidence,
        Vec::new(),
        vec!["Maintain consistent training schedule".to_string()],
    )
}

/// Analyze feasibility of frequency goal
fn analyze_frequency_goal_feasibility(
    activities: &[crate::models::Activity],
    _target_count: f64,
    timeframe_days: u32,
) -> (f64, f64, Vec<String>, Vec<String>) {
    // Convert activity count to f64 with safe conversion helper
    let current_count = safe_usize_to_f64(activities.len());
    let training_weeks = calculate_training_history_weeks(activities);
    let weeks_in_timeframe = f64::from(timeframe_days) / 7.0;
    let current_weekly_frequency = current_count / training_weeks;
    let projected_count = current_weekly_frequency * weeks_in_timeframe;

    let confidence = if current_count >= f64::from(crate::intelligence::physiological_constants::goal_feasibility::ADEQUATE_FREQUENCY_DATA_THRESHOLD) {
        crate::intelligence::physiological_constants::goal_feasibility::HIGH_CONFIDENCE_LEVEL
    } else {
        crate::intelligence::physiological_constants::goal_feasibility::GOOD_CONFIDENCE_LEVEL
    };

    (
        projected_count,
        confidence,
        Vec::new(),
        vec!["Schedule training days in advance".to_string()],
    )
}

/// Calculate training history in months from activity dates
fn calculate_training_history_months(activities: &[crate::models::Activity]) -> i32 {
    if activities.is_empty() {
        return 0;
    }

    // Find earliest activity date
    let earliest_date = activities
        .iter()
        .map(|a| a.start_date)
        .min()
        .unwrap_or_else(chrono::Utc::now);

    // Calculate months since earliest activity
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(earliest_date);
    let days = duration.num_days();

    // Convert days to months (using 30.44 days per month average)
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    ((days as f64 / 30.44).round() as i32).max(0)
}

/// Detect primary sport from activity frequency
fn detect_primary_sport(activities: &[crate::models::Activity]) -> Vec<String> {
    use std::collections::HashMap;

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
fn infer_fitness_level(
    activities: &[crate::models::Activity],
) -> crate::intelligence::FitnessLevel {
    if activities.is_empty() {
        return crate::intelligence::FitnessLevel::Beginner;
    }

    let training_weeks = calculate_training_history_weeks(activities);
    #[allow(clippy::cast_precision_loss)]
    let activities_per_week = activities.len() as f64 / training_weeks;

    // Classify based on training volume and consistency
    if activities_per_week >= 5.0 && training_weeks >= 26.0 {
        crate::intelligence::FitnessLevel::Advanced
    } else if activities_per_week >= 3.0 && training_weeks >= 12.0 {
        crate::intelligence::FitnessLevel::Intermediate
    } else {
        crate::intelligence::FitnessLevel::Beginner
    }
}

/// Create a fallback user profile when database profile is unavailable
///
/// Calculates real values from activity data instead of using hardcoded defaults:
/// - `training_history_months`: calculated from earliest activity date
/// - `primary_sports`: detected from activity frequency
/// - `fitness_level`: inferred from training consistency and volume
fn create_fallback_profile(
    user_id: String,
    activities: &[crate::models::Activity],
) -> crate::intelligence::UserFitnessProfile {
    let training_history_months = calculate_training_history_months(activities);
    let primary_sports = detect_primary_sport(activities);
    let fitness_level = infer_fitness_level(activities);

    crate::intelligence::UserFitnessProfile {
        user_id,
        age: None,
        gender: None,
        weight: None,
        height: None,
        fitness_level,
        primary_sports,
        training_history_months,
        preferences: crate::intelligence::UserPreferences {
            preferred_units: "metric".into(),
            training_focus: vec![],
            injury_history: vec![],
            time_availability: crate::intelligence::TimeAvailability {
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
    created_at: Option<chrono::DateTime<chrono::FixedOffset>>,
}

/// Extract goal details from JSON map
fn extract_goal_details(goal: &serde_json::Map<String, serde_json::Value>) -> Option<GoalDetails> {
    let goal_type = goal
        .get("goal_type")
        .and_then(|v| v.as_str())
        .unwrap_or("distance")
        .to_string();

    let goal_target = goal
        .get("target_value")
        .and_then(serde_json::Value::as_f64)?;

    let timeframe = goal
        .get("timeframe")
        .and_then(|v| v.as_str())
        .unwrap_or("month")
        .to_string();

    let created_at = goal
        .get("created_at")
        .and_then(|v| v.as_str())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());

    Some(GoalDetails {
        goal_type,
        goal_target,
        timeframe,
        created_at,
    })
}

/// Calculate days remaining in goal timeframe
fn calculate_days_remaining(
    created_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    timeframe: &str,
) -> u32 {
    created_at.map_or(
        crate::constants::defaults::DEFAULT_GOAL_TIMEFRAME_DAYS,
        |created| {
            let timeframe_days = match timeframe {
                "week" => crate::constants::time_constants::DAYS_PER_WEEK,
                "month" => crate::constants::time_constants::DAYS_PER_MONTH,
                "quarter" => crate::constants::time_constants::DAYS_PER_QUARTER,
                "year" => crate::constants::time_constants::DAYS_PER_YEAR,
                _ => crate::constants::defaults::DEFAULT_GOAL_TIMEFRAME_DAYS,
            };
            let elapsed = (chrono::Utc::now() - created.with_timezone(&chrono::Utc)).num_days();
            timeframe_days.saturating_sub(elapsed.max(0).try_into().unwrap_or(0))
        },
    )
}

/// Calculate current progress value based on goal type
fn calculate_current_progress(
    goal_type: &str,
    activities: &[&crate::models::Activity],
) -> (f64, &'static str) {
    match goal_type {
        "distance" => {
            let total_distance: f64 = activities
                .iter()
                .filter_map(|a| a.distance_meters)
                .sum::<f64>()
                / crate::constants::limits::METERS_PER_KILOMETER;
            (total_distance, "km")
        }
        "duration" => {
            let total_duration: u64 = activities.iter().map(|a| a.duration_seconds).sum();
            let hours = f64::from(
                u32::try_from(total_duration.min(u64::from(u32::MAX))).unwrap_or(u32::MAX),
            ) / crate::constants::time_constants::SECONDS_PER_HOUR_F64;
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
    created_at: Option<chrono::DateTime<chrono::FixedOffset>>,
) -> Option<f64> {
    if current_value > 0.0 {
        let days_elapsed = created_at.map_or(1, |c| {
            (chrono::Utc::now() - c.with_timezone(&chrono::Utc))
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
    relevant_activities: &'a [&'a crate::models::Activity],
    total_duration: u64,
}

/// Build progress tracking response
fn build_progress_response(params: &ProgressResponseParams) -> UniversalResponse {
    UniversalResponse {
        success: true,
        result: Some(serde_json::json!({
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
                "total_distance_km": params.relevant_activities.iter().filter_map(|a| a.distance_meters).sum::<f64>() / crate::constants::limits::METERS_PER_KILOMETER,
                "total_duration_hours": f64::from(u32::try_from(params.total_duration.min(u64::from(u32::MAX))).unwrap_or(u32::MAX)) / crate::constants::time_constants::SECONDS_PER_HOUR_F64
            }
        })),
        error: None,
        metadata: Some({
            let mut map = std::collections::HashMap::new();
            map.insert(
                "goal_loaded_from_db".to_string(),
                serde_json::Value::Bool(true),
            );
            map.insert(
                "activities_since_goal_created".to_string(),
                serde_json::Value::Number(params.relevant_activities.len().into()),
            );
            map
        }),
    }
}

/// Fetch activities for progress tracking
async fn fetch_progress_activities(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    user_uuid: uuid::Uuid,
    tenant_id: Option<&str>,
) -> Result<Vec<crate::models::Activity>, UniversalResponse> {
    match executor
        .auth_service
        .get_valid_token(
            user_uuid,
            crate::constants::oauth_providers::STRAVA,
            tenant_id,
        )
        .await
    {
        Ok(Some(_token_data)) => {
            tracing::debug!("Token available for progress tracking");
            if let Ok(provider) = executor
                .resources
                .provider_registry
                .create_provider(crate::constants::oauth_providers::STRAVA)
            {
                if let Ok(activities) = provider
                    .get_activities(
                        Some(crate::intelligence::physiological_constants::goal_feasibility::PROGRESS_TRACKING_ACTIVITY_LIMIT),
                        None,
                    )
                    .await
                {
                    return Ok(activities);
                }
            }
            Ok(Vec::new())
        }
        Ok(None) => {
            tracing::debug!("No token available for progress tracking");
            Ok(Vec::new())
        }
        Err(e) => {
            tracing::debug!("Token error for progress tracking: {e}");
            Err(UniversalResponse {
                success: false,
                result: None,
                error: Some("Authentication required for progress tracking".to_string()),
                metadata: None,
            })
        }
    }
}

/// Handle `track_progress` tool - track progress towards goals
#[must_use]
pub fn handle_track_progress(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        let goal_id = request
            .parameters
            .get("goal_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::InvalidParameters("goal_id is required".into()))?
            .to_string();

        let user_uuid = crate::utils::uuid::parse_user_id_for_protocol(&request.user_id)?;

        let goals = match (*executor.resources.database)
            .get_user_goals(user_uuid)
            .await
        {
            Ok(goals) => goals,
            Err(e) => {
                return Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!("Failed to load goals from database: {e}")),
                    metadata: None,
                });
            }
        };

        let Some(goal) = goals
            .iter()
            .find(|g| g.get("goal_id").and_then(|v| v.as_str()) == Some(&goal_id))
        else {
            return Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Goal {goal_id} not found")),
                metadata: None,
            });
        };

        let Some(goal_object) = goal.as_object() else {
            return Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some("Goal data is not a valid object".to_string()),
                metadata: None,
            });
        };

        let Some(details) = extract_goal_details(goal_object) else {
            return Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some("Goal is missing required target_value field".to_string()),
                metadata: None,
            });
        };

        let days_remaining = calculate_days_remaining(details.created_at, &details.timeframe);

        let activities = match fetch_progress_activities(
            executor,
            user_uuid,
            request.tenant_id.as_deref(),
        )
        .await
        {
            Ok(acts) => acts,
            Err(err_response) => return Ok(err_response),
        };

        let relevant_activities = details.created_at.map_or_else(
            || activities.iter().collect(),
            |created| {
                activities
                    .iter()
                    .filter(|a| a.start_date > created)
                    .collect::<Vec<_>>()
            },
        );

        let (current_value, unit) =
            calculate_current_progress(&details.goal_type, &relevant_activities);

        let total_duration: u64 = relevant_activities.iter().map(|a| a.duration_seconds).sum();
        let progress_percentage =
            (current_value / details.goal_target) * crate::constants::limits::PERCENTAGE_MULTIPLIER;
        let on_track = progress_percentage
            >= crate::intelligence::physiological_constants::goal_feasibility::SIMPLE_PROGRESS_THRESHOLD;

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
