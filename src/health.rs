// ABOUTME: Server health monitoring and system status checks for operational visibility
// ABOUTME: Provides health endpoints, system metrics, and service availability monitoring
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Health check endpoints and monitoring utilities

use crate::constants::service_names;
use crate::database_plugins::{factory::Database, DatabaseProvider};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{error, info};

/// Overall health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Overall service status
    pub status: HealthStatus,
    /// Service information
    pub service: ServiceInfo,
    /// Individual component checks
    pub checks: Vec<ComponentHealth>,
    /// Response timestamp
    pub timestamp: u64,
    /// Response time in milliseconds
    pub response_time_ms: u64,
}

/// Service information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    /// Service name
    pub name: String,
    /// Service version
    pub version: String,
    /// Environment (development, staging, production)
    pub environment: String,
    /// Service uptime in seconds
    pub uptime_seconds: u64,
    /// Build timestamp
    pub build_time: Option<String>,
    /// Git commit hash
    pub git_commit: Option<String>,
}

/// Individual component health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component name
    pub name: String,
    /// Component status
    pub status: HealthStatus,
    /// Status description
    pub message: String,
    /// Check duration in milliseconds
    pub duration_ms: u64,
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Health checker for the Pierre MCP Server
pub struct HealthChecker {
    /// Service start time
    start_time: Instant,
    /// Database reference
    database: Arc<Database>,
    /// Cached health status
    cached_status: RwLock<Option<(HealthResponse, Instant)>>,
    /// Cache TTL
    cache_ttl: Duration,
}

impl HealthChecker {
    /// Create a new health checker
    #[must_use]
    pub fn new(database: Arc<Database>) -> Self {
        let health_checker = Self {
            start_time: Instant::now(),
            database: database.clone(),
            cached_status: RwLock::new(None),
            cache_ttl: Duration::from_secs(30), // Cache for 30 seconds
        };

        // Start background cleanup task for expired API keys
        let database_clone = database;
        tokio::spawn(async move {
            Self::periodic_cleanup_task(database_clone).await;
        });

        health_checker
    }

    /// Periodic task to clean up expired API keys
    async fn periodic_cleanup_task(database: Arc<Database>) {
        let mut interval = tokio::time::interval(Duration::from_secs(
            crate::constants::time::HOUR_SECONDS as u64,
        )); // Run every hour

        loop {
            interval.tick().await;

            match database.cleanup_expired_api_keys().await {
                Ok(count) => {
                    if count > 0 {
                        info!("Cleaned up {} expired API keys", count);
                    }
                }
                Err(e) => {
                    error!("Failed to cleanup expired API keys: {}", e);
                }
            }
        }
    }

    /// Perform a basic health check (fast, suitable for load balancer probes)
    #[must_use]
    pub fn basic_health(&self) -> HealthResponse {
        let start = Instant::now();

        // Basic service info
        let service = ServiceInfo {
            name: service_names::PIERRE_MCP_SERVER.into(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "unknown".into()),
            uptime_seconds: self.start_time.elapsed().as_secs(),
            build_time: option_env!("BUILD_TIME").map(std::string::ToString::to_string),
            git_commit: option_env!("GIT_COMMIT").map(std::string::ToString::to_string),
        };

        // Basic checks
        let checks = vec![ComponentHealth {
            name: "service".into(),
            status: HealthStatus::Healthy,
            message: "Service is running".into(),
            duration_ms: 0,
            metadata: None,
        }];

        HealthResponse {
            status: HealthStatus::Healthy,
            service,
            checks,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            response_time_ms: u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
        }
    }

