// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! HTTP routes for user authentication and OAuth flows in multi-tenant mode

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{info, error};
use uuid::Uuid;
use base64::{Engine, engine::general_purpose};
use crate::{
    auth::AuthManager,
    database::Database,
    models::User,
};

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

#[derive(Debug, Deserialize)]
struct StravaTokenResponse {
    access_token: String,
    refresh_token: String,
    expires_at: i64,
    #[allow(dead_code)]
    expires_in: i64,
    #[allow(dead_code)]
    token_type: String,
    #[serde(default)]
    scope: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    athlete: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct FitbitTokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
    #[allow(dead_code)]
    token_type: String,
    scope: String,
    #[allow(dead_code)]
    user_id: String,
}

#[derive(Clone)]
pub struct AuthRoutes {
    database: Database,
    auth_manager: AuthManager,
}

impl AuthRoutes {
    pub fn new(database: Database, auth_manager: AuthManager) -> Self {
        Self {
            database,
            auth_manager,
        }
    }

    /// Handle user registration
    pub async fn register(&self, request: RegisterRequest) -> Result<RegisterResponse> {
        info!("User registration attempt for email: {}", request.email);

        // Validate email format
        if !self.is_valid_email(&request.email) {
            return Err(anyhow::anyhow!("Invalid email format"));
        }

        // Validate password strength
        if !self.is_valid_password(&request.password) {
            return Err(anyhow::anyhow!("Password must be at least 8 characters long"));
        }

        // Check if user already exists
        if let Ok(Some(_)) = self.database.get_user_by_email(&request.email).await {
            return Err(anyhow::anyhow!("User with this email already exists"));
        }

        // Hash password
        let password_hash = bcrypt::hash(&request.password, bcrypt::DEFAULT_COST)?;

        // Create user
        let user = User::new(
            request.email.clone(),
            password_hash,
            request.display_name,
        );

        // Save user to database
        let user_id = self.database.create_user(&user).await?;

        info!("User registered successfully: {} ({})", request.email, user_id);

        Ok(RegisterResponse {
            user_id: user_id.to_string(),
            message: "User registered successfully".to_string(),
        })
    }

