// Integration tests for CSRF token manager
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(missing_docs)]

use pierre_mcp_server::security::csrf::CsrfTokenManager;
use uuid::Uuid;

#[tokio::test]
async fn test_generate_csrf_token() -> anyhow::Result<()> {
    let manager = CsrfTokenManager::new();
    let user_id = Uuid::new_v4();

    let token = manager.generate_token(user_id).await?;

    // Token should be 64 characters (32 bytes hex encoded)
    assert_eq!(token.len(), 64, "CSRF token should be 64 characters");

    // Token should be valid hex
    assert!(
        token.chars().all(|c| c.is_ascii_hexdigit()),
        "CSRF token should be valid hex"
    );
    Ok(())
}

#[tokio::test]
async fn test_validate_csrf_token() -> anyhow::Result<()> {
    let manager = CsrfTokenManager::new();
    let user_id = Uuid::new_v4();
    let other_user_id = Uuid::new_v4();

    let token = manager.generate_token(user_id).await?;

    // Valid token for correct user
    assert!(
        manager.validate_token(&token, user_id).await.is_ok(),
        "Valid token should pass validation"
    );

    // Invalid token for different user
    assert!(
        manager.validate_token(&token, other_user_id).await.is_err(),
        "Token should fail for different user"
    );

    // Invalid token string
    assert!(
        manager
            .validate_token("invalid_token", user_id)
            .await
            .is_err(),
        "Invalid token should fail validation"
    );
    Ok(())
}

#[tokio::test]
async fn test_invalidate_csrf_token() -> anyhow::Result<()> {
    let manager = CsrfTokenManager::new();
    let user_id = Uuid::new_v4();

    let token = manager.generate_token(user_id).await?;

    // Token should be valid initially
    assert!(
        manager.validate_token(&token, user_id).await.is_ok(),
        "Token should be valid before invalidation"
    );

    // Invalidate token
    manager.invalidate_token(&token).await;

    // Token should be invalid after invalidation
    assert!(
        manager.validate_token(&token, user_id).await.is_err(),
        "Token should be invalid after invalidation"
    );
    Ok(())
}
