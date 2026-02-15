// ABOUTME: Database parity tests ensuring SQLite and PostgreSQL implementations behave identically
// ABOUTME: Tests that both database backends return equivalent results for Tool Selection and Chat
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]
#![cfg(feature = "postgresql")]

use chrono::Utc;
use pierre_mcp_server::{
    database_plugins::{factory::Database, DatabaseProvider},
    models::{Tenant, TenantId, TenantPlan, ToolCategory, User, UserStatus, UserTier},
    permissions::UserRole,
};
use std::sync::Arc;
use uuid::Uuid;

mod common;

// ============================================================================
// Tool Selection Parity Tests
// ============================================================================

/// Test that both `SQLite` and `PostgreSQL` return the same tool catalog
#[tokio::test]
async fn test_parity_tool_catalog() {
    let Some((sqlite_db, pg_db)) = create_both_databases().await else {
        eprintln!("Skipping parity test: PostgreSQL not available");
        return;
    };

    let sqlite_catalog = sqlite_db
        .get_tool_catalog()
        .await
        .expect("SQLite: Failed to get catalog");

    let pg_catalog = pg_db
        .get_tool_catalog()
        .await
        .expect("PostgreSQL: Failed to get catalog");

    // Both should return the same number of tools
    assert_eq!(
        sqlite_catalog.len(),
        pg_catalog.len(),
        "Tool catalog count should match: SQLite={}, PostgreSQL={}",
        sqlite_catalog.len(),
        pg_catalog.len()
    );

    // Compare tool names (both sorted for deterministic comparison)
    let mut sqlite_names: Vec<_> = sqlite_catalog.iter().map(|t| &t.tool_name).collect();
    let mut pg_names: Vec<_> = pg_catalog.iter().map(|t| &t.tool_name).collect();
    sqlite_names.sort();
    pg_names.sort();

    assert_eq!(
        sqlite_names, pg_names,
        "Tool names should match between SQLite and PostgreSQL"
    );
}

/// Test that both backends return the same tool entry by name
#[tokio::test]
async fn test_parity_get_tool_catalog_entry() {
    let Some((sqlite_db, pg_db)) = create_both_databases().await else {
        eprintln!("Skipping parity test: PostgreSQL not available");
        return;
    };

    let tool_name = "get_activities";

    let sqlite_entry = sqlite_db
        .get_tool_catalog_entry(tool_name)
        .await
        .expect("SQLite: Failed to get entry");

    let pg_entry = pg_db
        .get_tool_catalog_entry(tool_name)
        .await
        .expect("PostgreSQL: Failed to get entry");

    // Both should find the tool
    assert!(sqlite_entry.is_some(), "SQLite should find {tool_name}");
    assert!(pg_entry.is_some(), "PostgreSQL should find {tool_name}");

    let sqlite_entry = sqlite_entry.unwrap();
    let pg_entry = pg_entry.unwrap();

    // Compare key fields
    assert_eq!(
        sqlite_entry.tool_name, pg_entry.tool_name,
        "Tool name should match"
    );
    assert_eq!(
        sqlite_entry.display_name, pg_entry.display_name,
        "Display name should match"
    );
    assert_eq!(
        sqlite_entry.category, pg_entry.category,
        "Category should match"
    );
    assert_eq!(
        sqlite_entry.min_plan, pg_entry.min_plan,
        "Min plan should match"
    );
    assert_eq!(
        sqlite_entry.is_enabled_by_default, pg_entry.is_enabled_by_default,
        "Enabled by default should match"
    );
}

/// Test that both filter by category the same way
#[tokio::test]
async fn test_parity_tools_by_category() {
    let Some((sqlite_db, pg_db)) = create_both_databases().await else {
        eprintln!("Skipping parity test: PostgreSQL not available");
        return;
    };

    for category in [
        ToolCategory::Fitness,
        ToolCategory::Analysis,
        ToolCategory::Nutrition,
        ToolCategory::Configuration,
    ] {
        let sqlite_tools = sqlite_db
            .get_tools_by_category(category)
            .await
            .expect("SQLite: Failed to get tools by category");

        let pg_tools = pg_db
            .get_tools_by_category(category)
            .await
            .expect("PostgreSQL: Failed to get tools by category");

        assert_eq!(
            sqlite_tools.len(),
            pg_tools.len(),
            "Category {:?} tool count should match: SQLite={}, PostgreSQL={}",
            category,
            sqlite_tools.len(),
            pg_tools.len()
        );
    }
}

