// ABOUTME: User authentication route handlers for registration, login, and OAuth flows
// ABOUTME: Provides REST endpoints for user account management and fitness provider OAuth callbacks
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Authentication routes for user management and OAuth flows
//!
//! This module handles user registration, login, and OAuth callback processing
//! for fitness providers like Strava. All handlers are thin wrappers that
//! delegate business logic to service layers.
//!
//! ## Module Structure
//! - `types` - Request/response DTOs for auth endpoints

mod types;

pub use types::{
    ChangePasswordRequest, CompleteResetRequest, ConnectionStatus, FirebaseLoginRequest,
    LoginRequest, LoginResponse, OAuth2ErrorResponse, OAuth2TokenRequest, OAuth2TokenResponse,
    OAuthAuthorizationResponse, OAuthStatus, ProviderStatus, ProvidersStatusResponse,
    RefreshTokenRequest, RegisterRequest, RegisterResponse, SessionResponse, UpdateProfileRequest,
    UpdateProfileResponse, UserInfo, UserStatsResponse,
};

// Re-export OAuthCallbackResponse from types module (moved for proper layering)
pub use crate::types::OAuthCallbackResponse;

use std::{
    collections::{HashMap, HashSet},
    env,
    fmt::Write,
    sync::Arc,
    time::Duration as StdDuration,
};

