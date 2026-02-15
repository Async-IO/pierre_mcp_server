// ABOUTME: Admin-only tools for system coach management with direct database access.
// ABOUTME: Implements admin coach operations using CoachesManager directly.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Admin Tools
//!
//! This module provides admin-only tools for system coach management with direct database access:
//! - `AdminListSystemCoachesTool` - List all system coaches
//! - `AdminCreateSystemCoachTool` - Create a system-wide coach
//! - `AdminGetSystemCoachTool` - Get system coach details
//! - `AdminUpdateSystemCoachTool` - Update a system coach
//! - `AdminDeleteSystemCoachTool` - Delete a system coach
//! - `AdminAssignCoachTool` - Assign coach to a user
//! - `AdminUnassignCoachTool` - Remove coach assignment
//! - `AdminListCoachAssignmentsTool` - List coach assignments
//!
//! All tools require admin privileges and use direct `CoachesManager` access.

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::database::coaches::{
    Coach, CoachAssignment, CoachCategory, CoachVisibility, CoachesManager,
    CreateSystemCoachRequest, UpdateCoachRequest,
};
use crate::database_plugins::DatabaseProvider;
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
    let pool =
        ctx.resources.database.sqlite_pool().ok_or_else(|| {
            AppError::internal("SQLite database required for admin coach operations")
        })?;
    Ok(CoachesManager::new(pool.clone()))
}

/// Get tenant ID from context, defaulting to `user_id` as `TenantId`
fn get_tenant_id(ctx: &ToolExecutionContext) -> TenantId {
    ctx.tenant_id
        .map_or_else(|| TenantId::from(ctx.user_id), TenantId::from)
}

/// Verify that a target user belongs to the same tenant as the admin.
///
/// Prevents cross-tenant operations by checking tenant membership
/// before allowing assign/unassign/listing operations on a user.
async fn verify_user_in_tenant(
    ctx: &ToolExecutionContext,
    target_user_id: Uuid,
    tenant_id: TenantId,
) -> AppResult<()> {
    let user_tenants = ctx
        .resources
        .database
        .list_tenants_for_user(target_user_id)
        .await
        .map_err(|e| {
            AppError::database(format!(
                "Failed to verify tenant membership for user {target_user_id}: {e}"
            ))
        })?;

    if !user_tenants.iter().any(|t| t.id == tenant_id) {
        return Err(AppError::auth_invalid(format!(
            "User {target_user_id} does not belong to this tenant"
        )));
    }

    Ok(())
}

/// Format a system coach for JSON response
fn format_system_coach(coach: &Coach) -> Value {
    json!({
        "id": coach.id.to_string(),
        "title": coach.title,
        "description": coach.description,
        "system_prompt": coach.system_prompt,
        "category": coach.category.as_str(),
        "tags": coach.tags,
        "sample_prompts": coach.sample_prompts,
        "token_count": coach.token_count,
        "visibility": coach.visibility.as_str(),
        "use_count": coach.use_count,
        "last_used_at": coach.last_used_at.map(|dt| dt.to_rfc3339()),
        "created_at": coach.created_at.to_rfc3339(),
        "updated_at": coach.updated_at.to_rfc3339(),
    })
}

/// Format a system coach summary for list response
fn format_system_coach_summary(coach: &Coach) -> Value {
    json!({
        "id": coach.id.to_string(),
        "title": coach.title,
        "description": coach.description,
        "category": coach.category.as_str(),
        "tags": coach.tags,
        "visibility": coach.visibility.as_str(),
        "use_count": coach.use_count,
        "updated_at": coach.updated_at.to_rfc3339(),
    })
}

/// Format a coach assignment for JSON response
fn format_assignment(assignment: &CoachAssignment) -> Value {
    json!({
        "user_id": assignment.user_id,
        "user_email": assignment.user_email,
        "assigned_at": assignment.assigned_at,
        "assigned_by": assignment.assigned_by,
    })
}

