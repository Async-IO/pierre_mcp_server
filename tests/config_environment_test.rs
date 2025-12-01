// ABOUTME: Unit tests for config environment functionality
// ABOUTME: Validates config environment behavior, edge cases, and error handling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::config::environment::{
    BackupConfig, DatabaseConfig, DatabaseUrl, Environment, LogLevel, OAuthConfig,
    OAuthProviderConfig, SecurityConfig, ServerConfig,
};

// Tests for public configuration types

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

/// Helper function to create a valid test `ServerConfig`
fn create_test_server_config() -> ServerConfig {
    ServerConfig {
        http_port: 3000,
        database: DatabaseConfig {
            url: DatabaseUrl::SQLite {
                path: "./test.db".into(),
            },
            backup: BackupConfig {
                directory: "./backups".into(),
                ..Default::default()
            },
            ..Default::default()
        },
        oauth: OAuthConfig {
            strava: OAuthProviderConfig {
                client_id: Some("test_id".into()),
                client_secret: Some("test_secret".into()),
                redirect_uri: Some("http://localhost/callback".into()),
                scopes: vec!["read".into()],
                enabled: true,
            },
            ..Default::default()
        },
        security: SecurityConfig::default(),
        ..Default::default()
    }
}

#[test]
fn test_config_validation() {
    // Test valid configuration with single-port architecture
    let config = create_test_server_config();

    // With single-port architecture, validation should pass
    assert!(config.validate().is_ok());
}
