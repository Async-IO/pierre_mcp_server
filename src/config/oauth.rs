// ABOUTME: OAuth configuration types for fitness provider authentication
// ABOUTME: Handles Strava, Fitbit, Garmin, WHOOP, Terra OAuth and Firebase auth settings
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::constants::{oauth, oauth_providers};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use tracing::{debug, info, warn};

/// OAuth provider configuration for fitness platforms
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OAuthConfig {
    /// Strava OAuth configuration
    pub strava: OAuthProviderConfig,
    /// Fitbit OAuth configuration
    pub fitbit: OAuthProviderConfig,
    /// Garmin OAuth configuration
    pub garmin: OAuthProviderConfig,
    /// WHOOP OAuth configuration
    pub whoop: OAuthProviderConfig,
    /// Terra OAuth configuration
    pub terra: OAuthProviderConfig,
}

impl OAuthConfig {
    /// Load OAuth configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            strava: OAuthProviderConfig::load_strava(),
            fitbit: OAuthProviderConfig::load_fitbit(),
            garmin: OAuthProviderConfig::load_garmin(),
            whoop: OAuthProviderConfig::load_whoop(),
            terra: OAuthProviderConfig::load_terra(),
        }
    }
}

/// OAuth provider-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OAuthProviderConfig {
    /// OAuth client ID
    pub client_id: Option<String>,
    /// OAuth client secret
    pub client_secret: Option<String>,
    /// OAuth redirect URI
    pub redirect_uri: Option<String>,
    /// OAuth scopes
    pub scopes: Vec<String>,
    /// Enable this provider
    pub enabled: bool,
}

impl OAuthProviderConfig {
    /// Compute SHA256 fingerprint of client secret for debugging (first 8 hex chars)
    /// This allows comparing secrets without logging actual values
    #[must_use]
    pub fn secret_fingerprint(&self) -> Option<String> {
        self.client_secret.as_ref().map(|secret| {
            let mut hasher = Sha256::new();
            hasher.update(secret.as_bytes());
            let result = hasher.finalize();
            format!("{result:x}").chars().take(8).collect()
        })
    }

    /// Validate OAuth credentials and log diagnostics
    /// Returns true if credentials appear valid, false otherwise
    pub fn validate_and_log(&self, provider_name: &str) -> bool {
        if !self.enabled {
            info!("OAuth provider {provider_name} is disabled");
            return true; // Disabled is valid state
        }

        let Some(client_id) = self.validate_client_id(provider_name) else {
            return false;
        };

        let Some(client_secret) = self.validate_client_secret(provider_name) else {
            return false;
        };

        self.log_credential_diagnostics(provider_name, client_id, client_secret);
        Self::validate_secret_length(provider_name, client_secret)
    }

    /// Validate client ID is present and non-empty
    fn validate_client_id(&self, provider_name: &str) -> Option<&str> {
        match &self.client_id {
            Some(id) if !id.is_empty() => Some(id.as_str()),
            _ => {
                warn!("OAuth provider {provider_name}: client_id is missing or empty");
                None
            }
        }
    }

    /// Validate client secret is present and non-empty
    fn validate_client_secret(&self, provider_name: &str) -> Option<&str> {
        match &self.client_secret {
            Some(secret) if !secret.is_empty() => Some(secret.as_str()),
            _ => {
                warn!("OAuth provider {provider_name}: client_secret is missing or empty");
                None
            }
        }
    }

    /// Log OAuth credential diagnostics (fingerprint, lengths, etc.)
    fn log_credential_diagnostics(
        &self,
        provider_name: &str,
        client_id: &str,
        client_secret: &str,
    ) {
        let fingerprint = self
            .secret_fingerprint()
            .unwrap_or_else(|| "none".to_owned());
        info!(
            "OAuth provider {provider_name}: enabled=true, client_id={client_id}, \
             secret_length={}, secret_fingerprint={fingerprint}",
            client_secret.len()
        );
    }