/// Test that both filter by plan the same way
#[tokio::test]
async fn test_parity_tools_by_min_plan() {
    let Some((sqlite_db, pg_db)) = create_both_databases().await else {
        eprintln!("Skipping parity test: PostgreSQL not available");
        return;
    };

    for plan in [
        TenantPlan::Starter,
        TenantPlan::Professional,
        TenantPlan::Enterprise,
    ] {
        let sqlite_tools = sqlite_db
            .get_tools_by_min_plan(plan)
            .await
            .expect("SQLite: Failed to get tools by plan");

        let pg_tools = pg_db
            .get_tools_by_min_plan(plan)
            .await
            .expect("PostgreSQL: Failed to get tools by plan");

        assert_eq!(
            sqlite_tools.len(),
            pg_tools.len(),
            "Plan {:?} tool count should match: SQLite={}, PostgreSQL={}",
            plan,
            sqlite_tools.len(),
            pg_tools.len()
        );
    }
}

/// Test that tenant tool override operations behave identically
#[tokio::test]
async fn test_parity_tenant_tool_overrides() {
    let Some((sqlite_db, pg_db)) = create_both_databases().await else {
        eprintln!("Skipping parity test: PostgreSQL not available");
        return;
    };

    // Create identical tenants and users in both databases
    let tenant_id = TenantId::new();
    let sqlite_user_id = create_test_user(&sqlite_db).await;
    let pg_user_id = create_test_user(&pg_db).await;

    create_test_tenant(&sqlite_db, tenant_id, sqlite_user_id).await;
    create_test_tenant(&pg_db, tenant_id, pg_user_id).await;

    // Both should start with empty overrides
    let sqlite_overrides = sqlite_db
        .get_tenant_tool_overrides(tenant_id)
        .await
        .expect("SQLite: Failed to get overrides");
    let pg_overrides = pg_db
        .get_tenant_tool_overrides(tenant_id)
        .await
        .expect("PostgreSQL: Failed to get overrides");

    assert!(
        sqlite_overrides.is_empty(),
        "SQLite should have no overrides"
    );
    assert!(
        pg_overrides.is_empty(),
        "PostgreSQL should have no overrides"
    );

    // Create same override in both
    let sqlite_created = sqlite_db
        .upsert_tenant_tool_override(
            tenant_id,
            "get_activities",
            false,
            Some(sqlite_user_id),
            Some("Test reason".to_owned()),
        )
        .await
        .expect("SQLite: Failed to create override");

    let pg_created = pg_db
        .upsert_tenant_tool_override(
            tenant_id,
            "get_activities",
            false,
            Some(pg_user_id),
            Some("Test reason".to_owned()),
        )
        .await
        .expect("PostgreSQL: Failed to create override");

    // Verify same tool_name and is_enabled (upsert succeeded via expect() above)
    assert_eq!(sqlite_created.tool_name, pg_created.tool_name);
    assert_eq!(sqlite_created.is_enabled, pg_created.is_enabled);

    // Delete in both
    let sqlite_deleted = sqlite_db
        .delete_tenant_tool_override(tenant_id, "get_activities")
        .await
        .expect("SQLite: Failed to delete override");
    let pg_deleted = pg_db
        .delete_tenant_tool_override(tenant_id, "get_activities")
        .await
        .expect("PostgreSQL: Failed to delete override");

    assert!(sqlite_deleted, "SQLite delete should return true");
    assert!(pg_deleted, "PostgreSQL delete should return true");
}

// ============================================================================
// Chat Parity Tests
// ============================================================================

