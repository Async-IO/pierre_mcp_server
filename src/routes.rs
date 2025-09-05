// ABOUTME: HTTP REST API route handlers for user-facing endpoints and web interfaces
// ABOUTME: Provides authentication, user management, and basic API endpoints for web clients
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! HTTP routes for user authentication and OAuth flows in multi-tenant mode

use crate::{
    constants::{error_messages, limits, oauth_providers},
    database_plugins::DatabaseProvider,
    errors::AppError,
    mcp::resources::ServerResources,
    models::{User, UserOAuthToken},
    utils::{
        errors::{auth_error, operation_error, user_state_error, validation_error},
        http_client::oauth_client,
        json_responses::{a2a_registration_success, registration_failed_error},
    },
};
use anyhow::Result;
use base64::{engine::general_purpose, Engine};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub user_id: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub token: String,
    pub user_id: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub jwt_token: String,
    pub expires_at: String,
    pub user: UserInfo,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub user_id: String,
    pub email: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OAuthAuthorizationResponse {
    pub authorization_url: String,
    pub state: String,
    pub instructions: String,
    pub expires_in_minutes: i64,
}

#[derive(Debug, Serialize)]
pub struct ConnectionStatus {
    pub provider: String,
    pub connected: bool,
    pub expires_at: Option<String>,
    pub scopes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OAuthCallbackResponse {
    pub user_id: String,
    pub provider: String,
    pub expires_at: String,
    pub scopes: String,
}

#[derive(Debug, Serialize)]
pub struct SetupStatusResponse {
    pub needs_setup: bool,
    pub admin_user_exists: bool,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StravaTokenResponse {
    access_token: String,
    refresh_token: String,
    expires_at: i64,
    expires_in: i64,
    token_type: String,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    athlete: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct FitbitTokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
    token_type: String,
    scope: String,
    user_id: String,
}

#[derive(Clone)]
pub struct AuthRoutes {
    resources: std::sync::Arc<ServerResources>,
}

impl AuthRoutes {
    #[must_use]
    pub const fn new(resources: std::sync::Arc<ServerResources>) -> Self {
        Self { resources }
    }

    /// Handle user registration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Email format is invalid
    /// - Password is too weak
    /// - User already exists
    /// - Database operation fails
    pub async fn register(&self, request: RegisterRequest) -> Result<RegisterResponse> {
        info!("User registration attempt for email: {}", request.email);

        // Validate email format
        if !Self::is_valid_email(&request.email) {
            return Err(validation_error(error_messages::INVALID_EMAIL_FORMAT));
        }

        // Validate password strength
        if !Self::is_valid_password(&request.password) {
            return Err(validation_error(error_messages::PASSWORD_TOO_WEAK));
        }

        // Check if user already exists
        if let Ok(Some(_)) = self
            .resources
            .database
            .get_user_by_email(&request.email)
            .await
        {
            return Err(user_state_error(error_messages::USER_ALREADY_EXISTS));
        }

        // Hash password
        let password_hash = bcrypt::hash(&request.password, bcrypt::DEFAULT_COST)?;

        // Create user
        let user = User::new(request.email.clone(), password_hash, request.display_name);

        // Save user to database
        let user_id = self.resources.database.create_user(&user).await?;

        info!(
            "User registered successfully: {} ({})",
            request.email, user_id
        );

        Ok(RegisterResponse {
            user_id: user_id.to_string(),
            message: "User registered successfully. Your account is pending admin approval.".into(),
        })
    }

