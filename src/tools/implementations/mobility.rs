// ABOUTME: Mobility tools for stretching exercises and yoga poses.
// ABOUTME: Implements list_stretching_exercises, get_stretching_exercise, suggest_stretches_for_activity, list_yoga_poses, get_yoga_pose, suggest_yoga_sequence.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Mobility Tools
//!
//! This module provides tools for mobility, stretching, and yoga:
//! - `ListStretchingExercisesTool` - List stretching exercises with filtering
//! - `GetStretchingExerciseTool` - Get a specific stretching exercise
//! - `SuggestStretchesForActivityTool` - Suggest stretches based on activity type
//! - `ListYogaPosesTool` - List yoga poses with filtering
//! - `GetYogaPoseTool` - Get a specific yoga pose
//! - `SuggestYogaSequenceTool` - Suggest a yoga sequence for recovery
//!
//! All tools use direct database access for seeded mobility data.

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::Utc;
use serde_json::{json, Value};

use crate::database::mobility::{
    DifficultyLevel, ListStretchingFilter, ListYogaFilter, MobilityManager, StretchingCategory,
    YogaCategory, YogaPoseType,
};
use crate::errors::{AppError, AppResult};
use crate::mcp::schema::{JsonSchema, PropertySchema};
use crate::tools::context::ToolExecutionContext;
use crate::tools::result::ToolResult;
use crate::tools::traits::{McpTool, ToolCapabilities};

// ============================================================================
// Helper functions
// ============================================================================

/// Get mobility manager from context
fn get_mobility_manager(ctx: &ToolExecutionContext) -> AppResult<MobilityManager> {
    let pool = ctx
        .resources
        .database
        .sqlite_pool()
        .ok_or_else(|| AppError::internal("Mobility tools require SQLite backend"))?;

    Ok(MobilityManager::new(pool.clone()))
}

/// Parse stretching category from string
fn parse_stretching_category(cat_str: &str) -> Option<StretchingCategory> {
    match cat_str.to_lowercase().as_str() {
        "static" => Some(StretchingCategory::Static),
        "dynamic" => Some(StretchingCategory::Dynamic),
        "pnf" => Some(StretchingCategory::Pnf),
        "ballistic" => Some(StretchingCategory::Ballistic),
        _ => None,
    }
}

/// Parse difficulty level from string
fn parse_difficulty(diff_str: &str) -> Option<DifficultyLevel> {
    match diff_str.to_lowercase().as_str() {
        "beginner" => Some(DifficultyLevel::Beginner),
        "intermediate" => Some(DifficultyLevel::Intermediate),
        "advanced" => Some(DifficultyLevel::Advanced),
        _ => None,
    }
}

/// Parse yoga category from string
fn parse_yoga_category(cat_str: &str) -> Option<YogaCategory> {
    match cat_str.to_lowercase().as_str() {
        "standing" => Some(YogaCategory::Standing),
        "seated" => Some(YogaCategory::Seated),
        "supine" => Some(YogaCategory::Supine),
        "prone" => Some(YogaCategory::Prone),
        "inversion" => Some(YogaCategory::Inversion),
        "balance" => Some(YogaCategory::Balance),
        "twist" => Some(YogaCategory::Twist),
        _ => None,
    }
}

/// Parse yoga pose type from string
fn parse_yoga_pose_type(type_str: &str) -> Option<YogaPoseType> {
    match type_str.to_lowercase().as_str() {
        "stretch" => Some(YogaPoseType::Stretch),
        "strength" => Some(YogaPoseType::Strength),
        "balance" => Some(YogaPoseType::Balance),
        "relaxation" => Some(YogaPoseType::Relaxation),
        "breathing" => Some(YogaPoseType::Breathing),
        _ => None,
    }
}

/// Convert difficulty level to numeric value for comparison
const fn difficulty_to_level(difficulty: DifficultyLevel) -> u8 {
    match difficulty {
        DifficultyLevel::Beginner => 1,
        DifficultyLevel::Intermediate => 2,
        DifficultyLevel::Advanced => 3,
    }
}

/// Categories to try including in a balanced yoga sequence
const YOGA_CATEGORY_ORDER: [YogaCategory; 5] = [
    YogaCategory::Standing,
    YogaCategory::Balance,
    YogaCategory::Seated,
    YogaCategory::Supine,
    YogaCategory::Twist,
];

// ============================================================================
// ListStretchingExercisesTool
// ============================================================================

/// Tool for listing stretching exercises with filtering.
pub struct ListStretchingExercisesTool;

