// ABOUTME: Unit tests for CoachAuthorsManager database operations
// ABOUTME: Tests CRUD operations for author profiles in the Coach Store
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

mod common;

use common::{create_test_server_resources, create_test_user, create_test_user_with_email};
use pierre_mcp_server::database::coach_authors::{
    CoachAuthorsManager, CreateAuthorRequest, UpdateAuthorRequest,
};
use pierre_mcp_server::database_plugins::DatabaseProvider;
use pierre_mcp_server::models::TenantId;
use uuid::Uuid;

// ============================================================================
// Create Author Tests
// ============================================================================

#[tokio::test]
async fn test_create_author() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, _user) = create_test_user(&resources.database).await.unwrap();
    let tenants = resources
        .database
        .list_tenants_for_user(user_id)
        .await
        .unwrap();
    let tenant_id = tenants
        .first()
        .map_or_else(|| TenantId::from(user_id), |t| t.id);

    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let manager = CoachAuthorsManager::new(sqlite_pool);

    let request = CreateAuthorRequest {
        display_name: "Test Author".to_owned(),
        bio: Some("A test author bio".to_owned()),
        avatar_url: Some("https://example.com/avatar.png".to_owned()),
        website_url: Some("https://example.com".to_owned()),
    };

    let author = manager.create(user_id, tenant_id, &request).await.unwrap();

    assert_eq!(author.user_id, user_id);
    assert_eq!(author.tenant_id, tenant_id.to_string());
    assert_eq!(author.display_name, "Test Author");
    assert_eq!(author.bio, Some("A test author bio".to_owned()));
    assert_eq!(
        author.avatar_url,
        Some("https://example.com/avatar.png".to_owned())
    );
    assert_eq!(author.website_url, Some("https://example.com".to_owned()));
    assert!(!author.is_verified);
    assert!(author.verified_at.is_none());
    assert!(author.verified_by.is_none());
    assert_eq!(author.published_coach_count, 0);
    assert_eq!(author.total_install_count, 0);
}

#[tokio::test]
async fn test_create_author_minimal() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, _user) = create_test_user(&resources.database).await.unwrap();
    let tenants = resources
        .database
        .list_tenants_for_user(user_id)
        .await
        .unwrap();
    let tenant_id = tenants
        .first()
        .map_or_else(|| TenantId::from(user_id), |t| t.id);

    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let manager = CoachAuthorsManager::new(sqlite_pool);

    let request = CreateAuthorRequest {
        display_name: "Minimal Author".to_owned(),
        bio: None,
        avatar_url: None,
        website_url: None,
    };

    let author = manager.create(user_id, tenant_id, &request).await.unwrap();

    assert_eq!(author.display_name, "Minimal Author");
    assert!(author.bio.is_none());
    assert!(author.avatar_url.is_none());
    assert!(author.website_url.is_none());
}

#[tokio::test]
async fn test_create_author_duplicate() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, _user) = create_test_user(&resources.database).await.unwrap();
    let tenants = resources
        .database
        .list_tenants_for_user(user_id)
        .await
        .unwrap();
    let tenant_id = tenants
        .first()
        .map_or_else(|| TenantId::from(user_id), |t| t.id);

    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let manager = CoachAuthorsManager::new(sqlite_pool);

    let request = CreateAuthorRequest {
        display_name: "First Author".to_owned(),
        bio: None,
        avatar_url: None,
        website_url: None,
    };

    // Create first time - should succeed
    manager.create(user_id, tenant_id, &request).await.unwrap();

    // Try to create again - should fail
    let result = manager.create(user_id, tenant_id, &request).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}

// ============================================================================
// Get Author Tests
// ============================================================================

