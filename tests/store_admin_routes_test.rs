// ABOUTME: Integration tests for Coach Store admin review routes
// ABOUTME: Tests admin approval/rejection workflow for Store coach submissions
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

mod common;
mod helpers;

use anyhow::Result;
use helpers::axum_test::AxumTestRequest;
use pierre_mcp_server::{
    admin::{
        jwt::AdminJwtManager,
        models::{AdminPermission, AdminPermissions, GeneratedAdminToken},
        AdminAuthService,
    },
    constants::system_config::STARTER_MONTHLY_LIMIT,
    database::coaches::{
        CoachCategory, CoachVisibility, CoachesManager, CreateSystemCoachRequest, PublishStatus,
    },
    database::Coach,
    database_plugins::{factory::Database, DatabaseProvider},
    mcp::ToolSelectionService,
    models::TenantId,
    routes::admin::{AdminApiContext, AdminRoutes},
};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// Test Setup Helper
// ============================================================================

/// Test setup helper for admin store review testing
struct StoreAdminTestSetup {
    context: AdminApiContext,
    database: Arc<Database>,
    super_admin_token: GeneratedAdminToken,
    user_manager_token: GeneratedAdminToken,
    no_permission_token: GeneratedAdminToken,
    user_id: Uuid,
    tenant_id: TenantId,
}

impl StoreAdminTestSetup {
    async fn new() -> Result<Self> {
        // Create test database
        let database = common::create_test_database().await?;
        let auth_manager = common::create_test_auth_manager();

        // Create JWKS manager for RS256
        let jwks_manager = common::get_shared_test_jwks();

        // Create admin context
        let jwt_secret = "test_admin_jwt_secret_for_store_review_testing";
        let admin_api_key_monthly_limit = STARTER_MONTHLY_LIMIT;
        let database_arc = Arc::new((*database).clone());
        let tool_selection = Arc::new(ToolSelectionService::new(database_arc.clone()));
        let context = AdminApiContext::new(
            database_arc.clone(),
            jwt_secret,
            auth_manager.clone(),
            jwks_manager.clone(),
            admin_api_key_monthly_limit,
            AdminAuthService::DEFAULT_CACHE_TTL_SECS,
            tool_selection,
        );

        // Create test user
        let (user_id, _user) = common::create_test_user(&database).await?;
        let tenants = database.list_tenants_for_user(user_id).await?;
        let tenant_id = tenants
            .first()
            .map_or_else(|| TenantId::from(user_id), |t| t.id);

        // Create JWT manager
        let jwt_manager = AdminJwtManager::new();

        // Create super admin token
        let super_admin_permissions = AdminPermissions::super_admin();
        let super_admin_token_id = format!("admin_{}", Uuid::new_v4().simple());
        let super_admin_jwt = jwt_manager.generate_token(
            &super_admin_token_id,
            "test_super_admin_service",
            &super_admin_permissions,
            true,
            None,
            &jwks_manager,
        )?;

        let super_admin_token = GeneratedAdminToken {
            token_id: super_admin_token_id.clone(),
            service_name: "test_super_admin_service".to_owned(),
            jwt_token: super_admin_jwt.clone(),
            token_prefix: AdminJwtManager::generate_token_prefix(&super_admin_jwt),
            permissions: super_admin_permissions.clone(),
            is_super_admin: true,
            expires_at: None,
            created_at: chrono::Utc::now(),
        };
        Self::insert_admin_token_to_db(&database, &super_admin_token, jwt_secret).await?;

        // Create user manager token (ManageUsers permission)
        let user_manager_permissions = AdminPermissions::new(vec![AdminPermission::ManageUsers]);
        let user_manager_token_id = format!("admin_{}", Uuid::new_v4().simple());
        let user_manager_jwt = jwt_manager.generate_token(
            &user_manager_token_id,
            "test_user_manager_service",
            &user_manager_permissions,
            false,
            Some(chrono::Utc::now() + chrono::Duration::days(365)),
            &jwks_manager,
        )?;

        let user_manager_token = GeneratedAdminToken {
            token_id: user_manager_token_id.clone(),
            service_name: "test_user_manager_service".to_owned(),
            jwt_token: user_manager_jwt.clone(),
            token_prefix: AdminJwtManager::generate_token_prefix(&user_manager_jwt),
            permissions: user_manager_permissions.clone(),
            is_super_admin: false,
            expires_at: Some(chrono::Utc::now() + chrono::Duration::days(365)),
            created_at: chrono::Utc::now(),
        };
        Self::insert_admin_token_to_db(&database, &user_manager_token, jwt_secret).await?;

        // Create token without permission
        let no_permission_permissions = AdminPermissions::new(vec![AdminPermission::ListKeys]);
        let no_permission_token_id = format!("admin_{}", Uuid::new_v4().simple());
        let no_permission_jwt = jwt_manager.generate_token(
            &no_permission_token_id,
            "test_no_permission_service",
            &no_permission_permissions,
            false,
            Some(chrono::Utc::now() + chrono::Duration::days(365)),
            &jwks_manager,
        )?;

        let no_permission_token = GeneratedAdminToken {
            token_id: no_permission_token_id.clone(),
            service_name: "test_no_permission_service".to_owned(),
            jwt_token: no_permission_jwt.clone(),
            token_prefix: AdminJwtManager::generate_token_prefix(&no_permission_jwt),
            permissions: no_permission_permissions.clone(),
            is_super_admin: false,
            expires_at: Some(chrono::Utc::now() + chrono::Duration::days(365)),
            created_at: chrono::Utc::now(),
        };
        Self::insert_admin_token_to_db(&database, &no_permission_token, jwt_secret).await?;

        Ok(Self {
            context,
            database: database_arc,
            super_admin_token,
            user_manager_token,
            no_permission_token,
            user_id,
            tenant_id,
        })
    }