use axum::{
    extract::{Form, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json, Router,
};
use chrono::Utc;
use serde_json::{json, Value as JsonValue};
use tokio::task;
use tracing::{debug, error, field::Empty, info, warn, Span};
use urlencoding::encode;

use crate::mcp::oauth_flow_manager::OAuthTemplateRenderer;
use crate::{
    admin::{AdminAuthService, FirebaseAuth, FirebaseClaims},
    config::environment::get_oauth_config,
    constants::{error_messages, limits, tiers},
    context::{AuthContext, ConfigContext, DataContext, NotificationContext, ServerContext},
    database_plugins::{factory::Database, DatabaseProvider},
    errors::{AppError, AppResult, ErrorCode},
    mcp::{resources::ServerResources, schema::OAuthCompletedNotification},
    models::{ConnectionType, Tenant, User, UserOAuthToken, UserStatus, UserTier},
    oauth2_client::{OAuth2Client, OAuth2Config, OAuth2Token, OAuthClientState, PkceParams},
    permissions::UserRole,
    providers::ProviderDescriptor,
    security::cookies::{clear_auth_cookie, get_cookie_value, set_auth_cookie, set_csrf_cookie},
    tenant::{TenantContext, TenantRole},
    utils::{
        auth::extract_bearer_token_owned,
        errors::{auth_error, user_state_error, validation_error},
        http_client::get_oauth_callback_notification_timeout_secs,
    },
};

/// Authentication service for business logic
#[derive(Clone)]
pub struct AuthService {
    auth: AuthContext,
    config: ConfigContext,
    data: DataContext,
}

impl AuthService {
    /// Creates a new authentication service
    #[must_use]
    pub const fn new(auth: AuthContext, config: ConfigContext, data: DataContext) -> Self {
        Self { auth, config, data }
    }

    /// Handle user registration - implementation from existing routes.rs
    ///
    /// # Errors
    /// Returns error if user validation fails or database operation fails
    #[tracing::instrument(skip(self, request), fields(route = "register"))]
    pub async fn register(&self, request: RegisterRequest) -> AppResult<RegisterResponse> {
        info!("User registration attempt");

        // Validate email format
        if !Self::is_valid_email(&request.email) {
            return Err(validation_error(error_messages::INVALID_EMAIL_FORMAT));
        }

        // Validate password strength
        if !Self::is_valid_password(&request.password) {
            return Err(validation_error(error_messages::PASSWORD_TOO_WEAK));
        }

        // Check if user already exists
        if let Ok(Some(_)) = self.data.database().get_user_by_email(&request.email).await {
            return Err(user_state_error(error_messages::USER_ALREADY_EXISTS));
        }

        // Hash password
        let password_hash = bcrypt::hash(&request.password, bcrypt::DEFAULT_COST)
            .map_err(|e| AppError::internal(format!("Password hashing failed: {e}")))?;

        // Create user with default Pending status
        let mut user = User::new(request.email.clone(), password_hash, request.display_name); // Safe: String ownership needed for user model

        // Check if auto-approval is enabled (database setting takes precedence over config)
        if self.is_auto_approval_enabled().await {
            user.user_status = UserStatus::Active;
            user.approved_at = Some(Utc::now());
            info!("Auto-approving user registration (auto_approval_enabled=true)");
        }

        // Save user to database
        let user_id = self
            .data
            .database()
            .create_user(&user)
            .await
            .map_err(|e| AppError::database(format!("Failed to create user: {e}")))?;

        // Create a personal tenant for the user (required for MCP operations)
        let display_name = user
            .display_name
            .as_deref()
            .unwrap_or_else(|| request.email.split('@').next().unwrap_or("user"));

        let tenant_id = self
            .create_personal_tenant(user_id, display_name, tiers::STARTER)
            .await?;

        // Assign user to their personal tenant
        self.data
            .database()
            .update_user_tenant_id(user_id, &tenant_id.to_string())
            .await
            .map_err(|e| {
                error!("Failed to assign user to tenant: {}", e);
                AppError::database(format!("Failed to assign tenant: {e}"))
            })?;

        info!(user_id = %user_id, "User registered successfully");

        let message = if user.user_status == UserStatus::Active {
            "User registered successfully. Your account is ready to use.".to_owned()
        } else {
            "User registered successfully. Your account is pending admin approval.".to_owned()
        };

        Ok(RegisterResponse {
            user_id: user_id.to_string(),
            message,
        })
    }

    /// Handle user login - implementation from existing routes.rs
    ///
    /// # Errors
    /// Returns error if authentication fails or token generation fails
    #[tracing::instrument(skip(self, request), fields(route = "login"))]
    pub async fn login(&self, request: LoginRequest) -> AppResult<LoginResponse> {
        debug!("User login attempt");

        // Get user from database
        let user = self
            .data
            .database()
            .get_user_by_email_required(&request.email)
            .await
            .map_err(|e| {
                debug!(email = %request.email, error = %e, "Login failed: user lookup error");
                AppError::auth_invalid("Invalid email or password")
            })?;

        // Verify password using spawn_blocking to avoid blocking async executor
        let password = request.password.clone();
        let password_hash = user.password_hash.clone();
        let is_valid = task::spawn_blocking(move || bcrypt::verify(&password, &password_hash))
            .await
            .map_err(|e| AppError::internal(format!("Password verification task failed: {e}")))?
            .map_err(|_| AppError::auth_invalid("Invalid email or password"))?;

        if !is_valid {
            error!("Invalid password for login attempt");
            return Err(auth_error(error_messages::INVALID_CREDENTIALS));
        }

        // Log user status for auditing (pending/suspended users can authenticate
        // but frontend restricts access based on user_status)
        if !user.user_status.can_login() {
            info!(
                user_id = %user.id,
                status = ?user.user_status,
                "User login with restricted status"
            );
        }

        // Update last active timestamp
        self.data
            .database()
            .update_last_active(user.id)
            .await
            .map_err(|e| AppError::database(format!("Failed to update last active: {e}")))?;

        // Generate JWT token using RS256
        let jwt_token = self
            .auth
            .auth_manager()
            .generate_token(&user, self.auth.jwks_manager())
            .map_err(|e| AppError::auth_invalid(format!("Failed to generate token: {e}")))?;
        let expires_at =
            chrono::Utc::now() + chrono::Duration::hours(limits::DEFAULT_SESSION_HOURS); // Default 24h expiry

        info!(
            "User logged in successfully: {} ({})",
            request.email, user.id
        );

        Ok(LoginResponse {
            jwt_token: Some(jwt_token),
            csrf_token: String::new(), // Will be set by HTTP handler
            expires_at: expires_at.to_rfc3339(),
            user: UserInfo {
                user_id: user.id.to_string(),
                email: user.email.clone(),
                display_name: user.display_name,
                is_admin: user.is_admin,
                role: user.role.as_str().to_owned(),
                user_status: user.user_status.to_string(),
            },
        })
    }

    /// Handle Firebase login - authenticate with Firebase ID token
    ///
    /// This method validates the Firebase ID token, finds or creates a user,
    /// and returns a JWT token for our authentication system.
    ///
    /// # Errors
    /// Returns error if Firebase validation fails, or user creation fails
    pub async fn login_with_firebase(
        &self,
        request: FirebaseLoginRequest,
        firebase_auth: &FirebaseAuth,
    ) -> AppResult<LoginResponse> {
        tracing::info!("Firebase login attempt");

        // Validate the Firebase ID token
        let claims = firebase_auth.validate_token(&request.id_token).await?;

        // Get the email from the claims (required)
        let email = claims
            .email
            .as_ref()
            .ok_or_else(|| AppError::auth_invalid("Firebase token missing email claim"))?;

        // Find or create user from Firebase claims
        let user = self.find_or_create_firebase_user(&claims, email).await?;

        // Check if user can login (not suspended)
        Self::validate_user_can_login(&user)?;

        // Generate session and return response
        self.complete_firebase_login(&user, &claims.provider).await
    }

    /// Find existing user or create new one from Firebase claims
    async fn find_or_create_firebase_user(
        &self,
        claims: &FirebaseClaims,
        email: &str,
    ) -> AppResult<User> {
        // Try to find user by Firebase UID first
        if let Some(user) = self
            .data
            .database()
            .get_user_by_firebase_uid(&claims.sub)
            .await?
        {
            tracing::info!(user_id = %user.id, firebase_uid = %claims.sub, "Found user by Firebase UID");
            return Ok(user);
        }

        // Check if user exists by email (might need linking)
        if let Some(mut user) = self.data.database().get_user_by_email(email).await? {
            tracing::info!(user_id = %user.id, "Linking existing email user to Firebase UID");
            user.firebase_uid = Some(claims.sub.clone());
            user.auth_provider.clone_from(&claims.provider);
            self.data.database().create_user(&user).await?;
            return Ok(user);
        }

        // Create new user from Firebase claims
        self.create_firebase_user(claims, email).await
    }

    /// Create a personal tenant for a user (required for MCP operations)
    ///
    /// # Errors
    /// Returns error if tenant creation fails
    async fn create_personal_tenant(
        &self,
        user_id: uuid::Uuid,
        display_name: &str,
        plan: &str,
    ) -> AppResult<uuid::Uuid> {
        let tenant_id = uuid::Uuid::new_v4();
        let tenant_name = format!("{display_name}'s Workspace");
        let tenant_slug = format!("user-{}", user_id.as_simple());
        let now = Utc::now();

        let tenant = Tenant {
            id: tenant_id,
            name: tenant_name.clone(),
            slug: tenant_slug,
            domain: None,
            plan: plan.to_owned(),
            owner_user_id: user_id,
            created_at: now,
            updated_at: now,
        };

        self.data
            .database()
            .create_tenant(&tenant)
            .await
            .map_err(|e| {
                error!(
                    "Failed to create personal tenant for user {}: {}",
                    user_id, e
                );
                AppError::database(format!("Failed to create personal tenant: {e}"))
            })?;

        debug!("Created personal tenant: {} ({})", tenant_name, tenant_id);
        Ok(tenant_id)
    }

    /// Check if auto-approval is enabled
    ///
    /// Precedence order:
    /// 1. Environment variable (if explicitly set via `AUTO_APPROVE_USERS`)
    /// 2. Database setting (if present in `system_settings` table)
    /// 3. Default value (false)
    async fn is_auto_approval_enabled(&self) -> bool {
        let config = self.config.config();

        // Environment variable takes precedence when explicitly set
        if config.app_behavior.auto_approve_users_from_env {
            return config.app_behavior.auto_approve_users;
        }

        // Fall back to database setting if present
        match self.data.database().is_auto_approval_enabled().await {
            Ok(Some(db_setting)) => db_setting,
            Ok(None) => config.app_behavior.auto_approve_users,
            Err(e) => {
                tracing::warn!(
                    "Failed to check auto-approval setting, falling back to config: {e}"
                );
                config.app_behavior.auto_approve_users
            }
        }
    }

    /// Determine user approval status based on auto-approval setting
    async fn determine_approval_status(&self) -> (UserStatus, Option<chrono::DateTime<Utc>>) {
        let now = Utc::now();
        if self.is_auto_approval_enabled().await {
            tracing::debug!("Auto-approval enabled for new user");
            (UserStatus::Active, Some(now))
        } else {
            (UserStatus::Pending, None)
        }
    }

    /// Create a new user from Firebase claims
    async fn create_firebase_user(&self, claims: &FirebaseClaims, email: &str) -> AppResult<User> {
        tracing::info!(firebase_uid = %claims.sub, "Creating new Firebase user");

        let (user_status, approved_at) = self.determine_approval_status().await;
        let user_id = uuid::Uuid::new_v4();
        let display_name = claims
            .name
            .as_deref()
            .unwrap_or_else(|| email.split('@').next().unwrap_or("user"));

        // Step 1: Create user first - tenant membership managed via tenant_users table
        let now = Utc::now();
        let new_user = User {
            id: user_id,
            email: email.to_owned(),
            display_name: claims.name.clone(),
            password_hash: "!firebase-auth-only!".to_owned(),
            tier: UserTier::Starter,
            strava_token: None,
            fitbit_token: None,
            created_at: now,
            last_active: now,
            is_active: true,
            user_status,
            is_admin: false,
            role: UserRole::User,
            approved_by: None,
            approved_at,
            firebase_uid: Some(claims.sub.clone()),
            auth_provider: claims.provider.clone(),
        };

        self.data.database().create_user(&new_user).await?;

        // Step 2: Create personal tenant (adds user to tenant_users as owner)
        self.create_personal_tenant(user_id, display_name, tiers::STARTER)
            .await?;

        info!(firebase_uid = %claims.sub, user_id = %user_id, "Firebase user registered");
        Ok(new_user)
    }

    /// Validate that user is allowed to login
    fn validate_user_can_login(user: &User) -> AppResult<()> {
        if user.user_status.can_login() {
            return Ok(());
        }

        tracing::warn!(user_id = %user.id, status = %user.user_status, "Login denied: user status");
        let status_msg = match user.user_status {
            UserStatus::Pending => "Account pending approval",
            UserStatus::Suspended => "Account suspended",
            UserStatus::Active => "Account active",
        };
        Err(user_state_error(status_msg))
    }

    /// Complete Firebase login: generate JWT and update last active
    async fn complete_firebase_login(
        &self,
        user: &User,
        provider: &str,
    ) -> AppResult<LoginResponse> {
        let jwt_token = self
            .auth
            .auth_manager()
            .generate_token(user, self.auth.jwks_manager())
            .map_err(|e| AppError::auth_invalid(format!("Failed to generate token: {e}")))?;

        let expires_at = Utc::now() + chrono::Duration::hours(limits::DEFAULT_SESSION_HOURS);

        self.data.database().update_last_active(user.id).await?;

        tracing::info!(user_id = %user.id, provider = %provider, "Firebase login successful");

        Ok(LoginResponse {
            jwt_token: Some(jwt_token),
            csrf_token: String::new(),
            expires_at: expires_at.to_rfc3339(),
            user: UserInfo {
                user_id: user.id.to_string(),
                email: user.email.clone(),
                display_name: user.display_name.clone(),
                is_admin: user.is_admin,
                role: user.role.as_str().to_owned(),
                user_status: user.user_status.to_string(),
            },
        })
    }

    /// Handle token refresh - implementation from existing routes.rs
    ///
    /// # Errors
    /// Returns error if refresh token is invalid or token generation fails
    pub async fn refresh_token(&self, request: RefreshTokenRequest) -> AppResult<LoginResponse> {
        info!("Token refresh attempt for user with refresh token");

        // Extract user from refresh token using RS256 validation
        let token_claims = self
            .auth
            .auth_manager()
            .validate_token(&request.token, self.auth.jwks_manager())
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
            .data
            .database()
            .get_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user: {e}")))?
            .ok_or_else(|| AppError::not_found("User"))?;

        // Generate new JWT token using RS256
        let new_jwt_token = self
            .auth
            .auth_manager()
            .generate_token(&user, self.auth.jwks_manager())
            .map_err(|e| AppError::auth_invalid(format!("Failed to generate token: {e}")))?;
        let expires_at =
            chrono::Utc::now() + chrono::Duration::hours(limits::DEFAULT_SESSION_HOURS);

        // Update last active timestamp
        self.data
            .database()
            .update_last_active(user.id)
            .await
            .map_err(|e| AppError::database(format!("Failed to update last active: {e}")))?;

        info!("Token refreshed successfully for user: {}", user.id);

        Ok(LoginResponse {
            jwt_token: Some(new_jwt_token),
            csrf_token: String::new(), // Will be set by HTTP handler
            expires_at: expires_at.to_rfc3339(),
            user: UserInfo {
                user_id: user.id.to_string(),
                email: user.email.clone(),
                display_name: user.display_name,
                is_admin: user.is_admin,
                role: user.role.as_str().to_owned(),
                user_status: user.user_status.to_string(),
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

/// Parsed OAuth state containing user ID and optional mobile redirect URL
struct ParsedOAuthState {
    user_id: uuid::Uuid,
    /// Optional redirect URL for mobile OAuth flows (base64 encoded in state)
    mobile_redirect_url: Option<String>,
    /// PKCE code verifier recovered from server-side state storage
    pkce_code_verifier: Option<String>,
    /// Tenant ID from the OAuth state, used for tenant-specific credential lookup
    tenant_id: Option<uuid::Uuid>,
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
    /// Validates the state parameter against server-side storage to prevent CSRF attacks,
    /// then exchanges the authorization code for tokens. Uses PKCE when the code verifier
    /// was stored with the state during authorization URL generation.
    ///
    /// # Errors
    /// Returns error if OAuth state is invalid/expired/reused or callback processing fails
    pub async fn handle_callback(
        &self,
        code: &str,
        state: &str,
        provider: &str,
    ) -> AppResult<OAuthCallbackResponse> {
        // Validate provider is supported before consuming state
        self.validate_provider(provider)?;

        // Consume state atomically from database (verifies it was server-issued,
        // not expired, not reused, and matches the expected provider)
        let parsed_state = self.consume_and_validate_state(state, provider).await?;
        let user_id = parsed_state.user_id;
        let mobile_redirect_url = parsed_state.mobile_redirect_url;
        let pkce_code_verifier = parsed_state.pkce_code_verifier;
        let state_tenant_id = parsed_state.tenant_id;

        info!(
            "Processing OAuth callback for user {} provider {}{}",
            user_id,
            provider,
            if mobile_redirect_url.is_some() {
                " (mobile flow)"
            } else {
                ""
            }
        );

        // Get user and tenant from database
        let (.., tenant_id) = self.get_user_and_tenant(user_id, provider).await?;

        // Exchange OAuth code for access token (with PKCE if verifier was stored)
        // Pass tenant_id from state so exchange uses tenant-specific credentials if available
        let token = self
            .exchange_oauth_code(
                code,
                provider,
                user_id,
                pkce_code_verifier.as_deref(),
                state_tenant_id,
            )
            .await?;

        info!(
            "Successfully exchanged OAuth code for user {} provider {}",
            user_id, provider
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
            mobile_redirect_url,
        })
    }

    /// Consume and validate OAuth state from server-side storage
    ///
    /// Atomically verifies the state was issued by this server, has not expired,
    /// and has not been used before (one-time use). Uses the provider name as the
    /// `client_id` for additional validation that the callback matches the initiated flow.
    ///
    /// State format: `{user_id}:{random}` or `{user_id}:{random}:{base64_redirect_url}`
    /// The redirect URL allows mobile apps to specify where to redirect after OAuth completes.
    async fn consume_and_validate_state(
        &self,
        state: &str,
        provider: &str,
    ) -> AppResult<ParsedOAuthState> {
        // Atomically consume the state from database (marks as used, checks expiry)
        let consumed = self
            .data
            .database()
            .consume_oauth_client_state(state, provider, Utc::now())
            .await
            .map_err(|e| {
                warn!("Failed to consume OAuth state from database: {}", e);
                AppError::auth_invalid("OAuth state validation failed")
            })?;

        let client_state = consumed.ok_or_else(|| {
            warn!(
                "OAuth state not found, expired, or already used for provider {}",
                provider
            );
            AppError::auth_invalid("Invalid, expired, or already used OAuth state parameter")
        })?;

        let user_id = client_state.user_id.ok_or_else(|| {
            error!("OAuth state missing user_id for provider {}", provider);
            AppError::auth_invalid("OAuth state missing user identity")
        })?;

        // Extract optional mobile redirect URL from the state string
        // (embedded as base64 in the third segment of the state format)
        let mobile_redirect_url = Self::extract_mobile_redirect_from_state_str(state);

        // PKCE code verifier stored server-side during authorization URL generation
        let pkce_code_verifier = client_state.pkce_code_verifier;

        // Parse tenant_id from the stored OAuth client state for credential lookup
        let tenant_id = client_state
            .tenant_id
            .as_deref()
            .and_then(|tid| uuid::Uuid::parse_str(tid).ok());

        Ok(ParsedOAuthState {
            user_id,
            mobile_redirect_url,
            pkce_code_verifier,
            tenant_id,
        })
    }

    /// Extract mobile redirect URL from state string format
    ///
    /// State format: `{user_id}:{random}:{base64_redirect_url}`
    fn extract_mobile_redirect_from_state_str(state: &str) -> Option<String> {
        let parts: Vec<&str> = state.splitn(3, ':').collect();
        parts
            .get(2)
            .filter(|s| !s.is_empty())
            .and_then(|encoded| Self::decode_mobile_redirect_url(encoded))
    }

    /// Decode and validate a base64-encoded mobile redirect URL
    fn decode_mobile_redirect_url(encoded: &str) -> Option<String> {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

        URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|e| {
                warn!("Failed to decode base64 redirect URL: {}", e);
                e
            })
            .ok()
            .and_then(|bytes| {
                String::from_utf8(bytes)
                    .map_err(|e| {
                        warn!("Failed to decode redirect URL as UTF-8: {}", e);
                        e
                    })
                    .ok()
            })
            .and_then(|url| {
                // Validate URL scheme for security (only allow specific schemes)
                if url.starts_with("pierre://")
                    || url.starts_with("exp://")
                    || url.starts_with("http://localhost")
                    || url.starts_with("https://")
                {
                    Some(url)
                } else {
                    warn!("Invalid redirect URL scheme in OAuth state: {}", url);
                    None
                }
            })
    }

    /// Validate that provider is supported by checking the provider registry
    fn validate_provider(&self, provider: &str) -> AppResult<()> {
        if self.data.provider_registry().is_supported(provider) {
            Ok(())
        } else {
            Err(AppError::invalid_input(format!(
                "Unsupported provider: {provider}"
            )))
        }
    }

    /// Get user and tenant from database
    ///
    /// Tenant is determined from the `tenant_users` junction table.
    async fn get_user_and_tenant(
        &self,
        user_id: uuid::Uuid,
        provider: &str,
    ) -> AppResult<(User, String)> {
        let database = self.data.database();
        let user = database
            .get_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user: {e}")))?
            .ok_or_else(|| {
                error!(
                    "OAuth callback failed: User not found - user_id: {}, provider: {}",
                    user_id, provider
                );
                AppError::not_found("User")
            })?;

        // Get tenant from tenant_users table (user's default/first tenant)
        let tenants = database
            .list_tenants_for_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user tenants: {e}")))?;

        let tenant_id = tenants.first().map(|t| t.id.to_string()).ok_or_else(|| {
            error!(
                user_id = %user.id,
                provider = %provider,
                "OAuth callback failed: user has no tenant"
            );
            AppError::invalid_input("User has no tenant")
        })?;

        Ok((user, tenant_id))
    }

    /// Exchange OAuth code for access token, using PKCE when a code verifier is available
    ///
    /// When `tenant_id` is provided, attempts to use tenant-specific OAuth credentials
    /// (`client_id`, `client_secret`) before falling back to environment configuration.
    async fn exchange_oauth_code(
        &self,
        code: &str,
        provider: &str,
        user_id: uuid::Uuid,
        pkce_code_verifier: Option<&str>,
        tenant_id: Option<uuid::Uuid>,
    ) -> AppResult<OAuth2Token> {
        let oauth_config = self
            .create_oauth_config_with_tenant(provider, tenant_id)
            .await?;
        let oauth_client = OAuth2Client::new(oauth_config);

        let token = if let Some(verifier) = pkce_code_verifier {
            // Use PKCE-enhanced token exchange when verifier was stored with the state
            let pkce = PkceParams {
                code_verifier: verifier.to_owned(),
                code_challenge: String::new(),
                code_challenge_method: "S256".to_owned(),
            };
            oauth_client
                .exchange_code_with_pkce(code, &pkce)
                .await
                .map_err(|e| {
                    error!(
                        "OAuth PKCE token exchange failed for {provider} - user_id: {user_id}, error: {e}",
                    );
                    AppError::internal(format!("Failed to exchange OAuth code for token: {e}"))
                })?
        } else {
            oauth_client.exchange_code(code).await.map_err(|e| {
                error!(
                    "OAuth token exchange failed for {provider} - user_id: {user_id}, error: {e}",
                );
                AppError::internal(format!("Failed to exchange OAuth code for token: {e}"))
            })?
        };

        Ok(token)
    }

    /// Create `OAuth2` config for provider using descriptor and configuration
    ///
    /// # Errors
    /// Returns error if provider is unsupported or required credentials are not configured
    fn create_oauth_config(&self, provider: &str) -> AppResult<OAuth2Config> {
        // Get provider descriptor from registry
        let descriptor = self
            .data
            .provider_registry()
            .get_descriptor(provider)
            .ok_or_else(|| AppError::invalid_input(format!("Unsupported provider: {provider}")))?;

        // Get OAuth endpoints from descriptor
        let endpoints = descriptor.oauth_endpoints().ok_or_else(|| {
            AppError::invalid_input(format!("Provider {provider} does not support OAuth"))
        })?;

        // Get OAuth params from descriptor
        let params = descriptor.oauth_params().ok_or_else(|| {
            AppError::invalid_input(format!("Provider {provider} OAuth params not configured"))
        })?;

        // Get credentials from environment/config
        let env_config = get_oauth_config(provider);
        let client_id = env_config.client_id.ok_or_else(|| {
            AppError::invalid_input(format!(
                "{provider} client_id not configured for token exchange"
            ))
        })?;
        let client_secret = env_config.client_secret.ok_or_else(|| {
            AppError::invalid_input(format!(
                "{provider} client_secret not configured for token exchange"
            ))
        })?;

        // Build redirect URI - use BASE_URL if set for tunnel/external access
        let server_config = self.config.config();
        let redirect_uri = env_config.redirect_uri.unwrap_or_else(|| {
            let base_url = env::var("BASE_URL")
                .unwrap_or_else(|_| format!("http://localhost:{}", server_config.http_port));
            format!("{base_url}/api/oauth/callback/{provider}")
        });

        // Get default scopes and join with provider's separator
        let scopes = descriptor
            .default_scopes()
            .iter()
            .map(|s| (*s).to_owned())
            .collect::<Vec<_>>()
            .join(params.scope_separator);

        Ok(OAuth2Config {
            client_id,
            client_secret,
            auth_url: endpoints.auth_url.to_owned(),
            token_url: endpoints.token_url.to_owned(),
            redirect_uri,
            scopes: vec![scopes],
            use_pkce: params.use_pkce,
        })
    }

    /// Create `OAuth2` config using tenant-specific credentials when available
    ///
    /// Looks up tenant credentials from the database when `tenant_id` is provided.
    /// Falls back to environment-based configuration if no tenant credentials are found
    /// or if `tenant_id` is None.
    ///
    /// # Errors
    /// Returns error if provider is unsupported or no credentials are configured
    async fn create_oauth_config_with_tenant(
        &self,
        provider: &str,
        tenant_id: Option<uuid::Uuid>,
    ) -> AppResult<OAuth2Config> {
        // Try tenant-specific credentials first
        if let Some(tid) = tenant_id {
            let tenant_creds = self
                .data
                .database()
                .get_tenant_oauth_credentials(tid, provider)
                .await
                .map_err(|e| {
                    warn!(
                        "Failed to fetch tenant OAuth credentials for tenant {tid}, provider {provider}: {e}"
                    );
                    AppError::database(format!(
                        "Failed to fetch tenant OAuth credentials: {e}"
                    ))
                })?;

            if let Some(creds) = tenant_creds {
                debug!(
                    "Using tenant-specific OAuth credentials for tenant {tid}, provider {provider}"
                );

                // Get provider descriptor for endpoints and params
                let descriptor = self
                    .data
                    .provider_registry()
                    .get_descriptor(provider)
                    .ok_or_else(|| {
                        AppError::invalid_input(format!("Unsupported provider: {provider}"))
                    })?;

                let endpoints = descriptor.oauth_endpoints().ok_or_else(|| {
                    AppError::invalid_input(format!("Provider {provider} does not support OAuth"))
                })?;

                let params = descriptor.oauth_params().ok_or_else(|| {
                    AppError::invalid_input(format!(
                        "Provider {provider} OAuth params not configured"
                    ))
                })?;

                let scopes = creds.scopes.join(params.scope_separator);

                return Ok(OAuth2Config {
                    client_id: creds.client_id,
                    client_secret: creds.client_secret,
                    auth_url: endpoints.auth_url.to_owned(),
                    token_url: endpoints.token_url.to_owned(),
                    redirect_uri: creds.redirect_uri,
                    scopes: vec![scopes],
                    use_pkce: params.use_pkce,
                });
            }
        }

        // Fall back to environment-based configuration
        self.create_oauth_config(provider)
    }

    /// Store OAuth token in database
    async fn store_oauth_token(
        &self,
        user_id: uuid::Uuid,
        tenant_id: String,
        provider: &str,
        token: &OAuth2Token,
    ) -> AppResult<chrono::DateTime<chrono::Utc>> {
        let expires_at = token
            .expires_at
            .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::hours(1));

        let user_oauth_token = UserOAuthToken {
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

        // Register provider connection alongside the OAuth token
        self.data
            .database()
            .register_provider_connection(
                user_id,
                &user_oauth_token.tenant_id,
                provider,
                &ConnectionType::OAuth,
                None,
            )
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to register provider connection: {e}"))
            })?;

        Ok(expires_at)
    }

    /// Send OAuth completion notifications
    async fn send_oauth_notifications(
        &self,
        user_id: uuid::Uuid,
        provider: &str,
        expires_at: &chrono::DateTime<chrono::Utc>,
    ) -> AppResult<()> {
        let notification_id = self
            .store_oauth_notification(user_id, provider, expires_at)
            .await?;
        self.broadcast_oauth_notification(&notification_id, user_id, provider);
        Ok(())
    }

    /// Store OAuth notification in database
    async fn store_oauth_notification(
        &self,
        user_id: uuid::Uuid,
        provider: &str,
        expires_at: &chrono::DateTime<chrono::Utc>,
    ) -> AppResult<String> {
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

        info!(
            "Created OAuth completion notification {} for user {} provider {}",
            notification_id, user_id, provider
        );

        Ok(notification_id)
    }

    /// Broadcast OAuth completion notification via WebSocket/SSE
    fn broadcast_oauth_notification(
        &self,
        notification_id: &str,
        user_id: uuid::Uuid,
        provider: &str,
    ) {
        let Some(sender) = self.notifications.oauth_notification_sender() else {
            debug!(
                notification_id = %notification_id,
                user_id = %user_id,
                provider = %provider,
                "OAuth notification sender not configured"
            );
            return;
        };

        let notification = OAuthCompletedNotification::new(
            provider.to_owned(),
            true,
            format!("{provider} connected successfully"),
            Some(user_id.to_string()),
        );

        match sender.send(notification) {
            Ok(receiver_count) => {
                info!(
                    notification_id = %notification_id,
                    user_id = %user_id,
                    provider = %provider,
                    receiver_count = %receiver_count,
                    "OAuth notification broadcast to {} receivers",
                    receiver_count
                );
            }
            Err(e) => {
                debug!(
                    notification_id = %notification_id,
                    user_id = %user_id,
                    provider = %provider,
                    error = %e,
                    "No active receivers for OAuth notification"
                );
            }
        }
    }

    /// Build OAuth token data for bridge notification
    fn build_bridge_token_data(token: &OAuth2Token) -> JsonValue {
        // Calculate expires_in from expires_at if available
        let expires_in = token.expires_at.map(|expires_at| {
            let duration = expires_at - chrono::Utc::now();
            duration.num_seconds().max(0)
        });

        json!({
            "access_token": token.access_token,
            "refresh_token": token.refresh_token,
            "expires_in": expires_in,
            "token_type": token.token_type,
            "scope": token.scope
        })
    }

    /// Log bridge notification response
    fn log_bridge_notification_result(
        result: Result<reqwest::Response, reqwest::Error>,
        provider: &str,
    ) {
        match result {
            Ok(response) if response.status().is_success() => {
                info!(
                    "✅ Successfully notified bridge about {} OAuth completion",
                    provider
                );
            }
            Ok(response) => {
                warn!(
                    "Bridge notification responded with status {} for provider {}",
                    response.status(),
                    provider
                );
            }
            Err(e) => {
                warn!(
                    "Failed to notify bridge about {} OAuth (bridge may not be running): {}",
                    provider, e
                );
            }
        }
    }

    /// Notify bridge about successful OAuth (for client-side token storage and focus recovery)
    async fn notify_bridge_oauth_success(&self, provider: &str, token: &OAuth2Token) {
        let oauth_callback_port = self.config.config().oauth_callback_port;
        let callback_url =
            format!("http://localhost:{oauth_callback_port}/oauth/provider-callback/{provider}");

        let token_data = Self::build_bridge_token_data(token);

        debug!(
            "Notifying bridge about {} OAuth success at {}",
            provider, callback_url
        );

        // Best-effort notification with configured timeout - don't fail OAuth flow if bridge notification fails
        // Configuration must be initialized via initialize_http_clients() at server startup
        let timeout_secs = get_oauth_callback_notification_timeout_secs();
        let result = reqwest::Client::new()
            .post(&callback_url)
            .json(&token_data)
            .timeout(StdDuration::from_secs(timeout_secs))
            .send()
            .await;

        Self::log_bridge_notification_result(result, provider);
    }

    /// Disconnect OAuth provider for user
    ///
    /// # Errors
    /// Returns error if provider is unsupported or disconnection fails
    pub async fn disconnect_provider(&self, user_id: uuid::Uuid, provider: &str) -> AppResult<()> {
        debug!(
            "Processing OAuth provider disconnect for user {} provider {}",
            user_id, provider
        );

        // Validate provider is supported
        self.validate_provider(provider)?;

        // Get user's default tenant from tenant_users table
        let tenants = self
            .data
            .database()
            .list_tenants_for_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user tenants: {e}")))?;

        let tenant_id = tenants.first().map(|t| t.id.to_string()).ok_or_else(|| {
            AppError::auth_invalid("User has no tenant association — cannot disconnect provider")
        })?;

        // Delete OAuth tokens from database
        self.data
            .database()
            .delete_user_oauth_token(user_id, &tenant_id, provider)
            .await
            .map_err(|e| AppError::database(format!("Failed to delete OAuth token: {e}")))?;

        // Remove provider connection record
        self.data
            .database()
            .remove_provider_connection(user_id, &tenant_id, provider)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to remove provider connection: {e}"))
            })?;

        info!("Disconnected {} for user {}", provider, user_id);

        Ok(())
    }

    /// Generate OAuth authorization URL for provider
    ///
    /// This function supports both multi-tenant and single-tenant modes:
    /// - Multi-tenant: Uses tenant-specific OAuth credentials from database
    /// - Single-tenant: Falls back to server-level configuration
    ///
    /// Stores the OAuth state server-side with TTL for CSRF protection, and generates
    /// PKCE parameters when the provider declares `use_pkce=true`.
    ///
    /// # Errors
    /// Returns error if provider is unsupported or OAuth credentials not configured
    pub async fn get_auth_url(
        &self,
        user_id: uuid::Uuid,
        tenant_id: uuid::Uuid,
        provider: &str,
    ) -> AppResult<OAuthAuthorizationResponse> {
        // Get provider descriptor from registry
        let descriptor = self
            .data
            .provider_registry()
            .get_descriptor(provider)
            .ok_or_else(|| AppError::invalid_input(format!("Unsupported provider: {provider}")))?;

        // Get OAuth endpoints and params from descriptor
        let endpoints = descriptor.oauth_endpoints().ok_or_else(|| {
            AppError::invalid_input(format!("Provider {provider} does not support OAuth"))
        })?;
        let params = descriptor.oauth_params().ok_or_else(|| {
            AppError::invalid_input(format!("Provider {provider} OAuth params not configured"))
        })?;

        let use_pkce = params.use_pkce;

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
        // Use BASE_URL environment variable if set, otherwise fall back to localhost.
        // This allows dynamic OAuth callbacks when using tunnels for local development.
        let base_url = env::var("BASE_URL")
            .unwrap_or_else(|_| format!("http://localhost:{}", self.config.config().http_port));
        let redirect_uri = format!("{base_url}/api/oauth/callback/{provider}");

        // Generate PKCE parameters when provider supports it
        let pkce = if use_pkce {
            Some(PkceParams::generate())
        } else {
            None
        };

        // URL-encode parameters for OAuth URLs
        let encoded_state = encode(&state);
        let encoded_redirect_uri = encode(&redirect_uri);

        // Determine client_id and scopes (tenant-specific or environment)
        let (client_id, scope) = if let Some(creds) = tenant_creds {
            // Multi-tenant: use tenant-specific credentials
            let scope = creds.scopes.join(params.scope_separator);
            (creds.client_id, scope)
        } else {
            // Single-tenant: use environment configuration
            let env_config = get_oauth_config(provider);
            let client_id = env_config.client_id.ok_or_else(|| {
                AppError::invalid_input(format!(
                    "{provider} client_id not configured (set in environment or database)"
                ))
            })?;
            let scope = descriptor.default_scopes().join(params.scope_separator);
            (client_id, scope)
        };

        let encoded_scope = encode(&scope);

        // Build authorization URL with provider-specific parameters
        let mut auth_url = format!(
            "{}?client_id={}&response_type=code&redirect_uri={}&scope={}&state={}",
            endpoints.auth_url, client_id, encoded_redirect_uri, encoded_scope, encoded_state
        );

        // Add PKCE code_challenge to authorization URL when enabled
        if let Some(ref pkce_params) = pkce {
            use Write;
            let _ = write!(
                &mut auth_url,
                "&code_challenge={}&code_challenge_method={}",
                encode(&pkce_params.code_challenge),
                encode(&pkce_params.code_challenge_method)
            );
        }

        // Add provider-specific additional parameters
        for (key, value) in params.additional_auth_params {
            use Write;
            // Writing to String cannot fail
            let _ = write!(&mut auth_url, "&{}={}", encode(key), encode(value));
        }

        let authorization_url = auth_url;

        // Store state server-side for CSRF protection with 10-minute TTL.
        // The code_challenge field stores the PKCE code_verifier (needed during
        // token exchange to prove we initiated the authorization request).
        let now = Utc::now();
        let client_state = OAuthClientState {
            state: state.clone(),
            provider: provider.to_owned(),
            user_id: Some(user_id),
            tenant_id: Some(tenant_id.to_string()),
            redirect_uri,
            scope: Some(scope),
            pkce_code_verifier: pkce.as_ref().map(|p| p.code_verifier.clone()),
            created_at: now,
            expires_at: now + chrono::Duration::minutes(10),
            used: false,
        };

        self.data
            .database()
            .store_oauth_client_state(&client_state)
            .await
            .map_err(|e| {
                error!("Failed to store OAuth state for CSRF protection: {}", e);
                AppError::internal(format!("Failed to initiate OAuth flow: {e}"))
            })?;

        debug!(
            "Generated OAuth authorization URL for user {} tenant {} provider {}",
            user_id, tenant_id, provider
        );

        Ok(OAuthAuthorizationResponse {
            authorization_url,
            state,
            instructions: format!("Click the link to authorize {provider} access"),
            expires_in_minutes: 10,
        })
    }

    /// Get connection status for all providers for a user
    ///
    /// Uses `provider_connections` table as the single source of truth.
    /// For OAuth connections, also looks up token expiry and scope info.
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_connection_status(
        &self,
        user_id: uuid::Uuid,
    ) -> AppResult<Vec<ConnectionStatus>> {
        debug!("Getting provider connection status for user {}", user_id);

        // Get all provider connections (cross-tenant view)
        let connections = self
            .data
            .database()
            .get_user_provider_connections(user_id, None)
            .await
            .map_err(|e| AppError::database(format!("Failed to get provider connections: {e}")))?;

        // For OAuth connections, look up token expiry/scope info
        let oauth_tokens = self
            .data
            .database()
            .get_user_oauth_tokens(user_id, None)
            .await
            .unwrap_or_default();

        let token_map: HashMap<String, &UserOAuthToken> = oauth_tokens
            .iter()
            .map(|t| (t.provider.clone(), t))
            .collect();

        let mut providers_seen = HashSet::new();
        let mut statuses = Vec::new();

        // Build status for each connected provider
        for conn in &connections {
            if providers_seen.insert(conn.provider.clone()) {
                let (expires_at, scopes) = if conn.connection_type == ConnectionType::OAuth {
                    // Look up OAuth token details for expiry/scope info
                    token_map.get(&conn.provider).map_or((None, None), |t| {
                        (t.expires_at.map(|dt| dt.to_rfc3339()), t.scope.clone())
                    })
                } else {
                    (None, None)
                };

                statuses.push(ConnectionStatus {
                    provider: conn.provider.clone(),
                    connected: true,
                    connection_type: Some(conn.connection_type.as_str().to_owned()),
                    expires_at,
                    scopes,
                });
            }
        }

        // Add default disconnected status for all registered OAuth providers not in connections
        for provider_name in self.data.provider_registry().oauth_providers() {
            if !providers_seen.contains(provider_name) {
                statuses.push(ConnectionStatus {
                    provider: provider_name.to_owned(),
                    connected: false,
                    connection_type: None,
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

/// Authentication routes implementation
#[derive(Clone)]

/// Authentication routes implementation (Axum)
///
/// Provides user registration, login, logout, and OAuth client authentication endpoints.
pub struct AuthRoutes;

impl AuthRoutes {
    /// Create all authentication routes (Axum)
    pub fn routes(resources: Arc<ServerResources>) -> Router {
        use axum::{
            routing::{delete, get, post, put},
            Router,
        };

        Router::new()
            .route("/api/auth/register", post(Self::handle_public_register))
            .route("/api/auth/admin/register", post(Self::handle_register))
            .route("/api/auth/firebase", post(Self::handle_firebase_login))
            .route("/api/auth/logout", post(Self::handle_logout))
            .route("/api/auth/session", get(Self::handle_session))
            .route("/api/auth/refresh", post(Self::handle_refresh))
            .route("/api/user/profile", put(Self::handle_update_profile))
            .route(
                "/api/user/change-password",
                put(Self::handle_change_password),
            )
            .route(
                "/api/auth/complete-reset",
                post(Self::handle_complete_reset),
            )
            .route("/api/user/stats", get(Self::handle_user_stats))
            // OAuth2 ROPC endpoint (RFC 6749 Section 4.3) - unified login for all clients
            .route("/oauth/token", post(Self::handle_oauth2_token))
            .route(
                "/api/oauth/callback/:provider",
                get(Self::handle_oauth_callback),
            )
            .route("/api/oauth/status", get(Self::handle_oauth_status))
            .route("/api/providers", get(Self::handle_providers_status))
            .route(
                "/api/oauth/auth/:provider/:user_id",
                get(Self::handle_oauth_auth_initiate),
            )
            // Mobile OAuth initiation - returns OAuth URL in JSON (requires auth)
            .route(
                "/api/oauth/mobile/init/:provider",
                get(Self::handle_mobile_oauth_init),
            )
            // Disconnect a provider (requires auth)
            .route(
                "/api/oauth/providers/:provider/disconnect",
                delete(Self::handle_disconnect_provider_rest),
            )
            .with_state(resources)
    }

    /// Handle user registration (Axum)
    ///
    /// REQUIRES: Admin authentication (Bearer token in Authorization header)
    ///
    /// Security: Only administrators can create new users to prevent
    /// unauthorized user creation, database pollution, and `DoS` attacks.
    #[tracing::instrument(
        skip(resources, headers, request),
        fields(
            route = "admin_register",
            user_id = Empty,
            success = Empty,
        )
    )]
    async fn handle_register(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Json(request): Json<RegisterRequest>,
    ) -> Result<Response, AppError> {
        // Extract and validate admin token
        let auth_header = headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| {
                AppError::auth_invalid(
                    "Missing Authorization header for user registration - admin token required",
                )
            })?;

        let token = extract_bearer_token_owned(auth_header)
            .map_err(|_| AppError::auth_invalid("Invalid Authorization header format"))?;

        // Validate admin token
        let admin_auth_service = AdminAuthService::new(
            resources.database.as_ref().clone(),
            resources.jwks_manager.clone(),
            resources.config.auth.admin_token_cache_ttl_secs,
        );

        // Authenticate admin (no specific permission check - any valid admin token can register users)
        admin_auth_service
            .authenticate(&token, None)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to authenticate admin token for user registration");
                AppError::auth_invalid(format!("Admin authentication failed: {e}"))
            })?;

        info!("Admin-authenticated user registration attempt");

        let server_context = ServerContext::from(resources.as_ref());
        let auth_routes = AuthService::new(
            server_context.auth().clone(),
            server_context.config().clone(),
            server_context.data().clone(),
        );

        match auth_routes.register(request).await {
            Ok(response) => Ok((StatusCode::CREATED, Json(response)).into_response()),
            Err(e) => {
                error!("Registration failed: {}", e);
                Err(e)
            }
        }
    }

    /// Handle public user self-registration (Axum)
    ///
    /// This endpoint allows users to register themselves without admin authentication.
    /// New users are created in "Pending" status by default and require admin approval,
    /// unless `AUTO_APPROVE_USERS` environment variable is set to true.
    #[tracing::instrument(
        skip(resources, request),
        fields(
            route = "public_register",
            user_id = Empty,
            success = Empty,
        )
    )]
    async fn handle_public_register(
        State(resources): State<Arc<ServerResources>>,
        Json(request): Json<RegisterRequest>,
    ) -> Result<Response, AppError> {
        info!("Public self-registration attempt");

        let server_context = ServerContext::from(resources.as_ref());
        let auth_routes = AuthService::new(
            server_context.auth().clone(),
            server_context.config().clone(),
            server_context.data().clone(),
        );

        match auth_routes.register(request).await {
            Ok(response) => Ok((StatusCode::CREATED, Json(response)).into_response()),
            Err(e) => {
                error!("Public registration failed: {}", e);
                Err(e)
            }
        }
    }

    /// Handle Firebase authentication login (Axum)
    ///
    /// Authenticates users via Firebase ID tokens (Google Sign-In, Apple, etc.)
    #[tracing::instrument(
        skip(resources, request),
        fields(
            route = "firebase_login",
            user_id = Empty,
            auth_provider = Empty,
            success = Empty,
        )
    )]
    async fn handle_firebase_login(
        State(resources): State<Arc<ServerResources>>,
        Json(request): Json<FirebaseLoginRequest>,
    ) -> Result<Response, AppError> {
        // Check if Firebase is configured
        let firebase_auth = resources.firebase_auth.as_ref().ok_or_else(|| {
            AppError::invalid_input("Firebase authentication is not configured on this server")
        })?;

        let server_context = ServerContext::from(resources.as_ref());
        let auth_service = AuthService::new(
            server_context.auth().clone(),
            server_context.config().clone(),
            server_context.data().clone(),
        );

        match auth_service
            .login_with_firebase(request, firebase_auth)
            .await
        {
            Ok(mut response) => {
                // Clone JWT for cookie (also included in JSON response for API clients)
                let jwt_token = response
                    .jwt_token
                    .clone() // Safe: JWT string ownership for cookie
                    .ok_or_else(|| AppError::internal("JWT token missing from login response"))?;

                // Parse user ID for CSRF token generation
                let user_id = uuid::Uuid::parse_str(&response.user.user_id)
                    .map_err(|e| AppError::internal(format!("Invalid user ID format: {e}")))?;

                // Generate CSRF token
                let csrf_token = resources
                    .csrf_manager
                    .generate_token(user_id)
                    .await
                    .map_err(|e| {
                        AppError::internal(format!("Failed to generate CSRF token: {e}"))
                    })?;

                // Set response CSRF token
                response.csrf_token.clone_from(&csrf_token);

                // Build response with secure cookies
                let mut headers = HeaderMap::new();

                // Set httpOnly auth cookie (24 hour expiry to match JWT)
                set_auth_cookie(&mut headers, &jwt_token, 24 * 60 * 60);

                // Set CSRF cookie (30 minute expiry to match CSRF token)
                set_csrf_cookie(&mut headers, &csrf_token, 30 * 60);

                Ok((StatusCode::OK, headers, Json(response)).into_response())
            }
            Err(e) => {
                tracing::error!("Firebase login failed: {}", e);
                Err(e)
            }
        }
    }

    /// Handle token refresh (Axum)
    #[tracing::instrument(
        skip(resources, request),
        fields(
            route = "token_refresh",
            user_id = %request.user_id,
            success = Empty,
        )
    )]
    async fn handle_refresh(
        State(resources): State<Arc<ServerResources>>,
        Json(request): Json<RefreshTokenRequest>,
    ) -> Result<Response, AppError> {
        let server_context = ServerContext::from(resources.as_ref());
        let auth_service = AuthService::new(
            server_context.auth().clone(),
            server_context.config().clone(),
            server_context.data().clone(),
        );

        match auth_service.refresh_token(request).await {
            Ok(mut response) => {
                // Clone JWT for cookie (also included in JSON response for API clients)
                let jwt_token = response
                    .jwt_token
                    .clone() // Safe: JWT string ownership for cookie
                    .ok_or_else(|| AppError::internal("JWT token missing from refresh response"))?;

                // Parse user ID for CSRF token generation
                let user_id = uuid::Uuid::parse_str(&response.user.user_id)
                    .map_err(|e| AppError::internal(format!("Invalid user ID format: {e}")))?;

                // Generate new CSRF token
                let csrf_token = resources
                    .csrf_manager
                    .generate_token(user_id)
                    .await
                    .map_err(|e| {
                        AppError::internal(format!("Failed to generate CSRF token: {e}"))
                    })?;

                // Set response CSRF token
                response.csrf_token.clone_from(&csrf_token);

                // Build response with secure cookies
                let mut headers = HeaderMap::new();

                // Set httpOnly auth cookie (24 hour expiry to match JWT)
                set_auth_cookie(&mut headers, &jwt_token, 24 * 60 * 60);

                // Set CSRF cookie (30 minute expiry to match CSRF token)
                set_csrf_cookie(&mut headers, &csrf_token, 30 * 60);

                Ok((StatusCode::OK, headers, Json(response)).into_response())
            }
            Err(e) => {
                error!("Token refresh failed: {}", e);
                Err(e)
            }
        }
    }

    /// Handle user logout (Axum)
    async fn handle_logout() -> Result<Response, AppError> {
        // Yield to allow async context (required for Axum handler)
        task::yield_now().await;

        // Build response with cleared cookies
        let mut headers = HeaderMap::new();

        // Clear auth cookie
        clear_auth_cookie(&mut headers);

        // Return success response
        Ok((
            StatusCode::OK,
            headers,
            Json(json!({
                "message": "Logged out successfully"
            })),
        )
            .into_response())
    }

    /// Restore session from httpOnly cookie authentication
    ///
    /// Returns the authenticated user's info along with a fresh JWT (for WebSocket auth)
    /// and CSRF token. This allows the frontend to restore sessions on page refresh
    /// without storing JWT tokens in localStorage.
    #[tracing::instrument(
        skip(resources, headers),
        fields(
            route = "session",
            user_id = Empty,
            success = Empty,
        )
    )]
    async fn handle_session(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        // Authenticate using cookie or Authorization header
        let auth_result = resources
            .auth_middleware
            .authenticate_request_with_headers(&headers)
            .await?;

        let user_id = auth_result.user_id;
        Span::current().record("user_id", user_id.to_string());

        // Look up user details from database
        let user = resources
            .database
            .get_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to fetch user: {e}")))?
            .ok_or_else(|| AppError::not_found(format!("User {user_id}")))?;

        // Generate a fresh JWT token for WebSocket authentication
        let server_context = ServerContext::from(resources.as_ref());
        let jwt_token = server_context
            .auth()
            .auth_manager()
            .generate_token(&user, server_context.auth().jwks_manager())
            .map_err(|e| AppError::auth_invalid(format!("Failed to generate token: {e}")))?;

        // Generate fresh CSRF token
        let csrf_token = resources
            .csrf_manager
            .generate_token(user_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to generate CSRF token: {e}")))?;

        // Refresh the httpOnly auth cookie with the new JWT
        let mut response_headers = HeaderMap::new();
        set_auth_cookie(&mut response_headers, &jwt_token, 24 * 60 * 60);
        set_csrf_cookie(&mut response_headers, &csrf_token, 30 * 60);

        Span::current().record("success", true);
        info!("Session restored for user: {}", user_id);

        let session_response = SessionResponse {
            user: UserInfo {
                user_id: user.id.to_string(),
                email: user.email.clone(),
                display_name: user.display_name,
                is_admin: user.is_admin,
                role: user.role.as_str().to_owned(),
                user_status: user.user_status.to_string(),
            },
            access_token: jwt_token,
            csrf_token,
        };

        Ok((StatusCode::OK, response_headers, Json(session_response)).into_response())
    }

    /// Handle user profile update (Axum)
    ///
    /// Updates the authenticated user's display name.
    /// Requires valid JWT authentication via cookie or Bearer token.
    #[tracing::instrument(
        skip(resources, headers, request),
        fields(
            route = "update_profile",
            success = Empty,
        )
    )]
    async fn handle_update_profile(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Json(request): Json<UpdateProfileRequest>,
    ) -> Result<Response, AppError> {
        // Extract JWT from cookie or Authorization header
        let auth_value =
            if let Some(auth_header) = headers.get("authorization").and_then(|h| h.to_str().ok()) {
                auth_header.to_owned()
            } else if let Some(token) = get_cookie_value(&headers, "auth_token") {
                // Fall back to auth_token cookie, format as Bearer token
                format!("Bearer {token}")
            } else {
                return Err(AppError::auth_invalid(
                    "Missing authorization header or cookie",
                ));
            };

        // Authenticate and get user ID
        let auth = resources
            .auth_middleware
            .authenticate_request(Some(&auth_value))
            .await
            .map_err(|e| AppError::auth_invalid(format!("Authentication failed: {e}")))?;

        let user_id = auth.user_id;

        // Validate display name
        let display_name = request.display_name.trim();
        if display_name.is_empty() {
            return Err(AppError::invalid_input("Display name cannot be empty"));
        }
        if display_name.len() > 100 {
            return Err(AppError::invalid_input(
                "Display name must be 100 characters or less",
            ));
        }

        // Update user in database
        let updated_user = resources
            .database
            .update_user_display_name(user_id, display_name)
            .await?;

        // Build response
        let response = UpdateProfileResponse {
            message: "Profile updated successfully".to_owned(),
            user: UserInfo {
                user_id: updated_user.id.to_string(),
                email: updated_user.email,
                display_name: updated_user.display_name,
                is_admin: updated_user.is_admin,
                role: updated_user.role.to_string(),
                user_status: updated_user.user_status.to_string(),
            },
        };

        info!(user_id = %user_id, "User profile updated successfully");

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle password change for authenticated users
    ///
    /// Verifies the current password, validates the new password,
    /// then hashes and stores the new password.
    #[tracing::instrument(
        skip(resources, headers, request),
        fields(
            route = "change_password",
            success = Empty,
        )
    )]
    async fn handle_change_password(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Json(request): Json<ChangePasswordRequest>,
    ) -> Result<Response, AppError> {
        // Extract JWT from cookie or Authorization header
        let auth_value =
            if let Some(auth_header) = headers.get("authorization").and_then(|h| h.to_str().ok()) {
                auth_header.to_owned()
            } else if let Some(token) = get_cookie_value(&headers, "auth_token") {
                format!("Bearer {token}")
            } else {
                return Err(AppError::auth_invalid(
                    "Missing authorization header or cookie",
                ));
            };

        // Authenticate and get user ID
        let auth = resources
            .auth_middleware
            .authenticate_request(Some(&auth_value))
            .await
            .map_err(|e| AppError::auth_invalid(format!("Authentication failed: {e}")))?;

        let user_id = auth.user_id;

        // Fetch user to get current password hash
        let user = resources
            .database
            .get_user(user_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("User {user_id}")))?;

        // Verify current password using spawn_blocking to avoid blocking async executor
        let current_password = request.current_password;
        let stored_hash = user.password_hash.clone();
        let is_valid =
            task::spawn_blocking(move || bcrypt::verify(&current_password, &stored_hash))
                .await
                .map_err(|e| AppError::internal(format!("Password verification task failed: {e}")))?
                .map_err(|_| AppError::auth_invalid("Current password is incorrect"))?;

        if !is_valid {
            return Err(AppError::auth_invalid("Current password is incorrect"));
        }

        // Validate new password strength
        if !AuthService::is_valid_password(&request.new_password) {
            return Err(AppError::invalid_input(error_messages::PASSWORD_TOO_WEAK));
        }

        // Hash new password using spawn_blocking
        let password_to_hash = request.new_password;
        let password_hash =
            task::spawn_blocking(move || bcrypt::hash(&password_to_hash, bcrypt::DEFAULT_COST))
                .await
                .map_err(|e| AppError::internal(format!("Password hashing task failed: {e}")))?
                .map_err(|e| AppError::internal(format!("Password hashing failed: {e}")))?;

        // Update password in database
        resources
            .database
            .update_user_password(user_id, &password_hash)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to update user password");
                AppError::internal(format!("Failed to update password: {e}"))
            })?;

        Span::current().record("success", true);
        info!(user_id = %user_id, "User password changed successfully");

        Ok((
            StatusCode::OK,
            Json(json!({ "message": "Password changed successfully" })),
        )
            .into_response())
    }

    /// Complete a password reset using a one-time token
    ///
    /// Public endpoint — no authentication required. The reset token acts as proof
    /// of authorization (issued by an admin). The user provides the token and their
    /// chosen new password. The token is consumed atomically to prevent replay.
    #[tracing::instrument(skip(resources, request), fields(route = "complete_reset"))]
    async fn handle_complete_reset(
        State(resources): State<Arc<ServerResources>>,
        Json(request): Json<CompleteResetRequest>,
    ) -> Result<Response, AppError> {
        use sha2::{Digest, Sha256};

        // Validate new password strength
        if !AuthService::is_valid_password(&request.new_password) {
            return Err(AppError::invalid_input(error_messages::PASSWORD_TOO_WEAK));
        }

        // Hash the presented token to match against stored hash
        let token_hash = format!("{:x}", Sha256::digest(request.reset_token.as_bytes()));

        // Atomically consume the token (validates existence, expiry, and single-use)
        let user_id = resources
            .database
            .consume_password_reset_token(&token_hash)
            .await?;

        // Hash the new password using spawn_blocking to avoid blocking async executor
        let password_to_hash = request.new_password;
        let password_hash =
            task::spawn_blocking(move || bcrypt::hash(&password_to_hash, bcrypt::DEFAULT_COST))
                .await
                .map_err(|e| AppError::internal(format!("Password hashing task failed: {e}")))?
                .map_err(|e| AppError::internal(format!("Password hashing failed: {e}")))?;

        // Update the user's password
        resources
            .database
            .update_user_password(user_id, &password_hash)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to update user password during reset");
                AppError::internal(format!("Failed to update password: {e}"))
            })?;

        // Invalidate any other outstanding reset tokens for this user
        resources
            .database
            .invalidate_user_reset_tokens(user_id)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to invalidate remaining reset tokens");
                AppError::internal(format!("Failed to cleanup reset tokens: {e}"))
            })?;

        info!(user_id = %user_id, "Password reset completed via one-time token");

        Ok((
            StatusCode::OK,
            Json(json!({ "message": "Password has been reset successfully" })),
        )
            .into_response())
    }

    /// Handle user stats request for dashboard
    ///
    /// Returns aggregated stats: connected providers, activities synced, and days active.
    #[tracing::instrument(
        skip(resources, headers),
        fields(
            route = "user_stats",
            user_id = Empty,
        )
    )]
    async fn handle_user_stats(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        // Extract JWT from cookie or Authorization header
        let auth_value =
            if let Some(auth_header) = headers.get("authorization").and_then(|h| h.to_str().ok()) {
                auth_header.to_owned()
            } else if let Some(token) = get_cookie_value(&headers, "auth_token") {
                format!("Bearer {token}")
            } else {
                return Err(AppError::auth_invalid(
                    "Missing authorization header or cookie",
                ));
            };

        // Authenticate and get user ID
        let auth = resources
            .auth_middleware
            .authenticate_request(Some(&auth_value))
            .await
            .map_err(|e| AppError::auth_invalid(format!("Authentication failed: {e}")))?;

        let user_id = auth.user_id;
        Span::current().record("user_id", user_id.to_string());

        // Get connected providers count from OAuth tokens (cross-tenant view for user stats)
        let oauth_tokens = resources
            .database
            .get_user_oauth_tokens(user_id, None)
            .await?;
        let connected_providers = i64::try_from(oauth_tokens.len()).unwrap_or(0);

        // Get user creation date to calculate days active
        let user = resources.database.get_user(user_id).await?;
        let days_active = match user {
            Some(u) => {
                let now = chrono::Utc::now();
                let duration = now.signed_duration_since(u.created_at);
                duration.num_days().max(1)
            }
            None => 1,
        };

        let response = UserStatsResponse {
            connected_providers,
            days_active,
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle `OAuth2` ROPC (Resource Owner Password Credentials) token request
    ///
    /// This endpoint implements RFC 6749 Section 4.3 for MCP and CLI clients
    /// that need to obtain tokens without a browser-based OAuth flow.
    ///
    /// Request format: `application/x-www-form-urlencoded`
    /// ```text
    /// grant_type=password&username=user@example.com&password=secret
    /// ```
    ///
    /// Response format: RFC 6749 Section 5.1 compliant JSON
    #[tracing::instrument(
        skip(resources, request),
        fields(
            route = "oauth2_token",
            grant_type = %request.grant_type,
            username = %request.username,
            user_id = Empty,
            success = Empty,
        )
    )]
    async fn handle_oauth2_token(
        State(resources): State<Arc<ServerResources>>,
        Form(request): Form<OAuth2TokenRequest>,
    ) -> Result<Response, AppError> {
        // Validate grant_type
        if request.grant_type != "password" {
            let error_response = OAuth2ErrorResponse {
                error: "unsupported_grant_type".to_owned(),
                error_description: Some(format!(
                    "Grant type '{}' is not supported. Use 'password' for ROPC.",
                    request.grant_type
                )),
            };
            return Ok((StatusCode::BAD_REQUEST, Json(error_response)).into_response());
        }

        // Delegate to existing login logic
        let login_request = LoginRequest {
            email: request.username,
            password: request.password,
        };

        let server_context = ServerContext::from(resources.as_ref());
        let auth_service = AuthService::new(
            server_context.auth().clone(),
            server_context.config().clone(),
            server_context.data().clone(),
        );

        match auth_service.login(login_request).await {
            Ok(response) => {
                let jwt_token = response
                    .jwt_token
                    .clone()
                    .ok_or_else(|| AppError::internal("JWT token missing from login response"))?;

                // Parse expiration to calculate expires_in
                let expires_at = chrono::DateTime::parse_from_rfc3339(&response.expires_at)
                    .map_or_else(
                        |_| chrono::Utc::now() + chrono::Duration::hours(24),
                        |dt| dt.with_timezone(&chrono::Utc),
                    );
                let expires_in = (expires_at - chrono::Utc::now()).num_seconds();

                // Generate CSRF token for web clients
                let user_id = uuid::Uuid::parse_str(&response.user.user_id)
                    .map_err(|e| AppError::internal(format!("Invalid user ID format: {e}")))?;
                let csrf_token = resources
                    .csrf_manager
                    .generate_token(user_id)
                    .await
                    .map_err(|e| {
                        AppError::internal(format!("Failed to generate CSRF token: {e}"))
                    })?;

                let oauth2_response = OAuth2TokenResponse {
                    access_token: jwt_token.clone(),
                    token_type: "Bearer".to_owned(),
                    expires_in,
                    refresh_token: None,
                    scope: request.scope,
                    // Pierre extensions for frontend compatibility
                    user: Some(response.user),
                    csrf_token: Some(csrf_token.clone()),
                };

                // Build response with secure cookies for web clients
                let mut headers = HeaderMap::new();
                set_auth_cookie(&mut headers, &jwt_token, 24 * 60 * 60);
                set_csrf_cookie(&mut headers, &csrf_token, 30 * 60);

                Ok((StatusCode::OK, headers, Json(oauth2_response)).into_response())
            }
            Err(e) => {
                // Map to OAuth2 error format based on error code
                let error_code = match e.code {
                    ErrorCode::AuthInvalid | ErrorCode::AuthRequired | ErrorCode::AuthExpired => {
                        "invalid_grant"
                    }
                    ErrorCode::PermissionDenied => "unauthorized_client",
                    ErrorCode::InvalidInput | ErrorCode::InvalidFormat => "invalid_request",
                    _ => "server_error",
                };
                let error_desc = e.message;

                let error_response = OAuth2ErrorResponse {
                    error: error_code.to_owned(),
                    error_description: Some(error_desc),
                };

                // OAuth2 spec: invalid_grant returns 400, server_error returns 500
                let status = if error_code == "server_error" {
                    StatusCode::INTERNAL_SERVER_ERROR
                } else {
                    StatusCode::BAD_REQUEST
                };

                Ok((status, Json(error_response)).into_response())
            }
        }
    }

    /// Handle OAuth callback (Axum)
    #[tracing::instrument(
        skip(resources, params),
        fields(
            route = "oauth_callback",
            provider = %provider,
            user_id = Empty,
            success = Empty,
        )
    )]
    async fn handle_oauth_callback(
        State(resources): State<Arc<ServerResources>>,
        Path(provider): Path<String>,
        Query(params): Query<HashMap<String, String>>,
    ) -> Result<Response, AppError> {
        let server_context = ServerContext::from(resources.as_ref());
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

        // Check if we should redirect to a separate frontend URL
        let frontend_url = server_context.config().config().frontend_url.clone();

        match oauth_routes.handle_callback(code, state, &provider).await {
            Ok(response) => {
                // Priority: mobile redirect URL > frontend URL > render template
                // Mobile apps pass redirect URL through OAuth state for deep linking
                if let Some(mobile_url) = &response.mobile_redirect_url {
                    let redirect_url = format!(
                        "{}?provider={}&success=true",
                        mobile_url.trim_end_matches('/'),
                        encode(&provider)
                    );
                    info!("Redirecting OAuth success to mobile app: {}", redirect_url);
                    return Ok(
                        (StatusCode::FOUND, [(header::LOCATION, redirect_url)], "").into_response()
                    );
                }

                // If frontend URL is configured, redirect to frontend with success params
                if let Some(url) = frontend_url {
                    let redirect_url = format!(
                        "{}/oauth-callback?provider={}&success=true",
                        url.trim_end_matches('/'),
                        encode(&provider)
                    );
                    info!("Redirecting OAuth success to frontend: {}", redirect_url);
                    return Ok(
                        (StatusCode::FOUND, [(header::LOCATION, redirect_url)], "").into_response()
                    );
                }

                // Otherwise serve the success page directly (same-origin production)
                let html = OAuthTemplateRenderer::render_success_template(&provider, &response);

                Ok((StatusCode::OK, [(header::CONTENT_TYPE, "text/html")], html).into_response())
            }
            Err(e) => {
                error!("OAuth callback failed: {}", e);

                // Determine error message and description based on error type
                let (error_msg, description) = Self::categorize_oauth_error(&e);

                // For errors, we need to parse the state to check for mobile redirect URL
                // since handle_callback failed and didn't return the parsed state
                let mobile_redirect_url = Self::extract_mobile_redirect_from_state(state);

                // Priority: mobile redirect URL > frontend URL > render template
                if let Some(mobile_url) = mobile_redirect_url {
                    let redirect_url = format!(
                        "{}?provider={}&success=false&error={}",
                        mobile_url.trim_end_matches('/'),
                        encode(&provider),
                        encode(error_msg)
                    );
                    info!("Redirecting OAuth error to mobile app: {}", redirect_url);
                    return Ok(
                        (StatusCode::FOUND, [(header::LOCATION, redirect_url)], "").into_response()
                    );
                }

                // If frontend URL is configured, redirect to frontend with error params
                if let Some(url) = frontend_url {
                    let redirect_url = format!(
                        "{}/oauth-callback?provider={}&success=false&error={}",
                        url.trim_end_matches('/'),
                        encode(&provider),
                        encode(error_msg)
                    );
                    info!("Redirecting OAuth error to frontend: {}", redirect_url);
                    return Ok(
                        (StatusCode::FOUND, [(header::LOCATION, redirect_url)], "").into_response()
                    );
                }

                let html =
                    OAuthTemplateRenderer::render_error_template(&provider, error_msg, description);

                Ok((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(header::CONTENT_TYPE, "text/html")],
                    html,
                )
                    .into_response())
            }
        }
    }

    /// Handle OAuth status check (Axum)
    #[tracing::instrument(
        skip(resources, headers),
        fields(
            route = "oauth_status",
            user_id = Empty,
        )
    )]
    async fn handle_oauth_status(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        // Authenticate using middleware (supports both cookies and Authorization header)
        let auth_result = resources
            .auth_middleware
            .authenticate_request_with_headers(&headers)
            .await?;

        let user_id = auth_result.user_id;

        // Check OAuth provider connection status for the user (cross-tenant view)
        let provider_statuses = resources
            .database
            .get_user_oauth_tokens(user_id, None)
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
                    let mut providers_seen = HashSet::new();

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

        Ok((StatusCode::OK, Json(provider_statuses)).into_response())
    }

    /// Get all providers with connection status
    ///
    /// Returns all available providers from the registry with their connection status.
    /// Uses `provider_connections` table as the single source of truth for connectivity.
    async fn handle_providers_status(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        use crate::providers::registry::global_registry;

        // Authenticate using middleware
        let auth_result = resources
            .auth_middleware
            .authenticate_request_with_headers(&headers)
            .await?;

        let user_id = auth_result.user_id;

        // Get all supported providers from the registry
        let registry = global_registry();
        let supported_providers = registry.supported_providers();

        // Get user's provider connections (cross-tenant view, single source of truth)
        let connections = resources
            .database
            .get_user_provider_connections(user_id, None)
            .await
            .unwrap_or_default();

        let connected_providers: HashSet<String> =
            connections.into_iter().map(|c| c.provider).collect();

        // Build provider status list
        let mut provider_statuses = Vec::new();

        for provider_name in supported_providers {
            // Get provider descriptor from registry
            if let Some(descriptor) = registry.get_descriptor(provider_name) {
                let caps = descriptor.capabilities();
                let requires_oauth = caps.requires_oauth();

                // Determine connection status from the provider_connections table
                let connected = connected_providers.contains(provider_name);

                // Skip non-OAuth providers that aren't connected (no data available)
                // This prevents showing "Not Available" for synthetic providers without data
                if !requires_oauth && !connected {
                    continue;
                }

                // Build capabilities list from bitflags
                let mut capabilities = Vec::new();
                if caps.supports_activities() {
                    capabilities.push("activities".to_owned());
                }
                if caps.supports_sleep() {
                    capabilities.push("sleep".to_owned());
                }
                if caps.supports_recovery() {
                    capabilities.push("recovery".to_owned());
                }
                if caps.supports_health() {
                    capabilities.push("health".to_owned());
                }

                provider_statuses.push(ProviderStatus {
                    provider: provider_name.to_owned(),
                    display_name: descriptor.display_name().to_owned(),
                    requires_oauth,
                    connected,
                    capabilities,
                });
            }
        }

        let response = ProvidersStatusResponse {
            providers: provider_statuses,
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Parse a user ID string to UUID
    fn parse_user_id(user_id_str: &str) -> Result<uuid::Uuid, AppError> {
        uuid::Uuid::parse_str(user_id_str).map_err(|_| {
            error!("Invalid user_id format: {}", user_id_str);
            AppError::invalid_input("Invalid user ID format")
        })
    }

    /// Retrieve user from database with proper error handling
    async fn get_user_for_oauth(
        database: &Database,
        user_id: uuid::Uuid,
    ) -> Result<User, AppError> {
        match database.get_user(user_id).await {
            Ok(Some(user)) => Ok(user),
            Ok(None) => {
                error!("User {} not found in database", user_id);
                Err(AppError::not_found("User account not found"))
            }
            Err(e) => {
                error!("Failed to get user {} for OAuth: {}", user_id, e);
                Err(AppError::database(format!(
                    "Failed to retrieve user information: {e}"
                )))
            }
        }
    }

    /// Extract tenant ID from user's tenant memberships, falling back to `user_id` if no tenant
    ///
    /// NOTE: This is a helper that requires tenant info to be pre-fetched from `tenant_users` table.
    /// For async contexts, use the database method `list_tenants_for_user` directly.
    async fn extract_tenant_id_from_database(
        database: &Database,
        user_id: uuid::Uuid,
    ) -> Result<uuid::Uuid, AppError> {
        let tenants = database
            .list_tenants_for_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user tenants: {e}")))?;

        tenants.first().map_or_else(
            || {
                debug!(user_id = %user_id, "User has no tenants - using user_id as tenant");
                Ok(user_id)
            },
            |tenant| Ok(tenant.id),
        )
    }

    /// Handle OAuth authorization initiation (Axum)
    ///
    /// Requires authentication and verifies that the authenticated user matches
    /// the `user_id` in the path to prevent unauthorized OAuth flow initiation.
    #[tracing::instrument(
        skip(resources, headers),
        fields(
            route = "oauth_auth_initiate",
            provider = %provider,
            user_id = %user_id_str,
            tenant_id = Empty,
        )
    )]
    async fn handle_oauth_auth_initiate(
        State(resources): State<Arc<ServerResources>>,
        Path((provider, user_id_str)): Path<(String, String)>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        // Authenticate the request before proceeding
        let auth_result = resources
            .auth_middleware
            .authenticate_request_with_headers(&headers)
            .await?;

        let user_id = Self::parse_user_id(&user_id_str)?;

        // Verify authenticated user matches the requested user_id
        if auth_result.user_id != user_id {
            warn!(
                "OAuth auth initiate: authenticated user {} does not match path user_id {}",
                auth_result.user_id, user_id
            );
            return Err(AppError::new(
                ErrorCode::PermissionDenied,
                "Cannot initiate OAuth flow for a different user",
            ));
        }

        info!(
            "OAuth authorization initiation for provider: {} user: {}",
            provider, user_id_str
        );

        // Verify user exists
        Self::get_user_for_oauth(&resources.database, user_id).await?;
        let tenant_id = Self::extract_tenant_id_from_database(&resources.database, user_id).await?;

        let server_context = ServerContext::from(resources.as_ref());
        let oauth_service = OAuthService::new(
            server_context.data().clone(),
            server_context.config().clone(),
            server_context.notification().clone(),
        );

        let auth_response = oauth_service
            .get_auth_url(user_id, tenant_id, &provider)
            .await
            .map_err(|e| {
                error!(
                    "Failed to generate OAuth URL for {} user {}: {}",
                    provider, user_id, e
                );
                AppError::internal(format!("Failed to generate OAuth URL for {provider}: {e}"))
            })?;

        info!(
            "Generated OAuth URL for {} user {} (state issued)",
            provider, user_id
        );

        Ok((
            StatusCode::FOUND,
            [(header::LOCATION, auth_response.authorization_url)],
        )
            .into_response())
    }

    /// Handle mobile OAuth initiation (Axum)
    ///
    /// Returns OAuth URL in JSON format for mobile apps to use with in-app browsers.
    /// Accepts optional `redirect_url` query parameter for deep linking back to the app.
    #[tracing::instrument(
        skip(resources, headers, query),
        fields(
            route = "mobile_oauth_init",
            provider = %provider,
            user_id = Empty,
        )
    )]
    async fn handle_mobile_oauth_init(
        State(resources): State<Arc<ServerResources>>,
        Path(provider): Path<String>,
        headers: HeaderMap,
        Query(query): Query<HashMap<String, String>>,
    ) -> Result<Response, AppError> {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

        // Authenticate using middleware
        let auth_result = resources
            .auth_middleware
            .authenticate_request_with_headers(&headers)
            .await?;

        let user_id = auth_result.user_id;
        info!(
            "Mobile OAuth initiation for provider: {} user: {}",
            provider, user_id
        );

        // Get optional redirect_uri from query parameters (mobile app's deep link)
        let redirect_url = query.get("redirect_uri");

        // Validate redirect URL scheme if provided
        if let Some(url) = redirect_url {
            let is_valid_scheme = url.starts_with("pierre://")
                || url.starts_with("exp://")
                || url.starts_with("http://localhost")
                || url.starts_with("https://");
            if !is_valid_scheme {
                return Err(AppError::invalid_input(
                    "Invalid redirect_url scheme. Allowed schemes: pierre://, exp://, http://localhost, https://",
                ));
            }
        }

        // Verify user exists
        Self::get_user_for_oauth(&resources.database, user_id).await?;
        let tenant_id = Self::extract_tenant_id_from_database(&resources.database, user_id).await?;

        // Build OAuth state with optional redirect URL
        let state = redirect_url.map_or_else(
            || format!("{}:{}", user_id, uuid::Uuid::new_v4()),
            |url| {
                let encoded_url = URL_SAFE_NO_PAD.encode(url.as_bytes());
                format!("{}:{}:{}", user_id, uuid::Uuid::new_v4(), encoded_url)
            },
        );

        // Generate OAuth URL using the state with embedded redirect URL
        let tenant_name = resources
            .database
            .get_tenant_by_id(tenant_id)
            .await
            .map_or_else(|_| "Unknown Tenant".to_owned(), |t| t.name);

        let ctx = TenantContext {
            tenant_id,
            user_id,
            tenant_name,
            user_role: TenantRole::Member,
        };

        // Check if the provider supports PKCE for enhanced security
        let use_pkce = resources
            .provider_registry
            .get_descriptor(&provider)
            .and_then(ProviderDescriptor::oauth_params)
            .is_some_and(|p| p.use_pkce);

        let pkce = if use_pkce {
            Some(PkceParams::generate())
        } else {
            None
        };

        let authorization_url = if let Some(ref pkce_params) = pkce {
            resources
                .tenant_oauth_client
                .get_authorization_url_with_pkce(
                    &ctx,
                    &provider,
                    &state,
                    pkce_params,
                    resources.database.as_ref(),
                )
                .await
        } else {
            resources
                .tenant_oauth_client
                .get_authorization_url(&ctx, &provider, &state, resources.database.as_ref())
                .await
        }
        .map_err(|e| {
            error!(
                "Failed to generate OAuth URL for {} user {}: {}",
                provider, user_id, e
            );
            AppError::internal(format!("Failed to generate OAuth URL for {provider}: {e}"))
        })?;

        // Build redirect URI for state storage
        let base_url = env::var("BASE_URL")
            .unwrap_or_else(|_| format!("http://localhost:{}", resources.config.http_port));
        let oauth_redirect_uri = format!("{base_url}/api/oauth/callback/{provider}");

        // Store state server-side for CSRF protection with 10-minute TTL.
        // The pkce_code_verifier is stored alongside the state for PKCE token exchange.
        let now = Utc::now();
        let client_state = OAuthClientState {
            state: state.clone(),
            provider: provider.clone(),
            user_id: Some(user_id),
            tenant_id: Some(tenant_id.to_string()),
            redirect_uri: oauth_redirect_uri,
            scope: None,
            pkce_code_verifier: pkce.as_ref().map(|p| p.code_verifier.clone()),
            created_at: now,
            expires_at: now + chrono::Duration::minutes(10),
            used: false,
        };

        resources
            .database
            .store_oauth_client_state(&client_state)
            .await
            .map_err(|e| {
                error!("Failed to store OAuth state for CSRF protection: {}", e);
                AppError::internal("Failed to initiate OAuth flow")
            })?;

        info!(
            "Generated mobile OAuth URL for {} user {} (state issued){}",
            provider,
            user_id,
            if redirect_url.is_some() {
                " (with redirect)"
            } else {
                ""
            }
        );

        // Return JSON response with OAuth URL (mobile apps need this for in-app browsers)
        // State is returned so mobile apps can correlate the callback
        Ok((
            StatusCode::OK,
            Json(json!({
                "authorization_url": authorization_url,
                "provider": provider,
                "state": state,
                "message": format!("Visit the authorization URL to connect your {} account", provider)
            })),
        )
            .into_response())
    }

    /// REST endpoint to disconnect a provider
    ///
    /// DELETE /api/oauth/providers/:provider/disconnect
    ///
    /// Disconnects a fitness provider (e.g., Strava, Fitbit) by deleting the stored OAuth tokens.
    /// Requires valid JWT authentication via cookie or Authorization header.
    async fn handle_disconnect_provider_rest(
        State(resources): State<Arc<ServerResources>>,
        Path(provider): Path<String>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        // Authenticate using middleware (supports both cookies and Authorization header)
        let auth_result = resources
            .auth_middleware
            .authenticate_request_with_headers(&headers)
            .await?;

        let user_id = auth_result.user_id;
        info!("Disconnecting provider {} for user {}", provider, user_id);

        // Create OAuthService instance and call existing disconnect logic
        let server_context = ServerContext::from(resources.as_ref());
        let oauth_service = OAuthService::new(
            server_context.data().clone(),
            server_context.config().clone(),
            server_context.notification().clone(),
        );
        oauth_service
            .disconnect_provider(user_id, &provider)
            .await?;

        Ok(StatusCode::NO_CONTENT.into_response())
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

    /// Extract mobile redirect URL from OAuth state parameter for error handling
    ///
    /// This is used when the OAuth callback fails and we need to redirect
    /// the error to the mobile app. Duplicates some logic from `OAuthService::validate_oauth_state`
    /// but only extracts the redirect URL without full validation.
    fn extract_mobile_redirect_from_state(state: &str) -> Option<String> {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

        let parts: Vec<&str> = state.splitn(3, ':').collect();
        if parts.len() < 3 || parts[2].is_empty() {
            return None;
        }

        URL_SAFE_NO_PAD
            .decode(parts[2])
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .filter(|url| {
                // Validate URL scheme for security (only allow specific schemes)
                url.starts_with("pierre://")
                    || url.starts_with("exp://")
                    || url.starts_with("http://localhost")
                    || url.starts_with("https://")
            })
    }
}
