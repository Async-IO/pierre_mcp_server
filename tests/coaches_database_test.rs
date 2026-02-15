// ABOUTME: Unit tests for the coaches database module
// ABOUTME: Tests CRUD operations, favorites, active coach, and multi-tenant isolation
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// Test files: allow missing_docs (rustc lint) and unwrap (valid in tests per CLAUDE.md guidelines)
#![allow(missing_docs, clippy::unwrap_used)]

use pierre_mcp_server::database::coaches::{
    CoachCategory, CoachVisibility, CoachesManager, CreateCoachRequest, CreateSystemCoachRequest,
    ListCoachesFilter, UpdateCoachRequest,
};
use pierre_mcp_server::models::TenantId;
use sqlx::SqlitePool;
use uuid::Uuid;

/// Create a test database with coaches schema
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

    // Create second test user for isolation tests
    sqlx::query(
        r"
        INSERT INTO users (id, email, password_hash, created_at, last_active)
        VALUES ('660e8400-e29b-41d4-a716-446655440000', 'other@example.com', 'hash', '2025-01-01', '2025-01-01')
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Create coaches table
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
            forked_from TEXT REFERENCES coaches(id) ON DELETE SET NULL,
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

    // Create coach_versions table (for version history)
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

    // Create coach_assignments table (for system coaches)
    sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS coach_assignments (
            id TEXT PRIMARY KEY,
            coach_id TEXT NOT NULL REFERENCES coaches(id) ON DELETE CASCADE,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            assigned_by TEXT REFERENCES users(id) ON DELETE SET NULL,
            created_at TEXT NOT NULL,
            UNIQUE(coach_id, user_id)
        )
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Create user_coach_preferences table (for hiding coaches)
    sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS user_coach_preferences (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            coach_id TEXT NOT NULL REFERENCES coaches(id) ON DELETE CASCADE,
            is_hidden INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            UNIQUE(user_id, coach_id)
        )
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Create coach_relations table (for coach relationships)
    sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS coach_relations (
            id TEXT PRIMARY KEY,
            coach_id TEXT NOT NULL REFERENCES coaches(id) ON DELETE CASCADE,
            related_coach_id TEXT NOT NULL REFERENCES coaches(id) ON DELETE CASCADE,
            relation_type TEXT NOT NULL CHECK (relation_type IN ('related', 'alternative', 'prerequisite', 'sequel')),
            created_at TEXT NOT NULL,
            UNIQUE(coach_id, related_coach_id, relation_type)
        )
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Create coach_versions table (for version history)
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

fn other_tenant() -> TenantId {
    TenantId::from_uuid(Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap())
}

// ============================================================================
// Create Tests
// ============================================================================

#[tokio::test]
async fn test_create_coach() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateCoachRequest {
        title: "Marathon Coach".to_owned(),
        description: Some("Helps with marathon training".to_owned()),
        system_prompt: "You are an expert marathon coach.".to_owned(),
        category: CoachCategory::Training,
        tags: vec!["running".to_owned(), "marathon".to_owned()],
        sample_prompts: vec![],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    assert!(!coach.id.is_nil());
    assert_eq!(coach.user_id, test_user_id());
    assert_eq!(coach.tenant_id, test_tenant().to_string());
    assert_eq!(coach.title, "Marathon Coach");
    assert_eq!(
        coach.description,
        Some("Helps with marathon training".to_owned())
    );
    assert_eq!(coach.system_prompt, "You are an expert marathon coach.");
    assert_eq!(coach.category, CoachCategory::Training);
    assert_eq!(coach.tags, vec!["running", "marathon"]);
    assert!(!coach.is_favorite);
    assert!(!coach.is_active);
    assert_eq!(coach.use_count, 0);
    assert!(coach.last_used_at.is_none());
    // Token count should be estimated (~4 chars per token)
    assert!(coach.token_count > 0);
}

#[tokio::test]
async fn test_create_coach_minimal() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateCoachRequest {
        title: "Simple Coach".to_owned(),
        description: None,
        system_prompt: "You help.".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    assert_eq!(coach.title, "Simple Coach");
    assert!(coach.description.is_none());
    assert_eq!(coach.category, CoachCategory::Custom);
    assert!(coach.tags.is_empty());
}

// ============================================================================
// Get Tests
// ============================================================================

#[tokio::test]
async fn test_get_coach() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateCoachRequest {
        title: "Test Coach".to_owned(),
        description: None,
        system_prompt: "Test prompt".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };

    let created = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    let fetched = manager
        .get(&created.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap();

    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.title, "Test Coach");
}

#[tokio::test]
async fn test_get_coach_not_found() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let result = manager
        .get("nonexistent-id", test_user_id(), test_tenant())
        .await
        .unwrap();

    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_coach_wrong_user() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateCoachRequest {
        title: "Private Coach".to_owned(),
        description: None,
        system_prompt: "Private prompt".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };

    let created = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // Try to get with different user - should not find it
    let result = manager
        .get(&created.id.to_string(), other_user_id(), test_tenant())
        .await
        .unwrap();

    assert!(result.is_none());
}

// ============================================================================
// List Tests
// ============================================================================

#[tokio::test]
async fn test_list_coaches_empty() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let filter = ListCoachesFilter::default();
    let coaches = manager
        .list(test_user_id(), test_tenant(), &filter)
        .await
        .unwrap();

    assert!(coaches.is_empty());
}

#[tokio::test]
async fn test_list_coaches() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create multiple coaches
    for i in 1..=3 {
        let request = CreateCoachRequest {
            title: format!("Coach {i}"),
            description: None,
            system_prompt: format!("Prompt {i}"),
            category: CoachCategory::Custom,
            tags: vec![],
            sample_prompts: vec![],
        };
        manager
            .create(test_user_id(), test_tenant(), &request)
            .await
            .unwrap();
    }

    let filter = ListCoachesFilter::default();
    let coaches = manager
        .list(test_user_id(), test_tenant(), &filter)
        .await
        .unwrap();

    assert_eq!(coaches.len(), 3);
}

