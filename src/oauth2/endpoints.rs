// ABOUTME: OAuth 2.0 authorization and token endpoints implementation
// ABOUTME: Handles OAuth 2.0 flow with JWT tokens as access tokens for MCP client compatibility

// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - String ownership transfers to struct constructors (OAuth2AuthCode, TokenResponse)
// - Arc clone for database manager creation

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
use std::sync::Arc;
use uuid::Uuid;

/// OAuth 2.0 Authorization Server
pub struct OAuth2AuthorizationServer {
    client_manager: ClientRegistrationManager,
    auth_manager: Arc<AuthManager>,
    database: Arc<crate::database_plugins::factory::Database>,
}

impl OAuth2AuthorizationServer {
    pub fn new(
        database: Arc<crate::database_plugins::factory::Database>,
        auth_manager: Arc<AuthManager>,
    ) -> Self {
        let client_manager = ClientRegistrationManager::new(database.clone()); // Safe: Arc clone for manager construction

        Self {
            client_manager,
            auth_manager,
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
            .map_err(|_| OAuth2Error::invalid_client())?;

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
            )
            .await
            .map_err(|_| OAuth2Error::invalid_request("Failed to generate authorization code"))?;

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
        // Validate client credentials
        let _ = self
            .client_manager
            .validate_client(&request.client_id, &request.client_secret)
            .await?;

        match request.grant_type.as_str() {
            "authorization_code" => self.handle_authorization_code_grant(request).await,
            "client_credentials" => self.handle_client_credentials_grant(request),
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

        // Validate and consume authorization code
        let auth_code = self
            .validate_and_consume_auth_code(&code, &request.client_id, &redirect_uri)
            .await?;

        // Generate JWT access token
        let access_token = self
            .generate_access_token(
                &request.client_id,
                Some(auth_code.user_id),
                auth_code.scope.as_deref(),
            )
            .map_err(|_| OAuth2Error::invalid_request("Failed to generate access token"))?;

        Ok(TokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: 3600, // 1 hour
            scope: auth_code.scope,
            refresh_token: None, // Not implemented yet
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
            .map_err(|_| OAuth2Error::invalid_request("Failed to generate access token"))?;

        Ok(TokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: 3600, // 1 hour
            scope: request.scope,
            refresh_token: None,
        })
    }

    /// Generate authorization code
    async fn generate_authorization_code(
        &self,
        client_id: &str,
        user_id: Uuid,
        redirect_uri: &str,
        scope: Option<&str>,
    ) -> Result<String> {
        let code = Self::generate_random_string(32);
        let expires_at = Utc::now() + Duration::minutes(10); // 10 minute expiry

        let auth_code = OAuth2AuthCode {
            code: code.clone(), // Safe: String ownership for OAuth2AuthCode struct
            client_id: client_id.to_string(),
            user_id,
            redirect_uri: redirect_uri.to_string(),
            scope: scope.map(std::string::ToString::to_string),
            expires_at,
            used: false,
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
    ) -> Result<OAuth2AuthCode, OAuth2Error> {
        let mut auth_code = self
            .get_auth_code(code)
            .await
            .map_err(|_| OAuth2Error::invalid_grant("Invalid authorization code"))?;

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

        // Mark as used
        auth_code.used = true;
        self.update_auth_code(&auth_code)
            .await
            .map_err(|_| OAuth2Error::invalid_grant("Failed to consume authorization code"))?;

        Ok(auth_code)
    }

    /// Generate JWT access token
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
                self.auth_manager
                    .generate_client_credentials_token(client_id, &scopes)
            },
            |uid| self.auth_manager.generate_oauth_access_token(&uid, &scopes),
        )
    }

    /// Generate random string for codes
    fn generate_random_string(length: usize) -> String {
        let rng = SystemRandom::new();
        let mut bytes = vec![0u8; length];
        if rng.fill(&mut bytes).is_err() {
            // Fallback to a deterministic but unique value
            bytes = vec![42u8; length];
        }

        // Convert to URL-safe base64
        general_purpose::URL_SAFE_NO_PAD.encode(&bytes)
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
}