    /// Validate secret length meets minimum requirements
    fn validate_secret_length(provider_name: &str, client_secret: &str) -> bool {
        if client_secret.len() < 20 {
            warn!(
                "OAuth provider {provider_name}: client_secret is unusually short ({} chars) - \
                 this may indicate a configuration error",
                client_secret.len()
            );
            return false;
        }
        true
    }

    /// Load Strava OAuth configuration from environment
    #[must_use]
    pub fn load_strava() -> Self {
        let base_url = env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8081".to_owned());
        Self {
            client_id: env::var("STRAVA_CLIENT_ID").ok(),
            client_secret: env::var("STRAVA_CLIENT_SECRET").ok(),
            redirect_uri: Some(
                env::var("STRAVA_REDIRECT_URI")
                    .unwrap_or_else(|_| format!("{base_url}/auth/strava/callback")),
            ),
            scopes: parse_scopes(oauth::STRAVA_DEFAULT_SCOPES),
            enabled: env::var("STRAVA_CLIENT_ID").is_ok()
                && env::var("STRAVA_CLIENT_SECRET").is_ok(),
        }
    }

    /// Load Fitbit OAuth configuration from environment
    #[must_use]
    pub fn load_fitbit() -> Self {
        let base_url = env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8081".to_owned());
        Self {
            client_id: env::var("FITBIT_CLIENT_ID").ok(),
            client_secret: env::var("FITBIT_CLIENT_SECRET").ok(),
            redirect_uri: Some(
                env::var("FITBIT_REDIRECT_URI")
                    .unwrap_or_else(|_| format!("{base_url}/auth/fitbit/callback")),
            ),
            scopes: parse_scopes(oauth::FITBIT_DEFAULT_SCOPES),
            enabled: env::var("FITBIT_CLIENT_ID").is_ok()
                && env::var("FITBIT_CLIENT_SECRET").is_ok(),
        }
    }

    /// Load Garmin OAuth configuration from environment
    #[must_use]
    pub fn load_garmin() -> Self {
        let base_url = env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8081".to_owned());
        Self {
            client_id: env::var("GARMIN_CLIENT_ID").ok(),
            client_secret: env::var("GARMIN_CLIENT_SECRET").ok(),
            redirect_uri: Some(
                env::var("GARMIN_REDIRECT_URI")
                    .unwrap_or_else(|_| format!("{base_url}/api/oauth/callback/garmin")),
            ),
            scopes: vec![],
            enabled: env::var("GARMIN_CLIENT_ID").is_ok()
                && env::var("GARMIN_CLIENT_SECRET").is_ok(),
        }
    }

    /// Load WHOOP OAuth configuration from environment
    #[must_use]
    pub fn load_whoop() -> Self {
        let base_url = env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8081".to_owned());
        Self {
            client_id: env::var("WHOOP_CLIENT_ID").ok(),
            client_secret: env::var("WHOOP_CLIENT_SECRET").ok(),
            redirect_uri: Some(
                env::var("WHOOP_REDIRECT_URI")
                    .unwrap_or_else(|_| format!("{base_url}/auth/whoop/callback")),
            ),
            scopes: parse_scopes(oauth::WHOOP_DEFAULT_SCOPES),
            enabled: env::var("WHOOP_CLIENT_ID").is_ok() && env::var("WHOOP_CLIENT_SECRET").is_ok(),
        }
    }

    /// Load Terra OAuth configuration from environment
    #[must_use]
    pub fn load_terra() -> Self {
        let base_url = env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8081".to_owned());
        Self {
            client_id: env::var("TERRA_DEV_ID").ok(),
            client_secret: env::var("TERRA_API_KEY").ok(),
            redirect_uri: Some(
                env::var("TERRA_REDIRECT_URI")
                    .unwrap_or_else(|_| format!("{base_url}/auth/terra/callback")),
            ),
            scopes: parse_scopes(oauth::TERRA_DEFAULT_SCOPES),
            enabled: env::var("TERRA_DEV_ID").is_ok() && env::var("TERRA_API_KEY").is_ok(),
        }
    }
}

