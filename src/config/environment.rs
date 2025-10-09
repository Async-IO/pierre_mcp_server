// ABOUTME: Environment configuration management for deployment-specific settings
// ABOUTME: Handles environment variables, deployment modes, and runtime configuration parsing
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Environment-based configuration management for production deployment

use crate::constants::{defaults, env_config, limits, oauth};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use tracing::{info, warn};

/// Strongly typed log level configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    /// Convert to `tracing::Level`
    #[must_use]
    pub const fn to_tracing_level(&self) -> tracing::Level {
        match self {
            Self::Error => tracing::Level::ERROR,
            Self::Warn => tracing::Level::WARN,
            Self::Info => tracing::Level::INFO,
            Self::Debug => tracing::Level::DEBUG,
            Self::Trace => tracing::Level::TRACE,
        }
    }

    /// Parse from string with fallback
    #[must_use]
    pub fn from_str_or_default(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "error" => Self::Error,
            "warn" => Self::Warn,
            "debug" => Self::Debug,
            "trace" => Self::Trace,
            _ => Self::Info, // Default fallback (including "info")
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warn => write!(f, "warn"),
            Self::Info => write!(f, "info"),
            Self::Debug => write!(f, "debug"),
            Self::Trace => write!(f, "trace"),
        }
    }
}

/// Environment type for security and other configurations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    #[default]
    Development,
    Production,
    Testing,
}

impl Environment {
    /// Parse from string with fallback
    #[must_use]
    pub fn from_str_or_default(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "production" | "prod" => Self::Production,
            "testing" | "test" => Self::Testing,
            _ => Self::Development, // Default fallback (including "development" | "dev")
        }
    }

    /// Check if this is a production environment
    #[must_use]
    pub const fn is_production(&self) -> bool {
        matches!(self, Self::Production)
    }

    /// Check if this is a development environment
    #[must_use]
    pub const fn is_development(&self) -> bool {
        matches!(self, Self::Development)
    }

    /// Check if this is a testing environment
    #[must_use]
    pub const fn is_testing(&self) -> bool {
        matches!(self, Self::Testing)
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Development => write!(f, "development"),
            Self::Production => write!(f, "production"),
            Self::Testing => write!(f, "testing"),
        }
    }
}

/// Type-safe database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseUrl {
    /// `SQLite` database with file path
    SQLite { path: PathBuf },
    /// `PostgreSQL` connection
    PostgreSQL { connection_string: String },
    /// In-memory `SQLite` (for testing)
    Memory,
}

impl DatabaseUrl {
    /// Parse from string with validation
    ///
    /// # Errors
    ///
    /// Returns an error if the database URL format is invalid or unsupported
    pub fn parse_url(s: &str) -> Result<Self> {
        if s.starts_with("sqlite:") {
            let path_str = s.strip_prefix("sqlite:").unwrap_or(s);
            if path_str == ":memory:" {
                Ok(Self::Memory)
            } else {
                Ok(Self::SQLite {
                    path: PathBuf::from(path_str),
                })
            }
        } else if s.starts_with("postgresql://") || s.starts_with("postgres://") {
            Ok(Self::PostgreSQL {
                connection_string: s.to_string(),
            })
        } else {
            // Fallback: treat as SQLite file path
            Ok(Self::SQLite {
                path: PathBuf::from(s),
            })
        }
    }

    /// Convert to connection string
    #[must_use]
    pub fn to_connection_string(&self) -> String {
        match self {
            Self::SQLite { path } => format!("sqlite:{}", path.display()),
            Self::PostgreSQL { connection_string } => connection_string.clone(),
            Self::Memory => "sqlite::memory:".into(),
        }
    }

    /// Check if this is an in-memory database
    #[must_use]
    pub const fn is_memory(&self) -> bool {
        matches!(self, Self::Memory)
    }

    /// Check if this is a `SQLite` database
    #[must_use]
    pub const fn is_sqlite(&self) -> bool {
        matches!(self, Self::SQLite { .. } | Self::Memory)
    }