#[tokio::test]
async fn test_get_author_by_user() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, _user) = create_test_user(&resources.database).await.unwrap();
    let tenants = resources
        .database
        .list_tenants_for_user(user_id)
        .await
        .unwrap();
    let tenant_id = tenants
        .first()
        .map_or_else(|| TenantId::from(user_id), |t| t.id);

    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let manager = CoachAuthorsManager::new(sqlite_pool);

    let request = CreateAuthorRequest {
        display_name: "Get By User".to_owned(),
        bio: Some("Test bio".to_owned()),
        avatar_url: None,
        website_url: None,
    };

    let created = manager.create(user_id, tenant_id, &request).await.unwrap();

    let fetched = manager
        .get_by_user(user_id, tenant_id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.display_name, "Get By User");
    assert_eq!(fetched.bio, Some("Test bio".to_owned()));
}

#[tokio::test]
async fn test_get_author_by_user_not_found() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, _user) = create_test_user(&resources.database).await.unwrap();
    let tenants = resources
        .database
        .list_tenants_for_user(user_id)
        .await
        .unwrap();
    let tenant_id = tenants
        .first()
        .map_or_else(|| TenantId::from(user_id), |t| t.id);

    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let manager = CoachAuthorsManager::new(sqlite_pool);

    let result = manager.get_by_user(user_id, tenant_id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_author_by_id() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, _user) = create_test_user(&resources.database).await.unwrap();
    let tenants = resources
        .database
        .list_tenants_for_user(user_id)
        .await
        .unwrap();
    let tenant_id = tenants
        .first()
        .map_or_else(|| TenantId::from(user_id), |t| t.id);

    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let manager = CoachAuthorsManager::new(sqlite_pool);

    let request = CreateAuthorRequest {
        display_name: "Get By ID".to_owned(),
        bio: None,
        avatar_url: None,
        website_url: None,
    };

    let created = manager.create(user_id, tenant_id, &request).await.unwrap();

    let fetched = manager
        .get_by_id(&created.id.to_string())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.display_name, "Get By ID");
}

#[tokio::test]
async fn test_get_author_by_id_not_found() {
    let resources = create_test_server_resources().await.unwrap();

    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let manager = CoachAuthorsManager::new(sqlite_pool);

    let fake_id = Uuid::new_v4();
    let result = manager.get_by_id(&fake_id.to_string()).await.unwrap();
    assert!(result.is_none());
}

// ============================================================================
// Update Author Tests
// ============================================================================

#[tokio::test]
async fn test_update_author_all_fields() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, _user) = create_test_user(&resources.database).await.unwrap();
    let tenants = resources
        .database
        .list_tenants_for_user(user_id)
        .await
        .unwrap();
    let tenant_id = tenants
        .first()
        .map_or_else(|| TenantId::from(user_id), |t| t.id);

    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let manager = CoachAuthorsManager::new(sqlite_pool);

    // Create author first
    let create_request = CreateAuthorRequest {
        display_name: "Original Name".to_owned(),
        bio: Some("Original bio".to_owned()),
        avatar_url: Some("https://example.com/old.png".to_owned()),
        website_url: Some("https://old.example.com".to_owned()),
    };

    manager
        .create(user_id, tenant_id, &create_request)
        .await
        .unwrap();

    // Update all fields
    let update_request = UpdateAuthorRequest {
        display_name: Some("Updated Name".to_owned()),
        bio: Some("Updated bio".to_owned()),
        avatar_url: Some("https://example.com/new.png".to_owned()),
        website_url: Some("https://new.example.com".to_owned()),
    };

    let updated = manager
        .update(user_id, tenant_id, &update_request)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(updated.display_name, "Updated Name");
    assert_eq!(updated.bio, Some("Updated bio".to_owned()));
    assert_eq!(
        updated.avatar_url,
        Some("https://example.com/new.png".to_owned())
    );
    assert_eq!(
        updated.website_url,
        Some("https://new.example.com".to_owned())
    );
}