/// `OAuth2` authorization server configuration (for Pierre acting as OAuth server)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2ServerConfig {
    /// `OAuth2` issuer URL for RFC 8414 discovery (format: <https://your-domain.com>)
    /// MUST be set in production to actual deployment domain. Defaults to `http://localhost:PORT` in development.
    pub issuer_url: String,
    /// Default email for OAuth login page (dev/test only - do not use in production)
    pub default_login_email: Option<String>,
    /// Default password for OAuth login page (dev/test only - NEVER use in production!)
    pub default_login_password: Option<String>,
}

impl Default for OAuth2ServerConfig {
    fn default() -> Self {
        Self {
            issuer_url: "http://localhost:8081".to_owned(),
            default_login_email: None,
            default_login_password: None,
        }
    }
}

impl OAuth2ServerConfig {
    /// Load `OAuth2` authorization server configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        let http_port = env::var("HTTP_PORT")
            .or_else(|_| env::var("MCP_PORT"))
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8081);
        Self {
            issuer_url: env::var("OAUTH2_ISSUER_URL")
                .unwrap_or_else(|_| format!("http://localhost:{http_port}")),
            default_login_email: env::var("OAUTH_DEFAULT_EMAIL").ok(),
            default_login_password: env::var("OAUTH_DEFAULT_PASSWORD").ok(),
        }
    }
}

/// Firebase Authentication configuration for social logins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirebaseConfig {
    /// Firebase project ID (required for token validation)
    pub project_id: Option<String>,
    /// Firebase API key (optional, for client-side SDK)
    pub api_key: Option<String>,
    /// Whether Firebase authentication is enabled
    pub enabled: bool,
    /// Cache TTL for Firebase public keys in seconds (default: 3600 = 1 hour)
    pub key_cache_ttl_secs: u64,
}

impl Default for FirebaseConfig {
    fn default() -> Self {
        Self {
            project_id: None,
            api_key: None,
            enabled: false,
            key_cache_ttl_secs: 3600, // 1 hour - Firebase keys are rotated daily
        }
    }
}

impl FirebaseConfig {
    /// Check if Firebase is properly configured and enabled
    /// Returns `true` if Firebase is enabled and has a project ID configured
    #[must_use]
    pub const fn is_configured(&self) -> bool {
        self.enabled && self.project_id.is_some()
    }

    /// Load Firebase configuration from environment
    ///
    /// Environment variables:
    /// - `FIREBASE_PROJECT_ID` - Firebase project ID (required for token validation)
    /// - `FIREBASE_API_KEY` - Firebase API key (optional, for client-side SDK)
    /// - `FIREBASE_ENABLED` - Enable Firebase authentication (default: false)
    /// - `FIREBASE_KEY_CACHE_TTL_SECS` - Public key cache TTL (default: 3600)
    #[must_use]
    pub fn from_env() -> Self {
        let project_id = env::var("FIREBASE_PROJECT_ID").ok();
        let api_key = env::var("FIREBASE_API_KEY").ok();

        // Firebase is enabled if project_id is set and FIREBASE_ENABLED is not explicitly false
        let enabled = project_id.is_some()
            && env_var_or("FIREBASE_ENABLED", "true")
                .parse()
                .unwrap_or(true);

        if enabled {
            info!(
                project_id = project_id.as_deref().unwrap_or("(not set)"),
                "Firebase authentication enabled"
            );
        }

        Self {
            project_id,
            api_key,
            enabled,
            key_cache_ttl_secs: env::var("FIREBASE_KEY_CACHE_TTL_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600), // 1 hour default
        }
    }
}

/// Get the default provider from environment or use synthetic as fallback
///
/// Reads the `PIERRE_DEFAULT_PROVIDER` environment variable.
/// Falls back to "synthetic" if not set, making it ideal for development.
///
/// # Examples
///
/// ```bash
/// # Use Strava as default
/// export PIERRE_DEFAULT_PROVIDER=strava
///
/// # Use synthetic (no export needed, this is the default)
/// # PIERRE_DEFAULT_PROVIDER=synthetic
/// ```
#[must_use]
pub fn default_provider() -> String {
    let provider = env::var("PIERRE_DEFAULT_PROVIDER")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| oauth_providers::SYNTHETIC.to_owned());