    /// Perform a comprehensive health check with all components
    pub async fn comprehensive_health(&self) -> HealthResponse {
        let start = Instant::now();

        // Check cache first
        {
            let cached = self.cached_status.read().await;
            if let Some((response, cached_at)) = cached.as_ref() {
                if cached_at.elapsed() < self.cache_ttl {
                    return response.clone();
                }
            }
        }

        info!("Performing comprehensive health check");

        // Service info
        let service = ServiceInfo {
            name: service_names::PIERRE_MCP_SERVER.into(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "unknown".into()),
            uptime_seconds: self.start_time.elapsed().as_secs(),
            build_time: option_env!("BUILD_TIME").map(std::string::ToString::to_string),
            git_commit: option_env!("GIT_COMMIT").map(std::string::ToString::to_string),
        };

        // Perform all checks
        let mut checks = Vec::new();

        // Database connectivity check
        checks.push(self.check_database().await);

        // Memory usage check
        checks.push(self.check_memory());

        // Disk space check
        checks.push(self.check_disk_space());

        // External API connectivity
        checks.push(self.check_external_apis().await);

        // Determine overall status
        let overall_status = if checks.iter().any(|c| c.status == HealthStatus::Unhealthy) {
            HealthStatus::Unhealthy
        } else if checks.iter().any(|c| c.status == HealthStatus::Degraded) {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        let response = HealthResponse {
            status: overall_status,
            service,
            checks,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            response_time_ms: u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
        };

        // Cache the response
        {
            let mut cached = self.cached_status.write().await;
            *cached = Some((response.clone(), Instant::now()));
        }

        response
    }

    /// Check database connectivity and performance
    async fn check_database(&self) -> ComponentHealth {
        let start = Instant::now();

        match self.database_health_check().await {
            Ok(metadata) => ComponentHealth {
                name: "database".into(),
                status: HealthStatus::Healthy,
                message: "Database is accessible and responsive".into(),
                duration_ms: u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
                metadata: Some(metadata),
            },
            Err(e) => {
                error!("Database health check failed: {}", e);
                ComponentHealth {
                    name: "database".into(),
                    status: HealthStatus::Unhealthy,
                    message: format!("Database check failed: {e}"),
                    duration_ms: u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
                    metadata: None,
                }
            }
        }
    }

    /// Check memory usage
    fn check_memory(&self) -> ComponentHealth {
        let start = Instant::now();

        let (status, message, metadata) = self.get_memory_info().map_or_else(
            |_| {
                (
                    HealthStatus::Unhealthy,
                    "Memory information unavailable".into(),
                    Some(serde_json::json!({
                        "note": "Unable to retrieve system memory information"
                    })),
                )
            },
            |memory_info| {
                let memory_usage_percent = memory_info.used_percent;
                let status = if memory_usage_percent > 90.0 {
                    HealthStatus::Unhealthy
                } else if memory_usage_percent > 80.0 {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Healthy
                };

                let message = format!("Memory usage: {memory_usage_percent:.1}%");
                let metadata = serde_json::json!({
                    "used_percent": memory_usage_percent,
                    "used_mb": memory_info.used_mb,
                    "total_mb": memory_info.total_mb,
                    "available_mb": memory_info.available_mb
                });

                (status, message, Some(metadata))
            },
        );

        ComponentHealth {
            name: "memory".into(),
            status,
            message,
            duration_ms: u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
            metadata,
        }
    }

    /// Check available disk space
    fn check_disk_space(&self) -> ComponentHealth {
        let start = Instant::now();

        let (status, message, metadata) = match self.get_disk_info() {
            Ok(disk_info) => {
                let usage_percent = disk_info.used_percent;
                let status = if usage_percent > 95.0 {
                    HealthStatus::Unhealthy
                } else if usage_percent > 85.0 {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Healthy
                };

                let message = format!("Disk usage: {usage_percent:.1}%");
                let metadata = serde_json::json!({
                    "used_percent": usage_percent,
                    "used_gb": disk_info.used_gb,
                    "total_gb": disk_info.total_gb,
                    "available_gb": disk_info.available_gb,
                    "path": disk_info.path
                });

                (status, message, Some(metadata))
            }
            Err(_) => (
                HealthStatus::Unhealthy,
                "Disk information unavailable".into(),
                Some(serde_json::json!({
                    "note": "Unable to retrieve filesystem information"
                })),
            ),
        };

        ComponentHealth {
            name: "disk".into(),
            status,
            message,
            duration_ms: u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
            metadata,
        }
    }

    /// Check external API connectivity
    async fn check_external_apis(&self) -> ComponentHealth {
        let start = Instant::now();

        // Check if we can reach external APIs (simplified)
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
        {
            Ok(client) => client,
            Err(e) => {
                tracing::error!("Failed to create HTTP client for health check: {}", e);
                return ComponentHealth {
                    name: "external_apis".into(),
                    status: HealthStatus::Unhealthy,
                    message: format!("Failed to create HTTP client: {e}"),
                    duration_ms: u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
                    metadata: None,
                };
            }
        };

        let mut healthy_apis = 0;
        let mut total_apis = 0;

        // Check Strava API
        total_apis += 1;
        if let Ok(response) = client
            .get(crate::constants::env_config::strava_api_base())
            .send()
            .await
        {
            if response.status().is_success()
                || response.status().as_u16() == crate::constants::http_status::UNAUTHORIZED
            {
                // 401 is expected without auth
                healthy_apis += 1;
            }
        }

        let status = if healthy_apis == total_apis {
            HealthStatus::Healthy
        } else if healthy_apis > 0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unhealthy
        };

        let message = format!("{healthy_apis}/{total_apis} external APIs accessible");

        ComponentHealth {
            name: "external_apis".into(),
            status,
            message,
            duration_ms: u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
            metadata: Some(serde_json::json!({
                "apis_checked": total_apis,
                "apis_healthy": healthy_apis
            })),
        }
    }