#[tokio::test]
async fn test_update_author_partial() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, _user) = create_test_user(&resources.database).await.unwrap();
    let tenants = resources
        .database
        .list_tenants_for_user(user_id)
        .await
        .unwrap();
    let tenant_id = tenants
        .first()
        .map_or_else(|| TenantId::from(user_id), |t| t.id);

    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let manager = CoachAuthorsManager::new(sqlite_pool);

    // Create author first
    let create_request = CreateAuthorRequest {
        display_name: "Original".to_owned(),
        bio: Some("Original bio".to_owned()),
        avatar_url: None,
        website_url: None,
    };

    manager
        .create(user_id, tenant_id, &create_request)
        .await
        .unwrap();

    // Update only display_name
    let update_request = UpdateAuthorRequest {
        display_name: Some("New Name".to_owned()),
        bio: None,
        avatar_url: None,
        website_url: None,
    };

    let updated = manager
        .update(user_id, tenant_id, &update_request)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(updated.display_name, "New Name");
    // Bio should be preserved
    assert_eq!(updated.bio, Some("Original bio".to_owned()));
}

#[tokio::test]
async fn test_update_author_not_found() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, _user) = create_test_user(&resources.database).await.unwrap();
    let tenants = resources
        .database
        .list_tenants_for_user(user_id)
        .await
        .unwrap();
    let tenant_id = tenants
        .first()
        .map_or_else(|| TenantId::from(user_id), |t| t.id);

    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let manager = CoachAuthorsManager::new(sqlite_pool);

    let update_request = UpdateAuthorRequest {
        display_name: Some("New Name".to_owned()),
        bio: None,
        avatar_url: None,
        website_url: None,
    };

    // Update without creating - should fail
    let result = manager.update(user_id, tenant_id, &update_request).await;
    assert!(result.is_err());
}

// ============================================================================
// Verify Author Tests
// ============================================================================

#[tokio::test]
async fn test_verify_author() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, _user) = create_test_user(&resources.database).await.unwrap();
    let tenants = resources
        .database
        .list_tenants_for_user(user_id)
        .await
        .unwrap();
    let tenant_id = tenants
        .first()
        .map_or_else(|| TenantId::from(user_id), |t| t.id);

    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let manager = CoachAuthorsManager::new(sqlite_pool);

    let request = CreateAuthorRequest {
        display_name: "To Be Verified".to_owned(),
        bio: None,
        avatar_url: None,
        website_url: None,
    };

    let created = manager.create(user_id, tenant_id, &request).await.unwrap();
    assert!(!created.is_verified);

    // Use user_id as admin_id to satisfy FK constraint (in production, this would be an admin user)
    let verified = manager
        .verify_author(&created.id.to_string(), user_id)
        .await
        .unwrap();

    assert!(verified.is_verified);
    assert!(verified.verified_at.is_some());
    assert_eq!(verified.verified_by, Some(user_id));
}

#[tokio::test]
async fn test_verify_author_not_found() {
    let resources = create_test_server_resources().await.unwrap();

    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let manager = CoachAuthorsManager::new(sqlite_pool);

    let fake_id = Uuid::new_v4();
    let admin_id = Uuid::new_v4();
    let result = manager.verify_author(&fake_id.to_string(), admin_id).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

// ============================================================================
// Increment Published Count Tests
// ============================================================================

#[tokio::test]
async fn test_increment_published_count() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, _user) = create_test_user(&resources.database).await.unwrap();
    let tenants = resources
        .database
        .list_tenants_for_user(user_id)
        .await
        .unwrap();
    let tenant_id = tenants
        .first()
        .map_or_else(|| TenantId::from(user_id), |t| t.id);

    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let manager = CoachAuthorsManager::new(sqlite_pool);

    let request = CreateAuthorRequest {
        display_name: "Author With Coaches".to_owned(),
        bio: None,
        avatar_url: None,
        website_url: None,
    };

    let created = manager.create(user_id, tenant_id, &request).await.unwrap();
    assert_eq!(created.published_coach_count, 0);

    // Increment count
    manager
        .increment_published_count(&created.id.to_string())
        .await
        .unwrap();

    let fetched = manager
        .get_by_id(&created.id.to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.published_coach_count, 1);

    // Increment again
    manager
        .increment_published_count(&created.id.to_string())
        .await
        .unwrap();

    let fetched = manager
        .get_by_id(&created.id.to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.published_coach_count, 2);
}

