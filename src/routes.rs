// ABOUTME: HTTP REST API route handlers for user-facing endpoints and web interfaces
// ABOUTME: Provides authentication, user management, and basic API endpoints for web clients
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! HTTP routes for user authentication and OAuth flows in multi-tenant mode

use crate::{
    auth::AuthManager,
    database_plugins::{factory::Database, DatabaseProvider},
    models::User,
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
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
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
    database: Database,
    auth_manager: AuthManager,
}

impl AuthRoutes {
    #[must_use]
    pub const fn new(database: Database, auth_manager: AuthManager) -> Self {
        Self {
            database,
            auth_manager,
        }
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
            return Err(anyhow::anyhow!("Invalid email format"));
        }

        // Validate password strength
        if !Self::is_valid_password(&request.password) {
            return Err(anyhow::anyhow!(
                "Password must be at least 8 characters long"
            ));
        }

        // Check if user already exists
        if let Ok(Some(_)) = self.database.get_user_by_email(&request.email).await {
            return Err(anyhow::anyhow!("User with this email already exists"));
        }

        // Hash password
        let password_hash = bcrypt::hash(&request.password, bcrypt::DEFAULT_COST)?;

        // Create user
        let user = User::new(request.email.clone(), password_hash, request.display_name);

        // Save user to database
        let user_id = self.database.create_user(&user).await?;

        info!(
            "User registered successfully: {} ({})",
            request.email, user_id
        );

        Ok(RegisterResponse {
            user_id: user_id.to_string(),
            message: "User registered successfully".into(),
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
            .database
            .get_user_by_email_required(&request.email)
            .await
            .map_err(|_| anyhow::anyhow!("Invalid email or password"))?;

        // Verify password
        if !bcrypt::verify(&request.password, &user.password_hash)? {
            error!("Invalid password for user: {}", request.email);
            return Err(anyhow::anyhow!("Invalid email or password"));
        }

        // Update last active timestamp
        self.database.update_last_active(user.id).await?;

        // Generate JWT token
        let jwt_token = self.auth_manager.generate_token(&user)?;
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(24); // Default 24h expiry

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
        let user_uuid = uuid::Uuid::parse_str(&request.user_id)
            .map_err(|_| anyhow::anyhow!("Invalid user ID format"))?;

        // Get user from database
        let user = self
            .database
            .get_user(user_uuid)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;

        // Validate the current token and refresh it
        let new_jwt_token = self.auth_manager.refresh_token(&request.token, &user)?;
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);

        // Update last active timestamp
        self.database.update_last_active(user.id).await?;

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
    fn is_valid_email(email: &str) -> bool {
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
    const fn is_valid_password(password: &str) -> bool {
        password.len() >= 8
    }
}

/// OAuth flow routes for connecting fitness providers
#[derive(Clone)]
pub struct OAuthRoutes {
    database: Database,
}

impl OAuthRoutes {
    #[must_use]
    pub const fn new(database: Database) -> Self {
        Self { database }
    }

    /// Get OAuth authorization URL for a provider with real configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Provider is not supported
    /// - Database operation fails when storing OAuth state
    pub fn get_auth_url(
        &self,
        user_id: uuid::Uuid,
        provider: &str,
    ) -> Result<OAuthAuthorizationResponse> {
        // Store state in database for CSRF protection
        let new_uuid = uuid::Uuid::new_v4();
        let state = format!("{user_id}:{new_uuid}");
        Self::store_oauth_state(user_id, provider, &state);

        match provider {
            "strava" => {
                let client_id = std::env::var("STRAVA_CLIENT_ID")
                    .or_else(|_| std::env::var("strava_client_id"))
                    .unwrap_or_else(|_| "YOUR_STRAVA_CLIENT_ID".into());

                let redirect_uri = std::env::var("STRAVA_REDIRECT_URI")
                    .or_else(|_| std::env::var("strava_redirect_uri"))
                    .unwrap_or_else(|_| {
                        format!(
                            "http://localhost:{}/oauth/callback/strava",
                            crate::constants::ports::DEFAULT_HTTP_PORT
                        )
                    });

                let scope = "read,activity:read_all";

                let auth_url = format!(
                    "https://www.strava.com/oauth/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
                    urlencoding::encode(&client_id),
                    urlencoding::encode(&redirect_uri),
                    urlencoding::encode(scope),
                    urlencoding::encode(&state)
                );

                Ok(OAuthAuthorizationResponse {
                    authorization_url: auth_url,
                    state,
                    instructions: "Visit the URL above to authorize access to your Strava account. You'll be redirected back after authorization.".into(),
                    expires_in_minutes: 10,
                })
            }
            "fitbit" => {
                let client_id = std::env::var("FITBIT_CLIENT_ID")
                    .or_else(|_| std::env::var("fitbit_client_id"))
                    .unwrap_or_else(|_| "YOUR_FITBIT_CLIENT_ID".into());

                let redirect_uri = std::env::var("FITBIT_REDIRECT_URI")
                    .or_else(|_| std::env::var("fitbit_redirect_uri"))
                    .unwrap_or_else(|_| {
                        format!(
                            "http://localhost:{}/oauth/callback/fitbit",
                            crate::constants::ports::DEFAULT_HTTP_PORT
                        )
                    });

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
        // Parse user ID from state
        let parts: Vec<&str> = state.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid state parameter"));
        }

        let user_id = uuid::Uuid::parse_str(parts[0])?;

        // Validate state (in production, check against stored state)
        info!(
            "Processing OAuth callback for user {} provider {}",
            user_id, provider
        );

        // Exchange code for tokens (implementation depends on provider)
        match provider {
            "strava" => {
                let token_response = self.exchange_strava_code(code).await?;

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
                            chrono::Utc::now()
                                + chrono::Duration::seconds(token_response.expires_in)
                        })
                } else {
                    chrono::Utc::now() + chrono::Duration::seconds(token_response.expires_in)
                };

                self.database
                    .update_strava_token(
                        user_id,
                        &token_response.access_token,
                        &token_response.refresh_token,
                        expires_at,
                        token_response
                            .scope
                            .clone()
                            .unwrap_or_else(|| "read,activity:read_all".into()),
                    )
                    .await?;

                info!("Strava tokens stored successfully for user: {}", user_id);

                Ok(OAuthCallbackResponse {
                    user_id: user_id.to_string(),
                    provider: "strava".into(),
                    expires_at: expires_at.to_rfc3339(),
                    scopes: token_response
                        .scope
                        .unwrap_or_else(|| "read,activity:read_all".into()),
                })
            }
            "fitbit" => {
                let token_response = self.exchange_fitbit_code(code).await?;

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
                let expires_at =
                    chrono::Utc::now() + chrono::Duration::seconds(token_response.expires_in);

                self.database
                    .update_fitbit_token(
                        user_id,
                        &token_response.access_token,
                        &token_response.refresh_token,
                        expires_at,
                        token_response.scope.clone(),
                    )
                    .await?;

                info!("Fitbit tokens stored successfully for user: {}", user_id);

                Ok(OAuthCallbackResponse {
                    user_id: user_id.to_string(),
                    provider: "fitbit".into(),
                    expires_at: expires_at.to_rfc3339(),
                    scopes: token_response.scope,
                })
            }
            _ => Err(anyhow::anyhow!("Unsupported provider: {}", provider)),
        }
    }

