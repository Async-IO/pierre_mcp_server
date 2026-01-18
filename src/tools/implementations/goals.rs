// ABOUTME: Goal management tools for setting and tracking fitness goals.
// ABOUTME: Implements set_goal, suggest_goals, track_progress, analyze_goal_feasibility.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Goal Management Tools
//!
//! This module provides tools for fitness goal management:
//! - `SetGoalTool` - Create a new fitness goal
//! - `SuggestGoalsTool` - Get AI-suggested fitness goals
//! - `TrackProgressTool` - Track progress toward goals
//! - `AnalyzeGoalFeasibilityTool` - Assess goal achievability
//!
//! Uses the goal engine directly for clean, efficient goal management.

use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use tracing::info;

use crate::config::environment::default_provider;
use crate::database_plugins::DatabaseProvider;
use crate::errors::AppError;
use crate::errors::AppResult;
use crate::intelligence::goal_engine::{AdvancedGoalEngine, GoalDifficulty, GoalEngineTrait};
use crate::intelligence::{
    FitnessLevel, Goal, GoalStatus, GoalType, ProgressReport, TimeAvailability, TimeFrame,
    UserFitnessProfile, UserPreferences,
};
use crate::mcp::schema::{JsonSchema, PropertySchema};
use crate::models::Activity;
use crate::protocols::universal::auth_service::AuthService;
use crate::providers::core::{ActivityQueryParams, FitnessProvider};
use crate::tools::context::ToolExecutionContext;
use crate::tools::result::ToolResult;
use crate::tools::traits::{McpTool, ToolCapabilities};

// ============================================================================
// Helper functions
// ============================================================================

/// Create an authenticated provider from context
async fn create_provider(
    context: &ToolExecutionContext,
    provider_name: &str,
) -> Result<Box<dyn FitnessProvider>, ToolResult> {
    let auth_service = AuthService::new(context.resources.clone());
    let tenant_id = context.tenant_id.map(|id| id.to_string());

    auth_service
        .create_authenticated_provider(provider_name, context.user_id, tenant_id.as_deref())
        .await
        .map_err(|response| {
            ToolResult::error(json!({
                "error": response.error.unwrap_or_else(|| "Authentication failed".to_owned()),
                "provider": provider_name
            }))
        })
}

/// Fetch activities for goal analysis
async fn fetch_activities(
    provider: &dyn FitnessProvider,
    limit: usize,
) -> Result<Vec<Activity>, String> {
    let query_params = ActivityQueryParams {
        limit: Some(limit),
        offset: None,
        before: None,
        after: None,
    };

    provider
        .get_activities_with_params(&query_params)
        .await
        .map_err(|e| format!("Failed to fetch activities: {e}"))
}

/// Create a basic user profile from activities for goal suggestions
fn create_profile_from_activities(user_id: &str, activities: &[Activity]) -> UserFitnessProfile {
    let fitness_level = if activities.len() >= 50 {
        FitnessLevel::Advanced
    } else if activities.len() >= 20 {
        FitnessLevel::Intermediate
    } else {
        FitnessLevel::Beginner
    };

    let primary_sports: Vec<String> = activities
        .iter()
        .take(10)
        .map(|a| format!("{:?}", a.sport_type()))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    #[allow(clippy::cast_possible_truncation)]
    let training_months = if activities.is_empty() {
        0
    } else {
        let oldest = activities.iter().map(Activity::start_date).min();
        oldest.map_or(0, |date| {
            ((Utc::now() - date).num_days() / 30).min(i64::from(i32::MAX)) as i32
        })
    };

    UserFitnessProfile {
        user_id: user_id.to_owned(),
        age: None,
        gender: None,
        weight: None,
        height: None,
        fitness_level,
        primary_sports,
        training_history_months: training_months,
        preferences: UserPreferences {
            preferred_units: "metric".to_owned(),
            training_focus: vec![],
            injury_history: vec![],
            time_availability: TimeAvailability {
                hours_per_week: 5.0,
                preferred_days: vec![],
                preferred_duration_minutes: Some(60),
            },
        },
    }
}

/// Extracted goal details from JSON data
struct ExtractedGoalDetails {
    goal_type_str: String,
    target_value: f64,
    sport: Option<String>,
    title: String,
    created_at: chrono::DateTime<Utc>,
    target_date: chrono::DateTime<Utc>,
}

