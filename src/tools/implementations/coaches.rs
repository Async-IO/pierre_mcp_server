// ABOUTME: AI coach management tools with direct database access.
// ABOUTME: Implements list_coaches, create_coach, get_coach, etc. using CoachesManager.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # AI Coach Management Tools
//!
//! This module provides tools for AI coach management with direct business logic:
//! - `ListCoachesTool` - List available coaches
//! - `CreateCoachTool` - Create a custom coach
//! - `GetCoachTool` - Get coach details
//! - `UpdateCoachTool` - Update coach settings
//! - `DeleteCoachTool` - Delete a coach
//! - `ToggleCoachFavoriteTool` - Toggle favorite status
//! - `SearchCoachesTool` - Search coaches
//! - `ActivateCoachTool` - Activate a coach
//! - `DeactivateCoachTool` - Deactivate the active coach
//! - `GetActiveCoachTool` - Get currently active coach
//! - `HideCoachTool` - Hide a coach from listings
//! - `ShowCoachTool` - Show a hidden coach
//! - `ListHiddenCoachesTool` - List hidden coaches

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::database::coaches::{
    Coach, CoachCategory, CoachListItem, CoachesManager, CreateCoachRequest, ListCoachesFilter,
    UpdateCoachRequest,
};
use crate::errors::{AppError, AppResult};
use crate::mcp::schema::{JsonSchema, PropertySchema};
use crate::models::TenantId;
use crate::tools::context::ToolExecutionContext;
use crate::tools::result::ToolResult;
use crate::tools::traits::{McpTool, ToolCapabilities};

// ============================================================================
// Helper functions
// ============================================================================

/// Get `CoachesManager` from context resources
fn get_coaches_manager(ctx: &ToolExecutionContext) -> AppResult<CoachesManager> {
    let pool = ctx
        .resources
        .database
        .sqlite_pool()
        .ok_or_else(|| AppError::internal("SQLite database required for coaches"))?;
    Ok(CoachesManager::new(pool.clone()))
}

/// Get tenant ID from context, defaulting to `user_id` as `TenantId`
fn get_tenant_id(ctx: &ToolExecutionContext) -> TenantId {
    ctx.tenant_id
        .map_or_else(|| TenantId::from(ctx.user_id), TenantId::from)
}

/// Format a coach list item for JSON response
fn format_coach_summary(item: &CoachListItem) -> Value {
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
        "last_used_at": item.coach.last_used_at.map(|dt: chrono::DateTime<chrono::Utc>| dt.to_rfc3339()),
        "updated_at": item.coach.updated_at.to_rfc3339(),
    })
}

/// Format a full coach for detailed response
fn format_coach_full(coach: &Coach) -> Value {
    json!({
        "id": coach.id.to_string(),
        "title": coach.title,
        "description": coach.description,
        "system_prompt": coach.system_prompt,
        "category": coach.category.as_str(),
        "tags": coach.tags,
        "sample_prompts": coach.sample_prompts,
        "token_count": coach.token_count,
        "is_favorite": coach.is_favorite,
        "is_active": coach.is_active,
        "is_system": coach.is_system,
        "visibility": coach.visibility.as_str(),
        "use_count": coach.use_count,
        "last_used_at": coach.last_used_at.map(|dt: chrono::DateTime<chrono::Utc>| dt.to_rfc3339()),
        "created_at": coach.created_at.to_rfc3339(),
        "updated_at": coach.updated_at.to_rfc3339(),
    })
}

/// Format a coach for search results (without assignment info)
fn format_coach_for_search(coach: &Coach) -> Value {
    json!({
        "id": coach.id.to_string(),
        "title": coach.title,
        "description": coach.description,
        "category": coach.category.as_str(),
        "tags": coach.tags,
        "token_count": coach.token_count,
        "is_favorite": coach.is_favorite,
        "is_system": coach.is_system,
        "use_count": coach.use_count,
        "last_used_at": coach.last_used_at.map(|dt: chrono::DateTime<chrono::Utc>| dt.to_rfc3339()),
        "updated_at": coach.updated_at.to_rfc3339(),
    })
}

// ============================================================================
// ListCoachesTool
// ============================================================================

/// Tool for listing available AI coaches.
pub struct ListCoachesTool;

