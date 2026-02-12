// ABOUTME: Integration tests for ToolSelectionService per-tenant tool configuration
// ABOUTME: Tests effective tools computation, tenant fallback, overrides, and caching
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::{
    config::ToolSelectionConfig,
    database_plugins::factory::Database,
    mcp::tool_selection::ToolSelectionService,
    models::{TenantId, ToolEnablementSource},
};
use std::sync::Arc;

mod common;

/// Create a `ToolSelectionService` with the test database
fn create_test_service(db: &Arc<Database>) -> ToolSelectionService {
    ToolSelectionService::new(Arc::clone(db))
}

/// Create a `ToolSelectionService` with specific disabled tools
fn create_test_service_with_disabled(
    db: &Arc<Database>,
    disabled_tools: Vec<String>,
) -> ToolSelectionService {
    let config = ToolSelectionConfig::with_disabled_tools(disabled_tools);
    ToolSelectionService::with_config(Arc::clone(db), config)
}

#[tokio::test]
async fn test_get_effective_tools_returns_catalog() {
    let db = Arc::new(
        common::create_test_database()
            .await
            .expect("Failed to create test database"),
    );

    let service = create_test_service(&db);

    // Use a random tenant_id (won't exist, will fallback to Enterprise)
    let tenant_id = TenantId::new();

    let tools = service
        .get_effective_tools(tenant_id)
        .await
        .expect("Failed to get effective tools");

    // Should return all tools from catalog
    assert!(!tools.is_empty(), "Should return tools from catalog");

    // All tools should be enabled (Enterprise plan fallback)
    let all_enabled = tools.iter().all(|t| t.is_enabled);
    assert!(
        all_enabled,
        "All tools should be enabled for Enterprise plan"
    );

    // All should have Default source (no overrides)
    let all_default = tools
        .iter()
        .all(|t| t.source == ToolEnablementSource::Default);
    assert!(all_default, "All tools should have Default source");
}

#[tokio::test]
async fn test_get_effective_tools_handles_missing_tenant() {
    let db = Arc::new(
        common::create_test_database()
            .await
            .expect("Failed to create test database"),
    );

    let service = create_test_service(&db);

    // Use a tenant_id that doesn't exist
    let nonexistent_tenant_id = TenantId::new();

    // Should NOT return an error - should fallback gracefully
    let result = service.get_effective_tools(nonexistent_tenant_id).await;

    assert!(
        result.is_ok(),
        "Should not error on missing tenant, got: {:?}",
        result.err()
    );

    let tools = result.unwrap();
    assert!(
        !tools.is_empty(),
        "Should return tools even for missing tenant"
    );
}

#[tokio::test]
async fn test_get_enabled_tools_filters_correctly() {
    let db = Arc::new(
        common::create_test_database()
            .await
            .expect("Failed to create test database"),
    );

    // Create service with one tool globally disabled
    let service = create_test_service_with_disabled(&db, vec!["analyze_activity".to_owned()]);

    let tenant_id = TenantId::new();

    let enabled_tools = service
        .get_enabled_tools(tenant_id)
        .await
        .expect("Failed to get enabled tools");

    // Should not contain the disabled tool
    let has_disabled = enabled_tools
        .iter()
        .any(|t| t.tool_name == "analyze_activity");
    assert!(
        !has_disabled,
        "Enabled tools should not contain globally disabled tool"
    );
}

#[tokio::test]
async fn test_is_tool_enabled_global_disabled() {
    let db = Arc::new(
        common::create_test_database()
            .await
            .expect("Failed to create test database"),
    );

    // Create service with specific tool globally disabled
    let service = create_test_service_with_disabled(&db, vec!["get_activities".to_owned()]);

    let tenant_id = TenantId::new();

    // Check globally disabled tool
    let is_enabled = service
        .is_tool_enabled(tenant_id, "get_activities")
        .await
        .expect("Failed to check tool enablement");

    assert!(!is_enabled, "Globally disabled tool should not be enabled");

    // Check that a non-disabled tool IS enabled
    let other_enabled = service
        .is_tool_enabled(tenant_id, "get_athlete")
        .await
        .expect("Failed to check tool enablement");

    assert!(other_enabled, "Non-disabled tool should be enabled");
}

#[tokio::test]
async fn test_is_tool_enabled_nonexistent_tool() {
    let db = Arc::new(
        common::create_test_database()
            .await
            .expect("Failed to create test database"),
    );

    let service = create_test_service(&db);
    let tenant_id = TenantId::new();

    // Check a tool that doesn't exist
    let result = service
        .is_tool_enabled(tenant_id, "nonexistent_tool_xyz")
        .await;

    assert!(
        result.is_err(),
        "Should error for nonexistent tool in catalog"
    );
}