// ============================================================================
// Update Install Count Tests
// ============================================================================

#[tokio::test]
async fn test_update_install_count_increment() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, _user) = create_test_user(&resources.database).await.unwrap();
    let tenants = resources
        .database
        .list_tenants_for_user(user_id)
        .await
        .unwrap();
    let tenant_id = tenants
        .first()
        .map_or_else(|| TenantId::from(user_id), |t| t.id);

    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let manager = CoachAuthorsManager::new(sqlite_pool);

    let request = CreateAuthorRequest {
        display_name: "Install Count Test".to_owned(),
        bio: None,
        avatar_url: None,
        website_url: None,
    };

    let created = manager.create(user_id, tenant_id, &request).await.unwrap();
    assert_eq!(created.total_install_count, 0);

    // Increment by 5
    manager
        .update_install_count(&created.id.to_string(), 5)
        .await
        .unwrap();

    let fetched = manager
        .get_by_id(&created.id.to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.total_install_count, 5);
}

#[tokio::test]
async fn test_update_install_count_decrement() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, _user) = create_test_user(&resources.database).await.unwrap();
    let tenants = resources
        .database
        .list_tenants_for_user(user_id)
        .await
        .unwrap();
    let tenant_id = tenants
        .first()
        .map_or_else(|| TenantId::from(user_id), |t| t.id);

    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let manager = CoachAuthorsManager::new(sqlite_pool);

    let request = CreateAuthorRequest {
        display_name: "Decrement Test".to_owned(),
        bio: None,
        avatar_url: None,
        website_url: None,
    };

    let created = manager.create(user_id, tenant_id, &request).await.unwrap();

    // Increment first
    manager
        .update_install_count(&created.id.to_string(), 10)
        .await
        .unwrap();

    // Decrement by 3
    manager
        .update_install_count(&created.id.to_string(), -3)
        .await
        .unwrap();

    let fetched = manager
        .get_by_id(&created.id.to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.total_install_count, 7);
}

#[tokio::test]
async fn test_update_install_count_never_negative() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, _user) = create_test_user(&resources.database).await.unwrap();
    let tenants = resources
        .database
        .list_tenants_for_user(user_id)
        .await
        .unwrap();
    let tenant_id = tenants
        .first()
        .map_or_else(|| TenantId::from(user_id), |t| t.id);

    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let manager = CoachAuthorsManager::new(sqlite_pool);

    let request = CreateAuthorRequest {
        display_name: "Never Negative".to_owned(),
        bio: None,
        avatar_url: None,
        website_url: None,
    };

    let created = manager.create(user_id, tenant_id, &request).await.unwrap();

    // Try to decrement from 0 - should stay at 0
    manager
        .update_install_count(&created.id.to_string(), -10)
        .await
        .unwrap();

    let fetched = manager
        .get_by_id(&created.id.to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.total_install_count, 0);
}

// ============================================================================
// List Authors Tests
// ============================================================================

#[tokio::test]
async fn test_list_popular_authors() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, _user) = create_test_user(&resources.database).await.unwrap();
    let tenants = resources
        .database
        .list_tenants_for_user(user_id)
        .await
        .unwrap();
    let tenant_id = tenants
        .first()
        .map_or_else(|| TenantId::from(user_id), |t| t.id);

    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let manager = CoachAuthorsManager::new(sqlite_pool);

    // Create author with published coaches
    let request = CreateAuthorRequest {
        display_name: "Popular Author".to_owned(),
        bio: None,
        avatar_url: None,
        website_url: None,
    };
    let author = manager.create(user_id, tenant_id, &request).await.unwrap();

    // Increment published count (required to appear in list)
    manager
        .increment_published_count(&author.id.to_string())
        .await
        .unwrap();

    // Set install count
    manager
        .update_install_count(&author.id.to_string(), 100)
        .await
        .unwrap();

    let popular = manager.list_popular(tenant_id, None).await.unwrap();
    assert_eq!(popular.len(), 1);
    assert_eq!(popular[0].display_name, "Popular Author");
    assert_eq!(popular[0].total_install_count, 100);
}