#[async_trait]
impl McpTool for ListCoachesTool {
    fn name(&self) -> &'static str {
        "list_coaches"
    }

    fn description(&self) -> &'static str {
        "List available AI coaches for personalized training guidance"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "category".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("Filter by category".to_owned()),
            },
        );
        properties.insert(
            "include_system".to_owned(),
            PropertySchema {
                property_type: "boolean".to_owned(),
                description: Some("Include system coaches. Default: true".to_owned()),
            },
        );
        properties.insert(
            "favorites_only".to_owned(),
            PropertySchema {
                property_type: "boolean".to_owned(),
                description: Some("Only show favorites. Default: false".to_owned()),
            },
        );
        properties.insert(
            "limit".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Max results. Default: 50".to_owned()),
            },
        );
        properties.insert(
            "offset".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Pagination offset. Default: 0".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: None,
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::COACHES | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let manager = get_coaches_manager(ctx)?;
        let tenant_id = get_tenant_id(ctx);

        let category = args
            .get("category")
            .and_then(Value::as_str)
            .map(CoachCategory::parse);

        let favorites_only = args
            .get("favorites_only")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let include_system = args
            .get("include_system")
            .and_then(Value::as_bool)
            .unwrap_or(true);

        let include_hidden = args
            .get("include_hidden")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        #[allow(clippy::cast_possible_truncation)]
        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .map(|v| v.min(100) as u32);

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let offset = args.get("offset").and_then(|v| {
            v.as_u64()
                .map(|n| n.min(u64::from(u32::MAX)) as u32)
                .or_else(|| v.as_f64().map(|f| f as u32))
        });

        let filter = ListCoachesFilter {
            category,
            favorites_only,
            limit,
            offset,
            include_system,
            include_hidden,
        };

        let coaches = manager.list(ctx.user_id, tenant_id, &filter).await?;
        let total = manager.count(ctx.user_id, tenant_id).await?;

        let coach_summaries: Vec<Value> = coaches.iter().map(format_coach_summary).collect();

        let returned_count = coach_summaries.len();
        #[allow(clippy::cast_possible_truncation)]
        let has_more = limit.is_some_and(|l| returned_count == l as usize);

        Ok(ToolResult::ok(json!({
            "coaches": coach_summaries,
            "count": returned_count,
            "total": total,
            "offset": offset.unwrap_or(0),
            "limit": limit.unwrap_or(50),
            "has_more": has_more,
        })))
    }
}

// ============================================================================
// CreateCoachTool
// ============================================================================

/// Tool for creating a custom AI coach.
pub struct CreateCoachTool;

#[async_trait]
impl McpTool for CreateCoachTool {
    fn name(&self) -> &'static str {
        "create_coach"
    }

    fn description(&self) -> &'static str {
        "Create a custom AI coach with personalized training guidance"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "title".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("Display title for the coach".to_owned()),
            },
        );
        properties.insert(
            "system_prompt".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("System prompt that shapes AI responses".to_owned()),
            },
        );
        properties.insert(
            "description".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("Description of the coach".to_owned()),
            },
        );
        properties.insert(
            "category".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Category: training, nutrition, recovery, recipes, custom".to_owned(),
                ),
            },
        );
        properties.insert(
            "tags".to_owned(),
            PropertySchema {
                property_type: "array".to_owned(),
                description: Some("Tags for organization".to_owned()),
            },
        );
        properties.insert(
            "sample_prompts".to_owned(),
            PropertySchema {
                property_type: "array".to_owned(),
                description: Some("Example prompts to show users".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["title".to_owned(), "system_prompt".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::COACHES | ToolCapabilities::WRITES_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let title = args
            .get("title")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("title is required"))?;

        let system_prompt = args
            .get("system_prompt")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("system_prompt is required"))?;

        let description = args.get("description").and_then(Value::as_str);

        let category = args
            .get("category")
            .and_then(Value::as_str)
            .map(CoachCategory::parse)
            .unwrap_or_default();

        let tags: Vec<String> = args
            .get("tags")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let sample_prompts: Vec<String> = args
            .get("sample_prompts")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let manager = get_coaches_manager(ctx)?;
        let tenant_id = get_tenant_id(ctx);

        let request = CreateCoachRequest {
            title: title.to_owned(),
            description: description.map(String::from),
            system_prompt: system_prompt.to_owned(),
            category,
            tags,
            sample_prompts,
        };

        let coach = manager.create(ctx.user_id, tenant_id, &request).await?;

        Ok(ToolResult::ok(json!({
            "success": true,
            "coach": format_coach_full(&coach),
            "message": format!("Coach '{}' created successfully", coach.title),
        })))
    }
}

