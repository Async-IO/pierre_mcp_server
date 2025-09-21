// ABOUTME: OAuth 2.0 dynamic client registration implementation (RFC 7591)
// ABOUTME: Handles client registration endpoint for mcp-remote and other OAuth clients

use crate::database_plugins::DatabaseProvider;
use crate::oauth2::models::{
    ClientRegistrationRequest, ClientRegistrationResponse, OAuth2Client, OAuth2Error,
};
use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use chrono::{Duration, Utc};
use ring::digest::{digest, SHA256};
use ring::rand::{SecureRandom, SystemRandom};
use std::sync::Arc;
use uuid::Uuid;

/// OAuth 2.0 Client Registration Manager
pub struct ClientRegistrationManager {
    database: Arc<crate::database_plugins::factory::Database>,
}

impl ClientRegistrationManager {
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
        let client_secret = Self::generate_client_secret();
        let client_secret_hash = Self::hash_client_secret(&client_secret);

        // Set default values
        let grant_types = request.grant_types.unwrap_or_else(|| {
            vec![
                "authorization_code".to_string(),
                "client_credentials".to_string(),
            ]
        });

        let response_types = request
            .response_types
            .unwrap_or_else(|| vec!["code".to_string()]);

        let created_at = Utc::now();
        let expires_at = Some(created_at + Duration::days(365)); // 1 year expiry

        // Create client record
        let client = OAuth2Client {
            id: Uuid::new_v4().to_string(),
            client_id: client_id.clone(),
            client_secret_hash,
            redirect_uris: request.redirect_uris.clone(),
            grant_types: grant_types.clone(),
            response_types: response_types.clone(),
            client_name: request.client_name.clone(),
            client_uri: request.client_uri.clone(),
            scope: request.scope.clone(),
            created_at,
            expires_at,
        };

        // Store in database
        self.store_client(&client)
            .await
            .map_err(|_| OAuth2Error::invalid_request("Failed to store client registration"))?;

        // Return registration response
        Ok(ClientRegistrationResponse {
            client_id,
            client_secret,
            client_id_issued_at: Some(created_at.timestamp()),
            client_secret_expires_at: expires_at.map(|dt| dt.timestamp()),
            redirect_uris: request.redirect_uris,
            grant_types,
            response_types,
            client_name: request.client_name,
            client_uri: request.client_uri,
            scope: request
                .scope
                .or_else(|| Some("fitness:read activities:read profile:read".to_string())),
        })
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

        // Verify client secret
        let provided_hash = Self::hash_client_secret(client_secret);
        tracing::debug!(
            "Provided secret hash: {}, Stored hash: {}",
            provided_hash,
            client.client_secret_hash
        );

        if provided_hash != client.client_secret_hash {
            tracing::warn!("OAuth client {} secret validation failed", client_id);
            return Err(OAuth2Error::invalid_client());
        }

        // Check if client is expired
        if let Some(expires_at) = client.expires_at {
            if Utc::now() > expires_at {
                tracing::warn!("OAuth client {} has expired", client_id);
                return Err(OAuth2Error::invalid_client());
            }
        }

        tracing::info!("OAuth client {} validated successfully", client_id);
        Ok(client)
    }

    /// Get client by `client_id`
    ///
    /// # Errors
    /// Returns an error if client is not found in the database
    pub async fn get_client(&self, client_id: &str) -> Result<OAuth2Client> {
        self.database
            .get_oauth2_client(client_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("OAuth2 client not found"))
    }

    /// Store client in database
    async fn store_client(&self, client: &OAuth2Client) -> Result<()> {
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
        // Basic validation - should be HTTPS or localhost
        uri.starts_with("https://")
            || uri.starts_with("http://localhost")
            || uri.starts_with("http://127.0.0.1")
            || uri.starts_with("urn:ietf:wg:oauth:2.0:oob") // Out-of-band for native apps
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

    /// Generate client secret
    fn generate_client_secret() -> String {
        let rng = SystemRandom::new();
        let mut secret = [0u8; 32];
        if rng.fill(&mut secret).is_err() {
            return "fallback_secret_32_chars_long_xyz".to_string();
        }

        // Base64 encode the secret
        general_purpose::STANDARD.encode(secret)
    }

    /// Hash client secret for storage
    fn hash_client_secret(secret: &str) -> String {
        let hash = digest(&SHA256, secret.as_bytes());
        hex::encode(hash.as_ref())
    }
}
