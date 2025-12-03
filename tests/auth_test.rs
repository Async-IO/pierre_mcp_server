// ABOUTME: Unit tests for auth functionality
// ABOUTME: Validates auth behavior, edge cases, and error handling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

mod common;

use chrono::{Duration, Utc};
use pierre_mcp_server::{
    auth::{generate_jwt_secret, AuthManager, AuthMethod, Claims, JwtValidationError},
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    middleware::McpAuthMiddleware,
    models::{AuthRequest, User, UserStatus, UserTier},
};
use std::sync::Arc;
use uuid::Uuid;

fn create_test_user() -> User {
    User::new(
        "test@example.com".into(),
        "hashed_password_123".into(),
        Some("Test User".into()),
    )
}

fn create_auth_manager() -> AuthManager {
    let _secret = generate_jwt_secret().expect("Failed to generate JWT secret");
    AuthManager::new(24) // 24 hour expiry
}

#[test]
fn test_generate_and_validate_token() {
    let auth_manager = create_auth_manager();
    let user = create_test_user();

    // Generate token using shared test JWKS
    let jwks_manager = common::get_shared_test_jwks();
    let token = auth_manager.generate_token(&user, &jwks_manager).unwrap();
    assert!(!token.is_empty());

    // Validate token
    let claims = auth_manager.validate_token(&token, &jwks_manager).unwrap();
    assert_eq!(claims.email, "test@example.com");
    assert_eq!(claims.sub, user.id.to_string());
    assert!(claims.exp > Utc::now().timestamp());
}

#[test]
fn test_create_session() {
    let auth_manager = create_auth_manager();
    let user = create_test_user();

    let jwks_manager = common::get_shared_test_jwks();

    let session = auth_manager.create_session(&user, &jwks_manager).unwrap();
    assert_eq!(session.user_id, user.id);
    assert_eq!(session.email, "test@example.com");
    assert!(!session.jwt_token.is_empty());
    assert!(session.expires_at > Utc::now());
}

#[test]
fn test_authenticate_request() {
    let auth_manager = create_auth_manager();
    let user = create_test_user();

    let jwks_manager = common::get_shared_test_jwks();
    let token = auth_manager.generate_token(&user, &jwks_manager).unwrap();
    let auth_request = AuthRequest { token };

    let response = auth_manager.authenticate(&auth_request, &jwks_manager);
    assert!(response.authenticated);
    assert_eq!(response.user_id, Some(user.id));
    assert!(response.error.is_none());
}

#[test]
fn test_authenticate_invalid_token() {
    let auth_manager = create_auth_manager();

    // Setup JWKS manager for validation
    let jwks_manager = common::get_shared_test_jwks();

    let auth_request = AuthRequest {
        token: "invalid.jwt.token".into(),
    };

    let response = auth_manager.authenticate(&auth_request, &jwks_manager);
    assert!(!response.authenticated);
    assert!(response.user_id.is_none());
    assert!(response.error.is_some());
}

#[test]
fn test_refresh_token() {
    let auth_manager = create_auth_manager();
    let user = create_test_user();

    let jwks_manager = common::get_shared_test_jwks();
    let original_token = auth_manager.generate_token(&user, &jwks_manager).unwrap();
    let refreshed_token = auth_manager
        .refresh_token(&original_token, &user, &jwks_manager)
        .unwrap();

    // Both tokens should be valid (tokens might be identical if generated within same second)

    let original_claims = auth_manager
        .validate_token(&original_token, &jwks_manager)
        .unwrap();
    let refreshed_claims = auth_manager
        .validate_token(&refreshed_token, &jwks_manager)
        .unwrap();

    assert_eq!(original_claims.sub, refreshed_claims.sub);
    assert_eq!(original_claims.email, refreshed_claims.email);
    // Note: expiry times might be the same if generated within the same second
}

#[test]
fn test_extract_user_id_from_validated_token() {
    // This test demonstrates the CORRECT way to extract user IDs from JWT tokens.
    // Security Note: ALWAYS validate the token fully before extracting user IDs.
    // DO NOT bypass validation (e.g., disabling aud/exp checks) as this creates security gaps.

    let auth_manager = create_auth_manager();
    let user = create_test_user();
    let jwks_manager = common::get_shared_test_jwks();

    // Generate a valid token
    let token = auth_manager.generate_token(&user, &jwks_manager).unwrap();

    // CORRECT: Validate token with full security checks, THEN extract user ID
    let claims = auth_manager.validate_token(&token, &jwks_manager).unwrap();
    let user_id = pierre_mcp_server::utils::uuid::parse_uuid(&claims.sub).unwrap();

    assert_eq!(user_id, user.id);

    // Test error handling with invalid token
    let invalid_token = "invalid.jwt.token";
    let result = auth_manager.validate_token(invalid_token, &jwks_manager);
    assert!(result.is_err(), "Invalid token should fail validation");
}

