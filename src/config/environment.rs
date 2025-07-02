// ABOUTME: Environment configuration management for deployment-specific settings
// ABOUTME: Handles environment variables, deployment modes, and runtime configuration parsing
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

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
    /// Convert to tracing::Level
    pub fn to_tracing_level(&self) -> tracing::Level {
        match self {
            LogLevel::Error => tracing::Level::ERROR,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Trace => tracing::Level::TRACE,
        }
    }

    /// Parse from string with fallback
    pub fn from_str_or_default(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "error" => LogLevel::Error,
            "warn" => LogLevel::Warn,
            "info" => LogLevel::Info,
            "debug" => LogLevel::Debug,
            "trace" => LogLevel::Trace,
            _ => LogLevel::Info, // Default fallback
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Error => write!(f, "error"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Trace => write!(f, "trace"),
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
    pub fn from_str_or_default(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "production" | "prod" => Environment::Production,
            "testing" | "test" => Environment::Testing,
            "development" | "dev" => Environment::Development,
            _ => Environment::Development, // Default fallback for unrecognized values
        }
    }

    /// Check if this is a production environment
    pub fn is_production(&self) -> bool {
        matches!(self, Environment::Production)
    }

    /// Check if this is a development environment
    pub fn is_development(&self) -> bool {
        matches!(self, Environment::Development)
    }

    /// Check if this is a testing environment
    pub fn is_testing(&self) -> bool {
        matches!(self, Environment::Testing)
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Environment::Development => write!(f, "development"),
            Environment::Production => write!(f, "production"),
            Environment::Testing => write!(f, "testing"),
        }
    }
}

/// Type-safe database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseUrl {
    /// SQLite database with file path
    SQLite { path: PathBuf },
    /// PostgreSQL connection
    PostgreSQL { connection_string: String },
    /// In-memory SQLite (for testing)
    Memory,
}

impl DatabaseUrl {
    /// Parse from string with validation
    pub fn parse_url(s: &str) -> Result<Self> {
        if s.starts_with("sqlite:") {
            let path_str = s.strip_prefix("sqlite:").unwrap_or(s);
            if path_str == ":memory:" {
                Ok(DatabaseUrl::Memory)
            } else {
                Ok(DatabaseUrl::SQLite {
                    path: PathBuf::from(path_str),
                })
            }
        } else if s.starts_with("postgresql://") || s.starts_with("postgres://") {
            Ok(DatabaseUrl::PostgreSQL {
                connection_string: s.to_string(),
            })
        } else {
            // Fallback: treat as SQLite file path
            Ok(DatabaseUrl::SQLite {
                path: PathBuf::from(s),
            })
        }
    }

    /// Convert to connection string
    pub fn to_connection_string(&self) -> String {
        match self {
            DatabaseUrl::SQLite { path } => format!("sqlite:{}", path.display()),
            DatabaseUrl::PostgreSQL { connection_string } => connection_string.clone(),
            DatabaseUrl::Memory => "sqlite::memory:".to_string(),
        }
    }

    /// Check if this is an in-memory database
    pub fn is_memory(&self) -> bool {
        matches!(self, DatabaseUrl::Memory)
    }

    /// Check if this is a SQLite database
    pub fn is_sqlite(&self) -> bool {
        matches!(self, DatabaseUrl::SQLite { .. } | DatabaseUrl::Memory)
    }

    /// Check if this is a PostgreSQL database
    pub fn is_postgresql(&self) -> bool {
        matches!(self, DatabaseUrl::PostgreSQL { .. })
    }
}

impl Default for DatabaseUrl {
    fn default() -> Self {
        DatabaseUrl::SQLite {
            path: PathBuf::from("./data/users.db"),
        }
    }
}