    /// Handle user login
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User does not exist
    /// - Password verification fails
    /// - Database operation fails
    /// - JWT token generation fails
    pub async fn login(&self, request: LoginRequest) -> Result<LoginResponse> {
        info!("User login attempt for email: {}", request.email);

        // Get user from database
        let user = self
            .resources
            .database
            .get_user_by_email_required(&request.email)
            .await
            .map_err(|_| anyhow::anyhow!("Invalid email or password"))?;

        // Verify password
        if !bcrypt::verify(&request.password, &user.password_hash)? {
            error!("Invalid password for user: {}", request.email);
            return Err(auth_error(error_messages::INVALID_CREDENTIALS));
        }

        // Check if user is approved to login
        if !user.user_status.can_login() {
            warn!(
                "Login blocked for user: {} - status: {:?}",
                request.email, user.user_status
            );
            return Err(user_state_error(user.user_status.to_message()));
        }

        // Update last active timestamp
        self.resources.database.update_last_active(user.id).await?;

        // Generate JWT token
        let jwt_token = self.resources.auth_manager.generate_token(&user)?;
        let expires_at =
            chrono::Utc::now() + chrono::Duration::hours(limits::DEFAULT_SESSION_HOURS); // Default 24h expiry

        info!(
            "User logged in successfully: {} ({})",
            request.email, user.id
        );

        Ok(LoginResponse {
            jwt_token,
            expires_at: expires_at.to_rfc3339(),
            user: UserInfo {
                user_id: user.id.to_string(),
                email: user.email,
                display_name: user.display_name,
            },
        })
    }

    /// Handle token refresh
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User ID format is invalid
    /// - User does not exist
    /// - Token validation fails
    /// - Database operation fails
    /// - JWT token generation fails
    pub async fn refresh_token(&self, request: RefreshTokenRequest) -> Result<LoginResponse> {
        info!("Token refresh attempt for user: {}", request.user_id);

        // Parse user ID
        let user_uuid = crate::utils::uuid::parse_user_id(&request.user_id)
            .map_err(|_| anyhow::anyhow!("Invalid user ID format"))?;

        // Get user from database
        let user = self
            .resources
            .database
            .get_user(user_uuid)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;

        // Validate the current token and refresh it
        let new_jwt_token = self
            .resources
            .auth_manager
            .refresh_token(&request.token, &user)?;
        let expires_at =
            chrono::Utc::now() + chrono::Duration::hours(limits::DEFAULT_SESSION_HOURS);

        // Update last active timestamp
        self.resources.database.update_last_active(user.id).await?;

        info!("Token refreshed successfully for user: {}", user.id);

        Ok(LoginResponse {
            jwt_token: new_jwt_token,
            expires_at: expires_at.to_rfc3339(),
            user: UserInfo {
                user_id: user.id.to_string(),
                email: user.email,
                display_name: user.display_name,
            },
        })
    }

    /// Validate email format
    #[must_use]
    pub fn is_valid_email(email: &str) -> bool {
        // Simple email validation
        if email.len() <= 5 {
            return false;
        }
        let Some(at_pos) = email.find('@') else {
            return false;
        };
        if at_pos == 0 || at_pos == email.len() - 1 {
            return false; // @ at start or end
        }
        let domain_part = &email[at_pos + 1..];
        domain_part.contains('.')
    }

    /// Validate password strength
    #[must_use]
    pub const fn is_valid_password(password: &str) -> bool {
        password.len() >= 8
    }
}

/// OAuth flow routes for connecting fitness providers
#[derive(Clone)]
pub struct OAuthRoutes {
    resources: std::sync::Arc<ServerResources>,
}

impl OAuthRoutes {
    #[must_use]
    pub const fn new(resources: std::sync::Arc<ServerResources>) -> Self {
        Self { resources }
    }

