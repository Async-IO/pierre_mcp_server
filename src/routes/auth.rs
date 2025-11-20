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
    errors::{AppError, AppResult},
    mcp::resources::ServerResources,
    models::User,
    utils::errors::{auth_error, user_state_error, validation_error},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing;
use urlencoding::encode;

/// User registration request
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    /// User's email address
    pub email: String,
    /// User's password (will be hashed)
    pub password: String,
    /// Optional display name for the user
    pub display_name: Option<String>,
}

/// User registration response
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    /// Unique identifier for the newly created user
    pub user_id: String,
    /// Success message for the registration
    pub message: String,
}

/// User login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// User's email address
    pub email: String,
    /// User's password
    pub password: String,
}

/// User info for login response
#[derive(Debug, Serialize)]
pub struct UserInfo {
    /// Unique identifier for the user
    pub user_id: String,
    /// User's email address
    pub email: String,
    /// User's display name if set
    pub display_name: Option<String>,
}

/// User login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    /// JWT authentication token
    pub jwt_token: String,
    /// When the token expires (ISO 8601 format)
    pub expires_at: String,
    /// User information
    pub user: UserInfo,
}

/// Refresh token request
#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    /// Current JWT token to refresh
    pub token: String,
    /// User ID for validation
    pub user_id: String,
}

/// OAuth provider connection status
#[derive(Debug, Serialize)]
pub struct OAuthStatus {
    /// Name of the OAuth provider (e.g., "strava", "google")
    pub provider: String,
    /// Whether the user is currently connected to this provider
    pub connected: bool,
    /// When the last sync occurred (ISO 8601 format)
    pub last_sync: Option<String>,
}

/// Setup status response for admin setup endpoint
#[derive(Debug, Clone, Serialize)]
pub struct SetupStatusResponse {
    /// Whether the system needs initial setup
    pub needs_setup: bool,
    /// Whether an admin user already exists
    pub admin_user_exists: bool,
    /// Optional status message
    pub message: Option<String>,
}

/// OAuth authorization response for provider auth URLs
#[derive(Debug, Serialize)]
pub struct OAuthAuthorizationResponse {
    /// URL to redirect user to for OAuth authorization
    pub authorization_url: String,
    /// CSRF state token for validating callback
    pub state: String,
    /// Human-readable instructions for the user
    pub instructions: String,
    /// How long the authorization URL is valid (minutes)
    pub expires_in_minutes: i64,
}

/// Connection status for fitness providers
#[derive(Debug, Serialize)]
pub struct ConnectionStatus {
    /// Name of the fitness provider (e.g., "strava", "garmin")
    pub provider: String,
    /// Whether the user is connected to this provider
    pub connected: bool,
    /// When the connection expires (ISO 8601 format)
    pub expires_at: Option<String>,
    /// Space-separated list of granted OAuth scopes
    pub scopes: Option<String>,
}

/// Authentication service for business logic
#[derive(Clone)]
pub struct AuthService {
    auth_context: AuthContext,
    data_context: DataContext,
}

