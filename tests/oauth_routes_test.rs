// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Unit tests for OAuth routes module

use pierre_mcp_server::{
    routes::{AuthRoutes, RegisterRequest, LoginRequest},
    database::{Database, generate_encryption_key},
    auth::AuthManager,
};

#[tokio::test]
async fn test_email_validation() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key).await.unwrap();
    let auth_manager = AuthManager::new(vec![0u8; 64], 24);
    let auth_routes = AuthRoutes::new(database, auth_manager);
    
    // Valid emails
    let valid_emails = vec![
        "test@example.com",
        "user.name@domain.com",
        "user+tag@example.co.uk",
        "123@numbers.com",
    ];
    
    for email in valid_emails {
        let request = RegisterRequest {
            email: email.to_string(),
            password: "password123".to_string(),
            display_name: None,
        };
        
        // Should not fail on email validation
        let result = auth_routes.register(request).await;
        // May fail on duplicate email, but not on validation
        if result.is_err() {
            let err = result.unwrap_err().to_string();
            assert!(!err.contains("Invalid email format"), "Email {} should be valid", email);
        }
    }
    
    // Invalid emails
    let invalid_emails = vec![
        "@domain.com",
        "user@",
        "nodomain",
        "missing@dotcom",
        "",
        "a@b",
    ];
    
    for email in invalid_emails {
        let request = RegisterRequest {
            email: email.to_string(),
            password: "password123".to_string(),
            display_name: None,
        };
        
        let result = auth_routes.register(request).await;
        assert!(result.is_err(), "Email {} should be invalid", email);
        assert!(result.unwrap_err().to_string().contains("Invalid email format"));
    }
}

#[tokio::test]
async fn test_password_validation() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key).await.unwrap();
    let auth_manager = AuthManager::new(vec![0u8; 64], 24);
    let auth_routes = AuthRoutes::new(database, auth_manager);
    
    // Test short password
    let request = RegisterRequest {
        email: "test@example.com".to_string(),
        password: "short".to_string(),
        display_name: None,
    };
    
    let result = auth_routes.register(request).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Password must be at least 8 characters"));
    
    // Test valid password
    let request = RegisterRequest {
        email: "test2@example.com".to_string(),
        password: "validpassword123".to_string(),
        display_name: None,
    };
    
    let result = auth_routes.register(request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_duplicate_user_registration() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key).await.unwrap();
    let auth_manager = AuthManager::new(vec![0u8; 64], 24);
    let auth_routes = AuthRoutes::new(database, auth_manager);
    
    let request = RegisterRequest {
        email: "duplicate@example.com".to_string(),
        password: "password123".to_string(),
        display_name: Some("Test User".to_string()),
    };
    
    // First registration should succeed
    let result1 = auth_routes.register(request.clone()).await;
    assert!(result1.is_ok());
    
    // Second registration with same email should fail
    let result2 = auth_routes.register(request).await;
    assert!(result2.is_err());
    assert!(result2.unwrap_err().to_string().contains("already exists"));
}

#[tokio::test]
async fn test_login_with_correct_credentials() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key).await.unwrap();
    let auth_manager = AuthManager::new(vec![0u8; 64], 24);
    let auth_routes = AuthRoutes::new(database, auth_manager);
    
    // Register user
    let register_request = RegisterRequest {
        email: "login_test@example.com".to_string(),
        password: "password123".to_string(),
        display_name: Some("Login Test".to_string()),
    };
    
    auth_routes.register(register_request).await.unwrap();
    
    // Login with correct credentials
    let login_request = LoginRequest {
        email: "login_test@example.com".to_string(),
        password: "password123".to_string(),
    };
    
    let result = auth_routes.login(login_request).await;
    assert!(result.is_ok());
    
    let response = result.unwrap();
    assert!(!response.jwt_token.is_empty());
    assert_eq!(response.user.email, "login_test@example.com");
    assert_eq!(response.user.display_name, Some("Login Test".to_string()));
}

#[tokio::test]
async fn test_login_with_wrong_password() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key).await.unwrap();
    let auth_manager = AuthManager::new(vec![0u8; 64], 24);
    let auth_routes = AuthRoutes::new(database, auth_manager);
    
    // Register user
    let register_request = RegisterRequest {
        email: "wrong_pass@example.com".to_string(),
        password: "correctpassword".to_string(),
        display_name: None,
    };
    
    auth_routes.register(register_request).await.unwrap();
    
    // Login with wrong password
    let login_request = LoginRequest {
        email: "wrong_pass@example.com".to_string(),
        password: "wrongpassword".to_string(),
    };
    
    let result = auth_routes.login(login_request).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid email or password"));
}

#[tokio::test]
async fn test_login_with_non_existent_user() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key).await.unwrap();
    let auth_manager = AuthManager::new(vec![0u8; 64], 24);
    let auth_routes = AuthRoutes::new(database, auth_manager);
    
    let login_request = LoginRequest {
        email: "nonexistent@example.com".to_string(),
        password: "password123".to_string(),
    };
    
    let result = auth_routes.login(login_request).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid email or password"));
}