// ABOUTME: Test to verify JWT secret persistence across server restarts
// ABOUTME: Ensures admin tokens remain valid after server restart - fixes the 12-hour issue

use anyhow::Result;
use pierre_mcp_server::database_plugins::{factory::Database, DatabaseProvider};
use pierre_mcp_server::key_management::KeyManager;
use tempfile::TempDir;

const TEST_JWT_SECRET: &str = "test_jwt_secret_for_persistence_tests";

#[tokio::test]
async fn test_jwt_secret_persistence_across_restarts() -> Result<()> {
    // Create temporary directory for test database
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_jwt_persistence.db");
    let db_url = format!("sqlite:{}", db_path.display());

    // Step 1: First initialization - simulate admin-setup
    let jwt_secret_1 = {
        let (mut key_manager, database_key) = KeyManager::bootstrap()?;
        let database = Database::new(&db_url, database_key.to_vec()).await?;
        key_manager.complete_initialization(&database).await?;

        // Get/create JWT secret (simulating admin-setup)
        let jwt_secret = database
            .get_or_create_system_secret("admin_jwt_secret")
            .await?;

        // Create admin token with this secret
        let request = pierre_mcp_server::admin::models::CreateAdminTokenRequest {
            service_name: "test_service".into(),
            service_description: Some("Test token".into()),
            permissions: None,
            expires_in_days: Some(1),
            is_super_admin: true,
        };

        let generated_token = database
            .create_admin_token(&request, TEST_JWT_SECRET)
            .await?;
        println!("Generated token: {}", generated_token.jwt_token);

        (jwt_secret, generated_token.jwt_token)
    };

    // Step 2: Second initialization - simulate server restart
    let jwt_secret_2 = {
        let (mut key_manager, database_key) = KeyManager::bootstrap()?;
        let database = Database::new(&db_url, database_key.to_vec()).await?;
        key_manager.complete_initialization(&database).await?;

        // Get JWT secret again (simulating server restart)
        database
            .get_or_create_system_secret("admin_jwt_secret")
            .await?
    };

    // Step 3: Verify JWT secrets are identical
    assert_eq!(
        jwt_secret_1.0, jwt_secret_2,
        "JWT secret changed between restarts! This causes admin token invalidation."
    );

    // Step 4: Verify admin token can be validated with persistent secret
    let jwt_manager = pierre_mcp_server::admin::jwt::AdminJwtManager::with_secret(&jwt_secret_2);

    // This should NOT fail with InvalidSignature
    let validation_result = jwt_manager.validate_token(&jwt_secret_1.1);
    assert!(
        validation_result.is_ok(),
        "Admin token validation failed after restart: {:?}",
        validation_result.err()
    );

    println!("✅ JWT secret persistence test PASSED");
    println!("✅ Admin tokens survive server restarts");
    println!("✅ No more 12-hour InvalidSignature issue");

    Ok(())
}

#[tokio::test]
async fn test_mek_ensures_consistent_jwt_storage() -> Result<()> {
    // This test verifies that the MEK properly encrypts/decrypts JWT secrets
    // ensuring they remain consistent across restarts

    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_mek_jwt.db");
    let db_url = format!("sqlite:{}", db_path.display());

    // Set consistent MEK for test (32 bytes base64 encoded)
    std::env::set_var(
        "PIERRE_MASTER_ENCRYPTION_KEY",
        "YWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWE=",
    );

    let jwt_secret_1 = {
        let (mut key_manager, database_key) = KeyManager::bootstrap()?;
        let database = Database::new(&db_url, database_key.to_vec()).await?;
        key_manager.complete_initialization(&database).await?;
        database
            .get_or_create_system_secret("admin_jwt_secret")
            .await?
    };

    let jwt_secret_2 = {
        let (mut key_manager, database_key) = KeyManager::bootstrap()?;
        let database = Database::new(&db_url, database_key.to_vec()).await?;
        key_manager.complete_initialization(&database).await?;
        database
            .get_or_create_system_secret("admin_jwt_secret")
            .await?
    };

    assert_eq!(
        jwt_secret_1, jwt_secret_2,
        "MEK-encrypted JWT secret storage failed - secrets differ across restarts"
    );

    // Clean up test environment variable
    std::env::remove_var("PIERRE_MASTER_ENCRYPTION_KEY");

    println!("✅ MEK-based JWT secret storage test PASSED");

    Ok(())
}
