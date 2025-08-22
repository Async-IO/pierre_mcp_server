// ABOUTME: Integration test for tenant context resolution to verify factory delegation works
// ABOUTME: Tests that tenant operations work through the factory pattern (critical fix for 0% functional architecture)

use chrono::Utc;
use pierre_mcp_server::{
    database_plugins::{factory::Database, DatabaseProvider},
    models::{Tenant, User, UserTier},
};
use uuid::Uuid;

#[tokio::test]
async fn test_tenant_operations_work_through_factory() {
    // Create test database
    let database_url = "sqlite::memory:";
    let encryption_key = vec![0u8; 32];
    let database = Database::new(database_url, encryption_key).await.unwrap();

    // Create owner user first (required for tenant foreign key constraint)
    let owner_id = Uuid::new_v4();
    let owner = User {
        id: owner_id,
        email: "owner@example.com".to_string(),
        display_name: Some("Tenant Owner".to_string()),
        password_hash: "hashed_password".to_string(),
        tier: UserTier::Professional,
        strava_token: None,
        fitbit_token: None,
        tenant_id: Some("test-tenant".to_string()),
        is_active: true,
        user_status: pierre_mcp_server::models::UserStatus::Active,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: Utc::now(),
        last_active: Utc::now(),
    };
    database.create_user(&owner).await.unwrap();

    // Create test tenant
    let tenant_id = Uuid::new_v4();
    let tenant = Tenant {
        id: tenant_id,
        name: "Test Tenant".to_string(),
        slug: "test-tenant".to_string(),
        domain: None,
        plan: "starter".to_string(),
        owner_user_id: owner_id,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // CRITICAL TEST: This should NOT fail with "Tenant management not yet implemented"
    database.create_tenant(&tenant).await.unwrap();

    // CRITICAL TEST: get_tenant_by_id should work (was previously stubbed)
    let retrieved_tenant = database.get_tenant_by_id(tenant_id).await.unwrap();
    assert_eq!(retrieved_tenant.id, tenant_id);
    assert_eq!(retrieved_tenant.name, "Test Tenant");
    assert_eq!(retrieved_tenant.slug, "test-tenant");

    // CRITICAL TEST: get_tenant_by_slug should work (was previously stubbed)
    let retrieved_by_slug = database.get_tenant_by_slug("test-tenant").await.unwrap();
    assert_eq!(retrieved_by_slug.id, tenant_id);

    println!("SUCCESS: Factory delegation is FIXED!");
    println!("   - create_tenant() works (was stubbed)");
    println!("   - get_tenant_by_id() works (was stubbed)");
    println!("   - get_tenant_by_slug() works (was stubbed)");
    println!("   Tenant-aware MCP architecture is now FUNCTIONAL!");
}
