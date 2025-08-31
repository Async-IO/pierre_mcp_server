// ABOUTME: A2A client registration, management, and lifecycle operations
// ABOUTME: Manages client credentials, usage statistics, and rate limiting for A2A agents
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A2A Client Management
//!
//! Handles registration, management, and monitoring of A2A clients
//! that connect to Pierre for agent-to-agent communication.

use crate::a2a::auth::A2AClient;
use crate::a2a::system_user::A2ASystemUserService;
use crate::constants::tiers;
use crate::crypto::A2AKeyManager;
use crate::database_plugins::DatabaseProvider;
use chrono::Timelike;
use chrono::{DateTime, Datelike, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// A2A Client registration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRegistrationRequest {
    pub name: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub redirect_uris: Vec<String>,
    pub contact_email: String,
}

/// A2A Client credentials response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCredentials {
    pub client_id: String,
    pub client_secret: String,
    pub api_key: String,
    pub public_key: String,  // Ed25519 public key for verification
    pub private_key: String, // Ed25519 private key for signing (client-side only)
    pub key_type: String,    // "ed25519"
}

/// A2A Client usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientUsageStats {
    pub client_id: String,
    pub requests_today: u64,
    pub requests_this_month: u64,
    pub total_requests: u64,
    pub last_request_at: Option<chrono::DateTime<chrono::Utc>>,
    pub rate_limit_tier: String,
}

/// A2A Client rate limit tiers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum A2AClientTier {
    #[default]
    Trial, // 1,000 requests/month, auto-expires in 30 days
    Standard,     // 10,000 requests/month
    Professional, // 100,000 requests/month
    Enterprise,   // Unlimited
}

impl A2AClientTier {
    #[must_use]
    pub const fn monthly_limit(&self) -> Option<u32> {
        match self {
            Self::Trial => Some(1000),
            Self::Standard => Some(10000),
            Self::Professional => Some(100_000),
            Self::Enterprise => None, // Unlimited
        }
    }

    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Trial => "Trial",
            Self::Standard => "Standard",
            Self::Professional => "Professional",
            Self::Enterprise => "Enterprise",
        }
    }
}

/// A2A Rate limit status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ARateLimitStatus {
    pub is_rate_limited: bool,
    pub limit: Option<u32>,
    pub remaining: Option<u32>,
    pub reset_at: Option<DateTime<Utc>>,
    pub tier: A2AClientTier,
}

/// A2A Active session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ASession {
    pub id: String,
    pub client_id: String,
    pub user_id: Option<uuid::Uuid>,
    pub granted_scopes: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub requests_count: u64,
}

/// Parameters for detailed A2A usage recording
#[derive(Debug, Clone)]
pub struct A2AUsageParams {
    pub client_id: String,
    pub session_token: Option<String>,
    pub tool_name: String,
    pub response_time_ms: Option<u32>,
    pub status_code: u16,
    pub error_message: Option<String>,
    pub request_size_bytes: Option<u32>,
    pub response_size_bytes: Option<u32>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub client_capabilities: Vec<String>,
    pub granted_scopes: Vec<String>,
}

/// A2A Client Manager
pub struct A2AClientManager {
    database: Arc<crate::database_plugins::factory::Database>,
    system_user_service: Arc<A2ASystemUserService>,
    active_sessions: Arc<tokio::sync::RwLock<HashMap<String, A2ASession>>>,
}

