// ABOUTME: Logging configuration and structured logging setup for observability and debugging
// ABOUTME: Configures log levels, formatters, and output destinations for comprehensive system logging
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Production-ready logging configuration with structured output

/// Tenant-aware logging utilities and context management
pub mod tenant;

/// Re-export tenant logging utilities
pub use tenant::{
    record_performance_metrics, record_request_context, record_tenant_context, ProviderApiContext,
    TenantLogger,
};

use crate::constants::service_names;
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

// OpenTelemetry support disabled temporarily due to version compatibility issues

/// Logging configuration
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)] // Configuration struct needs multiple boolean flags for comprehensive control
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
    /// Enable OpenTelemetry tracing
    pub enable_telemetry: bool,
    /// Request ID header name
    pub request_id_header: String,
    /// Enable GCP Cloud Logging format
    pub enable_gcp_format: bool,
    /// Truncate long MCP request/response logs for readability
    pub truncate_mcp_logs: bool,
}

/// Log output format options
#[derive(Debug, Clone)]
pub enum LogFormat {
    /// `JSON` format for production logging
    Json,
    /// Pretty format for development
    Pretty,
    /// Compact format for space-constrained environments
    Compact,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".into(),
            format: LogFormat::Pretty,
            include_location: false,
            include_thread: false,
            include_spans: false,
            service_name: service_names::PIERRE_MCP_SERVER.into(),
            service_version: env!("CARGO_PKG_VERSION").to_owned(),
            environment: "development".into(),
            enable_telemetry: false,
            request_id_header: "x-request-id".into(),
            enable_gcp_format: false,
            truncate_mcp_logs: true, // Default to readable logs
        }
    }
}

