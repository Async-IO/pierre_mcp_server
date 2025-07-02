// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # OAuth Manager
//!
//! Central OAuth management for all providers and servers.
//! Handles the complete OAuth flow from authorization to token management.

use super::{CallbackResponse, OAuthError, OAuthProvider, ProviderRegistry, TokenData};
use crate::database_plugins::{factory::Database, DatabaseProvider};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

/// Central OAuth manager
pub struct OAuthManager {
    database: Arc<Database>,
    registry: ProviderRegistry,
    state_storage: Arc<tokio::sync::RwLock<HashMap<String, StateData>>>,
}

/// OAuth state data for CSRF protection
#[derive(Debug, Clone)]
struct StateData {
    user_id: Uuid,
    provider: String,
    created_at: chrono::DateTime<chrono::Utc>,
    expires_at: chrono::DateTime<chrono::Utc>,
}

impl OAuthManager {
    /// Create new OAuth manager
    pub fn new(database: Arc<Database>) -> Self {
        Self {
            database,
            registry: ProviderRegistry::new(),
            state_storage: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Register OAuth provider
    pub fn register_provider(&mut self, provider: Box<dyn OAuthProvider>) {
        info!("Registering OAuth provider: {}", provider.name());
        self.registry.register_provider(provider);
    }

    /// Generate authorization URL for a provider
    pub async fn generate_auth_url(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<super::AuthorizationResponse, OAuthError> {
        let oauth_provider = self
            .registry
            .get_provider(provider)
            .ok_or_else(|| OAuthError::UnsupportedProvider(provider.to_string()))?;

        // Generate secure state parameter
        let state = format!("{}:{}", user_id, uuid::Uuid::new_v4());

        // Store state for verification
        self.store_state(&state, user_id, provider).await?;

        // Generate authorization URL
        oauth_provider.generate_auth_url(user_id, state).await
    }

    /// Handle OAuth callback
    pub async fn handle_callback(
        &self,
        code: &str,
        state: &str,
        provider: &str,
    ) -> Result<CallbackResponse, OAuthError> {
        // Validate state
        let state_data = self.validate_and_consume_state(state).await?;

        if state_data.provider != provider {
            return Err(OAuthError::InvalidState);
        }

        // Get OAuth provider
        let oauth_provider = self
            .registry
            .get_provider(provider)
            .ok_or_else(|| OAuthError::UnsupportedProvider(provider.to_string()))?;

        // Exchange code for tokens
        let token_data = oauth_provider.exchange_code(code, state).await?;

        // Store tokens in database
        self.store_tokens(state_data.user_id, &token_data).await?;

        info!(
            "OAuth callback completed successfully for user {} provider {}",
            state_data.user_id, provider
        );

        Ok(CallbackResponse {
            user_id: state_data.user_id.to_string(),
            provider: provider.to_string(),
            expires_at: token_data.expires_at.to_rfc3339(),
            scopes: token_data.scopes,
            success: true,
            message: format!("{} connected successfully", provider),
        })
    }

    /// Disconnect provider for user
    pub async fn disconnect_provider(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<(), OAuthError> {
        // Get OAuth provider
        let oauth_provider = self
            .registry
            .get_provider(provider)
            .ok_or_else(|| OAuthError::UnsupportedProvider(provider.to_string()))?;

        // Get current token to revoke
        if let Ok(Some(token_data)) = self.get_token_data(user_id, provider).await {
            // Attempt to revoke token with provider
            if let Err(e) = oauth_provider.revoke_token(&token_data.access_token).await {
                warn!("Failed to revoke token with provider {}: {}", provider, e);
                // Continue with local deletion even if revocation fails
            }
        }

        // Remove tokens from database
        self.remove_tokens(user_id, provider).await?;

        info!("Provider {} disconnected for user {}", provider, user_id);
        Ok(())
    }

    /// Refresh token if needed
    pub async fn ensure_valid_token(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<Option<TokenData>, OAuthError> {
        // Get current token
        let token_data = match self.get_token_data(user_id, provider).await? {
            Some(token) => token,
            None => return Ok(None), // No token stored
        };

        // Get OAuth provider
        let oauth_provider = self
            .registry
            .get_provider(provider)
            .ok_or_else(|| OAuthError::UnsupportedProvider(provider.to_string()))?;

        // Check if token is still valid
        if oauth_provider.validate_token(&token_data).await? {
            return Ok(Some(token_data));
        }

        // Token needs refresh
        info!(
            "Refreshing token for user {} provider {}",
            user_id, provider
        );

        let new_token_data = oauth_provider
            .refresh_token(&token_data.refresh_token)
            .await?;

        // Store new tokens
        self.store_tokens(user_id, &new_token_data).await?;

        Ok(Some(new_token_data))
    }

    /// Get connection status for user
    pub async fn get_connection_status(
        &self,
        user_id: Uuid,
    ) -> Result<HashMap<String, bool>, OAuthError> {
        let mut statuses = HashMap::new();

        for provider_name in self.registry.list_providers() {
            let connected = self.get_token_data(user_id, provider_name).await?.is_some();
            statuses.insert(provider_name.to_string(), connected);
        }

        Ok(statuses)
    }

    /// Store OAuth state for CSRF protection
    async fn store_state(
        &self,
        state: &str,
        user_id: Uuid,
        provider: &str,
    ) -> Result<(), OAuthError> {
        let now = chrono::Utc::now();
        let state_data = StateData {
            user_id,
            provider: provider.to_string(),
            created_at: now,
            expires_at: now + chrono::Duration::minutes(10), // 10 minute expiry
        };

        let mut storage = self.state_storage.write().await;
        storage.insert(state.to_string(), state_data);

        // Clean up expired states
        storage.retain(|_, data| data.expires_at > now);

        Ok(())
    }

    /// Validate and consume OAuth state
    async fn validate_and_consume_state(&self, state: &str) -> Result<StateData, OAuthError> {
        let mut storage = self.state_storage.write().await;

        let state_data = storage.remove(state).ok_or(OAuthError::InvalidState)?;

        let now = chrono::Utc::now();

        // Check if state has expired
        if state_data.expires_at < now {
            return Err(OAuthError::InvalidState);
        }

        // Validate state age using created_at field
        let state_age = now - state_data.created_at;
        if state_age > chrono::Duration::minutes(15) {
            tracing::warn!("OAuth state is older than 15 minutes, rejecting for security");
            return Err(OAuthError::InvalidState);
        }

        Ok(state_data)
    }

    /// Store tokens in database
    async fn store_tokens(&self, user_id: Uuid, token_data: &TokenData) -> Result<(), OAuthError> {
        match token_data.provider.as_str() {
            "strava" => {
                self.database
                    .update_strava_token(
                        user_id,
                        &token_data.access_token,
                        &token_data.refresh_token,
                        token_data.expires_at,
                        token_data.scopes.clone(),
                    )
                    .await
                    .map_err(|e| OAuthError::DatabaseError(e.to_string()))?;
            }
            "fitbit" => {
                self.database
                    .update_fitbit_token(
                        user_id,
                        &token_data.access_token,
                        &token_data.refresh_token,
                        token_data.expires_at,
                        token_data.scopes.clone(),
                    )
                    .await
                    .map_err(|e| OAuthError::DatabaseError(e.to_string()))?;
            }
            _ => return Err(OAuthError::UnsupportedProvider(token_data.provider.clone())),
        }

        Ok(())
    }

    /// Remove tokens from database
    async fn remove_tokens(&self, user_id: Uuid, provider: &str) -> Result<(), OAuthError> {
        match provider {
            "strava" => {
                self.database
                    .clear_strava_token(user_id)
                    .await
                    .map_err(|e| OAuthError::DatabaseError(e.to_string()))?;
                Ok(())
            }
            "fitbit" => {
                self.database
                    .clear_fitbit_token(user_id)
                    .await
                    .map_err(|e| OAuthError::DatabaseError(e.to_string()))?;
                Ok(())
            }
            _ => Err(OAuthError::UnsupportedProvider(provider.to_string())),
        }
    }

    /// Get token data from database
    async fn get_token_data(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<Option<TokenData>, OAuthError> {
        match provider {
            "strava" => match self.database.get_strava_token(user_id).await {
                Ok(Some(token)) => Ok(Some(TokenData {
                    access_token: token.access_token,
                    refresh_token: token.refresh_token,
                    expires_at: token.expires_at,
                    scopes: token.scope,
                    provider: "strava".to_string(),
                })),
                Ok(None) => Ok(None),
                Err(e) => Err(OAuthError::DatabaseError(e.to_string())),
            },
            "fitbit" => match self.database.get_fitbit_token(user_id).await {
                Ok(Some(token)) => Ok(Some(TokenData {
                    access_token: token.access_token,
                    refresh_token: token.refresh_token,
                    expires_at: token.expires_at,
                    scopes: token.scope,
                    provider: "fitbit".to_string(),
                })),
                Ok(None) => Ok(None),
                Err(e) => Err(OAuthError::DatabaseError(e.to_string())),
            },
            _ => Err(OAuthError::UnsupportedProvider(provider.to_string())),
        }
    }
}
