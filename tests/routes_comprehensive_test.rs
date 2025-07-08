// ABOUTME: Comprehensive tests for authentication and OAuth route flows
// ABOUTME: Tests authentication, registration, and OAuth functionality
//! Comprehensive tests for routes.rs - Authentication and OAuth flows
//!
//! This test suite aims to improve coverage from 55.09% to 80%+ by testing
//! all critical authentication, registration, and OAuth functionality.

use anyhow::Result;
use pierre_mcp_server::routes::{
    AuthRoutes, LoginRequest, OAuthRoutes, RefreshTokenRequest, RegisterRequest,
};
use uuid::Uuid;

mod common;

// === Test Setup Helpers ===

async fn create_test_auth_routes() -> Result<AuthRoutes> {
    let database = common::create_test_database().await?;
    let auth_manager = common::create_test_auth_manager();

    Ok(AuthRoutes::new(
        (*database).clone(),
        (*auth_manager).clone(),
    ))
}

async fn create_test_oauth_routes() -> Result<OAuthRoutes> {
    let database = common::create_test_database().await?;

    Ok(OAuthRoutes::new((*database).clone()))
}

// === AuthRoutes Registration Tests ===

#[tokio::test]
async fn test_user_registration_success() -> Result<()> {
    let auth_routes = create_test_auth_routes().await?;

    let request = RegisterRequest {
        email: "test@example.com".to_string(),
        password: "securepassword123".to_string(),
        display_name: Some("Test User".to_string()),
    };

    let response = auth_routes.register(request).await?;

    assert!(!response.user_id.is_empty());
    assert!(response.message.contains("successfully"));

    Ok(())
}

#[tokio::test]
async fn test_user_registration_invalid_email() -> Result<()> {
    let auth_routes = create_test_auth_routes().await?;

    let request = RegisterRequest {
        email: "invalid-email".to_string(),
        password: "securepassword123".to_string(),
        display_name: Some("Test User".to_string()),
    };

    let result = auth_routes.register(request).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid email format"));

    Ok(())
}

#[tokio::test]
async fn test_user_registration_weak_password() -> Result<()> {
    let auth_routes = create_test_auth_routes().await?;

    let request = RegisterRequest {
        email: "test@example.com".to_string(),
        password: "weak".to_string(), // Too short
        display_name: Some("Test User".to_string()),
    };

    let result = auth_routes.register(request).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("8 characters"));

    Ok(())
}

#[tokio::test]
async fn test_user_registration_duplicate_email() -> Result<()> {
    let auth_routes = create_test_auth_routes().await?;

    let request = RegisterRequest {
        email: "duplicate@example.com".to_string(),
        password: "securepassword123".to_string(),
        display_name: Some("Test User".to_string()),
    };

    // First registration should succeed
    auth_routes.register(request.clone()).await?;

    // Second registration with same email should fail
    let result = auth_routes.register(request).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));

    Ok(())
}

#[tokio::test]
async fn test_user_registration_edge_cases() -> Result<()> {
    let auth_routes = create_test_auth_routes().await?;

    // Test with minimal valid input
    let minimal_request = RegisterRequest {
        email: "minimal@example.com".to_string(),
        password: "12345678".to_string(), // Exactly 8 characters
        display_name: None,
    };

    let response = auth_routes.register(minimal_request).await?;
    assert!(!response.user_id.is_empty());

    // Test with very long valid email
    let long_email_request = RegisterRequest {
        email: "very.long.email.address.for.testing@example.com".to_string(),
        password: "securepassword123".to_string(),
        display_name: Some("Very Long Display Name For Testing Purposes".to_string()),
    };

    let response = auth_routes.register(long_email_request).await?;
    assert!(!response.user_id.is_empty());

    Ok(())
}

// === AuthRoutes Login Tests ===

