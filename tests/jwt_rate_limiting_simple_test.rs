// ABOUTME: Simple test to verify JWT rate limiting works correctly
// ABOUTME: Tests critical security fix for JWT tokens having unlimited API access
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Simple test to verify JWT rate limiting works
//!
//! This is a focused test to verify that the critical security vulnerability
//! where JWT tokens had unlimited API access has been fixed.

use pierre_mcp_server::auth::AuthManager;
use pierre_mcp_server::database::generate_encryption_key;
use pierre_mcp_server::database_plugins::{factory::Database, DatabaseProvider};
use pierre_mcp_server::middleware::McpAuthMiddleware;
use pierre_mcp_server::models::User;
use std::sync::Arc;

#[tokio::test]
async fn test_jwt_tokens_now_have_rate_limiting() {
    // Create test database
    let database_url = "sqlite::memory:";
    let encryption_key = generate_encryption_key().to_vec();
    let database = Arc::new(Database::new(database_url, encryption_key).await.unwrap());

    // Create auth manager and middleware
    let jwt_secret = pierre_mcp_server::auth::generate_jwt_secret().to_vec();
    let auth_manager = AuthManager::new(jwt_secret.clone(), 24);
    let auth_middleware = Arc::new(McpAuthMiddleware::new(auth_manager, database.clone()));

    // Create and store a test user (defaults to Starter tier with 10,000 requests/month)
    let user = User::new(
        "jwt_test@example.com".to_string(),
        "hashed_password".to_string(),
        Some("JWT Test User".to_string()),
    );
    database.create_user(&user).await.unwrap();

    // Create a JWT token for the user (using same secret for consistency)
    let token_auth_manager = AuthManager::new(jwt_secret, 24);
    let token = token_auth_manager
        .generate_token(&user)
        .expect("Failed to generate JWT token");

    // Test authentication - should now include rate limiting info
    let auth_result = auth_middleware
        .authenticate_request(Some(&format!("Bearer {token}")))
        .await
        .expect("JWT authentication should succeed");

    // CRITICAL SECURITY FIX VERIFICATION
    // Before: JWT tokens had rate_limit: None (unlimited access)
    // After: JWT tokens have proper rate limiting based on user tier

    let rate_limit = &auth_result.rate_limit;

    // Verify JWT now has rate limiting (not None!)
    assert!(
        !rate_limit.is_rate_limited,
        "Fresh JWT should not be rate limited yet"
    );

    // Verify JWT has proper limits (10,000 for Starter tier)
    assert_eq!(
        rate_limit.limit,
        Some(10_000),
        "SECURITY FIX: JWT should have Starter tier limit, not unlimited!"
    );

    // Verify remaining requests are tracked
    assert_eq!(
        rate_limit.remaining,
        Some(10_000),
        "JWT should track remaining requests"
    );

    // Verify reset time is set
    assert!(rate_limit.reset_at.is_some(), "JWT should have reset time");

    // Verify tier tracking
    assert_eq!(rate_limit.tier, "starter", "JWT should show user's tier");

    // Verify auth method tracking
    assert_eq!(
        rate_limit.auth_method, "jwt_token",
        "Should identify as JWT token authentication"
    );

    println!("SECURITY FIX VERIFIED: JWT tokens now have proper rate limiting!");
    println!("   Before: Unlimited access (critical vulnerability)");
    println!("   After: {:?} requests/month limit", rate_limit.limit);
    println!("   Tier: {}", rate_limit.tier);
    println!("   Auth Method: {}", rate_limit.auth_method);
}