#[tokio::test]
async fn test_globally_disabled_tools_getter() {
    let db = Arc::new(
        common::create_test_database()
            .await
            .expect("Failed to create test database"),
    );

    let disabled = vec!["tool_a".to_owned(), "tool_b".to_owned()];
    let service = create_test_service_with_disabled(&db, disabled);

    let returned_disabled = service.get_globally_disabled_tools();

    assert_eq!(returned_disabled.len(), 2);
    assert!(returned_disabled.contains(&"tool_a".to_owned()));
    assert!(returned_disabled.contains(&"tool_b".to_owned()));
}

#[tokio::test]
async fn test_has_globally_disabled_tools() {
    let db = Arc::new(
        common::create_test_database()
            .await
            .expect("Failed to create test database"),
    );

    // Service with no disabled tools
    let service_none = create_test_service(&db);
    assert!(
        !service_none.has_globally_disabled_tools(),
        "Should return false when no tools disabled"
    );

    // Service with disabled tools
    let service_some = create_test_service_with_disabled(&db, vec!["some_tool".to_owned()]);
    assert!(
        service_some.has_globally_disabled_tools(),
        "Should return true when tools are disabled"
    );
}

#[tokio::test]
async fn test_get_availability_summary() {
    let db = Arc::new(
        common::create_test_database()
            .await
            .expect("Failed to create test database"),
    );

    let service = create_test_service(&db);
    let tenant_id = TenantId::new();

    let summary = service
        .get_availability_summary(tenant_id)
        .await
        .expect("Failed to get availability summary");

    // Should have tools
    assert!(summary.total_tools > 0, "Should have total tools");

    // All enabled (Enterprise plan fallback)
    assert_eq!(
        summary.enabled_tools, summary.total_tools,
        "All tools should be enabled for Enterprise"
    );

    // No overrides initially
    assert_eq!(summary.overridden_tools, 0, "No overrides initially");

    // No plan restrictions for Enterprise
    assert_eq!(
        summary.plan_restricted_tools, 0,
        "No plan restrictions for Enterprise"
    );

    // Should have category breakdown
    assert!(
        !summary.by_category.is_empty(),
        "Should have category breakdown"
    );
}

#[tokio::test]
async fn test_effective_tools_source_for_global_disabled() {
    let db = Arc::new(
        common::create_test_database()
            .await
            .expect("Failed to create test database"),
    );

    let service = create_test_service_with_disabled(&db, vec!["analyze_activity".to_owned()]);

    let tenant_id = TenantId::new();

    let tools = service
        .get_effective_tools(tenant_id)
        .await
        .expect("Failed to get effective tools");

    // Find the disabled tool
    let disabled_tool = tools
        .iter()
        .find(|t| t.tool_name == "analyze_activity")
        .expect("Should find analyze_activity in catalog");

    assert!(!disabled_tool.is_enabled, "Tool should be disabled");
    assert_eq!(
        disabled_tool.source,
        ToolEnablementSource::GlobalDisabled,
        "Source should be GlobalDisabled"
    );
}

#[tokio::test]
async fn test_cache_invalidation() {
    let db = Arc::new(
        common::create_test_database()
            .await
            .expect("Failed to create test database"),
    );

    let service = create_test_service(&db);
    let tenant_id = TenantId::new();

    // First call populates cache
    let tools1 = service
        .get_effective_tools(tenant_id)
        .await
        .expect("Failed to get effective tools");

    // Invalidate cache
    service.invalidate_tenant(tenant_id).await;

    // Second call should work (cache miss, recompute)
    let tools2 = service
        .get_effective_tools(tenant_id)
        .await
        .expect("Failed to get effective tools after invalidation");

    assert_eq!(
        tools1.len(),
        tools2.len(),
        "Should return same tools after cache invalidation"
    );
}

#[tokio::test]
async fn test_get_catalog() {
    let db = Arc::new(
        common::create_test_database()
            .await
            .expect("Failed to create test database"),
    );

    let service = create_test_service(&db);

    let catalog = service.get_catalog().await.expect("Failed to get catalog");

    assert!(!catalog.is_empty(), "Catalog should not be empty");

    // Check catalog entries have required fields
    for entry in &catalog {
        assert!(!entry.tool_name.is_empty(), "Tool name should not be empty");
        assert!(
            !entry.display_name.is_empty(),
            "Display name should not be empty"
        );
    }
}
