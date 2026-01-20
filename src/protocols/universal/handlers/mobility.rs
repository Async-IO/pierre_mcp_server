// ABOUTME: Mobility tool handlers for MCP protocol (stretching exercises and yoga poses)
// ABOUTME: Implements 6 tools: list_stretching_exercises, get_stretching_exercise, suggest_stretches_for_activity, list_yoga_poses, get_yoga_pose, suggest_yoga_sequence
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::database::mobility::{
    DifficultyLevel, ListStretchingFilter, ListYogaFilter, MobilityManager, StretchingCategory,
    YogaCategory, YogaPoseType,
};
use crate::protocols::universal::{UniversalRequest, UniversalResponse, UniversalToolExecutor};
use crate::protocols::ProtocolError;
use chrono::Utc;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

/// Handle `list_stretching_exercises` tool - list stretching exercises with optional filters
///
/// # Parameters
/// - `category`: Filter by category (static, dynamic, pnf, ballistic) (optional)
/// - `difficulty`: Filter by difficulty (beginner, intermediate, advanced) (optional)
/// - `muscle_group`: Filter by target muscle group (optional)
/// - `limit`: Maximum number of results (optional, default: 50)
/// - `offset`: Results offset for pagination (optional, default: 0)
///
/// # Returns
/// JSON object with array of stretching exercises and metadata
///
/// # Errors
/// Returns `ProtocolError` if database query fails
#[must_use]
pub fn handle_list_stretching_exercises(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "list_stretching_exercises cancelled".to_owned(),
                ));
            }
        }

        let category = request
            .parameters
            .get("category")
            .and_then(Value::as_str)
            .map(StretchingCategory::parse);

        let difficulty = request
            .parameters
            .get("difficulty")
            .and_then(Value::as_str)
            .map(DifficultyLevel::parse);

        let muscle_group = request
            .parameters
            .get("muscle_group")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

        // Safe: capped to 100, fits in u32
        #[allow(clippy::cast_possible_truncation)]
        let limit = request
            .parameters
            .get("limit")
            .and_then(Value::as_u64)
            .map(|l| l.min(100) as u32);

        // Safe: pagination offset, reasonable values fit in u32
        #[allow(clippy::cast_possible_truncation)]
        let offset = request
            .parameters
            .get("offset")
            .and_then(Value::as_u64)
            .map(|o| o as u32);

        let filter = ListStretchingFilter {
            category,
            difficulty,
            muscle_group,
            activity_type: None,
            limit,
            offset,
        };

        let pool = executor
            .resources
            .database
            .sqlite_pool()
            .ok_or_else(|| ProtocolError::InternalError("SQLite database required".to_owned()))?
            .clone();
        let manager = MobilityManager::new(pool);
        let exercises = manager
            .list_stretching_exercises(&filter)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Database error: {e}")))?;

        let exercises_json: Vec<Value> = exercises
            .iter()
            .map(|e| {
                json!({
                    "id": e.id,
                    "name": e.name,
                    "description": e.description,
                    "category": e.category.as_str(),
                    "difficulty": e.difficulty.as_str(),
                    "primary_muscles": e.primary_muscles,
                    "secondary_muscles": e.secondary_muscles,
                    "duration_seconds": e.duration_seconds,
                    "sets": e.sets,
                })
            })
            .collect();

        Ok(UniversalResponse {
            success: true,
            result: Some(json!({
                "exercises": exercises_json,
                "count": exercises_json.len(),
                "timestamp": Utc::now().to_rfc3339(),
            })),
            error: None,
            metadata: None,
        })
    })
}

/// Handle `get_stretching_exercise` tool - get a specific stretching exercise by ID
///
/// # Parameters
/// - `id`: Exercise ID (required)
///
/// # Returns
/// JSON object with full exercise details
///
/// # Errors
/// Returns `ProtocolError` if exercise not found or ID missing
#[must_use]
pub fn handle_get_stretching_exercise(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        let id = request
            .parameters
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| ProtocolError::InvalidParameters("id is required".to_owned()))?;

        let pool = executor
            .resources
            .database
            .sqlite_pool()
            .ok_or_else(|| ProtocolError::InternalError("SQLite database required".to_owned()))?
            .clone();
        let manager = MobilityManager::new(pool);
        let exercise_opt = manager
            .get_stretching_exercise(id)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Database error: {e}")))?;

        let Some(exercise) = exercise_opt else {
            return Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Stretching exercise not found: {id}")),
                metadata: None,
            });
        };

        Ok(UniversalResponse {
            success: true,
            result: Some(json!({
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
            })),
            error: None,
            metadata: None,
        })
    })
}