    /// Check if this is a `PostgreSQL` database
    #[must_use]
    pub const fn is_postgresql(&self) -> bool {
        matches!(self, Self::PostgreSQL { .. })
    }
}

impl Default for DatabaseUrl {
    fn default() -> Self {
        Self::SQLite {
            path: PathBuf::from("./data/users.db"),
        }
    }
}

impl std::fmt::Display for DatabaseUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_connection_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerConfig {
    /// Server port (handles both MCP and HTTP)
    pub http_port: u16,
    /// OAuth callback port for bridge focus recovery
    pub oauth_callback_port: u16,
    /// Log level
    pub log_level: LogLevel,
    /// Database configuration
    pub database: DatabaseConfig,
    /// Authentication configuration
    pub auth: AuthConfig,
    /// OAuth provider configurations
    pub oauth: OAuthConfig,
    /// Security settings
    pub security: SecurityConfig,
    /// External service configuration
    pub external_services: ExternalServicesConfig,
    /// Application behavior settings
    pub app_behavior: AppBehaviorConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatabaseConfig {
    /// Database URL (`SQLite` path or `PostgreSQL` connection string)
    pub url: DatabaseUrl,
    /// Path to encryption key file
    pub encryption_key_path: PathBuf,
    /// Enable database migrations on startup
    pub auto_migrate: bool,
    /// Database backup configuration
    pub backup: BackupConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackupConfig {
    /// Enable automatic backups
    pub enabled: bool,
    /// Backup interval in seconds
    pub interval_seconds: u64,
    /// Number of backups to retain
    pub retention_count: u32,
    /// Backup directory path
    pub directory: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthConfig {
    /// JWT secret key path
    pub jwt_secret_path: PathBuf,
    /// JWT expiry time in hours
    pub jwt_expiry_hours: u64,
    /// Enable JWT refresh tokens
    pub enable_refresh_tokens: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OAuthConfig {
    /// Strava OAuth configuration
    pub strava: OAuthProviderConfig,
    /// Fitbit OAuth configuration  
    pub fitbit: OAuthProviderConfig,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityConfig {
    /// CORS allowed origins
    pub cors_origins: Vec<String>,
    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,
    /// TLS configuration
    pub tls: TlsConfig,
    /// Security headers configuration
    pub headers: SecurityHeadersConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityHeadersConfig {
    /// Environment type for security headers (development, production)
    pub environment: Environment,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    pub enabled: bool,
    /// Requests per window
    pub requests_per_window: u32,
    /// Window duration in seconds
    pub window_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TlsConfig {
    /// Enable TLS
    pub enabled: bool,
    /// Path to TLS certificate
    pub cert_path: Option<PathBuf>,
    /// Path to TLS private key
    pub key_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExternalServicesConfig {
    /// Weather service configuration
    pub weather: WeatherServiceConfig,
    /// Geocoding service configuration
    pub geocoding: GeocodingServiceConfig,
    /// Strava API configuration
    pub strava_api: StravaApiConfig,
    /// Fitbit API configuration  
    pub fitbit_api: FitbitApiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WeatherServiceConfig {
    /// `OpenWeather` API key
    pub api_key: Option<String>,
    /// Weather service base URL
    pub base_url: String,
    /// Enable weather service
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeocodingServiceConfig {
    /// Geocoding service base URL
    pub base_url: String,
    /// Enable geocoding service
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StravaApiConfig {
    /// Strava API base URL
    pub base_url: String,
    /// Strava auth URL
    pub auth_url: String,
    /// Strava token URL
    pub token_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FitbitApiConfig {
    /// Fitbit API base URL
    pub base_url: String,
    /// Fitbit auth URL
    pub auth_url: String,
    /// Fitbit token URL
    pub token_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppBehaviorConfig {
    /// Maximum activities to fetch in one request
    pub max_activities_fetch: usize,
    /// Default limit for activities queries
    pub default_activities_limit: usize,
    /// Enable CI mode for testing
    pub ci_mode: bool,
    /// Protocol configuration
    pub protocol: ProtocolConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProtocolConfig {
    /// MCP protocol version
    pub mcp_version: String,
    /// Server name
    pub server_name: String,
    /// Server version (from Cargo.toml)
    pub server_version: String,
}

impl ServerConfig {
    /// Load configuration from environment variables
    ///
    /// # Errors
    ///
    /// Returns an error if environment variables contain invalid values or required configuration is missing
    pub fn from_env() -> Result<Self> {
        Self::initialize_environment();

        let config = Self {
            http_port: env_config::server_port(),
            oauth_callback_port: env_config::oauth_callback_port(),
            log_level: LogLevel::from_str_or_default(&env_config::log_level()),
            database: Self::load_database_config()?,
            auth: Self::load_auth_config()?,
            oauth: Self::load_oauth_config(),
            security: Self::load_security_config()?,
            external_services: Self::load_external_services_config()?,
            app_behavior: Self::load_app_behavior_config()?,
        };

        config.validate()?;
        info!("Configuration loaded successfully");
        Ok(config)
    }

    /// Validate configuration values
    ///
    /// # Errors
    ///
    /// Returns an error if configuration values are invalid or conflicting
    pub fn validate(&self) -> Result<()> {
        // Single-port architecture - no port conflicts possible

        // Database validation - URLs are now type-safe, so no need to check emptiness

        // OAuth validation
        if self.oauth.strava.enabled
            && (self.oauth.strava.client_id.is_none() || self.oauth.strava.client_secret.is_none())
        {
            warn!("Strava OAuth is enabled but missing client_id or client_secret");
        }

        if self.oauth.fitbit.enabled
            && (self.oauth.fitbit.client_id.is_none() || self.oauth.fitbit.client_secret.is_none())
        {
            warn!("Fitbit OAuth is enabled but missing client_id or client_secret");
        }

        // TLS validation
        if self.security.tls.enabled
            && (self.security.tls.cert_path.is_none() || self.security.tls.key_path.is_none())
        {
            return Err(anyhow::anyhow!(
                "TLS is enabled but cert_path or key_path is missing"
            ));
        }

        Ok(())
    }

    /// Initialize all configurations including intelligence config
    ///
    /// # Errors
    ///
    /// Returns an error if intelligence configuration cannot be loaded or validated
    pub fn init_all_configs(&self) -> Result<()> {
        // Initialize intelligence configuration
        let intelligence_config = crate::config::intelligence_config::IntelligenceConfig::global();

        // Validate intelligence configuration is properly loaded by accessing a field
        info!(
            "Intelligence config initialized successfully (min duration: {}s)",
            intelligence_config
                .activity_analyzer
                .analysis
                .min_duration_seconds
        );

        info!("All configurations initialized successfully");
        Ok(())
    }

    /// Get a summary of the configuration for logging (without secrets)
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "Pierre MCP Server Configuration:\n\
             - Server Port: {}\n\
             - Log Level: {}\n\
             - Database: {}\n\
             - Strava OAuth: {}\n\
             - Strava Redirect URI: {}\n\
             - Fitbit OAuth: {}\n\
             - Weather Service: {}\n\
             - TLS: {}\n\
             - Rate Limiting: {}\n\
             - CI Mode: {}\n\
             - Protocol Version: {}",
            self.http_port,
            self.log_level,
            if self.database.url.is_sqlite() {
                "SQLite"
            } else {
                "PostgreSQL"
            },
            "API-Configured",
            crate::constants::env_config::strava_redirect_uri(),
            "API-Configured",
            if self.external_services.weather.enabled
                && self.external_services.weather.api_key.is_some()
            {
                "Enabled"
            } else {
                "Disabled"
            },
            if self.security.tls.enabled {
                "Enabled"
            } else {
                "Disabled"
            },
            if self.security.rate_limit.enabled {
                "Enabled"
            } else {
                "Disabled"
            },
            self.app_behavior.ci_mode,
            self.app_behavior.protocol.mcp_version
        )
    }

    /// Convenience methods for accessing commonly used values
    /// Get the `OpenWeather` API key if available
    #[must_use]
    pub fn openweather_api_key(&self) -> Option<&str> {
        self.external_services.weather.api_key.as_deref()
    }

    /// Get Strava API configuration
    #[must_use]
    pub const fn strava_api_config(&self) -> &StravaApiConfig {
        &self.external_services.strava_api
    }

    /// Get Fitbit API configuration
    #[must_use]
    pub const fn fitbit_api_config(&self) -> &FitbitApiConfig {
        &self.external_services.fitbit_api
    }

    /// Check if CI mode is enabled
    #[must_use]
    pub const fn is_ci_mode(&self) -> bool {
        self.app_behavior.ci_mode
    }

    /// Get protocol information
    #[must_use]
    pub fn protocol_info(&self) -> (&str, &str, &str) {
        (
            &self.app_behavior.protocol.mcp_version,
            &self.app_behavior.protocol.server_name,
            &self.app_behavior.protocol.server_version,
        )
    }

    /// Get activity fetch limits
    #[must_use]
    pub const fn activity_limits(&self) -> (usize, usize) {
        (
            self.app_behavior.max_activities_fetch,
            self.app_behavior.default_activities_limit,
        )
    }
}

impl ServerConfig {
    /// Initialize environment by loading .env file and logging
    fn initialize_environment() {
        info!("Loading configuration from environment variables");

        // Load .env file if it exists
        if let Err(e) = dotenvy::dotenv() {
            warn!("No .env file found or failed to load: {}", e);
        }
    }

    /// Load database configuration from environment
    ///
    /// # Errors
    ///
    /// Returns an error if database environment variables are invalid
    fn load_database_config() -> Result<DatabaseConfig> {
        Ok(DatabaseConfig {
            url: DatabaseUrl::parse_url(&env_config::database_url())
                .unwrap_or_else(|_| DatabaseUrl::default()),
            encryption_key_path: PathBuf::from(env_config::encryption_key_path()),
            auto_migrate: env_var_or("AUTO_MIGRATE", "true")
                .parse()
                .context("Invalid AUTO_MIGRATE value")?,
            backup: Self::load_backup_config()?,
        })
    }

    /// Load backup configuration from environment
    ///
    /// # Errors
    ///
    /// Returns an error if backup environment variables are invalid
    fn load_backup_config() -> Result<BackupConfig> {
        Ok(BackupConfig {
            enabled: env_var_or("BACKUP_ENABLED", "true")
                .parse()
                .context("Invalid BACKUP_ENABLED value")?,
            interval_seconds: env_var_or(
                "BACKUP_INTERVAL",
                &limits::DEFAULT_BACKUP_INTERVAL_SECS.to_string(),
            )
            .parse()
            .context("Invalid BACKUP_INTERVAL value")?,
            retention_count: env_var_or(
                "BACKUP_RETENTION",
                &limits::DEFAULT_BACKUP_RETENTION_COUNT.to_string(),
            )
            .parse()
            .context("Invalid BACKUP_RETENTION value")?,
            directory: PathBuf::from(env_var_or("BACKUP_DIRECTORY", defaults::DEFAULT_BACKUP_DIR)),
        })
    }

    /// Load authentication configuration from environment
    ///
    /// # Errors
    ///
    /// Returns an error if auth environment variables are invalid
    fn load_auth_config() -> Result<AuthConfig> {
        Ok(AuthConfig {
            jwt_secret_path: PathBuf::from(env_config::jwt_secret_path()),
            jwt_expiry_hours: u64::try_from(env_config::jwt_expiry_hours().max(0)).unwrap_or(24),
            enable_refresh_tokens: env_var_or("ENABLE_REFRESH_TOKENS", "false")
                .parse()
                .context("Invalid ENABLE_REFRESH_TOKENS value")?,
        })
    }

    /// Load OAuth configuration from environment
    fn load_oauth_config() -> OAuthConfig {
        OAuthConfig {
            strava: Self::load_strava_oauth_config(),
            fitbit: Self::load_fitbit_oauth_config(),
        }
    }

    /// Load Strava OAuth configuration from environment (disabled for tenant-based OAuth)
    fn load_strava_oauth_config() -> OAuthProviderConfig {
        // Use environment variables for global provider registration
        OAuthProviderConfig {
            client_id: env::var("STRAVA_CLIENT_ID").ok(),
            client_secret: env::var("STRAVA_CLIENT_SECRET").ok(),
            redirect_uri: Some(crate::constants::env_config::strava_redirect_uri()),
            scopes: parse_scopes(oauth::STRAVA_DEFAULT_SCOPES),
            enabled: env::var("STRAVA_CLIENT_ID").is_ok()
                && env::var("STRAVA_CLIENT_SECRET").is_ok(),
        }
    }

    /// Load Fitbit OAuth configuration from environment (disabled for tenant-based OAuth)
    fn load_fitbit_oauth_config() -> OAuthProviderConfig {
        // Use environment variables for global provider registration
        OAuthProviderConfig {
            client_id: env::var("FITBIT_CLIENT_ID").ok(),
            client_secret: env::var("FITBIT_CLIENT_SECRET").ok(),
            redirect_uri: Some(crate::constants::env_config::fitbit_redirect_uri()),
            scopes: parse_scopes(oauth::FITBIT_DEFAULT_SCOPES),
            enabled: env::var("FITBIT_CLIENT_ID").is_ok()
                && env::var("FITBIT_CLIENT_SECRET").is_ok(),
        }
    }

    /// Load security configuration from environment
    ///
    /// # Errors
    ///
    /// Returns an error if security environment variables are invalid
    fn load_security_config() -> Result<SecurityConfig> {
        Ok(SecurityConfig {
            cors_origins: parse_origins(&env_var_or("CORS_ORIGINS", "*")),
            rate_limit: Self::load_rate_limit_config()?,
            tls: Self::load_tls_config()?,
            headers: Self::load_security_headers_config(),
        })
    }

    /// Load rate limiting configuration from environment
    ///
    /// # Errors
    ///
    /// Returns an error if rate limit environment variables are invalid
    fn load_rate_limit_config() -> Result<RateLimitConfig> {
        Ok(RateLimitConfig {
            enabled: env_var_or("RATE_LIMIT_ENABLED", "true")
                .parse()
                .context("Invalid RATE_LIMIT_ENABLED value")?,
            requests_per_window: env_var_or(
                "RATE_LIMIT_REQUESTS",
                &limits::DEFAULT_RATE_LIMIT_REQUESTS.to_string(),
            )
            .parse()
            .context("Invalid RATE_LIMIT_REQUESTS value")?,
            window_seconds: env_var_or(
                "RATE_LIMIT_WINDOW",
                &limits::DEFAULT_RATE_LIMIT_WINDOW_SECS.to_string(),
            )
            .parse()
            .context("Invalid RATE_LIMIT_WINDOW value")?,
        })
    }

    /// Load TLS configuration from environment
    ///
    /// # Errors
    ///
    /// Returns an error if TLS environment variables are invalid
    fn load_tls_config() -> Result<TlsConfig> {
        Ok(TlsConfig {
            enabled: env_var_or("TLS_ENABLED", "false")
                .parse()
                .context("Invalid TLS_ENABLED value")?,
            cert_path: env::var("TLS_CERT_PATH").ok().map(PathBuf::from),
            key_path: env::var("TLS_KEY_PATH").ok().map(PathBuf::from),
        })
    }

    /// Load security headers configuration from environment
    fn load_security_headers_config() -> SecurityHeadersConfig {
        SecurityHeadersConfig {
            environment: Environment::from_str_or_default(&env_var_or(
                "SECURITY_HEADERS_ENV",
                "development",
            )),
        }
    }

    /// Load external services configuration from environment
    ///
    /// # Errors
    ///
    /// Returns an error if external services environment variables are invalid
    fn load_external_services_config() -> Result<ExternalServicesConfig> {
        Ok(ExternalServicesConfig {
            weather: Self::load_weather_service_config()?,
            geocoding: Self::load_geocoding_service_config()?,
            strava_api: Self::load_strava_api_config(),
            fitbit_api: Self::load_fitbit_api_config(),
        })
    }

    /// Load weather service configuration from environment
    ///
    /// # Errors
    ///
    /// Returns an error if weather service environment variables are invalid
    fn load_weather_service_config() -> Result<WeatherServiceConfig> {
        Ok(WeatherServiceConfig {
            api_key: env::var("OPENWEATHER_API_KEY").ok(),
            base_url: env_var_or(
                "OPENWEATHER_BASE_URL",
                "https://api.openweathermap.org/data/2.5",
            ),
            enabled: env_var_or("WEATHER_SERVICE_ENABLED", "true")
                .parse()
                .context("Invalid WEATHER_SERVICE_ENABLED value")?,
        })
    }

    /// Load geocoding service configuration from environment
    ///
    /// # Errors
    ///
    /// Returns an error if geocoding service environment variables are invalid
    fn load_geocoding_service_config() -> Result<GeocodingServiceConfig> {
        Ok(GeocodingServiceConfig {
            base_url: env_var_or("GEOCODING_BASE_URL", "https://nominatim.openstreetmap.org"),
            enabled: env_var_or("GEOCODING_SERVICE_ENABLED", "true")
                .parse()
                .context("Invalid GEOCODING_SERVICE_ENABLED value")?,
        })
    }

    /// Load Strava API configuration from environment
    fn load_strava_api_config() -> StravaApiConfig {
        StravaApiConfig {
            base_url: env_var_or("STRAVA_API_BASE", "https://www.strava.com/api/v3"),
            auth_url: env_var_or("STRAVA_AUTH_URL", "https://www.strava.com/oauth/authorize"),
            token_url: env_var_or("STRAVA_TOKEN_URL", "https://www.strava.com/oauth/token"),
        }
    }

    /// Load Fitbit API configuration from environment
    fn load_fitbit_api_config() -> FitbitApiConfig {
        FitbitApiConfig {
            base_url: env_var_or("FITBIT_API_BASE", "https://api.fitbit.com"),
            auth_url: env_var_or("FITBIT_AUTH_URL", "https://www.fitbit.com/oauth2/authorize"),
            token_url: env_var_or("FITBIT_TOKEN_URL", "https://api.fitbit.com/oauth2/token"),
        }
    }

    /// Load application behavior configuration from environment
    ///
    /// # Errors
    ///
    /// Returns an error if application behavior environment variables are invalid
    fn load_app_behavior_config() -> Result<AppBehaviorConfig> {
        Ok(AppBehaviorConfig {
            max_activities_fetch: env_var_or("MAX_ACTIVITIES_FETCH", "100")
                .parse()
                .context("Invalid MAX_ACTIVITIES_FETCH value")?,
            default_activities_limit: env_var_or("DEFAULT_ACTIVITIES_LIMIT", "20")
                .parse()
                .context("Invalid DEFAULT_ACTIVITIES_LIMIT value")?,
            ci_mode: env_var_or("CI", "false")
                .parse()
                .context("Invalid CI value")?,
            protocol: Self::load_protocol_config(),
        })
    }

    /// Load protocol configuration from environment
    fn load_protocol_config() -> ProtocolConfig {
        ProtocolConfig {
            mcp_version: env_var_or("MCP_PROTOCOL_VERSION", "2025-06-18"),
            server_name: env_var_or("SERVER_NAME", "pierre-mcp-server"),
            server_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Get environment variable or default value
fn env_var_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Parse comma-separated scopes
#[must_use]
fn parse_scopes(scopes_str: &str) -> Vec<String> {
    scopes_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Parse comma-separated CORS origins
#[must_use]
fn parse_origins(origins_str: &str) -> Vec<String> {
    if origins_str == "*" {
        vec!["*".into()]
    } else {
        origins_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}