    info!("Default provider configured: {}", provider);
    provider
}

/// Get OAuth provider configuration by provider name
///
/// Returns the `OAuthProviderConfig` for the specified provider.
/// For unknown providers, returns a default (empty) config.
///
/// # Arguments
/// * `provider_name` - The provider name (e.g., "strava", "garmin", "fitbit")
#[must_use]
pub fn get_oauth_config(provider_name: &str) -> OAuthProviderConfig {
    fn parse_scopes_with_defaults(env_value: Option<String>, defaults: Vec<String>) -> Vec<String> {
        env_value.map_or(defaults, |s| s.split(',').map(str::to_owned).collect())
    }

    match provider_name {
        p if p == oauth_providers::STRAVA => {
            let client_id = env::var("STRAVA_CLIENT_ID")
                .ok()
                .or_else(|| env::var("PIERRE_STRAVA_CLIENT_ID").ok());
            let client_secret = env::var("STRAVA_CLIENT_SECRET")
                .ok()
                .or_else(|| env::var("PIERRE_STRAVA_CLIENT_SECRET").ok());
            let scopes_env = env::var("PIERRE_STRAVA_SCOPES")
                .ok()
                .or_else(|| env::var("STRAVA_SCOPES").ok());
            let scopes =
                parse_scopes_with_defaults(scopes_env, vec!["activity:read_all".to_owned()]);

            OAuthProviderConfig {
                client_id,
                client_secret,
                redirect_uri: env::var("STRAVA_REDIRECT_URI").ok(),
                scopes,
                enabled: true,
            }
        }
        p if p == oauth_providers::GARMIN => {
            let client_id = env::var("GARMIN_CLIENT_ID")
                .ok()
                .or_else(|| env::var("PIERRE_GARMIN_CLIENT_ID").ok());
            let client_secret = env::var("GARMIN_CLIENT_SECRET")
                .ok()
                .or_else(|| env::var("PIERRE_GARMIN_CLIENT_SECRET").ok());
            let scopes_env = env::var("PIERRE_GARMIN_SCOPES").ok();
            let scopes = parse_scopes_with_defaults(
                scopes_env,
                vec!["activity:read".to_owned(), "sleep:read".to_owned()],
            );

            OAuthProviderConfig {
                client_id,
                client_secret,
                redirect_uri: env::var("GARMIN_REDIRECT_URI").ok(),
                scopes,
                enabled: true,
            }
        }
        "fitbit" => {
            let client_id = env::var("FITBIT_CLIENT_ID")
                .ok()
                .or_else(|| env::var("PIERRE_FITBIT_CLIENT_ID").ok());
            let client_secret = env::var("FITBIT_CLIENT_SECRET")
                .ok()
                .or_else(|| env::var("PIERRE_FITBIT_CLIENT_SECRET").ok());
            let scopes_env = env::var("PIERRE_FITBIT_SCOPES").ok();
            let scopes = parse_scopes_with_defaults(
                scopes_env,
                vec!["activity".to_owned(), "sleep".to_owned()],
            );

            OAuthProviderConfig {
                client_id,
                client_secret,
                redirect_uri: env::var("FITBIT_REDIRECT_URI").ok(),
                scopes,
                enabled: true,
            }
        }
        _ => {
            debug!(
                "Unknown provider '{}', returning default config",
                provider_name
            );
            OAuthProviderConfig::default()
        }
    }
}

/// Provider configuration tuple from environment variables
///
/// Returns: (`client_id`, `client_secret`, `auth_url`, `token_url`, `api_base_url`, `revoke_url`, `scopes`)
pub type ProviderEnvConfig = (
    Option<String>,
    Option<String>,
    String,
    String,
    String,
    Option<String>,
    Vec<String>,
);

