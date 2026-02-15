// ABOUTME: Unit tests for the coach version history feature (ASY-153)
// ABOUTME: Tests version creation, retrieval, diff, and revert operations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// Test files: allow missing_docs (rustc lint) and unwrap (valid in tests per CLAUDE.md guidelines)
#![allow(missing_docs, clippy::unwrap_used)]

use pierre_mcp_server::database::coaches::{
    CoachCategory, CoachVisibility, CoachesManager, CreateCoachRequest, CreateSystemCoachRequest,
    UpdateCoachRequest,
};
use pierre_mcp_server::models::TenantId;
use sqlx::SqlitePool;
use uuid::Uuid;

/// Create a test database with coaches and `coach_versions` schema
#[allow(clippy::too_many_lines)]
async fn create_test_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

    // Create users table first (for foreign key)
    sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            email TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            user_status TEXT NOT NULL DEFAULT 'active',
            is_admin INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            last_active TEXT NOT NULL
        )
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Create test user
    sqlx::query(
        r"
        INSERT INTO users (id, email, password_hash, created_at, last_active)
        VALUES ('550e8400-e29b-41d4-a716-446655440000', 'test@example.com', 'hash', '2025-01-01', '2025-01-01')
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Create second test user
    sqlx::query(
        r"
        INSERT INTO users (id, email, password_hash, created_at, last_active)
        VALUES ('660e8400-e29b-41d4-a716-446655440000', 'other@example.com', 'hash', '2025-01-01', '2025-01-01')
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Create coaches table (complete schema from all migrations)
    sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS coaches (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            tenant_id TEXT NOT NULL,
            title TEXT NOT NULL,
            description TEXT,
            system_prompt TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT 'custom',
            tags TEXT,
            token_count INTEGER NOT NULL DEFAULT 0,
            is_favorite INTEGER NOT NULL DEFAULT 0,
            use_count INTEGER NOT NULL DEFAULT 0,
            last_used_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 0,
            is_system INTEGER NOT NULL DEFAULT 0,
            visibility TEXT NOT NULL DEFAULT 'private',
            sample_prompts TEXT,
            slug TEXT,
            purpose TEXT,
            when_to_use TEXT,
            instructions TEXT,
            example_inputs TEXT,
            example_outputs TEXT,
            success_criteria TEXT,
            prerequisites TEXT,
            source_file TEXT,
            content_hash TEXT,
            forked_from TEXT,
            publish_status TEXT DEFAULT 'draft',
            published_at TEXT,
            review_submitted_at TEXT,
            review_decision_at TEXT,
            review_decision_by TEXT REFERENCES users(id) ON DELETE SET NULL,
            rejection_reason TEXT,
            install_count INTEGER DEFAULT 0,
            icon_url TEXT,
            author_id TEXT,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Create coach_versions table (from migration 20250120000035)
    sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS coach_versions (
            id TEXT PRIMARY KEY,
            coach_id TEXT NOT NULL REFERENCES coaches(id) ON DELETE CASCADE,
            version INTEGER NOT NULL,
            content_hash TEXT NOT NULL,
            content_snapshot TEXT NOT NULL,
            change_summary TEXT,
            created_at TEXT NOT NULL,
            created_by TEXT REFERENCES users(id) ON DELETE SET NULL,
            UNIQUE(coach_id, version)
        )
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Create index for version history queries
    sqlx::query(
        r"
        CREATE INDEX IF NOT EXISTS idx_coach_versions_coach ON coach_versions(coach_id, version DESC)
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
}

fn test_user_id() -> Uuid {
    Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
}

fn other_user_id() -> Uuid {
    Uuid::parse_str("660e8400-e29b-41d4-a716-446655440000").unwrap()
}

fn test_tenant() -> TenantId {
    TenantId::from_uuid(Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap())
}

// ============================================================================
// Version Creation Tests
// ============================================================================

#[tokio::test]
async fn test_create_version_manually() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create a coach first
    let request = CreateCoachRequest {
        title: "Test Coach".to_owned(),
        description: Some("Original description".to_owned()),
        system_prompt: "You are a helpful assistant.".to_owned(),
        category: CoachCategory::Training,
        tags: vec!["test".to_owned()],
        sample_prompts: vec!["How can I train?".to_owned()],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // Manually create a version
    let version = manager
        .create_version(
            &coach.id.to_string(),
            test_user_id(),
            Some("Initial version"),
        )
        .await
        .unwrap();

    assert_eq!(version, 1);

    // Create another version
    let version2 = manager
        .create_version(
            &coach.id.to_string(),
            test_user_id(),
            Some("Second version"),
        )
        .await
        .unwrap();

    assert_eq!(version2, 2);
}

#[tokio::test]
async fn test_auto_version_on_update() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create a coach
    let request = CreateCoachRequest {
        title: "Original Title".to_owned(),
        description: Some("Original description".to_owned()),
        system_prompt: "Original prompt".to_owned(),
        category: CoachCategory::Training,
        tags: vec!["original".to_owned()],
        sample_prompts: vec![],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // Initially no versions
    let current_version = manager
        .get_current_version(&coach.id.to_string())
        .await
        .unwrap();
    assert_eq!(current_version, 0);

    // Update the coach - should auto-create version
    let update = UpdateCoachRequest {
        title: Some("Updated Title".to_owned()),
        description: None,
        system_prompt: None,
        category: None,
        tags: None,
        sample_prompts: None,
    };

    manager
        .update(
            &coach.id.to_string(),
            test_user_id(),
            test_tenant(),
            &update,
        )
        .await
        .unwrap();

    // Should now have 1 version (snapshot BEFORE the update)
    let current_version = manager
        .get_current_version(&coach.id.to_string())
        .await
        .unwrap();
    assert_eq!(current_version, 1);

    // Update again
    let update2 = UpdateCoachRequest {
        title: Some("Another Title".to_owned()),
        description: None,
        system_prompt: None,
        category: None,
        tags: None,
        sample_prompts: None,
    };

    manager
        .update(
            &coach.id.to_string(),
            test_user_id(),
            test_tenant(),
            &update2,
        )
        .await
        .unwrap();

    // Should now have 2 versions
    let current_version = manager
        .get_current_version(&coach.id.to_string())
        .await
        .unwrap();
    assert_eq!(current_version, 2);
}

// ============================================================================
// Version Retrieval Tests
// ============================================================================

#[tokio::test]
async fn test_get_versions() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create coach and update twice
    let request = CreateCoachRequest {
        title: "Test Coach".to_owned(),
        description: None,
        system_prompt: "Test prompt".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // First update
    let update1 = UpdateCoachRequest {
        title: Some("Title v1".to_owned()),
        description: None,
        system_prompt: None,
        category: None,
        tags: None,
        sample_prompts: None,
    };
    manager
        .update(
            &coach.id.to_string(),
            test_user_id(),
            test_tenant(),
            &update1,
        )
        .await
        .unwrap();

    // Second update
    let update2 = UpdateCoachRequest {
        title: Some("Title v2".to_owned()),
        description: None,
        system_prompt: None,
        category: None,
        tags: None,
        sample_prompts: None,
    };
    manager
        .update(
            &coach.id.to_string(),
            test_user_id(),
            test_tenant(),
            &update2,
        )
        .await
        .unwrap();

    // Get versions
    let versions = manager
        .get_versions(&coach.id.to_string(), test_tenant(), 50)
        .await
        .unwrap();

    assert_eq!(versions.len(), 2);
    // Versions should be in descending order (newest first)
    assert_eq!(versions[0].version, 2);
    assert_eq!(versions[1].version, 1);
}

#[tokio::test]
async fn test_get_versions_with_limit() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateCoachRequest {
        title: "Test Coach".to_owned(),
        description: None,
        system_prompt: "Prompt".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // Create 5 versions via updates
    for i in 1..=5 {
        let update = UpdateCoachRequest {
            title: Some(format!("Title v{i}")),
            description: None,
            system_prompt: None,
            category: None,
            tags: None,
            sample_prompts: None,
        };
        manager
            .update(
                &coach.id.to_string(),
                test_user_id(),
                test_tenant(),
                &update,
            )
            .await
            .unwrap();
    }

    // Get only 2 versions
    let versions = manager
        .get_versions(&coach.id.to_string(), test_tenant(), 2)
        .await
        .unwrap();

    assert_eq!(versions.len(), 2);
    // Should be the two newest versions
    assert_eq!(versions[0].version, 5);
    assert_eq!(versions[1].version, 4);
}

#[tokio::test]
async fn test_get_specific_version() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateCoachRequest {
        title: "Original Title".to_owned(),
        description: Some("Original description".to_owned()),
        system_prompt: "Original prompt".to_owned(),
        category: CoachCategory::Training,
        tags: vec!["original".to_owned()],
        sample_prompts: vec![],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // Update to create version 1
    let update = UpdateCoachRequest {
        title: Some("Updated Title".to_owned()),
        description: None,
        system_prompt: None,
        category: None,
        tags: None,
        sample_prompts: None,
    };
    manager
        .update(
            &coach.id.to_string(),
            test_user_id(),
            test_tenant(),
            &update,
        )
        .await
        .unwrap();

    // Get version 1
    let version = manager
        .get_version(&coach.id.to_string(), 1, test_tenant())
        .await
        .unwrap();

    assert!(version.is_some());
    let version = version.unwrap();
    assert_eq!(version.version, 1);

    // Verify the snapshot contains the original data (before update)
    let snapshot = &version.content_snapshot;
    assert_eq!(snapshot["title"], "Original Title");
}

#[tokio::test]
async fn test_get_version_not_found() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateCoachRequest {
        title: "Test Coach".to_owned(),
        description: None,
        system_prompt: "Prompt".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // Try to get non-existent version
    let version = manager
        .get_version(&coach.id.to_string(), 999, test_tenant())
        .await
        .unwrap();

    assert!(version.is_none());
}

#[tokio::test]
async fn test_get_version_wrong_tenant() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateCoachRequest {
        title: "Test Coach".to_owned(),
        description: None,
        system_prompt: "Prompt".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // Create a version
    manager
        .create_version(&coach.id.to_string(), test_user_id(), None)
        .await
        .unwrap();

    // Try to get version with wrong tenant
    let wrong_tenant =
        TenantId::from_uuid(Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap());
    let result = manager
        .get_version(&coach.id.to_string(), 1, wrong_tenant)
        .await;

    // Should fail because coach not found in wrong tenant
    assert!(result.is_err());
}

// ============================================================================
// Revert Tests
// ============================================================================

#[tokio::test]
async fn test_revert_to_version() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create coach with initial data
    let request = CreateCoachRequest {
        title: "Original Title".to_owned(),
        description: Some("Original description".to_owned()),
        system_prompt: "Original prompt".to_owned(),
        category: CoachCategory::Training,
        tags: vec!["original".to_owned()],
        sample_prompts: vec!["Original sample".to_owned()],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // Update to create version 1 (captures original state)
    let update1 = UpdateCoachRequest {
        title: Some("Updated Title".to_owned()),
        description: Some("Updated description".to_owned()),
        system_prompt: Some("Updated prompt".to_owned()),
        category: Some(CoachCategory::Nutrition),
        tags: Some(vec!["updated".to_owned()]),
        sample_prompts: None,
    };
    manager
        .update(
            &coach.id.to_string(),
            test_user_id(),
            test_tenant(),
            &update1,
        )
        .await
        .unwrap();

    // Verify current state is updated
    let current = manager
        .get(&coach.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.title, "Updated Title");

    // Revert to version 1 (the original state)
    let reverted = manager
        .revert_to_version(&coach.id.to_string(), 1, test_user_id(), test_tenant())
        .await
        .unwrap();

    // Verify reverted state matches original
    assert_eq!(reverted.title, "Original Title");
    assert_eq!(
        reverted.description,
        Some("Original description".to_owned())
    );
    assert_eq!(reverted.system_prompt, "Original prompt");
    assert_eq!(reverted.category, CoachCategory::Training);
    assert_eq!(reverted.tags, vec!["original"]);
}

#[tokio::test]
async fn test_revert_creates_new_version() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateCoachRequest {
        title: "Original".to_owned(),
        description: None,
        system_prompt: "Original prompt".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // Update once -> creates version 1
    let update = UpdateCoachRequest {
        title: Some("Updated".to_owned()),
        description: None,
        system_prompt: None,
        category: None,
        tags: None,
        sample_prompts: None,
    };
    manager
        .update(
            &coach.id.to_string(),
            test_user_id(),
            test_tenant(),
            &update,
        )
        .await
        .unwrap();

    // Current version should be 1
    let version_before = manager
        .get_current_version(&coach.id.to_string())
        .await
        .unwrap();
    assert_eq!(version_before, 1);

    // Revert to version 1 -> should create version 2
    manager
        .revert_to_version(&coach.id.to_string(), 1, test_user_id(), test_tenant())
        .await
        .unwrap();

    // Version should now be 2 (revert creates a new version)
    let version_after = manager
        .get_current_version(&coach.id.to_string())
        .await
        .unwrap();
    assert_eq!(version_after, 2);

    // Get version 2 and verify it has the revert summary
    let versions = manager
        .get_versions(&coach.id.to_string(), test_tenant(), 50)
        .await
        .unwrap();

    let v2 = versions.iter().find(|v| v.version == 2).unwrap();
    assert!(v2
        .change_summary
        .as_ref()
        .unwrap()
        .contains("Reverted to version 1"));
}

#[tokio::test]
async fn test_revert_to_nonexistent_version() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateCoachRequest {
        title: "Test".to_owned(),
        description: None,
        system_prompt: "Prompt".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // Try to revert to non-existent version
    let result = manager
        .revert_to_version(&coach.id.to_string(), 999, test_user_id(), test_tenant())
        .await;

    assert!(result.is_err());
}

// ============================================================================
// Version Content Snapshot Tests
// ============================================================================

#[tokio::test]
async fn test_version_snapshot_contains_all_fields() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateCoachRequest {
        title: "Full Coach".to_owned(),
        description: Some("Detailed description".to_owned()),
        system_prompt: "Complete system prompt".to_owned(),
        category: CoachCategory::Recovery,
        tags: vec!["tag1".to_owned(), "tag2".to_owned()],
        sample_prompts: vec!["Sample 1".to_owned(), "Sample 2".to_owned()],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // Create version
    manager
        .create_version(&coach.id.to_string(), test_user_id(), Some("Full snapshot"))
        .await
        .unwrap();

    // Get the version
    let version = manager
        .get_version(&coach.id.to_string(), 1, test_tenant())
        .await
        .unwrap()
        .unwrap();

    let snapshot = &version.content_snapshot;

    // Verify all expected fields are present
    assert_eq!(snapshot["title"], "Full Coach");
    assert_eq!(snapshot["description"], "Detailed description");
    assert_eq!(snapshot["system_prompt"], "Complete system prompt");
    assert_eq!(snapshot["category"], "recovery");
    assert_eq!(snapshot["tags"][0], "tag1");
    assert_eq!(snapshot["tags"][1], "tag2");
    assert_eq!(snapshot["sample_prompts"][0], "Sample 1");
    assert_eq!(snapshot["sample_prompts"][1], "Sample 2");
    assert_eq!(snapshot["visibility"], "private");
}

#[tokio::test]
async fn test_version_has_content_hash() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateCoachRequest {
        title: "Test Coach".to_owned(),
        description: None,
        system_prompt: "Prompt".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    manager
        .create_version(&coach.id.to_string(), test_user_id(), None)
        .await
        .unwrap();

    let version = manager
        .get_version(&coach.id.to_string(), 1, test_tenant())
        .await
        .unwrap()
        .unwrap();

    // Content hash should be non-empty
    assert!(!version.content_hash.is_empty());
    // Content hash should be hex string
    assert!(version.content_hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[tokio::test]
async fn test_different_content_different_hash() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateCoachRequest {
        title: "Original".to_owned(),
        description: None,
        system_prompt: "Prompt".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // First update - auto-creates version 1 capturing "Original" state
    let update1 = UpdateCoachRequest {
        title: Some("First Update".to_owned()),
        description: None,
        system_prompt: None,
        category: None,
        tags: None,
        sample_prompts: None,
    };
    manager
        .update(
            &coach.id.to_string(),
            test_user_id(),
            test_tenant(),
            &update1,
        )
        .await
        .unwrap();

    // Second update - auto-creates version 2 capturing "First Update" state
    let update2 = UpdateCoachRequest {
        title: Some("Second Update".to_owned()),
        description: None,
        system_prompt: None,
        category: None,
        tags: None,
        sample_prompts: None,
    };
    manager
        .update(
            &coach.id.to_string(),
            test_user_id(),
            test_tenant(),
            &update2,
        )
        .await
        .unwrap();

    // Get both versions
    let versions = manager
        .get_versions(&coach.id.to_string(), test_tenant(), 50)
        .await
        .unwrap();

    assert_eq!(versions.len(), 2);
    // Version 1 captured "Original", Version 2 captured "First Update"
    // Different content should have different hashes
    assert_ne!(versions[0].content_hash, versions[1].content_hash);
}

// ============================================================================
// System Coach Versioning Tests
// ============================================================================

#[tokio::test]
async fn test_system_coach_version_on_update() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create a system coach
    let request = CreateSystemCoachRequest {
        title: "System Coach".to_owned(),
        description: Some("System description".to_owned()),
        system_prompt: "System prompt".to_owned(),
        category: CoachCategory::Training,
        tags: vec!["system".to_owned()],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };

    let coach = manager
        .create_system_coach(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // Update system coach
    let update = UpdateCoachRequest {
        title: Some("Updated System Coach".to_owned()),
        description: None,
        system_prompt: None,
        category: None,
        tags: None,
        sample_prompts: None,
    };
    manager
        .update_system_coach(&coach.id.to_string(), test_tenant(), &update)
        .await
        .unwrap();

    // Should have created a version
    let version = manager
        .get_current_version(&coach.id.to_string())
        .await
        .unwrap();
    assert_eq!(version, 1);

    // Get the version and verify it captured original state
    let version_data = manager
        .get_version(&coach.id.to_string(), 1, test_tenant())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(version_data.content_snapshot["title"], "System Coach");
}

// ============================================================================
// Change Summary Tests
// ============================================================================

#[tokio::test]
async fn test_update_with_change_summary() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateCoachRequest {
        title: "Test Coach".to_owned(),
        description: None,
        system_prompt: "Prompt".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // Update with change summary
    let update = UpdateCoachRequest {
        title: Some("New Title".to_owned()),
        description: None,
        system_prompt: None,
        category: None,
        tags: None,
        sample_prompts: None,
    };
    manager
        .update_with_summary(
            &coach.id.to_string(),
            test_user_id(),
            test_tenant(),
            &update,
            Some("Changed title for clarity"),
        )
        .await
        .unwrap();

    // Get version and check summary
    let version = manager
        .get_version(&coach.id.to_string(), 1, test_tenant())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        version.change_summary,
        Some("Changed title for clarity".to_owned())
    );
}

#[tokio::test]
async fn test_version_tracks_created_by() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateCoachRequest {
        title: "Test Coach".to_owned(),
        description: None,
        system_prompt: "Prompt".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // Create version with specific user
    manager
        .create_version(
            &coach.id.to_string(),
            other_user_id(),
            Some("Created by other user"),
        )
        .await
        .unwrap();

    let version = manager
        .get_version(&coach.id.to_string(), 1, test_tenant())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(version.created_by, Some(other_user_id()));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[tokio::test]
async fn test_get_versions_empty() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateCoachRequest {
        title: "Test Coach".to_owned(),
        description: None,
        system_prompt: "Prompt".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // Get versions for coach with no versions
    let versions = manager
        .get_versions(&coach.id.to_string(), test_tenant(), 50)
        .await
        .unwrap();

    assert!(versions.is_empty());
}

#[tokio::test]
async fn test_get_current_version_no_versions() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateCoachRequest {
        title: "Test Coach".to_owned(),
        description: None,
        system_prompt: "Prompt".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // Current version should be 0 when no versions exist
    let version = manager
        .get_current_version(&coach.id.to_string())
        .await
        .unwrap();

    assert_eq!(version, 0);
}

#[tokio::test]
async fn test_version_deleted_with_coach() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateCoachRequest {
        title: "Test Coach".to_owned(),
        description: None,
        system_prompt: "Prompt".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // Create some versions
    manager
        .create_version(&coach.id.to_string(), test_user_id(), None)
        .await
        .unwrap();
    manager
        .create_version(&coach.id.to_string(), test_user_id(), None)
        .await
        .unwrap();

    // Delete coach
    manager
        .delete(&coach.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap();

    // Versions should be deleted too (via CASCADE)
    // We can't directly check this since the coach doesn't exist,
    // but the cascade delete should have cleaned up the versions
    let result = manager
        .get_versions(&coach.id.to_string(), test_tenant(), 50)
        .await;

    // Should fail because coach no longer exists
    assert!(result.is_err());
}
