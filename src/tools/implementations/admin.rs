// ABOUTME: Admin-only tools for system coach management.
// ABOUTME: Wraps universal protocol handlers for admin system coach operations.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Admin Tools
//!
//! This module provides admin-only tools for system coach management:
//! - `AdminListSystemCoachesTool` - List all system coaches
//! - `AdminCreateSystemCoachTool` - Create a system-wide coach
//! - `AdminGetSystemCoachTool` - Get system coach details
//! - `AdminUpdateSystemCoachTool` - Update a system coach
//! - `AdminDeleteSystemCoachTool` - Delete a system coach
//! - `AdminAssignCoachTool` - Assign coach to a user
//! - `AdminUnassignCoachTool` - Remove coach assignment
//! - `AdminListCoachAssignmentsTool` - List coach assignments
//!
//! All tools require admin privileges and wrap universal protocol handlers.

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::errors::AppResult;
use crate::mcp::schema::{JsonSchema, PropertySchema};
use crate::protocols::universal::executor::UniversalExecutor;
use crate::protocols::universal::handlers::coaches::{
    handle_admin_assign_coach, handle_admin_create_system_coach, handle_admin_delete_system_coach,
    handle_admin_get_system_coach, handle_admin_list_coach_assignments,
    handle_admin_list_system_coaches, handle_admin_unassign_coach,
    handle_admin_update_system_coach,
};
use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use crate::tools::context::ToolExecutionContext;
use crate::tools::result::ToolResult;
use crate::tools::traits::{McpTool, ToolCapabilities};

// ============================================================================
// Helper functions for converting between request/response types
// ============================================================================

/// Build a `UniversalRequest` from tool context and args
fn build_universal_request(
    tool_name: &str,
    args: &Value,
    context: &ToolExecutionContext,
) -> UniversalRequest {
    let parameters = if args.is_object() {
        args.clone()
    } else {
        json!({})
    };

    UniversalRequest {
        tool_name: tool_name.to_owned(),
        parameters,
        user_id: context.user_id.to_string(),
        protocol: "mcp".to_owned(),
        tenant_id: context.tenant_id.map(|id| id.to_string()),
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    }
}

/// Convert `UniversalResponse` to `ToolResult`
fn convert_to_tool_result(response: UniversalResponse) -> ToolResult {
    if response.success {
        let mut result = response.result.unwrap_or_else(|| json!({}));

        if let (Some(result_obj), Some(metadata)) = (result.as_object_mut(), response.metadata) {
            for (key, value) in metadata {
                if !result_obj.contains_key(&key) {
                    result_obj.insert(key, value);
                }
            }
        }

        ToolResult::ok(result)
    } else {
        let error_message = response.error.unwrap_or_else(|| "Unknown error".to_owned());
        ToolResult::error(json!({ "error": error_message }))
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

    async fn execute(&self, args: Value, context: &ToolExecutionContext) -> AppResult<ToolResult> {
        let request = build_universal_request("admin_list_system_coaches", &args, context);
        let executor = UniversalExecutor::new(context.resources.clone());

        match handle_admin_list_system_coaches(&executor, request).await {
            Ok(response) => Ok(convert_to_tool_result(response)),
            Err(e) => Ok(ToolResult::error(json!({ "error": e.to_string() }))),
        }
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

    async fn execute(&self, args: Value, context: &ToolExecutionContext) -> AppResult<ToolResult> {
        let request = build_universal_request("admin_create_system_coach", &args, context);
        let executor = UniversalExecutor::new(context.resources.clone());

        match handle_admin_create_system_coach(&executor, request).await {
            Ok(response) => Ok(convert_to_tool_result(response)),
            Err(e) => Ok(ToolResult::error(json!({ "error": e.to_string() }))),
        }
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

    async fn execute(&self, args: Value, context: &ToolExecutionContext) -> AppResult<ToolResult> {
        let request = build_universal_request("admin_get_system_coach", &args, context);
        let executor = UniversalExecutor::new(context.resources.clone());

        match handle_admin_get_system_coach(&executor, request).await {
            Ok(response) => Ok(convert_to_tool_result(response)),
            Err(e) => Ok(ToolResult::error(json!({ "error": e.to_string() }))),
        }
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

    async fn execute(&self, args: Value, context: &ToolExecutionContext) -> AppResult<ToolResult> {
        let request = build_universal_request("admin_update_system_coach", &args, context);
        let executor = UniversalExecutor::new(context.resources.clone());

        match handle_admin_update_system_coach(&executor, request).await {
            Ok(response) => Ok(convert_to_tool_result(response)),
            Err(e) => Ok(ToolResult::error(json!({ "error": e.to_string() }))),
        }
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

    async fn execute(&self, args: Value, context: &ToolExecutionContext) -> AppResult<ToolResult> {
        let request = build_universal_request("admin_delete_system_coach", &args, context);
        let executor = UniversalExecutor::new(context.resources.clone());

        match handle_admin_delete_system_coach(&executor, request).await {
            Ok(response) => Ok(convert_to_tool_result(response)),
            Err(e) => Ok(ToolResult::error(json!({ "error": e.to_string() }))),
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

    async fn execute(&self, args: Value, context: &ToolExecutionContext) -> AppResult<ToolResult> {
        let request = build_universal_request("admin_assign_coach", &args, context);
        let executor = UniversalExecutor::new(context.resources.clone());

        match handle_admin_assign_coach(&executor, request).await {
            Ok(response) => Ok(convert_to_tool_result(response)),
            Err(e) => Ok(ToolResult::error(json!({ "error": e.to_string() }))),
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

    async fn execute(&self, args: Value, context: &ToolExecutionContext) -> AppResult<ToolResult> {
        let request = build_universal_request("admin_unassign_coach", &args, context);
        let executor = UniversalExecutor::new(context.resources.clone());

        match handle_admin_unassign_coach(&executor, request).await {
            Ok(response) => Ok(convert_to_tool_result(response)),
            Err(e) => Ok(ToolResult::error(json!({ "error": e.to_string() }))),
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

    async fn execute(&self, args: Value, context: &ToolExecutionContext) -> AppResult<ToolResult> {
        let request = build_universal_request("admin_list_coach_assignments", &args, context);
        let executor = UniversalExecutor::new(context.resources.clone());

        match handle_admin_list_coach_assignments(&executor, request).await {
            Ok(response) => Ok(convert_to_tool_result(response)),
            Err(e) => Ok(ToolResult::error(json!({ "error": e.to_string() }))),
        }
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
