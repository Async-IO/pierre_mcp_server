// Integration tests for routes.rs module
// Tests for authentication routes, OAuth routes, and A2A routes

use pierre_mcp_server::{
    auth::AuthManager,
    database_plugins::factory::Database,
    routes::{AuthRoutes, RegisterRequest},
};
use tempfile::TempDir;

#[tokio::test]
async fn test_email_validation() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db_path_str = db_path.display();
    let database = Database::new(&format!("sqlite:{db_path_str}"), vec![0u8; 32])
        .await
        .unwrap();
    tracing::trace!("Created test database: {:?}", std::ptr::addr_of!(database));
    let auth_manager = AuthManager::new(vec![0u8; 64], 24);
    tracing::trace!(
        "Created test auth manager: {:?}",
        std::ptr::addr_of!(auth_manager)
    );
    // Email and password validation functions are now static, no need for routes instance
    assert!(AuthRoutes::is_valid_email("test@example.com"));
    assert!(AuthRoutes::is_valid_email("user.name+tag@domain.co.uk"));
    assert!(!AuthRoutes::is_valid_email("invalid-email"));
    assert!(!AuthRoutes::is_valid_email("@domain.com"));
    assert!(!AuthRoutes::is_valid_email("user@"));
}

#[tokio::test]
async fn test_password_validation() {
    // Password validation function is now static, no need for database setup
    assert!(AuthRoutes::is_valid_password("password123"));
    assert!(AuthRoutes::is_valid_password("verylongpassword"));
    assert!(!AuthRoutes::is_valid_password("short"));
    assert!(!AuthRoutes::is_valid_password("1234567"));
}

#[tokio::test]
async fn test_register_user() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db_path_str = db_path.display();
    let database = Database::new(&format!("sqlite:{db_path_str}"), vec![0u8; 32])
        .await
        .unwrap();
    let auth_manager = AuthManager::new(vec![0u8; 64], 24);
    let routes = AuthRoutes::new(database, auth_manager);

    let request = RegisterRequest {
        email: "test@example.com".into(),
        password: "password123".into(),
        display_name: Some("Test User".into()),
    };

    let response = routes.register(request).await.unwrap();
    assert!(!response.user_id.is_empty());
    assert_eq!(
        response.message,
        "User registered successfully. Your account is pending admin approval."
    );
}

#[tokio::test]
async fn test_register_duplicate_user() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db_path_str = db_path.display();
    let database = Database::new(&format!("sqlite:{db_path_str}"), vec![0u8; 32])
        .await
        .unwrap();
    let auth_manager = AuthManager::new(vec![0u8; 64], 24);
    let routes = AuthRoutes::new(database, auth_manager);

    let request = RegisterRequest {
        email: "test@example.com".into(),
        password: "password123".into(),
        display_name: Some("Test User".into()),
    };

    // First registration should succeed
    routes.register(request.clone()).await.unwrap();

    // Second registration should fail
    let result = routes.register(request).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}
