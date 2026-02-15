// ABOUTME: OAuth 2.0 authorization and token endpoints implementation
// ABOUTME: Handles OAuth 2.0 flow with JWT tokens as access tokens for MCP client compatibility
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - String ownership transfers to struct constructors (OAuth2AuthCode, TokenResponse)
// - Arc clone for database manager creation

use super::client_registration::ClientRegistrationManager;
use super::models::{
    AuthorizeRequest, AuthorizeResponse, OAuth2AuthCode, OAuth2Error, TokenRequest, TokenResponse,
};
use crate::admin::jwks::JwksManager;
use crate::auth::{AuthManager, Claims, JwtValidationError};
use crate::database_plugins::factory::Database;
use crate::database_plugins::DatabaseProvider;
use crate::errors::{AppError, AppResult, ErrorCode};
use base64::{engine::general_purpose, Engine as _};
use chrono::{Duration, Utc};
use jsonwebtoken::dangerous::insecure_decode;
use ring::rand::{SecureRandom, SystemRandom};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::sync::Arc;
use subtle::ConstantTimeEq;
use tracing::{debug, error, info, warn};
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

/// Validate PKCE `code_verifier` format per RFC 7636 Section 4.1
fn validate_pkce_verifier_format(verifier: &str) -> Result<(), OAuth2Error> {
    // Length: 43-128 characters
    if verifier.len() < 43 || verifier.len() > 128 {
        return Err(OAuth2Error::invalid_grant(
            "code_verifier must be between 43 and 128 characters",
        ));
    }

    // Characters: Only unreserved characters allowed: [A-Z] / [a-z] / [0-9] / "-" / "." / "_" / "~"
    if !verifier
        .chars()
        .all(|c| matches!(c, 'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '.' | '_' | '~'))
    {
        return Err(OAuth2Error::invalid_grant(
            "code_verifier contains invalid characters (RFC 7636: only [A-Z], [a-z], [0-9], -, ., _, ~ allowed)",
        ));
    }

    Ok(())
}

/// Compute PKCE challenge from verifier using S256 method
fn compute_pkce_challenge(verifier: &str, method: &str) -> Result<String, OAuth2Error> {
    if method != "S256" {
        return Err(OAuth2Error::invalid_grant(
            "Only S256 code_challenge_method is supported (plain method is not allowed for security reasons)",
        ));
    }

    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    Ok(general_purpose::URL_SAFE_NO_PAD.encode(hash))
}

/// Verify PKCE challenge using constant-time comparison
fn verify_pkce_challenge(
    stored_challenge: &str,
    code_verifier: Option<&str>,
    code_challenge_method: Option<&str>,
    client_id: &str,
) -> Result<(), OAuth2Error> {
    let verifier = code_verifier
        .ok_or_else(|| OAuth2Error::invalid_grant("code_verifier is required (PKCE)"))?;

    validate_pkce_verifier_format(verifier)?;

    let method = code_challenge_method.unwrap_or("S256");
    let computed_challenge = compute_pkce_challenge(verifier, method)?;

    // Constant-time comparison to prevent timing attacks
    if computed_challenge
        .as_bytes()
        .ct_eq(stored_challenge.as_bytes())
        .into()
    {
        debug!("PKCE verification successful for client {}", client_id);
        Ok(())
    } else {
        warn!(
            "PKCE verification failed for client {} - code_verifier does not match code_challenge",
            client_id
        );
        Err(OAuth2Error::invalid_grant("Invalid code_verifier"))
    }
}

/// OAuth 2.0 Authorization Server
pub struct OAuth2AuthorizationServer {
    client_manager: ClientRegistrationManager,
    auth_manager: Arc<AuthManager>,
    jwks_manager: Arc<JwksManager>,
    database: Arc<Database>,
}

impl OAuth2AuthorizationServer {
    /// Creates a new `OAuth2` authorization server instance
    #[must_use]
    pub fn new(
        database: Arc<Database>,
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
                error!(
                    "Client lookup failed for client_id={}: {:#}",
                    request.client_id, e
                );
                OAuth2Error::invalid_client()
            })?;

        // Validate response type is supported by the server
        if request.response_type != "code" {
            return Err(OAuth2Error::invalid_request(
                "Only 'code' response_type is supported",
            ));
        }

        // Validate client is registered for this response_type (RFC 6749 Section 3.1.1)
        if !client.response_types.contains(&request.response_type) {
            return Err(OAuth2Error::unauthorized_client(
                "Client is not registered for the requested response_type",
            ));
        }

        // Validate requested scope is within client's registered scope (RFC 6749 Section 3.3)
        // If client has no registered scope (None), any requested scope is allowed (no restriction)
        if let Some(ref requested_scope) = request.scope {
            if let Some(ref allowed_scope) = client.scope {
                let allowed_scopes: HashSet<&str> = allowed_scope.split(' ').collect();
                for scope in requested_scope.split(' ') {
                    if !allowed_scopes.contains(scope) {
                        return Err(OAuth2Error::invalid_scope(&format!(
                            "Client is not authorized for scope '{scope}'"
                        )));
                    }
                }
            }
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
        // Resolve tenant_id from JWT claims (active_tenant_id) or database lookup
        let tenant_id = if let Some(tid) = tenant_id {
            tid
        } else {
            // Resolve actual tenant from database - use first tenant user belongs to
            let tenants = self
                .database
                .list_tenants_for_user(user_id)
                .await
                .map_err(|e| {
                    error!("Failed to get tenants for user {}: {:#}", user_id, e);
                    OAuth2Error::invalid_request("Failed to resolve user tenant")
                })?;
            tenants.first().map(|t| t.id.to_string()).ok_or_else(|| {
                error!("User {} has no tenant memberships", user_id);
                OAuth2Error::invalid_request("User does not belong to any tenant")
            })?
        };
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
                error!(
                    "Failed to generate authorization code for client_id={}: {:#}",
                    request.client_id, e
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
        let client = self
            .client_manager
            .validate_client(&request.client_id, &request.client_secret)
            .await
            .inspect_err(|e| {
                error!(
                    client_id = %request.client_id,
                    grant_type = %request.grant_type,
                    error = ?e,
                    "OAuth client validation failed"
                );
            })?;

        // Enforce client's registered grant_types (RFC 6749 Section 2)
        // Clients can only use grant types they were registered for.
        // Per RFC 6749 Section 6, refresh_token is implicitly allowed when the client
        // is registered for authorization_code (since the auth code flow issues refresh tokens).
        let grant_allowed = client.grant_types.contains(&request.grant_type)
            || (request.grant_type == "refresh_token"
                && client
                    .grant_types
                    .iter()
                    .any(|gt| gt == "authorization_code"));
        if !grant_allowed {
            warn!(
                "Client {} attempted grant_type '{}' but is only registered for {:?}",
                request.client_id, request.grant_type, client.grant_types
            );
            return Err(OAuth2Error::unauthorized_client(
                "Client is not registered for the requested grant_type",
            ));
        }

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
                error!(
                    "Failed to generate access token for client_id={}: {:#}",
                    request.client_id, e
                );
                OAuth2Error::invalid_request("Failed to generate access token")
            })?;

        // Generate refresh token
        let refresh_token_value = Self::generate_refresh_token().map_err(|e| {
            error!("Failed to generate secure refresh token: {:#}", e);
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
                error!(
                    "Failed to store refresh token for client_id={}: {:#}",
                    request.client_id, e
                );
                OAuth2Error::invalid_request("Failed to store refresh token")
            })?;

        Ok(TokenResponse {
            access_token,
            token_type: "Bearer".to_owned(),
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
                error!(
                    "Failed to generate client credentials access token for client_id={}: {:#}",
                    request.client_id, e
                );
                OAuth2Error::invalid_request("Failed to generate access token")
            })?;

        Ok(TokenResponse {
            access_token,
            token_type: "Bearer".to_owned(),
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
                error!(
                    "Failed to generate access token from refresh for client_id={}: {:#}",
                    request.client_id, e
                );
                OAuth2Error::invalid_request("Failed to generate access token")
            })?;

        // Generate new refresh token (rotation)
        let new_refresh_token_value = Self::generate_refresh_token().map_err(|e| {
            error!(
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
                error!(
                    "Failed to store new refresh token for client_id={}: {:#}",
                    request.client_id, e
                );
                OAuth2Error::invalid_request("Failed to store new refresh token")
            })?;

        info!(
            "Refresh token rotated for client {} and user {}",
            request.client_id, old_refresh_token.user_id
        );

        Ok(TokenResponse {
            access_token,
            token_type: "Bearer".to_owned(),
            expires_in: 3600, // 1 hour
            scope: old_refresh_token.scope,
            refresh_token: Some(new_refresh_token_value),
        })
    }

    /// Generate authorization code
    async fn generate_authorization_code(&self, params: AuthCodeParams<'_>) -> AppResult<String> {
        let code = Self::generate_random_string(32)?;
        let expires_at = Utc::now() + Duration::minutes(10); // 10 minute expiry

        let auth_code = OAuth2AuthCode {
            code: code.clone(), // Safe: String ownership for OAuth2AuthCode struct
            client_id: params.client_id.to_owned(),
            user_id: params.user_id,
            tenant_id: params.tenant_id.to_owned(),
            redirect_uri: params.redirect_uri.to_owned(),
            scope: params.scope.map(str::to_owned),
            expires_at,
            used: false,
            state: params.state.map(str::to_owned),
            code_challenge: params.code_challenge.map(str::to_owned),
            code_challenge_method: params.code_challenge_method.map(str::to_owned),
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
                state: state_value.to_owned(),
                client_id: params.client_id.to_owned(),
                user_id: Some(params.user_id),
                tenant_id: Some(params.tenant_id.to_owned()),
                redirect_uri: params.redirect_uri.to_owned(),
                scope: params.scope.map(str::to_owned),
                code_challenge: params.code_challenge.map(str::to_owned),
                code_challenge_method: params.code_challenge_method.map(str::to_owned),
                created_at: Utc::now(),
                expires_at,
                used: false,
            };

            if let Err(e) = self.database.store_oauth2_state(&oauth2_state).await {
                error!(
                    "Failed to store OAuth2 state for client_id={}: {:#}",
                    params.client_id, e
                );
                return Err(e);
            }

            debug!(
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
                error!(
                    "Failed to atomically consume authorization code for client_id={}: {:#}",
                    client_id,
                    e
                );
                OAuth2Error::invalid_grant("Failed to consume authorization code")
            })?
            .ok_or_else(|| {
                warn!(
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
                    error!(
                        "Failed to consume OAuth2 state for client_id={}: {:#}",
                        client_id, e
                    );
                    OAuth2Error::invalid_grant("Failed to validate state parameter")
                })?;

            // None indicates validation failure (state not found, expired, used, or client_id mismatch)
            if consumed_state.is_none() {
                warn!(
                    "OAuth2 state validation failed for client_id={}: state not found, already used, expired, or client_id mismatch",
                    client_id
                );
                return Err(OAuth2Error::invalid_grant(
                    "Invalid state parameter - possible CSRF attack detected",
                ));
            }

            debug!(
                "OAuth2 state validation successful for client_id={}, state_length={}",
                client_id,
                state_value.len()
            );
        }

        // Verify PKCE code_verifier (RFC 7636)
        // Note: PKCE verification happens AFTER atomic consumption to prevent code reuse on verification failure
        if let Some(stored_challenge) = &auth_code.code_challenge {
            verify_pkce_challenge(
                stored_challenge,
                code_verifier,
                auth_code.code_challenge_method.as_deref(),
                client_id,
            )?;
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
    ) -> AppResult<String> {
        let scopes = scope.map_or_else(
            || {
                debug!(
                    client_id = %client_id,
                    user_id = ?user_id,
                    "No scopes provided for token generation, using empty scope list"
                );
                Vec::new()
            },
            |s| s.split(' ').map(str::to_owned).collect::<Vec<_>>(),
        );

        user_id.map_or_else(
            || {
                self.auth_manager
                    .generate_client_credentials_token(
                        &self.jwks_manager,
                        client_id,
                        &scopes,
                        None, // tenant_id for client credentials
                    )
                    .map_err(|e| {
                        AppError::internal(format!(
                            "Failed to generate client credentials token: {e}"
                        ))
                    })
            },
            |uid| {
                self.auth_manager
                    .generate_oauth_access_token(&self.jwks_manager, &uid, &scopes, None)
                    .map_err(|e| {
                        AppError::internal(format!("Failed to generate OAuth access token: {e}"))
                    })
            },
        )
    }

    /// Generate random string for codes
    ///
    /// # Errors
    /// Returns an error if system RNG fails - this is a critical security failure
    /// and the server cannot operate securely without working RNG
    fn generate_random_string(length: usize) -> AppResult<String> {
        let rng = SystemRandom::new();
        let mut bytes = vec![0u8; length];

        rng.fill(&mut bytes).map_err(|e| {
            error!(
                "CRITICAL: SystemRandom failed - cannot generate secure random bytes: {}",
                e
            );
            AppError::internal("System RNG failure - server cannot operate securely")
        })?;

        // Convert to URL-safe base64
        Ok(general_purpose::URL_SAFE_NO_PAD.encode(&bytes))
    }

    /// Store authorization code (database operation)
    async fn store_auth_code(&self, auth_code: &OAuth2AuthCode) -> AppResult<()> {
        self.database.store_oauth2_auth_code(auth_code).await
    }

    /// Generate refresh token with secure randomness
    ///
    /// # Errors
    /// Returns an error if system RNG fails
    fn generate_refresh_token() -> AppResult<String> {
        // Generate 32 bytes (256 bits) of secure random data
        Self::generate_random_string(32)
    }

    /// Store refresh token (database operation)
    async fn store_refresh_token(
        &self,
        refresh_token: &super::models::OAuth2RefreshToken,
    ) -> AppResult<()> {
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
                error!(
                    "Failed to atomically consume refresh token for client_id={}: {:#}",
                    client_id,
                    e
                );
                OAuth2Error::invalid_grant("Failed to consume refresh token")
            })?
            .ok_or_else(|| {
                warn!(
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
    ) -> AppResult<super::models::ValidateRefreshResponse> {
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
        claims: Claims,
    ) -> AppResult<super::models::ValidateRefreshResponse> {
        use super::models::{ValidateRefreshResponse, ValidationStatus};

        match Uuid::parse_str(&claims.sub) {
            // SECURITY: Global lookup — OAuth2 token validation, no tenant context
            Ok(user_id) => match self.database.get_user_global(user_id).await {
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
                    error!("Database error while validating token: {}", e);
                    Ok(Self::create_invalid_response("database_error"))
                }
            },
            Err(_) => Ok(Self::create_invalid_response("invalid_user_id")),
        }
    }

    /// Create a successful refresh response
    fn create_refreshed_response(
        new_access_token: String,
        refresh_token_value: &str,
    ) -> super::models::ValidateRefreshResponse {
        use super::models::{ValidateRefreshResponse, ValidationStatus};
        ValidateRefreshResponse {
            status: ValidationStatus::Refreshed,
            expires_in: Some(3600), // 1 hour
            access_token: Some(new_access_token),
            refresh_token: Some(refresh_token_value.to_owned()),
            token_type: Some("Bearer".to_owned()),
            reason: None,
            requires_full_reauth: None,
        }
    }

    /// Attempt to refresh an expired token using a refresh token
    async fn attempt_token_refresh(
        &self,
        refresh_token_value: &str,
        claims: &Claims,
    ) -> AppResult<super::models::ValidateRefreshResponse> {
        // Look up refresh token by value and verify it belongs to this user
        let refresh_token_data = match self
            .lookup_and_validate_refresh_token(refresh_token_value, &claims.sub)
            .await
        {
            Ok(data) => data,
            Err(e) => {
                warn!("Refresh token validation failed: {}", e);
                return Ok(Self::create_invalid_response("invalid_refresh_token"));
            }
        };

        // Generate new access token
        match self.generate_access_token(
            &refresh_token_data.client_id,
            Some(refresh_token_data.user_id),
            refresh_token_data.scope.as_deref(),
        ) {
            Ok(new_access_token) => {
                info!(
                    "Successfully refreshed access token for user {}",
                    claims.sub
                );
                Ok(Self::create_refreshed_response(
                    new_access_token,
                    refresh_token_value,
                ))
            }
            Err(e) => {
                error!("Failed to generate new access token: {}", e);
                Ok(Self::create_invalid_response(
                    "refresh_failed_token_generation",
                ))
            }
        }
    }

    /// Handle expired token with optional refresh
    async fn handle_expired_token(
        &self,
        expired_access_token: &str,
        refresh_token_value: Option<&String>,
    ) -> AppResult<super::models::ValidateRefreshResponse> {
        let Some(refresh_token_value) = refresh_token_value else {
            return Ok(Self::create_invalid_response("token_expired"));
        };

        info!("Access token expired, attempting refresh with provided refresh_token");

        // Decode expired token to extract user_id and client info (without validation)
        let claims = match Self::decode_expired_token(expired_access_token) {
            Ok(claims) => claims,
            Err(e) => {
                error!("Failed to decode expired token: {}", e);
                return Ok(Self::create_invalid_response("malformed_expired_token"));
            }
        };

        self.attempt_token_refresh(refresh_token_value, &claims)
            .await
    }

    /// Handle JWT validation errors
    async fn handle_token_validation_error(
        &self,
        validation_error: JwtValidationError,
        expired_access_token: &str,
        request: &super::models::ValidateRefreshRequest,
    ) -> AppResult<super::models::ValidateRefreshResponse> {
        use JwtValidationError;

        match validation_error {
            JwtValidationError::TokenExpired { .. } => {
                self.handle_expired_token(expired_access_token, request.refresh_token.as_ref())
                    .await
            }
            JwtValidationError::TokenInvalid { reason } => {
                debug!("Token invalid: {reason}");
                Ok(Self::create_invalid_response("invalid_signature"))
            }
            JwtValidationError::TokenMalformed { details } => {
                debug!("Token malformed: {details}");
                Ok(Self::create_invalid_response("malformed_token"))
            }
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
            reason: Some(reason.to_owned()),
            requires_full_reauth: Some(true),
        }
    }

    /// Decode an expired JWT token without validation to extract claims
    ///
    /// This is safe because we only need to read the claims, not trust them.
    /// The refresh token will be validated separately.
    fn decode_expired_token(token: &str) -> AppResult<Claims> {
        // Decode without validation - we only need the claims data.
        // The refresh token will be validated separately.
        let token_data = insecure_decode::<Claims>(token).map_err(|e| {
            AppError::new(
                ErrorCode::AuthMalformed,
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
    ) -> AppResult<super::models::OAuth2RefreshToken> {
        // Parse user_id from string
        let user_id = Uuid::parse_str(user_id_str).map_err(|e| {
            AppError::new(
                ErrorCode::AuthMalformed,
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
                AppError::new(
                    ErrorCode::DatabaseError,
                    format!("Database error looking up refresh token: {e}"),
                )
            })?
            .ok_or_else(|| AppError::new(ErrorCode::ResourceNotFound, "Refresh token not found"))?;

        // Verify the refresh token belongs to this user
        if refresh_token.user_id != user_id {
            return Err(AppError::new(
                ErrorCode::AuthInvalid,
                "Refresh token does not belong to the user in the access token",
            ));
        }

        // Verify the refresh token hasn't expired
        if refresh_token.expires_at < Utc::now() {
            return Err(AppError::new(
                ErrorCode::AuthExpired,
                "Refresh token has expired",
            ));
        }

        // Verify the refresh token hasn't been revoked
        if refresh_token.revoked {
            return Err(AppError::new(
                ErrorCode::AuthInvalid,
                "Refresh token has been revoked",
            ));
        }

        Ok(refresh_token)
    }
}
