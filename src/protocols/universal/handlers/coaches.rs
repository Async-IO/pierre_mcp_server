// ABOUTME: Coach management tool handlers for MCP protocol (custom AI personas)
// ABOUTME: Implements tools for CRUD operations on user-created and system coaches
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::database::coaches::{
    CoachCategory, CoachVisibility, CoachesManager, CreateCoachRequest, CreateSystemCoachRequest,
    ListCoachesFilter, UpdateCoachRequest,
};
use crate::database_plugins::DatabaseProvider;
use crate::models::TenantId;
use crate::permissions::UserRole;
use crate::protocols::universal::{UniversalRequest, UniversalResponse, UniversalToolExecutor};
use crate::protocols::ProtocolError;
use crate::utils::uuid::parse_user_id_for_protocol;
use serde::Deserialize;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

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
    #[serde(default)]
    sample_prompts: Vec<String>,
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
        let tenant_id: TenantId = request
            .tenant_id
            .as_deref()
            .unwrap_or(&user_id_string)
            .parse()
            .map_err(|_| ProtocolError::InvalidRequest("Invalid tenant_id format".to_owned()))?;

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

        let include_system = request
            .parameters
            .get("include_system")
            .and_then(Value::as_bool)
            .unwrap_or(true);

        let include_hidden = request
            .parameters
            .get("include_hidden")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let filter = ListCoachesFilter {
            category,
            favorites_only,
            limit,
            offset,
            include_system,
            include_hidden,
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
            .map(|item| {
                json!({
                    "id": item.coach.id.to_string(),
                    "title": item.coach.title,
                    "description": item.coach.description,
                    "category": item.coach.category.as_str(),
                    "tags": item.coach.tags,
                    "token_count": item.coach.token_count,
                    "is_favorite": item.coach.is_favorite,
                    "is_system": item.coach.is_system,
                    "is_assigned": item.is_assigned,
                    "use_count": item.coach.use_count,
                    "last_used_at": item.coach.last_used_at.map(|dt| dt.to_rfc3339()),
                    "updated_at": item.coach.updated_at.to_rfc3339(),
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
        let tenant_id: TenantId = request
            .tenant_id
            .as_deref()
            .unwrap_or(&user_id_string)
            .parse()
            .map_err(|_| ProtocolError::InvalidRequest("Invalid tenant_id format".to_owned()))?;

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
            sample_prompts: params.sample_prompts,
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
        let tenant_id: TenantId = request
            .tenant_id
            .as_deref()
            .unwrap_or(&user_id_string)
            .parse()
            .map_err(|_| ProtocolError::InvalidRequest("Invalid tenant_id format".to_owned()))?;

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
        let tenant_id: TenantId = request
            .tenant_id
            .as_deref()
            .unwrap_or(&user_id_string)
            .parse()
            .map_err(|_| ProtocolError::InvalidRequest("Invalid tenant_id format".to_owned()))?;

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
            sample_prompts: request
                .parameters
                .get("sample_prompts")
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
        let tenant_id: TenantId = request
            .tenant_id
            .as_deref()
            .unwrap_or(&user_id_string)
            .parse()
            .map_err(|_| ProtocolError::InvalidRequest("Invalid tenant_id format".to_owned()))?;

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
        let tenant_id: TenantId = request
            .tenant_id
            .as_deref()
            .unwrap_or(&user_id_string)
            .parse()
            .map_err(|_| ProtocolError::InvalidRequest("Invalid tenant_id format".to_owned()))?;

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
        let tenant_id: TenantId = request
            .tenant_id
            .as_deref()
            .unwrap_or(&user_id_string)
            .parse()
            .map_err(|_| ProtocolError::InvalidRequest("Invalid tenant_id format".to_owned()))?;

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

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let offset = request.parameters.get("offset").and_then(|v| {
            v.as_u64()
                .map(|n| n.min(u64::from(u32::MAX)) as u32)
                .or_else(|| v.as_f64().map(|f| f as u32))
        });

        let manager = get_coaches_manager(executor)?;
        let coaches = manager
            .search(user_id, tenant_id, query, limit, offset)
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

        let returned_count = results.len();
        let limit_val = limit.unwrap_or(20);
        #[allow(clippy::cast_possible_truncation)]
        let has_more = returned_count == limit_val as usize;

        let result = UniversalResponse {
            success: true,
            result: Some(json!({
                "query": query,
                "results": results,
                "returned_count": returned_count,
                "offset": offset.unwrap_or(0),
                "limit": limit_val,
                "has_more": has_more,
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
        let tenant_id: TenantId = request
            .tenant_id
            .as_deref()
            .unwrap_or(&user_id_string)
            .parse()
            .map_err(|_| ProtocolError::InvalidRequest("Invalid tenant_id format".to_owned()))?;

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
        let tenant_id: TenantId = request
            .tenant_id
            .as_deref()
            .unwrap_or(&user_id_string)
            .parse()
            .map_err(|_| ProtocolError::InvalidRequest("Invalid tenant_id format".to_owned()))?;

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
        let tenant_id: TenantId = request
            .tenant_id
            .as_deref()
            .unwrap_or(&user_id_string)
            .parse()
            .map_err(|_| ProtocolError::InvalidRequest("Invalid tenant_id format".to_owned()))?;

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

/// Handle `hide_coach` tool - hide a system or assigned coach from user's view
///
/// Users can only hide system coaches or coaches assigned to them by admins.
/// Personal coaches cannot be hidden.
///
/// # Parameters
/// - `coach_id`: UUID of the coach to hide (required)
///
/// # Returns
/// Success confirmation with hidden status
#[must_use]
pub fn handle_hide_coach(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "hide_coach cancelled".to_owned(),
                ));
            }
        }

        let user_id = parse_user_id_for_protocol(&request.user_id)?;

        let coach_id = request
            .parameters
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("Missing required parameter: coach_id".to_owned())
            })?;

        let manager = get_coaches_manager(executor)?;
        let success = manager
            .hide_coach(coach_id, user_id)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Failed to hide coach: {e}")))?;

        Ok(UniversalResponse {
            success,
            result: Some(json!({
                "coach_id": coach_id,
                "is_hidden": success,
            })),
            error: if success {
                None
            } else {
                Some(
                    "Coach cannot be hidden (only system or assigned coaches can be hidden)"
                        .to_owned(),
                )
            },
            metadata: None,
        })
    })
}

/// Handle `show_coach` tool - unhide a previously hidden coach
///
/// # Parameters
/// - `coach_id`: UUID of the coach to show (required)
///
/// # Returns
/// Success confirmation
#[must_use]
pub fn handle_show_coach(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "show_coach cancelled".to_owned(),
                ));
            }
        }

        let user_id = parse_user_id_for_protocol(&request.user_id)?;

        let coach_id = request
            .parameters
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("Missing required parameter: coach_id".to_owned())
            })?;

        let manager = get_coaches_manager(executor)?;
        let success = manager
            .show_coach(coach_id, user_id)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Failed to show coach: {e}")))?;

        Ok(UniversalResponse {
            success: true,
            result: Some(json!({
                "coach_id": coach_id,
                "is_hidden": false,
                "removed_preference": success,
            })),
            error: None,
            metadata: None,
        })
    })
}