#[async_trait]
impl McpTool for ListStretchingExercisesTool {
    fn name(&self) -> &'static str {
        "list_stretching_exercises"
    }

    fn description(&self) -> &'static str {
        "List stretching exercises with optional filtering by category, difficulty, muscle group, or activity type"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "category".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Filter by stretch category: static, dynamic, pnf, ballistic".to_owned(),
                ),
            },
        );
        properties.insert(
            "difficulty".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Filter by difficulty: beginner, intermediate, advanced".to_owned(),
                ),
            },
        );
        properties.insert(
            "muscle_group".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Filter by muscle group (e.g., hamstrings, quadriceps, calves)".to_owned(),
                ),
            },
        );
        properties.insert(
            "activity_type".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Filter by recommended activity (e.g., running, cycling, swimming)".to_owned(),
                ),
            },
        );
        properties.insert(
            "limit".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Maximum number of results to return (default: 20)".to_owned()),
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

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        tracing::debug!(user_id = %ctx.user_id, "Listing stretching exercises");

        let manager = get_mobility_manager(ctx)?;

        let filter = ListStretchingFilter {
            category: args
                .get("category")
                .and_then(Value::as_str)
                .and_then(parse_stretching_category),
            difficulty: args
                .get("difficulty")
                .and_then(Value::as_str)
                .and_then(parse_difficulty),
            muscle_group: args
                .get("muscle_group")
                .and_then(Value::as_str)
                .map(String::from),
            activity_type: args
                .get("activity_type")
                .and_then(Value::as_str)
                .map(String::from),
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            limit: args.get("limit").and_then(Value::as_u64).map(|l| l as u32),
            offset: None,
        };

        let exercises = manager.list_stretching_exercises(&filter).await?;

        let results: Vec<_> = exercises
            .iter()
            .map(|e| {
                json!({
                    "id": e.id,
                    "name": e.name,
                    "description": e.description,
                    "category": e.category.as_str(),
                    "difficulty": e.difficulty.as_str(),
                    "primary_muscles": e.primary_muscles,
                    "duration_seconds": e.duration_seconds,
                    "sets": e.sets,
                    "recommended_for_activities": e.recommended_for_activities,
                })
            })
            .collect();

        Ok(ToolResult::ok(json!({
            "exercises": results,
            "total_count": results.len(),
            "filters_applied": {
                "category": args.get("category"),
                "difficulty": args.get("difficulty"),
                "muscle_group": args.get("muscle_group"),
                "activity_type": args.get("activity_type"),
            },
            "retrieved_at": Utc::now().to_rfc3339(),
        })))
    }
}

// ============================================================================
// GetStretchingExerciseTool
// ============================================================================

/// Tool for getting a specific stretching exercise by ID.
pub struct GetStretchingExerciseTool;

#[async_trait]
impl McpTool for GetStretchingExerciseTool {
    fn name(&self) -> &'static str {
        "get_stretching_exercise"
    }

    fn description(&self) -> &'static str {
        "Get detailed information about a specific stretching exercise"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "exercise_id".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("The unique ID of the stretching exercise".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["exercise_id".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        tracing::debug!(user_id = %ctx.user_id, "Getting stretching exercise");

        let exercise_id = args
            .get("exercise_id")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("exercise_id is required"))?;

        let manager = get_mobility_manager(ctx)?;
        let exercise = manager.get_stretching_exercise(exercise_id).await?;

        let Some(exercise) = exercise else {
            return Ok(ToolResult::error(json!({
                "error": format!("Stretching exercise not found: {exercise_id}")
            })));
        };

        Ok(ToolResult::ok(json!({
            "id": exercise.id,
            "name": exercise.name,
            "description": exercise.description,
            "category": exercise.category.as_str(),
            "difficulty": exercise.difficulty.as_str(),
            "primary_muscles": exercise.primary_muscles,
            "secondary_muscles": exercise.secondary_muscles,
            "duration_seconds": exercise.duration_seconds,
            "repetitions": exercise.repetitions,
            "sets": exercise.sets,
            "recommended_for_activities": exercise.recommended_for_activities,
            "contraindications": exercise.contraindications,
            "instructions": exercise.instructions,
            "cues": exercise.cues,
            "image_url": exercise.image_url,
            "video_url": exercise.video_url,
            "retrieved_at": Utc::now().to_rfc3339(),
        })))
    }
}

// ============================================================================
// SuggestStretchesForActivityTool
// ============================================================================

/// Tool for suggesting stretches based on activity type.
pub struct SuggestStretchesForActivityTool;