#[tokio::test]
async fn test_mcp_auth_middleware() {
    let auth_manager = create_auth_manager();
    let user = create_test_user();

    // Create in-memory database for testing
    let database_url = "sqlite::memory:";
    let encryption_key = generate_encryption_key().to_vec();

    #[cfg(feature = "postgresql")]
    let database = Arc::new(
        Database::new(
            database_url,
            encryption_key,
            &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        )
        .await
        .unwrap(),
    );

    #[cfg(not(feature = "postgresql"))]
    let database = Arc::new(Database::new(database_url, encryption_key).await.unwrap());

    // Create the user in the database first (required for JWT rate limiting)
    database.create_user(&user).await.unwrap();

    let jwks_manager = common::get_shared_test_jwks();
    let middleware = McpAuthMiddleware::new(
        auth_manager,
        database,
        jwks_manager.clone(),
        pierre_mcp_server::config::environment::RateLimitConfig::default(),
    );

    let token = middleware
        .auth_manager()
        .generate_token(&user, &jwks_manager)
        .unwrap();
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

    #[cfg(feature = "postgresql")]
    let database = Arc::new(
        Database::new(
            database_url,
            encryption_key,
            &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        )
        .await
        .unwrap(),
    );

    #[cfg(not(feature = "postgresql"))]
    let database = Arc::new(Database::new(database_url, encryption_key).await.unwrap());

    let jwks_manager = common::get_shared_test_jwks();
    let middleware = McpAuthMiddleware::new(
        auth_manager,
        database,
        jwks_manager,
        pierre_mcp_server::config::environment::RateLimitConfig::default(),
    );

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

    #[cfg(feature = "postgresql")]
    let database = Arc::new(
        Database::new(
            database_url,
            encryption_key,
            &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        )
        .await
        .unwrap(),
    );

    #[cfg(not(feature = "postgresql"))]
    let database = Arc::new(Database::new(database_url, encryption_key).await.unwrap());

    let jwks_manager = common::get_shared_test_jwks();
    let middleware = McpAuthMiddleware::new(
        auth_manager,
        database,
        jwks_manager.clone(),
        pierre_mcp_server::config::environment::RateLimitConfig::default(),
    );

    // User has no providers initially
    let token = middleware
        .auth_manager()
        .generate_token(&user, &jwks_manager)
        .unwrap();

    // Validate token directly with auth manager
    let claims = middleware
        .auth_manager()
        .validate_token(&token, &jwks_manager)
        .unwrap();
    let has_strava = claims.providers.contains(&"strava".to_owned());
    assert!(!has_strava);
}

#[test]
fn test_jwt_detailed_validation_invalid_token() {
    let auth_manager = create_auth_manager();

    // Setup JWKS manager for validation
    let jwks_manager = common::get_shared_test_jwks();

    // Test with malformed token
    let result = auth_manager.validate_token_detailed("invalid.jwt.token", &jwks_manager);
    assert!(result.is_err());

    match result.unwrap_err() {
        JwtValidationError::TokenMalformed { details } => {
            assert!(!details.is_empty(), "Error details should not be empty");
        }
        _ => panic!("Expected TokenMalformed error"),
    }
}

#[test]
fn test_enhanced_authenticate_response() {
    let user = create_test_user();

    // Test with expired token - use same auth manager for validation
    let expired_auth_manager = AuthManager::new(-1);
    let jwks_manager = common::get_shared_test_jwks();
    let expired_token = expired_auth_manager
        .generate_token(&user, &jwks_manager)
        .unwrap();

    let auth_request = AuthRequest {
        token: expired_token,
    };
    let response = expired_auth_manager.authenticate(&auth_request, &jwks_manager);

    assert!(!response.authenticated);
    assert!(response.error.is_some());
    let error_msg = response.error.unwrap();
    assert!(error_msg.contains("JWT token expired"));
}

// Additional Comprehensive Auth Tests