/// Load provider-specific configuration from environment variables
///
/// Reads provider configuration from `PIERRE_<PROVIDER>_*` environment variables.
/// Falls back to provided defaults if environment variables are not set.
///
/// # Environment Variables
///
/// For each provider (e.g., STRAVA, GARMIN):
/// - `PIERRE_<PROVIDER>_CLIENT_ID` - OAuth client ID (falls back to legacy var)
/// - `PIERRE_<PROVIDER>_CLIENT_SECRET` - OAuth client secret (falls back to legacy var)
/// - `PIERRE_<PROVIDER>_AUTH_URL` - OAuth authorization URL (optional)
/// - `PIERRE_<PROVIDER>_TOKEN_URL` - OAuth token URL (optional)
/// - `PIERRE_<PROVIDER>_API_BASE_URL` - Provider API base URL (optional)
/// - `PIERRE_<PROVIDER>_REVOKE_URL` - Token revocation URL (optional)
/// - `PIERRE_<PROVIDER>_SCOPES` - Comma-separated scopes (optional)
///
/// # Examples
///
/// ```bash
/// # Strava configuration
/// export PIERRE_STRAVA_CLIENT_ID=your_client_id
/// export PIERRE_STRAVA_CLIENT_SECRET=your_secret
/// export PIERRE_STRAVA_SCOPES="activity:read_all,profile:read_all"
///
/// # Garmin configuration (optional URLs override defaults)
/// export PIERRE_GARMIN_CLIENT_ID=your_consumer_key
/// export PIERRE_GARMIN_CLIENT_SECRET=your_consumer_secret
/// export PIERRE_GARMIN_API_BASE_URL=https://custom-garmin-api.example.com
/// ```
#[must_use]
pub fn load_provider_env_config(
    provider: &str,
    default_auth_url: &str,
    default_token_url: &str,
    default_api_base_url: &str,
    default_revoke_url: Option<&str>,
    default_scopes: &[String],
) -> ProviderEnvConfig {
    let provider_upper = provider.to_uppercase();

    // Load client credentials with fallback to legacy env vars (backward compatible)
    let client_id = env::var(format!("PIERRE_{provider_upper}_CLIENT_ID"))
        .or_else(|_| env::var(format!("{provider_upper}_CLIENT_ID")))
        .ok();

    let client_secret = env::var(format!("PIERRE_{provider_upper}_CLIENT_SECRET"))
        .or_else(|_| env::var(format!("{provider_upper}_CLIENT_SECRET")))
        .ok();

    // Load URLs with defaults
    let auth_url = env::var(format!("PIERRE_{provider_upper}_AUTH_URL"))
        .unwrap_or_else(|_| default_auth_url.to_owned());

    let token_url = env::var(format!("PIERRE_{provider_upper}_TOKEN_URL"))
        .unwrap_or_else(|_| default_token_url.to_owned());

    let api_base_url = env::var(format!("PIERRE_{provider_upper}_API_BASE_URL"))
        .unwrap_or_else(|_| default_api_base_url.to_owned());

    let revoke_url = env::var(format!("PIERRE_{provider_upper}_REVOKE_URL"))
        .ok()
        .or_else(|| default_revoke_url.map(ToOwned::to_owned));

    // Load scopes with default
    let scopes = env::var(format!("PIERRE_{provider_upper}_SCOPES"))
        .ok()
        .map_or_else(
            || default_scopes.to_vec(),
            |s| s.split(',').map(|scope| scope.trim().to_owned()).collect(),
        );

    (
        client_id,
        client_secret,
        auth_url,
        token_url,
        api_base_url,
        revoke_url,
        scopes,
    )
}

/// Parse comma-separated scopes
#[must_use]
pub fn parse_scopes(scopes_str: &str) -> Vec<String> {
    scopes_str
        .split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Get environment variable or default value
fn env_var_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_owned())
}
