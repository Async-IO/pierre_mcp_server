// ABOUTME: Coach management tool handlers for MCP protocol (custom AI personas)
// ABOUTME: Implements tools for CRUD operations on user-created coaches with system prompts
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::database::coaches::{
    CoachCategory, CoachesManager, CreateCoachRequest, ListCoachesFilter, UpdateCoachRequest,
};
use crate::protocols::universal::{UniversalRequest, UniversalResponse, UniversalToolExecutor};
use crate::protocols::ProtocolError;
use crate::utils::uuid::parse_user_id_for_protocol;
use serde::Deserialize;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

use super::{apply_format_to_response, extract_output_format};

/// Input parameters for creating a coach
#[derive(Debug, Deserialize)]
struct CreateCoachParams {
    title: String,
    description: Option<String>,
    system_prompt: String,
    category: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

/// Get coaches manager from resources
fn get_coaches_manager(executor: &UniversalToolExecutor) -> Result<CoachesManager, ProtocolError> {
    let pool = executor.resources.database.sqlite_pool().ok_or_else(|| {
        ProtocolError::InternalError("SQLite database required for coaches".to_owned())
    })?;
    Ok(CoachesManager::new(pool.clone()))
}

/// Handle `list_coaches` tool - list user's coaches with optional filtering
///
/// # Parameters
/// - `category`: Filter by category (training, nutrition, recovery, recipes, custom)
/// - `favorites_only`: Return only favorited coaches (default: false)
/// - `limit`: Maximum results to return (default: 50, max: 100)
/// - `offset`: Pagination offset (default: 0)
/// - `format`: Output format ("json" or "toon")
///
/// # Returns
/// JSON array of coach summaries with metadata
#[must_use]
pub fn handle_list_coaches(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "list_coaches cancelled".to_owned(),
                ));
            }
        }

        let output_format = extract_output_format(&request);
        let user_id = parse_user_id_for_protocol(&request.user_id)?;
        let user_id_string = user_id.to_string();
        let tenant_id = request.tenant_id.as_deref().unwrap_or(&user_id_string);

        let category = request
            .parameters
            .get("category")
            .and_then(Value::as_str)
            .map(CoachCategory::parse);

        let favorites_only = request
            .parameters
            .get("favorites_only")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        #[allow(clippy::cast_possible_truncation)]
        let limit = request
            .parameters
            .get("limit")
            .and_then(Value::as_u64)
            .map(|v| v.min(100) as u32);

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let offset = request.parameters.get("offset").and_then(|v| {
            v.as_u64()
                .map(|n| n.min(u64::from(u32::MAX)) as u32)
                .or_else(|| v.as_f64().map(|f| f as u32))
        });

        let filter = ListCoachesFilter {
            category,
            favorites_only,
            limit,
            offset,
        };

        let manager = get_coaches_manager(executor)?;
        let coaches = manager
            .list(user_id, tenant_id, &filter)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Failed to list coaches: {e}")))?;

        let total = manager
            .count(user_id, tenant_id)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Failed to count coaches: {e}")))?;

        let coach_summaries: Vec<Value> = coaches
            .iter()
            .map(|c| {
                json!({
                    "id": c.id.to_string(),
                    "title": c.title,
                    "description": c.description,
                    "category": c.category.as_str(),
                    "tags": c.tags,
                    "token_count": c.token_count,
                    "is_favorite": c.is_favorite,
                    "use_count": c.use_count,
                    "last_used_at": c.last_used_at.map(|dt| dt.to_rfc3339()),
                    "updated_at": c.updated_at.to_rfc3339(),
                })
            })
            .collect();

        let returned_count = coach_summaries.len();
        #[allow(clippy::cast_possible_truncation)]
        let has_more = limit.is_some_and(|l| returned_count == l as usize);

        let result = UniversalResponse {
            success: true,
            result: Some(json!({
                "coaches": coach_summaries,
                "count": returned_count,
                "total": total,
                "offset": offset.unwrap_or(0),
                "limit": limit.unwrap_or(50),
                "has_more": has_more,
            })),
            error: None,
            metadata: None,
        };

        Ok(apply_format_to_response(result, "coaches", output_format))
    })
}

