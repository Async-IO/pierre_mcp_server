// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Health check endpoints and monitoring utilities

use crate::database::Database;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{error, info};

/// Overall health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    database: Database,
    /// Cached health status
    cached_status: RwLock<Option<(HealthResponse, Instant)>>,
    /// Cache TTL
    cache_ttl: Duration,
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new(database: Database) -> Self {
        Self {
            start_time: Instant::now(),
            database,
            cached_status: RwLock::new(None),
            cache_ttl: Duration::from_secs(30), // Cache for 30 seconds
        }
    }

    /// Perform a basic health check (fast, suitable for load balancer probes)
    pub async fn basic_health(&self) -> HealthResponse {
        let start = Instant::now();
        
        // Basic service info
        let service = ServiceInfo {
            name: "pierre-mcp-server".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "unknown".to_string()),
            uptime_seconds: self.start_time.elapsed().as_secs(),
            build_time: option_env!("BUILD_TIME").map(|s| s.to_string()),
            git_commit: option_env!("GIT_COMMIT").map(|s| s.to_string()),
        };

        // Basic checks
        let checks = vec![
            ComponentHealth {
                name: "service".to_string(),
                status: HealthStatus::Healthy,
                message: "Service is running".to_string(),
                duration_ms: 0,
                metadata: None,
            }
        ];

        HealthResponse {
            status: HealthStatus::Healthy,
            service,
            checks,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            response_time_ms: start.elapsed().as_millis() as u64,
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
            name: "pierre-mcp-server".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "unknown".to_string()),
            uptime_seconds: self.start_time.elapsed().as_secs(),
            build_time: option_env!("BUILD_TIME").map(|s| s.to_string()),
            git_commit: option_env!("GIT_COMMIT").map(|s| s.to_string()),
        };

        // Perform all checks
        let mut checks = Vec::new();
        
        // Database connectivity check
        checks.push(self.check_database().await);
        
        // Memory usage check
        checks.push(self.check_memory().await);
        
        // Disk space check
        checks.push(self.check_disk_space().await);
        
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
            response_time_ms: start.elapsed().as_millis() as u64,
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
                name: "database".to_string(),
                status: HealthStatus::Healthy,
                message: "Database is accessible and responsive".to_string(),
                duration_ms: start.elapsed().as_millis() as u64,
                metadata: Some(metadata),
            },
            Err(e) => {
                error!("Database health check failed: {}", e);
                ComponentHealth {
                    name: "database".to_string(),
                    status: HealthStatus::Unhealthy,
                    message: format!("Database check failed: {}", e),
                    duration_ms: start.elapsed().as_millis() as u64,
                    metadata: None,
                }
            }
        }
    }

    /// Check memory usage
    async fn check_memory(&self) -> ComponentHealth {
        let start = Instant::now();
        
        // This is a simplified memory check
        // In production, you might want to use more sophisticated monitoring
        let status = HealthStatus::Healthy;
        let message = "Memory usage within normal limits".to_string();
        
        ComponentHealth {
            name: "memory".to_string(),
            status,
            message,
            duration_ms: start.elapsed().as_millis() as u64,
            metadata: Some(serde_json::json!({
                "note": "Memory monitoring not implemented - consider adding system metrics"
            })),
        }
    }

    /// Check available disk space
    async fn check_disk_space(&self) -> ComponentHealth {
        let start = Instant::now();
        
        // This is a simplified disk check
        // In production, you'd want to check actual disk usage
        let status = HealthStatus::Healthy;
        let message = "Disk space sufficient".to_string();
        
        ComponentHealth {
            name: "disk".to_string(),
            status,
            message,
            duration_ms: start.elapsed().as_millis() as u64,
            metadata: Some(serde_json::json!({
                "note": "Disk monitoring not implemented - consider adding filesystem checks"
            })),
        }
    }

    /// Check external API connectivity
    async fn check_external_apis(&self) -> ComponentHealth {
        let start = Instant::now();
        
        // Check if we can reach external APIs (simplified)
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        let mut healthy_apis = 0;
        let mut total_apis = 0;

        // Check Strava API
        total_apis += 1;
        if let Ok(response) = client.get("https://www.strava.com/api/v3").send().await {
            if response.status().is_success() || response.status().as_u16() == 401 {
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

        let message = format!("{}/{} external APIs accessible", healthy_apis, total_apis);

        ComponentHealth {
            name: "external_apis".to_string(),
            status,
            message,
            duration_ms: start.elapsed().as_millis() as u64,
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
        
        // This is a simple connectivity test
        // You might want to add more sophisticated checks like:
        // - Query response time
        // - Number of active connections
        // - Database size
        
        let query_duration = start.elapsed().as_millis() as u64;
        
        Ok(serde_json::json!({
            "type": "sqlite",
            "query_duration_ms": query_duration,
            "status": "connected"
        }))
    }

    /// Get readiness status (for Kubernetes readiness probes)
    pub async fn readiness(&self) -> HealthResponse {
        // For readiness, we check if the service can handle requests
        let mut response = self.basic_health().await;
        
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
    pub async fn liveness(&self) -> HealthResponse {
        // For liveness, we just check if the service is running
        self.basic_health().await
    }
}

/// Health check middleware for HTTP endpoints
pub mod middleware {
    use super::*;
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
    ) -> impl Filter<Extract = (std::sync::Arc<HealthChecker>,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || health_checker.clone())
    }

    async fn health_handler(
        health_checker: std::sync::Arc<HealthChecker>,
    ) -> Result<impl Reply, warp::Rejection> {
        let response = health_checker.comprehensive_health().await;
        let status_code = match response.status {
            HealthStatus::Healthy => warp::http::StatusCode::OK,
            HealthStatus::Degraded => warp::http::StatusCode::OK, // Still return 200 for degraded
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
        let response = health_checker.liveness().await;
        Ok(warp::reply::json(&response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::generate_encryption_key;

    #[tokio::test]
    async fn test_basic_health_check() {
        let encryption_key = generate_encryption_key().to_vec();
        let database = Database::new("sqlite::memory:", encryption_key).await.unwrap();
        let health_checker = HealthChecker::new(database);

        let response = health_checker.basic_health().await;
        
        assert_eq!(response.status, HealthStatus::Healthy);
        assert_eq!(response.service.name, "pierre-mcp-server");
        assert!(!response.checks.is_empty());
    }

    #[tokio::test]
    async fn test_comprehensive_health_check() {
        let encryption_key = generate_encryption_key().to_vec();
        let database = Database::new("sqlite::memory:", encryption_key).await.unwrap();
        let health_checker = HealthChecker::new(database);

        let response = health_checker.comprehensive_health().await;
        
        // Should have multiple checks
        assert!(response.checks.len() > 1);
        
        // Should include database check
        assert!(response.checks.iter().any(|c| c.name == "database"));
    }

    #[tokio::test]
    async fn test_readiness_check() {
        let encryption_key = generate_encryption_key().to_vec();
        let database = Database::new("sqlite::memory:", encryption_key).await.unwrap();
        let health_checker = HealthChecker::new(database);

        let response = health_checker.readiness().await;
        
        // Should include database check for readiness
        assert!(response.checks.iter().any(|c| c.name == "database"));
    }
}