#[tokio::test]
async fn test_list_coaches_by_category() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create coaches with different categories
    let categories = [
        CoachCategory::Training,
        CoachCategory::Nutrition,
        CoachCategory::Training,
    ];

    for (i, category) in categories.iter().enumerate() {
        let request = CreateCoachRequest {
            title: format!("Coach {}", i + 1),
            description: None,
            system_prompt: format!("Prompt {}", i + 1),
            category: *category,
            tags: vec![],
            sample_prompts: vec![],
        };
        manager
            .create(test_user_id(), test_tenant(), &request)
            .await
            .unwrap();
    }

    // Filter by Training category
    let filter = ListCoachesFilter {
        category: Some(CoachCategory::Training),
        ..Default::default()
    };
    let coaches = manager
        .list(test_user_id(), test_tenant(), &filter)
        .await
        .unwrap();

    assert_eq!(coaches.len(), 2);
    for item in &coaches {
        assert_eq!(item.coach.category, CoachCategory::Training);
    }
}

#[tokio::test]
async fn test_list_coaches_favorites_only() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create coaches
    let mut coach_ids = Vec::new();
    for i in 1..=3 {
        let request = CreateCoachRequest {
            title: format!("Coach {i}"),
            description: None,
            system_prompt: format!("Prompt {i}"),
            category: CoachCategory::Custom,
            tags: vec![],
            sample_prompts: vec![],
        };
        let coach = manager
            .create(test_user_id(), test_tenant(), &request)
            .await
            .unwrap();
        coach_ids.push(coach.id.to_string());
    }

    // Mark first coach as favorite
    manager
        .toggle_favorite(&coach_ids[0], test_user_id(), test_tenant())
        .await
        .unwrap();

    // Filter favorites only
    let filter = ListCoachesFilter {
        favorites_only: true,
        ..Default::default()
    };
    let coaches = manager
        .list(test_user_id(), test_tenant(), &filter)
        .await
        .unwrap();

    assert_eq!(coaches.len(), 1);
    assert!(coaches[0].coach.is_favorite);
}

#[tokio::test]
async fn test_list_coaches_with_pagination() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create 5 coaches
    for i in 1..=5 {
        let request = CreateCoachRequest {
            title: format!("Coach {i}"),
            description: None,
            system_prompt: format!("Prompt {i}"),
            category: CoachCategory::Custom,
            tags: vec![],
            sample_prompts: vec![],
        };
        manager
            .create(test_user_id(), test_tenant(), &request)
            .await
            .unwrap();
    }

    // Get first page (limit 2)
    let filter = ListCoachesFilter {
        limit: Some(2),
        offset: Some(0),
        ..Default::default()
    };
    let page1 = manager
        .list(test_user_id(), test_tenant(), &filter)
        .await
        .unwrap();
    assert_eq!(page1.len(), 2);

    // Get second page
    let filter = ListCoachesFilter {
        limit: Some(2),
        offset: Some(2),
        ..Default::default()
    };
    let page2 = manager
        .list(test_user_id(), test_tenant(), &filter)
        .await
        .unwrap();
    assert_eq!(page2.len(), 2);

    // Get third page (only 1 remaining)
    let filter = ListCoachesFilter {
        limit: Some(2),
        offset: Some(4),
        ..Default::default()
    };
    let page3 = manager
        .list(test_user_id(), test_tenant(), &filter)
        .await
        .unwrap();
    assert_eq!(page3.len(), 1);
}

#[tokio::test]
async fn test_list_coaches_user_isolation() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create coach for user 1
    let request = CreateCoachRequest {
        title: "User 1 Coach".to_owned(),
        description: None,
        system_prompt: "Prompt 1".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };
    manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // Create coach for user 2
    let request = CreateCoachRequest {
        title: "User 2 Coach".to_owned(),
        description: None,
        system_prompt: "Prompt 2".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };
    manager
        .create(other_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // User 1 should only see their coach
    let filter = ListCoachesFilter::default();
    let user1_coaches = manager
        .list(test_user_id(), test_tenant(), &filter)
        .await
        .unwrap();
    assert_eq!(user1_coaches.len(), 1);
    assert_eq!(user1_coaches[0].coach.title, "User 1 Coach");

    // User 2 should only see their coach
    let user2_coaches = manager
        .list(other_user_id(), test_tenant(), &filter)
        .await
        .unwrap();
    assert_eq!(user2_coaches.len(), 1);
    assert_eq!(user2_coaches[0].coach.title, "User 2 Coach");
}

// ============================================================================
// Update Tests
// ============================================================================

#[tokio::test]
async fn test_update_coach() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateCoachRequest {
        title: "Original Title".to_owned(),
        description: Some("Original description".to_owned()),
        system_prompt: "Original prompt".to_owned(),
        category: CoachCategory::Custom,
        tags: vec!["tag1".to_owned()],
        sample_prompts: vec![],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    let update = UpdateCoachRequest {
        title: Some("Updated Title".to_owned()),
        description: Some("Updated description".to_owned()),
        system_prompt: Some("Updated prompt".to_owned()),
        category: Some(CoachCategory::Training),
        tags: Some(vec!["tag2".to_owned(), "tag3".to_owned()]),
        sample_prompts: None,
    };

    let updated = manager
        .update(
            &coach.id.to_string(),
            test_user_id(),
            test_tenant(),
            &update,
        )
        .await
        .unwrap();

    assert!(updated.is_some());
    let updated = updated.unwrap();
    assert_eq!(updated.title, "Updated Title");
    assert_eq!(updated.description, Some("Updated description".to_owned()));
    assert_eq!(updated.system_prompt, "Updated prompt");
    assert_eq!(updated.category, CoachCategory::Training);
    assert_eq!(updated.tags, vec!["tag2", "tag3"]);
}