/// Handle `create_coach` tool - create a new custom coach
///
/// # Parameters
/// - `title`: Display title for the coach (required)
/// - `system_prompt`: System prompt that shapes AI responses (required)
/// - `description`: Optional description explaining the coach's purpose
/// - `category`: Category for organization (training, nutrition, recovery, recipes, custom)
/// - `tags`: Optional array of tags for filtering
///
/// # Returns
/// Created coach details including generated ID
#[must_use]
pub fn handle_create_coach(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "create_coach cancelled".to_owned(),
                ));
            }
        }

        let user_id = parse_user_id_for_protocol(&request.user_id)?;
        let user_id_string = user_id.to_string();
        let tenant_id = request.tenant_id.as_deref().unwrap_or(&user_id_string);

        let params: CreateCoachParams = serde_json::from_value(request.parameters.clone())
            .map_err(|e| ProtocolError::InvalidRequest(format!("Invalid coach parameters: {e}")))?;

        let create_request = CreateCoachRequest {
            title: params.title.clone(),
            description: params.description,
            system_prompt: params.system_prompt,
            category: params
                .category
                .as_deref()
                .map(CoachCategory::parse)
                .unwrap_or_default(),
            tags: params.tags,
        };

        let manager = get_coaches_manager(executor)?;
        let coach = manager
            .create(user_id, tenant_id, &create_request)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Failed to create coach: {e}")))?;

        Ok(UniversalResponse {
            success: true,
            result: Some(json!({
                "id": coach.id.to_string(),
                "title": coach.title,
                "description": coach.description,
                "category": coach.category.as_str(),
                "tags": coach.tags,
                "token_count": coach.token_count,
                "created_at": coach.created_at.to_rfc3339(),
            })),
            error: None,
            metadata: None,
        })
    })
}

/// Handle `get_coach` tool - get a specific coach by ID
///
/// # Parameters
/// - `coach_id`: UUID of the coach (required)
/// - `format`: Output format ("json" or "toon")
///
/// # Returns
/// Full coach details including system prompt
#[must_use]
pub fn handle_get_coach(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "get_coach cancelled".to_owned(),
                ));
            }
        }

        let output_format = extract_output_format(&request);
        let user_id = parse_user_id_for_protocol(&request.user_id)?;
        let user_id_string = user_id.to_string();
        let tenant_id = request.tenant_id.as_deref().unwrap_or(&user_id_string);

        let coach_id = request
            .parameters
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("Missing required parameter: coach_id".to_owned())
            })?;

        let manager = get_coaches_manager(executor)?;
        let coach = manager
            .get(coach_id, user_id, tenant_id)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Failed to get coach: {e}")))?;

        match coach {
            Some(c) => {
                let result = UniversalResponse {
                    success: true,
                    result: Some(json!({
                        "id": c.id.to_string(),
                        "title": c.title,
                        "description": c.description,
                        "system_prompt": c.system_prompt,
                        "category": c.category.as_str(),
                        "tags": c.tags,
                        "token_count": c.token_count,
                        "is_favorite": c.is_favorite,
                        "use_count": c.use_count,
                        "last_used_at": c.last_used_at.map(|dt| dt.to_rfc3339()),
                        "created_at": c.created_at.to_rfc3339(),
                        "updated_at": c.updated_at.to_rfc3339(),
                    })),
                    error: None,
                    metadata: None,
                };
                Ok(apply_format_to_response(result, "coach", output_format))
            }
            None => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Coach not found: {coach_id}")),
                metadata: None,
            }),
        }
    })
}

/// Handle `update_coach` tool - update an existing coach
///
/// # Parameters
/// - `coach_id`: UUID of the coach to update (required)
/// - `title`: New title (optional)
/// - `description`: New description (optional)
/// - `system_prompt`: New system prompt (optional)
/// - `category`: New category (optional)
/// - `tags`: New tags array (optional)
///
/// # Returns
/// Updated coach details
#[must_use]
pub fn handle_update_coach(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "update_coach cancelled".to_owned(),
                ));
            }
        }

        let user_id = parse_user_id_for_protocol(&request.user_id)?;
        let user_id_string = user_id.to_string();
        let tenant_id = request.tenant_id.as_deref().unwrap_or(&user_id_string);

        let coach_id = request
            .parameters
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("Missing required parameter: coach_id".to_owned())
            })?;

        // Extract update parameters manually to allow partial updates
        let update_request = UpdateCoachRequest {
            title: request
                .parameters
                .get("title")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            description: request
                .parameters
                .get("description")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            system_prompt: request
                .parameters
                .get("system_prompt")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            category: request
                .parameters
                .get("category")
                .and_then(Value::as_str)
                .map(CoachCategory::parse),
            tags: request
                .parameters
                .get("tags")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .map(ToOwned::to_owned)
                        .collect()
                }),
        };

        let manager = get_coaches_manager(executor)?;
        let coach = manager
            .update(coach_id, user_id, tenant_id, &update_request)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Failed to update coach: {e}")))?;

        match coach {
            Some(c) => Ok(UniversalResponse {
                success: true,
                result: Some(json!({
                    "id": c.id.to_string(),
                    "title": c.title,
                    "description": c.description,
                    "system_prompt": c.system_prompt,
                    "category": c.category.as_str(),
                    "tags": c.tags,
                    "token_count": c.token_count,
                    "is_favorite": c.is_favorite,
                    "updated_at": c.updated_at.to_rfc3339(),
                })),
                error: None,
                metadata: None,
            }),
            None => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Coach not found: {coach_id}")),
                metadata: None,
            }),
        }
    })
}