/// Handle `list_hidden_coaches` tool - list coaches the user has hidden
///
/// # Parameters
/// - `format`: Output format ("json" or "toon")
///
/// # Returns
/// JSON array of hidden coaches
#[must_use]
pub fn handle_list_hidden_coaches(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "list_hidden_coaches cancelled".to_owned(),
                ));
            }
        }

        let output_format = extract_output_format(&request);
        let user_id = parse_user_id_for_protocol(&request.user_id)?;
        let user_id_string = user_id.to_string();
        let tenant_id: TenantId = request
            .tenant_id
            .as_deref()
            .unwrap_or(&user_id_string)
            .parse()
            .map_err(|_| ProtocolError::InvalidRequest("Invalid tenant_id format".to_owned()))?;

        let manager = get_coaches_manager(executor)?;
        let coaches = manager
            .list_hidden_coaches(user_id, tenant_id)
            .await
            .map_err(|e| {
                ProtocolError::InternalError(format!("Failed to list hidden coaches: {e}"))
            })?;

        let coach_summaries: Vec<Value> = coaches
            .iter()
            .map(|c| {
                json!({
                    "id": c.id.to_string(),
                    "title": c.title,
                    "description": c.description,
                    "category": c.category.as_str(),
                    "is_system": c.is_system,
                })
            })
            .collect();

        let count = coach_summaries.len();
        let result = UniversalResponse {
            success: true,
            result: Some(json!({
                "coaches": coach_summaries,
                "count": count,
            })),
            error: None,
            metadata: None,
        };

        Ok(apply_format_to_response(result, "coaches", output_format))
    })
}