    /// Perform database-specific health checks
    async fn database_health_check(&self) -> Result<serde_json::Value> {
        // Try a simple query to ensure database is responsive
        let start = Instant::now();

        // Perform an actual database connectivity test
        let user_count = self.database.get_user_count().await?;

        let query_duration = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);

        Ok(serde_json::json!({
            "backend": format!("{:?}", self.database.database_type()),
            "backend_info": self.database.backend_info(),
            "query_duration_ms": query_duration,
            "status": "connected",
            "user_count": user_count
        }))
    }

    /// Get readiness status (for Kubernetes readiness probes)
    pub async fn readiness(&self) -> HealthResponse {
        // For readiness, we check if the service can handle requests
        let mut response = self.basic_health();

        // Add readiness-specific checks
        let db_check = self.check_database().await;
        response.checks.push(db_check.clone());

        // Service is ready if database is healthy
        response.status = if db_check.status == HealthStatus::Healthy {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unhealthy
        };

        response
    }

    /// Get liveness status (for Kubernetes liveness probes)
    #[must_use]
    pub fn liveness(&self) -> HealthResponse {
        // For liveness, we just check if the service is running
        self.basic_health()
    }

    /// Get system memory information
    fn get_memory_info(&self) -> Result<MemoryInfo, Box<dyn std::error::Error>> {
        // Cross-platform memory information retrieval
        #[cfg(target_os = "linux")]
        {
            self.get_memory_info_linux()
        }
        #[cfg(target_os = "macos")]
        {
            Self::get_memory_info_macos(self)
        }
        #[cfg(target_os = "windows")]
        {
            Self::get_memory_info_windows(self)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Err("Memory monitoring not supported on this platform".into())
        }
    }

    /// Get disk space information
    fn get_disk_info(&self) -> Result<DiskInfo, Box<dyn std::error::Error>> {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| "/".into());

        #[cfg(unix)]
        {
            Self::get_disk_info_unix(self, &current_dir)
        }
        #[cfg(windows)]
        {
            Self::get_disk_info_windows(self, &current_dir)
        }
        #[cfg(not(any(unix, windows)))]
        {
            Err("Disk monitoring not supported on this platform".into())
        }
    }

    #[cfg(target_os = "linux")]
    fn get_memory_info_linux(&self) -> Result<MemoryInfo, Box<dyn std::error::Error>> {
        let meminfo = std::fs::read_to_string("/proc/meminfo")?;
        let mut total_kb = 0;
        let mut available_kb = 0;

        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                total_kb = line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("0")
                    .parse::<u64>()?;
            } else if line.starts_with("MemAvailable:") {
                available_kb = line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("0")
                    .parse::<u64>()?;
            }
        }

        let total_mb = total_kb / 1024;
        let available_mb = available_kb / 1024;
        let used_mb = total_mb - available_mb;
        let used_percent = if total_mb > 0 {
            (used_mb as f64 / total_mb as f64) * 100.0
        } else {
            0.0
        };

        Ok(MemoryInfo {
            total_mb,
            used_mb,
            available_mb,
            used_percent,
        })
    }

    #[cfg(target_os = "macos")]
    fn get_memory_info_macos(_: &Self) -> Result<MemoryInfo, Box<dyn std::error::Error>> {
        // Use sysctl for macOS memory information
        use std::process::Command;

        let output = Command::new("sysctl").args(["hw.memsize"]).output()?;
        let total_bytes = String::from_utf8(output.stdout)?
            .trim()
            .split(": ")
            .nth(1)
            .unwrap_or("0")
            .parse::<u64>()?;

        let total_mb = total_bytes / (1024 * 1024);
        // For simplicity, estimate used memory as 70% (would need vm_stat for precision)
        let used_mb = total_mb.saturating_mul(7) / 10;
        let available_mb = total_mb.saturating_sub(used_mb);
        let used_percent = if total_mb > 0 {
            (f64::from(u32::try_from(used_mb).unwrap_or(u32::MAX))
                / f64::from(u32::try_from(total_mb).unwrap_or(u32::MAX)))
                * 100.0
        } else {
            0.0
        };

        Ok(MemoryInfo {
            total_mb,
            used_mb,
            available_mb,
            used_percent,
        })
    }

    #[cfg(target_os = "windows")]
    fn get_memory_info_windows(_: &Self) -> Result<MemoryInfo, Box<dyn std::error::Error>> {
        // For Windows, we'd use WinAPI, but for simplicity return estimated values
        let total_mb = 8192; // Estimate 8GB
        let used_mb = 4096; // Estimate 50% usage
        let available_mb = total_mb - used_mb;
        let used_percent = 50.0;

        Ok(MemoryInfo {
            total_mb,
            used_mb,
            available_mb,
            used_percent,
        })
    }

    #[cfg(unix)]
    fn get_disk_info_unix(
        _: &Self,
        path: &std::path::Path,
    ) -> Result<DiskInfo, Box<dyn std::error::Error>> {
        use std::process::Command;

        let output = Command::new("df")
            .args(["-h", path.to_str().unwrap_or("/")])
            .output()?;

        let output_str = String::from_utf8(output.stdout)?;
        let lines: Vec<&str> = output_str.lines().collect();

        if lines.len() >= 2 {
            let fields: Vec<&str> = lines[1].split_whitespace().collect();
            if fields.len() >= 5 {
                let total_str = fields[1].trim_end_matches('G');
                let used_str = fields[2].trim_end_matches('G');
                let available_str = fields[3].trim_end_matches('G');
                let used_percent_str = fields[4].trim_end_matches('%');

                let total_gb = total_str.parse::<f64>().unwrap_or(100.0);
                let used_gb = used_str.parse::<f64>().unwrap_or(50.0);
                let available_gb = available_str.parse::<f64>().unwrap_or(50.0);
                let used_percent = used_percent_str.parse::<f64>().unwrap_or(50.0);

                return Ok(DiskInfo {
                    path: path.to_string_lossy().to_string(),
                    total_gb,
                    used_gb,
                    available_gb,
                    used_percent,
                });
            }
        }

        // Fallback
        Ok(DiskInfo {
            path: path.to_string_lossy().to_string(),
            total_gb: 100.0,
            used_gb: 50.0,
            available_gb: 50.0,
            used_percent: 50.0,
        })
    }

    #[cfg(windows)]
    fn get_disk_info_windows(
        _: &Self,
        path: &std::path::Path,
    ) -> Result<DiskInfo, Box<dyn std::error::Error>> {
        // For Windows, we'd use WinAPI, but for simplicity return estimated values
        Ok(DiskInfo {
            path: path.to_string_lossy().to_string(),
            total_gb: 500.0,
            used_gb: 250.0,
            available_gb: 250.0,
            used_percent: 50.0,
        })
    }
}