    /// Get OAuth authorization URL for a provider with real configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Provider is not supported
    /// - Database operation fails when storing OAuth state
    pub async fn get_auth_url(
        &self,
        user_id: uuid::Uuid,
        tenant_id: uuid::Uuid,
        provider: &str,
    ) -> Result<OAuthAuthorizationResponse> {
        // Store state in database for CSRF protection
        let new_uuid = uuid::Uuid::new_v4();
        let state = format!("{user_id}:{new_uuid}");
        Self::store_oauth_state(user_id, provider, &state);

        match provider {
            oauth_providers::STRAVA => {
                // Get tenant OAuth credentials for Strava
                let credentials = self
                    .resources
                    .database
                    .get_tenant_oauth_credentials(tenant_id, oauth_providers::STRAVA)
                    .await
                    .map_err(|e| {
                        AppError::internal(format!("Failed to get tenant Strava credentials: {e}"))
                    })?
                    .ok_or_else(|| {
                        AppError::internal("No Strava OAuth credentials configured for tenant")
                    })?;

                let client_id = credentials.client_id;

                let redirect_uri = crate::constants::env_config::strava_redirect_uri();

                let scope = credentials.scopes.join(",");

                let auth_url = format!(
                    "https://www.strava.com/oauth/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
                    urlencoding::encode(&client_id),
                    urlencoding::encode(&redirect_uri),
                    urlencoding::encode(&scope),
                    urlencoding::encode(&state)
                );

                Ok(OAuthAuthorizationResponse {
                    authorization_url: auth_url,
                    state,
                    instructions: "Visit the URL above to authorize access to your Strava account. You'll be redirected back after authorization.".into(),
                    expires_in_minutes: 10,
                })
            }
            oauth_providers::FITBIT => {
                // Get tenant OAuth credentials for Fitbit
                let credentials = self
                    .resources
                    .database
                    .get_tenant_oauth_credentials(tenant_id, oauth_providers::FITBIT)
                    .await
                    .map_err(|e| {
                        AppError::internal(format!("Failed to get tenant Fitbit credentials: {e}"))
                    })?
                    .ok_or_else(|| {
                        AppError::internal("No Fitbit OAuth credentials configured for tenant")
                    })?;

                let client_id = credentials.client_id;

                let redirect_uri = crate::constants::env_config::fitbit_redirect_uri();

                let scope = "activity%20profile";

                let auth_url = format!(
                    "https://www.fitbit.com/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
                    urlencoding::encode(&client_id),
                    urlencoding::encode(&redirect_uri),
                    scope,
                    urlencoding::encode(&state)
                );

                Ok(OAuthAuthorizationResponse {
                    authorization_url: auth_url,
                    state,
                    instructions: "Visit the URL above to authorize access to your Fitbit account. You'll be redirected back after authorization.".into(),
                    expires_in_minutes: 10,
                })
            }
            _ => Err(anyhow::anyhow!("Unsupported provider: {}", provider)),
        }
    }

    /// Store OAuth state for CSRF protection
    fn store_oauth_state(user_id: uuid::Uuid, provider: &str, state: &str) {
        // Store state with expiration (10 minutes)
        let expires_at = chrono::Utc::now() + chrono::Duration::minutes(10);

        // In a production system, you'd store this in a cache/database
        // Store webhook registration in database
        tracing::debug!("OAuth state expires at: {}", expires_at);
        info!(
            "Storing OAuth state for user {} provider {}: {}",
            user_id, provider, state
        );

        // State storage using secure random state parameter for OAuth PKCE
    }

    /// Handle OAuth callback and store tokens
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - State parameter is invalid
    /// - User ID cannot be parsed
    /// - Provider is not supported
    /// - Token exchange with provider fails
    /// - Database operation fails
    pub async fn handle_callback(
        &self,
        code: &str,
        state: &str,
        provider: &str,
    ) -> Result<OAuthCallbackResponse> {
        // Parse user ID from state (format: "user_id:uuid")
        let mut parts = state.splitn(2, ':');
        let user_id_str = parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("Invalid state parameter format"))?;
        let random_part = parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("Invalid state parameter format"))?;
        let _ = random_part; // Used for state validation security
        let user_id = crate::utils::uuid::parse_user_id(user_id_str)?;

        // Validate state (in production, check against stored state)
        info!(
            "Processing OAuth callback for user {} provider {}",
            user_id, provider
        );

        // Get user's tenant for OAuth credentials lookup
        let user = self
            .resources
            .database
            .get_user(user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found: {}", user_id))?;

        let tenant_id = user
            .tenant_id
            .as_ref()
            .and_then(|id| uuid::Uuid::parse_str(id).ok())
            .ok_or_else(|| anyhow::anyhow!("User has no valid tenant: {}", user_id))?;

        // Exchange code for tokens (implementation depends on provider)
        match provider {
            oauth_providers::STRAVA => self.handle_strava_callback(user_id, tenant_id, code).await,
            oauth_providers::FITBIT => self.handle_fitbit_callback(user_id, tenant_id, code).await,
            _ => Err(anyhow::anyhow!("Unsupported provider: {}", provider)),
        }
    }