// ============================================================================
// GetCoachTool
// ============================================================================

/// Tool for getting coach details.
pub struct GetCoachTool;

#[async_trait]
impl McpTool for GetCoachTool {
    fn name(&self) -> &'static str {
        "get_coach"
    }

    fn description(&self) -> &'static str {
        "Get detailed information about a specific coach"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "coach_id".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("ID of the coach to retrieve".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["coach_id".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::COACHES | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let coach_id = args
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("coach_id is required"))?;

        let manager = get_coaches_manager(ctx)?;
        let tenant_id = get_tenant_id(ctx);

        let coach = manager
            .get(coach_id, ctx.user_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Coach {coach_id}")))?;

        Ok(ToolResult::ok(json!({
            "coach": format_coach_full(&coach),
        })))
    }
}

// ============================================================================
// UpdateCoachTool
// ============================================================================

/// Tool for updating coach settings.
pub struct UpdateCoachTool;

#[async_trait]
impl McpTool for UpdateCoachTool {
    fn name(&self) -> &'static str {
        "update_coach"
    }

    fn description(&self) -> &'static str {
        "Update an existing coach's settings"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "coach_id".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("ID of the coach to update".to_owned()),
            },
        );
        properties.insert(
            "title".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("New title".to_owned()),
            },
        );
        properties.insert(
            "system_prompt".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("New system prompt".to_owned()),
            },
        );
        properties.insert(
            "description".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("New description".to_owned()),
            },
        );
        properties.insert(
            "category".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("New category".to_owned()),
            },
        );
        properties.insert(
            "tags".to_owned(),
            PropertySchema {
                property_type: "array".to_owned(),
                description: Some("New tags".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["coach_id".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::COACHES | ToolCapabilities::WRITES_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let coach_id = args
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("coach_id is required"))?;

        let title = args.get("title").and_then(Value::as_str).map(String::from);
        let description = args
            .get("description")
            .and_then(Value::as_str)
            .map(String::from);
        let system_prompt = args
            .get("system_prompt")
            .and_then(Value::as_str)
            .map(String::from);
        let category = args
            .get("category")
            .and_then(Value::as_str)
            .map(CoachCategory::parse);
        let tags: Option<Vec<String>> = args.get("tags").and_then(Value::as_array).map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        });
        let sample_prompts: Option<Vec<String>> = args
            .get("sample_prompts")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            });

        let manager = get_coaches_manager(ctx)?;
        let tenant_id = get_tenant_id(ctx);

        let request = UpdateCoachRequest {
            title,
            description,
            system_prompt,
            category,
            tags,
            sample_prompts,
        };

        let coach = manager
            .update(coach_id, ctx.user_id, tenant_id, &request)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Coach {coach_id}")))?;

        Ok(ToolResult::ok(json!({
            "success": true,
            "coach": format_coach_full(&coach),
            "message": format!("Coach '{}' updated successfully", coach.title),
        })))
    }
}

// ============================================================================
// DeleteCoachTool
// ============================================================================

/// Tool for deleting a coach.
pub struct DeleteCoachTool;

#[async_trait]
impl McpTool for DeleteCoachTool {
    fn name(&self) -> &'static str {
        "delete_coach"
    }

    fn description(&self) -> &'static str {
        "Delete a coach"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "coach_id".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("ID of the coach to delete".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["coach_id".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::COACHES | ToolCapabilities::WRITES_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let coach_id = args
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("coach_id is required"))?;

        let manager = get_coaches_manager(ctx)?;
        let tenant_id = get_tenant_id(ctx);

        let deleted = manager.delete(coach_id, ctx.user_id, tenant_id).await?;

        if !deleted {
            return Err(AppError::not_found(format!("Coach {coach_id}")));
        }

        Ok(ToolResult::ok(json!({
            "success": true,
            "message": "Coach deleted successfully",
        })))
    }
}

// ============================================================================
// ToggleCoachFavoriteTool
// ============================================================================

/// Tool for toggling coach favorite status.
pub struct ToggleCoachFavoriteTool;