/// Extract goal details from goal JSON data
fn extract_goal_details_from_json(goal_data: &Value) -> ExtractedGoalDetails {
    let goal_type_str = goal_data
        .get("goal_type")
        .and_then(Value::as_str)
        .unwrap_or("distance")
        .to_owned();
    let target_value = goal_data
        .get("target_value")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let sport = goal_data
        .get("sport")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let title = goal_data
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Fitness Goal")
        .to_owned();
    let created_at_str = goal_data.get("created_at").and_then(Value::as_str);
    let target_date_str = goal_data.get("target_date").and_then(Value::as_str);

    let created_at = created_at_str
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map_or_else(Utc::now, |dt: chrono::DateTime<chrono::FixedOffset>| {
            dt.with_timezone(&Utc)
        });

    let target_date = target_date_str
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map_or_else(
            || Utc::now() + Duration::days(30),
            |dt: chrono::DateTime<chrono::FixedOffset>| dt.with_timezone(&Utc),
        );

    ExtractedGoalDetails {
        goal_type_str,
        target_value,
        sport,
        title,
        created_at,
        target_date,
    }
}

/// Build a Goal struct from extracted details
fn build_goal_from_details(goal_id: &str, user_id: &str, details: &ExtractedGoalDetails) -> Goal {
    Goal {
        id: goal_id.to_owned(),
        user_id: user_id.to_owned(),
        title: details.title.clone(),
        description: String::new(),
        goal_type: parse_goal_type(
            &details.goal_type_str,
            details.target_value,
            details.sport.as_deref(),
        ),
        target_value: details.target_value,
        current_value: 0.0,
        created_at: details.created_at,
        target_date: details.target_date,
        updated_at: Utc::now(),
        status: GoalStatus::Active,
    }
}

/// Build the progress tracking response JSON
fn build_progress_response(
    goal_id: &str,
    progress: &ProgressReport,
    activities_count: usize,
    provider_name: &str,
) -> ToolResult {
    let milestones: Vec<Value> = progress
        .milestones_achieved
        .iter()
        .map(|m| {
            json!({
                "name": m.name,
                "target": m.target_value,
                "achieved": m.achieved,
                "achieved_date": m.achieved_date.map(|d: chrono::DateTime<Utc>| d.to_rfc3339())
            })
        })
        .collect();

    let insights: Vec<Value> = progress
        .insights
        .iter()
        .map(|i| {
            json!({
                "type": i.insight_type,
                "message": i.message,
                "severity": format!("{:?}", i.severity)
            })
        })
        .collect();

    ToolResult::ok(json!({
        "goal_id": goal_id,
        "progress_percentage": progress.progress_percentage.round(),
        "on_track": progress.on_track,
        "completion_estimate": progress.completion_date_estimate.map(|d: chrono::DateTime<Utc>| d.to_rfc3339()),
        "milestones": milestones,
        "insights": insights,
        "recommendations": progress.recommendations,
        "activities_analyzed": activities_count,
        "provider": provider_name
    }))
}

/// Parse goal type from parameters
fn parse_goal_type(goal_type: &str, target: f64, sport: Option<&str>) -> GoalType {
    let sport_name = sport.unwrap_or("Running").to_owned();
    match goal_type.to_lowercase().as_str() {
        "distance" => GoalType::Distance {
            sport: sport_name,
            timeframe: TimeFrame::Month,
        },
        "time" => GoalType::Time {
            sport: sport_name,
            distance: target * 1000.0, // Convert km to meters
        },
        "frequency" => GoalType::Frequency {
            sport: sport_name,
            #[allow(clippy::cast_possible_truncation)]
            sessions_per_week: target.round() as i32,
        },
        "performance" => GoalType::Performance {
            metric: "pace".to_owned(),
            improvement_percent: target,
        },
        _ => GoalType::Custom {
            metric: goal_type.to_owned(),
            unit: "units".to_owned(),
        },
    }
}

// ============================================================================
// SetGoalTool - Create a new fitness goal
// ============================================================================

/// Tool for creating a new fitness goal.
pub struct SetGoalTool;