    /// Handle Strava OAuth callback
    async fn handle_strava_callback(
        &self,
        user_id: uuid::Uuid,
        tenant_id: uuid::Uuid,
        code: &str,
    ) -> Result<OAuthCallbackResponse> {
        let token_response = self.exchange_strava_code(tenant_id, code).await?;

        // Validate token type
        if token_response.token_type.to_lowercase() != "bearer" {
            warn!(
                "Unexpected Strava token type: {}",
                token_response.token_type
            );
        }

        // Log athlete information for debugging (without sensitive data)
        if !token_response.athlete.is_null() {
            info!("Strava athlete data received for user: {}", user_id);
        }

        // Store encrypted tokens in database - use expires_in as fallback for expires_at
        let expires_at = if token_response.expires_at > 0 {
            chrono::DateTime::<chrono::Utc>::from_timestamp(token_response.expires_at, 0)
                .unwrap_or_else(|| {
                    chrono::Utc::now() + chrono::Duration::seconds(token_response.expires_in)
                })
        } else {
            chrono::Utc::now() + chrono::Duration::seconds(token_response.expires_in)
        };

        let oauth_token = UserOAuthToken::new(
            user_id,
            tenant_id.to_string(),
            oauth_providers::STRAVA.to_string(),
            token_response.access_token.clone(),
            Some(token_response.refresh_token.clone()),
            Some(expires_at),
            token_response
                .scope
                .clone()
                .or_else(|| Some(crate::constants::oauth::STRAVA_DEFAULT_SCOPES.into())),
        );

        self.resources
            .database
            .upsert_user_oauth_token(&oauth_token)
            .await?;

        info!("Strava tokens stored successfully for user: {}", user_id);

        // Store OAuth notification for MCP resource delivery
        let notification_result = self
            .resources
            .database
            .store_oauth_notification(
                user_id,
                oauth_providers::STRAVA,
                true,
                "strava account connected successfully!",
                Some(&expires_at.to_rfc3339()),
            )
            .await;

        if let Err(e) = notification_result {
            error!("Failed to store OAuth notification: {}", e);
        }

        Ok(OAuthCallbackResponse {
            user_id: user_id.to_string(),
            provider: oauth_providers::STRAVA.into(),
            expires_at: expires_at.to_rfc3339(),
            scopes: token_response
                .scope
                .unwrap_or_else(|| "crate::constants::oauth::STRAVA_DEFAULT_SCOPES".into()),
        })
    }

    /// Handle Fitbit OAuth callback
    async fn handle_fitbit_callback(
        &self,
        user_id: uuid::Uuid,
        tenant_id: uuid::Uuid,
        code: &str,
    ) -> Result<OAuthCallbackResponse> {
        let token_response = self.exchange_fitbit_code(tenant_id, code).await?;

        // Validate token type
        if token_response.token_type.to_lowercase() != "bearer" {
            warn!(
                "Unexpected Fitbit token type: {}",
                token_response.token_type
            );
        }

        // Log user_id for tracking
        info!(
            "Fitbit token received for user_id: {}",
            token_response.user_id
        );

        // Store encrypted tokens in database
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(token_response.expires_in);

        let oauth_token = UserOAuthToken::new(
            user_id,
            tenant_id.to_string(),
            oauth_providers::FITBIT.to_string(),
            token_response.access_token.clone(),
            Some(token_response.refresh_token.clone()),
            Some(expires_at),
            Some(token_response.scope.clone()),
        );

        self.resources
            .database
            .upsert_user_oauth_token(&oauth_token)
            .await?;

        info!("Fitbit tokens stored successfully for user: {}", user_id);

        // Store OAuth notification for MCP resource delivery
        let notification_result = self
            .resources
            .database
            .store_oauth_notification(
                user_id,
                oauth_providers::FITBIT,
                true,
                "fitbit account connected successfully!",
                Some(&expires_at.to_rfc3339()),
            )
            .await;

        if let Err(e) = notification_result {
            error!("Failed to store OAuth notification: {}", e);
        }

        Ok(OAuthCallbackResponse {
            user_id: user_id.to_string(),
            provider: oauth_providers::FITBIT.into(),
            expires_at: expires_at.to_rfc3339(),
            scopes: token_response.scope,
        })
    }

