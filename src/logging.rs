// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Production-ready logging configuration with structured output

use anyhow::Result;
use serde_json::json;
use std::env;
use std::io;
use tracing::{info, warn};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Output format (json, pretty, compact)
    pub format: LogFormat,
    /// Include source file and line numbers
    pub include_location: bool,
    /// Include thread information
    pub include_thread: bool,
    /// Include span information for tracing
    pub include_spans: bool,
    /// Service name for structured logging
    pub service_name: String,
    /// Service version
    pub service_version: String,
    /// Environment (development, staging, production)
    pub environment: String,
}

#[derive(Debug, Clone)]
pub enum LogFormat {
    /// JSON format for production logging
    Json,
    /// Pretty format for development
    Pretty,
    /// Compact format for space-constrained environments
    Compact,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Pretty,
            include_location: false,
            include_thread: false,
            include_spans: false,
            service_name: "pierre-mcp-server".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            environment: "development".to_string(),
        }
    }
}

impl LoggingConfig {
    /// Create logging configuration from environment variables
    pub fn from_env() -> Self {
        let level = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
        
        let format = match env::var("LOG_FORMAT").as_deref() {
            Ok("json") => LogFormat::Json,
            Ok("compact") => LogFormat::Compact,
            _ => LogFormat::Pretty,
        };

        let environment = env::var("ENVIRONMENT")
            .or_else(|_| env::var("NODE_ENV"))
            .unwrap_or_else(|_| "development".to_string());

        // In production, use more detailed logging
        let is_production = environment == "production";

        Self {
            level,
            format,
            include_location: is_production || env::var("LOG_INCLUDE_LOCATION").is_ok(),
            include_thread: is_production || env::var("LOG_INCLUDE_THREAD").is_ok(),
            include_spans: is_production || env::var("LOG_INCLUDE_SPANS").is_ok(),
            service_name: env::var("SERVICE_NAME")
                .unwrap_or_else(|_| "pierre-mcp-server".to_string()),
            service_version: env::var("SERVICE_VERSION")
                .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string()),
            environment,
        }
    }

    /// Initialize the global tracing subscriber
    pub fn init(&self) -> Result<()> {
        // Create environment filter
        let env_filter = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(&self.level))
            .unwrap_or_else(|_| EnvFilter::new("info"));

        // Create base registry
        let registry = tracing_subscriber::registry().with(env_filter);

        match self.format {
            LogFormat::Json => {
                let json_layer = fmt::layer()
                    .with_file(self.include_location)
                    .with_line_number(self.include_location)
                    .with_thread_ids(self.include_thread)
                    .with_thread_names(self.include_thread)
                    .with_target(true)
                    .with_writer(io::stdout)
                    .with_span_events(if self.include_spans {
                        FmtSpan::NEW | FmtSpan::CLOSE
                    } else {
                        FmtSpan::NONE
                    })
                    .json();

                registry.with(json_layer).init();
            }
            LogFormat::Pretty => {
                let pretty_layer = fmt::layer()
                    .with_file(self.include_location)
                    .with_line_number(self.include_location)
                    .with_thread_ids(self.include_thread)
                    .with_thread_names(self.include_thread)
                    .with_target(true)
                    .with_writer(io::stdout)
                    .with_span_events(if self.include_spans {
                        FmtSpan::NEW | FmtSpan::CLOSE
                    } else {
                        FmtSpan::NONE
                    });

                registry.with(pretty_layer).init();
            }
            LogFormat::Compact => {
                let compact_layer = fmt::layer()
                    .compact()
                    .with_file(false)
                    .with_line_number(false)
                    .with_thread_ids(false)
                    .with_thread_names(false)
                    .with_target(false)
                    .with_writer(io::stdout)
                    .with_span_events(FmtSpan::NONE);

                registry.with(compact_layer).init();
            }
        }

        // Log startup information
        self.log_startup_info();

        Ok(())
    }

    /// Log structured startup information
    fn log_startup_info(&self) {
        info!(
            service.name = %self.service_name,
            service.version = %self.service_version,
            environment = %self.environment,
            log.level = %self.level,
            log.format = ?self.format,
            "Pierre MCP Server starting up"
        );

        // Log configuration summary
        let config_summary = json!({
            "service": {
                "name": self.service_name,
                "version": self.service_version,
                "environment": self.environment
            },
            "logging": {
                "level": self.level,
                "format": format!("{:?}", self.format),
                "features": {
                    "location": self.include_location,
                    "thread": self.include_thread,
                    "spans": self.include_spans
                }
            }
        });

        info!("Configuration loaded: {}", config_summary);
    }
}