#[async_trait]
impl McpTool for SetGoalTool {
    fn name(&self) -> &'static str {
        "set_goal"
    }

    fn description(&self) -> &'static str {
        "Create a new fitness goal with specified type, target value, and timeframe"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "goal_type".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Type of goal: 'distance', 'time', 'frequency', or 'performance'".to_owned(),
                ),
            },
        );
        properties.insert(
            "target_value".to_owned(),
            PropertySchema {
                property_type: "number".to_owned(),
                description: Some(
                    "Target value for the goal (km for distance, sessions for frequency, etc.)"
                        .to_owned(),
                ),
            },
        );
        properties.insert(
            "timeframe".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Goal timeframe: 'week', 'month', 'quarter', or 'year'. Default: 'month'"
                        .to_owned(),
                ),
            },
        );
        properties.insert(
            "title".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("Title or description for the goal".to_owned()),
            },
        );
        properties.insert(
            "sport".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Sport type for the goal (e.g., 'Running', 'Cycling'). Default: 'Running'"
                        .to_owned(),
                ),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["goal_type".to_owned(), "target_value".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::WRITES_DATA
    }

    async fn execute(&self, args: Value, context: &ToolExecutionContext) -> AppResult<ToolResult> {
        let goal_type = args
            .get("goal_type")
            .and_then(Value::as_str)
            .unwrap_or("distance");

        let target_value = args
            .get("target_value")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);

        let timeframe = args
            .get("timeframe")
            .and_then(Value::as_str)
            .unwrap_or("month");

        let title = args
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("Fitness Goal");

        let sport = args.get("sport").and_then(Value::as_str);

        if target_value <= 0.0 {
            return Ok(ToolResult::error(json!({
                "error": "target_value must be a positive number",
                "provided": target_value
            })));
        }

        let goal_id = uuid::Uuid::new_v4().to_string();
        let created_at = Utc::now();
        let target_date = match timeframe {
            "week" => created_at + Duration::weeks(1),
            "quarter" => created_at + Duration::days(90),
            "year" => created_at + Duration::days(365),
            _ => created_at + Duration::days(30), // month default
        };

        // Store goal in database
        let goal_data = json!({
            "goal_type": goal_type,
            "target_value": target_value,
            "timeframe": timeframe,
            "title": title,
            "sport": sport.unwrap_or("Running"),
            "created_at": created_at.to_rfc3339(),
            "target_date": target_date.to_rfc3339()
        });

        match context
            .resources
            .database
            .create_goal(context.user_id, goal_data)
            .await
        {
            Ok(stored_goal_id) => {
                info!(
                    "Goal created: {} for user {} - {} {} by {}",
                    stored_goal_id, context.user_id, target_value, goal_type, timeframe
                );

                Ok(ToolResult::ok(json!({
                    "goal_id": stored_goal_id,
                    "goal_type": goal_type,
                    "target_value": target_value,
                    "timeframe": timeframe,
                    "title": title,
                    "sport": sport.unwrap_or("Running"),
                    "created_at": created_at.to_rfc3339(),
                    "target_date": target_date.to_rfc3339(),
                    "status": "created"
                })))
            }
            Err(e) => Ok(ToolResult::error(json!({
                "error": format!("Failed to create goal: {e}"),
                "goal_id": goal_id
            }))),
        }
    }
}

// ============================================================================
// SuggestGoalsTool - Get AI-suggested goals
// ============================================================================

/// Tool for getting AI-suggested fitness goals.
pub struct SuggestGoalsTool;

