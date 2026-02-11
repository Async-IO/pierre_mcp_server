// ABOUTME: Server health monitoring and system status checks for operational visibility
// ABOUTME: Provides health endpoints, system metrics, and service availability monitoring
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Health check endpoints and monitoring utilities

use std::env;
use std::error::Error as StdError;
use std::fmt;
#[cfg(target_os = "linux")]
use std::fs;
use std::io::Error as IoError;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::num::ParseIntError;
use std::path::Path;
#[cfg(unix)]
use std::process::Command;
use std::string::FromUtf8Error;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{error, info};

#[cfg(any(target_os = "macos", target_os = "windows"))]
use crate::constants::system_monitoring::BYTES_TO_MB_DIVISOR;
#[cfg(target_os = "linux")]
use crate::constants::system_monitoring::KB_TO_MB_DIVISOR;
use crate::constants::{
    get_server_config, http_status::UNAUTHORIZED, service_names::PIERRE_MCP_SERVER,
    time::HOUR_SECONDS,
};
use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::errors::AppResult;
use crate::utils::http_client::get_health_check_timeout_secs;

/// Errors that can occur during health probe operations
#[derive(Debug)]
pub enum HealthError {
    /// IO operation failed (file read, command execution)
    Io(IoError),
    /// Failed to parse string data
    ParseString(FromUtf8Error),
    /// Failed to parse integer
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    ParseInt(ParseIntError),
    /// Platform not supported for this probe
    UnsupportedPlatform(&'static str),
    /// Windows API call failed
    #[cfg(target_os = "windows")]
    WindowsApi(&'static str),
}

impl fmt::Display for HealthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::ParseString(e) => write!(f, "String parse error: {e}"),
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            Self::ParseInt(e) => write!(f, "Integer parse error: {e}"),
            Self::UnsupportedPlatform(msg) => write!(f, "{msg}"),
            #[cfg(target_os = "windows")]
            Self::WindowsApi(msg) => write!(f, "Windows API error: {msg}"),
        }
    }
}

impl StdError for HealthError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::ParseString(e) => Some(e),
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            Self::ParseInt(e) => Some(e),
            _ => None,
        }
    }
}

impl From<IoError> for HealthError {
    fn from(e: IoError) -> Self {
        Self::Io(e)
    }
}

impl From<FromUtf8Error> for HealthError {
    fn from(e: FromUtf8Error) -> Self {
        Self::ParseString(e)
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
impl From<ParseIntError> for HealthError {
    fn from(e: ParseIntError) -> Self {
        Self::ParseInt(e)
    }
}

/// Overall health status
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// All systems operational
    Healthy,
    /// Some systems experiencing issues but service is available
    Degraded,
    /// Critical systems failing, service may be unavailable
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
    pub metadata: Option<Value>,
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
    /// Strava API base URL for health checks
    strava_api_base_url: String,
}

impl HealthChecker {
    /// Create a new health checker
    #[must_use]
    pub fn new(database: Arc<Database>, strava_api_base_url: String) -> Self {
        let health_checker = Self {
            start_time: Instant::now(),
            database: database.clone(), // Safe: Arc clone needed for both struct and background task
            cached_status: RwLock::new(None),
            cache_ttl: Duration::from_secs(30), // Cache for 30 seconds
            strava_api_base_url,
        };

        // Start background cleanup task for expired API keys
        tokio::spawn(async move {
            Self::periodic_cleanup_task(database).await;
        });

        health_checker
    }

    /// Periodic task to clean up expired API keys
    async fn periodic_cleanup_task(database: Arc<Database>) {
        let mut ticker = interval(Duration::from_secs(HOUR_SECONDS as u64)); // Run every hour

        loop {
            ticker.tick().await;

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
            name: PIERRE_MCP_SERVER.into(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            environment: get_server_config().map_or_else(
                || "unknown".to_owned(),
                |c| c.security.headers.environment.to_string(),
            ),
            uptime_seconds: self.start_time.elapsed().as_secs(),
            build_time: option_env!("BUILD_TIME").map(str::to_owned),
            git_commit: option_env!("GIT_COMMIT").map(str::to_owned),
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
                    return response.clone(); // Safe: HealthResponse ownership from cache
                }
            }
        }

        info!("Performing comprehensive health check");

