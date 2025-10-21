// ABOUTME: OAuth 2.0 authorization and token endpoints implementation
// ABOUTME: Handles OAuth 2.0 flow with JWT tokens as access tokens for MCP client compatibility
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - String ownership transfers to struct constructors (OAuth2AuthCode, TokenResponse)
// - Arc clone for database manager creation

use crate::admin::jwks::JwksManager;
use crate::auth::AuthManager;
use crate::database_plugins::DatabaseProvider;
use crate::oauth2::client_registration::ClientRegistrationManager;
use crate::oauth2::models::{
    AuthorizeRequest, AuthorizeResponse, OAuth2AuthCode, OAuth2Error, TokenRequest, TokenResponse,
};
use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use chrono::{Duration, Utc};
use ring::rand::{SecureRandom, SystemRandom};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use subtle::ConstantTimeEq;
use uuid::Uuid;

/// OAuth 2.0 Authorization Server
pub struct OAuth2AuthorizationServer {
    client_manager: ClientRegistrationManager,
    auth_manager: Arc<AuthManager>,
    jwks_manager: Arc<JwksManager>,
    database: Arc<crate::database_plugins::factory::Database>,
}

impl OAuth2AuthorizationServer {
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
        user_id: Option<Uuid>, // From authentication
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

        // Generate authorization code
        let auth_code = self
            .generate_authorization_code(
                &request.client_id,
                user_id,
                &request.redirect_uri,
                request.scope.as_deref(),
                request.code_challenge.as_deref(),
                request.code_challenge_method.as_deref(),
            )
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