/// Handle `suggest_stretches_for_activity` tool - suggest stretches based on activity type
///
/// # Parameters
/// - `activity_type`: Type of activity (running, cycling, swimming, etc.) (required)
/// - `difficulty`: Preferred difficulty level (optional)
/// - `duration_minutes`: Available time for stretching (optional)
///
/// # Returns
/// JSON object with suggested stretching routine
///
/// # Errors
/// Returns `ProtocolError` if `activity_type` is missing
#[must_use]
pub fn handle_suggest_stretches_for_activity(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        let activity_type = request
            .parameters
            .get("activity_type")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProtocolError::InvalidParameters("activity_type is required".to_owned())
            })?;

        let difficulty = request
            .parameters
            .get("difficulty")
            .and_then(Value::as_str)
            .map(DifficultyLevel::parse);

        // Safe: duration in minutes is bounded to reasonable values (< 240 mins)
        #[allow(clippy::cast_possible_truncation)]
        let duration_minutes = request
            .parameters
            .get("duration_minutes")
            .and_then(Value::as_u64)
            .map(|d| d.min(240) as u32);

        let pool = executor
            .resources
            .database
            .sqlite_pool()
            .ok_or_else(|| ProtocolError::InternalError("SQLite database required".to_owned()))?
            .clone();
        let manager = MobilityManager::new(pool);

        // Get exercises recommended for this activity
        // Note: get_stretches_for_activity takes (activity_type, limit), difficulty filtering done post-query
        let all_exercises = manager
            .get_stretches_for_activity(activity_type, Some(20))
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Database error: {e}")))?;

        // Filter by difficulty if specified
        let exercises: Vec<_> = if let Some(ref target_difficulty) = difficulty {
            all_exercises
                .into_iter()
                .filter(|e| &e.difficulty == target_difficulty)
                .collect()
        } else {
            all_exercises
        };

        // Build a suggested routine based on duration
        let max_exercises = duration_minutes.map_or(6, |d| (d / 5).clamp(3, 12) as usize);
        let suggestions: Vec<Value> = exercises
            .iter()
            .take(max_exercises)
            .map(|e| {
                json!({
                    "id": e.id,
                    "name": e.name,
                    "category": e.category.as_str(),
                    "difficulty": e.difficulty.as_str(),
                    "duration_seconds": e.duration_seconds,
                    "sets": e.sets,
                    "primary_muscles": e.primary_muscles,
                    "instructions": e.instructions,
                })
            })
            .collect();

        let total_duration_seconds: u32 = exercises
            .iter()
            .take(max_exercises)
            .map(|e| e.duration_seconds * e.sets)
            .sum();

        Ok(UniversalResponse {
            success: true,
            result: Some(json!({
                "activity_type": activity_type,
                "exercises": suggestions,
                "count": suggestions.len(),
                "total_duration_seconds": total_duration_seconds,
                "suggested_at": Utc::now().to_rfc3339(),
            })),
            error: None,
            metadata: None,
        })
    })
}

/// Handle `list_yoga_poses` tool - list yoga poses with optional filters
///
/// # Parameters
/// - `category`: Filter by category (standing, seated, supine, prone, inversion, balance, twist) (optional)
/// - `difficulty`: Filter by difficulty (beginner, intermediate, advanced) (optional)
/// - `pose_type`: Filter by type (stretch, strength, balance, relaxation, breathing) (optional)
/// - `recovery_context`: Filter by recovery context (optional)
/// - `limit`: Maximum number of results (optional, default: 50)
/// - `offset`: Results offset for pagination (optional, default: 0)
///
/// # Returns
/// JSON object with array of yoga poses and metadata
///
/// # Errors
/// Returns `ProtocolError` if database query fails
#[must_use]
pub fn handle_list_yoga_poses(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "list_yoga_poses cancelled".to_owned(),
                ));
            }
        }

        let category = request
            .parameters
            .get("category")
            .and_then(Value::as_str)
            .map(YogaCategory::parse);

        let difficulty = request
            .parameters
            .get("difficulty")
            .and_then(Value::as_str)
            .map(DifficultyLevel::parse);

        let pose_type = request
            .parameters
            .get("pose_type")
            .and_then(Value::as_str)
            .map(YogaPoseType::parse);

        let recovery_context = request
            .parameters
            .get("recovery_context")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

        // Safe: capped to 100, fits in u32
        #[allow(clippy::cast_possible_truncation)]
        let limit = request
            .parameters
            .get("limit")
            .and_then(Value::as_u64)
            .map(|l| l.min(100) as u32);

        // Safe: pagination offset, reasonable values fit in u32
        #[allow(clippy::cast_possible_truncation)]
        let offset = request
            .parameters
            .get("offset")
            .and_then(Value::as_u64)
            .map(|o| o as u32);

        let filter = ListYogaFilter {
            category,
            difficulty,
            pose_type,
            muscle_group: None,
            activity_type: None,
            recovery_context,
            limit,
            offset,
        };

        let pool = executor
            .resources
            .database
            .sqlite_pool()
            .ok_or_else(|| ProtocolError::InternalError("SQLite database required".to_owned()))?
            .clone();
        let manager = MobilityManager::new(pool);
        let poses = manager
            .list_yoga_poses(&filter)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Database error: {e}")))?;

        let poses_json: Vec<Value> = poses
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
                })
            })
            .collect();

        Ok(UniversalResponse {
            success: true,
            result: Some(json!({
                "poses": poses_json,
                "count": poses_json.len(),
                "timestamp": Utc::now().to_rfc3339(),
            })),
            error: None,
            metadata: None,
        })
    })
}

