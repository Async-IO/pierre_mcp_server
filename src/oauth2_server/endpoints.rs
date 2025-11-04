// ABOUTME: OAuth 2.0 authorization and token endpoints implementation
// ABOUTME: Handles OAuth 2.0 flow with JWT tokens as access tokens for MCP client compatibility
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - String ownership transfers to struct constructors (OAuth2AuthCode, TokenResponse)
// - Arc clone for database manager creation

use super::client_registration::ClientRegistrationManager;
use super::models::{
    AuthorizeRequest, AuthorizeResponse, OAuth2AuthCode, OAuth2Error, TokenRequest, TokenResponse,
};
use crate::admin::jwks::JwksManager;
use crate::auth::AuthManager;
use crate::database_plugins::DatabaseProvider;
use crate::errors::AppError;
use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use chrono::{Duration, Utc};
use ring::rand::{SecureRandom, SystemRandom};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use subtle::ConstantTimeEq;
use uuid::Uuid;

/// Parameters for authorization code generation
struct AuthCodeParams<'a> {
    client_id: &'a str,
    user_id: Uuid,
    tenant_id: &'a str,
    redirect_uri: &'a str,
    scope: Option<&'a str>,
    state: Option<&'a str>,
    code_challenge: Option<&'a str>,
    code_challenge_method: Option<&'a str>,
}

/// OAuth 2.0 Authorization Server
pub struct OAuth2AuthorizationServer {
    client_manager: ClientRegistrationManager,
    auth_manager: Arc<AuthManager>,
    jwks_manager: Arc<JwksManager>,
    database: Arc<crate::database_plugins::factory::Database>,
}

impl OAuth2AuthorizationServer {
    #[must_use]
    pub fn new(
        database: Arc<crate::database_plugins::factory::Database>,
        auth_manager: Arc<AuthManager>,
        jwks_manager: Arc<JwksManager>,
    ) -> Self {
        let client_manager = ClientRegistrationManager::new(database.clone()); // Safe: Arc clone for manager construction

        Self {
            client_manager,
            auth_manager,
            jwks_manager,
            database,
        }
    }

