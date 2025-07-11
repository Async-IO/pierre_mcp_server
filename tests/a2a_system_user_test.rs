// ABOUTME: Integration tests for A2A system user creation and management
// ABOUTME: Tests the system user service functionality for A2A client authentication
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use pierre_mcp_server::a2a::system_user::A2ASystemUserService;
use pierre_mcp_server::database_plugins::factory::Database;
use std::sync::Arc;

async fn create_test_database() -> Arc<Database> {
    let database = Database::new("sqlite::memory:", vec![0u8; 32])
        .await
        .expect("Failed to create test database");
    Arc::new(database)
}

#[tokio::test]
async fn test_create_system_user() {
    let database = create_test_database().await;
    let service = A2ASystemUserService::new(database);

    let client_id = "test-client-123";
    let contact_email = "admin@example.com";

    let user_id = service
        .create_or_get_system_user(client_id, contact_email)
        .await
        .expect("Failed to create system user");

    // Verify user was created
    assert!(service
        .is_system_user(user_id)
        .await
        .expect("Failed to check if user is system user"));

    // Verify client ID extraction
    let extracted_client_id = service
        .get_client_id_for_system_user(user_id)
        .await
        .expect("Failed to get client ID for system user");
    assert_eq!(extracted_client_id, Some(client_id.to_string()));
}

#[tokio::test]
async fn test_get_existing_system_user() {
    let database = create_test_database().await;
    let service = A2ASystemUserService::new(database);

    let client_id = "test-client-456";
    let contact_email = "admin@example.com";

    // Create user first time
    let user_id1 = service
        .create_or_get_system_user(client_id, contact_email)
        .await
        .expect("Failed to create system user first time");

    // Get same user second time
    let user_id2 = service
        .create_or_get_system_user(client_id, contact_email)
        .await
        .expect("Failed to create system user second time");

    // Should be the same user
    assert_eq!(user_id1, user_id2);
}

#[tokio::test]
async fn test_password_generation() {
    let database = Arc::new(
        Database::new("sqlite::memory:", vec![0u8; 32])
            .await
            .expect("Failed to create test database"),
    );
    let _service = A2ASystemUserService::new(database);

    let password1 = A2ASystemUserService::generate_secure_system_password();
    let password2 = A2ASystemUserService::generate_secure_system_password();

    // Passwords should be different
    assert_ne!(password1, password2);

    // Should be 64 characters long
    assert_eq!(password1.len(), 64);
    assert_eq!(password2.len(), 64);
}