/// Handle `get_yoga_pose` tool - get a specific yoga pose by ID
///
/// # Parameters
/// - `id`: Pose ID (required)
///
/// # Returns
/// JSON object with full pose details
///
/// # Errors
/// Returns `ProtocolError` if pose not found or ID missing
#[must_use]
pub fn handle_get_yoga_pose(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        let id = request
            .parameters
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| ProtocolError::InvalidParameters("id is required".to_owned()))?;

        let pool = executor
            .resources
            .database
            .sqlite_pool()
            .ok_or_else(|| ProtocolError::InternalError("SQLite database required".to_owned()))?
            .clone();
        let manager = MobilityManager::new(pool);
        let pose_opt = manager
            .get_yoga_pose(id)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Database error: {e}")))?;

        let Some(pose) = pose_opt else {
            return Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Yoga pose not found: {id}")),
                metadata: None,
            });
        };

        Ok(UniversalResponse {
            success: true,
            result: Some(json!({
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
            })),
            error: None,
            metadata: None,
        })
    })
}

/// Handle `suggest_yoga_sequence` tool - suggest a yoga sequence for recovery
///
/// # Parameters
/// - `purpose`: Purpose of the sequence (required)
/// - `duration_minutes`: Target duration in minutes (optional, default: 15)
/// - `difficulty`: Preferred difficulty level (optional)
///
/// # Returns
/// JSON object with suggested yoga sequence
///
/// # Errors
/// Returns `ProtocolError` if purpose is missing
#[must_use]
pub fn handle_suggest_yoga_sequence(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        let purpose = request
            .parameters
            .get("purpose")
            .and_then(Value::as_str)
            .ok_or_else(|| ProtocolError::InvalidParameters("purpose is required".to_owned()))?;

        // Safe: duration in minutes, default 15, reasonable values fit in u32
        #[allow(clippy::cast_possible_truncation)]
        let duration_minutes = request
            .parameters
            .get("duration_minutes")
            .and_then(Value::as_u64)
            .map_or(15_u32, |v| v.min(240) as u32);

        let difficulty = request
            .parameters
            .get("difficulty")
            .and_then(Value::as_str)
            .map(DifficultyLevel::parse);

        let pool = executor
            .resources
            .database
            .sqlite_pool()
            .ok_or_else(|| ProtocolError::InternalError("SQLite database required".to_owned()))?
            .clone();
        let manager = MobilityManager::new(pool);

        // Get poses for the recovery purpose
        let all_poses = manager
            .get_poses_for_recovery(purpose, Some(20))
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Database error: {e}")))?;

        // Filter by difficulty if specified
        let poses: Vec<_> = if let Some(ref target_difficulty) = difficulty {
            all_poses
                .into_iter()
                .filter(|p| &p.difficulty == target_difficulty)
                .collect()
        } else {
            all_poses
        };

        // Calculate how many poses can fit in the duration
        let target_seconds = duration_minutes * 60;
        let mut sequence: Vec<Value> = Vec::new();
        let mut total_seconds: u32 = 0;

        for pose in &poses {
            if total_seconds + pose.hold_duration_seconds > target_seconds {
                break;
            }

            sequence.push(json!({
                "order": sequence.len() + 1,
                "id": pose.id,
                "english_name": pose.english_name,
                "sanskrit_name": pose.sanskrit_name,
                "category": pose.category.as_str(),
                "difficulty": pose.difficulty.as_str(),
                "hold_duration_seconds": pose.hold_duration_seconds,
                "breath_guidance": pose.breath_guidance,
                "primary_muscles": pose.primary_muscles,
                "instructions": pose.instructions,
            }));

            total_seconds += pose.hold_duration_seconds;
        }

        Ok(UniversalResponse {
            success: true,
            result: Some(json!({
                "purpose": purpose,
                "sequence": sequence,
                "pose_count": sequence.len(),
                "total_duration_seconds": total_seconds,
                "target_duration_minutes": duration_minutes,
                "guidance": format!(
                    "This {} yoga sequence is designed for {}. Take your time with each pose and listen to your body.",
                    duration_minutes,
                    purpose.replace('_', " ")
                ),
                "suggested_at": Utc::now().to_rfc3339(),
            })),
            error: None,
            metadata: None,
        })
    })
}