impl LoggingConfig {
    /// Create logging configuration from environment variables
    #[must_use]
    pub fn from_env() -> Self {
        let level = env::var("RUST_LOG").unwrap_or_else(|_| "info".into());

        let format = match env::var("LOG_FORMAT").as_deref() {
            Ok("json") => LogFormat::Json,
            Ok("compact") => LogFormat::Compact,
            _ => LogFormat::Pretty,
        };

        let environment = env::var("ENVIRONMENT")
            .or_else(|_| env::var("NODE_ENV"))
            .unwrap_or_else(|_| "development".into());

        // In production, use more detailed logging
        let is_production = environment == "production";

        Self {
            level,
            format,
            include_location: is_production || env::var("LOG_INCLUDE_LOCATION").is_ok(),
            include_thread: is_production || env::var("LOG_INCLUDE_THREAD").is_ok(),
            include_spans: is_production || env::var("LOG_INCLUDE_SPANS").is_ok(),
            service_name: env::var("SERVICE_NAME")
                .unwrap_or_else(|_| service_names::PIERRE_MCP_SERVER.into()),
            service_version: env::var("SERVICE_VERSION")
                .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_owned()),
            environment: environment.clone(), // Safe: String ownership for logging config
            enable_telemetry: is_production || env::var("ENABLE_TELEMETRY").is_ok(),
            request_id_header: env::var("REQUEST_ID_HEADER")
                .unwrap_or_else(|_| "x-request-id".into()),
            enable_gcp_format: environment == "production" && env::var("GCP_PROJECT_ID").is_ok(),
            truncate_mcp_logs: env::var("MCP_LOG_TRUNCATE")
                .map(|v| v != "false" && v != "0")
                .unwrap_or(true), // Default to true (truncated) unless explicitly disabled
        }
    }

    /// Initialize the global tracing subscriber
    ///
    /// # Errors
    ///
    /// Returns an error if the tracing subscriber fails to initialize
    pub fn init(&self) -> Result<()> {
        // Create environment filter that always applies our noise reduction rules
        let env_filter = env::var("RUST_LOG")
            .map_or_else(
                |_| {
                    // Default filter
                    EnvFilter::new(&self.level)
                },
                |env_directive| {
                    // If RUST_LOG is set, use it as base but add our noise reduction
                    EnvFilter::new(&env_directive)
                },
            )
            // Always apply noise reduction regardless of RUST_LOG setting
            .add_directive(
                "hyper=warn"
                    .parse()
                    .unwrap_or_else(|_| tracing::Level::WARN.into()),
            )
            .add_directive(
                "hyper::proto=warn"
                    .parse()
                    .unwrap_or_else(|_| tracing::Level::WARN.into()),
            )
            .add_directive(
                "reqwest=warn"
                    .parse()
                    .unwrap_or_else(|_| tracing::Level::WARN.into()),
            )
            .add_directive(
                "sqlx=info"
                    .parse()
                    .unwrap_or_else(|_| tracing::Level::INFO.into()),
            )
            .add_directive(
                "sqlx::query=info"
                    .parse()
                    .unwrap_or_else(|_| tracing::Level::INFO.into()),
            )
            .add_directive(
                "warp::server=info"
                    .parse()
                    .unwrap_or_else(|_| tracing::Level::INFO.into()),
            )
            .add_directive(
                "tower_http=info"
                    .parse()
                    .unwrap_or_else(|_| tracing::Level::INFO.into()),
            )
            // Keep our application logs at desired level
            .add_directive(
                format!("pierre_mcp_server={}", self.level)
                    .parse()
                    .unwrap_or_else(|_| tracing::Level::INFO.into()),
            );

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

    /// Create `OpenTelemetry` layer for distributed tracing
    ///
    /// Currently disabled due to dependency version conflicts with tokio-tungstenite.
    /// `OpenTelemetry` requires specific versions that conflict with `WebSocket` dependencies.
    #[allow(dead_code, clippy::unused_self, clippy::unnecessary_wraps)]
    fn create_telemetry_layer(&self) -> Result<(), anyhow::Error> {
        // OpenTelemetry integration disabled due to version compatibility issues
        // Can be enabled once dependency conflicts are resolved
        tracing::info!(
            "`OpenTelemetry` layer creation requested but disabled due to dependency conflicts"
        );
        Ok(())
    }

    /// Create GCP optimized logging configuration
    #[must_use]
    pub fn for_gcp_cloud_run() -> Self {
        Self {
            level: "info".into(),
            format: LogFormat::Json,
            include_location: false,
            include_thread: false,
            include_spans: true,
            service_name: service_names::PIERRE_MCP_SERVER.into(),
            service_version: env!("CARGO_PKG_VERSION").to_owned(),
            environment: "production".into(),
            enable_telemetry: true,
            request_id_header: "x-request-id".into(),
            enable_gcp_format: true,
            truncate_mcp_logs: false, // Production wants full logs
        }
    }
}

/// Initialize logging with default configuration
///
/// # Errors
///
/// Returns an error if logging initialization fails
pub fn init_default() -> Result<()> {
    LoggingConfig::default().init()
}

/// Initialize logging from environment
///
/// # Errors
///
/// Returns an error if logging initialization fails
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

    /// Log `OAuth` events
    pub fn log_oauth_event(user_id: &str, provider: &str, event: &str, success: bool) {
        info!(
            user.id = %user_id,
            oauth.provider = %provider,
            oauth.event = %event,
            oauth.success = %success,
            "OAuth event"
        );
    }

    /// Log `API` requests
    pub fn log_api_request(
        method: &str,
        path: &str,
        status: u16,
        duration_ms: u64,
        user_id: Option<&str>,
    ) {
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
    pub fn log_security_event(
        event_type: &str,
        severity: &str,
        details: &str,
        user_id: Option<&str>,
    ) {
        warn!(
            security.event = %event_type,
            security.severity = %severity,
            security.details = %details,
            user.id = user_id.unwrap_or("unknown"),
            "Security event"
        );
    }

    /// Log performance metrics
    pub fn log_performance_metric(
        metric_name: &str,
        value: f64,
        unit: &str,
        tags: Option<&serde_json::Value>,
    ) {
        let default_tags = json!({});
        info!(
            metric.name = %metric_name,
            metric.value = %value,
            metric.unit = %unit,
            metric.tags = %tags.unwrap_or(&default_tags),
            "Performance metric"
        );
    }
}