#[test]
fn test_generate_jwt_secret() {
    let secret = generate_jwt_secret().expect("Failed to generate JWT secret");
    assert_eq!(secret.len(), 64);

    // Generate another secret and ensure they're different
    let secret2 = generate_jwt_secret().expect("Failed to generate JWT secret");
    assert_ne!(secret, secret2);
}

#[test]
fn test_auth_manager_new() {
    let expiry_hours = 12;
    let auth_manager = AuthManager::new(expiry_hours);
    let user = create_test_user();

    // Note: RS256 auth manager doesn't store jwt_secret anymore
    // Verify it works by generating a token
    let jwks_manager = common::get_shared_test_jwks();

    let token = auth_manager.generate_token(&user, &jwks_manager);
    assert!(token.is_ok());
}

#[test]
fn test_generate_token_success() {
    let auth_manager = create_auth_manager();
    let user = create_test_user();

    let jwks_manager = common::get_shared_test_jwks();
    let token_result = auth_manager.generate_token(&user, &jwks_manager);
    assert!(token_result.is_ok());

    let token = token_result.unwrap();
    assert!(!token.is_empty());
    assert!(token.contains('.'));

    // JWT should have 3 parts separated by dots
    assert_eq!(token.split('.').count(), 3);
}

#[test]
fn test_validate_token_invalid_signature() {
    let auth_manager = create_auth_manager();
    let user = create_test_user();

    let jwks_manager = common::get_shared_test_jwks();
    let token = auth_manager.generate_token(&user, &jwks_manager).unwrap();

    // Create a different JWKS manager with different key - validation will fail
    let mut different_jwks_manager = pierre_mcp_server::admin::jwks::JwksManager::new();
    different_jwks_manager
        .generate_rsa_key_pair_with_size("different_key", 2048)
        .unwrap();
    let different_jwks_manager = Arc::new(different_jwks_manager);

    let different_auth_manager = create_auth_manager();
    let claims_result = different_auth_manager.validate_token(&token, &different_jwks_manager);

    assert!(claims_result.is_err());
}

#[test]
fn test_validate_token_malformed() {
    let auth_manager = create_auth_manager();

    let invalid_tokens = vec![
        "not.a.jwt",
        "invalid_token",
        "header.payload", // missing signature
        "too.many.parts.here.invalid",
        "",
    ];

    let jwks_manager = common::get_shared_test_jwks();

    for invalid_token in invalid_tokens {
        let result = auth_manager.validate_token(invalid_token, &jwks_manager);
        assert!(result.is_err(), "Token should be invalid: {invalid_token}");
    }
}

#[test]
fn test_validate_token_detailed_success() {
    let auth_manager = create_auth_manager();
    let user = create_test_user();

    let jwks_manager = common::get_shared_test_jwks();
    let token = auth_manager.generate_token(&user, &jwks_manager).unwrap();
    let claims_result = auth_manager.validate_token_detailed(&token, &jwks_manager);

    assert!(claims_result.is_ok());

    let claims = claims_result.unwrap();
    assert_eq!(claims.sub, user.id.to_string());
    assert_eq!(claims.email, user.email);
}

#[test]
fn test_validate_token_detailed_invalid_signature() {
    let auth_manager = create_auth_manager();
    let user = create_test_user();

    let jwks_manager = common::get_shared_test_jwks();
    let token = auth_manager.generate_token(&user, &jwks_manager).unwrap();

    // Create a different JWKS manager with different key
    let mut different_jwks_manager = pierre_mcp_server::admin::jwks::JwksManager::new();
    different_jwks_manager
        .generate_rsa_key_pair_with_size("different_key", 2048)
        .unwrap();
    let different_jwks_manager = Arc::new(different_jwks_manager);

    let different_auth_manager = create_auth_manager();
    let claims_result =
        different_auth_manager.validate_token_detailed(&token, &different_jwks_manager);

    assert!(claims_result.is_err());

    let error = claims_result.unwrap_err();
    match error {
        JwtValidationError::TokenInvalid { reason } => {
            assert!(reason.contains("Key not found") || reason.contains("JWKS"));
        }
        _ => panic!("Expected TokenInvalid error, got {error:?}"),
    }
}

#[test]
fn test_validate_token_detailed_malformed() {
    let auth_manager = create_auth_manager();

    // Setup JWKS manager for validation
    let jwks_manager = common::get_shared_test_jwks();

    let malformed_token = "not.a.jwt";
    let claims_result = auth_manager.validate_token_detailed(malformed_token, &jwks_manager);

    assert!(claims_result.is_err());

    let error = claims_result.unwrap_err();
    match error {
        JwtValidationError::TokenMalformed { details } => {
            assert!(!details.is_empty());
        }
        _ => panic!("Expected TokenMalformed error, got {error:?}"),
    }
}