impl std::fmt::Display for DatabaseUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_connection_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// MCP server port
    pub mcp_port: u16,
    /// HTTP API port  
    pub http_port: u16,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database URL (SQLite path or PostgreSQL connection string)
    pub url: DatabaseUrl,
    /// Path to encryption key file
    pub encryption_key_path: PathBuf,
    /// Enable database migrations on startup
    pub auto_migrate: bool,
    /// Database backup configuration
    pub backup: BackupConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// JWT secret key path
    pub jwt_secret_path: PathBuf,
    /// JWT expiry time in hours
    pub jwt_expiry_hours: u64,
    /// Enable JWT refresh tokens
    pub enable_refresh_tokens: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// Strava OAuth configuration
    pub strava: OAuthProviderConfig,
    /// Fitbit OAuth configuration  
    pub fitbit: OAuthProviderConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityHeadersConfig {
    /// Environment type for security headers (development, production)
    pub environment: Environment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    pub enabled: bool,
    /// Requests per window
    pub requests_per_window: u32,
    /// Window duration in seconds
    pub window_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Enable TLS
    pub enabled: bool,
    /// Path to TLS certificate
    pub cert_path: Option<PathBuf>,
    /// Path to TLS private key
    pub key_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherServiceConfig {
    /// OpenWeather API key
    pub api_key: Option<String>,
    /// Weather service base URL
    pub base_url: String,
    /// Enable weather service
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeocodingServiceConfig {
    /// Geocoding service base URL
    pub base_url: String,
    /// Enable geocoding service
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StravaApiConfig {
    /// Strava API base URL
    pub base_url: String,
    /// Strava auth URL
    pub auth_url: String,
    /// Strava token URL
    pub token_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitbitApiConfig {
    /// Fitbit API base URL
    pub base_url: String,
    /// Fitbit auth URL
    pub auth_url: String,
    /// Fitbit token URL
    pub token_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub fn from_env() -> Result<Self> {
        info!("Loading configuration from environment variables");

        // Load .env file if it exists
        if let Err(e) = dotenvy::dotenv() {
            warn!("No .env file found or failed to load: {}", e);
        }

        let config = ServerConfig {
            mcp_port: env_config::mcp_port(),
            http_port: env_config::http_port(),
            log_level: LogLevel::from_str_or_default(&env_config::log_level()),

            database: DatabaseConfig {
                url: DatabaseUrl::parse_url(&env_config::database_url())
                    .unwrap_or_else(|_| DatabaseUrl::default()),
                encryption_key_path: PathBuf::from(env_config::encryption_key_path()),
                auto_migrate: env_var_or("AUTO_MIGRATE", "true")?
                    .parse()
                    .context("Invalid AUTO_MIGRATE value")?,
                backup: BackupConfig {
                    enabled: env_var_or("BACKUP_ENABLED", "true")?
                        .parse()
                        .context("Invalid BACKUP_ENABLED value")?,
                    interval_seconds: env_var_or(
                        "BACKUP_INTERVAL",
                        &limits::DEFAULT_BACKUP_INTERVAL_SECS.to_string(),
                    )?
                    .parse()
                    .context("Invalid BACKUP_INTERVAL value")?,
                    retention_count: env_var_or(
                        "BACKUP_RETENTION",
                        &limits::DEFAULT_BACKUP_RETENTION_COUNT.to_string(),
                    )?
                    .parse()
                    .context("Invalid BACKUP_RETENTION value")?,
                    directory: PathBuf::from(env_var_or(
                        "BACKUP_DIRECTORY",
                        defaults::DEFAULT_BACKUP_DIR,
                    )?),
                },
            },

            auth: AuthConfig {
                jwt_secret_path: PathBuf::from(env_config::jwt_secret_path()),
                jwt_expiry_hours: env_config::jwt_expiry_hours() as u64,
                enable_refresh_tokens: env_var_or("ENABLE_REFRESH_TOKENS", "false")?
                    .parse()
                    .context("Invalid ENABLE_REFRESH_TOKENS value")?,
            },

            oauth: OAuthConfig {
                strava: OAuthProviderConfig {
                    client_id: env_config::strava_client_id(),
                    client_secret: env_config::strava_client_secret(),
                    redirect_uri: Some(env_config::strava_redirect_uri()),
                    scopes: parse_scopes(oauth::STRAVA_DEFAULT_SCOPES),
                    enabled: env_var_or("STRAVA_ENABLED", "true")?
                        .parse()
                        .context("Invalid STRAVA_ENABLED value")?,
                },
                fitbit: OAuthProviderConfig {
                    client_id: env::var("FITBIT_CLIENT_ID").ok(),
                    client_secret: env::var("FITBIT_CLIENT_SECRET").ok(),
                    redirect_uri: env::var("FITBIT_REDIRECT_URI").ok(),
                    scopes: parse_scopes(oauth::FITBIT_DEFAULT_SCOPES),
                    enabled: env_var_or("FITBIT_ENABLED", "true")?
                        .parse()
                        .context("Invalid FITBIT_ENABLED value")?,
                },
            },

            security: SecurityConfig {
                cors_origins: parse_origins(&env_var_or("CORS_ORIGINS", "*")?),
                rate_limit: RateLimitConfig {
                    enabled: env_var_or("RATE_LIMIT_ENABLED", "true")?
                        .parse()
                        .context("Invalid RATE_LIMIT_ENABLED value")?,
                    requests_per_window: env_var_or(
                        "RATE_LIMIT_REQUESTS",
                        &limits::DEFAULT_RATE_LIMIT_REQUESTS.to_string(),
                    )?
                    .parse()
                    .context("Invalid RATE_LIMIT_REQUESTS value")?,
                    window_seconds: env_var_or(
                        "RATE_LIMIT_WINDOW",
                        &limits::DEFAULT_RATE_LIMIT_WINDOW_SECS.to_string(),
                    )?
                    .parse()
                    .context("Invalid RATE_LIMIT_WINDOW value")?,
                },
                tls: TlsConfig {
                    enabled: env_var_or("TLS_ENABLED", "false")?
                        .parse()
                        .context("Invalid TLS_ENABLED value")?,
                    cert_path: env::var("TLS_CERT_PATH").ok().map(PathBuf::from),
                    key_path: env::var("TLS_KEY_PATH").ok().map(PathBuf::from),
                },
                headers: SecurityHeadersConfig {
                    environment: Environment::from_str_or_default(&env_var_or(
                        "SECURITY_HEADERS_ENV",
                        "development",
                    )?),
                },
            },

            external_services: ExternalServicesConfig {
                weather: WeatherServiceConfig {
                    api_key: env::var("OPENWEATHER_API_KEY").ok(),
                    base_url: env_var_or(
                        "OPENWEATHER_BASE_URL",
                        "https://api.openweathermap.org/data/2.5",
                    )?,
                    enabled: env_var_or("WEATHER_SERVICE_ENABLED", "true")?
                        .parse()
                        .context("Invalid WEATHER_SERVICE_ENABLED value")?,
                },
                geocoding: GeocodingServiceConfig {
                    base_url: env_var_or(
                        "GEOCODING_BASE_URL",
                        "https://nominatim.openstreetmap.org",
                    )?,
                    enabled: env_var_or("GEOCODING_SERVICE_ENABLED", "true")?
                        .parse()
                        .context("Invalid GEOCODING_SERVICE_ENABLED value")?,
                },
                strava_api: StravaApiConfig {
                    base_url: env_var_or("STRAVA_API_BASE", "https://www.strava.com/api/v3")?,
                    auth_url: env_var_or(
                        "STRAVA_AUTH_URL",
                        "https://www.strava.com/oauth/authorize",
                    )?,
                    token_url: env_var_or(
                        "STRAVA_TOKEN_URL",
                        "https://www.strava.com/oauth/token",
                    )?,
                },
                fitbit_api: FitbitApiConfig {
                    base_url: env_var_or("FITBIT_API_BASE", "https://api.fitbit.com")?,
                    auth_url: env_var_or(
                        "FITBIT_AUTH_URL",
                        "https://www.fitbit.com/oauth2/authorize",
                    )?,
                    token_url: env_var_or(
                        "FITBIT_TOKEN_URL",
                        "https://api.fitbit.com/oauth2/token",
                    )?,
                },
            },

            app_behavior: AppBehaviorConfig {
                max_activities_fetch: env_var_or("MAX_ACTIVITIES_FETCH", "100")?
                    .parse()
                    .context("Invalid MAX_ACTIVITIES_FETCH value")?,
                default_activities_limit: env_var_or("DEFAULT_ACTIVITIES_LIMIT", "20")?
                    .parse()
                    .context("Invalid DEFAULT_ACTIVITIES_LIMIT value")?,
                ci_mode: env_var_or("CI", "false")?
                    .parse()
                    .context("Invalid CI value")?,
                protocol: ProtocolConfig {
                    mcp_version: env_var_or("MCP_PROTOCOL_VERSION", "2024-11-05")?,
                    server_name: env_var_or("SERVER_NAME", "pierre-mcp-server")?,
                    server_version: env!("CARGO_PKG_VERSION").to_string(),
                },
            },
        };

        config.validate()?;
        info!("Configuration loaded successfully");
        Ok(config)
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        // Port validation
        if self.mcp_port == self.http_port {
            return Err(anyhow::anyhow!("MCP_PORT and HTTP_PORT cannot be the same"));
        }

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
    pub fn init_all_configs(&self) -> Result<()> {
        // Initialize intelligence configuration
        let _intelligence_config = crate::config::intelligence_config::IntelligenceConfig::global();

        info!("All configurations initialized successfully");
        Ok(())
    }

    /// Get a summary of the configuration for logging (without secrets)
    pub fn summary(&self) -> String {
        format!(
            "Pierre MCP Server Configuration:\n\
             - MCP Port: {}\n\
             - HTTP Port: {}\n\
             - Log Level: {}\n\
             - Database: {}\n\
             - Strava OAuth: {}\n\
             - Fitbit OAuth: {}\n\
             - Weather Service: {}\n\
             - TLS: {}\n\
             - Rate Limiting: {}\n\
             - CI Mode: {}\n\
             - Protocol Version: {}",
            self.mcp_port,
            self.http_port,
            self.log_level,
            if self.database.url.is_sqlite() {
                "SQLite"
            } else {
                "PostgreSQL"
            },
            if self.oauth.strava.enabled && self.oauth.strava.client_id.is_some() {
                "Enabled"
            } else {
                "Disabled"
            },
            if self.oauth.fitbit.enabled && self.oauth.fitbit.client_id.is_some() {
                "Enabled"
            } else {
                "Disabled"
            },
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
    /// Get the OpenWeather API key if available
    pub fn openweather_api_key(&self) -> Option<&str> {
        self.external_services.weather.api_key.as_deref()
    }

    /// Get Strava API configuration
    pub fn strava_api_config(&self) -> &StravaApiConfig {
        &self.external_services.strava_api
    }

    /// Get Fitbit API configuration
    pub fn fitbit_api_config(&self) -> &FitbitApiConfig {
        &self.external_services.fitbit_api
    }

    /// Check if CI mode is enabled
    pub fn is_ci_mode(&self) -> bool {
        self.app_behavior.ci_mode
    }

    /// Get protocol information
    pub fn protocol_info(&self) -> (&str, &str, &str) {
        (
            &self.app_behavior.protocol.mcp_version,
            &self.app_behavior.protocol.server_name,
            &self.app_behavior.protocol.server_version,
        )
    }

    /// Get activity fetch limits
    pub fn activity_limits(&self) -> (usize, usize) {
        (
            self.app_behavior.max_activities_fetch,
            self.app_behavior.default_activities_limit,
        )
    }
}

/// Get environment variable or default value
fn env_var_or(key: &str, default: &str) -> Result<String> {
    Ok(env::var(key).unwrap_or_else(|_| default.to_string()))
}

/// Parse comma-separated scopes
fn parse_scopes(scopes_str: &str) -> Vec<String> {
    scopes_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Parse comma-separated CORS origins
fn parse_origins(origins_str: &str) -> Vec<String> {
    if origins_str == "*" {
        vec!["*".to_string()]
    } else {
        origins_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_scopes() {
        assert_eq!(
            parse_scopes("read,write,admin"),
            vec!["read", "write", "admin"]
        );
        assert_eq!(
            parse_scopes("read, write , admin "),
            vec!["read", "write", "admin"]
        );
        assert_eq!(parse_scopes(""), Vec::<String>::new());
    }

    #[test]
    fn test_parse_origins() {
        assert_eq!(parse_origins("*"), vec!["*"]);
        assert_eq!(
            parse_origins("http://localhost:3000,https://app.example.com"),
            vec!["http://localhost:3000", "https://app.example.com"]
        );
    }

    #[test]
    fn test_log_level_parsing() {
        assert_eq!(LogLevel::from_str_or_default("error"), LogLevel::Error);
        assert_eq!(LogLevel::from_str_or_default("WARN"), LogLevel::Warn);
        assert_eq!(LogLevel::from_str_or_default("info"), LogLevel::Info);
        assert_eq!(LogLevel::from_str_or_default("Debug"), LogLevel::Debug);
        assert_eq!(LogLevel::from_str_or_default("trace"), LogLevel::Trace);
        assert_eq!(LogLevel::from_str_or_default("invalid"), LogLevel::Info); // Default fallback
    }

    #[test]
    fn test_environment_parsing() {
        assert_eq!(
            Environment::from_str_or_default("production"),
            Environment::Production
        );
        assert_eq!(
            Environment::from_str_or_default("PROD"),
            Environment::Production
        );
        assert_eq!(
            Environment::from_str_or_default("development"),
            Environment::Development
        );
        assert_eq!(
            Environment::from_str_or_default("dev"),
            Environment::Development
        );
        assert_eq!(
            Environment::from_str_or_default("testing"),
            Environment::Testing
        );
        assert_eq!(
            Environment::from_str_or_default("test"),
            Environment::Testing
        );
        assert_eq!(
            Environment::from_str_or_default("invalid"),
            Environment::Development
        ); // Default fallback
    }

    #[test]
    fn test_database_url_parsing() {
        // SQLite URLs
        let sqlite_url = DatabaseUrl::parse_url("sqlite:./test.db").unwrap();
        assert!(sqlite_url.is_sqlite());
        assert!(!sqlite_url.is_postgresql());
        assert_eq!(sqlite_url.to_connection_string(), "sqlite:./test.db");

        // Memory database
        let memory_url = DatabaseUrl::parse_url("sqlite::memory:").unwrap();
        assert!(memory_url.is_memory());
        assert!(memory_url.is_sqlite());

        // PostgreSQL URLs
        let pg_url = DatabaseUrl::parse_url("postgresql://user:pass@localhost/db").unwrap();
        assert!(pg_url.is_postgresql());
        assert!(!pg_url.is_sqlite());

        // Fallback to SQLite
        let fallback_url = DatabaseUrl::parse_url("./some/path.db").unwrap();
        assert!(fallback_url.is_sqlite());
    }

    #[test]
    fn test_config_validation() {
        // Test port conflict
        let mut config = ServerConfig {
            mcp_port: env_config::mcp_port(),
            http_port: env_config::mcp_port(), // Same as MCP port - should fail validation
            log_level: LogLevel::default(),
            database: DatabaseConfig {
                url: DatabaseUrl::SQLite {
                    path: PathBuf::from("test.db"),
                },
                encryption_key_path: PathBuf::from("test.key"),
                auto_migrate: true,
                backup: BackupConfig {
                    enabled: false,
                    interval_seconds: limits::DEFAULT_BACKUP_INTERVAL_SECS,
                    retention_count: limits::DEFAULT_BACKUP_RETENTION_COUNT as u32,
                    directory: PathBuf::from(defaults::DEFAULT_BACKUP_DIR),
                },
            },
            auth: AuthConfig {
                jwt_secret_path: PathBuf::from("test.secret"),
                jwt_expiry_hours: limits::JWT_EXPIRY_HOURS as u64,
                enable_refresh_tokens: false,
            },
            oauth: OAuthConfig {
                strava: OAuthProviderConfig {
                    client_id: None,
                    client_secret: None,
                    redirect_uri: None,
                    scopes: vec![],
                    enabled: false,
                },
                fitbit: OAuthProviderConfig {
                    client_id: None,
                    client_secret: None,
                    redirect_uri: None,
                    scopes: vec![],
                    enabled: false,
                },
            },
            security: SecurityConfig {
                cors_origins: vec!["*".to_string()],
                rate_limit: RateLimitConfig {
                    enabled: false,
                    requests_per_window: limits::DEFAULT_RATE_LIMIT_REQUESTS,
                    window_seconds: limits::DEFAULT_RATE_LIMIT_WINDOW_SECS,
                },
                tls: TlsConfig {
                    enabled: false,
                    cert_path: None,
                    key_path: None,
                },
                headers: SecurityHeadersConfig {
                    environment: Environment::Development,
                },
            },
            external_services: ExternalServicesConfig {
                weather: WeatherServiceConfig {
                    api_key: None,
                    base_url: "https://api.openweathermap.org/data/2.5".to_string(),
                    enabled: false,
                },
                geocoding: GeocodingServiceConfig {
                    base_url: "https://nominatim.openstreetmap.org".to_string(),
                    enabled: true,
                },
                strava_api: StravaApiConfig {
                    base_url: "https://www.strava.com/api/v3".to_string(),
                    auth_url: "https://www.strava.com/oauth/authorize".to_string(),
                    token_url: "https://www.strava.com/oauth/token".to_string(),
                },
                fitbit_api: FitbitApiConfig {
                    base_url: "https://api.fitbit.com".to_string(),
                    auth_url: "https://www.fitbit.com/oauth2/authorize".to_string(),
                    token_url: "https://api.fitbit.com/oauth2/token".to_string(),
                },
            },
            app_behavior: AppBehaviorConfig {
                max_activities_fetch: 100,
                default_activities_limit: 20,
                ci_mode: false,
                protocol: ProtocolConfig {
                    mcp_version: "2024-11-05".to_string(),
                    server_name: "pierre-mcp-server".to_string(),
                    server_version: "test".to_string(),
                },
            },
        };

        assert!(config.validate().is_err());

        // Fix port conflict
        config.http_port = env_config::http_port();
        assert!(config.validate().is_ok());
    }
}