// ============================================================================
// Admin Coach Management Handlers (System Coaches - Admin Only)
// ============================================================================

/// Verify that a target user belongs to a given tenant.
///
/// Prevents cross-tenant operations by checking tenant membership.
async fn verify_user_tenant_membership(
    executor: &UniversalToolExecutor,
    target_user_id: Uuid,
    tenant_id: TenantId,
) -> Result<(), ProtocolError> {
    let user_tenants = executor
        .resources
        .database
        .list_tenants_for_user(target_user_id)
        .await
        .map_err(|e| {
            ProtocolError::InternalError(format!(
                "Failed to verify tenant membership for user {target_user_id}: {e}"
            ))
        })?;

    if !user_tenants.iter().any(|t| t.id == tenant_id) {
        return Err(ProtocolError::InvalidRequest(format!(
            "User {target_user_id} does not belong to this tenant"
        )));
    }

    Ok(())
}

/// Verify admin access for a user
///
/// Returns the `tenant_id` if authorized, error if not `Admin`/`SuperAdmin`.
/// Uses `active_tenant_id` from request when available (user's selected tenant),
/// falling back to user's first tenant for clients without `active_tenant_id`.
async fn verify_admin_access(
    executor: &UniversalToolExecutor,
    user_uuid: Uuid,
    active_tenant_id: Option<&str>,
) -> Result<TenantId, ProtocolError> {
    let user = executor
        .resources
        .database
        .get_user(user_uuid)
        .await
        .map_err(|e| ProtocolError::InternalError(format!("Failed to get user: {e}")))?
        .ok_or_else(|| ProtocolError::InvalidRequest(format!("User {user_uuid} not found")))?;

    // Check admin role
    if !matches!(user.role, UserRole::Admin | UserRole::SuperAdmin) {
        return Err(ProtocolError::InvalidRequest(
            "Permission denied: Admin access required".to_owned(),
        ));
    }

    // Prefer active_tenant_id from request (user's selected tenant)
    if let Some(tid_str) = active_tenant_id {
        if let Ok(requested_tid) = tid_str.parse::<TenantId>() {
            // Verify user is a member of this tenant
            let tenants = executor
                .resources
                .database
                .list_tenants_for_user(user_uuid)
                .await
                .map_err(|e| {
                    ProtocolError::InternalError(format!("Failed to get user tenants: {e}"))
                })?;
            if tenants.iter().any(|t| t.id == requested_tid) {
                return Ok(requested_tid);
            }
        }
        // Fall through if user is not a member or invalid tenant_id (use default tenant)
    }

    // Fall back to user's first tenant (single-tenant users or tokens without active_tenant_id)
    let tenants = executor
        .resources
        .database
        .list_tenants_for_user(user_uuid)
        .await
        .map_err(|e| ProtocolError::InternalError(format!("Failed to get user tenants: {e}")))?;

    tenants.first().map(|t| t.id).ok_or_else(|| {
        ProtocolError::InvalidRequest("User not associated with a tenant".to_owned())
    })
}

/// Handle `admin_list_system_coaches` tool - list system coaches for the tenant
///
/// Admin only. Lists all system coaches (`is_system=true`) visible to the tenant.
///
/// # Parameters
/// - `visibility`: Filter by visibility level (tenant, global) - optional
/// - `limit`: Maximum results to return (default: 50, max: 100)
/// - `offset`: Pagination offset (default: 0)
/// - `format`: Output format ("json" or "toon")
///
/// # Returns
/// JSON array of system coach summaries
#[must_use]
pub fn handle_admin_list_system_coaches(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "admin_list_system_coaches cancelled".to_owned(),
                ));
            }
        }

        let output_format = extract_output_format(&request);
        let user_id = parse_user_id_for_protocol(&request.user_id)?;
        let tenant_id =
            verify_admin_access(executor, user_id, request.tenant_id.as_deref()).await?;

        let manager = get_coaches_manager(executor)?;
        let coaches = manager.list_system_coaches(tenant_id).await.map_err(|e| {
            ProtocolError::InternalError(format!("Failed to list system coaches: {e}"))
        })?;

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
                    "visibility": c.visibility.as_str(),
                    "created_at": c.created_at.to_rfc3339(),
                    "updated_at": c.updated_at.to_rfc3339(),
                })
            })
            .collect();

        let count = coach_summaries.len();
        let result = UniversalResponse {
            success: true,
            result: Some(json!({
                "coaches": coach_summaries,
                "count": count,
            })),
            error: None,
            metadata: None,
        };

        Ok(apply_format_to_response(result, "coaches", output_format))
    })
}