/// Parse category from string
fn parse_category(category_str: &str) -> AppResult<CoachCategory> {
    match category_str.to_lowercase().as_str() {
        "training" => Ok(CoachCategory::Training),
        "nutrition" => Ok(CoachCategory::Nutrition),
        "recovery" => Ok(CoachCategory::Recovery),
        "recipes" => Ok(CoachCategory::Recipes),
        "custom" => Ok(CoachCategory::Custom),
        other => Err(AppError::invalid_input(format!(
            "Invalid category '{other}'. Must be: training, nutrition, recovery, recipes, custom"
        ))),
    }
}

/// Parse visibility from string
fn parse_visibility(visibility_str: &str) -> AppResult<CoachVisibility> {
    match visibility_str.to_lowercase().as_str() {
        "tenant" => Ok(CoachVisibility::Tenant),
        "global" => Ok(CoachVisibility::Global),
        "private" => Ok(CoachVisibility::Private),
        other => Err(AppError::invalid_input(format!(
            "Invalid visibility '{other}'. Must be: tenant, global, private"
        ))),
    }
}

// ============================================================================
// AdminListSystemCoachesTool - List all system coaches
// ============================================================================

/// Tool for listing system coaches (admin only).
pub struct AdminListSystemCoachesTool;

#[async_trait]
impl McpTool for AdminListSystemCoachesTool {
    fn name(&self) -> &'static str {
        "admin_list_system_coaches"
    }

    fn description(&self) -> &'static str {
        "List all system coaches in the tenant (admin only)"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "limit".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Maximum number of coaches to return. Default: 50".to_owned()),
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
        ToolCapabilities::REQUIRES_AUTH
            | ToolCapabilities::READS_DATA
            | ToolCapabilities::ADMIN_ONLY
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        tracing::debug!(user_id = %ctx.user_id, "Admin listing system coaches");

        let manager = get_coaches_manager(ctx)?;
        let tenant_id = get_tenant_id(ctx);

        let coaches = manager.list_system_coaches(tenant_id).await?;

        // Apply pagination (manager returns all, we slice here)
        #[allow(clippy::cast_possible_truncation)]
        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .map_or(50, |v| v.min(200)) as usize;
        #[allow(clippy::cast_possible_truncation)]
        let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;

        let total = coaches.len();
        let paginated: Vec<_> = coaches
            .iter()
            .skip(offset)
            .take(limit)
            .map(format_system_coach_summary)
            .collect();

        Ok(ToolResult::ok(json!({
            "coaches": paginated,
            "total": total,
            "limit": limit,
            "offset": offset,
            "retrieved_at": Utc::now().to_rfc3339(),
        })))
    }
}

// ============================================================================
// AdminCreateSystemCoachTool - Create a system coach
// ============================================================================

/// Tool for creating system coaches (admin only).
pub struct AdminCreateSystemCoachTool;

#[async_trait]
impl McpTool for AdminCreateSystemCoachTool {
    fn name(&self) -> &'static str {
        "admin_create_system_coach"
    }

    fn description(&self) -> &'static str {
        "Create a new system coach visible to all tenant users (admin only)"
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
                description: Some("Description explaining the coach's purpose".to_owned()),
            },
        );
        properties.insert(
            "category".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Category: 'training', 'nutrition', 'recovery', 'recipes', 'custom'".to_owned(),
                ),
            },
        );
        properties.insert(
            "tags".to_owned(),
            PropertySchema {
                property_type: "array".to_owned(),
                description: Some("Tags for filtering and organization".to_owned()),
            },
        );
        properties.insert(
            "visibility".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("Visibility: 'tenant' (default) or 'global'".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["title".to_owned(), "system_prompt".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH
            | ToolCapabilities::WRITES_DATA
            | ToolCapabilities::ADMIN_ONLY
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        tracing::debug!(user_id = %ctx.user_id, "Admin creating system coach");

        let title = args
            .get("title")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("title is required"))?;

        let system_prompt = args
            .get("system_prompt")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("system_prompt is required"))?;

        let description = args.get("description").and_then(Value::as_str);

        let category_str = args
            .get("category")
            .and_then(Value::as_str)
            .unwrap_or("custom");
        let category = parse_category(category_str)?;

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

        let visibility_str = args
            .get("visibility")
            .and_then(Value::as_str)
            .unwrap_or("tenant");
        let visibility = parse_visibility(visibility_str)?;

        let request = CreateSystemCoachRequest {
            title: title.to_owned(),
            description: description.map(String::from),
            system_prompt: system_prompt.to_owned(),
            category,
            tags,
            sample_prompts: Vec::new(),
            visibility,
        };

        let manager = get_coaches_manager(ctx)?;
        let tenant_id = get_tenant_id(ctx);

        let coach = manager
            .create_system_coach(ctx.user_id, tenant_id, &request)
            .await?;

        Ok(ToolResult::ok(json!({
            "coach": format_system_coach(&coach),
            "message": format!("System coach '{}' created successfully", coach.title),
            "created_at": Utc::now().to_rfc3339(),
        })))
    }
}

