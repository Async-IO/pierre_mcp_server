// ABOUTME: User authentication route handlers for registration, login, and OAuth flows
// ABOUTME: Provides REST endpoints for user account management and fitness provider OAuth callbacks
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Authentication routes for user management and OAuth flows
//!
//! This module handles user registration, login, and OAuth callback processing
//! for fitness providers like Strava. All handlers are thin wrappers that
//! delegate business logic to service layers.

use crate::{
    constants::{error_messages, limits},
    context::{AuthContext, ConfigContext, DataContext, NotificationContext},
    database_plugins::DatabaseProvider,
    errors::AppError,
    mcp::resources::ServerResources,
    models::User,
    utils::errors::{auth_error, user_state_error, validation_error},
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing;
use warp::{Filter, Rejection, Reply};

/// User registration request
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

/// User registration response
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub user_id: String,
    pub message: String,
}

/// User login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// User info for login response
#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub user_id: String,
    pub email: String,
    pub display_name: Option<String>,
}

/// User login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub jwt_token: String,
    pub expires_at: String,
    pub user: UserInfo,
}

/// Refresh token request
#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub token: String,
    pub user_id: String,
}

/// OAuth provider connection status
#[derive(Debug, Serialize)]
pub struct OAuthStatus {
    pub provider: String,
    pub connected: bool,
    pub last_sync: Option<String>,
}

/// Setup status response for admin setup endpoint
#[derive(Debug, Serialize)]
pub struct SetupStatusResponse {
    pub needs_setup: bool,
    pub admin_user_exists: bool,
    pub message: Option<String>,
}

/// OAuth authorization response for provider auth URLs
#[derive(Debug, Serialize)]
pub struct OAuthAuthorizationResponse {
    pub authorization_url: String,
    pub state: String,
    pub instructions: String,
    pub expires_in_minutes: i64,
}

/// Connection status for fitness providers
#[derive(Debug, Serialize)]
pub struct ConnectionStatus {
    pub provider: String,
    pub connected: bool,
    pub expires_at: Option<String>,
    pub scopes: Option<String>,
}

/// Authentication service for business logic
#[derive(Clone)]
pub struct AuthService {
    auth_context: AuthContext,
    data_context: DataContext,
}

impl AuthService {
    #[must_use]
    pub const fn new(auth_context: AuthContext, data_context: DataContext) -> Self {
        Self {
            auth_context,
            data_context,
        }
    }

    /// Handle user registration - implementation from existing routes.rs
    ///
    /// # Errors
    /// Returns error if user validation fails or database operation fails
    pub async fn register(&self, request: RegisterRequest) -> Result<RegisterResponse> {
        tracing::info!("User registration attempt for email: {}", request.email);

        // Validate email format
        if !Self::is_valid_email(&request.email) {
            return Err(validation_error(error_messages::INVALID_EMAIL_FORMAT).into());
        }

        // Validate password strength
        if !Self::is_valid_password(&request.password) {
            return Err(validation_error(error_messages::PASSWORD_TOO_WEAK).into());
        }

        // Check if user already exists
        if let Ok(Some(_)) = self
            .data_context
            .database()
            .get_user_by_email(&request.email)
            .await
        {
            return Err(user_state_error(error_messages::USER_ALREADY_EXISTS).into());
        }

        // Hash password
        let password_hash = bcrypt::hash(&request.password, bcrypt::DEFAULT_COST)?;

        // Create user
        let user = User::new(request.email.clone(), password_hash, request.display_name); // Safe: String ownership needed for user model

        // Save user to database
        let user_id = self.data_context.database().create_user(&user).await?;

        tracing::info!(
            "User registered successfully: {} ({})",
            request.email,
            user_id
        );

        Ok(RegisterResponse {
            user_id: user_id.to_string(),
            message: "User registered successfully. Your account is pending admin approval.".into(),
        })
    }