    /// Handle user login
    pub async fn login(&self, request: LoginRequest) -> Result<LoginResponse> {
        info!("User login attempt for email: {}", request.email);

        // Get user from database
        let user = self.database.get_user_by_email_required(&request.email).await
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
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(24); // TODO: use auth_manager config

        info!("User logged in successfully: {} ({})", request.email, user.id);

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

    /// Validate email format
    fn is_valid_email(&self, email: &str) -> bool {
        // Simple email validation
        if email.len() <= 5 {
            return false;
        }
        let at_pos = email.find('@');
        if at_pos.is_none() {
            return false;
        }
        let at_pos = at_pos.unwrap();
        if at_pos == 0 || at_pos == email.len() - 1 {
            return false; // @ at start or end
        }
        let domain_part = &email[at_pos + 1..];
        domain_part.contains('.')
    }

    /// Validate password strength
    fn is_valid_password(&self, password: &str) -> bool {
        password.len() >= 8
    }
}

/// OAuth flow routes for connecting fitness providers
#[derive(Clone)]
pub struct OAuthRoutes {
    #[allow(dead_code)]
    database: Database,
}

impl OAuthRoutes {
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    /// Get OAuth authorization URL for a provider with real configuration
    pub async fn get_auth_url(&self, user_id: uuid::Uuid, provider: &str) -> Result<OAuthAuthorizationResponse> {
        // Store state in database for CSRF protection
        let state = format!("{}:{}", user_id, uuid::Uuid::new_v4());
        self.store_oauth_state(user_id, provider, &state).await?;
        
        match provider {
            "strava" => {
                let client_id = std::env::var("STRAVA_CLIENT_ID")
                    .or_else(|_| std::env::var("strava_client_id"))
                    .unwrap_or_else(|_| "YOUR_STRAVA_CLIENT_ID".to_string());
                
                let redirect_uri = std::env::var("STRAVA_REDIRECT_URI")
                    .or_else(|_| std::env::var("strava_redirect_uri"))
                    .unwrap_or_else(|_| "http://localhost:8081/oauth/callback/strava".to_string());
                
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
                    state: state.clone(),
                    instructions: "Visit the URL above to authorize access to your Strava account. You'll be redirected back after authorization.".to_string(),
                    expires_in_minutes: 10,
                })
            }
            "fitbit" => {
                let client_id = std::env::var("FITBIT_CLIENT_ID")
                    .or_else(|_| std::env::var("fitbit_client_id"))
                    .unwrap_or_else(|_| "YOUR_FITBIT_CLIENT_ID".to_string());
                
                let redirect_uri = std::env::var("FITBIT_REDIRECT_URI")
                    .or_else(|_| std::env::var("fitbit_redirect_uri"))
                    .unwrap_or_else(|_| "http://localhost:8081/oauth/callback/fitbit".to_string());
                
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
                    state: state.clone(),
                    instructions: "Visit the URL above to authorize access to your Fitbit account. You'll be redirected back after authorization.".to_string(),
                    expires_in_minutes: 10,
                })
            }
            _ => Err(anyhow::anyhow!("Unsupported provider: {}", provider))
        }
    }
    
    /// Store OAuth state for CSRF protection
    async fn store_oauth_state(&self, user_id: uuid::Uuid, provider: &str, state: &str) -> Result<()> {
        // Store state with expiration (10 minutes)
        let _expires_at = chrono::Utc::now() + chrono::Duration::minutes(10);
        
        // In a production system, you'd store this in a cache/database
        // For now, we'll store it in memory or use a simple approach
        info!("Storing OAuth state for user {} provider {}: {}", user_id, provider, state);
        
        // TODO: Implement proper state storage
        Ok(())
    }

    /// Handle OAuth callback and store tokens
    pub async fn handle_callback(&self, code: &str, state: &str, provider: &str) -> Result<OAuthCallbackResponse> {
        // Parse user ID from state
        let parts: Vec<&str> = state.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid state parameter"));
        }
        
        let user_id = uuid::Uuid::parse_str(parts[0])?;
        
        // Validate state (in production, check against stored state)
        info!("Processing OAuth callback for user {} provider {}", user_id, provider);
        
        // Exchange code for tokens (implementation depends on provider)
        match provider {
            "strava" => {
                let token_response = self.exchange_strava_code(code).await?;
                
                // Store encrypted tokens in database
                let expires_at = chrono::DateTime::<chrono::Utc>::from_timestamp(token_response.expires_at, 0)
                    .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::hours(6));
                
                self.database.update_strava_token(
                    user_id,
                    &token_response.access_token,
                    &token_response.refresh_token,
                    expires_at,
                    token_response.scope.clone().unwrap_or_else(|| "read,activity:read_all".to_string()),
                ).await?;
                
                info!("Strava tokens stored successfully for user: {}", user_id);
                
                Ok(OAuthCallbackResponse {
                    user_id: user_id.to_string(),
                    provider: "strava".to_string(),
                    expires_at: expires_at.to_rfc3339(),
                    scopes: token_response.scope.unwrap_or_else(|| "read,activity:read_all".to_string()),
                })
            }
            "fitbit" => {
                let token_response = self.exchange_fitbit_code(code).await?;
                
                // Store encrypted tokens in database
                let expires_at = chrono::Utc::now() + chrono::Duration::seconds(token_response.expires_in);
                
                self.database.update_fitbit_token(
                    user_id,
                    &token_response.access_token,
                    &token_response.refresh_token,
                    expires_at,
                    token_response.scope.clone(),
                ).await?;
                
                info!("Fitbit tokens stored successfully for user: {}", user_id);
                
                Ok(OAuthCallbackResponse {
                    user_id: user_id.to_string(),
                    provider: "fitbit".to_string(),
                    expires_at: expires_at.to_rfc3339(),
                    scopes: token_response.scope,
                })
            }
            _ => Err(anyhow::anyhow!("Unsupported provider: {}", provider))
        }
    }
    
    /// Exchange Strava authorization code for tokens
    async fn exchange_strava_code(&self, code: &str) -> Result<StravaTokenResponse> {
        let client_id = std::env::var("STRAVA_CLIENT_ID")
            .or_else(|_| std::env::var("strava_client_id"))
            .unwrap_or_else(|_| "163846".to_string()); // Default for testing
        
        let client_secret = std::env::var("STRAVA_CLIENT_SECRET")
            .or_else(|_| std::env::var("strava_client_secret"))
            .unwrap_or_else(|_| "1dfc45ad0a1f6983b835e4495aa9473d111d03bc".to_string()); // Default for testing
        
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
        
        info!("Strava token exchange response - Status: {}, Body: {}", status, response_text);
        
        if !status.is_success() {
            return Err(anyhow::anyhow!("Strava token exchange failed: {}", response_text));
        }
        
        let token_response: StravaTokenResponse = serde_json::from_str(&response_text)
            .map_err(|e| anyhow::anyhow!("Failed to parse Strava response: {}. Response was: {}", e, response_text))?;
        info!("Strava token exchange successful");
        
        Ok(token_response)
    }
    
    /// Exchange Fitbit authorization code for tokens
    async fn exchange_fitbit_code(&self, code: &str) -> Result<FitbitTokenResponse> {
        let client_id = std::env::var("FITBIT_CLIENT_ID")
            .or_else(|_| std::env::var("fitbit_client_id"))
            .unwrap_or_else(|_| "YOUR_FITBIT_CLIENT_ID".to_string());
        
        let client_secret = std::env::var("FITBIT_CLIENT_SECRET")
            .or_else(|_| std::env::var("fitbit_client_secret"))
            .unwrap_or_else(|_| "YOUR_FITBIT_CLIENT_SECRET".to_string());
        
        let redirect_uri = std::env::var("FITBIT_REDIRECT_URI")
            .or_else(|_| std::env::var("fitbit_redirect_uri"))
            .unwrap_or_else(|_| "http://localhost:8081/oauth/callback/fitbit".to_string());
        
        let params = [
            ("client_id", client_id.as_str()),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri.as_str()),
            ("code", code),
        ];
        
        let auth_header = general_purpose::STANDARD.encode(format!("{}:{}", client_id, client_secret));
        
        let client = reqwest::Client::new();
        let response = client
            .post("https://api.fitbit.com/oauth2/token")
            .header("Authorization", format!("Basic {}", auth_header))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Fitbit token exchange failed: {}", error_text));
        }
        
        let token_response: FitbitTokenResponse = response.json().await?;
        info!("Fitbit token exchange successful");
        
        Ok(token_response)
    }
    
    /// Get connection status for all providers for a user
    pub async fn get_connection_status(&self, user_id: Uuid) -> Result<Vec<ConnectionStatus>> {
        let mut statuses = Vec::new();
        
        // Check Strava connection
        if let Ok(Some(strava_token)) = self.database.get_strava_token(user_id).await {
            statuses.push(ConnectionStatus {
                provider: "strava".to_string(),
                connected: true,
                expires_at: Some(strava_token.expires_at.to_rfc3339()),
                scopes: Some(strava_token.scope),
            });
        } else {
            statuses.push(ConnectionStatus {
                provider: "strava".to_string(),
                connected: false,
                expires_at: None,
                scopes: None,
            });
        }
        
        // Check Fitbit connection
        if let Ok(Some(fitbit_token)) = self.database.get_fitbit_token(user_id).await {
            statuses.push(ConnectionStatus {
                provider: "fitbit".to_string(),
                connected: true,
                expires_at: Some(fitbit_token.expires_at.to_rfc3339()),
                scopes: Some(fitbit_token.scope),
            });
        } else {
            statuses.push(ConnectionStatus {
                provider: "fitbit".to_string(),
                connected: false,
                expires_at: None,
                scopes: None,
            });
        }
        
        Ok(statuses)
    }
    
    /// Disconnect a provider by removing stored tokens
    pub async fn disconnect_provider(&self, user_id: Uuid, provider: &str) -> Result<()> {
        match provider {
            "strava" => {
                // TODO: Implement token revocation by clearing the tokens in database
                // For now, we'd need to add a method to clear specific provider tokens
                info!("Disconnecting Strava for user {}", user_id);
                // self.database.clear_strava_token(user_id).await?;
                Ok(())
            }
            "fitbit" => {
                info!("Disconnecting Fitbit for user {}", user_id);
                // self.database.clear_fitbit_token(user_id).await?;
                Ok(())
            }
            _ => Err(anyhow::anyhow!("Unsupported provider: {}", provider))
        }
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
        let database = Database::new(&format!("sqlite:{}", db_path.display()), vec![0u8; 32]).await.unwrap();
        let auth_manager = AuthManager::new(vec![0u8; 64], 24);
        let routes = AuthRoutes::new(database, auth_manager);

        assert!(routes.is_valid_email("test@example.com"));
        assert!(routes.is_valid_email("user.name+tag@domain.co.uk"));
        assert!(!routes.is_valid_email("invalid-email"));
        assert!(!routes.is_valid_email("@domain.com"));
        assert!(!routes.is_valid_email("user@"));
    }

    #[tokio::test]
    async fn test_password_validation() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let database = Database::new(&format!("sqlite:{}", db_path.display()), vec![0u8; 32]).await.unwrap();
        let auth_manager = AuthManager::new(vec![0u8; 64], 24);
        let routes = AuthRoutes::new(database, auth_manager);

        assert!(routes.is_valid_password("password123"));
        assert!(routes.is_valid_password("verylongpassword"));
        assert!(!routes.is_valid_password("short"));
        assert!(!routes.is_valid_password("1234567"));
    }

    #[tokio::test]
    async fn test_register_user() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let database = Database::new(&format!("sqlite:{}", db_path.display()), vec![0u8; 32]).await.unwrap();
        let auth_manager = AuthManager::new(vec![0u8; 64], 24);
        let routes = AuthRoutes::new(database, auth_manager);

        let request = RegisterRequest {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            display_name: Some("Test User".to_string()),
        };

        let response = routes.register(request).await.unwrap();
        assert!(!response.user_id.is_empty());
        assert_eq!(response.message, "User registered successfully");
    }

    #[tokio::test]
    async fn test_register_duplicate_user() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let database = Database::new(&format!("sqlite:{}", db_path.display()), vec![0u8; 32]).await.unwrap();
        let auth_manager = AuthManager::new(vec![0u8; 64], 24);
        let routes = AuthRoutes::new(database, auth_manager);

        let request = RegisterRequest {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            display_name: Some("Test User".to_string()),
        };

        // First registration should succeed
        routes.register(request.clone()).await.unwrap();

        // Second registration should fail
        let result = routes.register(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }
}