/// Input parameters for creating a system coach
#[derive(Debug, Deserialize)]
struct CreateSystemCoachParams {
    title: String,
    description: Option<String>,
    system_prompt: String,
    category: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    sample_prompts: Vec<String>,
    visibility: Option<String>,
}

/// Handle `admin_create_system_coach` tool - create a new system coach
///
/// Admin only. Creates a system coach visible to tenant users.
///
/// # Parameters
/// - `title`: Display title for the coach (required)
/// - `system_prompt`: System prompt that shapes AI responses (required)
/// - `description`: Optional description explaining the coach's purpose
/// - `category`: Category for organization (training, nutrition, recovery, recipes, custom)
/// - `tags`: Optional array of tags for filtering
/// - `visibility`: "tenant" (default) or "global" (super-admin only)
///
/// # Returns
/// Created system coach details including generated ID
#[must_use]
pub fn handle_admin_create_system_coach(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "admin_create_system_coach cancelled".to_owned(),
                ));
            }
        }

        let user_id = parse_user_id_for_protocol(&request.user_id)?;
        let tenant_id =
            verify_admin_access(executor, user_id, request.tenant_id.as_deref()).await?;

        let params: CreateSystemCoachParams = serde_json::from_value(request.parameters.clone())
            .map_err(|e| {
                ProtocolError::InvalidRequest(format!("Invalid system coach parameters: {e}"))
            })?;

        let visibility = params
            .visibility
            .as_deref()
            .map_or(CoachVisibility::Tenant, CoachVisibility::parse);

        let create_request = CreateSystemCoachRequest {
            title: params.title.clone(),
            description: params.description,
            system_prompt: params.system_prompt,
            category: params
                .category
                .as_deref()
                .map(CoachCategory::parse)
                .unwrap_or_default(),
            tags: params.tags,
            sample_prompts: params.sample_prompts,
            visibility,
        };

        let manager = get_coaches_manager(executor)?;
        let coach = manager
            .create_system_coach(user_id, tenant_id, &create_request)
            .await
            .map_err(|e| {
                ProtocolError::InternalError(format!("Failed to create system coach: {e}"))
            })?;

        Ok(UniversalResponse {
            success: true,
            result: Some(json!({
                "id": coach.id.to_string(),
                "title": coach.title,
                "description": coach.description,
                "category": coach.category.as_str(),
                "tags": coach.tags,
                "token_count": coach.token_count,
                "visibility": coach.visibility.as_str(),
                "is_system": coach.is_system,
                "created_at": coach.created_at.to_rfc3339(),
            })),
            error: None,
            metadata: None,
        })
    })
}

/// Handle `admin_get_system_coach` tool - get a specific system coach by ID
///
/// Admin only.
///
/// # Parameters
/// - `coach_id`: UUID of the system coach (required)
/// - `format`: Output format ("json" or "toon")
///
/// # Returns
/// Full system coach details including system prompt
#[must_use]
pub fn handle_admin_get_system_coach(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "admin_get_system_coach cancelled".to_owned(),
                ));
            }
        }

        let output_format = extract_output_format(&request);
        let user_id = parse_user_id_for_protocol(&request.user_id)?;
        let tenant_id =
            verify_admin_access(executor, user_id, request.tenant_id.as_deref()).await?;

        let coach_id = request
            .parameters
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("Missing required parameter: coach_id".to_owned())
            })?;

        let manager = get_coaches_manager(executor)?;
        let coach = manager
            .get_system_coach(coach_id, tenant_id)
            .await
            .map_err(|e| {
                ProtocolError::InternalError(format!("Failed to get system coach: {e}"))
            })?;

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
                        "visibility": c.visibility.as_str(),
                        "is_system": c.is_system,
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
                error: Some(format!("System coach not found: {coach_id}")),
                metadata: None,
            }),
        }
    })
}