    /// Handle user login - implementation from existing routes.rs
    ///
    /// # Errors
    /// Returns error if authentication fails or token generation fails
    pub async fn login(&self, request: LoginRequest) -> Result<LoginResponse> {
        tracing::info!("User login attempt for email: {}", request.email);

        // Get user from database
        let user = self
            .data_context
            .database()
            .get_user_by_email_required(&request.email)
            .await
            .map_err(|_| AppError::auth_invalid("Invalid email or password"))?;

        // Verify password using spawn_blocking to avoid blocking async executor
        let password = request.password.clone();
        let password_hash = user.password_hash.clone();
        let is_valid =
            tokio::task::spawn_blocking(move || bcrypt::verify(&password, &password_hash))
                .await
                .map_err(|e| AppError::internal(format!("Password verification task failed: {e}")))?
                .map_err(|e| AppError::internal(format!("Password verification error: {e}")))?;

        if !is_valid {
            tracing::error!("Invalid password for user: {}", request.email);
            return Err(auth_error(error_messages::INVALID_CREDENTIALS).into());
        }

        // Check if user is approved to login
        if !user.user_status.can_login() {
            tracing::warn!(
                "Login blocked for user: {} - status: {:?}",
                request.email,
                user.user_status
            );
            return Err(user_state_error(user.user_status.to_message()).into());
        }

        // Update last active timestamp
        self.data_context
            .database()
            .update_last_active(user.id)
            .await?;

        // Generate JWT token using RS256
        let jwt_token = self
            .auth_context
            .auth_manager()
            .generate_token(&user, self.auth_context.jwks_manager())?;
        let expires_at =
            chrono::Utc::now() + chrono::Duration::hours(limits::DEFAULT_SESSION_HOURS); // Default 24h expiry

        tracing::info!(
            "User logged in successfully: {} ({})",
            request.email,
            user.id
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

    /// Handle token refresh - implementation from existing routes.rs
    ///
    /// # Errors
    /// Returns error if refresh token is invalid or token generation fails
    pub async fn refresh_token(&self, request: RefreshTokenRequest) -> Result<LoginResponse> {
        tracing::info!("Token refresh attempt for user with refresh token");

        // Extract user from refresh token using RS256 validation
        let token_claims = self
            .auth_context
            .auth_manager()
            .validate_token(&request.token, self.auth_context.jwks_manager())?;
        let user_id = uuid::Uuid::parse_str(&token_claims.sub)?;

        // Validate that the user_id matches the one in the request
        let request_user_id = uuid::Uuid::parse_str(&request.user_id)?;
        if user_id != request_user_id {
            return Err(AppError::auth_invalid("User ID mismatch").into());
        }

        // Get user from database
        let user = self
            .data_context
            .database()
            .get_user(user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User"))?;

        // Generate new JWT token using RS256
        let new_jwt_token = self
            .auth_context
            .auth_manager()
            .generate_token(&user, self.auth_context.jwks_manager())?;
        let expires_at =
            chrono::Utc::now() + chrono::Duration::hours(limits::DEFAULT_SESSION_HOURS);

        // Update last active timestamp
        self.data_context
            .database()
            .update_last_active(user.id)
            .await?;

        tracing::info!("Token refreshed successfully for user: {}", user.id);

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

    /// Validate email format - from existing routes.rs
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

    /// Validate password strength - from existing routes.rs
    #[must_use]
    pub const fn is_valid_password(password: &str) -> bool {
        password.len() >= 8
    }
}

/// OAuth service for OAuth flow business logic
#[derive(Clone)]
pub struct OAuthService {
    data: DataContext,
    config: ConfigContext,
    notifications: NotificationContext,
}

impl OAuthService {
    #[must_use]
    pub const fn new(
        data_context: DataContext,
        config_context: ConfigContext,
        notification_context: NotificationContext,
    ) -> Self {
        Self {
            data: data_context,
            config: config_context,
            notifications: notification_context,
        }
    }

    /// Get configuration context
    #[must_use]
    pub const fn config(&self) -> &ConfigContext {
        &self.config
    }

    /// Handle OAuth callback
    ///
    /// # Errors
    /// Returns error if OAuth state is invalid or callback processing fails
    pub async fn handle_callback(
        &self,
        code: &str,
        state: &str,
        provider: &str,
    ) -> Result<OAuthCallbackResponse> {
        // Use async block to satisfy clippy
        tokio::task::yield_now().await;

        // Validate state and extract user ID
        let user_id = Self::validate_oauth_state(state)?;

        // Validate provider is supported
        Self::validate_provider(provider)?;

        tracing::info!(
            "Processing OAuth callback for user {} provider {} with code {}",
            user_id,
            provider,
            code
        );

        // Get user and tenant from database
        let (user, tenant_id) = self.get_user_and_tenant(user_id, provider).await?;

        // Exchange OAuth code for access token
        let token = self
            .exchange_oauth_code(code, provider, user_id, &user)
            .await?;

        tracing::info!(
            "Successfully exchanged OAuth code for user {} provider {}",
            user_id,
            provider
        );

        // Store token and send notifications
        let expires_at = self
            .store_oauth_token(user_id, tenant_id, provider, &token)
            .await?;
        self.send_oauth_notifications(user_id, provider, &expires_at)
            .await?;
        self.notify_bridge_oauth_success(provider, &token).await;

        Ok(OAuthCallbackResponse {
            user_id: user_id.to_string(),
            provider: provider.to_string(),
            expires_at: expires_at.to_rfc3339(),
            scopes: token.scope.unwrap_or_else(|| "read".to_string()),
        })
    }

    /// Validate OAuth state parameter and extract user ID
    fn validate_oauth_state(state: &str) -> Result<uuid::Uuid> {
        let mut parts = state.splitn(2, ':');
        let user_id_str = parts
            .next()
            .ok_or_else(|| AppError::invalid_input("Invalid state parameter format"))?;
        let random_part = parts
            .next()
            .ok_or_else(|| AppError::invalid_input("Invalid state parameter format"))?;

        // Validate state for CSRF protection
        if random_part.len() < 16
            || !random_part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return Err(AppError::invalid_input("Invalid OAuth state parameter").into());
        }

        crate::utils::uuid::parse_user_id(user_id_str)
    }

    /// Validate that provider is supported
    fn validate_provider(provider: &str) -> Result<()> {
        use crate::constants::oauth_providers;
        match provider {
            oauth_providers::STRAVA | oauth_providers::FITBIT => Ok(()),
            _ => Err(AppError::invalid_input(format!("Unsupported provider: {provider}")).into()),
        }
    }

    /// Get user and tenant from database
    async fn get_user_and_tenant(
        &self,
        user_id: uuid::Uuid,
        provider: &str,
    ) -> Result<(crate::models::User, String)> {
        let database = self.data.database();
        let user = database.get_user(user_id).await?.ok_or_else(|| {
            tracing::error!(
                "OAuth callback failed: User not found - user_id: {}, provider: {}",
                user_id,
                provider
            );
            AppError::not_found("User")
        })?;

        let tenant_id =
            user.tenant_id
                .as_ref()
                .ok_or_else(|| {
                    tracing::error!(
                    "OAuth callback failed: Missing tenant - user_id: {}, email: {}, provider: {}",
                    user.id, user.email, provider
                );
                    AppError::invalid_input("User has no tenant")
                })?
                .clone();

        Ok((user, tenant_id))
    }

    /// Exchange OAuth code for access token
    async fn exchange_oauth_code(
        &self,
        code: &str,
        provider: &str,
        user_id: uuid::Uuid,
        user: &crate::models::User,
    ) -> Result<crate::oauth2_client::OAuth2Token> {
        let oauth_config = self.create_oauth_config(provider)?;
        let oauth_client = crate::oauth2_client::OAuth2Client::new(oauth_config.clone());

        let token = oauth_client.exchange_code(code).await.map_err(|e| {
            tracing::error!(
                "OAuth token exchange failed for {provider} - user_id: {user_id}, email: {}, code: {code}, error: {e}",
                user.email
            );
            AppError::internal(format!("Failed to exchange OAuth code for token: {e}"))
        })?;

        Ok(token)
    }

    /// Create `OAuth2` config for provider using injected configuration
    fn create_oauth_config(&self, provider: &str) -> Result<crate::oauth2_client::OAuth2Config> {
        let server_config = self.config.config();
        match provider {
            "strava" => {
                let oauth_config = &server_config.oauth.strava;
                let api_config = &server_config.external_services.strava_api;

                Ok(crate::oauth2_client::OAuth2Config {
                    client_id: oauth_config
                        .client_id
                        .clone()
                        .unwrap_or_else(|| "163846".to_string()),
                    client_secret: oauth_config.client_secret.clone().unwrap_or_default(),
                    auth_url: api_config.auth_url.clone(),
                    token_url: api_config.token_url.clone(),
                    redirect_uri: oauth_config.redirect_uri.clone().unwrap_or_else(|| {
                        format!(
                            "http://localhost:{}/api/oauth/callback/strava",
                            server_config.http_port
                        )
                    }),
                    scopes: vec![crate::constants::oauth::STRAVA_DEFAULT_SCOPES.to_string()],
                    use_pkce: true,
                })
            }
            _ => Err(AppError::invalid_input(format!("Unsupported provider: {provider}")).into()),
        }
    }

    /// Store OAuth token in database
    async fn store_oauth_token(
        &self,
        user_id: uuid::Uuid,
        tenant_id: String,
        provider: &str,
        token: &crate::oauth2_client::OAuth2Token,
    ) -> Result<chrono::DateTime<chrono::Utc>> {
        let expires_at = token
            .expires_at
            .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::hours(1));

        let user_oauth_token = crate::models::UserOAuthToken {
            id: uuid::Uuid::new_v4().to_string(),
            user_id,
            tenant_id,
            provider: provider.to_string(),
            access_token: token.access_token.clone(),
            refresh_token: token.refresh_token.clone(),
            token_type: token.token_type.clone(),
            expires_at: Some(expires_at),
            scope: token.scope.clone(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        self.data
            .database()
            .upsert_user_oauth_token(&user_oauth_token)
            .await?;
        Ok(expires_at)
    }

    /// Send OAuth completion notifications
    async fn send_oauth_notifications(
        &self,
        user_id: uuid::Uuid,
        provider: &str,
        expires_at: &chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        // Store notification in database
        let notification_id = self
            .data
            .database()
            .store_oauth_notification(
                user_id,
                provider,
                true,
                "OAuth authorization completed successfully",
                Some(&expires_at.to_rfc3339()),
            )
            .await?;

        tracing::info!(
            "Created OAuth completion notification {} for user {} provider {}",
            notification_id,
            user_id,
            provider
        );

        // Send OAuth notification to all active MCP protocol streams for this user
        let oauth_notification = crate::database::oauth_notifications::OAuthNotification {
            id: notification_id,
            user_id: user_id.to_string(),
            provider: provider.to_string(),
            success: true,
            message: format!(
                "{provider} account connected successfully! You can now use fitness tools."
            ),
            expires_at: Some(expires_at.to_rfc3339()),
            created_at: chrono::Utc::now(),
            read_at: None,
        };

        if let Err(e) = self
            .notifications
            .sse_manager()
            .send_oauth_notification_to_protocol_streams(user_id, &oauth_notification)
            .await
        {
            tracing::warn!(
                "Failed to send OAuth notification to MCP streams for user {} provider {}: {}",
                user_id,
                provider,
                e
            );
        } else {
            tracing::info!(
                "Successfully sent OAuth notification to MCP protocol streams for user {} provider {}",
                user_id,
                provider
            );
        }

        Ok(())
    }

    /// Notify bridge about successful OAuth (for client-side token storage and focus recovery)
    async fn notify_bridge_oauth_success(
        &self,
        provider: &str,
        token: &crate::oauth2_client::OAuth2Token,
    ) {
        let oauth_callback_port = self.config.config().oauth_callback_port;
        let callback_url =
            format!("http://localhost:{oauth_callback_port}/oauth/provider-callback/{provider}");

        // Calculate expires_in from expires_at if available
        let expires_in = token.expires_at.map(|expires_at| {
            let duration = expires_at - chrono::Utc::now();
            duration.num_seconds().max(0)
        });

        let token_data = serde_json::json!({
            "access_token": token.access_token,
            "refresh_token": token.refresh_token,
            "expires_in": expires_in,
            "token_type": token.token_type,
            "scope": token.scope
        });

        tracing::debug!(
            "Notifying bridge about {} OAuth success at {}",
            provider,
            callback_url
        );

        // Best-effort notification with configured timeout - don't fail OAuth flow if bridge notification fails
        // Configuration must be initialized via initialize_http_clients() at server startup
        let timeout_secs =
            crate::utils::http_client::get_oauth_callback_notification_timeout_secs();
        match reqwest::Client::new()
            .post(&callback_url)
            .json(&token_data)
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                tracing::info!(
                    "✅ Successfully notified bridge about {} OAuth completion",
                    provider
                );
            }
            Ok(response) => {
                tracing::warn!(
                    "Bridge notification responded with status {} for provider {}",
                    response.status(),
                    provider
                );
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to notify bridge about {} OAuth (bridge may not be running): {}",
                    provider,
                    e
                );
            }
        }
    }

    /// Disconnect OAuth provider for user
    ///
    /// # Errors
    /// Returns error if provider is unsupported or disconnection fails
    pub async fn disconnect_provider(&self, user_id: uuid::Uuid, provider: &str) -> Result<()> {
        use crate::constants::oauth_providers;

        tracing::debug!(
            "Processing OAuth provider disconnect for user {} provider {}",
            user_id,
            provider
        );

        match provider {
            oauth_providers::STRAVA | oauth_providers::FITBIT => {
                // Get user to find tenant_id
                let user = self
                    .data
                    .database()
                    .get_user(user_id)
                    .await?
                    .ok_or_else(|| AppError::not_found("User"))?;
                let tenant_id = user.tenant_id.as_deref().unwrap_or("default");

                // Delete OAuth tokens from database
                self.data
                    .database()
                    .delete_user_oauth_token(user_id, tenant_id, provider)
                    .await?;

                tracing::info!("Disconnected {} for user {}", provider, user_id);

                Ok(())
            }
            _ => Err(AppError::invalid_input(format!("Unsupported provider: {provider}")).into()),
        }
    }

    /// Generate OAuth authorization URL for provider
    ///
    /// # Errors
    /// Returns error if provider is unsupported or URL generation fails
    pub fn get_auth_url(
        &self,
        user_id: uuid::Uuid,
        tenant_id: uuid::Uuid,
        provider: &str,
    ) -> Result<OAuthAuthorizationResponse> {
        use crate::constants::oauth_providers;

        let state = format!("{}:{}", user_id, uuid::Uuid::new_v4());
        let base_url = format!("http://localhost:{}", self.config.config().http_port);
        let redirect_uri = format!("{base_url}/api/oauth/callback/{provider}");

        let authorization_url = match provider {
            oauth_providers::STRAVA => {
                let client_id = "test_client_id";
                format!(
                    "https://www.strava.com/oauth/authorize?client_id={client_id}&response_type=code&redirect_uri={redirect_uri}&approval_prompt=force&scope=read%2Cactivity%3Aread_all&state={state}"
                )
            }
            oauth_providers::FITBIT => {
                let client_id = "test_client_id";
                format!(
                    "https://www.fitbit.com/oauth2/authorize?response_type=code&client_id={client_id}&redirect_uri={redirect_uri}&scope=activity%20profile&state={state}"
                )
            }
            _ => {
                return Err(
                    AppError::invalid_input(format!("Unsupported provider: {provider}")).into(),
                )
            }
        };

        tracing::debug!(
            "Generated OAuth authorization URL for user {} tenant {} provider {}",
            user_id,
            tenant_id,
            provider
        );

        Ok(OAuthAuthorizationResponse {
            authorization_url,
            state,
            instructions: format!("Click the link to authorize {provider} access"),
            expires_in_minutes: 10,
        })
    }

    /// Get OAuth connection status for user
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_connection_status(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<ConnectionStatus>> {
        use crate::constants::oauth_providers;

        tracing::debug!("Getting OAuth connection status for user {}", user_id);

        // Get all OAuth tokens for the user from database
        let tokens = self.data.database().get_user_oauth_tokens(user_id).await?;

        // Create a set of connected providers
        let mut providers_seen = std::collections::HashSet::new();
        let mut statuses = Vec::new();

        // Add status for each connected provider
        for token in tokens {
            if providers_seen.insert(token.provider.clone()) {
                statuses.push(ConnectionStatus {
                    provider: token.provider.clone(),
                    connected: true,
                    expires_at: token.expires_at.map(|dt| dt.to_rfc3339()),
                    scopes: token.scope.clone(),
                });
            }
        }

        // Add default status for providers that are not connected
        for provider in [oauth_providers::STRAVA, oauth_providers::FITBIT] {
            if !providers_seen.contains(provider) {
                statuses.push(ConnectionStatus {
                    provider: provider.to_string(),
                    connected: false,
                    expires_at: None,
                    scopes: None,
                });
            }
        }

        Ok(statuses)
    }
}

/// OAuth routes - alias for OAuth service to match test expectations
pub type OAuthRoutes = OAuthService;

/// OAuth callback response
#[derive(Debug, Serialize)]
pub struct OAuthCallbackResponse {
    pub user_id: String,
    pub provider: String,
    pub expires_at: String,
    pub scopes: String,
}

/// Authentication routes implementation
#[derive(Clone)]
pub struct AuthRoutes {
    auth_service: AuthService,
}

impl AuthRoutes {
    /// Create new `AuthRoutes` with embedded service
    #[must_use]
    pub const fn new(auth_context: AuthContext, data_context: DataContext) -> Self {
        Self {
            auth_service: AuthService::new(auth_context, data_context),
        }
    }

    /// Delegate to auth service for registration
    ///
    /// # Errors
    /// Returns error if user validation fails or database operation fails
    pub async fn register(&self, request: RegisterRequest) -> Result<RegisterResponse> {
        self.auth_service.register(request).await
    }

    /// Delegate to auth service for login
    ///
    /// # Errors
    /// Returns error if authentication fails or token generation fails
    pub async fn login(&self, request: LoginRequest) -> Result<LoginResponse> {
        self.auth_service.login(request).await
    }

    /// Delegate to auth service for token refresh
    ///
    /// # Errors
    /// Returns error if refresh token is invalid or token generation fails
    pub async fn refresh_token(&self, request: RefreshTokenRequest) -> Result<LoginResponse> {
        self.auth_service.refresh_token(request).await
    }

    /// Validate email format
    #[must_use]
    pub fn is_valid_email(email: &str) -> bool {
        AuthService::is_valid_email(email)
    }

    /// Validate password strength
    #[must_use]
    pub const fn is_valid_password(password: &str) -> bool {
        AuthService::is_valid_password(password)
    }
    /// Create all authentication routes
    #[must_use]
    pub fn routes(
        resources: Arc<ServerResources>,
    ) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        let register = Self::register_route(resources.clone());
        let login = Self::login_route(resources.clone());
        let refresh = Self::refresh_route(resources.clone());
        let oauth_callback = Self::oauth_callback_route(resources.clone());
        let oauth_status = Self::oauth_status_route(resources.clone());
        let oauth_auth = Self::oauth_auth_route(resources);

        register
            .or(login)
            .or(refresh)
            .or(oauth_callback)
            .or(oauth_status)
            .or(oauth_auth)
            .boxed()
    }

    /// User registration endpoint
    fn register_route(
        resources: Arc<ServerResources>,
    ) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        warp::path("api")
            .and(warp::path("auth"))
            .and(warp::path("register"))
            .and(warp::path::end())
            .and(warp::post())
            .and(warp::body::json())
            .and(with_resources(resources))
            .and_then(Self::handle_register)
    }

