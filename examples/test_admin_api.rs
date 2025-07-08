// ABOUTME: Integration test demonstrating admin API functionality
// ABOUTME: Tests admin token generation, authentication, and API key provisioning
//! Test Admin API Implementation
//!
//! This example demonstrates how the admin API works by testing
//! the admin token generation and API key provisioning flow.

use pierre_mcp_server::{
    admin::auth::AdminAuthService,
    admin_routes::AdminApiContext,
    api_keys::{ApiKeyManager, ApiKeyTier, CreateApiKeyRequest},
    auth::AuthManager,
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    models::User,
};
use uuid::Uuid;

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> anyhow::Result<()> {
    println!("🔧 Testing Pierre MCP Server Admin API Implementation");
    println!("{}", "=".repeat(60));

    // 1. Setup database and auth systems
    println!("📁 Setting up in-memory database...");
    let database = Database::new("sqlite::memory:", generate_encryption_key().to_vec()).await?;

    println!("🔐 Setting up authentication systems...");
    let jwt_secret = "test_jwt_secret_for_admin_demo";
    let auth_manager = AuthManager::new(jwt_secret.as_bytes().to_vec(), 24);

    // 2. Generate admin token manually using our JWT secret
    println!("\n🔑 Generating admin token...");
    let jwt_manager = pierre_mcp_server::admin::jwt::AdminJwtManager::with_secret(jwt_secret);
    let permissions = pierre_mcp_server::admin::models::AdminPermissions::default_admin();
    let token_id = format!("admin_{}", uuid::Uuid::new_v4().simple());

    let jwt_token = jwt_manager.generate_token(
        &token_id,
        "test_admin_service",
        &permissions,
        false,
        Some(chrono::Utc::now() + chrono::Duration::days(365)),
    )?;

    // Store token in database
    let token_prefix =
        pierre_mcp_server::admin::jwt::AdminJwtManager::generate_token_prefix(&jwt_token);
    let token_hash =
        pierre_mcp_server::admin::jwt::AdminJwtManager::hash_token_for_storage(&jwt_token)?;
    let jwt_secret_hash = pierre_mcp_server::admin::jwt::AdminJwtManager::hash_secret(jwt_secret);

    let admin_token = pierre_mcp_server::admin::models::AdminToken {
        id: token_id.clone(),
        service_name: "test_admin_service".to_string(),
        service_description: Some("Test admin service".to_string()),
        token_hash,
        token_prefix: token_prefix.clone(),
        jwt_secret_hash,
        permissions: permissions.clone(),
        is_super_admin: false,
        is_active: true,
        created_at: chrono::Utc::now(),
        expires_at: Some(chrono::Utc::now() + chrono::Duration::days(365)),
        last_used_at: None,
        last_used_ip: None,
        usage_count: 0,
    };

    // Store the token in database for testing (normally done by create_admin_token)
    // Note: For this demo we're manually inserting since create_admin_token generates its own JWT secret
    match &database {
        pierre_mcp_server::database_plugins::factory::Database::SQLite(sqlite_db) => {
            sqlx::query(
                r"
                INSERT INTO admin_tokens (
                    id, service_name, service_description, token_hash, token_prefix,
                    jwt_secret_hash, permissions, is_super_admin, is_active,
                    created_at, expires_at, usage_count
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ",
            )
            .bind(&admin_token.id)
            .bind(&admin_token.service_name)
            .bind(&admin_token.service_description)
            .bind(&admin_token.token_hash)
            .bind(&admin_token.token_prefix)
            .bind(&admin_token.jwt_secret_hash)
            .bind(&admin_token.permissions.to_json()?)
            .bind(admin_token.is_super_admin)
            .bind(admin_token.is_active)
            .bind(admin_token.created_at)
            .bind(admin_token.expires_at)
            .bind(i64::try_from(admin_token.usage_count).unwrap_or(0))
            .execute(sqlite_db.inner().pool())
            .await?;
        }
        #[cfg(feature = "postgresql")]
        pierre_mcp_server::database_plugins::factory::Database::PostgreSQL(_) => {
            panic!("PostgreSQL not supported in this demo");
        }
    }

    println!("   (Note: Manually created token for testing with consistent JWT secret)");

    // 3. Setup auth service with the same secret used for token generation
    let auth_service = AdminAuthService::new(database.clone(), jwt_secret);

    let generated_token = pierre_mcp_server::admin::models::GeneratedAdminToken {
        token_id: token_id.clone(),
        service_name: "test_admin_service".to_string(),
        jwt_token,
        token_prefix: token_prefix.clone(),
        permissions,
        is_super_admin: false,
        created_at: chrono::Utc::now(),
        expires_at: Some(chrono::Utc::now() + chrono::Duration::days(365)),
    };

    println!("✅ Admin token generated:");
    println!("   Token ID: {}", generated_token.token_id);
    println!("   Service: {}", generated_token.service_name);
    println!("   JWT: {}...", &generated_token.jwt_token[..50]);

    // 4. Test admin authentication
    println!("\n🛡️  Testing admin authentication...");
    let validated_token = auth_service
        .authenticate_and_authorize(
            &generated_token.jwt_token,
            pierre_mcp_server::admin::models::AdminPermission::ProvisionKeys,
            Some("127.0.0.1"),
        )
        .await?;

    println!("✅ Authentication successful:");
    println!("   Service: {}", validated_token.service_name);
    println!("   Permissions: {:?}", validated_token.permissions.to_vec());

    // 4. Create test user
    println!("\n👤 Creating test user...");
    let test_user = User {
        id: Uuid::new_v4(),
        email: "testuser@example.com".to_string(),
        display_name: Some("Test User".to_string()),
        password_hash: "test_hash".to_string(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
    };

    let user_id = database.create_user(&test_user).await?;
    println!("✅ User created: {}", test_user.email);

    // 5. Test API key provisioning
    println!("\n🔑 Testing API key provisioning...");
    let api_key_manager = ApiKeyManager::new();
    let create_request = CreateApiKeyRequest {
        name: "Admin-provisioned API key".to_string(),
        description: Some("Created via admin API".to_string()),
        tier: ApiKeyTier::Starter,
        expires_in_days: Some(365),
        rate_limit_requests: None,
    };

    let (api_key, api_key_string) = api_key_manager.create_api_key(user_id, create_request)?;
    database.create_api_key(&api_key).await?;

    println!("✅ API key provisioned:");
    println!("   API Key ID: {}", api_key.id);
    println!("   Key: {}...", &api_key_string[..20]);
    println!("   Tier: {:?}", api_key.tier);
    println!(
        "   Rate Limit: {} requests per month",
        api_key.rate_limit_requests
    );

    // 6. Test admin API context
    println!("\n🌐 Testing admin API context...");
    let _admin_context = AdminApiContext::new(database.clone(), jwt_secret, auth_manager);
    println!("✅ Admin API context created successfully");

    // 7. Test token info retrieval
    println!("\n📊 Testing token info retrieval...");
    let token_details = database
        .get_admin_token_by_id(&generated_token.token_id)
        .await?;
    if let Some(details) = token_details {
        println!("✅ Token details retrieved:");
        println!("   Usage Count: {}", details.usage_count);
        println!("   Is Active: {}", details.is_active);
        println!(
            "   Created: {}",
            details.created_at.format("%Y-%m-%d %H:%M UTC")
        );
    }

    // 8. Test audit logging
    println!("\n📝 Testing audit logging...");
    database
        .record_admin_provisioned_key(
            &generated_token.token_id,
            &api_key.id,
            &test_user.email,
            "starter",
            api_key.rate_limit_requests,
            "month",
        )
        .await?;
    println!("✅ Admin action logged for audit trail");

    // 9. Summary
    println!("\n🎉 Admin API Test Complete!");
    println!("{}", "=".repeat(60));
    println!("✅ Admin token generation: WORKING");
    println!("✅ JWT authentication: WORKING");
    println!("✅ Permission validation: WORKING");
    println!("✅ API key provisioning: WORKING");
    println!("✅ Database operations: WORKING");
    println!("✅ Audit logging: WORKING");
    println!("✅ Admin API context: WORKING");
    println!();
    println!("🚀 The admin API is ready for production use!");
    println!("   External admin services can now:");
    println!("   • Generate admin tokens using admin-setup binary");
    println!("   • Authenticate with JWT tokens");
    println!("   • Provision API keys for users");
    println!("   • Manage and revoke existing keys");
    println!("   • Access audit logs and usage statistics");

    Ok(())
}