#[tokio::test]
async fn test_update_coach_partial() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateCoachRequest {
        title: "Original Title".to_owned(),
        description: Some("Original description".to_owned()),
        system_prompt: "Original prompt".to_owned(),
        category: CoachCategory::Training,
        tags: vec!["tag1".to_owned()],
        sample_prompts: vec![],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // Only update title
    let update = UpdateCoachRequest {
        title: Some("New Title".to_owned()),
        description: None,
        system_prompt: None,
        category: None,
        tags: None,
        sample_prompts: None,
    };

    let updated = manager
        .update(
            &coach.id.to_string(),
            test_user_id(),
            test_tenant(),
            &update,
        )
        .await
        .unwrap()
        .unwrap();

    assert_eq!(updated.title, "New Title");
    assert_eq!(updated.description, Some("Original description".to_owned()));
    assert_eq!(updated.system_prompt, "Original prompt");
    assert_eq!(updated.category, CoachCategory::Training);
}

#[tokio::test]
async fn test_update_coach_not_found() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let update = UpdateCoachRequest {
        title: Some("New Title".to_owned()),
        description: None,
        system_prompt: None,
        category: None,
        tags: None,
        sample_prompts: None,
    };

    let result = manager
        .update("nonexistent-id", test_user_id(), test_tenant(), &update)
        .await
        .unwrap();

    assert!(result.is_none());
}

// ============================================================================
// Delete Tests
// ============================================================================