/// Handle `delete_coach` tool - delete a coach from user's collection
///
/// # Parameters
/// - `coach_id`: UUID of the coach to delete (required)
///
/// # Returns
/// Success confirmation
#[must_use]
pub fn handle_delete_coach(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "delete_coach cancelled".to_owned(),
                ));
            }
        }

        let user_id = parse_user_id_for_protocol(&request.user_id)?;
        let user_id_string = user_id.to_string();
        let tenant_id = request.tenant_id.as_deref().unwrap_or(&user_id_string);

        let coach_id = request
            .parameters
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("Missing required parameter: coach_id".to_owned())
            })?;

        let manager = get_coaches_manager(executor)?;
        let deleted = manager
            .delete(coach_id, user_id, tenant_id)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Failed to delete coach: {e}")))?;

        if deleted {
            Ok(UniversalResponse {
                success: true,
                result: Some(json!({
                    "deleted": true,
                    "coach_id": coach_id,
                })),
                error: None,
                metadata: None,
            })
        } else {
            Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Coach not found: {coach_id}")),
                metadata: None,
            })
        }
    })
}

/// Handle `toggle_coach_favorite` tool - toggle favorite status of a coach
///
/// # Parameters
/// - `coach_id`: UUID of the coach (required)
///
/// # Returns
/// New favorite status
#[must_use]
pub fn handle_toggle_coach_favorite(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "toggle_coach_favorite cancelled".to_owned(),
                ));
            }
        }

        let user_id = parse_user_id_for_protocol(&request.user_id)?;
        let user_id_string = user_id.to_string();
        let tenant_id = request.tenant_id.as_deref().unwrap_or(&user_id_string);

        let coach_id = request
            .parameters
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("Missing required parameter: coach_id".to_owned())
            })?;

        let manager = get_coaches_manager(executor)?;
        let is_favorite = manager
            .toggle_favorite(coach_id, user_id, tenant_id)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Failed to toggle favorite: {e}")))?;

        is_favorite.map_or_else(
            || {
                Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!("Coach not found: {coach_id}")),
                    metadata: None,
                })
            },
            |fav| {
                Ok(UniversalResponse {
                    success: true,
                    result: Some(json!({
                        "coach_id": coach_id,
                        "is_favorite": fav,
                    })),
                    error: None,
                    metadata: None,
                })
            },
        )
    })
}

/// Handle `search_coaches` tool - search coaches by query
///
/// # Parameters
/// - `query`: Search query string (required)
/// - `limit`: Maximum results (default: 20, max: 100)
/// - `format`: Output format ("json" or "toon")
///
/// # Returns
/// JSON array of matching coaches
#[must_use]
pub fn handle_search_coaches(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "search_coaches cancelled".to_owned(),
                ));
            }
        }

        let output_format = extract_output_format(&request);
        let user_id = parse_user_id_for_protocol(&request.user_id)?;
        let user_id_string = user_id.to_string();
        let tenant_id = request.tenant_id.as_deref().unwrap_or(&user_id_string);

        let query = request
            .parameters
            .get("query")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("Missing required parameter: query".to_owned())
            })?;

        #[allow(clippy::cast_possible_truncation)]
        let limit = request
            .parameters
            .get("limit")
            .and_then(Value::as_u64)
            .map(|v| v.min(100) as u32);

        let manager = get_coaches_manager(executor)?;
        let coaches = manager
            .search(user_id, tenant_id, query, limit)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Failed to search coaches: {e}")))?;

        let results: Vec<Value> = coaches
            .iter()
            .map(|c| {
                json!({
                    "id": c.id.to_string(),
                    "title": c.title,
                    "description": c.description,
                    "category": c.category.as_str(),
                    "tags": c.tags,
                    "token_count": c.token_count,
                    "is_favorite": c.is_favorite,
                })
            })
            .collect();

        let result = UniversalResponse {
            success: true,
            result: Some(json!({
                "query": query,
                "results": results,
                "count": results.len(),
            })),
            error: None,
            metadata: None,
        };

        Ok(apply_format_to_response(result, "results", output_format))
    })
}

