// ABOUTME: CSRF (Cross-Site Request Forgery) protection token generation and validation
// ABOUTME: Provides secure token-based CSRF protection for state-changing operations
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! CSRF protection module
//!
//! Generates cryptographically secure CSRF tokens and provides validation.
//! Tokens are tied to user sessions and have configurable expiration.

use crate::errors::AppResult;
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// CSRF token length in bytes (32 bytes = 256 bits)
const CSRF_TOKEN_LENGTH: usize = 32;

/// CSRF token expiration in seconds (30 minutes)
const CSRF_TOKEN_EXPIRY_SECS: i64 = 30 * 60;

/// CSRF token metadata (token itself is the `HashMap` key)
#[derive(Clone)]
struct CsrfToken {
    user_id: uuid::Uuid,
    expires_at: chrono::DateTime<chrono::Utc>,
}

/// CSRF token manager with in-memory storage
///
/// In production, consider using Redis or database storage
/// for distributed systems.
pub struct CsrfTokenManager {
    tokens: Arc<RwLock<HashMap<String, CsrfToken>>>,
}

impl CsrfTokenManager {
    /// Create a new CSRF token manager
    #[must_use]
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Generate a new CSRF token for a user
    ///
    /// # Arguments
    /// * `user_id` - The user ID to associate with the token
    ///
    /// # Returns
    /// A cryptographically secure random token string
    ///
    /// # Errors
    /// This function is currently infallible but returns `AppResult` for future extensibility
    pub async fn generate_token(&self, user_id: uuid::Uuid) -> AppResult<String> {
        // Generate cryptographically secure random bytes
        let random_bytes: Vec<u8> = (0..CSRF_TOKEN_LENGTH)
            .map(|_| rand::thread_rng().gen())
            .collect();

        let token = hex::encode(random_bytes);
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(CSRF_TOKEN_EXPIRY_SECS);

        // Store token and cleanup expired tokens
        let mut tokens = self.tokens.write().await;
        tokens.insert(
            token.clone(),
            CsrfToken {
                user_id,
                expires_at,
            },
        );

        // Cleanup expired tokens (simple cleanup on insert)
        Self::cleanup_expired_tokens_locked(&mut tokens);
        drop(tokens);

        Ok(token)
    }

    /// Validate a CSRF token
    ///
    /// # Arguments
    /// * `token` - The token to validate
    /// * `user_id` - The expected user ID
    ///
    /// # Returns
    /// `Ok(())` if valid, `Err` if invalid or expired
    ///
    /// # Errors
    /// Returns an error if:
    /// - Token is not found
    /// - Token has expired
    /// - Token user ID doesn't match the provided user ID
    pub async fn validate_token(&self, token: &str, user_id: uuid::Uuid) -> AppResult<()> {
        let csrf_token = {
            let tokens = self.tokens.read().await;
            tokens
                .get(token)
                .ok_or_else(|| crate::errors::AppError::auth_invalid("Invalid CSRF token"))?
                .clone()
        };

        // Check expiration
        if chrono::Utc::now() > csrf_token.expires_at {
            return Err(crate::errors::AppError::auth_invalid("CSRF token expired"));
        }

        // Check user ID
        if csrf_token.user_id != user_id {
            return Err(crate::errors::AppError::auth_invalid(
                "CSRF token user mismatch",
            ));
        }

        Ok(())
    }

    /// Invalidate a CSRF token after use (one-time use pattern)
    pub async fn invalidate_token(&self, token: &str) {
        let mut tokens = self.tokens.write().await;
        tokens.remove(token);
    }

    /// Cleanup expired tokens (internal helper)
    fn cleanup_expired_tokens_locked(tokens: &mut HashMap<String, CsrfToken>) {
        let now = chrono::Utc::now();
        tokens.retain(|_, csrf_token| csrf_token.expires_at > now);
    }

    /// Cleanup expired tokens (public method)
    pub async fn cleanup_expired_tokens(&self) {
        let mut tokens = self.tokens.write().await;
        Self::cleanup_expired_tokens_locked(&mut tokens);
    }
}

impl Default for CsrfTokenManager {
    fn default() -> Self {
        Self::new()
    }
}