#[async_trait]
impl McpTool for ToggleCoachFavoriteTool {
    fn name(&self) -> &'static str {
        "toggle_coach_favorite"
    }

    fn description(&self) -> &'static str {
        "Toggle the favorite status of a coach"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "coach_id".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("ID of the coach".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["coach_id".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::COACHES | ToolCapabilities::WRITES_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let coach_id = args
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("coach_id is required"))?;

        let manager = get_coaches_manager(ctx)?;
        let tenant_id = get_tenant_id(ctx);

        let is_favorite = manager
            .toggle_favorite(coach_id, ctx.user_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Coach {coach_id}")))?;

        Ok(ToolResult::ok(json!({
            "success": true,
            "coach_id": coach_id,
            "is_favorite": is_favorite,
            "message": if is_favorite { "Coach added to favorites" } else { "Coach removed from favorites" },
        })))
    }
}

// ============================================================================
// SearchCoachesTool
// ============================================================================

/// Tool for searching coaches.
pub struct SearchCoachesTool;

#[async_trait]
impl McpTool for SearchCoachesTool {
    fn name(&self) -> &'static str {
        "search_coaches"
    }

    fn description(&self) -> &'static str {
        "Search for coaches by query. Returns up to 20 results by default. Check the `has_more` field before requesting additional results with offset."
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "query".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("Search query".to_owned()),
            },
        );
        properties.insert(
            "category".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("Filter by category".to_owned()),
            },
        );
        properties.insert(
            "limit".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Maximum results per request. Default: 20, max: 100".to_owned()),
            },
        );
        properties.insert(
            "offset".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Pagination offset. Default: 0. Only use if previous response had has_more=true".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["query".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::COACHES | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let query = args
            .get("query")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("query is required"))?;

        #[allow(clippy::cast_possible_truncation)]
        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .map(|v| v.min(100) as u32);

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let offset = args.get("offset").and_then(|v| {
            v.as_u64()
                .map(|n| n.min(u64::from(u32::MAX)) as u32)
                .or_else(|| v.as_f64().map(|f| f as u32))
        });

        let manager = get_coaches_manager(ctx)?;
        let tenant_id = get_tenant_id(ctx);

        let coaches = manager
            .search(ctx.user_id, tenant_id, query, limit, offset)
            .await?;

        let results: Vec<Value> = coaches.iter().map(format_coach_for_search).collect();

        let returned_count = results.len();
        let limit_val = limit.unwrap_or(20);
        #[allow(clippy::cast_possible_truncation)]
        let has_more = returned_count == limit_val as usize;

        Ok(ToolResult::ok(json!({
            "results": results,
            "returned_count": returned_count,
            "offset": offset.unwrap_or(0),
            "limit": limit_val,
            "has_more": has_more,
            "query": query,
        })))
    }
}

// ============================================================================
// ActivateCoachTool
// ============================================================================

/// Tool for activating a coach.
pub struct ActivateCoachTool;

#[async_trait]
impl McpTool for ActivateCoachTool {
    fn name(&self) -> &'static str {
        "activate_coach"
    }

    fn description(&self) -> &'static str {
        "Activate a coach for personalized training guidance"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "coach_id".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("ID of the coach to activate".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["coach_id".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::COACHES | ToolCapabilities::WRITES_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let coach_id = args
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("coach_id is required"))?;

        let manager = get_coaches_manager(ctx)?;
        let tenant_id = get_tenant_id(ctx);

        let coach = manager
            .activate_coach(coach_id, ctx.user_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Coach {coach_id}")))?;

        Ok(ToolResult::ok(json!({
            "success": true,
            "coach": format_coach_full(&coach),
            "message": format!("Coach '{}' is now active", coach.title),
        })))
    }
}

// ============================================================================
// DeactivateCoachTool
// ============================================================================

/// Tool for deactivating the current coach.
pub struct DeactivateCoachTool;

#[async_trait]
impl McpTool for DeactivateCoachTool {
    fn name(&self) -> &'static str {
        "deactivate_coach"
    }

    fn description(&self) -> &'static str {
        "Deactivate the current coach and return to default AI guidance"
    }

    fn input_schema(&self) -> JsonSchema {
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(HashMap::new()),
            required: None,
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::COACHES | ToolCapabilities::WRITES_DATA
    }

    async fn execute(&self, _args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let manager = get_coaches_manager(ctx)?;
        let tenant_id = get_tenant_id(ctx);

        let deactivated = manager.deactivate_coach(ctx.user_id, tenant_id).await?;

        Ok(ToolResult::ok(json!({
            "success": true,
            "was_active": deactivated,
            "message": if deactivated {
                "Coach deactivated. Using default AI guidance."
            } else {
                "No coach was active."
            },
        })))
    }
}

// ============================================================================
// GetActiveCoachTool
// ============================================================================