/// Handle `activate_coach` tool - set a coach as the active coach for the session
///
/// Only one coach can be active at a time. Activating a coach automatically
/// deactivates any previously active coach.
///
/// # Parameters
/// - `coach_id`: UUID of the coach to activate (required)
///
/// # Returns
/// Activated coach details
#[must_use]
pub fn handle_activate_coach(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "activate_coach cancelled".to_owned(),
                ));
            }
        }

        let user_id = parse_user_id_for_protocol(&request.user_id)?;
        let user_id_string = user_id.to_string();
        let tenant_id = request.tenant_id.as_deref().unwrap_or(&user_id_string);

        let coach_id = request
            .parameters
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("Missing required parameter: coach_id".to_owned())
            })?;

        let manager = get_coaches_manager(executor)?;
        let coach = manager
            .activate_coach(coach_id, user_id, tenant_id)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Failed to activate coach: {e}")))?;

        match coach {
            Some(c) => Ok(UniversalResponse {
                success: true,
                result: Some(json!({
                    "id": c.id.to_string(),
                    "title": c.title,
                    "description": c.description,
                    "system_prompt": c.system_prompt,
                    "category": c.category.as_str(),
                    "is_active": true,
                    "token_count": c.token_count,
                })),
                error: None,
                metadata: None,
            }),
            None => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Coach not found: {coach_id}")),
                metadata: None,
            }),
        }
    })
}

/// Handle `deactivate_coach` tool - deactivate the currently active coach
///
/// # Returns
/// Success confirmation
#[must_use]
pub fn handle_deactivate_coach(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "deactivate_coach cancelled".to_owned(),
                ));
            }
        }

        let user_id = parse_user_id_for_protocol(&request.user_id)?;
        let user_id_string = user_id.to_string();
        let tenant_id = request.tenant_id.as_deref().unwrap_or(&user_id_string);

        let manager = get_coaches_manager(executor)?;
        let deactivated = manager
            .deactivate_coach(user_id, tenant_id)
            .await
            .map_err(|e| {
                ProtocolError::InternalError(format!("Failed to deactivate coach: {e}"))
            })?;

        Ok(UniversalResponse {
            success: true,
            result: Some(json!({
                "deactivated": deactivated,
            })),
            error: None,
            metadata: None,
        })
    })
}

/// Handle `get_active_coach` tool - get the currently active coach for the user
///
/// # Parameters
/// - `format`: Output format ("json" or "toon")
///
/// # Returns
/// Active coach details including system prompt, or null if no coach is active
#[must_use]
pub fn handle_get_active_coach(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "get_active_coach cancelled".to_owned(),
                ));
            }
        }

        let output_format = extract_output_format(&request);
        let user_id = parse_user_id_for_protocol(&request.user_id)?;
        let user_id_string = user_id.to_string();
        let tenant_id = request.tenant_id.as_deref().unwrap_or(&user_id_string);

        let manager = get_coaches_manager(executor)?;
        let coach = manager
            .get_active_coach(user_id, tenant_id)
            .await
            .map_err(|e| {
                ProtocolError::InternalError(format!("Failed to get active coach: {e}"))
            })?;

        match coach {
            Some(c) => {
                let result = UniversalResponse {
                    success: true,
                    result: Some(json!({
                        "active": true,
                        "coach": {
                            "id": c.id.to_string(),
                            "title": c.title,
                            "description": c.description,
                            "system_prompt": c.system_prompt,
                            "category": c.category.as_str(),
                            "tags": c.tags,
                            "token_count": c.token_count,
                            "is_favorite": c.is_favorite,
                            "use_count": c.use_count,
                            "last_used_at": c.last_used_at.map(|dt| dt.to_rfc3339()),
                        }
                    })),
                    error: None,
                    metadata: None,
                };
                Ok(apply_format_to_response(result, "coach", output_format))
            }
            None => Ok(UniversalResponse {
                success: true,
                result: Some(json!({
                    "active": false,
                    "coach": null,
                })),
                error: None,
                metadata: None,
            }),
        }
    })
}