    /// Handle authorization request (GET /oauth/authorize)
    ///
    /// # Errors
    /// Returns an error if client validation fails, invalid parameters, or authorization code generation fails
    pub async fn authorize(
        &self,
        request: AuthorizeRequest,
        user_id: Option<Uuid>,     // From authentication
        tenant_id: Option<String>, // From JWT claims
    ) -> Result<AuthorizeResponse, OAuth2Error> {
        // Validate client
        let client = self
            .client_manager
            .get_client(&request.client_id)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Client lookup failed for client_id={}: {:#}",
                    request.client_id,
                    e
                );
                OAuth2Error::invalid_client()
            })?;

        // Validate response type
        if request.response_type != "code" {
            return Err(OAuth2Error::invalid_request(
                "Only 'code' response_type is supported",
            ));
        }

        // Validate redirect URI
        if !client.redirect_uris.contains(&request.redirect_uri) {
            return Err(OAuth2Error::invalid_request("Invalid redirect_uri"));
        }

        // Validate PKCE parameters (RFC 7636)
        if let Some(ref code_challenge) = request.code_challenge {
            // Validate code_challenge format (base64url-encoded, 43-128 characters)
            if code_challenge.len() < 43 || code_challenge.len() > 128 {
                return Err(OAuth2Error::invalid_request(
                    "code_challenge must be between 43 and 128 characters",
                ));
            }

            // Validate code_challenge_method - only S256 is allowed (RFC 7636 security best practice)
            let method = request.code_challenge_method.as_deref().unwrap_or("S256");
            if method != "S256" {
                return Err(OAuth2Error::invalid_request(
                    "code_challenge_method must be 'S256' (plain method is not supported for security reasons)",
                ));
            }
        } else {
            // PKCE is required for authorization code flow
            return Err(OAuth2Error::invalid_request(
                "code_challenge is required for authorization_code flow (PKCE)",
            ));
        }

        // For now, we'll skip the consent screen and auto-approve
        // In a real implementation, this would redirect to a consent page
        let user_id =
            user_id.ok_or_else(|| OAuth2Error::invalid_request("User authentication required"))?;

        // Generate authorization code with tenant isolation and state binding
        let tenant_id = tenant_id.unwrap_or_else(|| user_id.to_string());
        let auth_code = self
            .generate_authorization_code(AuthCodeParams {
                client_id: &request.client_id,
                user_id,
                tenant_id: &tenant_id,
                redirect_uri: &request.redirect_uri,
                scope: request.scope.as_deref(),
                state: request.state.as_deref(),
                code_challenge: request.code_challenge.as_deref(),
                code_challenge_method: request.code_challenge_method.as_deref(),
            })
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to generate authorization code for client_id={}: {:#}",
                    request.client_id,
                    e
                );
                OAuth2Error::invalid_request("Failed to generate authorization code")
            })?;

        Ok(AuthorizeResponse {
            code: auth_code,
            state: request.state,
        })
    }

    /// Handle token request (POST /oauth/token)
    ///
    /// # Errors
    /// Returns an error if client validation fails or token generation fails
    pub async fn token(&self, request: TokenRequest) -> Result<TokenResponse, OAuth2Error> {
        // ALWAYS validate client credentials for ALL grant types (RFC 6749 Section 6)
        // RFC 6749 §6 states: "If the client type is confidential or the client was issued
        // client credentials, the client MUST authenticate with the authorization server"
        // MCP clients are confidential clients, so authentication is REQUIRED
        let _ = self
            .client_manager
            .validate_client(&request.client_id, &request.client_secret)
            .await?;

        match request.grant_type.as_str() {
            "authorization_code" => self.handle_authorization_code_grant(request).await,
            "client_credentials" => self.handle_client_credentials_grant(request),
            "refresh_token" => self.handle_refresh_token_grant(request).await,
            _ => Err(OAuth2Error::unsupported_grant_type()),
        }
    }

    /// Handle authorization code grant
    async fn handle_authorization_code_grant(
        &self,
        request: TokenRequest,
    ) -> Result<TokenResponse, OAuth2Error> {
        let code = request
            .code
            .ok_or_else(|| OAuth2Error::invalid_request("Missing authorization code"))?;

        let redirect_uri = request
            .redirect_uri
            .ok_or_else(|| OAuth2Error::invalid_request("Missing redirect_uri"))?;

        // Validate and consume authorization code (with PKCE verification)
        let auth_code = self
            .validate_and_consume_auth_code(
                &code,
                &request.client_id,
                &redirect_uri,
                request.code_verifier.as_deref(),
            )
            .await?;

        // Generate JWT access token
        let access_token = self
            .generate_access_token(
                &request.client_id,
                Some(auth_code.user_id),
                auth_code.scope.as_deref(),
            )
            .map_err(|e| {
                tracing::error!(
                    "Failed to generate access token for client_id={}: {:#}",
                    request.client_id,
                    e
                );
                OAuth2Error::invalid_request("Failed to generate access token")
            })?;

        // Generate refresh token
        let refresh_token_value = Self::generate_refresh_token().map_err(|e| {
            tracing::error!("Failed to generate secure refresh token: {:#}", e);
            OAuth2Error::invalid_request("Failed to generate secure refresh token")
        })?;
        let refresh_token_expires_at = Utc::now() + Duration::days(30); // 30 days

        let refresh_token = super::models::OAuth2RefreshToken {
            token: refresh_token_value.clone(),   // Safe: Clone for storage
            client_id: request.client_id.clone(), // Safe: Clone for ownership
            user_id: auth_code.user_id,
            tenant_id: auth_code.tenant_id.clone(), // Safe: Clone for tenant isolation
            scope: auth_code.scope.clone(),         // Safe: Clone for storage
            expires_at: refresh_token_expires_at,
            created_at: Utc::now(),
            revoked: false,
        };

        // Store refresh token
        self.store_refresh_token(&refresh_token)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to store refresh token for client_id={}: {:#}",
                    request.client_id,
                    e
                );
                OAuth2Error::invalid_request("Failed to store refresh token")
            })?;

        Ok(TokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: 3600, // 1 hour
            scope: auth_code.scope,
            refresh_token: Some(refresh_token_value),
        })
    }

    /// Handle client credentials grant
    fn handle_client_credentials_grant(
        &self,
        request: TokenRequest,
    ) -> Result<TokenResponse, OAuth2Error> {
        // Generate JWT access token for client
        let access_token = self
            .generate_access_token(
                &request.client_id,
                None, // No user for client credentials
                request.scope.as_deref(),
            )
            .map_err(|e| {
                tracing::error!(
                    "Failed to generate client credentials access token for client_id={}: {:#}",
                    request.client_id,
                    e
                );
                OAuth2Error::invalid_request("Failed to generate access token")
            })?;

        Ok(TokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: 3600, // 1 hour
            scope: request.scope,
            refresh_token: None,
        })
    }

    /// Handle refresh token grant with rotation
    async fn handle_refresh_token_grant(
        &self,
        request: TokenRequest,
    ) -> Result<TokenResponse, OAuth2Error> {
        let refresh_token_value = request
            .refresh_token
            .ok_or_else(|| OAuth2Error::invalid_request("Missing refresh_token"))?;

        // Validate and atomically consume existing refresh token (already marks as revoked)
        let old_refresh_token = self
            .validate_and_consume_refresh_token(&refresh_token_value, &request.client_id)
            .await?;

        // Generate new access token
        let access_token = self
            .generate_access_token(
                &request.client_id,
                Some(old_refresh_token.user_id),
                old_refresh_token.scope.as_deref(),
            )
            .map_err(|e| {
                tracing::error!(
                    "Failed to generate access token from refresh for client_id={}: {:#}",
                    request.client_id,
                    e
                );
                OAuth2Error::invalid_request("Failed to generate access token")
            })?;

        // Generate new refresh token (rotation)
        let new_refresh_token_value = Self::generate_refresh_token().map_err(|e| {
            tracing::error!(
                "Failed to generate new refresh token during rotation: {:#}",
                e
            );
            OAuth2Error::invalid_request("Failed to generate secure refresh token")
        })?;
        let refresh_token_expires_at = Utc::now() + Duration::days(30); // 30 days

        let new_refresh_token = super::models::OAuth2RefreshToken {
            token: new_refresh_token_value.clone(), // Safe: Clone for storage
            client_id: request.client_id.clone(),   // Safe: Clone for ownership
            user_id: old_refresh_token.user_id,
            tenant_id: old_refresh_token.tenant_id.clone(), // Safe: Clone for tenant isolation
            scope: old_refresh_token.scope.clone(),         // Safe: Clone for storage
            expires_at: refresh_token_expires_at,
            created_at: Utc::now(),
            revoked: false,
        };

        // Store new refresh token
        self.store_refresh_token(&new_refresh_token)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to store new refresh token for client_id={}: {:#}",
                    request.client_id,
                    e
                );
                OAuth2Error::invalid_request("Failed to store new refresh token")
            })?;

        tracing::info!(
            "Refresh token rotated for client {} and user {}",
            request.client_id,
            old_refresh_token.user_id
        );

        Ok(TokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: 3600, // 1 hour
            scope: old_refresh_token.scope,
            refresh_token: Some(new_refresh_token_value),
        })
    }

    /// Generate authorization code
    async fn generate_authorization_code(&self, params: AuthCodeParams<'_>) -> Result<String> {
        let code = Self::generate_random_string(32)?;
        let expires_at = Utc::now() + Duration::minutes(10); // 10 minute expiry

        let auth_code = OAuth2AuthCode {
            code: code.clone(), // Safe: String ownership for OAuth2AuthCode struct
            client_id: params.client_id.to_string(),
            user_id: params.user_id,
            tenant_id: params.tenant_id.to_string(),
            redirect_uri: params.redirect_uri.to_string(),
            scope: params.scope.map(std::string::ToString::to_string),
            expires_at,
            used: false,
            state: params.state.map(std::string::ToString::to_string),
            code_challenge: params.code_challenge.map(std::string::ToString::to_string),
            code_challenge_method: params
                .code_challenge_method
                .map(std::string::ToString::to_string),
        };

        self.store_auth_code(&auth_code).await?;

        // Server-Side State Validation (Defense-in-Depth CSRF Protection)
        //
        // RFC 6749 § 10.12 BASELINE: State is client-side CSRF protection. Server echoes state unchanged.
        // The state parameter is OPAQUE to the server - clients generate it, store it in their session,
        // and validate it matches on callback. Server's only job is to echo it back.
        //
        // OWASP ENHANCEMENT: We ALSO validate state server-side for defense-in-depth security.
        //
        // Why defense-in-depth?
        // 1. Early CSRF Detection: Detects attacks at the server level before client validation
        // 2. Replay Prevention: 10-minute TTL + single-use flag prevents state reuse
        // 3. Client Binding: State bound to client_id prevents cross-client attacks
        // 4. Tenant Isolation: State bound to tenant_id enforces multi-tenant security
        // 5. Audit Trail: Server-side validation provides security event logging
        //
        // Implementation: oauth2_states table (src/database/mod.rs:232)
        // Consumption: validate_and_consume_auth_code() below (line ~457)
        // Tests: tests/oauth2_state_validation_test.rs (7 security scenarios)
        //
        // See docs/oauth2-server.md "State Parameter Validation" for integration guide
        if let Some(state_value) = params.state {
            let oauth2_state = super::models::OAuth2State {
                state: state_value.to_string(),
                client_id: params.client_id.to_string(),
                user_id: Some(params.user_id),
                tenant_id: Some(params.tenant_id.to_string()),
                redirect_uri: params.redirect_uri.to_string(),
                scope: params.scope.map(std::string::ToString::to_string),
                code_challenge: params.code_challenge.map(std::string::ToString::to_string),
                code_challenge_method: params
                    .code_challenge_method
                    .map(std::string::ToString::to_string),
                created_at: Utc::now(),
                expires_at,
                used: false,
            };

            if let Err(e) = self.database.store_oauth2_state(&oauth2_state).await {
                tracing::error!(
                    "Failed to store OAuth2 state for client_id={}: {:#}",
                    params.client_id,
                    e
                );
                return Err(e);
            }

            tracing::debug!(
                "Stored OAuth2 state for server-side validation: client_id={}, state_length={}",
                params.client_id,
                state_value.len()
            );
        }

        Ok(code)
    }

    /// Validate and consume authorization code
    async fn validate_and_consume_auth_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        code_verifier: Option<&str>,
    ) -> Result<OAuth2AuthCode, OAuth2Error> {
        // Atomically consume authorization code (prevents TOCTOU race conditions)
        // This validates client_id, redirect_uri, expiration, and used status in a single atomic operation
        let auth_code = self
            .database
            .consume_auth_code(code, client_id, redirect_uri, Utc::now())
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to atomically consume authorization code for client_id={}: {:#}",
                    client_id,
                    e
                );
                OAuth2Error::invalid_grant("Failed to consume authorization code")
            })?
            .ok_or_else(|| {
                tracing::warn!(
                    "Authorization code validation failed for client_id={}: code not found, already used, expired, or mismatched credentials",
                    client_id
                );
                OAuth2Error::invalid_grant("Invalid or expired authorization code")
            })?;

        // Server-Side State Consumption (Atomic CSRF Validation)
        //
        // This is the validation counterpart to state storage above (line ~389).
        //
        // consume_oauth2_state() performs ATOMIC validation with these security checks:
        // 1. State EXISTS in database (prevents fake states)
        // 2. State NOT EXPIRED (10-minute TTL, prevents replay of old states)
        // 3. State NOT USED (single-use flag, prevents replay attacks)
        // 4. client_id MATCHES (prevents cross-client state theft)
        // 5. Marks state as USED atomically (prevents TOCTOU race conditions)
        //
        // Why atomic consumption matters:
        // - Prevents race condition where two concurrent requests could reuse same state
        // - Database transaction ensures state marked used in same operation as retrieval
        // - Implementation: src/database_plugins/sqlite.rs:1796-1849 (with UPDATE ... WHERE used=0)
        //
        // Rejection scenarios (returns None):
        // - State not found in database
        // - State expired (created_at + TTL < now)
        // - State already used (used=true)
        // - client_id mismatch
        //
        // Tests: tests/oauth2_state_validation_test.rs:
        //   - test_state_replay_attack_prevention (line 127)
        //   - test_state_client_id_mismatch (line 288)
        //   - test_expired_state_rejection (line 190)
        if let Some(state_value) = &auth_code.state {
            let consumed_state = self
                .database
                .consume_oauth2_state(state_value, client_id, Utc::now())
                .await
                .map_err(|e| {
                    tracing::error!(
                        "Failed to consume OAuth2 state for client_id={}: {:#}",
                        client_id,
                        e
                    );
                    OAuth2Error::invalid_grant("Failed to validate state parameter")
                })?;

            // None indicates validation failure (state not found, expired, used, or client_id mismatch)
            if consumed_state.is_none() {
                tracing::warn!(
                    "OAuth2 state validation failed for client_id={}: state not found, already used, expired, or client_id mismatch",
                    client_id
                );
                return Err(OAuth2Error::invalid_grant(
                    "Invalid state parameter - possible CSRF attack detected",
                ));
            }

            tracing::debug!(
                "OAuth2 state validation successful for client_id={}, state_length={}",
                client_id,
                state_value.len()
            );
        }

        // Verify PKCE code_verifier (RFC 7636)
        // Note: PKCE verification happens AFTER atomic consumption to prevent code reuse on verification failure
        if let Some(stored_challenge) = &auth_code.code_challenge {
            let verifier = code_verifier
                .ok_or_else(|| OAuth2Error::invalid_grant("code_verifier is required (PKCE)"))?;

            // Validate verifier format per RFC 7636 Section 4.1
            // Length: 43-128 characters
            if verifier.len() < 43 || verifier.len() > 128 {
                return Err(OAuth2Error::invalid_grant(
                    "code_verifier must be between 43 and 128 characters",
                ));
            }

            // Characters: Only unreserved characters allowed: [A-Z] / [a-z] / [0-9] / "-" / "." / "_" / "~"
            if !verifier.chars().all(|c| {
                matches!(c,
                    'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '.' | '_' | '~'
                )
            }) {
                return Err(OAuth2Error::invalid_grant(
                    "code_verifier contains invalid characters (RFC 7636: only [A-Z], [a-z], [0-9], -, ., _, ~ allowed)",
                ));
            }

            let method = auth_code.code_challenge_method.as_deref().unwrap_or("S256");

            // Only S256 is supported - plain method is rejected for security reasons
            let computed_challenge = if method == "S256" {
                // SHA-256 hash of verifier, then base64url encode
                let mut hasher = Sha256::new();
                hasher.update(verifier.as_bytes());
                let hash = hasher.finalize();
                general_purpose::URL_SAFE_NO_PAD.encode(hash)
            } else {
                return Err(OAuth2Error::invalid_grant(
                    "Only S256 code_challenge_method is supported (plain method is not allowed for security reasons)",
                ));
            };

            // Constant-time comparison to prevent timing attacks
            if computed_challenge
                .as_bytes()
                .ct_eq(stored_challenge.as_bytes())
                .into()
            {
                tracing::debug!("PKCE verification successful for client {}", client_id);
            } else {
                tracing::warn!(
                    "PKCE verification failed for client {} - code_verifier does not match code_challenge",
                    client_id
                );
                return Err(OAuth2Error::invalid_grant("Invalid code_verifier"));
            }
        } else if code_verifier.is_some() {
            // Client provided verifier but no challenge was stored
            return Err(OAuth2Error::invalid_grant(
                "code_verifier provided but no code_challenge was issued",
            ));
        }

        Ok(auth_code)
    }

    /// Generate JWT access token with RS256 asymmetric signing
    fn generate_access_token(
        &self,
        client_id: &str,
        user_id: Option<Uuid>,
        scope: Option<&str>,
    ) -> Result<String> {
        let scopes = scope.map_or_else(
            || {
                tracing::debug!(
                    client_id = %client_id,
                    user_id = ?user_id,
                    "No scopes provided for token generation, using empty scope list"
                );
                Vec::new()
            },
            |s| {
                s.split(' ')
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
            },
        );

        user_id.map_or_else(
            || {
                self.auth_manager.generate_client_credentials_token(
                    &self.jwks_manager,
                    client_id,
                    &scopes,
                    None, // tenant_id for client credentials
                )
            },
            |uid| {
                self.auth_manager.generate_oauth_access_token(
                    &self.jwks_manager,
                    &uid,
                    &scopes,
                    None,
                )
            },
        )
    }

    /// Generate random string for codes
    ///
    /// # Errors
    /// Returns an error if system RNG fails - this is a critical security failure
    /// and the server cannot operate securely without working RNG
    fn generate_random_string(length: usize) -> Result<String> {
        let rng = SystemRandom::new();
        let mut bytes = vec![0u8; length];

        rng.fill(&mut bytes).map_err(|e| {
            tracing::error!(
                "CRITICAL: SystemRandom failed - cannot generate secure random bytes: {}",
                e
            );
            AppError::internal("System RNG failure - server cannot operate securely")
        })?;

        // Convert to URL-safe base64
        Ok(general_purpose::URL_SAFE_NO_PAD.encode(&bytes))
    }

    /// Store authorization code (database operation)
    async fn store_auth_code(&self, auth_code: &OAuth2AuthCode) -> Result<()> {
        self.database.store_oauth2_auth_code(auth_code).await
    }

    /// Generate refresh token with secure randomness
    ///
    /// # Errors
    /// Returns an error if system RNG fails
    fn generate_refresh_token() -> Result<String> {
        // Generate 32 bytes (256 bits) of secure random data
        Self::generate_random_string(32)
    }

    /// Store refresh token (database operation)
    async fn store_refresh_token(
        &self,
        refresh_token: &super::models::OAuth2RefreshToken,
    ) -> Result<()> {
        self.database
            .store_oauth2_refresh_token(refresh_token)
            .await
    }

    /// Validate and consume refresh token
    async fn validate_and_consume_refresh_token(
        &self,
        token: &str,
        client_id: &str,
    ) -> Result<super::models::OAuth2RefreshToken, OAuth2Error> {
        // Atomically consume refresh token (prevents TOCTOU race conditions)
        // This validates client_id, revoked status, and expiration in a single atomic operation
        let refresh_token = self
            .database
            .consume_refresh_token(token, client_id, Utc::now())
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to atomically consume refresh token for client_id={}: {:#}",
                    client_id,
                    e
                );
                OAuth2Error::invalid_grant("Failed to consume refresh token")
            })?
            .ok_or_else(|| {
                tracing::warn!(
                    "Refresh token validation failed for client_id={}: token not found, already revoked, expired, or mismatched client",
                    client_id
                );
                OAuth2Error::invalid_grant("Invalid or expired refresh token")
            })?;

        Ok(refresh_token)
    }

    /// Validate and optionally refresh an access token
    ///
    /// This endpoint checks if a JWT access token is valid. If valid, it returns the expiration time.
    /// If expired but a refresh token is provided, it attempts to refresh and return new tokens.
    /// If invalid or cannot be refreshed, it returns an error with the reason.
    ///
    /// # Errors
    /// Returns an error if token validation fails catastrophically (database errors, etc.)
    pub async fn validate_and_refresh(
        &self,
        access_token: &str,
        request: super::models::ValidateRefreshRequest,
    ) -> Result<super::models::ValidateRefreshResponse> {
        // Validate the JWT token
        match self
            .auth_manager
            .validate_token_detailed(access_token, &self.jwks_manager)
        {
            Ok(claims) => self.handle_valid_token_claims(claims).await,
            Err(validation_error) => {
                self.handle_token_validation_error(validation_error, access_token, &request)
                    .await
            }
        }
    }

    /// Handle valid token claims by checking user existence
    async fn handle_valid_token_claims(
        &self,
        claims: crate::auth::Claims,
    ) -> Result<super::models::ValidateRefreshResponse> {
        use super::models::{ValidateRefreshResponse, ValidationStatus};

        match Uuid::parse_str(&claims.sub) {
            Ok(user_id) => match self.database.get_user(user_id).await {
                Ok(Some(_user)) => Ok(ValidateRefreshResponse {
                    status: ValidationStatus::Valid,
                    expires_in: Some(claims.exp - Utc::now().timestamp()),
                    access_token: None,
                    refresh_token: None,
                    token_type: None,
                    reason: None,
                    requires_full_reauth: None,
                }),
                Ok(None) => Ok(Self::create_invalid_response("user_not_found")),
                Err(e) => {
                    tracing::error!("Database error while validating token: {}", e);
                    Ok(Self::create_invalid_response("database_error"))
                }
            },
            Err(_) => Ok(Self::create_invalid_response("invalid_user_id")),
        }
    }

    /// Handle JWT validation errors
    async fn handle_token_validation_error(
        &self,
        validation_error: crate::auth::JwtValidationError,
        expired_access_token: &str,
        request: &super::models::ValidateRefreshRequest,
    ) -> Result<super::models::ValidateRefreshResponse> {
        use super::models::{ValidateRefreshResponse, ValidationStatus};
        use crate::auth::JwtValidationError;

        match validation_error {
            JwtValidationError::TokenExpired { .. } => {
                if let Some(refresh_token_value) = &request.refresh_token {
                    tracing::info!(
                        "Access token expired, attempting refresh with provided refresh_token"
                    );

                    // Decode expired token to extract user_id and client info (without validation)
                    // We can safely decode expired tokens to read claims
                    match Self::decode_expired_token(expired_access_token) {
                        Ok(claims) => {
                            // Look up refresh token by value and verify it belongs to this user
                            match self
                                .lookup_and_validate_refresh_token(refresh_token_value, &claims.sub)
                                .await
                            {
                                Ok(refresh_token_data) => {
                                    // Generate new access token
                                    match self.generate_access_token(
                                        &refresh_token_data.client_id,
                                        Some(refresh_token_data.user_id),
                                        refresh_token_data.scope.as_deref(),
                                    ) {
                                        Ok(new_access_token) => {
                                            tracing::info!(
                                                "Successfully refreshed access token for user {}",
                                                claims.sub
                                            );
                                            Ok(ValidateRefreshResponse {
                                                status: ValidationStatus::Refreshed,
                                                expires_in: Some(3600), // 1 hour
                                                access_token: Some(new_access_token),
                                                refresh_token: Some(refresh_token_value.clone()),
                                                token_type: Some("Bearer".to_string()),
                                                reason: None,
                                                requires_full_reauth: None,
                                            })
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                "Failed to generate new access token: {}",
                                                e
                                            );
                                            Ok(Self::create_invalid_response(
                                                "refresh_failed_token_generation",
                                            ))
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!("Refresh token validation failed: {}", e);
                                    Ok(Self::create_invalid_response("invalid_refresh_token"))
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to decode expired token: {}", e);
                            Ok(Self::create_invalid_response("malformed_expired_token"))
                        }
                    }
                } else {
                    Ok(Self::create_invalid_response("token_expired"))
                }
            }
            JwtValidationError::TokenInvalid { reason } => Ok(Self::create_invalid_response(
                &format!("invalid_signature: {reason}"),
            )),
            JwtValidationError::TokenMalformed { details } => Ok(Self::create_invalid_response(
                &format!("malformed_token: {details}"),
            )),
        }
    }

    /// Create an invalid token response
    fn create_invalid_response(reason: &str) -> super::models::ValidateRefreshResponse {
        use super::models::{ValidateRefreshResponse, ValidationStatus};

        ValidateRefreshResponse {
            status: ValidationStatus::Invalid,
            expires_in: None,
            access_token: None,
            refresh_token: None,
            token_type: None,
            reason: Some(reason.to_string()),
            requires_full_reauth: Some(true),
        }
    }

    /// Decode an expired JWT token without validation to extract claims
    ///
    /// This is safe because we only need to read the claims, not trust them.
    /// The refresh token will be validated separately.
    fn decode_expired_token(token: &str) -> Result<crate::auth::Claims> {
        use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};

        // Create a permissive validation that doesn't check expiry or signature
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = false; // Don't validate expiration
        validation.insecure_disable_signature_validation(); // We just need to read claims

        // Decode without validation - we only need the claims data
        let token_data = decode::<crate::auth::Claims>(
            token,
            &DecodingKey::from_secret(&[]), // Dummy key since we're not validating signature
            &validation,
        )
        .map_err(|e| {
            crate::errors::AppError::new(
                crate::errors::ErrorCode::AuthMalformed,
                format!("Failed to decode expired token: {e}"),
            )
        })?;

        Ok(token_data.claims)
    }

    /// Look up refresh token by value and validate it belongs to the specified user
    async fn lookup_and_validate_refresh_token(
        &self,
        refresh_token_value: &str,
        user_id_str: &str,
    ) -> Result<super::models::OAuth2RefreshToken> {
        // Parse user_id from string
        let user_id = Uuid::parse_str(user_id_str).map_err(|e| {
            crate::errors::AppError::new(
                crate::errors::ErrorCode::AuthMalformed,
                format!("Invalid user_id in token claims: {e}"),
            )
        })?;

        // Look up refresh token in database
        // We need to find it without knowing the client_id
        let refresh_token = self
            .database
            .get_refresh_token_by_value(refresh_token_value)
            .await
            .map_err(|e| {
                crate::errors::AppError::new(
                    crate::errors::ErrorCode::DatabaseError,
                    format!("Database error looking up refresh token: {e}"),
                )
            })?
            .ok_or_else(|| {
                crate::errors::AppError::new(
                    crate::errors::ErrorCode::ResourceNotFound,
                    "Refresh token not found",
                )
            })?;

        // Verify the refresh token belongs to this user
        if refresh_token.user_id != user_id {
            return Err(crate::errors::AppError::new(
                crate::errors::ErrorCode::AuthInvalid,
                "Refresh token does not belong to the user in the access token",
            )
            .into());
        }

        // Verify the refresh token hasn't expired
        if refresh_token.expires_at < Utc::now() {
            return Err(crate::errors::AppError::new(
                crate::errors::ErrorCode::AuthExpired,
                "Refresh token has expired",
            )
            .into());
        }

        // Verify the refresh token hasn't been revoked
        if refresh_token.revoked {
            return Err(crate::errors::AppError::new(
                crate::errors::ErrorCode::AuthInvalid,
                "Refresh token has been revoked",
            )
            .into());
        }

        Ok(refresh_token)
    }
}
