use chrono::Utc;
use pierre_mcp_server::{
    auth::{generate_jwt_secret, AuthManager, JwtValidationError, McpAuthMiddleware},
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    models::{AuthRequest, User},
};
use std::sync::Arc;

fn create_test_user() -> User {
    User::new(
        "test@example.com".into(),
        "hashed_password_123".into(),
        Some("Test User".into()),
    )
}

fn create_auth_manager() -> AuthManager {
    let secret = generate_jwt_secret().to_vec();
    AuthManager::new(secret, 24) // 24 hour expiry
}

#[test]
fn test_generate_and_validate_token() {
    let auth_manager = create_auth_manager();
    let user = create_test_user();

    // Generate token
    let token = auth_manager.generate_token(&user).unwrap();
    assert!(!token.is_empty());

    // Validate token
    let claims = auth_manager.validate_token(&token).unwrap();
    assert_eq!(claims.email, "test@example.com");
    assert_eq!(claims.sub, user.id.to_string());
    assert!(claims.exp > Utc::now().timestamp());
}

#[test]
fn test_create_session() {
    let auth_manager = create_auth_manager();
    let user = create_test_user();

    let session = auth_manager.create_session(&user).unwrap();
    assert_eq!(session.user_id, user.id);
    assert_eq!(session.email, "test@example.com");
    assert!(!session.jwt_token.is_empty());
    assert!(session.expires_at > Utc::now());
}

#[test]
fn test_authenticate_request() {
    let auth_manager = create_auth_manager();
    let user = create_test_user();

    let token = auth_manager.generate_token(&user).unwrap();
    let auth_request = AuthRequest { token };

    let response = auth_manager.authenticate(&auth_request);
    assert!(response.authenticated);
    assert_eq!(response.user_id, Some(user.id));
    assert!(response.error.is_none());
}

#[test]
fn test_authenticate_invalid_token() {
    let auth_manager = create_auth_manager();
    let auth_request = AuthRequest {
        token: "invalid.jwt.token".into(),
    };

    let response = auth_manager.authenticate(&auth_request);
    assert!(!response.authenticated);
    assert!(response.user_id.is_none());
    assert!(response.error.is_some());
}

#[test]
fn test_refresh_token() {
    let auth_manager = create_auth_manager();
    let user = create_test_user();

    let original_token = auth_manager.generate_token(&user).unwrap();
    let refreshed_token = auth_manager.refresh_token(&original_token, &user).unwrap();

    // Both tokens should be valid (tokens might be identical if generated within same second)

    let original_claims = auth_manager.validate_token(&original_token).unwrap();
    let refreshed_claims = auth_manager.validate_token(&refreshed_token).unwrap();

    assert_eq!(original_claims.sub, refreshed_claims.sub);
    assert_eq!(original_claims.email, refreshed_claims.email);
    // Note: expiry times might be the same if generated within the same second
}

#[test]
fn test_extract_user_id() {
    let auth_manager = create_auth_manager();
    let user = create_test_user();

    let token = auth_manager.generate_token(&user).unwrap();
    let extracted_id = auth_manager.extract_user_id(&token).unwrap();

    assert_eq!(extracted_id, user.id);
}

#[tokio::test]
async fn test_mcp_auth_middleware() {
    let auth_manager = create_auth_manager();
    let user = create_test_user();

    // Create in-memory database for testing
    let database_url = "sqlite::memory:";
    let encryption_key = generate_encryption_key().to_vec();
    let database = Arc::new(Database::new(database_url, encryption_key).await.unwrap());

    // Create the user in the database first (required for JWT rate limiting)
    database.create_user(&user).await.unwrap();

    let middleware = McpAuthMiddleware::new(auth_manager, database);

    let token = middleware.auth_manager().generate_token(&user).unwrap();
    let auth_header = format!("Bearer {token}");

    let auth_result = middleware
        .authenticate_request(Some(&auth_header))
        .await
        .unwrap();
    assert_eq!(auth_result.user_id, user.id);
    assert!(matches!(
        auth_result.auth_method,
        pierre_mcp_server::auth::AuthMethod::JwtToken { .. }
    ));
}

#[tokio::test]
async fn test_mcp_auth_middleware_invalid_header() {
    let auth_manager = create_auth_manager();

    // Create in-memory database for testing
    let database_url = "sqlite::memory:";
    let encryption_key = generate_encryption_key().to_vec();
    let database = Arc::new(Database::new(database_url, encryption_key).await.unwrap());

    let middleware = McpAuthMiddleware::new(auth_manager, database);

    // Test missing header
    let result = middleware.authenticate_request(None).await;
    assert!(result.is_err());

    // Test invalid format
    let result = middleware
        .authenticate_request(Some("Invalid header"))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_provider_access_check() {
    let auth_manager = create_auth_manager();
    let user = create_test_user();

    // Create in-memory database for testing
    let database_url = "sqlite::memory:";
    let encryption_key = generate_encryption_key().to_vec();
    let database = Arc::new(Database::new(database_url, encryption_key).await.unwrap());

    let middleware = McpAuthMiddleware::new(auth_manager, database);

    // User has no providers initially
    let token = middleware.auth_manager().generate_token(&user).unwrap();

    let has_strava = middleware.check_provider_access(&token, "strava").unwrap();
    assert!(!has_strava);
}

#[test]
fn test_jwt_detailed_validation_invalid_token() {
    let auth_manager = create_auth_manager();

    // Test with malformed token
    let result = auth_manager.validate_token_detailed("invalid.jwt.token");
    assert!(result.is_err());

    match result.unwrap_err() {
        JwtValidationError::TokenMalformed { details } => {
            assert!(details.contains("Token"));
        }
        _ => panic!("Expected TokenMalformed error"),
    }
}

#[test]
fn test_enhanced_authenticate_response() {
    let user = create_test_user();

    // Test with expired token - use same auth manager for validation
    let secret = generate_jwt_secret().to_vec();
    let expired_auth_manager = AuthManager::new(secret, -1);
    let expired_token = expired_auth_manager.generate_token(&user).unwrap();

    let auth_request = AuthRequest {
        token: expired_token,
    };
    let response = expired_auth_manager.authenticate(&auth_request);

    assert!(!response.authenticated);
    assert!(response.error.is_some());
    let error_msg = response.error.unwrap();
    assert!(error_msg.contains("JWT token expired"));
}
