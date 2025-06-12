// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Environment-based configuration management for production deployment

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// MCP server port
    pub mcp_port: u16,
    /// HTTP API port  
    pub http_port: u16,
    /// Log level
    pub log_level: String,
    /// Database configuration
    pub database: DatabaseConfig,
    /// Authentication configuration
    pub auth: AuthConfig,
    /// OAuth provider configurations
    pub oauth: OAuthConfig,
    /// Security settings
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database URL (SQLite path or PostgreSQL connection string)
    pub url: String,
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

impl ServerConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        info!("Loading configuration from environment variables");
        
        // Load .env file if it exists
        if let Err(e) = dotenv::dotenv() {
            warn!("No .env file found or failed to load: {}", e);
        }

        let config = ServerConfig {
            mcp_port: env_var_or("MCP_PORT", "8080")?.parse()
                .context("Invalid MCP_PORT value")?,
            http_port: env_var_or("HTTP_PORT", "8081")?.parse()
                .context("Invalid HTTP_PORT value")?,
            log_level: env_var_or("RUST_LOG", "info")?,
            
            database: DatabaseConfig {
                url: env_var_or("DATABASE_URL", "sqlite:./data/users.db")?,
                encryption_key_path: PathBuf::from(env_var_or("ENCRYPTION_KEY_PATH", "./data/encryption.key")?),
                auto_migrate: env_var_or("AUTO_MIGRATE", "true")?.parse()
                    .context("Invalid AUTO_MIGRATE value")?,
                backup: BackupConfig {
                    enabled: env_var_or("BACKUP_ENABLED", "true")?.parse()
                        .context("Invalid BACKUP_ENABLED value")?,
                    interval_seconds: env_var_or("BACKUP_INTERVAL", "21600")?.parse()
                        .context("Invalid BACKUP_INTERVAL value")?,
                    retention_count: env_var_or("BACKUP_RETENTION", "7")?.parse()
                        .context("Invalid BACKUP_RETENTION value")?,
                    directory: PathBuf::from(env_var_or("BACKUP_DIRECTORY", "./backups")?),
                },
            },
            
            auth: AuthConfig {
                jwt_secret_path: PathBuf::from(env_var_or("JWT_SECRET_PATH", "./data/jwt.secret")?),
                jwt_expiry_hours: env_var_or("JWT_EXPIRY_HOURS", "24")?.parse()
                    .context("Invalid JWT_EXPIRY_HOURS value")?,
                enable_refresh_tokens: env_var_or("ENABLE_REFRESH_TOKENS", "false")?.parse()
                    .context("Invalid ENABLE_REFRESH_TOKENS value")?,
            },
            
            oauth: OAuthConfig {
                strava: OAuthProviderConfig {
                    client_id: env::var("STRAVA_CLIENT_ID").ok(),
                    client_secret: env::var("STRAVA_CLIENT_SECRET").ok(),
                    redirect_uri: env::var("STRAVA_REDIRECT_URI").ok(),
                    scopes: parse_scopes(&env_var_or("STRAVA_SCOPES", "read,activity:read_all")?),
                    enabled: env_var_or("STRAVA_ENABLED", "true")?.parse()
                        .context("Invalid STRAVA_ENABLED value")?,
                },
                fitbit: OAuthProviderConfig {
                    client_id: env::var("FITBIT_CLIENT_ID").ok(),
                    client_secret: env::var("FITBIT_CLIENT_SECRET").ok(),
                    redirect_uri: env::var("FITBIT_REDIRECT_URI").ok(),
                    scopes: parse_scopes(&env_var_or("FITBIT_SCOPES", "activity,profile")?),
                    enabled: env_var_or("FITBIT_ENABLED", "true")?.parse()
                        .context("Invalid FITBIT_ENABLED value")?,
                },
            },
            
            security: SecurityConfig {
                cors_origins: parse_origins(&env_var_or("CORS_ORIGINS", "*")?),
                rate_limit: RateLimitConfig {
                    enabled: env_var_or("RATE_LIMIT_ENABLED", "true")?.parse()
                        .context("Invalid RATE_LIMIT_ENABLED value")?,
                    requests_per_window: env_var_or("RATE_LIMIT_REQUESTS", "100")?.parse()
                        .context("Invalid RATE_LIMIT_REQUESTS value")?,
                    window_seconds: env_var_or("RATE_LIMIT_WINDOW", "60")?.parse()
                        .context("Invalid RATE_LIMIT_WINDOW value")?,
                },
                tls: TlsConfig {
                    enabled: env_var_or("TLS_ENABLED", "false")?.parse()
                        .context("Invalid TLS_ENABLED value")?,
                    cert_path: env::var("TLS_CERT_PATH").ok().map(PathBuf::from),
                    key_path: env::var("TLS_KEY_PATH").ok().map(PathBuf::from),
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

        // Database validation
        if self.database.url.is_empty() {
            return Err(anyhow::anyhow!("DATABASE_URL cannot be empty"));
        }

        // OAuth validation
        if self.oauth.strava.enabled {
            if self.oauth.strava.client_id.is_none() || self.oauth.strava.client_secret.is_none() {
                warn!("Strava OAuth is enabled but missing client_id or client_secret");
            }
        }

        if self.oauth.fitbit.enabled {
            if self.oauth.fitbit.client_id.is_none() || self.oauth.fitbit.client_secret.is_none() {
                warn!("Fitbit OAuth is enabled but missing client_id or client_secret");
            }
        }

        // TLS validation
        if self.security.tls.enabled {
            if self.security.tls.cert_path.is_none() || self.security.tls.key_path.is_none() {
                return Err(anyhow::anyhow!("TLS is enabled but cert_path or key_path is missing"));
            }
        }

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
             - TLS: {}\n\
             - Rate Limiting: {}",
            self.mcp_port,
            self.http_port,
            self.log_level,
            if self.database.url.starts_with("sqlite:") { "SQLite" } else { "External DB" },
            if self.oauth.strava.enabled && self.oauth.strava.client_id.is_some() { "Enabled" } else { "Disabled" },
            if self.oauth.fitbit.enabled && self.oauth.fitbit.client_id.is_some() { "Enabled" } else { "Disabled" },
            if self.security.tls.enabled { "Enabled" } else { "Disabled" },
            if self.security.rate_limit.enabled { "Enabled" } else { "Disabled" }
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
    fn test_config_validation() {
        // Test port conflict
        let mut config = ServerConfig {
            mcp_port: 8080,
            http_port: 8080,  // Same as MCP port
            log_level: "info".to_string(),
            database: DatabaseConfig {
                url: "sqlite:test.db".to_string(),
                encryption_key_path: PathBuf::from("test.key"),
                auto_migrate: true,
                backup: BackupConfig {
                    enabled: false,
                    interval_seconds: 3600,
                    retention_count: 7,
                    directory: PathBuf::from("./backups"),
                },
            },
            auth: AuthConfig {
                jwt_secret_path: PathBuf::from("test.secret"),
                jwt_expiry_hours: 24,
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
                    requests_per_window: 100,
                    window_seconds: 60,
                },
                tls: TlsConfig {
                    enabled: false,
                    cert_path: None,
                    key_path: None,
                },
            },
        };

        assert!(config.validate().is_err());

        // Fix port conflict
        config.http_port = 8081;
        assert!(config.validate().is_ok());
    }
}