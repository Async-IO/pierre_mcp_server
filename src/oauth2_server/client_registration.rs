// ABOUTME: OAuth 2.0 dynamic client registration implementation (RFC 7591)
// ABOUTME: Handles client registration endpoint for MCP clients and other OAuth clients
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::models::{
    ClientRegistrationRequest, ClientRegistrationResponse, OAuth2Client, OAuth2Error,
};
use crate::database_plugins::DatabaseProvider;
use crate::errors::{AppError, AppResult};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use base64::{engine::general_purpose, Engine as _};
use chrono::{Duration, Utc};
use ring::rand::{SecureRandom, SystemRandom};
use std::sync::Arc;
use uuid::Uuid;

/// OAuth 2.0 Client Registration Manager
pub struct ClientRegistrationManager {
    database: Arc<crate::database_plugins::factory::Database>,
}

impl ClientRegistrationManager {
    /// Creates a new client registration manager
    #[must_use]
    pub const fn new(database: Arc<crate::database_plugins::factory::Database>) -> Self {
        Self { database }
    }

    /// Register a new OAuth 2.0 client (RFC 7591)
    ///
    /// # Errors
    /// Returns an error if client registration validation fails or database storage fails
    pub async fn register_client(
        &self,
        request: ClientRegistrationRequest,
    ) -> Result<ClientRegistrationResponse, OAuth2Error> {
        // Validate request
        Self::validate_registration_request(&request)?;

        // Generate client credentials
        let client_id = Self::generate_client_id();
        let client_secret = Self::generate_client_secret()?;
        let client_secret_hash = Self::hash_client_secret(&client_secret)?;

        // Set default values - only authorization_code by default for security (RFC 8252 best practices)
        // Clients must explicitly request client_credentials if needed
        let grant_types = request
            .grant_types
            .unwrap_or_else(|| vec!["authorization_code".to_owned()]);

        let response_types = request
            .response_types
            .unwrap_or_else(|| vec!["code".to_owned()]);

        let created_at = Utc::now();
        let expires_at = Some(created_at + Duration::days(365)); // 1 year expiry

        // Create client record
        let client = OAuth2Client {
            id: Uuid::new_v4().to_string(),
            client_id: client_id.clone(), // Safe: String ownership for OAuth client struct
            client_secret_hash,
            redirect_uris: request.redirect_uris.clone(), // Safe: Vec ownership for OAuth client struct
            grant_types: grant_types.clone(),             // Safe: Vec ownership for OAuth client
            response_types: response_types.clone(),       // Safe: Vec ownership for OAuth client
            client_name: request.client_name.clone(),     // Safe: String ownership for OAuth client
            client_uri: request.client_uri.clone(), // Safe: Option<String> ownership for OAuth client
            scope: request.scope.clone(), // Safe: Option<String> ownership for OAuth client
            created_at,
            expires_at,
        };

        // Store in database
        self.store_client(&client)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, client_id = %client_id, "Failed to store OAuth2 client registration in database");
                OAuth2Error::invalid_request("Failed to store client registration")
            })?;

        // Return registration response
        // Build default client_uri from actual server configuration (if initialized)
        // Falls back to localhost:8081 for test environments
        let default_client_uri = Self::get_default_client_uri();

        Ok(ClientRegistrationResponse {
            client_id,
            client_secret,
            client_id_issued_at: Some(created_at.timestamp()),
            client_secret_expires_at: expires_at.map(|dt| dt.timestamp()),
            redirect_uris: request.redirect_uris,
            grant_types,
            response_types,
            client_name: request.client_name,
            // RFC 7591: client_uri is OPTIONAL but Claude Code requires it to be non-null
            // Provide actual server URL when not specified by the client
            client_uri: request.client_uri.or(Some(default_client_uri)),
            scope: request
                .scope
                .or_else(|| Some("fitness:read activities:read profile:read".to_owned())),
        })
    }

    /// Verify client secret using Argon2 password hash
    fn verify_client_secret(
        client_id: &str,
        client_secret: &str,
        client_secret_hash: &str,
    ) -> Result<(), OAuth2Error> {
        let parsed_hash = PasswordHash::new(client_secret_hash).map_err(|e| {
            tracing::error!("Failed to parse stored password hash: {}", e);
            OAuth2Error::invalid_client()
        })?;

        let argon2 = Argon2::default();
        if argon2
            .verify_password(client_secret.as_bytes(), &parsed_hash)
            .is_err()
        {
            tracing::warn!("OAuth client {} secret validation failed", client_id);
            return Err(OAuth2Error::invalid_client());
        }

        Ok(())
    }

    /// Check if client is expired
    fn check_client_expiry(
        client_id: &str,
        expires_at: Option<chrono::DateTime<Utc>>,
    ) -> Result<(), OAuth2Error> {
        if let Some(expires_at) = expires_at {
            if Utc::now() > expires_at {
                tracing::warn!("OAuth client {} has expired", client_id);
                return Err(OAuth2Error::invalid_client());
            }
        }
        Ok(())
    }

    /// Validate client credentials
    ///
    /// # Errors
    /// Returns an error if client is not found, credentials are invalid, or client is expired
    pub async fn validate_client(
        &self,
        client_id: &str,
        client_secret: &str,
    ) -> Result<OAuth2Client, OAuth2Error> {
        tracing::debug!("Validating OAuth client: {}", client_id);

        let client = self.get_client(client_id).await.map_err(|e| {
            tracing::warn!("OAuth client {} not found: {}", client_id, e);
            OAuth2Error::invalid_client()
        })?;

        tracing::debug!("OAuth client {} found, validating secret", client_id);

        // Verify client secret using constant-time comparison via Argon2
        Self::verify_client_secret(client_id, client_secret, &client.client_secret_hash)?;

        // Check if client is expired
        Self::check_client_expiry(client_id, client.expires_at)?;

        tracing::info!("OAuth client {} validated successfully", client_id);
        Ok(client)
    }

    /// Get client by `client_id`
    ///
    /// # Errors
    /// Returns an error if client is not found in the database
    pub async fn get_client(&self, client_id: &str) -> AppResult<OAuth2Client> {
        self.database
            .get_oauth2_client(client_id)
            .await?
            .ok_or_else(|| AppError::not_found("OAuth2 client not found"))
    }

    /// Store client in database
    async fn store_client(&self, client: &OAuth2Client) -> AppResult<()> {
        self.database.store_oauth2_client(client).await
    }

    /// Validate registration request
    fn validate_registration_request(
        request: &ClientRegistrationRequest,
    ) -> Result<(), OAuth2Error> {
        // Validate redirect URIs
        if request.redirect_uris.is_empty() {
            return Err(OAuth2Error::invalid_request(
                "At least one redirect_uri is required",
            ));
        }

        for uri in &request.redirect_uris {
            if !Self::is_valid_redirect_uri(uri) {
                return Err(OAuth2Error::invalid_request(&format!(
                    "Invalid redirect_uri: {uri}"
                )));
            }
        }

        // Validate grant types
        if let Some(ref grant_types) = request.grant_types {
            for grant_type in grant_types {
                if !Self::is_supported_grant_type(grant_type) {
                    return Err(OAuth2Error::invalid_request(&format!(
                        "Unsupported grant_type: {grant_type}"
                    )));
                }
            }
        }

        // Validate response types
        if let Some(ref response_types) = request.response_types {
            for response_type in response_types {
                if !Self::is_supported_response_type(response_type) {
                    return Err(OAuth2Error::invalid_request(&format!(
                        "Unsupported response_type: {response_type}"
                    )));
                }
            }
        }

        Ok(())
    }

    /// Check if redirect URI is valid
    fn is_valid_redirect_uri(uri: &str) -> bool {
        // OAuth 2.0 Security Best Practices (RFC 6749 Section 3.1.2.2)
        // - MUST be absolute URI
        // - MUST NOT include fragment component
        // - SHOULD use https:// except for localhost/loopback

        if !Self::validate_uri_format(uri) {
            return false;
        }

        // Allow out-of-band URN for native apps (RFC 8252)
        if uri == "urn:ietf:wg:oauth:2.0:oob" {
            return true;
        }

        // Parse and validate HTTP(S) URIs
        Self::validate_http_uri(uri)
    }

    /// Validate basic URI format requirements
    fn validate_uri_format(uri: &str) -> bool {
        // Reject empty or whitespace-only URIs
        if uri.trim().is_empty() {
            return false;
        }

        // Reject URIs with fragments (security risk - RFC 6749 Section 3.1.2)
        if uri.contains('#') {
            tracing::warn!("Rejected redirect_uri with fragment: {}", uri);
            return false;
        }

        // Reject wildcard patterns (subdomain bypass attack prevention)
        if uri.contains('*') {
            tracing::warn!("Rejected redirect_uri with wildcard: {}", uri);
            return false;
        }

        true
    }

    /// Validate HTTP(S) URI scheme and host
    fn validate_http_uri(uri: &str) -> bool {
        let Ok(parsed_uri) = url::Url::parse(uri) else {
            tracing::warn!("Rejected malformed redirect_uri: {}", uri);
            return false;
        };

        let scheme = parsed_uri.scheme();
        let is_localhost = parsed_uri.host_str() == Some("localhost")
            || parsed_uri.host_str() == Some("127.0.0.1");

        if scheme == "https" {
            // HTTPS is always allowed
            return true;
        }

        if scheme == "http" && is_localhost {
            // HTTP only allowed for localhost/loopback
            return true;
        }

        tracing::warn!(
            "Rejected redirect_uri with non-HTTPS scheme for non-localhost: {}",
            uri
        );
        false
    }

    /// Check if grant type is supported
    fn is_supported_grant_type(grant_type: &str) -> bool {
        matches!(
            grant_type,
            "authorization_code" | "client_credentials" | "refresh_token"
        )
    }

    /// Check if response type is supported
    fn is_supported_response_type(response_type: &str) -> bool {
        matches!(response_type, "code")
    }

    /// Generate client ID
    fn generate_client_id() -> String {
        format!("mcp_client_{}", Uuid::new_v4().simple())
    }

    /// Get default `client_uri` for OAuth client registration
    ///
    /// Uses server config if initialized (production), falls back to localhost:8081 (tests)
    fn get_default_client_uri() -> String {
        crate::constants::try_get_server_config().map_or_else(
            || "http://localhost:8081".to_owned(),
            |config| format!("http://{}:{}", config.host, config.http_port),
        )
    }

    /// Generate client secret
    ///
    /// # Errors
    /// Returns an error if the system RNG fails to generate cryptographically secure random bytes
    fn generate_client_secret() -> Result<String, OAuth2Error> {
        let rng = SystemRandom::new();
        let mut secret = [0u8; 32];
        rng.fill(&mut secret).map_err(|e| {
            tracing::error!(error = ?e, "System RNG failure - cannot generate secure client secret (CRITICAL SECURITY ISSUE)");
            OAuth2Error::invalid_request(
                "System RNG failure - cannot generate secure client secret",
            )
        })?;

        // Base64 encode the secret
        Ok(general_purpose::STANDARD.encode(secret))
    }

    /// Hash client secret for storage using Argon2id
    ///
    /// Uses Argon2id with a random salt for secure password hashing.
    /// Argon2id provides resistance against GPU-based attacks and side-channel attacks.
    ///
    /// # Errors
    /// Returns an error if Argon2 password hashing fails
    fn hash_client_secret(secret: &str) -> Result<String, OAuth2Error> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let hash = argon2
            .hash_password(secret.as_bytes(), &salt)
            .map_err(|e| {
                OAuth2Error::invalid_request(&format!("Argon2 password hashing failed: {e}"))
            })?;

        Ok(hash.to_string())
    }
}
