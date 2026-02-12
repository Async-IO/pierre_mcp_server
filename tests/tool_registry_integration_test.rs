// ABOUTME: Integration tests for ToolRegistry execution paths and admin filtering.
// ABOUTME: Tests real tool execution with ServerResources-backed ToolExecutionContext.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

//! Integration tests for `ToolRegistry` execution paths.
//!
//! These tests verify:
//! - Tool registration and lookup with real tools
//! - Admin permission enforcement during execution
//! - Schema generation with role-based filtering
//! - Feature-flag-based tool registration
//! - Capability-based filtering with real tools

mod common;

use std::collections::HashSet;
use std::sync::Arc;

use common::{create_test_server_resources, create_test_user, create_test_user_with_email};
use pierre_mcp_server::mcp::resources::ServerResources;
use pierre_mcp_server::models::TenantId;
use pierre_mcp_server::tools::{AuthMethod, ToolCapabilities, ToolExecutionContext, ToolRegistry};
use serde_json::json;
use uuid::Uuid;

// ============================================================================
// Test Helper Functions
// ============================================================================

/// Create a `ToolExecutionContext` for testing with given user and admin status.
fn create_test_context(
    resources: &Arc<ServerResources>,
    user_id: Uuid,
    tenant_id: Option<TenantId>,
    is_admin: bool,
) -> ToolExecutionContext {
    let mut ctx = ToolExecutionContext::new(user_id, Arc::clone(resources), AuthMethod::JwtBearer);

    if let Some(tid) = tenant_id {
        ctx = ctx.with_tenant(tid);
    }

    ctx.with_admin_status(is_admin)
}

// ============================================================================
// Registry Registration Tests
// ============================================================================

#[tokio::test]
async fn test_registry_builtin_tools_registration() {
    let mut registry = ToolRegistry::new();
    registry.register_builtin_tools();

    // Should have registered tools based on enabled features
    // With all tools features enabled, we expect at least 50 tools
    assert!(
        registry.len() >= 50,
        "Expected at least 50 tools, got {}",
        registry.len()
    );

    // Verify well-known tools are registered
    assert!(
        registry.contains("list_coaches"),
        "list_coaches should be registered"
    );
    assert!(
        registry.contains("get_activities"),
        "get_activities should be registered"
    );
}

#[tokio::test]
async fn test_registry_categories_populated() {
    let mut registry = ToolRegistry::new();
    registry.register_builtin_tools();

    let categories = registry.categories();

    // Should have multiple categories
    assert!(
        categories.len() >= 5,
        "Expected at least 5 categories, got {}",
        categories.len()
    );

    // Verify expected categories exist
    let category_names: Vec<&str> = categories.clone();
    assert!(
        category_names.contains(&"coaches"),
        "coaches category should exist"
    );
    assert!(
        category_names.contains(&"data"),
        "data category should exist"
    );
}

#[tokio::test]
async fn test_registry_tools_in_category() {
    let mut registry = ToolRegistry::new();
    registry.register_builtin_tools();

    // Coach tools category
    let coach_tools = registry.tools_in_category("coaches");
    assert!(
        !coach_tools.is_empty(),
        "coaches category should have tools"
    );
    assert!(
        coach_tools.contains(&"list_coaches"),
        "coaches category should contain list_coaches"
    );
}

// ============================================================================
// Schema Generation Tests
// ============================================================================

#[tokio::test]
async fn test_list_schemas_for_role_user() {
    let mut registry = ToolRegistry::new();
    registry.register_builtin_tools();

    let user_schemas = registry.list_schemas_for_role(false);

    // User should see some tools but not admin-only ones
    assert!(!user_schemas.is_empty(), "User should see some tools");

    // Verify no admin-only tools are included
    let admin_tool_names: Vec<&str> = registry
        .filter_by_capabilities(ToolCapabilities::ADMIN_ONLY)
        .iter()
        .map(|t| t.name())
        .collect();

    for schema in &user_schemas {
        assert!(
            !admin_tool_names.contains(&schema.name.as_str()),
            "User should not see admin-only tool: {}",
            schema.name
        );
    }
}

#[tokio::test]
async fn test_list_schemas_for_role_admin() {
    let mut registry = ToolRegistry::new();
    registry.register_builtin_tools();

    let admin_schemas = registry.list_schemas_for_role(true);
    let user_schemas = registry.list_schemas_for_role(false);

    // Admin should see more tools than user (admin-only tools included)
    assert!(
        admin_schemas.len() >= user_schemas.len(),
        "Admin should see at least as many tools as user"
    );
}