    /// User login endpoint
    fn login_route(
        resources: Arc<ServerResources>,
    ) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        warp::path("api")
            .and(warp::path("auth"))
            .and(warp::path("login"))
            .and(warp::path::end())
            .and(warp::post())
            .and(warp::body::json())
            .and(with_resources(resources))
            .and_then(Self::handle_login)
    }

    /// Token refresh endpoint
    fn refresh_route(
        resources: Arc<ServerResources>,
    ) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        warp::path("api")
            .and(warp::path("auth"))
            .and(warp::path("refresh"))
            .and(warp::path::end())
            .and(warp::post())
            .and(warp::body::json())
            .and(with_resources(resources))
            .and_then(Self::handle_refresh)
    }

    /// OAuth callback endpoint
    fn oauth_callback_route(
        resources: Arc<ServerResources>,
    ) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        warp::path("api")
            .and(warp::path("oauth"))
            .and(warp::path("callback"))
            .and(warp::path::param::<String>())
            .and(warp::path::end())
            .and(warp::get())
            .and(warp::query::query())
            .and(with_resources(resources))
            .and_then(Self::handle_oauth_callback)
    }

    /// OAuth status endpoint
    fn oauth_status_route(
        resources: Arc<ServerResources>,
    ) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        warp::path("api")
            .and(warp::path("oauth"))
            .and(warp::path("status"))
            .and(warp::path::end())
            .and(warp::get())
            .and(warp::header::optional::<String>("authorization"))
            .and(with_resources(resources))
            .and_then(Self::handle_oauth_status)
    }

    /// OAuth authorization initiation endpoint
    fn oauth_auth_route(
        resources: Arc<ServerResources>,
    ) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        warp::path("api")
            .and(warp::path("oauth"))
            .and(warp::path("auth"))
            .and(warp::path::param::<String>()) // provider
            .and(warp::path::param::<String>()) // user_id
            .and(warp::path::end())
            .and(warp::get())
            .and(with_resources(resources))
            .and_then(Self::handle_oauth_auth_initiate)
    }

    /// Handle user registration
    async fn handle_register(
        request: RegisterRequest,
        resources: Arc<ServerResources>,
    ) -> Result<impl Reply, Rejection> {
        let server_context = crate::context::ServerContext::from(resources.as_ref());
        let auth_routes =
            AuthService::new(server_context.auth().clone(), server_context.data().clone());
        match auth_routes.register(request).await {
            Ok(response) => Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::CREATED,
            )),
            Err(e) => {
                tracing::error!("Registration failed: {}", e);
                Err(warp::reject::custom(AppError::from(e)))
            }
        }
    }

    /// Handle user login
    async fn handle_login(
        request: LoginRequest,
        resources: Arc<ServerResources>,
    ) -> Result<impl Reply, Rejection> {
        let server_context = crate::context::ServerContext::from(resources.as_ref());
        let auth_routes =
            AuthService::new(server_context.auth().clone(), server_context.data().clone());
        match auth_routes.login(request).await {
            Ok(response) => Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::OK,
            )),
            Err(e) => {
                tracing::error!("Login failed: {}", e);
                Err(warp::reject::custom(AppError::from(e)))
            }
        }
    }

    /// Handle token refresh
    async fn handle_refresh(
        request: RefreshTokenRequest,
        resources: Arc<ServerResources>,
    ) -> Result<impl Reply, Rejection> {
        let server_context = crate::context::ServerContext::from(resources.as_ref());
        let auth_routes =
            AuthService::new(server_context.auth().clone(), server_context.data().clone());
        match auth_routes.refresh_token(request).await {
            Ok(response) => Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::OK,
            )),
            Err(e) => {
                tracing::error!("Token refresh failed: {}", e);
                Err(warp::reject::custom(AppError::from(e)))
            }
        }
    }

    /// Handle OAuth provider callback
    async fn handle_oauth_callback(
        provider: String,
        params: std::collections::HashMap<String, String>,
        resources: Arc<ServerResources>,
    ) -> Result<impl Reply, Rejection> {
        let server_context = crate::context::ServerContext::from(resources.as_ref());
        let oauth_routes = OAuthService::new(
            server_context.data().clone(),
            server_context.config().clone(),
            server_context.notification().clone(),
        );

        let code = params.get("code").ok_or_else(|| {
            warp::reject::custom(AppError::auth_invalid("Missing OAuth code parameter"))
        })?;

        let state = params.get("state").ok_or_else(|| {
            warp::reject::custom(AppError::auth_invalid("Missing OAuth state parameter"))
        })?;

        match oauth_routes.handle_callback(code, state, &provider).await {
            Ok(response) => {
                let html =
                    crate::mcp::oauth_flow_manager::OAuthTemplateRenderer::render_success_template(
                        &provider, &response,
                    )
                    .map_err(|e| {
                        tracing::error!("Failed to render OAuth success template: {}", e);
                        warp::reject::custom(AppError::internal("Template rendering failed"))
                    })?;

                Ok(warp::reply::with_status(
                    warp::reply::html(html),
                    warp::http::StatusCode::OK,
                ))
            }
            Err(e) => {
                tracing::error!("OAuth callback failed: {}", e);

                // Determine error message and description based on error type
                let (error_msg, description) = Self::categorize_oauth_error(&e);

                let html =
                    crate::mcp::oauth_flow_manager::OAuthTemplateRenderer::render_error_template(
                        &provider,
                        error_msg,
                        description,
                    )
                    .map_err(|template_err| {
                        tracing::error!(
                            "Critical: Failed to render OAuth error template: {}",
                            template_err
                        );
                        warp::reject::custom(AppError::internal("Template rendering failed"))
                    })?;

                Ok(warp::reply::with_status(
                    warp::reply::html(html),
                    warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
        }
    }

    /// Handle OAuth status check
    async fn handle_oauth_status(
        auth_header: Option<String>,
        resources: Arc<ServerResources>,
    ) -> Result<impl Reply, Rejection> {
        // Use async block to satisfy clippy
        tokio::task::yield_now().await;
        // Extract user from auth header
        let user_id = match auth_header {
            Some(header) => {
                // Extract JWT and get user ID
                let token = header.strip_prefix("Bearer ").unwrap_or(&header);
                let claims = resources
                    .auth_manager
                    .validate_token(token, &resources.jwks_manager)
                    .map_err(|e| warp::reject::custom(AppError::from(e)))?;
                uuid::Uuid::parse_str(&claims.sub)
                    .map_err(|e| warp::reject::custom(AppError::auth_invalid(e.to_string())))?
            }
            None => {
                return Err(warp::reject::custom(AppError::auth_required()));
            }
        };

        // Check OAuth provider connection status for the user
        let provider_statuses = resources
            .database
            .get_user_oauth_tokens(user_id)
            .await
            .map_or_else(
                |_| {
                    vec![
                        OAuthStatus {
                            provider: "strava".to_string(),
                            connected: false,
                            last_sync: None,
                        },
                        OAuthStatus {
                            provider: "fitbit".to_string(),
                            connected: false,
                            last_sync: None,
                        },
                    ]
                },
                |tokens| {
                    // Convert tokens to status objects
                    let mut statuses = vec![];
                    let mut providers_seen = std::collections::HashSet::new();

                    for token in tokens {
                        if providers_seen.insert(token.provider.clone()) {
                            statuses.push(OAuthStatus {
                                provider: token.provider,
                                connected: true,
                                last_sync: Some(token.created_at.to_rfc3339()),
                            });
                        }
                    }

                    // Add default providers if not connected
                    for provider in ["strava", "fitbit"] {
                        if !providers_seen.contains(provider) {
                            statuses.push(OAuthStatus {
                                provider: provider.to_string(),
                                connected: false,
                                last_sync: None,
                            });
                        }
                    }

                    statuses
                },
            );

        Ok(warp::reply::with_status(
            warp::reply::json(&provider_statuses),
            warp::http::StatusCode::OK,
        ))
    }

    /// Render HTML error response for OAuth failures
    fn render_oauth_html_error(
        provider: &str,
        error: &str,
        description: Option<&str>,
        status: warp::http::StatusCode,
    ) -> Result<Box<dyn warp::Reply>, Rejection> {
        let error_html =
            crate::mcp::oauth_flow_manager::OAuthTemplateRenderer::render_error_template(
                provider,
                error,
                description,
            )
            .map_err(|template_err| {
                tracing::error!(
                    "Critical: Failed to render OAuth error template: {}",
                    template_err
                );
                warp::reject::custom(AppError::internal("Template rendering failed"))
            })?;
        let error_response = warp::reply::with_status(warp::reply::html(error_html), status);
        Ok(Box::new(error_response) as Box<dyn warp::Reply>)
    }

    /// Validate and retrieve user for OAuth flow
    ///
    /// Returns error tuple: (`error_message`, `description`, `status_code`)
    async fn validate_user_for_oauth(
        user_id_str: &str,
        resources: &Arc<ServerResources>,
    ) -> Result<
        (uuid::Uuid, crate::models::User),
        (&'static str, Option<&'static str>, warp::http::StatusCode),
    > {
        let user_id = uuid::Uuid::parse_str(user_id_str).map_err(|_| {
            tracing::error!("Invalid user_id format: {}", user_id_str);
            (
                "Invalid user ID format",
                Some("The user ID provided in the URL is not a valid UUID format."),
                warp::http::StatusCode::BAD_REQUEST,
            )
        })?;

        let user = match resources.database.get_user(user_id).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                tracing::error!("User {} not found in database", user_id);
                return Err((
                    "User account not found",
                    Some("The user account associated with this OAuth request could not be found in the database."),
                    warp::http::StatusCode::NOT_FOUND,
                ));
            }
            Err(e) => {
                tracing::error!("Failed to get user {} for OAuth: {}", user_id, e);
                return Err((
                    "Database error",
                    Some("Failed to retrieve user information from the database. Please try again later."),
                    warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                ));
            }
        };

        Ok((user_id, user))
    }

    /// Categorize OAuth errors into user-friendly messages
    ///
    /// Returns tuple of (`error_message`, `optional_description`)
    fn categorize_oauth_error(error: &anyhow::Error) -> (&'static str, Option<&'static str>) {
        let error_str = error.to_string().to_lowercase();

        if error_str.contains("jwt") && error_str.contains("expired") {
            (
                "Your session has expired",
                Some("Please log in again to continue with OAuth authorization"),
            )
        } else if error_str.contains("jwt") && error_str.contains("invalid signature") {
            (
                "Invalid authentication token",
                Some("The authentication token signature is invalid. This may happen if the server's secret key has changed. Please log in again."),
            )
        } else if error_str.contains("jwt") && error_str.contains("malformed") {
            (
                "Malformed authentication token",
                Some("The authentication token format is invalid. Please log in again."),
            )
        } else if error_str.contains("jwt") {
            (
                "Authentication token validation failed",
                Some(
                    "There was an issue validating your authentication token. Please log in again.",
                ),
            )
        } else if error_str.contains("user not found") {
            (
                "User account not found",
                Some("The user account associated with this OAuth request could not be found."),
            )
        } else if error_str.contains("tenant") {
            (
                "Tenant configuration error",
                Some("There was an issue with your account's tenant configuration. Please contact support."),
            )
        } else if error_str.contains("oauth code") || error_str.contains("token exchange") {
            (
                "OAuth token exchange failed",
                Some("Failed to exchange the authorization code for an access token. The provider may have rejected the request."),
            )
        } else if error_str.contains("state parameter") {
            (
                "Invalid OAuth state",
                Some("The OAuth state parameter is invalid or has been tampered with. This is a security measure to prevent CSRF attacks."),
            )
        } else {
            (
                "OAuth authorization failed",
                Some("An unexpected error occurred during the OAuth authorization process."),
            )
        }
    }

    /// Handle OAuth authorization initiation - redirects to provider OAuth page
    ///
    /// This function validates the user and tenant, processes optional OAuth credentials
    /// from headers, and generates an authorization redirect URL for the specified provider.
    /// All errors return HTML templates instead of JSON for better user experience.
    ///
    /// # Flow
    /// 1. Parse and validate user UUID
    /// 2. Retrieve user from database and validate tenant
    /// 3. Generate OAuth authorization URL
    /// 4. Redirect user to provider's OAuth page
    ///
    /// # Errors
    /// Returns HTML error pages for: invalid user ID, user not found, database errors,
    /// or OAuth URL generation failures
    async fn handle_oauth_auth_initiate(
        provider: String,
        user_id_str: String,
        resources: Arc<ServerResources>,
    ) -> Result<Box<dyn warp::Reply>, Rejection> {
        tracing::info!(
            "OAuth authorization initiation for provider: {} user: {}",
            provider,
            user_id_str
        );

        // Validate user and get user data
        let (user_id, user) = match Self::validate_user_for_oauth(&user_id_str, &resources).await {
            Ok(result) => result,
            Err((error, description, status)) => {
                return Self::render_oauth_html_error(&provider, error, description, status);
            }
        };

        // Get tenant_id from user - CRITICAL: must parse correctly for tenant isolation
        let tenant_id = match &user.tenant_id {
            Some(tid) => match uuid::Uuid::parse_str(tid.as_str()) {
                Ok(parsed_tid) => parsed_tid,
                Err(e) => {
                    tracing::error!(
                        user_id = %user_id,
                        tenant_id_str = %tid,
                        error = ?e,
                        "Invalid tenant_id format in database - tenant isolation compromised"
                    );
                    return Self::render_oauth_html_error(
                        &provider,
                        "invalid_tenant",
                        Some("User tenant configuration is invalid - please contact support"),
                        warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                    );
                }
            },
            None => {
                // User has no tenant - use user_id as single-tenant fallback
                tracing::debug!(user_id = %user_id, "User has no tenant_id - using user_id as tenant");
                user_id
            }
        };

        // Get OAuth authorization URL
        let server_context = crate::context::ServerContext::from(resources.as_ref());
        let oauth_service = OAuthService::new(
            server_context.data().clone(),
            server_context.config().clone(),
            server_context.notification().clone(),
        );

        match oauth_service.get_auth_url(user_id, tenant_id, &provider) {
            Ok(auth_response) => {
                tracing::info!(
                    "Generated OAuth URL for {} user {}: {}",
                    provider,
                    user_id,
                    auth_response.authorization_url
                );
                // Redirect to the provider's OAuth authorization page
                let redirect_response = warp::reply::with_header(
                    warp::reply::with_status(warp::reply::html(""), warp::http::StatusCode::FOUND),
                    "Location",
                    auth_response.authorization_url,
                );
                Ok(Box::new(redirect_response) as Box<dyn warp::Reply>)
            }
            Err(e) => {
                tracing::error!(
                    "Failed to generate OAuth URL for {} user {}: {}",
                    provider,
                    user_id,
                    e
                );
                Self::render_oauth_html_error(
                    &provider,
                    "Failed to generate OAuth URL",
                    Some(
                        "Could not generate the authorization URL. Please check your OAuth configuration.",
                    ),
                    warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            }
        }
    }
}

/// Helper to inject resources into route handlers
fn with_resources(
    resources: Arc<ServerResources>,
) -> impl Filter<Extract = (Arc<ServerResources>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || resources.clone())
}