/// Tool for getting the currently active coach.
pub struct GetActiveCoachTool;

#[async_trait]
impl McpTool for GetActiveCoachTool {
    fn name(&self) -> &'static str {
        "get_active_coach"
    }

    fn description(&self) -> &'static str {
        "Get the currently active coach"
    }

    fn input_schema(&self) -> JsonSchema {
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(HashMap::new()),
            required: None,
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::COACHES | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, _args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let manager = get_coaches_manager(ctx)?;
        let tenant_id = get_tenant_id(ctx);

        let active_coach = manager.get_active_coach(ctx.user_id, tenant_id).await?;

        Ok(active_coach.map_or_else(
            || {
                ToolResult::ok(json!({
                    "has_active_coach": false,
                    "message": "No coach is currently active. Using default AI guidance.",
                }))
            },
            |coach| {
                ToolResult::ok(json!({
                    "has_active_coach": true,
                    "coach": format_coach_full(&coach),
                }))
            },
        ))
    }
}

// ============================================================================
// HideCoachTool
// ============================================================================

/// Tool for hiding a coach from listings.
pub struct HideCoachTool;

#[async_trait]
impl McpTool for HideCoachTool {
    fn name(&self) -> &'static str {
        "hide_coach"
    }

    fn description(&self) -> &'static str {
        "Hide a coach from listings"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "coach_id".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("ID of the coach to hide".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["coach_id".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::COACHES | ToolCapabilities::WRITES_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let coach_id = args
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("coach_id is required"))?;

        let manager = get_coaches_manager(ctx)?;

        manager.hide_coach(coach_id, ctx.user_id).await?;

        Ok(ToolResult::ok(json!({
            "success": true,
            "coach_id": coach_id,
            "is_hidden": true,
            "message": "Coach is now hidden from listings",
        })))
    }
}

// ============================================================================
// ShowCoachTool
// ============================================================================

/// Tool for showing a hidden coach.
pub struct ShowCoachTool;

#[async_trait]
impl McpTool for ShowCoachTool {
    fn name(&self) -> &'static str {
        "show_coach"
    }

    fn description(&self) -> &'static str {
        "Show a previously hidden coach"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "coach_id".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("ID of the coach to show".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["coach_id".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::COACHES | ToolCapabilities::WRITES_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let coach_id = args
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("coach_id is required"))?;

        let manager = get_coaches_manager(ctx)?;

        let shown = manager.show_coach(coach_id, ctx.user_id).await?;

        Ok(ToolResult::ok(json!({
            "success": shown,
            "coach_id": coach_id,
            "is_hidden": false,
            "message": if shown { "Coach is now visible in listings" } else { "Coach was not hidden" },
        })))
    }
}

// ============================================================================
// ListHiddenCoachesTool
// ============================================================================

/// Tool for listing hidden coaches.
pub struct ListHiddenCoachesTool;

#[async_trait]
impl McpTool for ListHiddenCoachesTool {
    fn name(&self) -> &'static str {
        "list_hidden_coaches"
    }

    fn description(&self) -> &'static str {
        "List all hidden coaches"
    }

    fn input_schema(&self) -> JsonSchema {
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(HashMap::new()),
            required: None,
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::COACHES | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, _args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let manager = get_coaches_manager(ctx)?;
        let tenant_id = get_tenant_id(ctx);

        let hidden_coaches = manager.list_hidden_coaches(ctx.user_id, tenant_id).await?;

        let coaches: Vec<Value> = hidden_coaches.iter().map(format_coach_for_search).collect();

        Ok(ToolResult::ok(json!({
            "coaches": coaches,
            "count": coaches.len(),
        })))
    }
}

// ============================================================================
// Module exports
// ============================================================================

/// Create all coach tools for registration
#[must_use]
pub fn create_coach_tools() -> Vec<Box<dyn McpTool>> {
    vec![
        Box::new(ListCoachesTool),
        Box::new(CreateCoachTool),
        Box::new(GetCoachTool),
        Box::new(UpdateCoachTool),
        Box::new(DeleteCoachTool),
        Box::new(ToggleCoachFavoriteTool),
        Box::new(SearchCoachesTool),
        Box::new(ActivateCoachTool),
        Box::new(DeactivateCoachTool),
        Box::new(GetActiveCoachTool),
        Box::new(HideCoachTool),
        Box::new(ShowCoachTool),
        Box::new(ListHiddenCoachesTool),
    ]
}