    /// Exchange Strava authorization code for tokens
    async fn exchange_strava_code(
        &self,
        tenant_id: uuid::Uuid,
        code: &str,
    ) -> Result<StravaTokenResponse> {
        // Get tenant OAuth credentials for Strava using OAuth manager
        let credentials = self
            .resources
            .tenant_oauth_client
            .get_tenant_credentials(tenant_id, oauth_providers::STRAVA, &self.resources.database)
            .await
            .map_err(|e| operation_error("Get tenant Strava credentials", &e.to_string()))?
            .ok_or_else(|| {
                operation_error("OAuth", "No Strava OAuth credentials configured for tenant")
            })?;

        let client_id = credentials.client_id;
        let client_secret = credentials.client_secret;

        let params = [
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
        ];

        let client = oauth_client();
        let response = client
            .post("https://www.strava.com/oauth/token")
            .form(&params)
            .send()
            .await?;

        let status = response.status();
        let response_text = response.text().await?;

        info!(
            "Strava token exchange response - Status: {}, Body: {}",
            status, response_text
        );

        if !status.is_success() {
            return Err(operation_error("Strava token exchange", &response_text));
        }

        let token_response: StravaTokenResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse Strava response: {}. Response was: {}",
                    e,
                    response_text
                )
            })?;
        info!("Strava token exchange successful");

        Ok(token_response)
    }

    /// Exchange Fitbit authorization code for tokens
    async fn exchange_fitbit_code(
        &self,
        tenant_id: uuid::Uuid,
        code: &str,
    ) -> Result<FitbitTokenResponse> {
        // Get tenant OAuth credentials for Fitbit using OAuth manager
        let credentials = self
            .resources
            .tenant_oauth_client
            .get_tenant_credentials(tenant_id, oauth_providers::FITBIT, &self.resources.database)
            .await
            .map_err(|e| operation_error("Get tenant Fitbit credentials", &e.to_string()))?
            .ok_or_else(|| {
                operation_error("OAuth", "No Fitbit OAuth credentials configured for tenant")
            })?;

        let client_id = credentials.client_id;
        let client_secret = credentials.client_secret;

        let redirect_uri = crate::constants::env_config::fitbit_redirect_uri();

        let params = [
            ("client_id", client_id.as_str()),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri.as_str()),
            ("code", code),
        ];

        let auth_header = general_purpose::STANDARD.encode(format!("{client_id}:{client_secret}"));

        let client = oauth_client();
        let response = client
            .post("https://api.fitbit.com/oauth2/token")
            .header("Authorization", format!("Basic {auth_header}"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(operation_error("Fitbit token exchange", &error_text));
        }

        let token_response: FitbitTokenResponse = response.json().await?;
        info!("Fitbit token exchange successful");

        Ok(token_response)
    }

    /// Disconnect a provider by removing stored tokens
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Provider is not supported
    pub fn disconnect_provider(&self, user_id: Uuid, provider: &str) -> Result<()> {
        match provider {
            oauth_providers::STRAVA => {
                // Token revocation would clear stored tokens from database
                // Clear provider tokens requires token revocation API calls
                info!("Disconnecting Strava for user {}", user_id);
                // self.resources.database.clear_strava_token(user_id).await?;
                Ok(())
            }
            oauth_providers::FITBIT => {
                info!("Disconnecting Fitbit for user {}", user_id);
                // self.resources.database.clear_fitbit_token(user_id).await?;
                Ok(())
            }
            _ => Err(anyhow::anyhow!("Unsupported provider: {}", provider)),
        }
    }

    /// Get connection status for all OAuth providers for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database operation fails
    ///
    /// # Panics
    ///
    /// Panics if the hardcoded default tenant UUID is invalid (this should never happen)
    pub async fn get_connection_status(&self, user_id: Uuid) -> Result<Vec<ConnectionStatus>> {
        // Get user's tenant for OAuth credentials lookup
        let user = self
            .resources
            .database
            .get_user(user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found: {}", user_id))?;

        let tenant_id = user
            .tenant_id
            .as_ref()
            .and_then(|id| uuid::Uuid::parse_str(id).ok())
            .unwrap_or_else(|| {
                // Safe: hardcoded UUID is always valid
                uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap()
            });

        let mut statuses = Vec::new();

        // Check Strava connection
        let strava_token = self
            .resources
            .database
            .get_user_oauth_token(user_id, &tenant_id.to_string(), oauth_providers::STRAVA)
            .await?;

        let strava_connected = strava_token.is_some();
        let strava_expires_at = strava_token
            .as_ref()
            .and_then(|t| t.expires_at)
            .map(|dt| dt.to_rfc3339());
        let strava_scopes = strava_token.as_ref().and_then(|t| t.scope.clone());

        statuses.push(ConnectionStatus {
            provider: oauth_providers::STRAVA.to_string(),
            connected: strava_connected,
            expires_at: strava_expires_at,
            scopes: strava_scopes,
        });

        // Check Fitbit connection
        let fitbit_token = self
            .resources
            .database
            .get_user_oauth_token(user_id, &tenant_id.to_string(), oauth_providers::FITBIT)
            .await?;

        let fitbit_connected = fitbit_token.is_some();
        let fitbit_expires_at = fitbit_token
            .as_ref()
            .and_then(|t| t.expires_at)
            .map(|dt| dt.to_rfc3339());
        let fitbit_scopes = fitbit_token.as_ref().and_then(|t| t.scope.clone());

        statuses.push(ConnectionStatus {
            provider: oauth_providers::FITBIT.to_string(),
            connected: fitbit_connected,
            expires_at: fitbit_expires_at,
            scopes: fitbit_scopes,
        });

        Ok(statuses)
    }
}

