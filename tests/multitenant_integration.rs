use anyhow::Result;
use pierre_mcp_server::{
    auth::{AuthManager, generate_jwt_secret},
    database::{Database, generate_encryption_key},
    routes::{AuthRoutes, RegisterRequest, LoginRequest},
};
use tempfile::TempDir;
use uuid::Uuid;

/// Test full multi-tenant authentication flow
#[tokio::test]
async fn test_multitenant_auth_flow() -> Result<()> {
    // Setup
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let database_url = format!("sqlite:{}", db_path.display());
    let encryption_key = generate_encryption_key().to_vec();
    let jwt_secret = generate_jwt_secret().to_vec();
    
    let database = Database::new(&database_url, encryption_key).await?;
    let auth_manager = AuthManager::new(jwt_secret, 24);
    let auth_routes = AuthRoutes::new(database.clone(), auth_manager.clone());
    
    // Test user registration
    let register_request = RegisterRequest {
        email: "test@multitenant.com".to_string(),
        password: "securepassword123".to_string(),
        display_name: Some("Multi-Tenant User".to_string()),
    };
    
    let register_response = auth_routes.register(register_request).await?;
    assert!(!register_response.user_id.is_empty());
    assert_eq!(register_response.message, "User registered successfully");
    
    // Parse user ID
    let user_id = Uuid::parse_str(&register_response.user_id)?;
    
    // Verify user exists in database
    let user = database.get_user(user_id).await?.unwrap();
    assert_eq!(user.email, "test@multitenant.com");
    assert_eq!(user.display_name, Some("Multi-Tenant User".to_string()));
    assert!(user.is_active);
    
    // Test user login
    let login_request = LoginRequest {
        email: "test@multitenant.com".to_string(),
        password: "securepassword123".to_string(),
    };
    
    let login_response = auth_routes.login(login_request).await?;
    assert!(!login_response.jwt_token.is_empty());
    assert_eq!(login_response.user.email, "test@multitenant.com");
    assert_eq!(login_response.user.user_id, register_response.user_id);
    
    // Test JWT token validation
    let claims = auth_manager.validate_token(&login_response.jwt_token)?;
    assert_eq!(claims.email, "test@multitenant.com");
    assert_eq!(claims.sub, register_response.user_id);
    
    // Test duplicate registration fails
    let duplicate_request = RegisterRequest {
        email: "test@multitenant.com".to_string(),
        password: "differentpassword".to_string(),
        display_name: None,
    };
    
    let duplicate_result = auth_routes.register(duplicate_request).await;
    assert!(duplicate_result.is_err());
    assert!(duplicate_result.unwrap_err().to_string().contains("already exists"));
    
    // Test login with wrong password fails
    let wrong_password_request = LoginRequest {
        email: "test@multitenant.com".to_string(),
        password: "wrongpassword".to_string(),
    };
    
    let wrong_password_result = auth_routes.login(wrong_password_request).await;
    assert!(wrong_password_result.is_err());
    assert!(wrong_password_result.unwrap_err().to_string().contains("Invalid email or password"));
    
    Ok(())
}

/// Test database encryption functionality
#[tokio::test]
async fn test_database_encryption() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("encryption_test.db");
    let database_url = format!("sqlite:{}", db_path.display());
    let encryption_key = generate_encryption_key().to_vec();
    
    let database = Database::new(&database_url, encryption_key).await?;
    
    // Create user
    let user = pierre_mcp_server::models::User::new(
        "encryption@test.com".to_string(),
        "bcrypt_hashed_password".to_string(),
        Some("Encryption Test".to_string()),
    );
    let user_id = database.create_user(&user).await?;
    
    // Store encrypted Strava token
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(6);
    database.update_strava_token(
        user_id,
        "secret_access_token_123",
        "secret_refresh_token_456",
        expires_at,
        "read,activity:read_all".to_string(),
    ).await?;
    
    // Retrieve and decrypt token
    let decrypted_token = database.get_strava_token(user_id).await?.unwrap();
    assert_eq!(decrypted_token.access_token, "secret_access_token_123");
    assert_eq!(decrypted_token.refresh_token, "secret_refresh_token_456");
    assert_eq!(decrypted_token.scope, "read,activity:read_all");
    
    Ok(())
}