/// Test that conversation creation behaves identically
#[tokio::test]
async fn test_parity_chat_create_conversation() {
    let Some((sqlite_db, pg_db)) = create_both_databases().await else {
        eprintln!("Skipping parity test: PostgreSQL not available");
        return;
    };

    let sqlite_user_id = create_test_user(&sqlite_db).await;
    let pg_user_id = create_test_user(&pg_db).await;
    let tenant_id = TenantId::new();

    let sqlite_conv = sqlite_db
        .chat_create_conversation(
            &sqlite_user_id.to_string(),
            tenant_id,
            "Test Chat",
            "gpt-4",
            Some("System prompt"),
        )
        .await
        .expect("SQLite: Failed to create conversation");

    let pg_conv = pg_db
        .chat_create_conversation(
            &pg_user_id.to_string(),
            tenant_id,
            "Test Chat",
            "gpt-4",
            Some("System prompt"),
        )
        .await
        .expect("PostgreSQL: Failed to create conversation");

    // Compare structure (IDs will differ)
    assert_eq!(sqlite_conv.title, pg_conv.title, "Titles should match");
    assert_eq!(sqlite_conv.model, pg_conv.model, "Models should match");
    assert_eq!(
        sqlite_conv.system_prompt, pg_conv.system_prompt,
        "System prompts should match"
    );
    assert_eq!(
        sqlite_conv.total_tokens, pg_conv.total_tokens,
        "Token counts should match"
    );
}

/// Test that message operations behave identically
#[tokio::test]
async fn test_parity_chat_messages() {
    let Some((sqlite_db, pg_db)) = create_both_databases().await else {
        eprintln!("Skipping parity test: PostgreSQL not available");
        return;
    };

    let sqlite_user_id = create_test_user(&sqlite_db).await;
    let pg_user_id = create_test_user(&pg_db).await;
    let tenant_id = TenantId::new();

    // Create conversations
    let sqlite_conv = sqlite_db
        .chat_create_conversation(
            &sqlite_user_id.to_string(),
            tenant_id,
            "Message Test",
            "gpt-4",
            None,
        )
        .await
        .expect("SQLite: Failed to create conversation");

    let pg_conv = pg_db
        .chat_create_conversation(
            &pg_user_id.to_string(),
            tenant_id,
            "Message Test",
            "gpt-4",
            None,
        )
        .await
        .expect("PostgreSQL: Failed to create conversation");

    // Add same messages to both
    let messages = vec![
        ("user", "Hello!", None, None),
        ("assistant", "Hi there!", Some(10u32), Some("stop")),
        ("user", "How are you?", None, None),
    ];

    let sqlite_uid = sqlite_user_id.to_string();
    let pg_uid = pg_user_id.to_string();

    for (role, content, tokens, finish) in &messages {
        sqlite_db
            .chat_add_message(
                &sqlite_conv.id,
                &sqlite_uid,
                role,
                content,
                *tokens,
                *finish,
            )
            .await
            .expect("SQLite: Failed to add message");

        pg_db
            .chat_add_message(&pg_conv.id, &pg_uid, role, content, *tokens, *finish)
            .await
            .expect("PostgreSQL: Failed to add message");
    }

    // Get all messages
    let sqlite_messages = sqlite_db
        .chat_get_messages(&sqlite_conv.id, &sqlite_uid)
        .await
        .expect("SQLite: Failed to get messages");

    let pg_messages = pg_db
        .chat_get_messages(&pg_conv.id, &pg_uid)
        .await
        .expect("PostgreSQL: Failed to get messages");

    assert_eq!(
        sqlite_messages.len(),
        pg_messages.len(),
        "Message count should match"
    );

    // Compare message content
    for (sqlite_msg, pg_msg) in sqlite_messages.iter().zip(pg_messages.iter()) {
        assert_eq!(sqlite_msg.role, pg_msg.role, "Roles should match");
        assert_eq!(sqlite_msg.content, pg_msg.content, "Content should match");
        assert_eq!(
            sqlite_msg.token_count, pg_msg.token_count,
            "Token counts should match"
        );
        assert_eq!(
            sqlite_msg.finish_reason, pg_msg.finish_reason,
            "Finish reasons should match"
        );
    }

    // Compare message counts
    let sqlite_count = sqlite_db
        .chat_get_message_count(&sqlite_conv.id, &sqlite_uid)
        .await
        .expect("SQLite: Failed to get count");

    let pg_count = pg_db
        .chat_get_message_count(&pg_conv.id, &pg_uid)
        .await
        .expect("PostgreSQL: Failed to get count");

    assert_eq!(sqlite_count, pg_count, "Message counts should match");
}