#[tokio::test]
async fn test_delete_coach() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateCoachRequest {
        title: "To Delete".to_owned(),
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

    let deleted = manager
        .delete(&coach.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap();

    assert!(deleted);

    // Verify it's gone
    let result = manager
        .get(&coach.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_delete_coach_not_found() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let deleted = manager
        .delete("nonexistent-id", test_user_id(), test_tenant())
        .await
        .unwrap();

    assert!(!deleted);
}

#[tokio::test]
async fn test_delete_coach_wrong_user() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateCoachRequest {
        title: "Private Coach".to_owned(),
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

    // Try to delete with different user
    let deleted = manager
        .delete(&coach.id.to_string(), other_user_id(), test_tenant())
        .await
        .unwrap();

    assert!(!deleted);

    // Verify it still exists for original user
    let result = manager
        .get(&coach.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap();
    assert!(result.is_some());
}

// ============================================================================
// Favorite Tests
// ============================================================================

#[tokio::test]
async fn test_toggle_favorite() {
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
    assert!(!coach.is_favorite);

    // Toggle to favorite
    let is_favorite = manager
        .toggle_favorite(&coach.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap();
    assert_eq!(is_favorite, Some(true));

    // Verify
    let fetched = manager
        .get(&coach.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap()
        .unwrap();
    assert!(fetched.is_favorite);

    // Toggle back
    let is_favorite = manager
        .toggle_favorite(&coach.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap();
    assert_eq!(is_favorite, Some(false));

    // Verify
    let fetched = manager
        .get(&coach.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap()
        .unwrap();
    assert!(!fetched.is_favorite);
}

#[tokio::test]
async fn test_toggle_favorite_not_found() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let result = manager
        .toggle_favorite("nonexistent-id", test_user_id(), test_tenant())
        .await
        .unwrap();

    assert!(result.is_none());
}

// ============================================================================
// Active Coach Tests
// ============================================================================

#[tokio::test]
async fn test_activate_coach() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateCoachRequest {
        title: "Active Coach".to_owned(),
        description: None,
        system_prompt: "Active prompt".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();
    assert!(!coach.is_active);

    // Activate
    let activated = manager
        .activate_coach(&coach.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap();

    assert!(activated.is_some());
    let activated = activated.unwrap();
    assert!(activated.is_active);
}

#[tokio::test]
async fn test_activate_coach_deactivates_others() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create two coaches
    let request1 = CreateCoachRequest {
        title: "Coach 1".to_owned(),
        description: None,
        system_prompt: "Prompt 1".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };
    let coach1 = manager
        .create(test_user_id(), test_tenant(), &request1)
        .await
        .unwrap();

    let request2 = CreateCoachRequest {
        title: "Coach 2".to_owned(),
        description: None,
        system_prompt: "Prompt 2".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };
    let coach2 = manager
        .create(test_user_id(), test_tenant(), &request2)
        .await
        .unwrap();

    // Activate first coach
    manager
        .activate_coach(&coach1.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap();

    // Verify first is active
    let fetched1 = manager
        .get(&coach1.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap()
        .unwrap();
    assert!(fetched1.is_active);

    // Activate second coach
    manager
        .activate_coach(&coach2.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap();

    // Verify second is active and first is not
    let fetched1 = manager
        .get(&coach1.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap()
        .unwrap();
    assert!(!fetched1.is_active);

    let fetched2 = manager
        .get(&coach2.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap()
        .unwrap();
    assert!(fetched2.is_active);
}

#[tokio::test]
async fn test_deactivate_coach() {
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

    // Activate
    manager
        .activate_coach(&coach.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap();

    // Deactivate
    let deactivated = manager
        .deactivate_coach(test_user_id(), test_tenant())
        .await
        .unwrap();
    assert!(deactivated);

    // Verify
    let fetched = manager
        .get(&coach.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap()
        .unwrap();
    assert!(!fetched.is_active);
}

#[tokio::test]
async fn test_deactivate_when_none_active() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Deactivate when nothing is active
    let deactivated = manager
        .deactivate_coach(test_user_id(), test_tenant())
        .await
        .unwrap();
    assert!(!deactivated);
}

#[tokio::test]
async fn test_get_active_coach() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // No active coach initially
    let active = manager
        .get_active_coach(test_user_id(), test_tenant())
        .await
        .unwrap();
    assert!(active.is_none());

    // Create and activate a coach
    let request = CreateCoachRequest {
        title: "Active Coach".to_owned(),
        description: None,
        system_prompt: "Active prompt".to_owned(),
        category: CoachCategory::Training,
        tags: vec![],
        sample_prompts: vec![],
    };

    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    manager
        .activate_coach(&coach.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap();

    // Now should have active coach
    let active = manager
        .get_active_coach(test_user_id(), test_tenant())
        .await
        .unwrap();
    assert!(active.is_some());
    let active = active.unwrap();
    assert_eq!(active.title, "Active Coach");
    assert!(active.is_active);
}

#[tokio::test]
async fn test_active_coach_user_isolation() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create coach for user 1 and activate
    let request = CreateCoachRequest {
        title: "User 1 Coach".to_owned(),
        description: None,
        system_prompt: "Prompt 1".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };
    let coach = manager
        .create(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();
    manager
        .activate_coach(&coach.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap();

    // User 1 should see active coach
    let active = manager
        .get_active_coach(test_user_id(), test_tenant())
        .await
        .unwrap();
    assert!(active.is_some());

    // User 2 should not see active coach
    let active = manager
        .get_active_coach(other_user_id(), test_tenant())
        .await
        .unwrap();
    assert!(active.is_none());
}

// ============================================================================
// Usage Recording Tests
// ============================================================================

#[tokio::test]
async fn test_record_usage() {
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
    assert_eq!(coach.use_count, 0);
    assert!(coach.last_used_at.is_none());

    // Record usage
    let recorded = manager
        .record_usage(&coach.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap();
    assert!(recorded);

    // Verify
    let fetched = manager
        .get(&coach.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.use_count, 1);
    assert!(fetched.last_used_at.is_some());

    // Record again
    manager
        .record_usage(&coach.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap();

    let fetched = manager
        .get(&coach.id.to_string(), test_user_id(), test_tenant())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.use_count, 2);
}

// ============================================================================
// Search Tests
// ============================================================================

#[tokio::test]
async fn test_search_coaches() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create coaches with searchable content
    let requests = [
        CreateCoachRequest {
            title: "Marathon Coach".to_owned(),
            description: Some("Helps with marathon training".to_owned()),
            system_prompt: "You are a marathon expert".to_owned(),
            category: CoachCategory::Training,
            tags: vec!["running".to_owned(), "marathon".to_owned()],
            sample_prompts: vec![],
        },
        CreateCoachRequest {
            title: "Nutrition Advisor".to_owned(),
            description: Some("Provides nutrition guidance".to_owned()),
            system_prompt: "You are a nutrition expert".to_owned(),
            category: CoachCategory::Nutrition,
            tags: vec!["diet".to_owned(), "nutrition".to_owned()],
            sample_prompts: vec![],
        },
        CreateCoachRequest {
            title: "Recovery Coach".to_owned(),
            description: Some("Specializes in recovery and rest".to_owned()),
            system_prompt: "You help with recovery".to_owned(),
            category: CoachCategory::Recovery,
            tags: vec!["rest".to_owned(), "recovery".to_owned()],
            sample_prompts: vec![],
        },
    ];

    for request in &requests {
        manager
            .create(test_user_id(), test_tenant(), request)
            .await
            .unwrap();
    }

    // Search by title
    let results = manager
        .search(test_user_id(), test_tenant(), "Marathon", None, None)
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Marathon Coach");

    // Search by description
    let results = manager
        .search(test_user_id(), test_tenant(), "nutrition", None, None)
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Nutrition Advisor");

    // Search by tags
    let results = manager
        .search(test_user_id(), test_tenant(), "running", None, None)
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Marathon Coach");

    // Search with no results
    let results = manager
        .search(test_user_id(), test_tenant(), "swimming", None, None)
        .await
        .unwrap();
    assert!(results.is_empty());
}

#[tokio::test]
async fn test_search_coaches_with_limit() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create multiple coaches with "Coach" in title
    for i in 1..=5 {
        let request = CreateCoachRequest {
            title: format!("Coach {i}"),
            description: None,
            system_prompt: format!("Prompt {i}"),
            category: CoachCategory::Custom,
            tags: vec![],
            sample_prompts: vec![],
        };
        manager
            .create(test_user_id(), test_tenant(), &request)
            .await
            .unwrap();
    }

    // Search with limit
    let results = manager
        .search(test_user_id(), test_tenant(), "Coach", Some(2), None)
        .await
        .unwrap();
    assert_eq!(results.len(), 2);
}

// ============================================================================
// Count Tests
// ============================================================================

#[tokio::test]
async fn test_count_coaches() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Initially zero
    let count = manager.count(test_user_id(), test_tenant()).await.unwrap();
    assert_eq!(count, 0);

    // Create coaches
    for i in 1..=3 {
        let request = CreateCoachRequest {
            title: format!("Coach {i}"),
            description: None,
            system_prompt: format!("Prompt {i}"),
            category: CoachCategory::Custom,
            tags: vec![],
            sample_prompts: vec![],
        };
        manager
            .create(test_user_id(), test_tenant(), &request)
            .await
            .unwrap();
    }

    let count = manager.count(test_user_id(), test_tenant()).await.unwrap();
    assert_eq!(count, 3);
}

// ============================================================================
// Category Tests
// ============================================================================

#[test]
fn test_coach_category_parsing() {
    // Lowercase parsing
    assert_eq!(CoachCategory::parse("training"), CoachCategory::Training);
    assert_eq!(CoachCategory::parse("nutrition"), CoachCategory::Nutrition);
    assert_eq!(CoachCategory::parse("recovery"), CoachCategory::Recovery);
    assert_eq!(CoachCategory::parse("recipes"), CoachCategory::Recipes);
    assert_eq!(CoachCategory::parse("mobility"), CoachCategory::Mobility);
    assert_eq!(CoachCategory::parse("custom"), CoachCategory::Custom);

    // Case-insensitive parsing (frontend sends capitalized values)
    assert_eq!(CoachCategory::parse("Training"), CoachCategory::Training);
    assert_eq!(CoachCategory::parse("Nutrition"), CoachCategory::Nutrition);
    assert_eq!(CoachCategory::parse("Recovery"), CoachCategory::Recovery);
    assert_eq!(CoachCategory::parse("Recipes"), CoachCategory::Recipes);
    assert_eq!(CoachCategory::parse("Mobility"), CoachCategory::Mobility);
    assert_eq!(CoachCategory::parse("Custom"), CoachCategory::Custom);

    // Mixed case
    assert_eq!(CoachCategory::parse("TRAINING"), CoachCategory::Training);
    assert_eq!(CoachCategory::parse("TraInInG"), CoachCategory::Training);

    // Unknown values default to Custom
    assert_eq!(CoachCategory::parse("unknown"), CoachCategory::Custom);
    assert_eq!(CoachCategory::parse(""), CoachCategory::Custom);
}

#[test]
fn test_coach_category_as_str() {
    assert_eq!(CoachCategory::Training.as_str(), "training");
    assert_eq!(CoachCategory::Nutrition.as_str(), "nutrition");
    assert_eq!(CoachCategory::Recovery.as_str(), "recovery");
    assert_eq!(CoachCategory::Recipes.as_str(), "recipes");
    assert_eq!(CoachCategory::Mobility.as_str(), "mobility");
    assert_eq!(CoachCategory::Custom.as_str(), "custom");
}

#[test]
fn test_coach_category_round_trip() {
    // Verify that as_str -> parse round-trips correctly for all variants
    let categories = [
        CoachCategory::Training,
        CoachCategory::Nutrition,
        CoachCategory::Recovery,
        CoachCategory::Recipes,
        CoachCategory::Mobility,
        CoachCategory::Custom,
    ];

    for category in categories {
        let serialized = category.as_str();
        let deserialized = CoachCategory::parse(serialized);
        assert_eq!(category, deserialized, "Round-trip failed for {serialized}");
    }
}

#[test]
fn test_coach_category_serde_serialization() {
    // Test serde serialization produces snake_case values
    let training = CoachCategory::Training;
    let json = serde_json::to_string(&training).unwrap();
    assert_eq!(json, "\"training\"");

    let mobility = CoachCategory::Mobility;
    let json = serde_json::to_string(&mobility).unwrap();
    assert_eq!(json, "\"mobility\"");

    let custom = CoachCategory::Custom;
    let json = serde_json::to_string(&custom).unwrap();
    assert_eq!(json, "\"custom\"");
}

#[test]
fn test_coach_category_serde_deserialization() {
    // Test serde deserialization handles various cases
    let training: CoachCategory = serde_json::from_str("\"training\"").unwrap();
    assert_eq!(training, CoachCategory::Training);

    let mobility: CoachCategory = serde_json::from_str("\"mobility\"").unwrap();
    assert_eq!(mobility, CoachCategory::Mobility);

    // Note: serde uses rename_all = "snake_case", so it expects lowercase
    // Frontend may send capitalized values through the API which gets parsed via parse()
}

// ============================================================================
// System Coach Tests
// ============================================================================

#[tokio::test]
async fn test_create_system_coach() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateSystemCoachRequest {
        title: "Pierre Default Coach".to_owned(),
        description: Some("The official Pierre fitness coach".to_owned()),
        system_prompt: "You are Pierre, an expert fitness coach.".to_owned(),
        category: CoachCategory::Training,
        tags: vec!["official".to_owned(), "default".to_owned()],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };

    let coach = manager
        .create_system_coach(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    assert!(!coach.id.is_nil());
    assert_eq!(coach.tenant_id, test_tenant().to_string());
    assert_eq!(coach.title, "Pierre Default Coach");
    assert!(coach.is_system);
    assert_eq!(coach.visibility, CoachVisibility::Tenant);
}

#[tokio::test]
async fn test_list_system_coaches() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create two system coaches
    for i in 1..=2 {
        let request = CreateSystemCoachRequest {
            title: format!("System Coach {i}"),
            description: None,
            system_prompt: format!("System prompt {i}"),
            category: CoachCategory::Training,
            tags: vec![],
            visibility: CoachVisibility::Tenant,
            sample_prompts: vec![],
        };
        manager
            .create_system_coach(test_user_id(), test_tenant(), &request)
            .await
            .unwrap();
    }

    let coaches = manager.list_system_coaches(test_tenant()).await.unwrap();
    assert_eq!(coaches.len(), 2);
    assert!(coaches.iter().all(|c| c.is_system));
}

#[tokio::test]
async fn test_get_system_coach() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateSystemCoachRequest {
        title: "System Coach".to_owned(),
        description: Some("System description".to_owned()),
        system_prompt: "System prompt".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };

    let created = manager
        .create_system_coach(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    let fetched = manager
        .get_system_coach(&created.id.to_string(), test_tenant())
        .await
        .unwrap();

    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.title, "System Coach");
    assert!(fetched.is_system);
}

#[tokio::test]
async fn test_update_system_coach() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateSystemCoachRequest {
        title: "Original System Coach".to_owned(),
        description: Some("Original description".to_owned()),
        system_prompt: "Original prompt".to_owned(),
        category: CoachCategory::Training,
        tags: vec!["original".to_owned()],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };

    let coach = manager
        .create_system_coach(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    let update = UpdateCoachRequest {
        title: Some("Updated System Coach".to_owned()),
        description: Some("Updated description".to_owned()),
        system_prompt: Some("Updated prompt".to_owned()),
        category: Some(CoachCategory::Nutrition),
        tags: Some(vec!["updated".to_owned()]),
        sample_prompts: None,
    };

    let updated = manager
        .update_system_coach(&coach.id.to_string(), test_tenant(), &update)
        .await
        .unwrap();

    assert!(updated.is_some());
    let updated = updated.unwrap();
    assert_eq!(updated.title, "Updated System Coach");
    assert_eq!(updated.description, Some("Updated description".to_owned()));
    assert_eq!(updated.category, CoachCategory::Nutrition);
    assert!(updated.is_system);
}

#[tokio::test]
async fn test_delete_system_coach() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    let request = CreateSystemCoachRequest {
        title: "To Delete".to_owned(),
        description: None,
        system_prompt: "Prompt".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };

    let coach = manager
        .create_system_coach(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    let deleted = manager
        .delete_system_coach(&coach.id.to_string(), test_tenant())
        .await
        .unwrap();

    assert!(deleted);

    // Verify it's gone
    let result = manager
        .get_system_coach(&coach.id.to_string(), test_tenant())
        .await
        .unwrap();
    assert!(result.is_none());
}

// ============================================================================
// Coach Assignment Tests
// ============================================================================

#[tokio::test]
async fn test_assign_coach_to_user() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create a system coach
    let request = CreateSystemCoachRequest {
        title: "System Coach".to_owned(),
        description: None,
        system_prompt: "System prompt".to_owned(),
        category: CoachCategory::Training,
        tags: vec![],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };
    let coach = manager
        .create_system_coach(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // Assign to user
    let assigned = manager
        .assign_coach(&coach.id.to_string(), other_user_id(), test_user_id())
        .await
        .unwrap();

    assert!(assigned);

    // Verify assignment
    let assignments = manager
        .list_assignments(&coach.id.to_string())
        .await
        .unwrap();
    assert_eq!(assignments.len(), 1);
    assert_eq!(assignments[0].user_id, other_user_id().to_string());
}

#[tokio::test]
async fn test_unassign_coach_from_user() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create and assign
    let request = CreateSystemCoachRequest {
        title: "System Coach".to_owned(),
        description: None,
        system_prompt: "Prompt".to_owned(),
        category: CoachCategory::Training,
        tags: vec![],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };
    let coach = manager
        .create_system_coach(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    manager
        .assign_coach(&coach.id.to_string(), other_user_id(), test_user_id())
        .await
        .unwrap();

    // Unassign
    let unassigned = manager
        .unassign_coach(&coach.id.to_string(), other_user_id())
        .await
        .unwrap();

    assert!(unassigned);

    // Verify unassignment
    let assignments = manager
        .list_assignments(&coach.id.to_string())
        .await
        .unwrap();
    assert!(assignments.is_empty());
}

#[tokio::test]
async fn test_list_assignments() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create system coach
    let request = CreateSystemCoachRequest {
        title: "System Coach".to_owned(),
        description: None,
        system_prompt: "Prompt".to_owned(),
        category: CoachCategory::Training,
        tags: vec![],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };
    let coach = manager
        .create_system_coach(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // Assign to both users
    manager
        .assign_coach(&coach.id.to_string(), test_user_id(), test_user_id())
        .await
        .unwrap();
    manager
        .assign_coach(&coach.id.to_string(), other_user_id(), test_user_id())
        .await
        .unwrap();

    let assignments = manager
        .list_assignments(&coach.id.to_string())
        .await
        .unwrap();
    assert_eq!(assignments.len(), 2);
}

#[tokio::test]
async fn test_list_coaches_includes_assigned_system_coaches() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create a personal coach for user 1
    let personal_request = CreateCoachRequest {
        title: "Personal Coach".to_owned(),
        description: None,
        system_prompt: "Personal prompt".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };
    manager
        .create(test_user_id(), test_tenant(), &personal_request)
        .await
        .unwrap();

    // Create a system coach and assign to user 1
    let system_request = CreateSystemCoachRequest {
        title: "System Coach".to_owned(),
        description: None,
        system_prompt: "System prompt".to_owned(),
        category: CoachCategory::Training,
        tags: vec![],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };
    let system_coach = manager
        .create_system_coach(test_user_id(), test_tenant(), &system_request)
        .await
        .unwrap();

    manager
        .assign_coach(&system_coach.id.to_string(), test_user_id(), test_user_id())
        .await
        .unwrap();

    // User 1 should see both coaches
    let filter = ListCoachesFilter::default();
    let coaches = manager
        .list(test_user_id(), test_tenant(), &filter)
        .await
        .unwrap();

    assert_eq!(coaches.len(), 2);

    // One should be assigned (system coach)
    let assigned_coaches: Vec<_> = coaches.iter().filter(|c| c.is_assigned).collect();
    assert_eq!(assigned_coaches.len(), 1);
    assert_eq!(assigned_coaches[0].coach.title, "System Coach");
}

// ============================================================================
// Hide/Show Coach Tests
// ============================================================================

#[tokio::test]
async fn test_hide_coach() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create a system coach and assign it
    let request = CreateSystemCoachRequest {
        title: "System Coach".to_owned(),
        description: None,
        system_prompt: "Prompt".to_owned(),
        category: CoachCategory::Training,
        tags: vec![],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };
    let coach = manager
        .create_system_coach(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    manager
        .assign_coach(&coach.id.to_string(), test_user_id(), test_user_id())
        .await
        .unwrap();

    // Hide the coach
    let hidden = manager
        .hide_coach(&coach.id.to_string(), test_user_id())
        .await
        .unwrap();

    assert!(hidden);
}

#[tokio::test]
async fn test_hide_coach_not_found() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Try to hide non-existent coach - should return error
    let result = manager.hide_coach("nonexistent-id", test_user_id()).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_show_coach() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create a system coach and assign it
    let request = CreateSystemCoachRequest {
        title: "System Coach".to_owned(),
        description: None,
        system_prompt: "Prompt".to_owned(),
        category: CoachCategory::Training,
        tags: vec![],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };
    let coach = manager
        .create_system_coach(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    manager
        .assign_coach(&coach.id.to_string(), test_user_id(), test_user_id())
        .await
        .unwrap();

    // Hide first
    manager
        .hide_coach(&coach.id.to_string(), test_user_id())
        .await
        .unwrap();

    // Then show
    let shown = manager
        .show_coach(&coach.id.to_string(), test_user_id())
        .await
        .unwrap();

    assert!(shown);
}

#[tokio::test]
async fn test_list_hidden_coaches() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create two system coaches
    let mut coach_ids = Vec::new();
    for i in 1..=2 {
        let request = CreateSystemCoachRequest {
            title: format!("System Coach {i}"),
            description: None,
            system_prompt: format!("Prompt {i}"),
            category: CoachCategory::Training,
            tags: vec![],
            visibility: CoachVisibility::Tenant,
            sample_prompts: vec![],
        };
        let coach = manager
            .create_system_coach(test_user_id(), test_tenant(), &request)
            .await
            .unwrap();
        coach_ids.push(coach.id.to_string());
    }

    // Assign both to user
    for id in &coach_ids {
        manager
            .assign_coach(id, test_user_id(), test_user_id())
            .await
            .unwrap();
    }

    // Hide only the first one
    manager
        .hide_coach(&coach_ids[0], test_user_id())
        .await
        .unwrap();

    // List hidden coaches
    let hidden = manager
        .list_hidden_coaches(test_user_id(), test_tenant())
        .await
        .unwrap();

    assert_eq!(hidden.len(), 1);
    assert_eq!(hidden[0].id.to_string(), coach_ids[0]);
    assert_eq!(hidden[0].title, "System Coach 1");
}

#[tokio::test]
async fn test_hidden_coach_excluded_from_list() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create a system coach and assign it
    let request = CreateSystemCoachRequest {
        title: "System Coach".to_owned(),
        description: None,
        system_prompt: "Prompt".to_owned(),
        category: CoachCategory::Training,
        tags: vec![],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };
    let coach = manager
        .create_system_coach(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    manager
        .assign_coach(&coach.id.to_string(), test_user_id(), test_user_id())
        .await
        .unwrap();

    // Should see 1 coach
    let filter = ListCoachesFilter::default();
    let coaches = manager
        .list(test_user_id(), test_tenant(), &filter)
        .await
        .unwrap();
    assert_eq!(coaches.len(), 1);

    // Hide the coach
    manager
        .hide_coach(&coach.id.to_string(), test_user_id())
        .await
        .unwrap();

    // Should see 0 coaches now
    let coaches = manager
        .list(test_user_id(), test_tenant(), &filter)
        .await
        .unwrap();
    assert_eq!(coaches.len(), 0);
}

#[tokio::test]
async fn test_unhidden_coach_appears_in_list() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create a system coach and assign it
    let request = CreateSystemCoachRequest {
        title: "System Coach".to_owned(),
        description: None,
        system_prompt: "Prompt".to_owned(),
        category: CoachCategory::Training,
        tags: vec![],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };
    let coach = manager
        .create_system_coach(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    manager
        .assign_coach(&coach.id.to_string(), test_user_id(), test_user_id())
        .await
        .unwrap();

    // Hide the coach
    manager
        .hide_coach(&coach.id.to_string(), test_user_id())
        .await
        .unwrap();

    // Should see 0 coaches
    let filter = ListCoachesFilter::default();
    let coaches = manager
        .list(test_user_id(), test_tenant(), &filter)
        .await
        .unwrap();
    assert_eq!(coaches.len(), 0);

    // Unhide the coach
    manager
        .show_coach(&coach.id.to_string(), test_user_id())
        .await
        .unwrap();

    // Should see 1 coach again
    let coaches = manager
        .list(test_user_id(), test_tenant(), &filter)
        .await
        .unwrap();
    assert_eq!(coaches.len(), 1);
}

#[tokio::test]
async fn test_hide_coach_user_isolation() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create a system coach and assign to both users
    let request = CreateSystemCoachRequest {
        title: "System Coach".to_owned(),
        description: None,
        system_prompt: "Prompt".to_owned(),
        category: CoachCategory::Training,
        tags: vec![],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };
    let coach = manager
        .create_system_coach(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    manager
        .assign_coach(&coach.id.to_string(), test_user_id(), test_user_id())
        .await
        .unwrap();
    manager
        .assign_coach(&coach.id.to_string(), other_user_id(), test_user_id())
        .await
        .unwrap();

    // User 1 hides the coach
    manager
        .hide_coach(&coach.id.to_string(), test_user_id())
        .await
        .unwrap();

    // User 1 should not see the coach
    let filter = ListCoachesFilter::default();
    let user1_coaches = manager
        .list(test_user_id(), test_tenant(), &filter)
        .await
        .unwrap();
    assert_eq!(user1_coaches.len(), 0);

    // User 2 should still see the coach
    let user2_coaches = manager
        .list(other_user_id(), test_tenant(), &filter)
        .await
        .unwrap();
    assert_eq!(user2_coaches.len(), 1);
}

// ============================================================================
// Cross-Tenant System Coach Visibility Tests
// ============================================================================

/// System coaches with `is_system=1` should be visible to users from ANY tenant
/// when `include_system` filter is enabled, regardless of the tenant they were created in.
#[tokio::test]
async fn test_system_coach_visible_across_tenants() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create a system coach in test_tenant() (tenant A)
    let request = CreateSystemCoachRequest {
        title: "Global System Coach".to_owned(),
        description: Some("Visible to all tenants".to_owned()),
        system_prompt: "You are a globally available coach.".to_owned(),
        category: CoachCategory::Training,
        tags: vec!["global".to_owned()],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };

    let system_coach = manager
        .create_system_coach(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    assert!(system_coach.is_system);
    assert_eq!(system_coach.tenant_id, test_tenant().to_string());

    // User from other_tenant() (tenant B) should see the system coach
    // when include_system filter is enabled
    let filter = ListCoachesFilter {
        include_system: true,
        ..Default::default()
    };

    let coaches = manager
        .list(other_user_id(), other_tenant(), &filter)
        .await
        .unwrap();

    // Should find the system coach even though user is from a different tenant
    assert_eq!(coaches.len(), 1);
    assert_eq!(coaches[0].coach.title, "Global System Coach");
    assert!(coaches[0].coach.is_system);
}

/// When `include_system` is false, system coaches from other tenants should NOT be visible
#[tokio::test]
async fn test_system_coach_hidden_when_include_system_false() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create a system coach in test_tenant()
    let request = CreateSystemCoachRequest {
        title: "System Coach".to_owned(),
        description: None,
        system_prompt: "Prompt".to_owned(),
        category: CoachCategory::Training,
        tags: vec![],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };

    manager
        .create_system_coach(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // User from other_tenant() should NOT see the system coach
    // when include_system is false (default)
    let filter = ListCoachesFilter {
        include_system: false,
        ..Default::default()
    };

    let coaches = manager
        .list(other_user_id(), other_tenant(), &filter)
        .await
        .unwrap();

    // Should not find any coaches - no personal coaches and system coaches excluded
    assert!(coaches.is_empty());
}

/// Multiple system coaches from different tenants should all be visible
/// to users from any tenant when `include_system` is enabled
#[tokio::test]
async fn test_multiple_system_coaches_visible_across_tenants() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create system coach in test_tenant()
    let request1 = CreateSystemCoachRequest {
        title: "System Coach From Tenant A".to_owned(),
        description: None,
        system_prompt: "Prompt A".to_owned(),
        category: CoachCategory::Training,
        tags: vec![],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };
    manager
        .create_system_coach(test_user_id(), test_tenant(), &request1)
        .await
        .unwrap();

    // Create system coach in other_tenant()
    let request2 = CreateSystemCoachRequest {
        title: "System Coach From Tenant B".to_owned(),
        description: None,
        system_prompt: "Prompt B".to_owned(),
        category: CoachCategory::Nutrition,
        tags: vec![],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };
    manager
        .create_system_coach(other_user_id(), other_tenant(), &request2)
        .await
        .unwrap();

    // User from test_tenant() should see both system coaches
    let filter = ListCoachesFilter {
        include_system: true,
        ..Default::default()
    };

    let coaches = manager
        .list(test_user_id(), test_tenant(), &filter)
        .await
        .unwrap();

    assert_eq!(coaches.len(), 2);
    let titles: Vec<&str> = coaches.iter().map(|c| c.coach.title.as_str()).collect();
    assert!(titles.contains(&"System Coach From Tenant A"));
    assert!(titles.contains(&"System Coach From Tenant B"));

    // User from other_tenant() should also see both system coaches
    let coaches = manager
        .list(other_user_id(), other_tenant(), &filter)
        .await
        .unwrap();

    assert_eq!(coaches.len(), 2);
}

/// Personal coaches should remain tenant-isolated even when system coaches are visible
#[tokio::test]
async fn test_personal_coaches_remain_isolated_with_system_coaches() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create a personal coach in test_tenant()
    let personal_request = CreateCoachRequest {
        title: "Personal Coach".to_owned(),
        description: None,
        system_prompt: "Personal prompt".to_owned(),
        category: CoachCategory::Custom,
        tags: vec![],
        sample_prompts: vec![],
    };
    manager
        .create(test_user_id(), test_tenant(), &personal_request)
        .await
        .unwrap();

    // Create a system coach in test_tenant()
    let system_request = CreateSystemCoachRequest {
        title: "System Coach".to_owned(),
        description: None,
        system_prompt: "System prompt".to_owned(),
        category: CoachCategory::Training,
        tags: vec![],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };
    manager
        .create_system_coach(test_user_id(), test_tenant(), &system_request)
        .await
        .unwrap();

    // User from other_tenant() with include_system should see ONLY the system coach
    // NOT the personal coach from test_tenant()
    let filter = ListCoachesFilter {
        include_system: true,
        ..Default::default()
    };

    let coaches = manager
        .list(other_user_id(), other_tenant(), &filter)
        .await
        .unwrap();

    assert_eq!(coaches.len(), 1);
    assert_eq!(coaches[0].coach.title, "System Coach");
    assert!(coaches[0].coach.is_system);

    // User from test_tenant() should see both their personal coach and the system coach
    let coaches = manager
        .list(test_user_id(), test_tenant(), &filter)
        .await
        .unwrap();

    assert_eq!(coaches.len(), 2);
}

/// System coaches can be hidden by users from ANY tenant, not just the tenant that created them.
/// This is the expected behavior because system coaches are globally visible.
#[tokio::test]
async fn test_hide_system_coach_cross_tenant() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create a system coach in test_tenant() (tenant A)
    let request = CreateSystemCoachRequest {
        title: "Global System Coach".to_owned(),
        description: None,
        system_prompt: "You are a globally available coach.".to_owned(),
        category: CoachCategory::Training,
        tags: vec![],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };

    let system_coach = manager
        .create_system_coach(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    assert!(system_coach.is_system);
    assert_eq!(system_coach.tenant_id, test_tenant().to_string());

    // User from other_tenant() (tenant B) should be able to hide this system coach
    // Even though the coach was created by test_tenant()
    let hidden = manager
        .hide_coach(&system_coach.id.to_string(), other_user_id())
        .await
        .unwrap();

    assert!(hidden);

    // Verify the coach is hidden for other_user
    let filter = ListCoachesFilter {
        include_system: true,
        include_hidden: false,
        ..Default::default()
    };

    let coaches = manager
        .list(other_user_id(), other_tenant(), &filter)
        .await
        .unwrap();

    // Should NOT see the system coach (it's hidden for this user)
    assert!(coaches.is_empty());

    // But the original tenant user should still see it
    let coaches = manager
        .list(test_user_id(), test_tenant(), &filter)
        .await
        .unwrap();

    assert_eq!(coaches.len(), 1);
    assert_eq!(coaches[0].coach.title, "Global System Coach");
}

/// Users can show (unhide) system coaches from other tenants
#[tokio::test]
async fn test_show_system_coach_cross_tenant() {
    let pool = create_test_db().await;
    let manager = CoachesManager::new(pool);

    // Create a system coach in test_tenant()
    let request = CreateSystemCoachRequest {
        title: "Global System Coach".to_owned(),
        description: None,
        system_prompt: "Prompt".to_owned(),
        category: CoachCategory::Training,
        tags: vec![],
        visibility: CoachVisibility::Tenant,
        sample_prompts: vec![],
    };

    let system_coach = manager
        .create_system_coach(test_user_id(), test_tenant(), &request)
        .await
        .unwrap();

    // User from other_tenant() hides the coach
    manager
        .hide_coach(&system_coach.id.to_string(), other_user_id())
        .await
        .unwrap();

    // Verify it's hidden
    let filter = ListCoachesFilter {
        include_system: true,
        include_hidden: false,
        ..Default::default()
    };

    let coaches = manager
        .list(other_user_id(), other_tenant(), &filter)
        .await
        .unwrap();
    assert!(coaches.is_empty());

    // Now show (unhide) the coach
    let shown = manager
        .show_coach(&system_coach.id.to_string(), other_user_id())
        .await
        .unwrap();

    assert!(shown);

    // Should now see the coach again
    let coaches = manager
        .list(other_user_id(), other_tenant(), &filter)
        .await
        .unwrap();

    assert_eq!(coaches.len(), 1);
    assert_eq!(coaches[0].coach.title, "Global System Coach");
}