// ============================================================================
// AdminGetSystemCoachTool - Get system coach details
// ============================================================================

/// Tool for getting system coach details (admin only).
pub struct AdminGetSystemCoachTool;

#[async_trait]
impl McpTool for AdminGetSystemCoachTool {
    fn name(&self) -> &'static str {
        "admin_get_system_coach"
    }

    fn description(&self) -> &'static str {
        "Get detailed information about a system coach (admin only)"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "coach_id".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("ID of the system coach to retrieve".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["coach_id".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH
            | ToolCapabilities::READS_DATA
            | ToolCapabilities::ADMIN_ONLY
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let coach_id = args
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("coach_id is required"))?;

        tracing::debug!(user_id = %ctx.user_id, coach_id = %coach_id, "Admin getting system coach");

        let manager = get_coaches_manager(ctx)?;
        let tenant_id = get_tenant_id(ctx);

        manager
            .get_system_coach(coach_id, tenant_id)
            .await?
            .map_or_else(
                || {
                    Ok(ToolResult::ok(json!({
                        "coach": null,
                        "message": format!("System coach '{coach_id}' not found"),
                    })))
                },
                |coach| {
                    Ok(ToolResult::ok(json!({
                        "coach": format_system_coach(&coach),
                        "retrieved_at": Utc::now().to_rfc3339(),
                    })))
                },
            )
    }
}

// ============================================================================
// AdminUpdateSystemCoachTool - Update a system coach
// ============================================================================

/// Tool for updating system coaches (admin only).
pub struct AdminUpdateSystemCoachTool;

#[async_trait]
impl McpTool for AdminUpdateSystemCoachTool {
    fn name(&self) -> &'static str {
        "admin_update_system_coach"
    }

    fn description(&self) -> &'static str {
        "Update an existing system coach (admin only)"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "coach_id".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("ID of the system coach to update".to_owned()),
            },
        );
        properties.insert(
            "title".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("New display title".to_owned()),
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
        ToolCapabilities::REQUIRES_AUTH
            | ToolCapabilities::WRITES_DATA
            | ToolCapabilities::ADMIN_ONLY
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let coach_id = args
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("coach_id is required"))?;

        tracing::debug!(user_id = %ctx.user_id, coach_id = %coach_id, "Admin updating system coach");

        let title = args.get("title").and_then(Value::as_str).map(String::from);
        let system_prompt = args
            .get("system_prompt")
            .and_then(Value::as_str)
            .map(String::from);
        let description = args
            .get("description")
            .and_then(Value::as_str)
            .map(String::from);

        let category = args
            .get("category")
            .and_then(Value::as_str)
            .map(parse_category)
            .transpose()?;

        let tags: Option<Vec<String>> = args.get("tags").and_then(Value::as_array).map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        });

        let request = UpdateCoachRequest {
            title,
            description,
            system_prompt,
            category,
            tags,
            sample_prompts: None,
        };

        let manager = get_coaches_manager(ctx)?;
        let tenant_id = get_tenant_id(ctx);

        manager
            .update_system_coach(coach_id, tenant_id, &request)
            .await?
            .map_or_else(
                || {
                    Ok(ToolResult::ok(json!({
                        "coach": null,
                        "message": format!("System coach '{coach_id}' not found"),
                    })))
                },
                |coach| {
                    Ok(ToolResult::ok(json!({
                        "coach": format_system_coach(&coach),
                        "message": format!("System coach '{}' updated successfully", coach.title),
                        "updated_at": Utc::now().to_rfc3339(),
                    })))
                },
            )
    }
}

