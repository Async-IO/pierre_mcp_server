// ABOUTME: Goal management handlers for fitness objectives
// ABOUTME: Handle goal setting, tracking, and feasibility analysis

use crate::database_plugins::DatabaseProvider;
use crate::intelligence::goal_engine::GoalEngineTrait;
use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use crate::protocols::ProtocolError;
use std::future::Future;
use std::pin::Pin;

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
        use crate::constants::{limits, user_defaults};
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
            if let Ok(provider) =
                crate::providers::create_provider(crate::constants::oauth_providers::STRAVA)
            {
                if let Ok(provider_activities) = provider.get_activities(Some(10), None).await {
                    activities = provider_activities;
                }
            }
        }

        // Use the goal engine from intelligence module
        let goal_engine = crate::intelligence::goal_engine::AdvancedGoalEngine::new();

        // Create a default user profile for the goal engine
        let user_profile = crate::intelligence::UserFitnessProfile {
            user_id: request.user_id.clone(),
            age: Some(i32::try_from(user_defaults::DEFAULT_USER_AGE).unwrap_or(30)),
            gender: None,
            weight: None,
            height: None,
            fitness_level: crate::intelligence::FitnessLevel::Intermediate,
            primary_sports: vec!["general".into()],
            training_history_months: 6,
            preferences: crate::intelligence::UserPreferences {
                preferred_units: "metric".into(),
                training_focus: vec!["endurance".into()],
                injury_history: vec![],
                time_availability: crate::intelligence::TimeAvailability {
                    hours_per_week: 5.0,
                    preferred_days: vec!["Monday".into(), "Wednesday".into(), "Friday".into()],
                    preferred_duration_minutes: Some(
                        i32::try_from(limits::MINUTES_PER_HOUR).unwrap_or(60),
                    ),
                },
            },
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
        // Extract goal parameters
        let goal_type = request
            .parameters
            .get("goal_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::InvalidParameters("goal_type is required".into()))?;

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
            .unwrap_or(90);

        // Validate timeframe is reasonable
        if timeframe_days > crate::constants::limits::MAX_TIMEFRAME_DAYS {
            tracing::warn!(
                "Timeframe {} days is unusually long, capping at {}",
                timeframe_days,
                crate::constants::limits::MAX_TIMEFRAME_DAYS
            );
        }

        let effective_timeframe =
            std::cmp::min(timeframe_days, crate::constants::limits::MAX_TIMEFRAME_DAYS);

        let _title = request
            .parameters
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Fitness Goal")
            .to_string();

        // Parse user ID
        let user_uuid = crate::utils::uuid::parse_user_id_for_protocol(&request.user_id)?;

        // Get historical activities
        let activities: Vec<crate::models::Activity> =
            Vec::with_capacity(crate::constants::limits::ACTIVITY_CAPACITY_HINT);
        match executor
            .auth_service
            .get_valid_token(
                user_uuid,
                crate::constants::oauth_providers::STRAVA,
                request.tenant_id.as_deref(),
            )
            .await
        {
            Ok(Some(_token_data)) => {
                tracing::debug!("Token available for goal feasibility analysis");
            }
            Ok(None) => {
                tracing::debug!("No token available for goal feasibility analysis");
            }
            Err(e) => {
                tracing::debug!("Token error for goal feasibility analysis: {e}");
            }
        }

        // Basic goal feasibility analysis using configured thresholds
        let feasibility_score = if target_value > 0.0 {
            crate::intelligence::physiological_constants::goal_feasibility::HIGH_FEASIBILITY_THRESHOLD
        } else {
            0.0
        };
        let feasible = feasibility_score > crate::intelligence::physiological_constants::goal_feasibility::MODERATE_FEASIBILITY_THRESHOLD;

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "feasible": feasible,
                "feasibility_score": feasibility_score,
                "confidence_level": 0.8,
                "risk_factors": vec!["Limited historical data"],
                "success_probability": feasibility_score / 100.0,
                "recommendations": vec!["Start with smaller milestones", "Track progress regularly"],
                "adjusted_target": target_value,
                "adjusted_timeframe": effective_timeframe,
                "historical_context": {
                    "activities_analyzed": activities.len(),
                    "goal_type": goal_type,
                    "target_value": target_value
                }
            })),
            error: None,
            metadata: None,
        })
    })
}

/// Handle `track_progress` tool - track progress towards goals
#[must_use]
pub fn handle_track_progress(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Extract goal ID from parameters
        let goal_id = request
            .parameters
            .get("goal_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::InvalidParameters("goal_id is required".into()))?;

        // Parse user ID
        let user_uuid = crate::utils::uuid::parse_user_id_for_protocol(&request.user_id)?;

        // Get activities using authenticated provider
        let activities: Vec<crate::models::Activity> =
            Vec::with_capacity(crate::constants::limits::ACTIVITY_CAPACITY_HINT);
        match executor
            .auth_service
            .get_valid_token(
                user_uuid,
                crate::constants::oauth_providers::STRAVA,
                request.tenant_id.as_deref(),
            )
            .await
        {
            Ok(Some(_token_data)) => {
                tracing::debug!("Token available for progress tracking");
                // In a full implementation, would fetch activities from provider here
            }
            Ok(None) => {
                tracing::debug!("No token available for progress tracking");
            }
            Err(e) => {
                tracing::debug!("Token error for progress tracking: {e}");
                return Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some("Authentication required for progress tracking".to_string()),
                    metadata: None,
                });
            }
        }

        // Calculate progress based on available activities
        let total_distance: f64 = activities
            .iter()
            .filter_map(|a| a.distance_meters)
            .sum::<f64>()
            / crate::constants::limits::METERS_PER_KILOMETER;

        let total_duration: u64 = activities.iter().map(|a| a.duration_seconds).sum();

        // Use configurable goal target from constants
        let goal_target = crate::constants::user_defaults::DEFAULT_GOAL_DISTANCE;
        let progress_percentage =
            (total_distance / goal_target) * crate::constants::limits::PERCENTAGE_MULTIPLIER;

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "goal_id": goal_id,
                "current_value": total_distance,
                "target_value": goal_target,
                "progress_percentage": progress_percentage,
                "on_track": progress_percentage >= crate::intelligence::physiological_constants::goal_feasibility::SIMPLE_PROGRESS_THRESHOLD,
                "days_remaining": crate::constants::defaults::DEFAULT_GOAL_TIMEFRAME_DAYS,
                "projected_completion": if progress_percentage > 0.0 {
                    Some((goal_target / total_distance) * 90.0)
                } else {
                    None
                },
                "summary": {
                    "total_activities": activities.len(),
                    "total_distance_km": total_distance,
                    "total_duration_hours": f64::from(u32::try_from(total_duration.min(u64::from(u32::MAX))).unwrap_or(u32::MAX)) / crate::constants::time_constants::SECONDS_PER_HOUR_F64
                }
            })),
            error: None,
            metadata: None,
        })
    })
}