/// Handle `admin_update_system_coach` tool - update an existing system coach
///
/// Admin only.
///
/// # Parameters
/// - `coach_id`: UUID of the system coach to update (required)
/// - `title`: New title (optional)
/// - `description`: New description (optional)
/// - `system_prompt`: New system prompt (optional)
/// - `category`: New category (optional)
/// - `tags`: New tags array (optional)
/// - `visibility`: New visibility level (optional)
///
/// # Returns
/// Updated system coach details
#[must_use]
pub fn handle_admin_update_system_coach(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "admin_update_system_coach cancelled".to_owned(),
                ));
            }
        }

        let user_id = parse_user_id_for_protocol(&request.user_id)?;
        let tenant_id =
            verify_admin_access(executor, user_id, request.tenant_id.as_deref()).await?;

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
            sample_prompts: request
                .parameters
                .get("sample_prompts")
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
            .update_system_coach(coach_id, tenant_id, &update_request)
            .await
            .map_err(|e| {
                ProtocolError::InternalError(format!("Failed to update system coach: {e}"))
            })?;

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
                    "visibility": c.visibility.as_str(),
                    "is_system": c.is_system,
                    "updated_at": c.updated_at.to_rfc3339(),
                })),
                error: None,
                metadata: None,
            }),
            None => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("System coach not found: {coach_id}")),
                metadata: None,
            }),
        }
    })
}

/// Handle `admin_delete_system_coach` tool - delete a system coach
///
/// Admin only. This will also remove all assignments.
///
/// # Parameters
/// - `coach_id`: UUID of the system coach to delete (required)
///
/// # Returns
/// Success confirmation
#[must_use]
pub fn handle_admin_delete_system_coach(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "admin_delete_system_coach cancelled".to_owned(),
                ));
            }
        }

        let user_id = parse_user_id_for_protocol(&request.user_id)?;
        let tenant_id =
            verify_admin_access(executor, user_id, request.tenant_id.as_deref()).await?;

        let coach_id = request
            .parameters
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("Missing required parameter: coach_id".to_owned())
            })?;

        let manager = get_coaches_manager(executor)?;
        let deleted = manager
            .delete_system_coach(coach_id, tenant_id)
            .await
            .map_err(|e| {
                ProtocolError::InternalError(format!("Failed to delete system coach: {e}"))
            })?;

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
                error: Some(format!("System coach not found: {coach_id}")),
                metadata: None,
            })
        }
    })
}

/// Handle `admin_assign_coach` tool - assign a system coach to a user
///
/// Admin only. Assigns a system coach to a specific user or all users in the tenant.
///
/// # Parameters
/// - `coach_id`: UUID of the system coach to assign (required)
/// - `user_id`: UUID of the user to assign to. If omitted, assigns to all tenant users.
///
/// # Returns
/// Assignment details
#[must_use]
pub fn handle_admin_assign_coach(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "admin_assign_coach cancelled".to_owned(),
                ));
            }
        }

        let admin_user_id = parse_user_id_for_protocol(&request.user_id)?;
        let tenant_id =
            verify_admin_access(executor, admin_user_id, request.tenant_id.as_deref()).await?;

        let coach_id = request
            .parameters
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("Missing required parameter: coach_id".to_owned())
            })?;

        let target_user_id_str = request
            .parameters
            .get("user_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("Missing required parameter: user_id".to_owned())
            })?;

        let target_user_id = Uuid::parse_str(target_user_id_str).map_err(|_| {
            ProtocolError::InvalidRequest(format!("Invalid user_id: {target_user_id_str}"))
        })?;

        let manager = get_coaches_manager(executor)?;

        // First verify the coach exists and is a system coach in this tenant
        let coach = manager
            .get_system_coach(coach_id, tenant_id)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Failed to get coach: {e}")))?
            .ok_or_else(|| {
                ProtocolError::InvalidRequest(format!("System coach not found: {coach_id}"))
            })?;

        // Verify target user belongs to the same tenant as the admin
        verify_user_tenant_membership(executor, target_user_id, tenant_id).await?;

        // Assign to specific user
        manager
            .assign_coach(coach_id, target_user_id, admin_user_id)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Failed to assign coach: {e}")))?;

        Ok(UniversalResponse {
            success: true,
            result: Some(json!({
                "assigned": true,
                "coach_id": coach_id,
                "coach_title": coach.title,
                "user_id": target_user_id.to_string(),
                "assigned_by": admin_user_id.to_string(),
            })),
            error: None,
            metadata: None,
        })
    })
}