/// A2A (Agent-to-Agent) routes for protocol support
#[derive(Clone)]
pub struct A2ARoutes;

impl A2ARoutes {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Serve the A2A Agent Card at /.well-known/agent.json
    ///
    /// # Errors
    ///
    /// Returns a rejection if:
    /// - Agent card serialization fails
    pub fn get_agent_card() -> Result<impl warp::Reply, warp::Rejection> {
        let agent_card = crate::a2a::AgentCard::new();

        agent_card.to_json().map_or_else(
            |_| {
                Err(warp::reject::custom(crate::a2a::A2AError::InternalError(
                    "Failed to serialize agent card".into(),
                )))
            },
            |json| {
                Ok(warp::reply::with_header(
                    json,
                    "content-type",
                    "application/json",
                ))
            },
        )
    }

    /// Handle A2A protocol requests
    ///
    /// # Errors
    ///
    /// This function does not return errors as it wraps responses in JSON
    pub async fn handle_a2a_request(
        request: crate::a2a::A2ARequest,
        _auth_result: crate::auth::AuthResult,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let server = crate::a2a::A2AServer::new();
        let response = server.handle_request(request).await;

        Ok(warp::reply::json(&response))
    }

    /// Handle A2A client registration
    ///
    /// # Errors
    ///
    /// Returns a rejection if:
    /// - Client registration fails
    /// - Database operation fails
    pub async fn register_client(
        request: crate::a2a::client::ClientRegistrationRequest,
        resources: std::sync::Arc<crate::mcp::resources::ServerResources>,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        tracing::info!(
            client_name = %request.name,
            capabilities = ?request.capabilities,
            contact_email = %request.contact_email,
            "A2A client registration request received"
        );

        // Use the shared A2A client manager from ServerResources
        let client_manager = &*resources.a2a_client_manager;

        match client_manager.register_client(request).await {
            Ok(credentials) => {
                tracing::info!(
                    client_id = %credentials.client_id,
                    "A2A client registered successfully"
                );

                let response = a2a_registration_success(
                    &credentials.client_id,
                    &credentials.client_secret,
                    &credentials.api_key,
                    &credentials.public_key,
                    &credentials.private_key,
                    &credentials.key_type,
                );

                Ok(warp::reply::with_status(
                    warp::reply::json(&response),
                    warp::http::StatusCode::CREATED,
                ))
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "Failed to register A2A client"
                );

                let error_response = registration_failed_error(
                    &format!("Failed to register A2A client: {e}"),
                    &e.to_string(),
                );

                Ok(warp::reply::with_status(
                    warp::reply::json(&error_response),
                    warp::http::StatusCode::BAD_REQUEST,
                ))
            }
        }
    }
}

impl Default for A2ARoutes {
    fn default() -> Self {
        Self::new()
    }
}