impl AuthService {
    /// Creates a new authentication service
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
    #[tracing::instrument(skip(self, request), fields(route = "register", email = %request.email))]
    pub async fn register(&self, request: RegisterRequest) -> AppResult<RegisterResponse> {
        tracing::info!("User registration attempt for email: {}", request.email);

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
            .data_context
            .database()
            .get_user_by_email(&request.email)
            .await
        {
            return Err(user_state_error(error_messages::USER_ALREADY_EXISTS));
        }

        // Hash password
        let password_hash = bcrypt::hash(&request.password, bcrypt::DEFAULT_COST)
            .map_err(|e| AppError::internal(format!("Password hashing failed: {e}")))?;

        // Create user
        let user = User::new(request.email.clone(), password_hash, request.display_name); // Safe: String ownership needed for user model

        // Save user to database
        let user_id = self
            .data_context
            .database()
            .create_user(&user)
            .await
            .map_err(|e| AppError::database(format!("Failed to create user: {e}")))?;

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
    #[tracing::instrument(skip(self, request), fields(route = "login", email = %request.email))]
    pub async fn login(&self, request: LoginRequest) -> AppResult<LoginResponse> {
        tracing::info!("User login attempt for email: {}", request.email);

        // Get user from database
        let user = self
            .data_context
            .database()
            .get_user_by_email_required(&request.email)
            .await
            .map_err(|e| {
                tracing::debug!(email = %request.email, error = %e, "Login failed: user lookup error");
                AppError::auth_invalid("Invalid email or password")
            })?;

        // Verify password using spawn_blocking to avoid blocking async executor
        let password = request.password.clone();
        let password_hash = user.password_hash.clone();
        let is_valid =
            tokio::task::spawn_blocking(move || bcrypt::verify(&password, &password_hash))
                .await
                .map_err(|e| AppError::internal(format!("Password verification task failed: {e}")))?
                .map_err(|_| AppError::auth_invalid("Invalid email or password"))?;

        if !is_valid {
            tracing::error!("Invalid password for user: {}", request.email);
            return Err(auth_error(error_messages::INVALID_CREDENTIALS));
        }

        // Check if user is approved to login
        if !user.user_status.can_login() {
            tracing::warn!(
                "Login blocked for user: {} - status: {:?}",
                request.email,
                user.user_status
            );
            return Err(user_state_error(user.user_status.to_message()));
        }

        // Update last active timestamp
        self.data_context
            .database()
            .update_last_active(user.id)
            .await
            .map_err(|e| AppError::database(format!("Failed to update last active: {e}")))?;

        // Generate JWT token using RS256
        let jwt_token = self
            .auth_context
            .auth_manager()
            .generate_token(&user, self.auth_context.jwks_manager())
            .map_err(|e| AppError::auth_invalid(format!("Failed to generate token: {e}")))?;
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
    pub async fn refresh_token(&self, request: RefreshTokenRequest) -> AppResult<LoginResponse> {
        tracing::info!("Token refresh attempt for user with refresh token");

        // Extract user from refresh token using RS256 validation
        let token_claims = self
            .auth_context
            .auth_manager()
            .validate_token(&request.token, self.auth_context.jwks_manager())
            .map_err(|_| AppError::auth_invalid("Invalid or expired token"))?;
        let user_id = uuid::Uuid::parse_str(&token_claims.sub)
            .map_err(|e| AppError::auth_invalid(format!("Invalid token format: {e}")))?;

        // Validate that the user_id matches the one in the request
        let request_user_id = uuid::Uuid::parse_str(&request.user_id)?;
        if user_id != request_user_id {
            return Err(AppError::auth_invalid("User ID mismatch"));
        }

        // Get user from database
        let user = self
            .data_context
            .database()
            .get_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user: {e}")))?
            .ok_or_else(|| AppError::not_found("User"))?;

        // Generate new JWT token using RS256
        let new_jwt_token = self
            .auth_context
            .auth_manager()
            .generate_token(&user, self.auth_context.jwks_manager())
            .map_err(|e| AppError::auth_invalid(format!("Failed to generate token: {e}")))?;
        let expires_at =
            chrono::Utc::now() + chrono::Duration::hours(limits::DEFAULT_SESSION_HOURS);

        // Update last active timestamp
        self.data_context
            .database()
            .update_last_active(user.id)
            .await
            .map_err(|e| AppError::database(format!("Failed to update last active: {e}")))?;

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
    _notifications: NotificationContext,
}

impl OAuthService {
    /// Creates a new OAuth service instance
    #[must_use]
    pub const fn new(
        data_context: DataContext,
        config_context: ConfigContext,
        notification_context: NotificationContext,
    ) -> Self {
        Self {
            data: data_context,
            config: config_context,
            _notifications: notification_context,
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
    ) -> AppResult<OAuthCallbackResponse> {
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
            provider: provider.to_owned(),
            expires_at: expires_at.to_rfc3339(),
            scopes: token.scope.unwrap_or_else(|| "read".to_owned()),
        })
    }

    /// Validate OAuth state parameter and extract user ID
    fn validate_oauth_state(state: &str) -> AppResult<uuid::Uuid> {
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
            return Err(AppError::invalid_input("Invalid OAuth state parameter"));
        }

        crate::utils::uuid::parse_user_id(user_id_str)
            .map_err(|e| AppError::invalid_input(format!("Invalid user ID in state: {e}")))
    }

    /// Validate that provider is supported
    fn validate_provider(provider: &str) -> AppResult<()> {
        use crate::constants::oauth_providers;
        match provider {
            oauth_providers::STRAVA | oauth_providers::FITBIT => Ok(()),
            _ => Err(AppError::invalid_input(format!(
                "Unsupported provider: {provider}"
            ))),
        }
    }

    /// Get user and tenant from database
    async fn get_user_and_tenant(
        &self,
        user_id: uuid::Uuid,
        provider: &str,
    ) -> AppResult<(crate::models::User, String)> {
        let database = self.data.database();
        let user = database
            .get_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user: {e}")))?
            .ok_or_else(|| {
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
    ) -> AppResult<crate::oauth2_client::OAuth2Token> {
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
    ///
    /// # Errors
    /// Returns error if provider is unsupported or required credentials are not configured
    fn create_oauth_config(&self, provider: &str) -> AppResult<crate::oauth2_client::OAuth2Config> {
        let server_config = self.config.config();
        match provider {
            "strava" => {
                let oauth_config = &server_config.oauth.strava;
                let api_config = &server_config.external_services.strava_api;

                Ok(crate::oauth2_client::OAuth2Config {
                    client_id: oauth_config.client_id.clone().ok_or_else(|| {
                        AppError::invalid_input(
                            "Strava client_id not configured for token exchange",
                        )
                    })?,
                    client_secret: oauth_config.client_secret.clone().ok_or_else(|| {
                        AppError::invalid_input(
                            "Strava client_secret not configured for token exchange",
                        )
                    })?,
                    auth_url: api_config.auth_url.clone(),
                    token_url: api_config.token_url.clone(),
                    redirect_uri: oauth_config.redirect_uri.clone().unwrap_or_else(|| {
                        format!(
                            "http://localhost:{}/api/oauth/callback/strava",
                            server_config.http_port
                        )
                    }),
                    scopes: vec![crate::constants::oauth::STRAVA_DEFAULT_SCOPES.to_owned()],
                    use_pkce: true,
                })
            }
            "fitbit" => {
                let oauth_config = &server_config.oauth.fitbit;
                let api_config = &server_config.external_services.fitbit_api;

                Ok(crate::oauth2_client::OAuth2Config {
                    client_id: oauth_config.client_id.clone().ok_or_else(|| {
                        AppError::invalid_input(
                            "Fitbit client_id not configured for token exchange",
                        )
                    })?,
                    client_secret: oauth_config.client_secret.clone().ok_or_else(|| {
                        AppError::invalid_input(
                            "Fitbit client_secret not configured for token exchange",
                        )
                    })?,
                    auth_url: api_config.auth_url.clone(),
                    token_url: api_config.token_url.clone(),
                    redirect_uri: oauth_config.redirect_uri.clone().unwrap_or_else(|| {
                        format!(
                            "http://localhost:{}/api/oauth/callback/fitbit",
                            server_config.http_port
                        )
                    }),
                    scopes: vec![crate::constants::oauth::FITBIT_DEFAULT_SCOPES.to_owned()],
                    use_pkce: false, // Fitbit uses client_secret instead of PKCE
                })
            }
            _ => Err(AppError::invalid_input(format!(
                "Unsupported provider: {provider}"
            ))),
        }
    }

    /// Store OAuth token in database
    async fn store_oauth_token(
        &self,
        user_id: uuid::Uuid,
        tenant_id: String,
        provider: &str,
        token: &crate::oauth2_client::OAuth2Token,
    ) -> AppResult<chrono::DateTime<chrono::Utc>> {
        let expires_at = token
            .expires_at
            .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::hours(1));

        let user_oauth_token = crate::models::UserOAuthToken {
            id: uuid::Uuid::new_v4().to_string(),
            user_id,
            tenant_id,
            provider: provider.to_owned(),
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
            .await
            .map_err(|e| AppError::database(format!("Failed to upsert OAuth token: {e}")))?;
        Ok(expires_at)
    }

    /// Send OAuth completion notifications
    async fn send_oauth_notifications(
        &self,
        user_id: uuid::Uuid,
        provider: &str,
        expires_at: &chrono::DateTime<chrono::Utc>,
    ) -> AppResult<()> {
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
            .await
            .map_err(|e| AppError::database(format!("Failed to store OAuth notification: {e}")))?;

        tracing::info!(
            "Created OAuth completion notification {} for user {} provider {}",
            notification_id,
            user_id,
            provider
        );

        // OAuth notification - SSE notification sending disabled
        tracing::info!(
            notification_id = %notification_id,
            user_id = %user_id,
            provider = %provider,
            "OAuth notification processed (SSE disabled)"
        );

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
    pub async fn disconnect_provider(&self, user_id: uuid::Uuid, provider: &str) -> AppResult<()> {
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
                    .await
                    .map_err(|e| AppError::database(format!("Failed to get user: {e}")))?
                    .ok_or_else(|| AppError::not_found("User"))?;
                let tenant_id = user.tenant_id.as_deref().unwrap_or("default");

                // Delete OAuth tokens from database
                self.data
                    .database()
                    .delete_user_oauth_token(user_id, tenant_id, provider)
                    .await
                    .map_err(|e| {
                        AppError::database(format!("Failed to delete OAuth token: {e}"))
                    })?;

                tracing::info!("Disconnected {} for user {}", provider, user_id);

                Ok(())
            }
            _ => Err(AppError::invalid_input(format!(
                "Unsupported provider: {provider}"
            ))),
        }
    }

    /// Generate OAuth authorization URL for provider
    ///
    /// This function supports both multi-tenant and single-tenant modes:
    /// - Multi-tenant: Uses tenant-specific OAuth credentials from database
    /// - Single-tenant: Falls back to server-level configuration
    ///
    /// # Errors
    /// Returns error if provider is unsupported or OAuth credentials not configured
    pub async fn get_auth_url(
        &self,
        user_id: uuid::Uuid,
        tenant_id: uuid::Uuid,
        provider: &str,
    ) -> AppResult<OAuthAuthorizationResponse> {
        use crate::constants::oauth_providers;

        // Check for tenant-specific OAuth credentials first (multi-tenant mode)
        let tenant_creds = self
            .data
            .database()
            .get_tenant_oauth_credentials(tenant_id, provider)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to get tenant OAuth credentials: {e}"))
            })?;

        let state = format!("{}:{}", user_id, uuid::Uuid::new_v4());
        let base_url = format!("http://localhost:{}", self.config.config().http_port);
        let redirect_uri = format!("{base_url}/api/oauth/callback/{provider}");

        // URL-encode parameters for OAuth URLs
        let encoded_state = encode(&state);
        let encoded_redirect_uri = encode(&redirect_uri);

        let authorization_url = match provider {
            oauth_providers::STRAVA => {
                let (client_id, scope) = if let Some(creds) = tenant_creds {
                    // Multi-tenant: use tenant-specific credentials
                    let scope = creds.scopes.join(",");
                    (creds.client_id, scope)
                } else {
                    // Single-tenant: use server-level configuration
                    let server_config = self.config.config();
                    let oauth_config = &server_config.oauth.strava;
                    let client_id = oauth_config
                        .client_id
                        .as_ref()
                        .ok_or_else(|| {
                            AppError::invalid_input(
                                "Strava client_id not configured (set in environment or database)",
                            )
                        })?
                        .clone();
                    (
                        client_id,
                        crate::constants::oauth::STRAVA_DEFAULT_SCOPES.to_owned(),
                    )
                };

                let encoded_scope = encode(&scope);

                format!(
                    "https://www.strava.com/oauth/authorize?client_id={client_id}&response_type=code&redirect_uri={encoded_redirect_uri}&approval_prompt=force&scope={encoded_scope}&state={encoded_state}"
                )
            }
            oauth_providers::FITBIT => {
                let (client_id, scope) = if let Some(creds) = tenant_creds {
                    // Multi-tenant: use tenant-specific credentials
                    let scope = creds.scopes.join(" "); // Fitbit uses space-separated scopes
                    (creds.client_id, scope)
                } else {
                    // Single-tenant: use server-level configuration
                    let server_config = self.config.config();
                    let oauth_config = &server_config.oauth.fitbit;
                    let client_id = oauth_config
                        .client_id
                        .as_ref()
                        .ok_or_else(|| {
                            AppError::invalid_input(
                                "Fitbit client_id not configured (set in environment or database)",
                            )
                        })?
                        .clone();
                    (
                        client_id,
                        crate::constants::oauth::FITBIT_DEFAULT_SCOPES.to_owned(),
                    )
                };

                let encoded_scope = encode(&scope);

                format!(
                    "https://www.fitbit.com/oauth2/authorize?response_type=code&client_id={client_id}&redirect_uri={encoded_redirect_uri}&scope={encoded_scope}&state={encoded_state}"
                )
            }
            _ => {
                return Err(AppError::invalid_input(format!(
                    "Unsupported provider: {provider}"
                )))
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
    ) -> AppResult<Vec<ConnectionStatus>> {
        use crate::constants::oauth_providers;

        tracing::debug!("Getting OAuth connection status for user {}", user_id);

        // Get all OAuth tokens for the user from database
        let tokens = self
            .data
            .database()
            .get_user_oauth_tokens(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user OAuth tokens: {e}")))?;

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
                    provider: provider.to_owned(),
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
    /// User ID for the connected account
    pub user_id: String,
    /// Name of the OAuth provider
    pub provider: String,
    /// When the OAuth token expires (ISO 8601 format)
    pub expires_at: String,
    /// Space-separated list of granted OAuth scopes
    pub scopes: String,
}

/// Authentication routes implementation
#[derive(Clone)]

/// Authentication routes implementation (Axum)
///
/// Provides user registration, login, logout, and OAuth client authentication endpoints.
pub struct AuthRoutes;

impl AuthRoutes {
    /// Create all authentication routes (Axum)
    pub fn routes(resources: Arc<ServerResources>) -> axum::Router {
        use axum::{
            routing::{get, post},
            Router,
        };

        Router::new()
            .route("/api/auth/register", post(Self::handle_register))
            .route("/api/auth/login", post(Self::handle_login))
            .route("/api/auth/refresh", post(Self::handle_refresh))
            .route(
                "/api/oauth/callback/:provider",
                get(Self::handle_oauth_callback),
            )
            .route("/api/oauth/status", get(Self::handle_oauth_status))
            .route(
                "/api/oauth/auth/:provider/:user_id",
                get(Self::handle_oauth_auth_initiate),
            )
            .with_state(resources)
    }

    /// Handle user registration (Axum)
    async fn handle_register(
        axum::extract::State(resources): axum::extract::State<Arc<ServerResources>>,
        axum::Json(request): axum::Json<RegisterRequest>,
    ) -> Result<axum::response::Response, AppError> {
        use axum::response::IntoResponse;

        let server_context = crate::context::ServerContext::from(resources.as_ref());
        let auth_routes =
            AuthService::new(server_context.auth().clone(), server_context.data().clone());

        match auth_routes.register(request).await {
            Ok(response) => {
                Ok((axum::http::StatusCode::CREATED, axum::Json(response)).into_response())
            }
            Err(e) => {
                tracing::error!("Registration failed: {}", e);
                Err(e)
            }
        }
    }

    /// Handle user login (Axum)
    async fn handle_login(
        axum::extract::State(resources): axum::extract::State<Arc<ServerResources>>,
        axum::Json(request): axum::Json<LoginRequest>,
    ) -> Result<axum::response::Response, AppError> {
        use axum::response::IntoResponse;

        let server_context = crate::context::ServerContext::from(resources.as_ref());
        let auth_routes =
            AuthService::new(server_context.auth().clone(), server_context.data().clone());

        match auth_routes.login(request).await {
            Ok(response) => Ok((axum::http::StatusCode::OK, axum::Json(response)).into_response()),
            Err(e) => {
                tracing::error!("Login failed: {}", e);
                Err(e)
            }
        }
    }

    /// Handle token refresh (Axum)
    async fn handle_refresh(
        axum::extract::State(resources): axum::extract::State<Arc<ServerResources>>,
        axum::Json(request): axum::Json<RefreshTokenRequest>,
    ) -> Result<axum::response::Response, AppError> {
        use axum::response::IntoResponse;

        let server_context = crate::context::ServerContext::from(resources.as_ref());
        let auth_routes =
            AuthService::new(server_context.auth().clone(), server_context.data().clone());

        match auth_routes.refresh_token(request).await {
            Ok(response) => Ok((axum::http::StatusCode::OK, axum::Json(response)).into_response()),
            Err(e) => {
                tracing::error!("Token refresh failed: {}", e);
                Err(e)
            }
        }
    }

    /// Handle OAuth callback (Axum)
    async fn handle_oauth_callback(
        axum::extract::State(resources): axum::extract::State<Arc<ServerResources>>,
        axum::extract::Path(provider): axum::extract::Path<String>,
        axum::extract::Query(params): axum::extract::Query<
            std::collections::HashMap<String, String>,
        >,
    ) -> Result<axum::response::Response, AppError> {
        use axum::response::IntoResponse;

        let server_context = crate::context::ServerContext::from(resources.as_ref());
        let oauth_routes = OAuthService::new(
            server_context.data().clone(),
            server_context.config().clone(),
            server_context.notification().clone(),
        );

        let code = params
            .get("code")
            .ok_or_else(|| AppError::auth_invalid("Missing OAuth code parameter"))?;

        let state = params
            .get("state")
            .ok_or_else(|| AppError::auth_invalid("Missing OAuth state parameter"))?;

        match oauth_routes.handle_callback(code, state, &provider).await {
            Ok(response) => {
                let html =
                    crate::mcp::oauth_flow_manager::OAuthTemplateRenderer::render_success_template(
                        &provider, &response,
                    )
                    .map_err(|e| {
                        tracing::error!("Failed to render OAuth success template: {}", e);
                        AppError::internal("Template rendering failed")
                    })?;

                Ok((
                    axum::http::StatusCode::OK,
                    [(axum::http::header::CONTENT_TYPE, "text/html")],
                    html,
                )
                    .into_response())
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
                        AppError::internal("Template rendering failed")
                    })?;

                Ok((
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    [(axum::http::header::CONTENT_TYPE, "text/html")],
                    html,
                )
                    .into_response())
            }
        }
    }

    /// Handle OAuth status check (Axum)
    async fn handle_oauth_status(
        axum::extract::State(resources): axum::extract::State<Arc<ServerResources>>,
        headers: axum::http::HeaderMap,
    ) -> Result<axum::response::Response, AppError> {
        use axum::response::IntoResponse;

        // Extract user from auth header
        let auth_header = headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .map(String::from);

        let user_id = match auth_header {
            Some(header) => {
                // Extract JWT and get user ID
                let token = header.strip_prefix("Bearer ").unwrap_or(&header);
                let claims = resources
                    .auth_manager
                    .validate_token(token, &resources.jwks_manager)
                    .map_err(|_| AppError::auth_invalid("Invalid or expired token"))?;
                uuid::Uuid::parse_str(&claims.sub)
                    .map_err(|e| AppError::auth_invalid(e.to_string()))?
            }
            None => {
                return Err(AppError::auth_required());
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
                            provider: "strava".to_owned(),
                            connected: false,
                            last_sync: None,
                        },
                        OAuthStatus {
                            provider: "fitbit".to_owned(),
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
                                provider: provider.to_owned(),
                                connected: false,
                                last_sync: None,
                            });
                        }
                    }

                    statuses
                },
            );