// ============================================================================
// AdminDeleteSystemCoachTool - Delete a system coach
// ============================================================================

/// Tool for deleting system coaches (admin only).
pub struct AdminDeleteSystemCoachTool;

#[async_trait]
impl McpTool for AdminDeleteSystemCoachTool {
    fn name(&self) -> &'static str {
        "admin_delete_system_coach"
    }

    fn description(&self) -> &'static str {
        "Delete a system coach and remove all assignments (admin only)"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "coach_id".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("ID of the system coach to delete".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["coach_id".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH
            | ToolCapabilities::WRITES_DATA
            | ToolCapabilities::ADMIN_ONLY
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let coach_id = args
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("coach_id is required"))?;

        tracing::debug!(user_id = %ctx.user_id, coach_id = %coach_id, "Admin deleting system coach");

        let manager = get_coaches_manager(ctx)?;
        let tenant_id = get_tenant_id(ctx);

        let deleted = manager.delete_system_coach(coach_id, tenant_id).await?;

        if deleted {
            Ok(ToolResult::ok(json!({
                "success": true,
                "coach_id": coach_id,
                "message": format!("System coach '{coach_id}' deleted successfully"),
                "deleted_at": Utc::now().to_rfc3339(),
            })))
        } else {
            Ok(ToolResult::ok(json!({
                "success": false,
                "coach_id": coach_id,
                "message": format!("System coach '{coach_id}' not found or not a system coach"),
            })))
        }
    }
}

// ============================================================================
// AdminAssignCoachTool - Assign coach to user
// ============================================================================

/// Tool for assigning coaches to users (admin only).
pub struct AdminAssignCoachTool;

#[async_trait]
impl McpTool for AdminAssignCoachTool {
    fn name(&self) -> &'static str {
        "admin_assign_coach"
    }

    fn description(&self) -> &'static str {
        "Assign a system coach to a specific user (admin only)"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "coach_id".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("ID of the system coach to assign".to_owned()),
            },
        );
        properties.insert(
            "user_id".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("ID of the user to assign the coach to".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["coach_id".to_owned(), "user_id".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH
            | ToolCapabilities::WRITES_DATA
            | ToolCapabilities::ADMIN_ONLY
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let coach_id = args
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("coach_id is required"))?;

        let user_id_str = args
            .get("user_id")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("user_id is required"))?;

        let target_user_id = Uuid::parse_str(user_id_str)
            .map_err(|_| AppError::invalid_input(format!("Invalid user_id UUID: {user_id_str}")))?;

        tracing::debug!(
            admin_user_id = %ctx.user_id,
            coach_id = %coach_id,
            target_user_id = %target_user_id,
            "Admin assigning coach to user"
        );

        let manager = get_coaches_manager(ctx)?;
        let tenant_id = get_tenant_id(ctx);

        // Verify target user belongs to the same tenant as the admin
        verify_user_in_tenant(ctx, target_user_id, tenant_id).await?;

        // Verify the coach belongs to this tenant before assigning
        let coach = manager.get_system_coach(coach_id, tenant_id).await?;
        if coach.is_none() {
            return Err(AppError::not_found(format!(
                "Coach '{coach_id}' not found in this tenant"
            )));
        }

        let assigned = manager
            .assign_coach(coach_id, target_user_id, ctx.user_id)
            .await?;

        if assigned {
            Ok(ToolResult::ok(json!({
                "success": true,
                "coach_id": coach_id,
                "user_id": user_id_str,
                "message": format!("Coach '{coach_id}' assigned to user '{user_id_str}'"),
                "assigned_at": Utc::now().to_rfc3339(),
            })))
        } else {
            Ok(ToolResult::ok(json!({
                "success": false,
                "coach_id": coach_id,
                "user_id": user_id_str,
                "message": "Assignment already exists or coach not found",
            })))
        }
    }
}

// ============================================================================
// AdminUnassignCoachTool - Remove coach assignment
// ============================================================================

/// Tool for removing coach assignments (admin only).
pub struct AdminUnassignCoachTool;