#[test]
fn test_generate_oauth_access_token() {
    let auth_manager = create_auth_manager();
    let user_id = Uuid::new_v4();
    let scopes = vec!["read".to_owned(), "write".to_owned()];

    // Setup JWKS manager for RS256 token generation
    let jwks_manager = common::get_shared_test_jwks();

    let token_result =
        auth_manager.generate_oauth_access_token(&jwks_manager, &user_id, &scopes, None);
    assert!(token_result.is_ok());

    let token = token_result.unwrap();
    assert!(!token.is_empty());

    // Validate the token
    let claims = auth_manager.validate_token(&token, &jwks_manager).unwrap();
    assert_eq!(claims.sub, user_id.to_string());
}

#[test]
fn test_generate_client_credentials_token() {
    let auth_manager = create_auth_manager();
    let client_id = "test_client_id";
    let scopes = vec!["client_read".to_owned(), "client_write".to_owned()];

    // Setup JWKS manager for RS256 token generation
    let jwks_manager = common::get_shared_test_jwks();

    let token_result =
        auth_manager.generate_client_credentials_token(&jwks_manager, client_id, &scopes, None);
    assert!(token_result.is_ok());

    let token = token_result.unwrap();
    assert!(!token.is_empty());

    // Validate the token structure
    assert_eq!(token.split('.').count(), 3);
}

#[test]
fn test_jwt_validation_error_display() {
    let now = Utc::now();
    let expired_at = now - Duration::minutes(30);

    let error = JwtValidationError::TokenExpired {
        expired_at,
        current_time: now,
    };

    let error_string = error.to_string();
    assert!(error_string.contains("expired"));
    assert!(error_string.contains("minutes ago"));

    let invalid_error = JwtValidationError::TokenInvalid {
        reason: "Test reason".to_owned(),
    };

    let invalid_string = invalid_error.to_string();
    assert!(invalid_string.contains("invalid"));
    assert!(invalid_string.contains("Test reason"));

    let malformed_error = JwtValidationError::TokenMalformed {
        details: "Test details".to_owned(),
    };

    let malformed_string = malformed_error.to_string();
    assert!(malformed_string.contains("malformed"));
    assert!(malformed_string.contains("Test details"));
}

#[test]
fn test_auth_method_details() {
    let jwt_method = AuthMethod::JwtToken {
        tier: "professional".to_owned(),
    };

    let jwt_details = jwt_method.details();
    assert!(jwt_details.contains("JWT"));
    assert!(jwt_details.contains("professional"));

    let api_key_method = AuthMethod::ApiKey {
        key_id: "key123".to_owned(),
        tier: "enterprise".to_owned(),
    };

    let api_key_details = api_key_method.details();
    assert!(api_key_details.contains("API"));
    assert!(api_key_details.contains("key123"));
    assert!(api_key_details.contains("enterprise"));
}

// Note: humanize_duration is private, so we can't test it directly

#[test]
fn test_claims_serialization() {
    let claims = Claims {
        sub: Uuid::new_v4().to_string(),
        email: "test@example.com".to_owned(),
        iat: Utc::now().timestamp(),
        exp: (Utc::now() + Duration::hours(1)).timestamp(),
        iss: "pierre-mcp-server".to_owned(),
        jti: Uuid::new_v4().to_string(),
        providers: vec!["strava".to_owned(), "fitbit".to_owned()],
        aud: "mcp".to_owned(),
        tenant_id: None,
        impersonator_id: None,
        impersonation_session_id: None,
    };

    let json = serde_json::to_string(&claims).unwrap();
    let deserialized: Claims = serde_json::from_str(&json).unwrap();

    assert_eq!(claims.sub, deserialized.sub);
    assert_eq!(claims.email, deserialized.email);
    assert_eq!(claims.iat, deserialized.iat);
    assert_eq!(claims.exp, deserialized.exp);
    assert_eq!(claims.providers, deserialized.providers);
}