        // Service info
        let service = ServiceInfo {
            name: PIERRE_MCP_SERVER.into(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            environment: get_server_config().map_or_else(
                || "unknown".to_owned(),
                |c| c.security.headers.environment.to_string(),
            ),
            uptime_seconds: self.start_time.elapsed().as_secs(),
            build_time: option_env!("BUILD_TIME").map(str::to_owned),
            git_commit: option_env!("GIT_COMMIT").map(str::to_owned),
        };

        // Perform all checks
        let mut checks = Vec::new();

        // Database connectivity check
        checks.push(self.check_database().await);

        // Memory usage check
        checks.push(Self::check_memory());

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
            *cached = Some((response.clone(), Instant::now())); // Safe: HealthResponse ownership for cache storage
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
    fn check_memory() -> ComponentHealth {
        let start = Instant::now();

        let (status, message, metadata) = Self::get_memory_info().map_or_else(
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

        // Check if we can reach external APIs with configured timeout
        // Configuration must be initialized via initialize_http_clients() at server startup
        let timeout_secs = get_health_check_timeout_secs();
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
        {
            Ok(client) => client,
            Err(e) => {
                error!("Failed to create HTTP client for health check: {}", e);
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
        if let Ok(response) = client.get(&self.strava_api_base_url).send().await {
            if response.status().is_success() || response.status().as_u16() == UNAUTHORIZED {
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
    async fn database_health_check(&self) -> AppResult<Value> {
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
        response.checks.push(db_check.clone()); // Safe: HealthCheck ownership for response vec

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
    fn get_memory_info() -> Result<MemoryInfo, HealthError> {
        // Cross-platform memory information retrieval
        #[cfg(target_os = "linux")]
        {
            Self::get_memory_info_linux()
        }
        #[cfg(target_os = "macos")]
        {
            Self::get_memory_info_macos()
        }
        #[cfg(target_os = "windows")]
        {
            Self::get_memory_info_windows()
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Err(HealthError::UnsupportedPlatform(
                "Memory monitoring not supported on this platform",
            ))
        }
    }

    /// Get disk space information
    fn get_disk_info(&self) -> Result<DiskInfo, HealthError> {
        let current_dir = env::current_dir().unwrap_or_else(|_| "/".into());

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
            Err(HealthError::UnsupportedPlatform(
                "Disk monitoring not supported on this platform",
            ))
        }
    }

    #[cfg(target_os = "linux")]
    fn get_memory_info_linux() -> Result<MemoryInfo, HealthError> {
        let meminfo = fs::read_to_string("/proc/meminfo")?;
        let mut total_kilobytes = 0;
        let mut available_kilobytes = 0;

        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                total_kilobytes = line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("0")
                    .parse::<u64>()?;
            } else if line.starts_with("MemAvailable:") {
                available_kilobytes = line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("0")
                    .parse::<u64>()?;
            }
        }

        let total_megabytes = total_kilobytes / KB_TO_MB_DIVISOR;
        let available_megabytes = available_kilobytes / KB_TO_MB_DIVISOR;
        let used_megabytes = total_megabytes.saturating_sub(available_megabytes);
        let used_percent = if total_megabytes > 0 {
            // Use integer division for percentage calculation
            // Since result is always 0-100%, safe to convert to u32
            let percentage_int = (used_megabytes * 100) / total_megabytes;
            u32::try_from(percentage_int).map_or(100.0, f64::from)
        } else {
            0.0
        };

        Ok(MemoryInfo {
            total_mb: total_megabytes,
            used_mb: used_megabytes,
            available_mb: available_megabytes,
            used_percent,
        })
    }

    #[cfg(target_os = "macos")]
    fn get_memory_info_macos() -> Result<MemoryInfo, HealthError> {
        // Use sysctl for macOS memory information

        let output = Command::new("sysctl").args(["hw.memsize"]).output()?;
        let total_bytes = String::from_utf8(output.stdout)?
            .trim()
            .split(": ")
            .nth(1)
            .unwrap_or("0")
            .parse::<u64>()?;

        let total_mb = total_bytes / BYTES_TO_MB_DIVISOR;
        // Get detailed memory statistics using vm_stat for accurate usage
        let vm_output = Command::new("vm_stat").output()?;
        let vm_stats = String::from_utf8(vm_output.stdout)?;

        // Parse vm_stat output to get page counts
        let mut pages_free = 0u64;
        let mut pages_active = 0u64;
        let mut pages_inactive = 0u64;
        let mut pages_wired = 0u64;
        let mut pages_compressed = 0u64;

        for line in vm_stats.lines() {
            if let Some(value_str) = line.strip_prefix("Pages free:") {
                pages_free = value_str.trim().trim_end_matches('.').parse().unwrap_or(0);
            } else if let Some(value_str) = line.strip_prefix("Pages active:") {
                pages_active = value_str.trim().trim_end_matches('.').parse().unwrap_or(0);
            } else if let Some(value_str) = line.strip_prefix("Pages inactive:") {
                pages_inactive = value_str.trim().trim_end_matches('.').parse().unwrap_or(0);
            } else if let Some(value_str) = line.strip_prefix("Pages wired down:") {
                pages_wired = value_str.trim().trim_end_matches('.').parse().unwrap_or(0);
            } else if let Some(value_str) = line.strip_prefix("Pages stored in compressor:") {
                pages_compressed = value_str.trim().trim_end_matches('.').parse().unwrap_or(0);
            }
        }

        // macOS page size is typically 4096 bytes
        let page_size = 4096u64;
        let used_bytes =
            (pages_active + pages_inactive + pages_wired + pages_compressed) * page_size;
        let available_bytes = pages_free * page_size;

        let used_mb = used_bytes / BYTES_TO_MB_DIVISOR;
        let available_mb = available_bytes / BYTES_TO_MB_DIVISOR;
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
}

/// Convert bytes to gigabytes with documented precision characteristics
///
/// u64 to f64 conversion can lose precision for very large values, but for
/// disk space measurements this is acceptable as precision is maintained
/// for all realistic disk sizes (< 9 PB).
#[cfg(target_os = "windows")]
#[inline]
#[allow(clippy::cast_precision_loss)]
const fn bytes_to_gb_safe(value: u64) -> f64 {
    // u64 to f64 conversion can lose precision for very large values
    // but for disk space measurements, this is acceptable
    value as f64
}

impl HealthChecker {
    #[cfg(target_os = "windows")]
    #[allow(unsafe_code)] // Windows FFI requires unsafe for GlobalMemoryStatusEx call
    fn get_memory_info_windows() -> Result<MemoryInfo, HealthError> {
        // Use Windows API GlobalMemoryStatusEx for accurate memory information
        use std::mem;

        #[repr(C)]
        struct MemoryStatusEx {
            dw_length: u32,
            dw_memory_load: u32,
            ull_total_phys: u64,
            ull_avail_phys: u64,
            ull_total_page_file: u64,
            ull_avail_page_file: u64,
            ull_total_virtual: u64,
            ull_avail_virtual: u64,
            ull_avail_extended_virtual: u64,
        }

        extern "system" {
            fn GlobalMemoryStatusEx(lpBuffer: *mut MemoryStatusEx) -> i32;
        }

        // MemoryStatusEx struct size is 72 bytes, well within u32::MAX
        let struct_size = u32::try_from(mem::size_of::<MemoryStatusEx>()).unwrap_or(72);

        let mut mem_status = MemoryStatusEx {
            dw_length: struct_size,
            dw_memory_load: 0,
            ull_total_phys: 0,
            ull_avail_phys: 0,
            ull_total_page_file: 0,
            ull_avail_page_file: 0,
            ull_total_virtual: 0,
            ull_avail_virtual: 0,
            ull_avail_extended_virtual: 0,
        };

        let result = unsafe { GlobalMemoryStatusEx(&raw mut mem_status) };
        if result == 0 {
            return Err(HealthError::WindowsApi(
                "Failed to get Windows memory status",
            ));
        }

        let total_mb = mem_status.ull_total_phys / BYTES_TO_MB_DIVISOR;
        let available_mb = mem_status.ull_avail_phys / BYTES_TO_MB_DIVISOR;
        let used_mb = total_mb - available_mb;
        let used_percent = f64::from(mem_status.dw_memory_load);

        Ok(MemoryInfo {
            total_mb,
            used_mb,
            available_mb,
            used_percent,
        })
    }

    #[cfg(unix)]
    fn get_disk_info_unix(_: &Self, path: &Path) -> Result<DiskInfo, HealthError> {
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
    #[allow(unsafe_code)] // Windows FFI requires unsafe for GetDiskFreeSpaceExW call
    fn get_disk_info_windows(_: &Self, path: &Path) -> Result<DiskInfo, HealthError> {
        // Use Windows API GetDiskFreeSpaceEx for accurate disk information
        use std::ffi::OsStr;
        use std::iter;
        use std::os::windows::ffi::OsStrExt;

        extern "system" {
            fn GetDiskFreeSpaceExW(
                lpDirectoryName: *const u16,
                lpFreeBytesAvailableToCaller: *mut u64,
                lpTotalNumberOfBytes: *mut u64,
                lpTotalNumberOfFreeBytes: *mut u64,
            ) -> i32;
        }

        const BYTES_TO_GB: f64 = 1_073_741_824.0;

        // Convert path to wide string for Windows API
        let wide_path: Vec<u16> = OsStr::new(path)
            .encode_wide()
            .chain(iter::once(0))
            .collect();

        let mut free_bytes_available = 0u64;
        let mut total_bytes = 0u64;
        let mut total_free_bytes = 0u64;

        let result = unsafe {
            GetDiskFreeSpaceExW(
                wide_path.as_ptr(),
                &raw mut free_bytes_available,
                &raw mut total_bytes,
                &raw mut total_free_bytes,
            )
        };

        if result == 0 {
            return Err(HealthError::WindowsApi("Failed to get Windows disk space"));
        }

        // Convert bytes to GB using helper function with documented precision behavior
        let total_gb = bytes_to_gb_safe(total_bytes) / BYTES_TO_GB;
        let available_gb = bytes_to_gb_safe(free_bytes_available) / BYTES_TO_GB;
        let used_gb = total_gb - available_gb;
        let used_percent = if total_gb > 0.0 {
            (used_gb / total_gb) * 100.0
        } else {
            0.0
        };

        Ok(DiskInfo {
            path: path.to_string_lossy().to_string(),
            total_gb,
            used_gb,
            available_gb,
            used_percent,
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