#[tokio::test]
async fn test_list_popular_authors_empty_no_published() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, _user) = create_test_user(&resources.database).await.unwrap();
    let tenants = resources
        .database
        .list_tenants_for_user(user_id)
        .await
        .unwrap();
    let tenant_id = tenants
        .first()
        .map_or_else(|| TenantId::from(user_id), |t| t.id);

    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let manager = CoachAuthorsManager::new(sqlite_pool);

    // Create author without published coaches
    let request = CreateAuthorRequest {
        display_name: "No Published".to_owned(),
        bio: None,
        avatar_url: None,
        website_url: None,
    };
    manager.create(user_id, tenant_id, &request).await.unwrap();

    // Should be empty because author has no published coaches
    let popular = manager.list_popular(tenant_id, None).await.unwrap();
    assert!(popular.is_empty());
}

#[tokio::test]
async fn test_list_verified_authors() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, _user) = create_test_user(&resources.database).await.unwrap();
    let tenants = resources
        .database
        .list_tenants_for_user(user_id)
        .await
        .unwrap();
    let tenant_id = tenants
        .first()
        .map_or_else(|| TenantId::from(user_id), |t| t.id);

    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let manager = CoachAuthorsManager::new(sqlite_pool);

    // Create and verify author
    let request = CreateAuthorRequest {
        display_name: "Verified Author".to_owned(),
        bio: None,
        avatar_url: None,
        website_url: None,
    };
    let author = manager.create(user_id, tenant_id, &request).await.unwrap();

    // Verify the author (use user_id as admin_id to satisfy FK constraint)
    manager
        .verify_author(&author.id.to_string(), user_id)
        .await
        .unwrap();

    let verified = manager.list_verified(tenant_id, None).await.unwrap();
    assert_eq!(verified.len(), 1);
    assert_eq!(verified[0].display_name, "Verified Author");
    assert!(verified[0].is_verified);
}

#[tokio::test]
async fn test_list_verified_authors_excludes_unverified() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, _user) = create_test_user(&resources.database).await.unwrap();
    let tenants = resources
        .database
        .list_tenants_for_user(user_id)
        .await
        .unwrap();
    let tenant_id = tenants
        .first()
        .map_or_else(|| TenantId::from(user_id), |t| t.id);

    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let manager = CoachAuthorsManager::new(sqlite_pool);

    // Create author without verifying
    let request = CreateAuthorRequest {
        display_name: "Unverified Author".to_owned(),
        bio: None,
        avatar_url: None,
        website_url: None,
    };
    manager.create(user_id, tenant_id, &request).await.unwrap();

    let verified = manager.list_verified(tenant_id, None).await.unwrap();
    assert!(verified.is_empty());
}

// ============================================================================
// Get or Create Tests
// ============================================================================

#[tokio::test]
async fn test_get_or_create_new() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, _user) = create_test_user(&resources.database).await.unwrap();
    let tenants = resources
        .database
        .list_tenants_for_user(user_id)
        .await
        .unwrap();
    let tenant_id = tenants
        .first()
        .map_or_else(|| TenantId::from(user_id), |t| t.id);

    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let manager = CoachAuthorsManager::new(sqlite_pool);

    // Should create new author
    let author = manager
        .get_or_create(user_id, tenant_id, "Auto Created")
        .await
        .unwrap();

    assert_eq!(author.display_name, "Auto Created");
    assert!(author.bio.is_none());
}