#[async_trait]
impl McpTool for SuggestStretchesForActivityTool {
    fn name(&self) -> &'static str {
        "suggest_stretches_for_activity"
    }

    fn description(&self) -> &'static str {
        "Get personalized stretching recommendations based on your recent activity type"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "activity_type".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "The activity type to get stretches for (e.g., running, cycling, swimming, hiking)"
                        .to_owned(),
                ),
            },
        );
        properties.insert(
            "focus".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Optional focus: warmup (dynamic stretches) or cooldown (static stretches)"
                        .to_owned(),
                ),
            },
        );
        properties.insert(
            "limit".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Maximum number of stretches to suggest (default: 6)".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["activity_type".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        tracing::debug!(user_id = %ctx.user_id, "Suggesting stretches for activity");

        let activity_type = args
            .get("activity_type")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("activity_type is required"))?;

        let focus = args.get("focus").and_then(Value::as_str);

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .map_or(6_u32, |l| l as u32);

        let manager = get_mobility_manager(ctx)?;

        // Get activity-muscle mapping for context
        let mapping = manager.get_activity_muscle_mapping(activity_type).await?;

        // Get stretches recommended for this activity
        let mut stretches = manager
            .get_stretches_for_activity(activity_type, Some(limit * 2))
            .await?;

        // Filter by focus (warmup = dynamic, cooldown = static)
        if let Some(focus_str) = focus {
            let target_category = match focus_str.to_lowercase().as_str() {
                "warmup" | "warm_up" | "warm-up" => Some(StretchingCategory::Dynamic),
                "cooldown" | "cool_down" | "cool-down" => Some(StretchingCategory::Static),
                _ => None,
            };

            if let Some(cat) = target_category {
                stretches.retain(|s| s.category == cat);
            }
        }

        // Limit results
        stretches.truncate(limit as usize);

        let results: Vec<_> = stretches
            .iter()
            .map(|e| {
                json!({
                    "id": e.id,
                    "name": e.name,
                    "description": e.description,
                    "category": e.category.as_str(),
                    "difficulty": e.difficulty.as_str(),
                    "primary_muscles": e.primary_muscles,
                    "duration_seconds": e.duration_seconds,
                    "sets": e.sets,
                    "instructions": e.instructions,
                    "cues": e.cues,
                })
            })
            .collect();

        let muscle_context = mapping.as_ref().map(|m| {
            json!({
                "primary_muscles_stressed": m.primary_muscles,
                "secondary_muscles_stressed": m.secondary_muscles,
            })
        });

        Ok(ToolResult::ok(json!({
            "activity_type": activity_type,
            "focus": focus,
            "suggested_stretches": results,
            "total_suggestions": results.len(),
            "muscle_context": muscle_context,
            "recommendation": format!(
                "After {}, focus on stretching: {}",
                activity_type,
                stretches.iter()
                    .flat_map(|s| s.primary_muscles.iter())
                    .take(5)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            "suggested_at": Utc::now().to_rfc3339(),
        })))
    }
}

// ============================================================================
// ListYogaPosesTool
// ============================================================================

/// Tool for listing yoga poses with filtering.
pub struct ListYogaPosesTool;

#[async_trait]
impl McpTool for ListYogaPosesTool {
    fn name(&self) -> &'static str {
        "list_yoga_poses"
    }

    fn description(&self) -> &'static str {
        "List yoga poses with optional filtering by category, difficulty, pose type, or recovery context"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "category".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Filter by pose category: standing, seated, supine, prone, inversion, balance, twist"
                        .to_owned(),
                ),
            },
        );
        properties.insert(
            "difficulty".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Filter by difficulty: beginner, intermediate, advanced".to_owned(),
                ),
            },
        );
        properties.insert(
            "pose_type".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Filter by pose type: stretch, strength, balance, relaxation, breathing"
                        .to_owned(),
                ),
            },
        );
        properties.insert(
            "muscle_group".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Filter by muscle group (e.g., hamstrings, hips, shoulders)".to_owned(),
                ),
            },
        );
        properties.insert(
            "recovery_context".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Filter by recovery context: post_cardio, rest_day, morning, evening"
                        .to_owned(),
                ),
            },
        );
        properties.insert(
            "limit".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Maximum number of results to return (default: 20)".to_owned()),
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

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        tracing::debug!(user_id = %ctx.user_id, "Listing yoga poses");

        let manager = get_mobility_manager(ctx)?;

        let filter = ListYogaFilter {
            category: args
                .get("category")
                .and_then(Value::as_str)
                .and_then(parse_yoga_category),
            difficulty: args
                .get("difficulty")
                .and_then(Value::as_str)
                .and_then(parse_difficulty),
            pose_type: args
                .get("pose_type")
                .and_then(Value::as_str)
                .and_then(parse_yoga_pose_type),
            muscle_group: args
                .get("muscle_group")
                .and_then(Value::as_str)
                .map(String::from),
            activity_type: args
                .get("activity_type")
                .and_then(Value::as_str)
                .map(String::from),
            recovery_context: args
                .get("recovery_context")
                .and_then(Value::as_str)
                .map(String::from),
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            limit: args.get("limit").and_then(Value::as_u64).map(|l| l as u32),
            offset: None,
        };

        let poses = manager.list_yoga_poses(&filter).await?;

        let results: Vec<_> = poses
            .iter()
            .map(|p| {
                json!({
                    "id": p.id,
                    "english_name": p.english_name,
                    "sanskrit_name": p.sanskrit_name,
                    "description": p.description,
                    "category": p.category.as_str(),
                    "difficulty": p.difficulty.as_str(),
                    "pose_type": p.pose_type.as_str(),
                    "primary_muscles": p.primary_muscles,
                    "hold_duration_seconds": p.hold_duration_seconds,
                    "benefits": p.benefits,
                })
            })
            .collect();

        Ok(ToolResult::ok(json!({
            "poses": results,
            "total_count": results.len(),
            "filters_applied": {
                "category": args.get("category"),
                "difficulty": args.get("difficulty"),
                "pose_type": args.get("pose_type"),
                "muscle_group": args.get("muscle_group"),
                "recovery_context": args.get("recovery_context"),
            },
            "retrieved_at": Utc::now().to_rfc3339(),
        })))
    }
}