/// Initialize logging with default configuration
pub fn init_default() -> Result<()> {
    LoggingConfig::default().init()
}

/// Initialize logging from environment
pub fn init_from_env() -> Result<()> {
    LoggingConfig::from_env().init()
}

/// Application-specific logging utilities
pub struct AppLogger;

impl AppLogger {
    /// Log user authentication events
    pub fn log_auth_event(user_id: &str, event: &str, success: bool, details: Option<&str>) {
        info!(
            user.id = %user_id,
            auth.event = %event,
            auth.success = %success,
            auth.details = details.unwrap_or(""),
            "Authentication event"
        );
    }

    /// Log OAuth events
    pub fn log_oauth_event(user_id: &str, provider: &str, event: &str, success: bool) {
        info!(
            user.id = %user_id,
            oauth.provider = %provider,
            oauth.event = %event,
            oauth.success = %success,
            "OAuth event"
        );
    }

    /// Log API requests
    pub fn log_api_request(method: &str, path: &str, status: u16, duration_ms: u64, user_id: Option<&str>) {
        info!(
            http.method = %method,
            http.path = %path,
            http.status = %status,
            http.duration_ms = %duration_ms,
            user.id = user_id.unwrap_or("anonymous"),
            "HTTP request"
        );
    }

    /// Log MCP tool calls
    pub fn log_mcp_tool_call(user_id: &str, tool_name: &str, success: bool, duration_ms: u64) {
        info!(
            user.id = %user_id,
            mcp.tool = %tool_name,
            mcp.success = %success,
            mcp.duration_ms = %duration_ms,
            "MCP tool call"
        );
    }

    /// Log database operations
    pub fn log_database_operation(operation: &str, table: &str, success: bool, duration_ms: u64) {
        info!(
            db.operation = %operation,
            db.table = %table,
            db.success = %success,
            db.duration_ms = %duration_ms,
            "Database operation"
        );
    }

    /// Log security events
    pub fn log_security_event(event_type: &str, severity: &str, details: &str, user_id: Option<&str>) {
        warn!(
            security.event = %event_type,
            security.severity = %severity,
            security.details = %details,
            user.id = user_id.unwrap_or("unknown"),
            "Security event"
        );
    }

    /// Log performance metrics
    pub fn log_performance_metric(metric_name: &str, value: f64, unit: &str, tags: Option<&serde_json::Value>) {
        info!(
            metric.name = %metric_name,
            metric.value = %value,
            metric.unit = %unit,
            metric.tags = %tags.unwrap_or(&json!({})),
            "Performance metric"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_logging_config_from_env() {
        // Set test environment variables
        env::set_var("RUST_LOG", "debug");
        env::set_var("LOG_FORMAT", "json");
        env::set_var("ENVIRONMENT", "production");
        env::set_var("SERVICE_NAME", "test-service");

        let config = LoggingConfig::from_env();

        assert_eq!(config.level, "debug");
        assert!(matches!(config.format, LogFormat::Json));
        assert_eq!(config.environment, "production");
        assert_eq!(config.service_name, "test-service");
        assert!(config.include_location); // Should be true for production

        // Clean up
        env::remove_var("RUST_LOG");
        env::remove_var("LOG_FORMAT");
        env::remove_var("ENVIRONMENT");
        env::remove_var("SERVICE_NAME");
    }

    #[test]
    fn test_default_logging_config() {
        let config = LoggingConfig::default();

        assert_eq!(config.level, "info");
        assert!(matches!(config.format, LogFormat::Pretty));
        assert_eq!(config.environment, "development");
        assert_eq!(config.service_name, "pierre-mcp-server");
        assert!(!config.include_location); // Should be false for development
    }
}