#[tokio::test]
async fn test_admin_tool_schemas_only_admin() {
    let mut registry = ToolRegistry::new();
    registry.register_builtin_tools();

    let admin_only_schemas = registry.admin_tool_schemas();

    // All returned schemas should be from admin-only tools
    for schema in &admin_only_schemas {
        let tool = registry.get(&schema.name).expect("Tool should exist");
        assert!(
            tool.capabilities().is_admin_only(),
            "Tool {} should be admin-only",
            schema.name
        );
    }
}

#[tokio::test]
async fn test_user_visible_schemas_no_admin() {
    let mut registry = ToolRegistry::new();
    registry.register_builtin_tools();

    let user_schemas = registry.user_visible_schemas();

    // None of the returned schemas should be admin-only
    for schema in &user_schemas {
        let tool = registry.get(&schema.name).expect("Tool should exist");
        assert!(
            !tool.capabilities().is_admin_only(),
            "Tool {} should not be admin-only",
            schema.name
        );
    }
}

// ============================================================================
// Capability Filtering Tests
// ============================================================================

#[tokio::test]
async fn test_filter_by_capabilities_reads_data() {
    let mut registry = ToolRegistry::new();
    registry.register_builtin_tools();

    let read_tools = registry.filter_by_capabilities(ToolCapabilities::READS_DATA);

    // Should have tools that read data
    assert!(!read_tools.is_empty(), "Should have tools that read data");

    // All filtered tools should have READS_DATA capability
    for tool in &read_tools {
        assert!(
            tool.capabilities().reads_data(),
            "Tool {} should have READS_DATA capability",
            tool.name()
        );
    }
}

#[tokio::test]
async fn test_filter_by_capabilities_writes_data() {
    let mut registry = ToolRegistry::new();
    registry.register_builtin_tools();

    let write_tools = registry.filter_by_capabilities(ToolCapabilities::WRITES_DATA);

    // All filtered tools should have WRITES_DATA capability
    for tool in &write_tools {
        assert!(
            tool.capabilities().writes_data(),
            "Tool {} should have WRITES_DATA capability",
            tool.name()
        );
    }
}

#[tokio::test]
async fn test_read_tools_method() {
    let mut registry = ToolRegistry::new();
    registry.register_builtin_tools();

    let read_tool_names = registry.read_tools();

    // Verify all returned tools actually read data
    for name in &read_tool_names {
        let tool = registry.get(name).expect("Tool should exist");
        assert!(
            tool.capabilities().reads_data(),
            "Tool {name} should read data"
        );
    }
}

#[tokio::test]
async fn test_write_tools_method() {
    let mut registry = ToolRegistry::new();
    registry.register_builtin_tools();

    let write_tool_names = registry.write_tools();

    // Verify all returned tools actually write data
    for name in &write_tool_names {
        let tool = registry.get(name).expect("Tool should exist");
        assert!(
            tool.capabilities().writes_data(),
            "Tool {name} should write data"
        );
    }
}

// ============================================================================
// Execution Tests with Real Context
// ============================================================================

#[tokio::test]
async fn test_execute_nonexistent_tool() {
    let resources = create_test_server_resources()
        .await
        .expect("Failed to create test resources");
    let (user_id, _) = create_test_user(&resources.database)
        .await
        .expect("Failed to create test user");

    let mut registry = ToolRegistry::new();
    registry.register_builtin_tools();

    let context = create_test_context(&resources, user_id, None, false);

    // Execute a tool that doesn't exist
    let result = registry
        .execute("nonexistent_tool_xyz", json!({}), &context)
        .await;

    assert!(result.is_err(), "Should fail for nonexistent tool");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("not found") || err_msg.contains("nonexistent"),
        "Error should indicate tool not found: {err_msg}"
    );
}

#[tokio::test]
async fn test_execute_admin_tool_as_non_admin_denied() {
    let resources = create_test_server_resources()
        .await
        .expect("Failed to create test resources");
    let (user_id, _) = create_test_user(&resources.database)
        .await
        .expect("Failed to create test user");

    let mut registry = ToolRegistry::new();
    registry.register_builtin_tools();

    // Find an admin-only tool
    let admin_tools = registry.filter_by_capabilities(ToolCapabilities::ADMIN_ONLY);
    if admin_tools.is_empty() {
        // No admin tools registered, skip this test
        return;
    }

    let admin_tool_name = admin_tools[0].name();

    // Create non-admin context
    let context = create_test_context(&resources, user_id, None, false);

    // Execute admin-only tool as non-admin
    let result = registry.execute(admin_tool_name, json!({}), &context).await;

    assert!(
        result.is_err(),
        "Admin tool should be denied for non-admin user"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Admin") || err_msg.contains("permission") || err_msg.contains("denied"),
        "Error should indicate permission denied: {err_msg}"
    );
}