/// Test that listing conversations behaves identically
#[tokio::test]
async fn test_parity_chat_list_conversations() {
    let Some((sqlite_db, pg_db)) = create_both_databases().await else {
        eprintln!("Skipping parity test: PostgreSQL not available");
        return;
    };

    let sqlite_user_id = create_test_user(&sqlite_db).await;
    let pg_user_id = create_test_user(&pg_db).await;
    let tenant_id = TenantId::new();

    // Create same conversations in both
    for i in 1..=5 {
        sqlite_db
            .chat_create_conversation(
                &sqlite_user_id.to_string(),
                tenant_id,
                &format!("Chat {i}"),
                "gpt-4",
                None,
            )
            .await
            .expect("SQLite: Failed to create conversation");

        pg_db
            .chat_create_conversation(
                &pg_user_id.to_string(),
                tenant_id,
                &format!("Chat {i}"),
                "gpt-4",
                None,
            )
            .await
            .expect("PostgreSQL: Failed to create conversation");
    }

    // Test pagination works the same
    let sqlite_list = sqlite_db
        .chat_list_conversations(&sqlite_user_id.to_string(), tenant_id, 3, 0)
        .await
        .expect("SQLite: Failed to list");

    let pg_list = pg_db
        .chat_list_conversations(&pg_user_id.to_string(), tenant_id, 3, 0)
        .await
        .expect("PostgreSQL: Failed to list");

    assert_eq!(
        sqlite_list.len(),
        pg_list.len(),
        "Pagination should return same count"
    );

    // Test delete all works the same
    let sqlite_deleted = sqlite_db
        .chat_delete_all_user_conversations(&sqlite_user_id.to_string(), tenant_id)
        .await
        .expect("SQLite: Failed to delete all");

    let pg_deleted = pg_db
        .chat_delete_all_user_conversations(&pg_user_id.to_string(), tenant_id)
        .await
        .expect("PostgreSQL: Failed to delete all");

    assert_eq!(
        sqlite_deleted, pg_deleted,
        "Delete all should remove same count"
    );
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create both `SQLite` and `PostgreSQL` test databases.
/// Returns None if `PostgreSQL` is not available.
async fn create_both_databases() -> Option<(Arc<Database>, Arc<Database>)> {
    // Create SQLite database
    let sqlite_db = common::create_test_database()
        .await
        .expect("Failed to create SQLite test database");

    // Try to create PostgreSQL database
    let pg_db = match common::IsolatedPostgresDb::new().await {
        Ok(isolated_db) => {
            let db = isolated_db
                .get_database()
                .await
                .expect("Failed to get PostgreSQL database");
            // Wrap in Arc - note that IsolatedPostgresDb will clean up when dropped
            // but we need to keep it alive, so we leak it for the test
            Arc::new(db)
        }
        Err(e) => {
            eprintln!("PostgreSQL not available: {e}");
            return None;
        }
    };

    Some((sqlite_db, pg_db))
}

async fn create_test_user(db: &Database) -> Uuid {
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: format!("parity-test-{user_id}@example.com"),
        display_name: Some("Parity Test User".to_owned()),
        password_hash: "test_hash".to_owned(),
        tier: UserTier::Starter,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        role: UserRole::User,
        approved_by: None,
        approved_at: Some(Utc::now()),
        created_at: Utc::now(),
        last_active: Utc::now(),
        strava_token: None,
        fitbit_token: None,
        firebase_uid: None,
        auth_provider: String::new(),
    };

    db.create_user(&user).await.expect("Failed to create user");
    user_id
}

async fn create_test_tenant(db: &Database, tenant_id: TenantId, owner_id: Uuid) {
    let tenant = Tenant {
        id: tenant_id,
        name: "Parity Test Tenant".to_owned(),
        slug: format!("parity-test-{tenant_id}"),
        domain: None,
        plan: "starter".to_owned(),
        owner_user_id: owner_id,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    db.create_tenant(&tenant)
        .await
        .expect("Failed to create tenant");
}