#[async_trait]
impl McpTool for AdminUnassignCoachTool {
    fn name(&self) -> &'static str {
        "admin_unassign_coach"
    }

    fn description(&self) -> &'static str {
        "Remove a coach assignment from a user (admin only)"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "coach_id".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("ID of the system coach to unassign".to_owned()),
            },
        );
        properties.insert(
            "user_id".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("ID of the user to remove the assignment from".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["coach_id".to_owned(), "user_id".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH
            | ToolCapabilities::WRITES_DATA
            | ToolCapabilities::ADMIN_ONLY
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let coach_id = args
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("coach_id is required"))?;

        let user_id_str = args
            .get("user_id")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("user_id is required"))?;

        let target_user_id = Uuid::parse_str(user_id_str)
            .map_err(|_| AppError::invalid_input(format!("Invalid user_id UUID: {user_id_str}")))?;

        tracing::debug!(
            admin_user_id = %ctx.user_id,
            coach_id = %coach_id,
            target_user_id = %target_user_id,
            "Admin unassigning coach from user"
        );

        let manager = get_coaches_manager(ctx)?;
        let tenant_id = get_tenant_id(ctx);

        // Verify target user belongs to the same tenant as the admin
        verify_user_in_tenant(ctx, target_user_id, tenant_id).await?;

        // Verify the coach belongs to this tenant before unassigning
        let coach = manager.get_system_coach(coach_id, tenant_id).await?;
        if coach.is_none() {
            return Err(AppError::not_found(format!(
                "Coach '{coach_id}' not found in this tenant"
            )));
        }

        let unassigned = manager.unassign_coach(coach_id, target_user_id).await?;

        if unassigned {
            Ok(ToolResult::ok(json!({
                "success": true,
                "coach_id": coach_id,
                "user_id": user_id_str,
                "message": format!("Coach '{coach_id}' unassigned from user '{user_id_str}'"),
                "unassigned_at": Utc::now().to_rfc3339(),
            })))
        } else {
            Ok(ToolResult::ok(json!({
                "success": false,
                "coach_id": coach_id,
                "user_id": user_id_str,
                "message": "Assignment not found",
            })))
        }
    }
}

// ============================================================================
// AdminListCoachAssignmentsTool - List coach assignments
// ============================================================================

/// Tool for listing coach assignments (admin only).
pub struct AdminListCoachAssignmentsTool;

#[async_trait]
impl McpTool for AdminListCoachAssignmentsTool {
    fn name(&self) -> &'static str {
        "admin_list_coach_assignments"
    }

    fn description(&self) -> &'static str {
        "List all assignments for a system coach (admin only)"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "coach_id".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("ID of the coach to list assignments for".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["coach_id".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH
            | ToolCapabilities::READS_DATA
            | ToolCapabilities::ADMIN_ONLY
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let coach_id = args
            .get("coach_id")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::invalid_input("coach_id is required"))?;

        tracing::debug!(
            user_id = %ctx.user_id,
            coach_id = %coach_id,
            "Admin listing coach assignments"
        );

        let manager = get_coaches_manager(ctx)?;
        let tenant_id = get_tenant_id(ctx);

        // Verify the coach belongs to the admin's tenant
        manager
            .get_system_coach(coach_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("System coach {coach_id}")))?;

        // List assignments scoped to the admin's tenant
        let assignments = manager
            .list_assignments_for_tenant(coach_id, tenant_id)
            .await?;

        let formatted: Vec<_> = assignments.iter().map(format_assignment).collect();

        Ok(ToolResult::ok(json!({
            "coach_id": coach_id,
            "assignments": formatted,
            "total": assignments.len(),
            "retrieved_at": Utc::now().to_rfc3339(),
        })))
    }
}

// ============================================================================
// Module exports
// ============================================================================

/// Create all admin tools for registration
#[must_use]
pub fn create_admin_tools() -> Vec<Box<dyn McpTool>> {
    vec![
        Box::new(AdminListSystemCoachesTool),
        Box::new(AdminCreateSystemCoachTool),
        Box::new(AdminGetSystemCoachTool),
        Box::new(AdminUpdateSystemCoachTool),
        Box::new(AdminDeleteSystemCoachTool),
        Box::new(AdminAssignCoachTool),
        Box::new(AdminUnassignCoachTool),
        Box::new(AdminListCoachAssignmentsTool),
    ]
}