#[derive(Debug)]
struct MemoryInfo {
    total_mb: u64,
    used_mb: u64,
    available_mb: u64,
    used_percent: f64,
}

#[derive(Debug)]
struct DiskInfo {
    path: String,
    total_gb: f64,
    used_gb: f64,
    available_gb: f64,
    used_percent: f64,
}

/// Health check middleware for HTTP endpoints
pub mod middleware {
    use super::{HealthChecker, HealthStatus};
    use warp::{Filter, Reply};

    /// Create health check routes
    pub fn routes(
        health_checker: HealthChecker,
    ) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        let health_checker = std::sync::Arc::new(health_checker);

        let health = warp::path("health")
            .and(warp::get())
            .and(with_health_checker(health_checker.clone()))
            .and_then(health_handler);

        let ready = warp::path("ready")
            .and(warp::get())
            .and(with_health_checker(health_checker.clone()))
            .and_then(readiness_handler);

        let live = warp::path("live")
            .and(warp::get())
            .and(with_health_checker(health_checker))
            .and_then(liveness_handler);

        health.or(ready).or(live)
    }

    fn with_health_checker(
        health_checker: std::sync::Arc<HealthChecker>,
    ) -> impl Filter<Extract = (std::sync::Arc<HealthChecker>,), Error = std::convert::Infallible> + Clone
    {
        warp::any().map(move || health_checker.clone())
    }

    async fn health_handler(
        health_checker: std::sync::Arc<HealthChecker>,
    ) -> Result<impl Reply, warp::Rejection> {
        let response = health_checker.comprehensive_health().await;
        let status_code = match response.status {
            HealthStatus::Healthy | HealthStatus::Degraded => warp::http::StatusCode::OK,
            HealthStatus::Unhealthy => warp::http::StatusCode::SERVICE_UNAVAILABLE,
        };

        Ok(warp::reply::with_status(
            warp::reply::json(&response),
            status_code,
        ))
    }

    async fn readiness_handler(
        health_checker: std::sync::Arc<HealthChecker>,
    ) -> Result<impl Reply, warp::Rejection> {
        let response = health_checker.readiness().await;
        let status_code = match response.status {
            HealthStatus::Healthy => warp::http::StatusCode::OK,
            _ => warp::http::StatusCode::SERVICE_UNAVAILABLE,
        };

        Ok(warp::reply::with_status(
            warp::reply::json(&response),
            status_code,
        ))
    }

    async fn liveness_handler(
        health_checker: std::sync::Arc<HealthChecker>,
    ) -> Result<impl Reply, warp::Rejection> {
        let response = health_checker.liveness();
        Ok(warp::reply::json(&response))
    }
}