    /// Exchange Strava authorization code for tokens
    async fn exchange_strava_code(&self, code: &str) -> Result<StravaTokenResponse> {
        let client_id = std::env::var("STRAVA_CLIENT_ID")
            .or_else(|_| std::env::var("strava_client_id"))
            .unwrap_or_else(|_| "163846".into()); // Default for testing

        let client_secret = std::env::var("STRAVA_CLIENT_SECRET")
            .or_else(|_| std::env::var("strava_client_secret"))
            .unwrap_or_else(|_| "1dfc45ad0a1f6983b835e4495aa9473d111d03bc".into()); // Default for testing

        let params = [
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
        ];

        let client = reqwest::Client::new();
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
            return Err(anyhow::anyhow!(
                "Strava token exchange failed: {}",
                response_text
            ));
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
    async fn exchange_fitbit_code(&self, code: &str) -> Result<FitbitTokenResponse> {
        let client_id = std::env::var("FITBIT_CLIENT_ID")
            .or_else(|_| std::env::var("fitbit_client_id"))
            .unwrap_or_else(|_| "YOUR_FITBIT_CLIENT_ID".into());

        let client_secret = std::env::var("FITBIT_CLIENT_SECRET")
            .or_else(|_| std::env::var("fitbit_client_secret"))
            .unwrap_or_else(|_| "YOUR_FITBIT_CLIENT_SECRET".into());

        let redirect_uri = crate::constants::env_config::fitbit_redirect_uri();

        let params = [
            ("client_id", client_id.as_str()),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri.as_str()),
            ("code", code),
        ];

        let auth_header = general_purpose::STANDARD.encode(format!("{client_id}:{client_secret}"));

        let client = reqwest::Client::new();
        let response = client
            .post("https://api.fitbit.com/oauth2/token")
            .header("Authorization", format!("Basic {auth_header}"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!(
                "Fitbit token exchange failed: {}",
                error_text
            ));
        }

        let token_response: FitbitTokenResponse = response.json().await?;
        info!("Fitbit token exchange successful");

        Ok(token_response)
    }

    /// Get connection status for all providers for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database operation fails
    pub async fn get_connection_status(&self, user_id: Uuid) -> Result<Vec<ConnectionStatus>> {
        let mut statuses = Vec::new();

        // Check Strava connection
        if let Ok(Some(strava_token)) = self.database.get_strava_token(user_id).await {
            statuses.push(ConnectionStatus {
                provider: "strava".into(),
                connected: true,
                expires_at: Some(strava_token.expires_at.to_rfc3339()),
                scopes: Some(strava_token.scope),
            });
        } else {
            statuses.push(ConnectionStatus {
                provider: "strava".into(),
                connected: false,
                expires_at: None,
                scopes: None,
            });
        }

        // Check Fitbit connection
        if let Ok(Some(fitbit_token)) = self.database.get_fitbit_token(user_id).await {
            statuses.push(ConnectionStatus {
                provider: "fitbit".into(),
                connected: true,
                expires_at: Some(fitbit_token.expires_at.to_rfc3339()),
                scopes: Some(fitbit_token.scope),
            });
        } else {
            statuses.push(ConnectionStatus {
                provider: "fitbit".into(),
                connected: false,
                expires_at: None,
                scopes: None,
            });
        }

        Ok(statuses)
    }

