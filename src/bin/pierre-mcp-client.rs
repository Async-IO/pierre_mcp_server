// ABOUTME: MCP client for Pierre Fitness API server
// ABOUTME: Handles JWT authentication and HTTP transport with SSE notifications

use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Utc};
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use std::{env, sync::Arc, time::Duration};
use tokio::{sync::RwLock, time::interval};

#[derive(Debug, Clone)]
struct Config {
    #[allow(dead_code)] // Used in future HTTP client features
    server_url: String,
    server_auth_url: String, // For token refresh endpoint
    sse_url: String,         // For SSE notifications
    jwt_token: Option<String>,
    timeout_seconds: u64,
    refresh_enabled: bool,
    refresh_threshold_minutes: i64, // Minutes before expiry to refresh
    sse_enabled: bool,              // Enable SSE notifications
}

impl Config {
    fn from_env() -> Self {
        let server_url =
            env::var("PIERRE_MCP_URL").unwrap_or_else(|_| "http://127.0.0.1:8080/mcp".to_string());

        // Extract auth URL from MCP URL by replacing /mcp with /api/auth
        let server_auth_url = server_url.replace("/mcp", "/api/auth");

        // Extract SSE URL from MCP URL by replacing /mcp with /api/notifications/sse
        let sse_url = server_url.replace("/mcp", "/api/notifications/sse");

        let jwt_token = env::var("PIERRE_JWT_TOKEN").ok();

        let timeout_seconds = env::var("PIERRE_MCP_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        let refresh_enabled = env::var("PIERRE_AUTO_REFRESH")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        let refresh_threshold_minutes = env::var("PIERRE_REFRESH_THRESHOLD_MINUTES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5); // Default 5 minutes before expiry

        let sse_enabled = env::var("PIERRE_SSE_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        Self {
            server_url,
            server_auth_url,
            sse_url,
            jwt_token,
            timeout_seconds,
            refresh_enabled,
            refresh_threshold_minutes,
            sse_enabled,
        }
    }

    fn validate(&self) -> Result<()> {
        if self.jwt_token.is_none() {
            anyhow::bail!("PIERRE_JWT_TOKEN environment variable is required");
        }
        Ok(())
    }
}

/// JWT claims structure for token parsing
#[derive(Debug, serde::Deserialize)]
struct Claims {
    sub: String, // User ID
    #[allow(dead_code)] // Used for user info logging
    email: String, // User email
    exp: i64,    // Expiration timestamp
    #[allow(dead_code)] // Used for token age validation
    iat: i64, // Issued at timestamp
}

/// Token refresh response from server
#[derive(Debug, serde::Deserialize)]
struct RefreshResponse {
    jwt_token: String,
    #[allow(dead_code)] // Used for token expiry display
    expires_at: String,
}

/// Shared token state with thread-safe access
#[derive(Debug, Clone)]
struct TokenState {
    token: String,
    user_id: String,
    expires_at: DateTime<Utc>,
}

struct McpClient {
    client: Client,
    config: Config,
    token_state: Arc<RwLock<Option<TokenState>>>,
}

impl McpClient {
    fn new(config: Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .context("Failed to create HTTP client")?;

        // Initialize token state from config
        let token_state = if let Some(token) = &config.jwt_token {
            Self::parse_initial_token(token)?
        } else {
            None
        };

        Ok(Self {
            client,
            config,
            token_state: Arc::new(RwLock::new(token_state)),
        })
    }