/// Handle `admin_unassign_coach` tool - remove a coach assignment from a user
///
/// Admin only.
///
/// # Parameters
/// - `coach_id`: UUID of the system coach to unassign (required)
/// - `user_id`: UUID of the user to unassign from. If omitted, unassigns from all users.
///
/// # Returns
/// Success confirmation
#[must_use]
pub fn handle_admin_unassign_coach(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "admin_unassign_coach cancelled".to_owned(),
                ));
            }
        }

        let admin_user_id = parse_user_id_for_protocol(&request.user_id)?;
        let tenant_id =
            verify_admin_access(executor, admin_user_id, request.tenant_id.as_deref()).await?;

        let coach_id = request
            .parameters
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("Missing required parameter: coach_id".to_owned())
            })?;

        let target_user_id_str = request
            .parameters
            .get("user_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("Missing required parameter: user_id".to_owned())
            })?;

        let target_user_id = Uuid::parse_str(target_user_id_str).map_err(|_| {
            ProtocolError::InvalidRequest(format!("Invalid user_id: {target_user_id_str}"))
        })?;

        let manager = get_coaches_manager(executor)?;

        // Verify target user belongs to the same tenant as the admin
        verify_user_tenant_membership(executor, target_user_id, tenant_id).await?;

        let unassigned = manager
            .unassign_coach(coach_id, target_user_id)
            .await
            .map_err(|e| ProtocolError::InternalError(format!("Failed to unassign coach: {e}")))?;

        if unassigned {
            Ok(UniversalResponse {
                success: true,
                result: Some(json!({
                    "unassigned": true,
                    "coach_id": coach_id,
                    "user_id": target_user_id.to_string(),
                })),
                error: None,
                metadata: None,
            })
        } else {
            Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!(
                    "Assignment not found for coach {coach_id} and user {target_user_id}"
                )),
                metadata: None,
            })
        }
    })
}

/// Handle `admin_list_coach_assignments` tool - list coach assignments
///
/// Admin only. Lists all assignments with optional filtering.
///
/// # Parameters
/// - `coach_id`: Filter by coach ID (optional)
/// - `user_id`: Filter by user ID (optional)
/// - `limit`: Maximum results (default: 100)
/// - `offset`: Pagination offset (default: 0)
///
/// # Returns
/// JSON array of assignment records
#[must_use]
pub fn handle_admin_list_coach_assignments(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "admin_list_coach_assignments cancelled".to_owned(),
                ));
            }
        }

        let admin_user_id = parse_user_id_for_protocol(&request.user_id)?;
        let tenant_id =
            verify_admin_access(executor, admin_user_id, request.tenant_id.as_deref()).await?;

        let coach_id = request.parameters.get("coach_id").and_then(Value::as_str);

        let manager = get_coaches_manager(executor)?;

        // Currently the database method requires a coach_id
        // If no coach_id provided, return error for now
        let Some(coach_id) = coach_id else {
            return Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some("coach_id is required to list assignments".to_owned()),
                metadata: None,
            });
        };

        // Verify the coach belongs to the admin's tenant
        manager
            .get_system_coach(coach_id, tenant_id)
            .await
            .map_err(|e| {
                ProtocolError::InternalError(format!("Failed to verify coach tenant: {e}"))
            })?
            .ok_or_else(|| {
                ProtocolError::InvalidRequest(format!("System coach {coach_id} not found"))
            })?;

        // List assignments scoped to the admin's tenant
        let assignments = manager
            .list_assignments_for_tenant(coach_id, tenant_id)
            .await
            .map_err(|e| {
                ProtocolError::InternalError(format!("Failed to list assignments: {e}"))
            })?;

        let assignment_list: Vec<Value> = assignments
            .iter()
            .map(|a| {
                json!({
                    "user_id": a.user_id,
                    "user_email": a.user_email,
                    "assigned_at": a.assigned_at,
                    "assigned_by": a.assigned_by,
                })
            })
            .collect();

        Ok(UniversalResponse {
            success: true,
            result: Some(json!({
                "coach_id": coach_id,
                "assignments": assignment_list,
                "count": assignment_list.len(),
            })),
            error: None,
            metadata: None,
        })
    })
}