#[tokio::test]
async fn test_check_setup_status_admin_exists() {
    let auth_manager = create_auth_manager();

    // Create in-memory database with admin user
    let database_url = "sqlite::memory:";
    let encryption_key = generate_encryption_key().to_vec();

    #[cfg(feature = "postgresql")]
    let database = Database::new(
        database_url,
        encryption_key,
        &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
    )
    .await
    .unwrap();

    #[cfg(not(feature = "postgresql"))]
    let database = Database::new(database_url, encryption_key).await.unwrap();

    // Create admin user
    let mut admin_user = User::new(
        "admin@pierre.mcp".into(),
        "hashed_password".into(),
        Some("Admin User".into()),
    );
    admin_user.is_admin = true;
    admin_user.user_status = UserStatus::Active;

    database.create_user(&admin_user).await.unwrap();

    let setup_status = auth_manager.check_setup_status(&database).await.unwrap();
    assert!(!setup_status.needs_setup);
    assert!(setup_status.admin_user_exists);
    assert!(setup_status.message.is_none());
}

#[tokio::test]
async fn test_check_setup_status_no_admin() {
    let auth_manager = create_auth_manager();

    // Create in-memory database without admin user
    let database_url = "sqlite::memory:";
    let encryption_key = generate_encryption_key().to_vec();

    #[cfg(feature = "postgresql")]
    let database = Database::new(
        database_url,
        encryption_key,
        &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
    )
    .await
    .unwrap();

    #[cfg(not(feature = "postgresql"))]
    let database = Database::new(database_url, encryption_key).await.unwrap();

    let setup_status = auth_manager.check_setup_status(&database).await.unwrap();
    assert!(setup_status.needs_setup);
    assert!(!setup_status.admin_user_exists);
    assert!(setup_status.message.is_some());
}

fn create_test_user_with_tier(tier: UserTier) -> User {
    let mut user = create_test_user();
    user.tier = tier;
    user.user_status = UserStatus::Active;
    user
}

#[tokio::test]
async fn test_mcp_auth_middleware_different_user_tiers() {
    let auth_manager = create_auth_manager();

    let database_url = "sqlite::memory:";
    let encryption_key = generate_encryption_key().to_vec();

    #[cfg(feature = "postgresql")]
    let database = Arc::new(
        Database::new(
            database_url,
            encryption_key,
            &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        )
        .await
        .unwrap(),
    );

    #[cfg(not(feature = "postgresql"))]
    let database = Arc::new(Database::new(database_url, encryption_key).await.unwrap());

    // Use shared JWKS manager for all tier tests
    let jwks_manager = common::get_shared_test_jwks();

    // Test different user tiers
    for (i, tier) in [
        UserTier::Starter,
        UserTier::Professional,
        UserTier::Enterprise,
    ]
    .iter()
    .enumerate()
    {
        let mut user = create_test_user_with_tier(tier.clone());
        user.email = format!("tier_test_{i}@example.com"); // Unique email for each user
        database.create_user(&user).await.unwrap();

        let middleware = McpAuthMiddleware::new(
            auth_manager.clone(),
            database.clone(),
            jwks_manager.clone(),
            pierre_mcp_server::config::environment::RateLimitConfig::default(),
        );
        let token = middleware
            .auth_manager()
            .generate_token(&user, &jwks_manager)
            .unwrap();
        let auth_header = format!("Bearer {token}");

        let auth_result = middleware
            .authenticate_request(Some(&auth_header))
            .await
            .unwrap();

        assert_eq!(auth_result.user_id, user.id);
        match auth_result.auth_method {
            AuthMethod::JwtToken { tier: tier_str } => {
                assert_eq!(tier_str, format!("{tier:?}").to_lowercase());
            }
            AuthMethod::ApiKey { .. } => panic!("Expected JwtToken auth method"),
            _ => panic!("Unknown auth method variant"),
        }
    }
}

#[test]
fn test_token_counter_uniqueness() {
    let auth_manager = create_auth_manager();
    let user = create_test_user();

    let jwks_manager = common::get_shared_test_jwks();

    // Generate multiple tokens rapidly
    let mut tokens = Vec::new();
    for _ in 0..10 {
        let token = auth_manager.generate_token(&user, &jwks_manager).unwrap();
        tokens.push(token);
    }

    // Verify all tokens have unique jti (JWT ID) values
    // RFC 7519: jti provides a unique identifier for the JWT
    let mut jtis = Vec::new();
    for token in tokens {
        let claims = auth_manager.validate_token(&token, &jwks_manager).unwrap();
        jtis.push(claims.jti);
    }

    // All jti values should be unique (guaranteed by UUID v4)
    jtis.sort_unstable();
    jtis.dedup();
    assert_eq!(
        jtis.len(),
        10,
        "All tokens should have unique jti (JWT ID) values"
    );
}