#[async_trait]
impl McpTool for SuggestGoalsTool {
    fn name(&self) -> &'static str {
        "suggest_goals"
    }

    fn description(&self) -> &'static str {
        "Get AI-suggested fitness goals based on your activity history and fitness level"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "provider".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Fitness provider to analyze. Defaults to configured provider.".to_owned(),
                ),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: None,
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, context: &ToolExecutionContext) -> AppResult<ToolResult> {
        let provider_name = args
            .get("provider")
            .and_then(Value::as_str)
            .map_or_else(default_provider, String::from);

        let provider = match create_provider(context, &provider_name).await {
            Ok(p) => p,
            Err(result) => return Ok(result),
        };

        let activities = match fetch_activities(provider.as_ref(), 100).await {
            Ok(acts) => acts,
            Err(e) => {
                return Ok(ToolResult::error(json!({
                    "error": e,
                    "provider": provider_name
                })));
            }
        };

        if activities.is_empty() {
            return Ok(ToolResult::ok(json!({
                "message": "No activities found. Start tracking activities to get personalized goal suggestions.",
                "suggested_goals": [],
                "provider": provider_name
            })));
        }

        let user_profile =
            create_profile_from_activities(&context.user_id.to_string(), &activities);
        let goal_engine = AdvancedGoalEngine::new();

        match goal_engine.suggest_goals(&user_profile, &activities).await {
            Ok(suggestions) => {
                let formatted_suggestions: Vec<Value> = suggestions
                    .into_iter()
                    .map(|s| {
                        json!({
                            "goal_type": format!("{:?}", s.goal_type),
                            "target_value": s.suggested_target,
                            "difficulty": format!("{:?}", s.difficulty),
                            "rationale": s.rationale,
                            "estimated_days": s.estimated_timeline_days,
                            "success_probability": (s.success_probability * 100.0).round()
                        })
                    })
                    .collect();

                info!(
                    "Generated {} goal suggestions for user {}",
                    formatted_suggestions.len(),
                    context.user_id
                );

                Ok(ToolResult::ok(json!({
                    "suggested_goals": formatted_suggestions,
                    "activities_analyzed": activities.len(),
                    "fitness_level": format!("{:?}", user_profile.fitness_level),
                    "provider": provider_name
                })))
            }
            Err(e) => Ok(ToolResult::error(json!({
                "error": format!("Failed to generate goal suggestions: {e}"),
                "provider": provider_name
            }))),
        }
    }
}

// ============================================================================
// TrackProgressTool - Track goal progress
// ============================================================================

/// Tool for tracking progress toward fitness goals.
pub struct TrackProgressTool;

#[async_trait]
impl McpTool for TrackProgressTool {
    fn name(&self) -> &'static str {
        "track_progress"
    }

    fn description(&self) -> &'static str {
        "Track progress toward a specific fitness goal with milestone achievements and projections"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "goal_id".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("ID of the goal to track progress for".to_owned()),
            },
        );
        properties.insert(
            "provider".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Fitness provider to query. Defaults to configured provider.".to_owned(),
                ),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["goal_id".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, context: &ToolExecutionContext) -> AppResult<ToolResult> {
        let Some(goal_id) = args.get("goal_id").and_then(Value::as_str) else {
            return Ok(ToolResult::error(json!({ "error": "goal_id is required" })));
        };

        let provider_name = args
            .get("provider")
            .and_then(Value::as_str)
            .map_or_else(default_provider, String::from);

        // Load and find goal from database
        let goals: Vec<Value> = context
            .resources
            .database
            .get_user_goals(context.user_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to load goals: {e}")))?;

        let Some(goal_data) = goals
            .iter()
            .find(|g: &&Value| g.get("goal_id").and_then(Value::as_str) == Some(goal_id))
        else {
            return Ok(ToolResult::error(json!({
                "error": format!("Goal {goal_id} not found"),
                "goal_id": goal_id
            })));
        };

        // Extract goal details and build Goal struct
        let details = extract_goal_details_from_json(goal_data);
        let goal = build_goal_from_details(goal_id, &context.user_id.to_string(), &details);

        // Fetch activities since goal creation
        let provider = match create_provider(context, &provider_name).await {
            Ok(p) => p,
            Err(result) => return Ok(result),
        };

        let query_params = ActivityQueryParams {
            limit: Some(200),
            offset: None,
            before: None,
            after: Some(details.created_at.timestamp()),
        };

        let activities = provider
            .get_activities_with_params(&query_params)
            .await
            .unwrap_or_default();

        // Track progress using goal engine
        let goal_engine = AdvancedGoalEngine::new();

        match goal_engine.track_progress(&goal, &activities).await {
            Ok(progress) => {
                info!(
                    "Progress tracked for goal {}: {:.1}%",
                    goal_id, progress.progress_percentage
                );
                Ok(build_progress_response(
                    goal_id,
                    &progress,
                    activities.len(),
                    &provider_name,
                ))
            }
            Err(e) => Ok(ToolResult::error(json!({
                "error": format!("Failed to track progress: {e}"),
                "goal_id": goal_id
            }))),
        }
    }
}

// ============================================================================
// AnalyzeGoalFeasibilityTool - Assess goal achievability
// ============================================================================