// ============================================================================
// GetYogaPoseTool
// ============================================================================

/// Tool for getting a specific yoga pose by ID.
pub struct GetYogaPoseTool;

#[async_trait]
impl McpTool for GetYogaPoseTool {
    fn name(&self) -> &'static str {
        "get_yoga_pose"
    }

    fn description(&self) -> &'static str {
        "Get detailed information about a specific yoga pose"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "pose_id".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("The unique ID of the yoga pose".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["pose_id".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        tracing::debug!(user_id = %ctx.user_id, "Getting yoga pose");

        let pose_id = args
            .get("pose_id")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("pose_id is required"))?;

        let manager = get_mobility_manager(ctx)?;
        let pose = manager.get_yoga_pose(pose_id).await?;

        let Some(pose) = pose else {
            return Ok(ToolResult::error(json!({
                "error": format!("Yoga pose not found: {pose_id}")
            })));
        };

        Ok(ToolResult::ok(json!({
            "id": pose.id,
            "english_name": pose.english_name,
            "sanskrit_name": pose.sanskrit_name,
            "description": pose.description,
            "benefits": pose.benefits,
            "category": pose.category.as_str(),
            "difficulty": pose.difficulty.as_str(),
            "pose_type": pose.pose_type.as_str(),
            "primary_muscles": pose.primary_muscles,
            "secondary_muscles": pose.secondary_muscles,
            "chakras": pose.chakras,
            "hold_duration_seconds": pose.hold_duration_seconds,
            "breath_guidance": pose.breath_guidance,
            "recommended_for_activities": pose.recommended_for_activities,
            "recommended_for_recovery": pose.recommended_for_recovery,
            "contraindications": pose.contraindications,
            "instructions": pose.instructions,
            "modifications": pose.modifications,
            "progressions": pose.progressions,
            "cues": pose.cues,
            "warmup_poses": pose.warmup_poses,
            "followup_poses": pose.followup_poses,
            "image_url": pose.image_url,
            "video_url": pose.video_url,
            "retrieved_at": Utc::now().to_rfc3339(),
        })))
    }
}

// ============================================================================
// SuggestYogaSequenceTool
// ============================================================================

/// Tool for suggesting a yoga sequence for recovery.
pub struct SuggestYogaSequenceTool;