/// Test JWT authentication edge cases
#[tokio::test]
async fn test_jwt_edge_cases() -> Result<()> {
    let auth_manager = AuthManager::new(generate_jwt_secret().to_vec(), 1); // 1 hour expiry
    
    let user = pierre_mcp_server::models::User::new(
        "jwt@test.com".to_string(),
        "hashed_password".to_string(),
        Some("JWT Test".to_string()),
    );
    
    // Test token generation and validation
    let token = auth_manager.generate_token(&user)?;
    let claims = auth_manager.validate_token(&token)?;
    assert_eq!(claims.email, "jwt@test.com");
    assert_eq!(claims.sub, user.id.to_string());
    
    // Test token refresh
    let refreshed_token = auth_manager.refresh_token(&token, &user)?;
    let refreshed_claims = auth_manager.validate_token(&refreshed_token)?;
    assert_eq!(refreshed_claims.email, claims.email);
    assert_eq!(refreshed_claims.sub, claims.sub);
    
    // Test invalid token
    let invalid_token = "invalid.token.here";
    let invalid_result = auth_manager.validate_token(invalid_token);
    assert!(invalid_result.is_err());
    
    // Test malformed token
    let malformed_token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.malformed.signature";
    let malformed_result = auth_manager.validate_token(malformed_token);
    assert!(malformed_result.is_err());
    
    Ok(())
}

/// Test user isolation in multi-tenant database
#[tokio::test]
async fn test_user_isolation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("isolation_test.db");
    let database_url = format!("sqlite:{}", db_path.display());
    let encryption_key = generate_encryption_key().to_vec();
    
    let database = Database::new(&database_url, encryption_key).await?;
    
    // Create two users
    let user1 = pierre_mcp_server::models::User::new(
        "user1@isolation.test".to_string(),
        "password1".to_string(),
        Some("User One".to_string()),
    );
    let user1_id = database.create_user(&user1).await?;
    
    let user2 = pierre_mcp_server::models::User::new(
        "user2@isolation.test".to_string(),
        "password2".to_string(),
        Some("User Two".to_string()),
    );
    let user2_id = database.create_user(&user2).await?;
    
    // Store tokens for each user
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(6);
    
    database.update_strava_token(
        user1_id,
        "user1_access_token",
        "user1_refresh_token",
        expires_at,
        "read,activity:read_all".to_string(),
    ).await?;
    
    database.update_strava_token(
        user2_id,
        "user2_access_token",
        "user2_refresh_token",
        expires_at,
        "read,activity:read_all".to_string(),
    ).await?;
    
    // Verify user isolation - each user can only access their own tokens
    let user1_token = database.get_strava_token(user1_id).await?.unwrap();
    assert_eq!(user1_token.access_token, "user1_access_token");
    
    let user2_token = database.get_strava_token(user2_id).await?.unwrap();
    assert_eq!(user2_token.access_token, "user2_access_token");
    
    // Verify users cannot access each other's data
    assert_ne!(user1_token.access_token, user2_token.access_token);
    assert_ne!(user1_token.refresh_token, user2_token.refresh_token);
    
    Ok(())
}

/// Test input validation
#[tokio::test]
async fn test_input_validation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("validation_test.db");
    let database_url = format!("sqlite:{}", db_path.display());
    let encryption_key = generate_encryption_key().to_vec();
    let jwt_secret = generate_jwt_secret().to_vec();
    
    let database = Database::new(&database_url, encryption_key).await?;
    let auth_manager = AuthManager::new(jwt_secret, 24);
    let auth_routes = AuthRoutes::new(database, auth_manager);
    
    // Test invalid email formats
    let invalid_emails = vec![
        "not-an-email",
        "@domain.com",
        "user@",
        "user",
        "a@b",
        "",
    ];
    
    for invalid_email in invalid_emails {
        let request = RegisterRequest {
            email: invalid_email.to_string(),
            password: "validpassword123".to_string(),
            display_name: None,
        };
        
        let result = auth_routes.register(request).await;
        assert!(result.is_err(), "Should reject invalid email: {}", invalid_email);
        assert!(result.unwrap_err().to_string().contains("Invalid email format"));
    }
    
    // Test short passwords
    let short_passwords = vec!["1234567", "short", "", "a"];
    
    for short_password in short_passwords {
        let request = RegisterRequest {
            email: "test@valid.com".to_string(),
            password: short_password.to_string(),
            display_name: None,
        };
        
        let result = auth_routes.register(request).await;
        assert!(result.is_err(), "Should reject short password: {}", short_password);
        assert!(result.unwrap_err().to_string().contains("at least 8 characters"));
    }
    
    // Test valid inputs
    let valid_request = RegisterRequest {
        email: "valid@email.com".to_string(),
        password: "validpassword123".to_string(),
        display_name: Some("Valid User".to_string()),
    };
    
    let result = auth_routes.register(valid_request).await;
    assert!(result.is_ok(), "Should accept valid inputs");
    
    Ok(())
}