    fn auth_header(token: &str) -> String {
        format!("Bearer {token}")
    }

    fn routes(&self) -> axum::Router {
        AdminRoutes::routes(self.context.clone())
    }

    /// Create a coach in `pending_review` status
    async fn create_pending_review_coach(&self, title: &str) -> Result<Coach> {
        let sqlite_pool = self.database.sqlite_pool().unwrap().clone();
        let coaches_manager = CoachesManager::new(sqlite_pool);

        let system_request = CreateSystemCoachRequest {
            title: title.to_owned(),
            description: Some(format!("Description for {title}")),
            system_prompt: format!("You are a {title} coach."),
            category: CoachCategory::Training,
            tags: vec!["test".to_owned()],
            visibility: CoachVisibility::Tenant,
            sample_prompts: vec!["Sample prompt".to_owned()],
        };

        let coach = coaches_manager
            .create_system_coach(self.user_id, self.tenant_id, &system_request)
            .await?;

        // Submit for review
        coaches_manager
            .submit_for_review(&coach.id.to_string(), self.user_id, self.tenant_id)
            .await?;

        Ok(coaches_manager
            .get(&coach.id.to_string(), self.user_id, self.tenant_id)
            .await?
            .unwrap())
    }

    /// Helper to insert admin token into database
    async fn insert_admin_token_to_db(
        database: &Database,
        token: &GeneratedAdminToken,
        jwt_secret: &str,
    ) -> Result<()> {
        let token_hash = AdminJwtManager::hash_token_for_storage(&token.jwt_token)?;
        let jwt_secret_hash = AdminJwtManager::hash_secret(jwt_secret);
        let permissions_json = token.permissions.to_json()?;

        match database {
            Database::SQLite(sqlite_db) => {
                let query = r"
                    INSERT INTO admin_tokens (
                        id, service_name, service_description, token_hash, token_prefix,
                        jwt_secret_hash, permissions, is_super_admin, is_active,
                        created_at, expires_at, usage_count
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ";

                sqlx::query(query)
                    .bind(&token.token_id)
                    .bind(&token.service_name)
                    .bind(Some("Test admin token"))
                    .bind(&token_hash)
                    .bind(&token.token_prefix)
                    .bind(&jwt_secret_hash)
                    .bind(&permissions_json)
                    .bind(token.is_super_admin)
                    .bind(true)
                    .bind(token.created_at)
                    .bind(token.expires_at)
                    .bind(0)
                    .execute(sqlite_db.pool())
                    .await?;
            }
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(_) => {
                return Err(anyhow::anyhow!("PostgreSQL not supported in test helper"));
            }
        }

        Ok(())
    }
}

// ============================================================================
// List Pending Coaches Tests
// ============================================================================

#[tokio::test]
async fn test_list_pending_coaches_empty() -> Result<()> {
    let setup = StoreAdminTestSetup::new().await?;
    let routes = setup.routes();
    let auth_header = StoreAdminTestSetup::auth_header(&setup.super_admin_token.jwt_token);

    let response = AxumTestRequest::get(&format!(
        "/admin/store/pending?tenant_id={}",
        setup.tenant_id
    ))
    .header("authorization", &auth_header)
    .send(routes)
    .await;

    assert_eq!(response.status(), 200);

    let result: Value = response.json();
    assert!(result["coaches"].is_array());
    assert_eq!(result["coaches"].as_array().unwrap().len(), 0);
    Ok(())
}

#[tokio::test]
async fn test_list_pending_coaches_with_coaches() -> Result<()> {
    let setup = StoreAdminTestSetup::new().await?;
    let routes = setup.routes();
    let auth_header = StoreAdminTestSetup::auth_header(&setup.super_admin_token.jwt_token);

    // Create coaches in pending_review status
    setup.create_pending_review_coach("Pending Coach 1").await?;
    setup.create_pending_review_coach("Pending Coach 2").await?;

    let response = AxumTestRequest::get(&format!(
        "/admin/store/pending?tenant_id={}",
        setup.tenant_id
    ))
    .header("authorization", &auth_header)
    .send(routes)
    .await;

    assert_eq!(response.status(), 200);

    let result: Value = response.json();
    let coaches = result["coaches"].as_array().unwrap();
    assert_eq!(coaches.len(), 2);
    Ok(())
}

#[tokio::test]
async fn test_list_pending_coaches_with_user_manager_permission() -> Result<()> {
    let setup = StoreAdminTestSetup::new().await?;
    let routes = setup.routes();
    let auth_header = StoreAdminTestSetup::auth_header(&setup.user_manager_token.jwt_token);

    setup.create_pending_review_coach("Pending Coach").await?;

    let response = AxumTestRequest::get(&format!(
        "/admin/store/pending?tenant_id={}",
        setup.tenant_id
    ))
    .header("authorization", &auth_header)
    .send(routes)
    .await;

    assert_eq!(response.status(), 200);

    let result: Value = response.json();
    let coaches = result["coaches"].as_array().unwrap();
    assert_eq!(coaches.len(), 1);
    Ok(())
}

#[tokio::test]
async fn test_list_pending_coaches_insufficient_permission() -> Result<()> {
    let setup = StoreAdminTestSetup::new().await?;
    let routes = setup.routes();
    let auth_header = StoreAdminTestSetup::auth_header(&setup.no_permission_token.jwt_token);

    let response = AxumTestRequest::get(&format!(
        "/admin/store/pending?tenant_id={}",
        setup.tenant_id
    ))
    .header("authorization", &auth_header)
    .send(routes)
    .await;

    assert_eq!(response.status(), 403);
    Ok(())
}

#[tokio::test]
async fn test_list_pending_coaches_with_pagination() -> Result<()> {
    let setup = StoreAdminTestSetup::new().await?;
    let routes = setup.routes();
    let auth_header = StoreAdminTestSetup::auth_header(&setup.super_admin_token.jwt_token);

    // Create 5 pending coaches
    for i in 1..=5 {
        setup
            .create_pending_review_coach(&format!("Pending Coach {i}"))
            .await?;
    }

    // Get first page
    let response = AxumTestRequest::get(&format!(
        "/admin/store/pending?tenant_id={}&limit=2&offset=0",
        setup.tenant_id
    ))
    .header("authorization", &auth_header)
    .send(routes.clone())
    .await;

    assert_eq!(response.status(), 200);
    let result: Value = response.json();
    let coaches = result["coaches"].as_array().unwrap();
    assert_eq!(coaches.len(), 2);

    // Get second page
    let response = AxumTestRequest::get(&format!(
        "/admin/store/pending?tenant_id={}&limit=2&offset=2",
        setup.tenant_id
    ))
    .header("authorization", &auth_header)
    .send(routes)
    .await;

    assert_eq!(response.status(), 200);
    let result: Value = response.json();
    let coaches = result["coaches"].as_array().unwrap();
    assert_eq!(coaches.len(), 2);
    Ok(())
}

// ============================================================================
// Approve Coach Tests
// ============================================================================

#[tokio::test]
async fn test_approve_coach_success() -> Result<()> {
    let setup = StoreAdminTestSetup::new().await?;
    let routes = setup.routes();
    let auth_header = StoreAdminTestSetup::auth_header(&setup.super_admin_token.jwt_token);

    let coach = setup.create_pending_review_coach("Approve Me").await?;

    let response = AxumTestRequest::post(&format!(
        "/admin/store/coaches/{}/approve?tenant_id={}",
        coach.id, setup.tenant_id
    ))
    .header("authorization", &auth_header)
    .send(routes)
    .await;

    assert_eq!(response.status(), 200);

    let result: Value = response.json();
    assert!(result["message"].as_str().unwrap().contains("approved"));
    assert_eq!(
        result["coach"]["publish_status"].as_str().unwrap(),
        "published"
    );
    Ok(())
}

#[tokio::test]
async fn test_approve_coach_with_user_manager_permission() -> Result<()> {
    let setup = StoreAdminTestSetup::new().await?;
    let routes = setup.routes();
    let auth_header = StoreAdminTestSetup::auth_header(&setup.user_manager_token.jwt_token);

    let coach = setup.create_pending_review_coach("Manager Approve").await?;

    let response = AxumTestRequest::post(&format!(
        "/admin/store/coaches/{}/approve?tenant_id={}",
        coach.id, setup.tenant_id
    ))
    .header("authorization", &auth_header)
    .send(routes)
    .await;

    assert_eq!(response.status(), 200);
    Ok(())
}

#[tokio::test]
async fn test_approve_coach_insufficient_permission() -> Result<()> {
    let setup = StoreAdminTestSetup::new().await?;
    let routes = setup.routes();
    let auth_header = StoreAdminTestSetup::auth_header(&setup.no_permission_token.jwt_token);

    let coach = setup.create_pending_review_coach("No Permission").await?;

    let response = AxumTestRequest::post(&format!(
        "/admin/store/coaches/{}/approve?tenant_id={}",
        coach.id, setup.tenant_id
    ))
    .header("authorization", &auth_header)
    .send(routes)
    .await;

    assert_eq!(response.status(), 403);
    Ok(())
}

#[tokio::test]
async fn test_approve_coach_not_found() -> Result<()> {
    let setup = StoreAdminTestSetup::new().await?;
    let routes = setup.routes();
    let auth_header = StoreAdminTestSetup::auth_header(&setup.super_admin_token.jwt_token);

    let fake_id = Uuid::new_v4();
    let response = AxumTestRequest::post(&format!(
        "/admin/store/coaches/{}/approve?tenant_id={}",
        fake_id, setup.tenant_id
    ))
    .header("authorization", &auth_header)
    .send(routes)
    .await;

    // Should return error for non-existent coach
    assert!(response.status() == 400 || response.status() == 404);
    Ok(())
}

#[tokio::test]
async fn test_approve_coach_not_pending_review() -> Result<()> {
    let setup = StoreAdminTestSetup::new().await?;
    let routes = setup.routes();
    let auth_header = StoreAdminTestSetup::auth_header(&setup.super_admin_token.jwt_token);

    // Create a coach but don't submit for review (it's in draft status)
    let sqlite_pool = setup.database.sqlite_pool().unwrap().clone();
    let coaches_manager = CoachesManager::new(sqlite_pool);

    let system_request = CreateSystemCoachRequest {
        title: "Draft Coach".to_owned(),
        description: None,
        system_prompt: "Prompt".to_owned(),
        category: CoachCategory::Training,
        tags: vec![],
        visibility: CoachVisibility::Private,
        sample_prompts: vec![],
    };
    let coach = coaches_manager
        .create_system_coach(setup.user_id, setup.tenant_id, &system_request)
        .await?;

    let response = AxumTestRequest::post(&format!(
        "/admin/store/coaches/{}/approve?tenant_id={}",
        coach.id, setup.tenant_id
    ))
    .header("authorization", &auth_header)
    .send(routes)
    .await;

    // Should fail because coach is not in pending_review status
    assert!(response.status() == 400 || response.status() == 404);
    Ok(())
}

// ============================================================================
// Reject Coach Tests
// ============================================================================

#[tokio::test]
async fn test_reject_coach_success() -> Result<()> {
    let setup = StoreAdminTestSetup::new().await?;
    let routes = setup.routes();
    let auth_header = StoreAdminTestSetup::auth_header(&setup.super_admin_token.jwt_token);

    let coach = setup.create_pending_review_coach("Reject Me").await?;

    let response = AxumTestRequest::post(&format!(
        "/admin/store/coaches/{}/reject?tenant_id={}",
        coach.id, setup.tenant_id
    ))
    .header("authorization", &auth_header)
    .json(&serde_json::json!({
        "reason": "Does not meet quality guidelines"
    }))
    .send(routes)
    .await;

    assert_eq!(response.status(), 200);

    let result: Value = response.json();
    assert!(result["message"].as_str().unwrap().contains("rejected"));
    assert_eq!(
        result["coach"]["publish_status"].as_str().unwrap(),
        "rejected"
    );
    Ok(())
}

#[tokio::test]
async fn test_reject_coach_with_user_manager_permission() -> Result<()> {
    let setup = StoreAdminTestSetup::new().await?;
    let routes = setup.routes();
    let auth_header = StoreAdminTestSetup::auth_header(&setup.user_manager_token.jwt_token);

    let coach = setup.create_pending_review_coach("Manager Reject").await?;

    let response = AxumTestRequest::post(&format!(
        "/admin/store/coaches/{}/reject?tenant_id={}",
        coach.id, setup.tenant_id
    ))
    .header("authorization", &auth_header)
    .json(&serde_json::json!({
        "reason": "Content policy violation"
    }))
    .send(routes)
    .await;

    assert_eq!(response.status(), 200);
    Ok(())
}

#[tokio::test]
async fn test_reject_coach_insufficient_permission() -> Result<()> {
    let setup = StoreAdminTestSetup::new().await?;
    let routes = setup.routes();
    let auth_header = StoreAdminTestSetup::auth_header(&setup.no_permission_token.jwt_token);

    let coach = setup
        .create_pending_review_coach("No Permission Reject")
        .await?;

    let response = AxumTestRequest::post(&format!(
        "/admin/store/coaches/{}/reject?tenant_id={}",
        coach.id, setup.tenant_id
    ))
    .header("authorization", &auth_header)
    .json(&serde_json::json!({
        "reason": "Some reason"
    }))
    .send(routes)
    .await;

    assert_eq!(response.status(), 403);
    Ok(())
}

#[tokio::test]
async fn test_reject_coach_empty_reason() -> Result<()> {
    let setup = StoreAdminTestSetup::new().await?;
    let routes = setup.routes();
    let auth_header = StoreAdminTestSetup::auth_header(&setup.super_admin_token.jwt_token);

    let coach = setup.create_pending_review_coach("Empty Reason").await?;

    let response = AxumTestRequest::post(&format!(
        "/admin/store/coaches/{}/reject?tenant_id={}",
        coach.id, setup.tenant_id
    ))
    .header("authorization", &auth_header)
    .json(&serde_json::json!({
        "reason": ""
    }))
    .send(routes)
    .await;

    // Should fail because reason is required
    assert_eq!(response.status(), 400);
    Ok(())
}

#[tokio::test]
async fn test_reject_coach_whitespace_only_reason() -> Result<()> {
    let setup = StoreAdminTestSetup::new().await?;
    let routes = setup.routes();
    let auth_header = StoreAdminTestSetup::auth_header(&setup.super_admin_token.jwt_token);

    let coach = setup
        .create_pending_review_coach("Whitespace Reason")
        .await?;

    let response = AxumTestRequest::post(&format!(
        "/admin/store/coaches/{}/reject?tenant_id={}",
        coach.id, setup.tenant_id
    ))
    .header("authorization", &auth_header)
    .json(&serde_json::json!({
        "reason": "   "
    }))
    .send(routes)
    .await;

    // Should fail because whitespace-only reason is not valid
    assert_eq!(response.status(), 400);
    Ok(())
}

#[tokio::test]
async fn test_reject_coach_not_found() -> Result<()> {
    let setup = StoreAdminTestSetup::new().await?;
    let routes = setup.routes();
    let auth_header = StoreAdminTestSetup::auth_header(&setup.super_admin_token.jwt_token);

    let fake_id = Uuid::new_v4();
    let response = AxumTestRequest::post(&format!(
        "/admin/store/coaches/{}/reject?tenant_id={}",
        fake_id, setup.tenant_id
    ))
    .header("authorization", &auth_header)
    .json(&serde_json::json!({
        "reason": "Not found"
    }))
    .send(routes)
    .await;

    // Should return error for non-existent coach
    assert!(response.status() == 400 || response.status() == 404);
    Ok(())
}

// ============================================================================
// Authentication Tests
// ============================================================================

#[tokio::test]
async fn test_admin_store_routes_require_auth() -> Result<()> {
    let setup = StoreAdminTestSetup::new().await?;
    let routes = setup.routes();

    // Try accessing without auth
    // The middleware returns 400 Bad Request for missing auth header
    let response = AxumTestRequest::get(&format!(
        "/admin/store/pending?tenant_id={}",
        setup.tenant_id
    ))
    .send(routes.clone())
    .await;

    assert!(
        response.status() == 400 || response.status() == 401,
        "Expected 400 or 401 for missing auth, got {}",
        response.status()
    );

    // Try with invalid token
    // The middleware returns 401 Unauthorized for invalid tokens
    let response = AxumTestRequest::get(&format!(
        "/admin/store/pending?tenant_id={}",
        setup.tenant_id
    ))
    .header("authorization", "Bearer invalid_token")
    .send(routes)
    .await;

    assert!(
        response.status() == 400 || response.status() == 401,
        "Expected 400 or 401 for invalid token, got {}",
        response.status()
    );
    Ok(())
}

// ============================================================================
// Workflow Integration Tests
// ============================================================================

#[tokio::test]
async fn test_full_approval_workflow() -> Result<()> {
    let setup = StoreAdminTestSetup::new().await?;
    let routes = setup.routes();
    let auth_header = StoreAdminTestSetup::auth_header(&setup.super_admin_token.jwt_token);

    // Create coach in pending review
    let coach = setup.create_pending_review_coach("Workflow Coach").await?;
    assert_eq!(coach.publish_status, PublishStatus::PendingReview);

    // List pending - should find coach
    let response = AxumTestRequest::get(&format!(
        "/admin/store/pending?tenant_id={}",
        setup.tenant_id
    ))
    .header("authorization", &auth_header)
    .send(routes.clone())
    .await;

    assert_eq!(response.status(), 200);
    let result: Value = response.json();
    let coaches = result["coaches"].as_array().unwrap();
    assert_eq!(coaches.len(), 1);

    // Approve coach
    let response = AxumTestRequest::post(&format!(
        "/admin/store/coaches/{}/approve?tenant_id={}",
        coach.id, setup.tenant_id
    ))
    .header("authorization", &auth_header)
    .send(routes.clone())
    .await;

    assert_eq!(response.status(), 200);

    // List pending - should be empty now
    let response = AxumTestRequest::get(&format!(
        "/admin/store/pending?tenant_id={}",
        setup.tenant_id
    ))
    .header("authorization", &auth_header)
    .send(routes)
    .await;

    assert_eq!(response.status(), 200);
    let result: Value = response.json();
    let coaches = result["coaches"].as_array().unwrap();
    assert_eq!(coaches.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_full_rejection_workflow() -> Result<()> {
    let setup = StoreAdminTestSetup::new().await?;
    let routes = setup.routes();
    let auth_header = StoreAdminTestSetup::auth_header(&setup.super_admin_token.jwt_token);

    // Create coach in pending review
    let coach = setup
        .create_pending_review_coach("Rejection Workflow Coach")
        .await?;
    assert_eq!(coach.publish_status, PublishStatus::PendingReview);

    // Reject coach
    let response = AxumTestRequest::post(&format!(
        "/admin/store/coaches/{}/reject?tenant_id={}",
        coach.id, setup.tenant_id
    ))
    .header("authorization", &auth_header)
    .json(&serde_json::json!({
        "reason": "Does not meet quality standards"
    }))
    .send(routes.clone())
    .await;

    assert_eq!(response.status(), 200);
    let result: Value = response.json();
    assert_eq!(
        result["coach"]["publish_status"].as_str().unwrap(),
        "rejected"
    );

    // List pending - should be empty
    let response = AxumTestRequest::get(&format!(
        "/admin/store/pending?tenant_id={}",
        setup.tenant_id
    ))
    .header("authorization", &auth_header)
    .send(routes)
    .await;

    assert_eq!(response.status(), 200);
    let result: Value = response.json();
    let coaches = result["coaches"].as_array().unwrap();
    assert_eq!(coaches.len(), 0);

    Ok(())
}