#[tokio::test]
async fn test_get_or_create_existing() {
    let resources = create_test_server_resources().await.unwrap();
    let (user_id, _user) = create_test_user(&resources.database).await.unwrap();
    let tenants = resources
        .database
        .list_tenants_for_user(user_id)
        .await
        .unwrap();
    let tenant_id = tenants
        .first()
        .map_or_else(|| TenantId::from(user_id), |t| t.id);

    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let manager = CoachAuthorsManager::new(sqlite_pool);

    // Create author first
    let request = CreateAuthorRequest {
        display_name: "Original Name".to_owned(),
        bio: Some("Original bio".to_owned()),
        avatar_url: None,
        website_url: None,
    };
    let created = manager.create(user_id, tenant_id, &request).await.unwrap();

    // get_or_create should return existing, not create new
    let fetched = manager
        .get_or_create(user_id, tenant_id, "Different Name")
        .await
        .unwrap();

    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.display_name, "Original Name");
    assert_eq!(fetched.bio, Some("Original bio".to_owned()));
}

// ============================================================================
// Multi-Tenant Isolation Tests
// ============================================================================

#[tokio::test]
async fn test_authors_isolated_by_tenant() {
    let resources = create_test_server_resources().await.unwrap();

    // Create user in tenant 1
    let (user1_id, _user1) = create_test_user_with_email(&resources.database, "user1@example.com")
        .await
        .unwrap();
    let tenants1 = resources
        .database
        .list_tenants_for_user(user1_id)
        .await
        .unwrap();
    let tenant1_id = tenants1
        .first()
        .map_or_else(|| TenantId::from(user1_id), |t| t.id);

    // Create user in tenant 2
    let (user2_id, _user2) = create_test_user_with_email(&resources.database, "user2@example.com")
        .await
        .unwrap();
    let tenants2 = resources
        .database
        .list_tenants_for_user(user2_id)
        .await
        .unwrap();
    let tenant2_id = tenants2
        .first()
        .map_or_else(|| TenantId::from(user2_id), |t| t.id);

    let sqlite_pool = resources.database.sqlite_pool().unwrap().clone();
    let manager = CoachAuthorsManager::new(sqlite_pool);

    // Create author in tenant 1
    let request1 = CreateAuthorRequest {
        display_name: "Tenant 1 Author".to_owned(),
        bio: None,
        avatar_url: None,
        website_url: None,
    };
    let author1 = manager
        .create(user1_id, tenant1_id, &request1)
        .await
        .unwrap();
    manager
        .increment_published_count(&author1.id.to_string())
        .await
        .unwrap();

    // Create author in tenant 2
    let request2 = CreateAuthorRequest {
        display_name: "Tenant 2 Author".to_owned(),
        bio: None,
        avatar_url: None,
        website_url: None,
    };
    let author2 = manager
        .create(user2_id, tenant2_id, &request2)
        .await
        .unwrap();
    manager
        .increment_published_count(&author2.id.to_string())
        .await
        .unwrap();

    // List popular in tenant 1 - should only see tenant 1 author
    let tenant1_popular = manager.list_popular(tenant1_id, None).await.unwrap();
    assert_eq!(tenant1_popular.len(), 1);
    assert_eq!(tenant1_popular[0].display_name, "Tenant 1 Author");

    // List popular in tenant 2 - should only see tenant 2 author
    let tenant2_popular = manager.list_popular(tenant2_id, None).await.unwrap();
    assert_eq!(tenant2_popular.len(), 1);
    assert_eq!(tenant2_popular[0].display_name, "Tenant 2 Author");

    // Get by user should respect tenant
    let fetched1 = manager.get_by_user(user1_id, tenant1_id).await.unwrap();
    assert!(fetched1.is_some());

    // User 1 should not find author in tenant 2
    let fetched_wrong_tenant = manager.get_by_user(user1_id, tenant2_id).await.unwrap();
    assert!(fetched_wrong_tenant.is_none());
}