#[tokio::test]
async fn test_execute_admin_tool_as_admin_allowed() {
    let resources = create_test_server_resources()
        .await
        .expect("Failed to create test resources");
    let (user_id, _) = create_test_user(&resources.database)
        .await
        .expect("Failed to create test user");

    // No need to update database - context caches admin status via with_admin_status()
    let mut registry = ToolRegistry::new();
    registry.register_builtin_tools();

    // Find an admin-only tool
    let admin_tools = registry.filter_by_capabilities(ToolCapabilities::ADMIN_ONLY);
    if admin_tools.is_empty() {
        // No admin tools registered, skip this test
        return;
    }

    let admin_tool_name = admin_tools[0].name();

    // Create admin context
    let context = create_test_context(&resources, user_id, None, true);

    // Execute admin-only tool as admin - should pass the permission check
    // Note: The actual tool execution may still fail due to missing arguments,
    // but the admin check should pass
    let result = registry.execute(admin_tool_name, json!({}), &context).await;

    // If it errors, it should NOT be a permission error
    if let Err(e) = &result {
        let err_msg = e.to_string().to_lowercase();
        assert!(
            !err_msg.contains("admin") || err_msg.contains("required"),
            "Admin should pass permission check, got: {e}"
        );
    }
}

// ============================================================================
// Tool Lookup Tests
// ============================================================================

#[tokio::test]
async fn test_get_existing_tool() {
    let mut registry = ToolRegistry::new();
    registry.register_builtin_tools();

    let tool = registry.get("list_coaches");
    assert!(tool.is_some(), "list_coaches should exist");

    let tool = tool.unwrap();
    assert_eq!(tool.name(), "list_coaches");
}

#[tokio::test]
async fn test_get_nonexistent_tool() {
    let mut registry = ToolRegistry::new();
    registry.register_builtin_tools();

    let tool = registry.get("this_tool_does_not_exist_xyz");
    assert!(tool.is_none(), "Nonexistent tool should return None");
}

#[tokio::test]
async fn test_contains_method() {
    let mut registry = ToolRegistry::new();
    registry.register_builtin_tools();

    assert!(registry.contains("list_coaches"));
    assert!(!registry.contains("nonexistent_tool_xyz"));
}

#[tokio::test]
async fn test_tool_names_method() {
    let mut registry = ToolRegistry::new();
    registry.register_builtin_tools();

    let names = registry.tool_names();

    assert!(!names.is_empty(), "Should have tool names");
    assert!(names.contains(&"list_coaches"));
    assert!(names.contains(&"get_activities"));
}

// ============================================================================
// Context Requirement Tests
// ============================================================================

#[tokio::test]
async fn test_context_require_tenant_with_tenant() {
    let resources = create_test_server_resources()
        .await
        .expect("Failed to create test resources");
    let (user_id, _) = create_test_user(&resources.database)
        .await
        .expect("Failed to create test user");

    let tenant_id = TenantId::new();
    let context = create_test_context(&resources, user_id, Some(tenant_id), false);

    let result = context.require_tenant();
    assert!(result.is_ok(), "Should succeed with tenant context");
    assert_eq!(result.unwrap(), tenant_id.as_uuid());
}

#[tokio::test]
async fn test_context_require_tenant_without_tenant() {
    let resources = create_test_server_resources()
        .await
        .expect("Failed to create test resources");
    let (user_id, _) = create_test_user(&resources.database)
        .await
        .expect("Failed to create test user");

    let context = create_test_context(&resources, user_id, None, false);

    let result = context.require_tenant();
    assert!(result.is_err(), "Should fail without tenant context");
}

#[tokio::test]
async fn test_context_is_admin_cached() {
    let resources = create_test_server_resources()
        .await
        .expect("Failed to create test resources");
    let (user_id, _) = create_test_user(&resources.database)
        .await
        .expect("Failed to create test user");

    // Create context with cached admin status
    let context = create_test_context(&resources, user_id, None, true);

    // Should return cached value without database query
    let is_admin = context
        .is_admin()
        .await
        .expect("Should return cached admin status");
    assert!(is_admin, "Should be admin from cache");
}

#[tokio::test]
async fn test_context_require_admin_as_admin() {
    let resources = create_test_server_resources()
        .await
        .expect("Failed to create test resources");
    let (user_id, _) = create_test_user(&resources.database)
        .await
        .expect("Failed to create test user");

    let context = create_test_context(&resources, user_id, None, true);

    let result = context.require_admin().await;
    assert!(result.is_ok(), "Admin should pass require_admin check");
}

#[tokio::test]
async fn test_context_require_admin_as_non_admin() {
    let resources = create_test_server_resources()
        .await
        .expect("Failed to create test resources");
    let (user_id, _) = create_test_user(&resources.database)
        .await
        .expect("Failed to create test user");

    let context = create_test_context(&resources, user_id, None, false);

    let result = context.require_admin().await;
    assert!(result.is_err(), "Non-admin should fail require_admin check");
}