/// Feasibility analysis results
struct FeasibilityAnalysis {
    score: f64,
    feasible: bool,
    improvement_percent: f64,
    safe_capacity: f64,
    months: f64,
    difficulty: GoalDifficulty,
}

/// Calculate feasibility metrics
fn calculate_feasibility(
    current_level: f64,
    target_value: f64,
    timeframe_days: u32,
) -> FeasibilityAnalysis {
    let improvement_percent = if current_level > 0.0 {
        ((target_value - current_level) / current_level) * 100.0
    } else {
        100.0
    };

    let months = f64::from(timeframe_days) / 30.0;
    let safe_capacity = 10.0 * months;

    let score = if improvement_percent <= 0.0 {
        100.0
    } else if improvement_percent <= safe_capacity {
        (improvement_percent / safe_capacity).mul_add(-30.0, 100.0)
    } else {
        let excess = improvement_percent - safe_capacity;
        (excess / safe_capacity).mul_add(-50.0, 70.0).max(0.0)
    };

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let difficulty = match score as u32 {
        80..=100 => GoalDifficulty::Easy,
        60..=79 => GoalDifficulty::Moderate,
        40..=59 => GoalDifficulty::Challenging,
        20..=39 => GoalDifficulty::Ambitious,
        _ => GoalDifficulty::Unknown,
    };

    FeasibilityAnalysis {
        score,
        feasible: score >= 50.0,
        improvement_percent,
        safe_capacity,
        months,
        difficulty,
    }
}

/// Parameters for building feasibility response
struct FeasibilityResponseParams<'a> {
    analysis: &'a FeasibilityAnalysis,
    current_level: f64,
    target_value: f64,
    confidence: f64,
    timeframe_days: u32,
    risk_factors: &'a [&'a str],
    recommendations: &'a [String],
    activities_count: usize,
    provider_name: &'a str,
}

/// Build feasibility response JSON
fn build_feasibility_response(params: &FeasibilityResponseParams) -> ToolResult {
    let analysis = params.analysis;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let suggested_days = (analysis.improvement_percent / 10.0 * 30.0).ceil() as u32;

    ToolResult::ok(json!({
        "feasible": analysis.feasible,
        "feasibility_score": analysis.score.round(),
        "difficulty": format!("{:?}", analysis.difficulty),
        "success_probability": (analysis.score / 100.0).min(1.0),
        "confidence_level": params.confidence,
        "analysis": {
            "current_level": params.current_level,
            "target_value": params.target_value,
            "improvement_required_percent": analysis.improvement_percent.round(),
            "safe_improvement_capacity_percent": analysis.safe_capacity.round(),
            "timeframe_days": params.timeframe_days,
            "timeframe_months": analysis.months
        },
        "risk_factors": params.risk_factors,
        "recommendations": params.recommendations,
        "adjusted_suggestions": {
            "safer_target": params.current_level * (1.0 + analysis.safe_capacity / 100.0),
            "suggested_timeframe_days": if analysis.feasible { params.timeframe_days } else { suggested_days }
        },
        "activities_analyzed": params.activities_count,
        "provider": params.provider_name
    }))
}

/// Tool for analyzing if a fitness goal is achievable.
pub struct AnalyzeGoalFeasibilityTool;