    /// Parse the initial JWT token to extract user information and expiry
    fn parse_initial_token(token: &str) -> Result<Option<TokenState>> {
        // Decode JWT token without verification (just to get claims)
        let token_parts: Vec<&str> = token.split('.').collect();
        if token_parts.len() != 3 {
            return Ok(None);
        }

        // Decode the payload (middle part)
        let payload = token_parts[1];
        let decoded = general_purpose::URL_SAFE_NO_PAD
            .decode(payload)
            .with_context(|| "Failed to decode JWT payload")?;

        let claims: Claims =
            serde_json::from_slice(&decoded).with_context(|| "Failed to parse JWT claims")?;

        let expires_at = DateTime::from_timestamp(claims.exp, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid expiration timestamp in JWT"))?;

        Ok(Some(TokenState {
            token: token.to_string(),
            user_id: claims.sub,
            expires_at,
        }))
    }

    /// Check if current token needs refresh (within threshold of expiry)
    async fn needs_refresh(&self) -> bool {
        if !self.config.refresh_enabled {
            return false;
        }

        let token_state = self.token_state.read().await;
        token_state.as_ref().is_some_and(|state| {
            let threshold =
                Utc::now() + chrono::Duration::minutes(self.config.refresh_threshold_minutes);
            threshold >= state.expires_at
        })
    }

    /// Refresh the JWT token using the server's refresh endpoint
    async fn refresh_token(&self) -> Result<bool> {
        let current_state = {
            let token_state = self.token_state.read().await;
            token_state.clone()
        };

        let Some(state) = current_state else {
            return Ok(false); // No token to refresh
        };

        tracing::debug!("Refreshing JWT token for user: {}", state.user_id);

        let refresh_request = json!({
            "token": state.token,
            "user_id": state.user_id
        });

        let response = self
            .client
            .post(format!("{}/refresh", self.config.server_auth_url))
            .header("Content-Type", "application/json")
            .json(&refresh_request)
            .send()
            .await
            .with_context(|| "Failed to send token refresh request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::warn!("Token refresh failed: {} - {}", status, error_text);
            return Ok(false);
        }

        let refresh_response: RefreshResponse = response
            .json()
            .await
            .with_context(|| "Failed to parse token refresh response")?;

        // Parse the new token
        let new_token_state = Self::parse_initial_token(&refresh_response.jwt_token)?
            .ok_or_else(|| anyhow::anyhow!("Invalid token received from refresh"))?;

        // Update stored token state
        {
            let mut token_state = self.token_state.write().await;
            *token_state = Some(new_token_state);
        }

        tracing::info!(
            "JWT token refreshed successfully for user: {}",
            state.user_id
        );
        Ok(true)
    }

    /// Get current valid token, refreshing if necessary
    async fn get_valid_token(&self) -> Result<Option<String>> {
        // Check if we need to refresh
        if self.needs_refresh().await {
            if let Err(e) = self.refresh_token().await {
                tracing::error!("Failed to refresh token: {}", e);
                // Continue with current token - might still work
            }
        }

        let token_state = self.token_state.read().await;
        Ok(token_state.as_ref().map(|state| state.token.clone()))
    }

    /// Start background token refresh task
    fn start_refresh_task(self: Arc<Self>) {
        if !self.config.refresh_enabled {
            return;
        }

        let mut refresh_interval = interval(Duration::from_secs(60)); // Check every minute

        tokio::spawn(async move {
            loop {
                refresh_interval.tick().await;

                if self.needs_refresh().await {
                    if let Err(e) = self.refresh_token().await {
                        tracing::error!("Background token refresh failed: {}", e);
                    }
                }
            }
        });
    }

    /// Start SSE notification listener
    async fn start_sse_listener(self: Arc<Self>) -> Result<()> {
        if !self.config.sse_enabled {
            tracing::debug!("SSE notifications disabled");
            return Ok(());
        }

        let token_state = self.token_state.read().await;
        let Some(state) = token_state.as_ref() else {
            tracing::warn!("No token available for SSE connection");
            return Ok(());
        };

        let user_id = state.user_id.clone();
        drop(token_state);

        let sse_url = format!("{}?user_id={}", self.config.sse_url, user_id);
        tracing::info!("Starting SSE listener: {}", sse_url);

        let client = self.client.clone();
        tokio::spawn(async move {
            loop {
                match Self::connect_sse(&client, &sse_url).await {
                    Ok(()) => {
                        tracing::info!("SSE connection ended, reconnecting in 5 seconds...");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                    Err(e) => {
                        tracing::error!("SSE connection failed: {}, retrying in 10 seconds...", e);
                        tokio::time::sleep(Duration::from_secs(10)).await;
                    }
                }
            }
        });

        Ok(())
    }

    /// Connect to SSE stream and handle notifications
    async fn connect_sse(client: &Client, sse_url: &str) -> Result<()> {
        let response = client
            .get(sse_url)
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("SSE connection failed with status: {}", response.status());
        }

        tracing::info!("SSE connection established, listening for OAuth notifications");

        // Convert the response body to an SSE stream
        let byte_stream = response.bytes_stream();
        let mut event_stream = byte_stream.eventsource();

        // Process each SSE event
        while let Some(event) = event_stream.next().await {
            match event {
                Ok(event) => {
                    if !event.data.is_empty() {
                        tracing::info!("Received SSE event: {}", event.data);

                        // Parse the OAuth notification
                        if let Ok(notification) = serde_json::from_str::<Value>(&event.data) {
                            if let Some(event_type) =
                                notification.get("type").and_then(|t| t.as_str())
                            {
                                if event_type == "oauth_notification" {
                                    let message = notification
                                        .get("message")
                                        .and_then(|m| m.as_str())
                                        .unwrap_or("OAuth notification received");
                                    let provider = notification
                                        .get("provider")
                                        .and_then(|p| p.as_str())
                                        .unwrap_or("unknown");

                                    tracing::info!(
                                        "OAuth notification from {}: {}",
                                        provider,
                                        message
                                    );
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("SSE stream error: {}", e);
                    break;
                }
            }
        }

        tracing::info!("SSE stream ended");
        Ok(())
    }

    /// Run HTTP-only MCP client
    async fn run_http_client(&self) -> Result<()> {
        tracing::info!("Starting Pierre MCP HTTP client");

        // Keep the client running to maintain connections
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;

            // Health check - ensure we still have a valid token
            if let Err(e) = self.get_valid_token().await {
                tracing::error!("Token validation failed: {}", e);
                break;
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = Config::from_env();

    // Validate configuration
    config.validate().context("Invalid configuration")?;

    // Create and run client
    let client = Arc::new(McpClient::new(config).context("Failed to create MCP client")?);

    // Start background token refresh task
    client.clone().start_refresh_task();

    // Start SSE listener for OAuth notifications
    if let Err(e) = client.clone().start_sse_listener().await {
        tracing::warn!("Failed to start SSE listener: {}", e);
    }

    // Run HTTP client
    client
        .run_http_client()
        .await
        .context("MCP HTTP client failed")?;

    Ok(())
}