        let refresh_token = crate::oauth2::models::OAuth2RefreshToken {
            token: refresh_token_value.clone(),   // Safe: Clone for storage
            client_id: request.client_id.clone(), // Safe: Clone for ownership
            user_id: auth_code.user_id,
            scope: auth_code.scope.clone(), // Safe: Clone for storage
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

        // Validate and get existing refresh token
        let old_refresh_token = self
            .validate_and_consume_refresh_token(&refresh_token_value, &request.client_id)
            .await?;

        // Revoke old refresh token (rotation)
        self.revoke_refresh_token(&refresh_token_value)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to revoke old refresh token for client_id={}: {:#}",
                    request.client_id,
                    e
                );
                OAuth2Error::invalid_request("Failed to revoke old refresh token")
            })?;

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

        let new_refresh_token = crate::oauth2::models::OAuth2RefreshToken {
            token: new_refresh_token_value.clone(), // Safe: Clone for storage
            client_id: request.client_id.clone(),   // Safe: Clone for ownership
            user_id: old_refresh_token.user_id,
            scope: old_refresh_token.scope.clone(), // Safe: Clone for storage
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
    async fn generate_authorization_code(
        &self,
        client_id: &str,
        user_id: Uuid,
        redirect_uri: &str,
        scope: Option<&str>,
        code_challenge: Option<&str>,
        code_challenge_method: Option<&str>,
    ) -> Result<String> {
        let code = Self::generate_random_string(32)?;
        let expires_at = Utc::now() + Duration::minutes(10); // 10 minute expiry

        let auth_code = OAuth2AuthCode {
            code: code.clone(), // Safe: String ownership for OAuth2AuthCode struct
            client_id: client_id.to_string(),
            user_id,
            redirect_uri: redirect_uri.to_string(),
            scope: scope.map(std::string::ToString::to_string),
            expires_at,
            used: false,
            code_challenge: code_challenge.map(std::string::ToString::to_string),
            code_challenge_method: code_challenge_method.map(std::string::ToString::to_string),
        };

        self.store_auth_code(&auth_code).await?;
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
        let mut auth_code = self.get_auth_code(code).await.map_err(|e| {
            tracing::error!(
                "Failed to get authorization code for client_id={}: {:#}",
                client_id,
                e
            );
            OAuth2Error::invalid_grant("Invalid authorization code")
        })?;

        // Validate code properties
        if auth_code.client_id != client_id {
            return Err(OAuth2Error::invalid_grant(
                "Code was issued to different client",
            ));
        }

        if auth_code.redirect_uri != redirect_uri {
            return Err(OAuth2Error::invalid_grant("Redirect URI mismatch"));
        }

        if auth_code.used {
            return Err(OAuth2Error::invalid_grant(
                "Authorization code already used",
            ));
        }

        if Utc::now() > auth_code.expires_at {
            return Err(OAuth2Error::invalid_grant("Authorization code expired"));
        }

        // Verify PKCE code_verifier (RFC 7636)
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

        // Mark as used
        auth_code.used = true;
        self.update_auth_code(&auth_code).await.map_err(|e| {
            tracing::error!(
                "Failed to consume authorization code for client_id={}: {:#}",
                auth_code.client_id,
                e
            );
            OAuth2Error::invalid_grant("Failed to consume authorization code")
        })?;

        Ok(auth_code)
    }

    /// Generate JWT access token with RS256 asymmetric signing
    fn generate_access_token(
        &self,
        client_id: &str,
        user_id: Option<Uuid>,
        scope: Option<&str>,
    ) -> Result<String> {
        let scopes = scope
            .map(|s| {
                s.split(' ')
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        user_id.map_or_else(
            || {
                self.auth_manager.generate_client_credentials_token(
                    &self.jwks_manager,
                    client_id,
                    &scopes,
                )
            },
            |uid| {
                self.auth_manager
                    .generate_oauth_access_token(&self.jwks_manager, &uid, &scopes)
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
            anyhow::anyhow!("System RNG failure - server cannot operate securely")
        })?;

        // Convert to URL-safe base64
        Ok(general_purpose::URL_SAFE_NO_PAD.encode(&bytes))
    }

    /// Store authorization code (database operation)
    async fn store_auth_code(&self, auth_code: &OAuth2AuthCode) -> Result<()> {
        self.database.store_oauth2_auth_code(auth_code).await
    }

    /// Get authorization code (database operation)
    async fn get_auth_code(&self, code: &str) -> Result<OAuth2AuthCode> {
        self.database
            .get_oauth2_auth_code(code)
            .await?
            .ok_or_else(|| anyhow::anyhow!("OAuth2 authorization code not found"))
    }

    /// Update authorization code (database operation)
    async fn update_auth_code(&self, auth_code: &OAuth2AuthCode) -> Result<()> {
        self.database.update_oauth2_auth_code(auth_code).await
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
        refresh_token: &crate::oauth2::models::OAuth2RefreshToken,
    ) -> Result<()> {
        self.database
            .store_oauth2_refresh_token(refresh_token)
            .await
    }

    /// Get refresh token (database operation)
    async fn get_refresh_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::oauth2::models::OAuth2RefreshToken>> {
        self.database.get_oauth2_refresh_token(token).await
    }

    /// Revoke refresh token (database operation)
    async fn revoke_refresh_token(&self, token: &str) -> Result<()> {
        self.database.revoke_oauth2_refresh_token(token).await
    }

    /// Validate and consume refresh token
    async fn validate_and_consume_refresh_token(
        &self,
        token: &str,
        client_id: &str,
    ) -> Result<crate::oauth2::models::OAuth2RefreshToken, OAuth2Error> {
        let refresh_token = self
            .get_refresh_token(token)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to get refresh token for client_id={}: {:#}",
                    client_id,
                    e
                );
                OAuth2Error::invalid_grant("Invalid refresh token")
            })?
            .ok_or_else(|| OAuth2Error::invalid_grant("Refresh token not found"))?;

        // Validate token properties
        if refresh_token.client_id != client_id {
            return Err(OAuth2Error::invalid_grant(
                "Refresh token was issued to different client",
            ));
        }

        if refresh_token.revoked {
            return Err(OAuth2Error::invalid_grant("Refresh token has been revoked"));
        }

        if Utc::now() > refresh_token.expires_at {
            return Err(OAuth2Error::invalid_grant("Refresh token expired"));
        }

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
        request: crate::oauth2::models::ValidateRefreshRequest,
    ) -> Result<crate::oauth2::models::ValidateRefreshResponse> {
        // Validate the JWT token
        match self
            .auth_manager
            .validate_token_detailed(access_token, &self.jwks_manager)
        {
            Ok(claims) => self.handle_valid_token_claims(claims).await,
            Err(validation_error) => Ok(Self::handle_token_validation_error(
                validation_error,
                &request,
            )),
        }
    }

    /// Handle valid token claims by checking user existence
    async fn handle_valid_token_claims(
        &self,
        claims: crate::auth::Claims,
    ) -> Result<crate::oauth2::models::ValidateRefreshResponse> {
        use crate::oauth2::models::{ValidateRefreshResponse, ValidationStatus};

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
    fn handle_token_validation_error(
        validation_error: crate::auth::JwtValidationError,
        request: &crate::oauth2::models::ValidateRefreshRequest,
    ) -> crate::oauth2::models::ValidateRefreshResponse {
        use crate::auth::JwtValidationError;

        match validation_error {
            JwtValidationError::TokenExpired { .. } => {
                if request.refresh_token.is_some() {
                    tracing::warn!("Token expired but refresh not yet fully implemented");
                    Self::create_invalid_response("token_expired_refresh_not_implemented")
                } else {
                    Self::create_invalid_response("token_expired")
                }
            }
            JwtValidationError::TokenInvalid { reason } => {
                Self::create_invalid_response(&format!("invalid_signature: {reason}"))
            }
            JwtValidationError::TokenMalformed { details } => {
                Self::create_invalid_response(&format!("malformed_token: {details}"))
            }
        }
    }

    /// Create an invalid token response
    fn create_invalid_response(reason: &str) -> crate::oauth2::models::ValidateRefreshResponse {
        use crate::oauth2::models::{ValidateRefreshResponse, ValidationStatus};

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
}