impl A2AClientManager {
    #[must_use]
    pub fn new(
        database: Arc<crate::database_plugins::factory::Database>,
        system_user_service: Arc<A2ASystemUserService>,
    ) -> Self {
        Self {
            database,
            system_user_service,
            active_sessions: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Register a new A2A client
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Registration request validation fails
    /// - Keypair generation fails  
    /// - System user creation fails
    /// - Database storage fails
    #[allow(clippy::cast_possible_truncation)] // Safe: HOUR_SECONDS is 3600, well within u32 range
    pub async fn register_client(
        &self,
        request: ClientRegistrationRequest,
    ) -> Result<ClientCredentials, crate::a2a::A2AError> {
        // Validate registration request
        Self::validate_registration_request(&request)?;

        // Generate client credentials
        let client_id = format!("a2a_client_{}", Uuid::new_v4());
        let client_secret = format!("a2a_secret_{}", Uuid::new_v4());
        let api_key = format!("a2a_{}", uuid::Uuid::new_v4());

        // Generate Ed25519 keypair for the client
        let keypair = A2AKeyManager::generate_keypair().map_err(|e| {
            crate::a2a::A2AError::InternalError(format!("Failed to generate keypair: {e}"))
        })?;

        // Create proper system user (not dummy user)
        let system_user_id = self
            .system_user_service
            .create_or_get_system_user(&client_id, &request.contact_email)
            .await
            .map_err(|e| {
                crate::a2a::A2AError::InternalError(format!("Failed to create system user: {e}"))
            })?;

        // Create client record with real public key
        let client = A2AClient {
            id: client_id.clone(),
            user_id: uuid::Uuid::new_v4(), // Generate consistent user ID for this A2A client
            name: request.name.clone(),
            description: request.description.clone(),
            public_key: keypair.public_key.clone(),
            capabilities: request.capabilities.clone(),
            redirect_uris: request.redirect_uris.clone(),
            is_active: true,
            created_at: chrono::Utc::now(),
            permissions: vec!["read_activities".into()], // Default permissions
            rate_limit_requests: crate::constants::rate_limits::DEFAULT_BURST_LIMIT * 10,
            rate_limit_window_seconds: crate::constants::time::HOUR_SECONDS as u32,
            updated_at: chrono::Utc::now(),
        };

        // Store client in database
        self.store_client_secure(&client, &client_secret, &api_key, system_user_id)
            .await?;

        tracing::info!(
            client_id = %client_id,
            contact_email = %request.contact_email,
            capabilities = ?request.capabilities,
            "A2A client registered successfully"
        );

        Ok(ClientCredentials {
            client_id,
            client_secret,
            api_key,
            public_key: keypair.public_key,
            private_key: keypair.private_key,
            key_type: "ed25519".into(),
        })
    }

    /// Validate client registration request
    fn validate_registration_request(
        request: &ClientRegistrationRequest,
    ) -> Result<(), crate::a2a::A2AError> {
        if request.name.is_empty() {
            return Err(crate::a2a::A2AError::InvalidRequest(
                "Client name is required".into(),
            ));
        }

        if request.capabilities.is_empty() {
            return Err(crate::a2a::A2AError::InvalidRequest(
                "At least one capability is required".into(),
            ));
        }

        // Validate capabilities are known
        let valid_capabilities = [
            "fitness-data-analysis",
            "activity-intelligence",
            "goal-management",
            "performance-prediction",
            "training-analytics",
            "provider-integration",
        ];

        for capability in &request.capabilities {
            if !valid_capabilities.contains(&capability.as_str()) {
                return Err(crate::a2a::A2AError::InvalidRequest(format!(
                    "Unknown capability: {capability}"
                )));
            }
        }

        Ok(())
    }

    /// Store client in database with proper security
    async fn store_client_secure(
        &self,
        client: &A2AClient,
        client_secret: &str,
        api_key_for_link: &str,
        system_user_id: Uuid,
    ) -> Result<(), crate::a2a::A2AError> {
        // Create API key using the proper system user
        let api_key_manager = crate::api_keys::ApiKeyManager::new();

        let request = crate::api_keys::CreateApiKeyRequest {
            name: format!("A2A Client: {}", client.name),
            description: Some(format!("API key for A2A client: {}", client.description)),
            tier: crate::api_keys::ApiKeyTier::Professional, // Default tier for A2A clients
            rate_limit_requests: None,                       // Use tier default
            expires_in_days: None,                           // No expiration
        };

        let (api_key_obj, generated_key) = api_key_manager
            .create_api_key(system_user_id, request)
            .map_err(|e| {
                crate::a2a::A2AError::InternalError(format!("Failed to create API key: {e}"))
            })?;

        // Store the API key in database
        self.database
            .create_api_key(&api_key_obj)
            .await
            .map_err(|e| {
                crate::a2a::A2AError::InternalError(format!("Failed to store API key: {e}"))
            })?;

        // Log the generated API key for audit purposes
        tracing::debug!(
            "Generated API key: {} (hidden for security)",
            if generated_key.len() > 8 {
                &generated_key[..8]
            } else {
                "[too_short]"
            }
        );

        // Create A2A client entry linked to the API key
        self.database
            .create_a2a_client(client, client_secret, &api_key_obj.id)
            .await
            .map_err(|e| {
                crate::a2a::A2AError::InternalError(format!("Failed to create A2A client: {e}"))
            })?;

        tracing::info!(
            client_id = %client.id,
            client_name = %client.name,
            system_user_id = %system_user_id,
            api_key_link = %api_key_for_link,
            "A2A client stored securely in database"
        );
        Ok(())
    }

    /// Get client by ID
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn get_client(
        &self,
        client_id: &str,
    ) -> Result<Option<A2AClient>, crate::a2a::A2AError> {
        self.database
            .get_a2a_client(client_id)
            .await
            .map_err(crate::a2a::map_db_error("Failed to get A2A client"))
    }

    /// List all registered clients for a specific user
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn list_clients_for_user(
        &self,
        user_id: &uuid::Uuid,
    ) -> Result<Vec<A2AClient>, crate::a2a::A2AError> {
        self.database
            .list_a2a_clients(user_id)
            .await
            .map_err(crate::a2a::map_db_error("Failed to list A2A clients"))
    }

    /// List all registered clients (system-wide - admin only)
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn list_all_clients(&self) -> Result<Vec<A2AClient>, crate::a2a::A2AError> {
        // For system-wide listing, we use nil UUID to get all clients
        let system_user_id = uuid::Uuid::nil();
        self.database
            .list_a2a_clients(&system_user_id)
            .await
            .map_err(crate::a2a::map_db_error("Failed to list all A2A clients"))
    }

    /// Legacy method for backwards compatibility - lists all clients
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn list_clients(&self) -> Result<Vec<A2AClient>, crate::a2a::A2AError> {
        self.list_all_clients().await
    }

    /// Deactivate a client
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client does not exist
    /// - Database deactivation fails
    pub async fn deactivate_client(&self, client_id: &str) -> Result<(), crate::a2a::A2AError> {
        // First verify the client exists
        self.get_client(client_id)
            .await?
            .ok_or_else(|| crate::a2a::A2AError::ClientNotRegistered(client_id.to_string()))?;

        // Deactivate the client in the database
        self.database
            .deactivate_a2a_client(client_id)
            .await
            .map_err(crate::a2a::map_db_error("Failed to deactivate A2A client"))?;

        // Invalidate all active sessions for this client
        if let Err(e) = self
            .database
            .invalidate_a2a_client_sessions(client_id)
            .await
        {
            tracing::error!(
                "Failed to invalidate sessions for client {}: {}",
                client_id,
                e
            );
            // Continue with deactivation even if session invalidation fails
        }

        // Deactivate associated API keys - this is critical for security
        if let Err(e) = self.database.deactivate_client_api_keys(client_id).await {
            tracing::error!(
                "Failed to deactivate API keys for client {}: {}",
                client_id,
                e
            );
            // Continue with deactivation even if API key deactivation fails
        }

        Ok(())
    }

    /// Get client usage statistics
    ///
    /// # Errors
    ///
    /// Returns an error if database queries fail
    ///
    /// # Panics
    ///
    /// Panics if time manipulation operations fail (should not happen in practice)
    pub async fn get_client_usage(
        &self,
        client_id: &str,
    ) -> Result<ClientUsageStats, crate::a2a::A2AError> {
        // Get current month usage
        let requests_this_month = u64::from(
            self.database
                .get_a2a_client_current_usage(client_id)
                .await
                .map_err(crate::a2a::map_db_error("Failed to get current usage"))?,
        );

        // Get today's usage
        let start_of_day = chrono::Utc::now()
            .with_hour(0)
            .ok_or_else(|| {
                crate::a2a::A2AError::InternalError("Failed to set hour to 0".to_string())
            })?
            .with_minute(0)
            .ok_or_else(|| {
                crate::a2a::A2AError::InternalError("Failed to set minute to 0".to_string())
            })?
            .with_second(0)
            .ok_or_else(|| {
                crate::a2a::A2AError::InternalError("Failed to set second to 0".to_string())
            })?;
        let end_of_day = chrono::Utc::now();

        let today_stats = self
            .database
            .get_a2a_usage_stats(client_id, start_of_day, end_of_day)
            .await
            .map_err(|e| {
                crate::a2a::A2AError::InternalError(format!("Failed to get today's stats: {e}"))
            })?;

        // Get last request from recent usage history
        let recent_usage = self
            .database
            .get_a2a_client_usage_history(client_id, 1)
            .await
            .map_err(|e| {
                crate::a2a::A2AError::InternalError(format!("Failed to get recent usage: {e}"))
            })?;

        let last_request_at = recent_usage.first().map(|usage| usage.0);

        // Get total requests (use a long period to approximate total)
        let total_start = chrono::Utc::now() - chrono::Duration::days(365);
        let total_stats = self
            .database
            .get_a2a_usage_stats(client_id, total_start, chrono::Utc::now())
            .await
            .map_err(|e| {
                crate::a2a::A2AError::InternalError(format!("Failed to get total stats: {e}"))
            })?;

        Ok(ClientUsageStats {
            client_id: client_id.to_string(),
            requests_today: u64::from(today_stats.total_requests),
            requests_this_month,
            total_requests: u64::from(total_stats.total_requests),
            last_request_at,
            rate_limit_tier: tiers::PROFESSIONAL.into(), // Default for A2A clients
        })
    }

    /// Create a new session for a client
    ///
    /// # Errors
    ///
    /// Returns an error if session creation in database fails
    pub async fn create_session(
        &self,
        client_id: &str,
        user_id: Option<&str>,
    ) -> Result<String, crate::a2a::A2AError> {
        let user_uuid = user_id.and_then(|id| uuid::Uuid::parse_str(id).ok());
        let granted_scopes = vec!["fitness:read".into(), "analytics:read".into()];

        let session_token = self
            .database
            .create_a2a_session(client_id, user_uuid.as_ref(), &granted_scopes, 24)
            .await
            .map_err(|e| {
                crate::a2a::A2AError::InternalError(format!("Failed to create A2A session: {e}"))
            })?;

        // Cache the session for quick access
        let session = A2ASession {
            id: session_token.clone(),
            client_id: client_id.to_string(),
            user_id: user_uuid,
            granted_scopes,
            created_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + chrono::Duration::hours(24),
            last_activity: chrono::Utc::now(),
            requests_count: 0,
        };

        self.active_sessions
            .write()
            .await
            .insert(session_token.clone(), session);

        Ok(session_token)
    }

    /// Update session activity
    ///
    /// # Errors
    ///
    /// Returns an error if database update fails
    pub async fn update_session_activity(
        &self,
        session_token: &str,
    ) -> Result<(), crate::a2a::A2AError> {
        self.database
            .update_a2a_session_activity(session_token)
            .await
            .map_err(|e| {
                crate::a2a::A2AError::InternalError(format!(
                    "Failed to update session activity: {e}"
                ))
            })
    }

    /// Get active sessions for a client
    pub async fn get_active_sessions(&self, client_id: &str) -> Vec<A2ASession> {
        // Check cache for active sessions
        let cached_sessions = {
            let sessions = self.active_sessions.read().await;
            sessions
                .values()
                .filter(|session| {
                    session.client_id == client_id && session.expires_at > chrono::Utc::now()
                })
                .cloned()
                .collect::<Vec<A2ASession>>()
        };

        if !cached_sessions.is_empty() {
            return cached_sessions;
        }

        // Query database for active sessions if cache is empty
        match self.database.get_active_a2a_sessions(client_id).await {
            Ok(db_sessions) => {
                // Update cache with sessions from database
                {
                    let mut cache = self.active_sessions.write().await;
                    for session in &db_sessions {
                        cache.insert(session.id.clone(), session.clone());
                    }
                }
                db_sessions
            }
            Err(e) => {
                tracing::error!("Failed to query active sessions from database: {e}");
                vec![]
            }
        }
    }

    /// Clean up expired sessions
    pub const fn cleanup_expired_sessions(&self) {
        // With database storage, expired sessions are automatically filtered out
        // This could trigger a cleanup job if needed
    }

    /// Record API usage for a client
    ///
    /// # Errors
    ///
    /// Returns an error if usage recording fails
    pub async fn record_usage(
        &self,
        client_id: &str,
        method: &str,
        success: bool,
    ) -> Result<(), crate::a2a::A2AError> {
        let params = A2AUsageParams {
            client_id: client_id.to_string(),
            session_token: None,
            tool_name: method.to_string(),
            response_time_ms: None,
            status_code: if success { 200 } else { 500 },
            error_message: None,
            request_size_bytes: None,
            response_size_bytes: None,
            ip_address: None,
            user_agent: None,
            client_capabilities: vec![],
            granted_scopes: vec![],
        };
        self.record_detailed_usage(params).await
    }

    /// Record detailed A2A usage for tracking and analytics
    ///
    /// # Errors
    ///
    /// Returns an error if database storage fails
    pub async fn record_detailed_usage(
        &self,
        params: A2AUsageParams,
    ) -> Result<(), crate::a2a::A2AError> {
        let usage = crate::database::A2AUsage {
            id: None,
            client_id: params.client_id.clone(),
            session_token: params.session_token,
            timestamp: chrono::Utc::now(),
            tool_name: params.tool_name.clone(),
            response_time_ms: params.response_time_ms,
            status_code: params.status_code,
            error_message: params.error_message,
            request_size_bytes: params.request_size_bytes,
            response_size_bytes: params.response_size_bytes,
            ip_address: params.ip_address,
            user_agent: params.user_agent,
            protocol_version: "1.0".into(),
            client_capabilities: params.client_capabilities,
            granted_scopes: params.granted_scopes,
        };

        self.database.record_a2a_usage(&usage).await.map_err(|e| {
            crate::a2a::A2AError::InternalError(format!("Failed to record A2A usage: {e}"))
        })?;

        tracing::debug!(
            "A2A usage recorded - Client: {}, Tool: {}, Status: {}",
            params.client_id,
            params.tool_name,
            params.status_code
        );
        Ok(())
    }

    /// Calculate rate limit status for a client
    ///
    /// # Errors
    ///
    /// Returns an error if database queries fail
    pub async fn calculate_rate_limit_status(
        &self,
        client_id: &str,
        tier: A2AClientTier,
    ) -> Result<A2ARateLimitStatus, crate::a2a::A2AError> {
        if tier == A2AClientTier::Enterprise {
            Ok(A2ARateLimitStatus {
                is_rate_limited: false,
                limit: None,
                remaining: None,
                reset_at: None,
                tier,
            })
        } else {
            let current_usage = self
                .database
                .get_a2a_client_current_usage(client_id)
                .await
                .map_err(|e| {
                    crate::a2a::A2AError::InternalError(format!("Failed to get current usage: {e}"))
                })?;

            let limit = tier.monthly_limit().unwrap_or(0);
            let remaining = limit.saturating_sub(current_usage);
            let is_rate_limited = current_usage >= limit;

            // Calculate reset time (beginning of next month)
            let reset_at = Self::calculate_next_month_start().map_err(|e| {
                crate::a2a::A2AError::InternalError(format!("Failed to calculate reset time: {e}"))
            })?;

            Ok(A2ARateLimitStatus {
                is_rate_limited,
                limit: Some(limit),
                remaining: Some(remaining),
                reset_at: Some(reset_at),
                tier,
            })
        }
    }

    /// Check if a client is rate limited
    ///
    /// # Errors
    ///
    /// Returns an error if rate limit calculation fails
    pub async fn is_client_rate_limited(
        &self,
        client_id: &str,
        tier: A2AClientTier,
    ) -> Result<bool, crate::a2a::A2AError> {
        let status = self.calculate_rate_limit_status(client_id, tier).await?;
        Ok(status.is_rate_limited)
    }

    /// Get rate limit status for a client by ID
    ///
    /// # Errors
    ///
    /// Returns an error if rate limit calculation fails
    pub async fn get_client_rate_limit_status(
        &self,
        client_id: &str,
    ) -> Result<A2ARateLimitStatus, crate::a2a::A2AError> {
        // Default to trial tier - tier information stored in database
        let tier = A2AClientTier::Trial;
        self.calculate_rate_limit_status(client_id, tier).await
    }

    /// Calculate the start of next month for rate limit reset
    fn calculate_next_month_start() -> Result<DateTime<Utc>, anyhow::Error> {
        let now = Utc::now();

        // Use chrono's built-in date construction to avoid edge cases
        let next_month_start = if now.month() == 12 {
            Utc.with_ymd_and_hms(now.year() + 1, 1, 1, 0, 0, 0)
        } else {
            Utc.with_ymd_and_hms(now.year(), now.month() + 1, 1, 0, 0, 0)
        };

        next_month_start
            .single()
            .ok_or_else(|| anyhow::anyhow!("Failed to create valid date for next month start"))
    }

    /// Get client credentials for authentication
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn get_client_credentials(
        &self,
        client_id: &str,
    ) -> Result<Option<ClientCredentials>, crate::a2a::A2AError> {
        // Fetch credentials from database
        let creds = self
            .database
            .get_a2a_client_credentials(client_id)
            .await
            .map_err(|e| crate::a2a::A2AError::InternalError(format!("Database error: {e}")))?;

        if let Some((id, secret)) = creds {
            // Get the actual public key from the client record
            let client = self.get_client(&id).await?;
            let public_key = client.map_or_else(
                || {
                    tracing::warn!("Could not retrieve public key for client {id}");
                    String::new()
                },
                |c| c.public_key,
            );

            let credentials = ClientCredentials {
                client_id: id.to_string(),
                client_secret: secret,
                api_key: format!("a2a_{client_id}"),
                public_key,
                private_key: String::new(), // Never expose private keys
                key_type: "ed25519".into(),
            };

            Ok(Some(credentials))
        } else {
            Ok(None)
        }
    }
}