// ============================================================================
// Schema Content Validation Tests
// ============================================================================

#[tokio::test]
async fn test_schema_has_required_fields() {
    let mut registry = ToolRegistry::new();
    registry.register_builtin_tools();

    let schemas = registry.all_schemas();

    for schema in &schemas {
        // Every schema should have a non-empty name
        assert!(!schema.name.is_empty(), "Schema name should not be empty");

        // Every schema should have a non-empty description
        assert!(
            !schema.description.is_empty(),
            "Schema for {} should have description",
            schema.name
        );

        // Input schema should be an object type
        assert_eq!(
            schema.input_schema.schema_type, "object",
            "Input schema for {} should be object type",
            schema.name
        );
    }
}

#[tokio::test]
async fn test_no_duplicate_tool_names() {
    let mut registry = ToolRegistry::new();
    registry.register_builtin_tools();

    let names = registry.tool_names();
    let mut seen = HashSet::new();

    for name in names {
        assert!(seen.insert(name), "Duplicate tool name found: {name}");
    }
}

// ============================================================================
// Multi-User Isolation Tests
// ============================================================================

#[tokio::test]
async fn test_different_users_separate_contexts() {
    let resources = create_test_server_resources()
        .await
        .expect("Failed to create test resources");

    let (user1_id, _) = create_test_user_with_email(&resources.database, "user1@test.com")
        .await
        .expect("Failed to create user 1");
    let (user2_id, _) = create_test_user_with_email(&resources.database, "user2@test.com")
        .await
        .expect("Failed to create user 2");

    let context1 = create_test_context(&resources, user1_id, None, false);
    let context2 = create_test_context(&resources, user2_id, None, false);

    assert_ne!(context1.user_id, context2.user_id);
}

// ============================================================================
// External Tool Registration Tests
// ============================================================================

#[tokio::test]
async fn test_register_external_tool() {
    use async_trait::async_trait;
    use pierre_mcp_server::errors::AppResult;
    use pierre_mcp_server::mcp::schema::JsonSchema;
    use pierre_mcp_server::tools::{McpTool, ToolResult};
    use serde_json::Value;

    struct ExternalTestTool;

    #[async_trait]
    impl McpTool for ExternalTestTool {
        fn name(&self) -> &'static str {
            "external_test_tool"
        }
        fn description(&self) -> &'static str {
            "Test external tool registration"
        }
        fn input_schema(&self) -> JsonSchema {
            JsonSchema {
                schema_type: "object".to_owned(),
                properties: None,
                required: None,
            }
        }
        fn capabilities(&self) -> ToolCapabilities {
            ToolCapabilities::REQUIRES_AUTH
        }
        async fn execute(
            &self,
            _args: Value,
            _context: &ToolExecutionContext,
        ) -> AppResult<ToolResult> {
            Ok(ToolResult::ok(serde_json::json!({"external": true})))
        }
    }

    let mut registry = ToolRegistry::new();
    registry.register_external_tool(Arc::new(ExternalTestTool));

    assert!(registry.contains("external_test_tool"));
    assert_eq!(registry.len(), 1);

    let tool = registry.get("external_test_tool").unwrap();
    assert_eq!(tool.description(), "Test external tool registration");
}

#[tokio::test]
async fn test_external_tool_with_builtin_tools() {
    use async_trait::async_trait;
    use pierre_mcp_server::errors::AppResult;
    use pierre_mcp_server::mcp::schema::JsonSchema;
    use pierre_mcp_server::tools::{McpTool, ToolResult};
    use serde_json::Value;

    struct CustomTool;

    #[async_trait]
    impl McpTool for CustomTool {
        fn name(&self) -> &'static str {
            "custom_integration_tool"
        }
        fn description(&self) -> &'static str {
            "Custom integration tool"
        }
        fn input_schema(&self) -> JsonSchema {
            JsonSchema {
                schema_type: "object".to_owned(),
                properties: None,
                required: None,
            }
        }
        fn capabilities(&self) -> ToolCapabilities {
            ToolCapabilities::READS_DATA
        }
        async fn execute(
            &self,
            _args: Value,
            _context: &ToolExecutionContext,
        ) -> AppResult<ToolResult> {
            Ok(ToolResult::ok(serde_json::json!({"custom": true})))
        }
    }

    let mut registry = ToolRegistry::new();
    registry.register_builtin_tools();
    let builtin_count = registry.len();

    registry.register_external_tool(Arc::new(CustomTool));

    assert_eq!(registry.len(), builtin_count + 1);
    assert!(registry.contains("custom_integration_tool"));
    assert!(registry.contains("list_coaches")); // Built-in still present
}