        Ok((axum::http::StatusCode::OK, axum::Json(provider_statuses)).into_response())
    }

    /// Handle OAuth authorization initiation (Axum)
    async fn handle_oauth_auth_initiate(
        axum::extract::State(resources): axum::extract::State<Arc<ServerResources>>,
        axum::extract::Path((provider, user_id_str)): axum::extract::Path<(String, String)>,
    ) -> Result<axum::response::Response, AppError> {
        use axum::response::IntoResponse;

        tracing::info!(
            "OAuth authorization initiation for provider: {} user: {}",
            provider,
            user_id_str
        );

        // Parse and validate user UUID
        let user_id = uuid::Uuid::parse_str(&user_id_str).map_err(|_| {
            tracing::error!("Invalid user_id format: {}", user_id_str);
            AppError::invalid_input("Invalid user ID format")
        })?;

        // Retrieve user from database
        let user = match resources.database.get_user(user_id).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                tracing::error!("User {} not found in database", user_id);
                return Err(AppError::not_found("User account not found"));
            }
            Err(e) => {
                tracing::error!("Failed to get user {} for OAuth: {}", user_id, e);
                return Err(AppError::database(format!(
                    "Failed to retrieve user information: {e}"
                )));
            }
        };

        // Get tenant_id from user - CRITICAL: must parse correctly for tenant isolation
        let tenant_id = if let Some(tid) = &user.tenant_id {
            match uuid::Uuid::parse_str(tid.as_str()) {
                Ok(parsed_tid) => parsed_tid,
                Err(e) => {
                    tracing::error!(
                        user_id = %user_id,
                        tenant_id_str = %tid,
                        error = ?e,
                        "Invalid tenant_id format in database - tenant isolation compromised"
                    );
                    return Err(AppError::internal(
                        "User tenant configuration is invalid - please contact support",
                    ));
                }
            }
        } else {
            // User has no tenant - use user_id as single-tenant fallback
            tracing::debug!(user_id = %user_id, "User has no tenant_id - using user_id as tenant");
            user_id
        };

        // Get OAuth authorization URL
        let server_context = crate::context::ServerContext::from(resources.as_ref());
        let oauth_service = OAuthService::new(
            server_context.data().clone(),
            server_context.config().clone(),
            server_context.notification().clone(),
        );

        match oauth_service
            .get_auth_url(user_id, tenant_id, &provider)
            .await
        {
            Ok(auth_response) => {
                tracing::info!(
                    "Generated OAuth URL for {} user {}: {}",
                    provider,
                    user_id,
                    auth_response.authorization_url
                );
                // Redirect to the provider's OAuth authorization page
                // Use 302 Found (not 307) for OAuth redirects (standard HTTP temporary redirect)
                Ok((
                    axum::http::StatusCode::FOUND,
                    [(
                        axum::http::header::LOCATION,
                        auth_response.authorization_url,
                    )],
                )
                    .into_response())
            }
            Err(e) => {
                tracing::error!(
                    "Failed to generate OAuth URL for {} user {}: {}",
                    provider,
                    user_id,
                    e
                );
                Err(AppError::internal(format!(
                    "Failed to generate OAuth URL for {provider}: {e}"
                )))
            }
        }
    }

    /// Categorize OAuth errors for better user messaging
    fn categorize_oauth_error(error: &AppError) -> (&'static str, Option<&'static str>) {
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
}