#[async_trait]
impl McpTool for SuggestYogaSequenceTool {
    fn name(&self) -> &'static str {
        "suggest_yoga_sequence"
    }

    fn description(&self) -> &'static str {
        "Create a personalized yoga sequence for recovery based on your recent activities or goals"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "purpose".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Purpose of the sequence: post_cardio, rest_day, morning, evening, stress_relief"
                        .to_owned(),
                ),
            },
        );
        properties.insert(
            "duration_minutes".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some(
                    "Target duration in minutes (10, 15, 20, 30). Default: 15".to_owned(),
                ),
            },
        );
        properties.insert(
            "difficulty".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Maximum difficulty level: beginner, intermediate, advanced".to_owned(),
                ),
            },
        );
        properties.insert(
            "focus_area".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Optional muscle/area focus: hips, hamstrings, back, shoulders".to_owned(),
                ),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["purpose".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        tracing::debug!(user_id = %ctx.user_id, "Suggesting yoga sequence");

        let purpose = args
            .get("purpose")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("purpose is required"))?;

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let duration_minutes = args
            .get("duration_minutes")
            .and_then(Value::as_u64)
            .map_or(15_u32, |d| d as u32);

        let max_difficulty = args
            .get("difficulty")
            .and_then(Value::as_str)
            .and_then(parse_difficulty)
            .unwrap_or(DifficultyLevel::Intermediate);

        let focus_area = args.get("focus_area").and_then(Value::as_str);

        let manager = get_mobility_manager(ctx)?;

        // Get poses recommended for this recovery context
        let mut all_poses = manager.get_poses_for_recovery(purpose, Some(30)).await?;

        // Filter by difficulty - pose difficulty must not exceed max_difficulty
        let max_level = difficulty_to_level(max_difficulty);
        all_poses.retain(|p| difficulty_to_level(p.difficulty) <= max_level);

        // If focus area specified, prioritize those poses
        if let Some(focus) = focus_area {
            all_poses.sort_by(|a, b| {
                let a_has_focus = a
                    .primary_muscles
                    .iter()
                    .any(|m| m.to_lowercase().contains(&focus.to_lowercase()));
                let b_has_focus = b
                    .primary_muscles
                    .iter()
                    .any(|m| m.to_lowercase().contains(&focus.to_lowercase()));
                b_has_focus.cmp(&a_has_focus)
            });
        }

        // Calculate how many poses to include based on duration
        // Assume average hold time of 45 seconds + 15 seconds transition
        let poses_count = duration_minutes.clamp(3, 12);

        // Build a balanced sequence with variety of categories
        let (mut sequence, mut total_time) = (Vec::new(), 0_u32);
        let target_time = duration_minutes * 60;

        for cat in YOGA_CATEGORY_ORDER {
            if sequence.len() >= poses_count as usize || total_time >= target_time {
                break;
            }
            if let Some(pose) = all_poses.iter().find(|p| p.category == cat) {
                total_time += pose.hold_duration_seconds;
                sequence.push(pose.clone());
            }
        }

        // Fill remaining slots with any suitable poses
        for pose in &all_poses {
            if sequence.len() >= poses_count as usize || total_time >= target_time {
                break;
            }

            if !sequence.iter().any(|p| p.id == pose.id) {
                sequence.push(pose.clone());
                total_time += pose.hold_duration_seconds;
            }
        }

        // Always end with relaxation if available
        if let Some(savasana) = all_poses
            .iter()
            .find(|p| p.pose_type == YogaPoseType::Relaxation)
        {
            if !sequence.iter().any(|p| p.id == savasana.id) {
                sequence.push(savasana.clone());
            }
        }

        let result_poses: Vec<_> = sequence
            .iter()
            .enumerate()
            .map(|(i, p)| {
                json!({
                    "order": i + 1,
                    "id": p.id,
                    "english_name": p.english_name,
                    "sanskrit_name": p.sanskrit_name,
                    "category": p.category.as_str(),
                    "hold_duration_seconds": p.hold_duration_seconds,
                    "breath_guidance": p.breath_guidance,
                    "instructions": p.instructions,
                    "modifications": p.modifications,
                    "cues": p.cues,
                })
            })
            .collect();

        let actual_duration: u32 = sequence.iter().map(|p| p.hold_duration_seconds).sum();

        Ok(ToolResult::ok(json!({
            "purpose": purpose,
            "requested_duration_minutes": duration_minutes,
            "actual_duration_seconds": actual_duration,
            "actual_duration_minutes": actual_duration / 60,
            "difficulty": max_difficulty.as_str(),
            "focus_area": focus_area,
            "sequence": result_poses,
            "pose_count": result_poses.len(),
            "guidance": format!(
                "This {} yoga sequence is designed for {}. Take your time with each pose and listen to your body.",
                duration_minutes,
                purpose.replace('_', " ")
            ),
            "suggested_at": Utc::now().to_rfc3339(),
        })))
    }
}

// ============================================================================
// Module exports
// ============================================================================

/// Create all mobility tools for registration
#[must_use]
pub fn create_mobility_tools() -> Vec<Box<dyn McpTool>> {
    vec![
        Box::new(ListStretchingExercisesTool),
        Box::new(GetStretchingExerciseTool),
        Box::new(SuggestStretchesForActivityTool),
        Box::new(ListYogaPosesTool),
        Box::new(GetYogaPoseTool),
        Box::new(SuggestYogaSequenceTool),
    ]
}