#[async_trait]
impl McpTool for AnalyzeGoalFeasibilityTool {
    fn name(&self) -> &'static str {
        "analyze_goal_feasibility"
    }

    fn description(&self) -> &'static str {
        "Analyze whether a fitness goal is achievable based on your current fitness level and training history"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "goal_type".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Type of goal: 'distance', 'time', 'frequency', or 'performance'".to_owned(),
                ),
            },
        );
        properties.insert(
            "target_value".to_owned(),
            PropertySchema {
                property_type: "number".to_owned(),
                description: Some("Target value for the goal".to_owned()),
            },
        );
        properties.insert(
            "timeframe_days".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Number of days to achieve the goal. Default: 30.".to_owned()),
            },
        );
        properties.insert(
            "provider".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Fitness provider to analyze. Defaults to configured provider.".to_owned(),
                ),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["goal_type".to_owned(), "target_value".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, context: &ToolExecutionContext) -> AppResult<ToolResult> {
        let Some(goal_type) = args.get("goal_type").and_then(Value::as_str) else {
            return Ok(ToolResult::error(
                json!({ "error": "goal_type is required" }),
            ));
        };

        let Some(target_value) = args
            .get("target_value")
            .and_then(Value::as_f64)
            .filter(|&tv| tv > 0.0)
        else {
            return Ok(ToolResult::error(json!({
                "error": "target_value must be a positive number"
            })));
        };

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let timeframe_days = args
            .get("timeframe_days")
            .and_then(Value::as_i64)
            .unwrap_or(30)
            .min(365) as u32;

        let provider_name = args
            .get("provider")
            .and_then(Value::as_str)
            .map_or_else(default_provider, String::from);

        let provider = match create_provider(context, &provider_name).await {
            Ok(p) => p,
            Err(result) => return Ok(result),
        };

        let activities = match fetch_activities(provider.as_ref(), 100).await {
            Ok(acts) => acts,
            Err(e) => {
                return Ok(ToolResult::error(
                    json!({ "error": e, "provider": provider_name }),
                ))
            }
        };

        // Calculate current level and feasibility
        let (current_level, confidence) = calculate_current_level(goal_type, &activities);
        let analysis = calculate_feasibility(current_level, target_value, timeframe_days);

        // Generate risk factors and recommendations
        let mut risk_factors: Vec<&str> = Vec::new();
        let mut recommendations = Vec::new();

        if analysis.improvement_percent > analysis.safe_capacity * 2.0 {
            risk_factors
                .push("Target requires significant improvement beyond safe progression rate");
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let suggested_days = (analysis.improvement_percent / 10.0 * 30.0).ceil() as u32;
            recommendations.push(format!(
                "Consider extending timeframe to {suggested_days} days"
            ));
        }

        if activities.len() < 10 {
            risk_factors.push("Limited training history for accurate assessment");
            recommendations.push("Build more training data for better predictions".to_owned());
        }

        if analysis.feasible {
            recommendations.push("Maintain consistent training schedule".to_owned());
        } else {
            let safer = current_level * (1.0 + analysis.safe_capacity / 100.0);
            recommendations.push(format!(
                "Consider reducing target to {safer:.1} for this timeframe"
            ));
        }

        info!(
            "Feasibility analysis for {} goal: {:.1}% feasible",
            goal_type, analysis.score
        );

        Ok(build_feasibility_response(&FeasibilityResponseParams {
            analysis: &analysis,
            current_level,
            target_value,
            confidence,
            timeframe_days,
            risk_factors: &risk_factors,
            recommendations: &recommendations,
            activities_count: activities.len(),
            provider_name: &provider_name,
        }))
    }
}

/// Calculate current performance level based on goal type and activities
fn calculate_current_level(goal_type: &str, activities: &[Activity]) -> (f64, f64) {
    if activities.is_empty() {
        return (0.0, 0.2);
    }

    #[allow(clippy::cast_precision_loss)]
    let confidence = match activities.len() {
        0..=5 => 0.3,
        6..=20 => 0.6,
        21..=50 => 0.8,
        _ => 0.9,
    };

    #[allow(clippy::cast_precision_loss)]
    let current_level = match goal_type {
        "distance" => {
            // Average weekly distance in km
            let total_km: f64 = activities
                .iter()
                .filter_map(Activity::distance_meters)
                .sum::<f64>()
                / 1000.0;
            let weeks = (activities.len() as f64 / 3.0).max(1.0);
            total_km / weeks
        }
        "duration" | "time" => {
            // Average weekly hours
            let total_hours: f64 = activities
                .iter()
                .map(|a| a.duration_seconds() as f64 / 3600.0)
                .sum();
            let weeks = (activities.len() as f64 / 3.0).max(1.0);
            total_hours / weeks
        }
        "frequency" => {
            // Current weekly frequency
            let count = activities.len() as f64;
            let weeks = 4.0;
            count / weeks
        }
        _ => 0.0,
    };

    (current_level, confidence)
}

// ============================================================================
// Module exports
// ============================================================================

/// Create all goal management tools for registration
#[must_use]
pub fn create_goal_tools() -> Vec<Box<dyn McpTool>> {
    vec![
        Box::new(SetGoalTool),
        Box::new(SuggestGoalsTool),
        Box::new(TrackProgressTool),
        Box::new(AnalyzeGoalFeasibilityTool),
    ]
}