#[tokio::test]
async fn test_user_login_success() -> Result<()> {
    let auth_routes = create_test_auth_routes().await?;

    // First register a user
    let register_request = RegisterRequest {
        email: "login@example.com".to_string(),
        password: "loginpassword123".to_string(),
        display_name: Some("Login User".to_string()),
    };

    auth_routes.register(register_request).await?;

    // Now test login
    let login_request = LoginRequest {
        email: "login@example.com".to_string(),
        password: "loginpassword123".to_string(),
    };

    let response = auth_routes.login(login_request).await?;

    assert!(!response.jwt_token.is_empty());
    assert!(!response.expires_at.is_empty());
    assert_eq!(response.user.email, "login@example.com");
    assert_eq!(response.user.display_name, Some("Login User".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_user_login_invalid_email() -> Result<()> {
    let auth_routes = create_test_auth_routes().await?;

    let login_request = LoginRequest {
        email: "nonexistent@example.com".to_string(),
        password: "anypassword".to_string(),
    };

    let result = auth_routes.login(login_request).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid email or password"));

    Ok(())
}

#[tokio::test]
async fn test_user_login_invalid_password() -> Result<()> {
    let auth_routes = create_test_auth_routes().await?;

    // Register a user first
    let register_request = RegisterRequest {
        email: "password_test@example.com".to_string(),
        password: "correctpassword123".to_string(),
        display_name: Some("Password User".to_string()),
    };

    auth_routes.register(register_request).await?;

    // Try to login with wrong password
    let login_request = LoginRequest {
        email: "password_test@example.com".to_string(),
        password: "wrongpassword".to_string(),
    };

    let result = auth_routes.login(login_request).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid email or password"));

    Ok(())
}

#[tokio::test]
async fn test_user_login_case_sensitivity() -> Result<()> {
    let auth_routes = create_test_auth_routes().await?;

    // Register with lowercase email
    let register_request = RegisterRequest {
        email: "case@example.com".to_string(),
        password: "casepassword123".to_string(),
        display_name: Some("Case User".to_string()),
    };

    auth_routes.register(register_request).await?;

    // Try to login with uppercase email (should fail for security)
    let login_request = LoginRequest {
        email: "CASE@EXAMPLE.COM".to_string(),
        password: "casepassword123".to_string(),
    };

    let result = auth_routes.login(login_request).await;

    // Email should be case-sensitive for security
    assert!(result.is_err());

    Ok(())
}

// === AuthRoutes Token Refresh Tests ===

#[tokio::test]
async fn test_token_refresh_success() -> Result<()> {
    let auth_routes = create_test_auth_routes().await?;

    // Register and login to get initial token
    let register_request = RegisterRequest {
        email: "refresh@example.com".to_string(),
        password: "refreshpassword123".to_string(),
        display_name: Some("Refresh User".to_string()),
    };

    let register_response = auth_routes.register(register_request).await?;
    let user_id = register_response.user_id;

    let login_request = LoginRequest {
        email: "refresh@example.com".to_string(),
        password: "refreshpassword123".to_string(),
    };

    let login_response = auth_routes.login(login_request).await?;
    let original_token = login_response.jwt_token;

    // Test token refresh
    let refresh_request = RefreshTokenRequest {
        token: original_token.clone(),
        user_id: user_id.clone(),
    };

    let refresh_response = auth_routes.refresh_token(refresh_request).await?;

    // Token refresh should return a valid token (may be same or different depending on implementation)
    assert!(!refresh_response.jwt_token.is_empty());
    assert_eq!(refresh_response.user.email, "refresh@example.com");

    Ok(())
}

#[tokio::test]
async fn test_token_refresh_invalid_token() -> Result<()> {
    let auth_routes = create_test_auth_routes().await?;

    let refresh_request = RefreshTokenRequest {
        token: "invalid.jwt.token".to_string(),
        user_id: Uuid::new_v4().to_string(),
    };

    let result = auth_routes.refresh_token(refresh_request).await;

    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_token_refresh_mismatched_user() -> Result<()> {
    let auth_routes = create_test_auth_routes().await?;

    // Register and login to get a valid token
    let register_request = RegisterRequest {
        email: "mismatch@example.com".to_string(),
        password: "mismatchpassword123".to_string(),
        display_name: Some("Mismatch User".to_string()),
    };

    auth_routes.register(register_request).await?;

    let login_request = LoginRequest {
        email: "mismatch@example.com".to_string(),
        password: "mismatchpassword123".to_string(),
    };

    let login_response = auth_routes.login(login_request).await?;

    // Try to refresh with different user ID
    let refresh_request = RefreshTokenRequest {
        token: login_response.jwt_token,
        user_id: Uuid::new_v4().to_string(), // Different user ID
    };

    let result = auth_routes.refresh_token(refresh_request).await;

    assert!(result.is_err());

    Ok(())
}

// === OAuthRoutes Tests ===

#[tokio::test]
async fn test_oauth_get_auth_url_strava() -> Result<()> {
    let oauth_routes = create_test_oauth_routes().await?;
    let user_id = Uuid::new_v4();

    let response = oauth_routes.get_auth_url(user_id, "strava")?;

    assert!(response.authorization_url.contains("strava.com"));
    assert!(response.authorization_url.contains("authorize"));
    assert!(!response.state.is_empty());
    assert!(!response.instructions.is_empty());
    assert!(response.expires_in_minutes > 0);

    Ok(())
}

#[tokio::test]
async fn test_oauth_get_auth_url_fitbit() -> Result<()> {
    let oauth_routes = create_test_oauth_routes().await?;
    let user_id = Uuid::new_v4();

    let response = oauth_routes.get_auth_url(user_id, "fitbit")?;

    assert!(response.authorization_url.contains("fitbit.com"));
    assert!(response.authorization_url.contains("authorize"));
    assert!(!response.state.is_empty());
    assert!(!response.instructions.is_empty());
    assert!(response.expires_in_minutes > 0);

    Ok(())
}

#[tokio::test]
async fn test_oauth_get_auth_url_unsupported_provider() -> Result<()> {
    let oauth_routes = create_test_oauth_routes().await?;
    let user_id = Uuid::new_v4();

    let result = oauth_routes.get_auth_url(user_id, "unsupported_provider");

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Unsupported provider"));

    Ok(())
}

#[tokio::test]
async fn test_oauth_connection_status_no_connections() -> Result<()> {
    let oauth_routes = create_test_oauth_routes().await?;
    let user_id = Uuid::new_v4();

    let status = oauth_routes.get_connection_status(user_id).await?;

    // Should return status for all providers
    assert!(status.len() >= 2); // At least Strava and Fitbit

    // All should be disconnected initially
    for connection in status {
        assert!(!connection.connected);
        assert!(connection.expires_at.is_none());
    }

    Ok(())
}

#[tokio::test]
async fn test_oauth_disconnect_provider_success() -> Result<()> {
    let oauth_routes = create_test_oauth_routes().await?;
    let user_id = Uuid::new_v4();

    // Disconnecting a provider that wasn't connected should succeed (idempotent)
    let result = oauth_routes.disconnect_provider(user_id, "strava");

    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_oauth_disconnect_invalid_provider() -> Result<()> {
    let oauth_routes = create_test_oauth_routes().await?;
    let user_id = Uuid::new_v4();

    let result = oauth_routes.disconnect_provider(user_id, "invalid_provider");

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Unsupported provider"));

    Ok(())
}

// === Email and Password Validation Tests ===

#[tokio::test]
async fn test_email_validation_comprehensive() -> Result<()> {
    let auth_routes = create_test_auth_routes().await?;

    // Test obviously invalid email formats that should definitely fail
    let invalid_emails = ["invalid-email", "@example.com", "user@", ""];

    for email in invalid_emails {
        let request = RegisterRequest {
            email: email.to_string(),
            password: "validpassword123".to_string(),
            display_name: Some("Test User".to_string()),
        };

        let result = auth_routes.register(request).await;
        assert!(result.is_err(), "Email '{email}' should be invalid");
    }

    // Test valid email formats
    let valid_emails = [
        "user@example.com",
        "test.user@example.com",
        "user+tag@example.com",
        "user123@example123.com",
        "a@b.co",
    ];

    for (i, email) in valid_emails.iter().enumerate() {
        let request = RegisterRequest {
            email: (*email).to_string(),
            password: "validpassword123".to_string(),
            display_name: Some(format!("Test User {i}")),
        };

        let result = auth_routes.register(request).await;
        assert!(result.is_ok(), "Email '{email}' should be valid");
    }

    Ok(())
}

#[tokio::test]
async fn test_password_validation_comprehensive() -> Result<()> {
    let auth_routes = create_test_auth_routes().await?;

    // Test invalid passwords (too short)
    let invalid_passwords = [
        "", "1", "12", "123", "1234", "12345", "123456",
        "1234567", // 7 characters - should fail
    ];

    for (i, password) in invalid_passwords.iter().enumerate() {
        let request = RegisterRequest {
            email: format!("test{i}@example.com"),
            password: (*password).to_string(),
            display_name: Some("Test User".to_string()),
        };

        let result = auth_routes.register(request).await;
        assert!(result.is_err(), "Password '{password}' should be invalid");
    }

    // Test valid passwords (8+ characters)
    let valid_passwords = [
        "12345678", // Exactly 8 characters
        "validpassword",
        "ValidPassword123",
        "very_long_password_that_exceeds_minimum_requirements",
        "P@ssw0rd!",
        "简单密码", // Unicode characters
    ];

    for (i, password) in valid_passwords.iter().enumerate() {
        let request = RegisterRequest {
            email: format!("valid{i}@example.com"),
            password: (*password).to_string(),
            display_name: Some("Test User".to_string()),
        };

        let result = auth_routes.register(request).await;
        assert!(result.is_ok(), "Password should be valid");
    }

    Ok(())
}

// === Integration Tests ===

#[tokio::test]
async fn test_complete_auth_flow() -> Result<()> {
    let auth_routes = create_test_auth_routes().await?;
    let oauth_routes = create_test_oauth_routes().await?;

    // 1. Register user
    let register_request = RegisterRequest {
        email: "integration@example.com".to_string(),
        password: "integrationpass123".to_string(),
        display_name: Some("Integration User".to_string()),
    };

    let register_response = auth_routes.register(register_request).await?;
    let user_id = Uuid::parse_str(&register_response.user_id)?;

    // 2. Login
    let login_request = LoginRequest {
        email: "integration@example.com".to_string(),
        password: "integrationpass123".to_string(),
    };

    let login_response = auth_routes.login(login_request).await?;

    // 3. Refresh token
    let refresh_request = RefreshTokenRequest {
        token: login_response.jwt_token,
        user_id: user_id.to_string(),
    };

    let refresh_response = auth_routes.refresh_token(refresh_request).await?;

    // 4. Check OAuth connection status
    let connections = oauth_routes.get_connection_status(user_id).await?;

    // 5. Get OAuth authorization URL
    let auth_url = oauth_routes.get_auth_url(user_id, "strava")?;

    // Verify everything worked
    assert!(!register_response.user_id.is_empty());
    assert!(!refresh_response.jwt_token.is_empty());
    assert!(!connections.is_empty());
    assert!(!auth_url.authorization_url.is_empty());

    Ok(())
}

// === Concurrency Tests ===

#[tokio::test]
async fn test_concurrent_registrations() -> Result<()> {
    let auth_routes = create_test_auth_routes().await?;

    let mut handles = vec![];

    for i in 0..5 {
        let routes = auth_routes.clone();
        handles.push(tokio::spawn(async move {
            let request = RegisterRequest {
                email: format!("concurrent{i}@example.com"),
                password: "concurrentpass123".to_string(),
                display_name: Some(format!("Concurrent User {i}")),
            };

            routes.register(request).await
        }));
    }

    // All registrations should succeed
    for handle in handles {
        let result = handle.await?;
        assert!(result.is_ok());
    }

    Ok(())
}

#[tokio::test]
async fn test_concurrent_logins() -> Result<()> {
    let auth_routes = create_test_auth_routes().await?;

    // First register users
    for i in 0..3 {
        let request = RegisterRequest {
            email: format!("login_concurrent{i}@example.com"),
            password: "loginpass123".to_string(),
            display_name: Some(format!("Login User {i}")),
        };
        auth_routes.register(request).await?;
    }

    let mut handles = vec![];

    for i in 0..3 {
        let routes = auth_routes.clone();
        handles.push(tokio::spawn(async move {
            let request = LoginRequest {
                email: format!("login_concurrent{i}@example.com"),
                password: "loginpass123".to_string(),
            };

            routes.login(request).await
        }));
    }

    // All logins should succeed
    for handle in handles {
        let result = handle.await?;
        assert!(result.is_ok());
    }

    Ok(())
}