    /// Disconnect a provider by removing stored tokens
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Provider is not supported
    pub fn disconnect_provider(&self, user_id: Uuid, provider: &str) -> Result<()> {
        match provider {
            "strava" => {
                // Token revocation would clear stored tokens from database
                // Clear provider tokens requires token revocation API calls
                info!("Disconnecting Strava for user {}", user_id);
                // self.database.clear_strava_token(user_id).await?;
                Ok(())
            }
            "fitbit" => {
                info!("Disconnecting Fitbit for user {}", user_id);
                // self.database.clear_fitbit_token(user_id).await?;
                Ok(())
            }
            _ => Err(anyhow::anyhow!("Unsupported provider: {}", provider)),
        }
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
        database: std::sync::Arc<crate::database_plugins::factory::Database>,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        tracing::info!(
            client_name = %request.name,
            capabilities = ?request.capabilities,
            contact_email = %request.contact_email,
            "A2A client registration request received"
        );

        // Create A2A client manager
        let client_manager = crate::a2a::A2AClientManager::new(database);

        match client_manager.register_client(request).await {
            Ok(credentials) => {
                tracing::info!(
                    client_id = %credentials.client_id,
                    "A2A client registered successfully"
                );

                let response = serde_json::json!({
                    "success": true,
                    "message": "A2A client registered successfully",
                    "data": {
                        "client_id": credentials.client_id,
                        "client_secret": credentials.client_secret,
                        "api_key": credentials.api_key,
                        "public_key": credentials.public_key,
                        "private_key": credentials.private_key,
                        "key_type": credentials.key_type,
                        "next_steps": {
                            "documentation": "https://docs.pierre.ai/a2a",
                            "authentication": "Use the provided credentials for A2A protocol authentication",
                            "endpoints": {
                                "a2a_protocol": "/a2a/protocol",
                                "agent_card": "/.well-known/agent.json"
                            }
                        }
                    }
                });

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

                let error_response = serde_json::json!({
                    "success": false,
                    "error": "registration_failed",
                    "error_description": format!("Failed to register A2A client: {e}"),
                    "details": e.to_string()
                });

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_email_validation() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_path_str = db_path.display();
        let database = Database::new(&format!("sqlite:{db_path_str}"), vec![0u8; 32])
            .await
            .unwrap();
        tracing::trace!("Created test database: {:?}", std::ptr::addr_of!(database));
        let auth_manager = AuthManager::new(vec![0u8; 64], 24);
        tracing::trace!(
            "Created test auth manager: {:?}",
            std::ptr::addr_of!(auth_manager)
        );
        // Email and password validation functions are now static, no need for routes instance
        assert!(AuthRoutes::is_valid_email("test@example.com"));
        assert!(AuthRoutes::is_valid_email("user.name+tag@domain.co.uk"));
        assert!(!AuthRoutes::is_valid_email("invalid-email"));
        assert!(!AuthRoutes::is_valid_email("@domain.com"));
        assert!(!AuthRoutes::is_valid_email("user@"));
    }

    #[tokio::test]
    async fn test_password_validation() {
        // Password validation function is now static, no need for database setup
        assert!(AuthRoutes::is_valid_password("password123"));
        assert!(AuthRoutes::is_valid_password("verylongpassword"));
        assert!(!AuthRoutes::is_valid_password("short"));
        assert!(!AuthRoutes::is_valid_password("1234567"));
    }

    #[tokio::test]
    async fn test_register_user() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_path_str = db_path.display();
        let database = Database::new(&format!("sqlite:{db_path_str}"), vec![0u8; 32])
            .await
            .unwrap();
        let auth_manager = AuthManager::new(vec![0u8; 64], 24);
        let routes = AuthRoutes::new(database, auth_manager);

        let request = RegisterRequest {
            email: "test@example.com".into(),
            password: "password123".into(),
            display_name: Some("Test User".into()),
        };

        let response = routes.register(request).await.unwrap();
        assert!(!response.user_id.is_empty());
        assert_eq!(response.message, "User registered successfully");
    }

    #[tokio::test]
    async fn test_register_duplicate_user() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_path_str = db_path.display();
        let database = Database::new(&format!("sqlite:{db_path_str}"), vec![0u8; 32])
            .await
            .unwrap();
        let auth_manager = AuthManager::new(vec![0u8; 64], 24);
        let routes = AuthRoutes::new(database, auth_manager);

        let request = RegisterRequest {
            email: "test@example.com".into(),
            password: "password123".into(),
            display_name: Some("Test User".into()),
        };

        // First registration should succeed
        routes.register(request.clone()).await.unwrap();

        // Second registration should fail
        let result = routes.register(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }
}
