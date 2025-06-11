// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! HTTP routes for user authentication and OAuth flows in multi-tenant mode

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{info, error};
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

    /// Get OAuth authorization URL for a provider
    pub async fn get_auth_url(&self, user_id: &str, provider: &str) -> Result<String> {
        match provider {
            "strava" => {
                // Generate state parameter for CSRF protection
                let state = format!("{}:{}", user_id, uuid::Uuid::new_v4());
                
                // In a real implementation, you would:
                // 1. Store the state in a temporary cache/database
                // 2. Generate the OAuth URL with your client ID
                // 3. Return the URL for the user to visit
                
                Ok(format!("https://www.strava.com/oauth/authorize?client_id=YOUR_CLIENT_ID&redirect_uri=YOUR_REDIRECT_URI&response_type=code&scope=read,activity:read_all&state={}", state))
            }
            "fitbit" => {
                let state = format!("{}:{}", user_id, uuid::Uuid::new_v4());
                Ok(format!("https://www.fitbit.com/oauth2/authorize?client_id=YOUR_CLIENT_ID&redirect_uri=YOUR_REDIRECT_URI&response_type=code&scope=activity%20profile&state={}", state))
            }
            _ => Err(anyhow::anyhow!("Unsupported provider: {}", provider))
        }
    }

    /// Handle OAuth callback and store tokens
    pub async fn handle_callback(&self, _code: &str, state: &str, provider: &str) -> Result<()> {
        // Parse user ID from state
        let parts: Vec<&str> = state.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid state parameter"));
        }
        
        let user_id = uuid::Uuid::parse_str(parts[0])?;
        
        // Exchange code for tokens (implementation depends on provider)
        match provider {
            "strava" => {
                // Exchange authorization code for access and refresh tokens
                // Store encrypted tokens in database
                info!("Processing Strava OAuth callback for user: {}", user_id);
                // TODO: Implement token exchange and storage
            }
            "fitbit" => {
                info!("Processing Fitbit OAuth callback for user: {}", user_id);
                // TODO: Implement token exchange and storage
            }
            _ => return Err(anyhow::anyhow!("Unsupported provider: {}", provider))
        }

        Ok(())
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