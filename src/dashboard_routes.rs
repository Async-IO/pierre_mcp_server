// ABOUTME: Dashboard web interface routes for user fitness data visualization
// ABOUTME: Provides HTTP endpoints for dashboard UI, charts, and interactive fitness analytics
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Dashboard routes for the API Key Management System frontend
//!
// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - HashMap key ownership for statistics aggregation (tool_name.clone())

use crate::auth::AuthResult;
use crate::database_plugins::DatabaseProvider;
use crate::errors::{AppError, AppResult};
use crate::mcp::resources::ServerResources;
use chrono::{Datelike, Duration, TimeZone, Utc};
use serde::Serialize;
use uuid::Uuid;

/// Dashboard overview with key metrics and recent activity
#[derive(Debug, Serialize)]
pub struct DashboardOverview {
    /// Total number of API keys
    pub total_api_keys: u32,
    /// Number of active API keys
    pub active_api_keys: u32,
    /// Total requests made today
    pub total_requests_today: u64,
    /// Total requests made this month
    pub total_requests_this_month: u64,
    /// Usage breakdown by tier
    pub current_month_usage_by_tier: Vec<TierUsage>,
    /// Recent API activity
    pub recent_activity: Vec<RecentActivity>,
}

/// Usage statistics for a specific tier
#[derive(Debug, Serialize)]
pub struct TierUsage {
    /// Tier name (free, basic, pro, enterprise)
    pub tier: String,
    /// Number of keys in this tier
    pub key_count: u32,
    /// Total requests from this tier
    pub total_requests: u64,
    /// Average requests per key in this tier
    pub average_requests_per_key: f64,
}

/// Recent API activity entry
#[derive(Debug, Serialize)]
pub struct RecentActivity {
    /// When the request occurred
    pub timestamp: chrono::DateTime<Utc>,
    /// Name of the API key used
    pub api_key_name: String,
    /// Tool that was invoked
    pub tool_name: String,
    /// HTTP status code of response
    pub status_code: i32,
    /// Response time in milliseconds
    pub response_time_ms: Option<i32>,
}

/// Detailed usage analytics with time series data
#[derive(Debug, Serialize)]
pub struct UsageAnalytics {
    /// Time series of usage data points
    pub time_series: Vec<UsageDataPoint>,
    /// Most frequently used tools
    pub top_tools: Vec<ToolUsage>,
    /// Percentage of requests that resulted in errors
    pub error_rate: f64,
    /// Average response time across all requests (ms)
    pub average_response_time: f64,
}

/// Single data point in usage time series
#[derive(Debug, Serialize)]
pub struct UsageDataPoint {
    /// Timestamp for this data point
    pub timestamp: chrono::DateTime<Utc>,
    /// Number of requests in this period
    pub request_count: u64,
    /// Number of errors in this period
    pub error_count: u64,
    /// Average response time in this period (ms)
    pub average_response_time: f64,
}

/// Usage statistics for a specific tool
#[derive(Debug, Serialize)]
pub struct ToolUsage {
    /// Name of the tool
    pub tool_name: String,
    /// Number of times the tool was called
    pub request_count: u64,
    /// Percentage of successful calls
    pub success_rate: f64,
    /// Average response time (ms)
    pub average_response_time: f64,
}

/// Rate limit status for an API key
#[derive(Debug, Serialize)]
pub struct RateLimitOverview {
    /// API key UUID
    pub api_key_id: String,
    /// Friendly name of the API key
    pub api_key_name: String,
    /// Tier of the API key
    pub tier: String,
    /// Current usage count
    pub current_usage: u64,
    /// Rate limit (None for unlimited)
    pub limit: Option<u64>,
    /// Percentage of limit used
    pub usage_percentage: f64,
    /// When the rate limit resets
    pub reset_date: Option<chrono::DateTime<Utc>>,
}

/// Individual request log entry with detailed information
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct RequestLog {
    /// Unique identifier for this log entry
    pub id: String,
    /// When the request was made
    pub timestamp: chrono::DateTime<Utc>,
    /// API key UUID used for the request
    pub api_key_id: String,
    /// Friendly name of the API key
    pub api_key_name: String,
    /// Tool/endpoint that was invoked
    pub tool_name: String,
    /// HTTP status code of the response
    pub status_code: i32,
    /// Response time in milliseconds (if available)
    pub response_time_ms: Option<i32>,
    /// Error message if the request failed
    pub error_message: Option<String>,
    /// Request payload size in bytes
    pub request_size_bytes: Option<i32>,
    /// Response payload size in bytes
    pub response_size_bytes: Option<i32>,
}

/// Statistics about API request performance and success rates
#[derive(Debug, Serialize)]
pub struct RequestStats {
    /// Total number of requests in the time period
    pub total_requests: u64,
    /// Number of successful requests (2xx status codes)
    pub successful_requests: u64,
    /// Number of failed requests (4xx, 5xx status codes)
    pub failed_requests: u64,
    /// Average response time across all requests (ms)
    pub average_response_time: f64,
    /// Minimum response time observed (ms)
    pub min_response_time: Option<u32>,
    /// Maximum response time observed (ms)
    pub max_response_time: Option<u32>,
    /// Average requests per minute
    pub requests_per_minute: f64,
    /// Percentage of requests that failed (0-100)
    pub error_rate: f64,
}

/// Route handlers for the admin dashboard and metrics
#[derive(Clone)]
pub struct DashboardRoutes {
    resources: std::sync::Arc<ServerResources>,
}

impl DashboardRoutes {
    /// Creates a new dashboard routes instance with the given server resources
    #[must_use]
    pub const fn new(resources: std::sync::Arc<ServerResources>) -> Self {
        Self { resources }
    }

    /// Get dashboard overview data
    ///
    /// # Errors
    /// Returns an error if database queries fail, or date parsing fails
    ///
    /// # Panics
    /// Panics if date construction fails with invalid values
    pub async fn get_dashboard_overview(&self, auth: AuthResult) -> AppResult<DashboardOverview> {
        tracing::debug!("Dashboard overview request received");

        let user_id = auth.user_id;

        // Validate user_id is not nil
        if user_id.is_nil() {
            return Err(AppError::invalid_input("Invalid user ID"));
        }

        tracing::info!(
            "Dashboard overview data access granted for user: {}",
            user_id
        );

        // Get user's API keys
        let api_keys = self.resources.database.get_user_api_keys(user_id).await?;
        let total_api_keys = u32::try_from(api_keys.len()).unwrap_or(0);
        let active_api_keys =
            u32::try_from(api_keys.iter().filter(|k| k.is_active).count()).unwrap_or(0);

        // Calculate time ranges
        let today_start = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| AppError::internal("Failed to create today start time"))?
            .and_utc();
        let month_start = Utc::now()
            .date_naive()
            .with_day(1)
            .ok_or_else(|| AppError::internal("Failed to set month start day"))?
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| AppError::internal("Failed to create month start time"))?
            .and_utc();

        // Get usage statistics
        let mut total_requests_today = 0u64;
        let mut total_requests_this_month = 0u64;

        for api_key in &api_keys {
            // Today's usage
            let today_stats = self
                .resources
                .database
                .get_api_key_usage_stats(&api_key.id, today_start, Utc::now())
                .await?;
            total_requests_today += u64::from(today_stats.total_requests);

            // This month's usage
            let month_stats = self
                .resources
                .database
                .get_api_key_usage_stats(&api_key.id, month_start, Utc::now())
                .await?;
            total_requests_this_month += u64::from(month_stats.total_requests);
        }

        // Group by tier
        let mut tier_map: std::collections::HashMap<String, (u32, u64)> =
            std::collections::HashMap::new();
        for api_key in &api_keys {
            let tier_name = format!("{:?}", api_key.tier).to_lowercase();
            let month_stats = self
                .resources
                .database
                .get_api_key_usage_stats(&api_key.id, month_start, Utc::now())
                .await?;

            let entry = tier_map.entry(tier_name).or_insert((0, 0));
            entry.0 += 1; // key count
            entry.1 += u64::from(month_stats.total_requests); // total requests
        }

        let current_month_usage_by_tier: Vec<TierUsage> = tier_map
            .into_iter()
            .map(|(tier, (key_count, total_requests))| TierUsage {
                tier,
                key_count,
                total_requests,
                average_requests_per_key: if key_count > 0 {
                    {
                        f64::from(u32::try_from(total_requests).unwrap_or(u32::MAX))
                            / f64::from(key_count)
                    }
                } else {
                    0.0
                },
            })
            .collect();

        // Get recent activity (last 10 events)
        let recent_activity = self.get_recent_activity(user_id, 10).await?;

        Ok(DashboardOverview {
            total_api_keys,
            active_api_keys,
            total_requests_today,
            total_requests_this_month,
            current_month_usage_by_tier,
            recent_activity,
        })
    }

    /// Get usage analytics for charts
    ///
    /// # Errors
    /// Returns an error if authentication fails or database queries fail
    pub async fn get_usage_analytics(
        &self,
        auth: AuthResult,
        days: u32,
    ) -> AppResult<UsageAnalytics> {
        tracing::debug!("Dashboard analytics request received for {} days", days);

        let user_id = auth.user_id;

        // Validate user_id is not nil
        if user_id.is_nil() {
            return Err(AppError::invalid_input("Invalid user ID"));
        }

        tracing::info!(
            "Dashboard analytics data access granted for user: {} (timeframe: {} days)",
            user_id,
            days
        );

        let api_keys = self.resources.database.get_user_api_keys(user_id).await?;
        let start_date = Utc::now() - Duration::days(i64::from(days));

        // Time series data (daily aggregates)
        let mut time_series = Vec::new();
        for day in 0..days {
            let day_start = start_date + Duration::days(i64::from(day));
            let day_end = day_start + Duration::days(1);

            let mut total_requests = 0u64;
            let mut total_errors = 0u64;
            let mut total_response_time = 0u64;
            let mut response_count = 0u64;

            for api_key in &api_keys {
                let stats = self
                    .resources
                    .database
                    .get_api_key_usage_stats(&api_key.id, day_start, day_end)
                    .await?;

                total_requests += u64::from(stats.total_requests);
                total_errors += u64::from(stats.failed_requests);
                total_response_time += stats.total_response_time_ms;
                response_count += u64::from(stats.total_requests);
            }

            time_series.push(UsageDataPoint {
                timestamp: day_start,
                request_count: total_requests,
                error_count: total_errors,
                average_response_time: if response_count > 0 {
                    {
                        f64::from(u32::try_from(total_response_time).unwrap_or(u32::MAX))
                            / f64::from(u32::try_from(response_count).unwrap_or(u32::MAX))
                    }
                } else {
                    0.0
                },
            });
        }

        // Top tools analysis
        let top_tools = self
            .get_top_tools_analysis(user_id, start_date, Utc::now())
            .await?;

        // Overall metrics
        let total_requests: u64 = time_series.iter().map(|d| d.request_count).sum();
        let total_errors: u64 = time_series.iter().map(|d| d.error_count).sum();
        let error_rate = if total_requests > 0 {
            (f64::from(u32::try_from(total_errors).unwrap_or(u32::MAX))
                / f64::from(u32::try_from(total_requests).unwrap_or(u32::MAX)))
                * 100.0
        } else {
            0.0
        };

        let average_response_time = if time_series.is_empty() {
            0.0
        } else {
            time_series
                .iter()
                .map(|d| d.average_response_time)
                .sum::<f64>()
                / {
                    {
                        f64::from(u32::try_from(time_series.len()).unwrap_or(u32::MAX))
                    }
                }
        };

        Ok(UsageAnalytics {
            time_series,
            top_tools,
            error_rate,
            average_response_time,
        })
    }

    /// Get rate limit overview for all user's API keys
    ///
    /// # Errors
    /// Returns an error if authentication fails or database queries fail
    ///
    /// # Panics
    /// Panics if date construction fails with invalid values
    pub async fn get_rate_limit_overview(
        &self,
        auth: AuthResult,
    ) -> AppResult<Vec<RateLimitOverview>> {
        tracing::debug!("Dashboard rate limit overview request received");

        let user_id = auth.user_id;

        // Validate user_id is not nil
        if user_id.is_nil() {
            return Err(AppError::invalid_input("Invalid user ID"));
        }

        tracing::info!(
            "Dashboard rate limit data access granted for user: {}",
            user_id
        );

        let api_keys = self.resources.database.get_user_api_keys(user_id).await?;
        let mut overview = Vec::new();

        for api_key in api_keys {
            let current_usage = self
                .resources
                .database
                .get_api_key_current_usage(&api_key.id)
                .await?;

            let limit = if api_key.tier == crate::api_keys::ApiKeyTier::Enterprise {
                None
            } else {
                Some(u64::from(api_key.rate_limit_requests))
            };

            let usage_percentage = limit.map_or(0.0, |limit_val| {
                if limit_val > 0 {
                    {
                        (f64::from(current_usage)
                            / f64::from(u32::try_from(limit_val).unwrap_or(u32::MAX)))
                            * 100.0
                    }
                } else {
                    0.0
                }
            });

            // Calculate reset date (first day of next month)
            let now = Utc::now();
            // Use chrono's built-in date construction to avoid edge cases
            let next_month_start = if now.month() == 12 {
                Utc.with_ymd_and_hms(now.year() + 1, 1, 1, 0, 0, 0)
            } else {
                Utc.with_ymd_and_hms(now.year(), now.month() + 1, 1, 0, 0, 0)
            };

            let reset_date = next_month_start.single().ok_or_else(|| {
                AppError::internal("Failed to create valid date for next month start")
            })?;

            overview.push(RateLimitOverview {
                api_key_id: api_key.id,
                api_key_name: api_key.name,
                tier: format!("{:?}", api_key.tier).to_lowercase(),
                current_usage: current_usage.into(),
                limit,
                usage_percentage,
                reset_date: Some(reset_date),
            });
        }

        Ok(overview)
    }

    /// Get recent activity for user
    async fn get_recent_activity(
        &self,
        user_id: Uuid,
        limit: u32,
    ) -> AppResult<Vec<RecentActivity>> {
        let api_keys = self.resources.database.get_user_api_keys(user_id).await?;
        let mut recent_activity = Vec::new();

        // Get recent usage for all user's API keys
        for api_key in api_keys {
            let start_time = Utc::now() - Duration::days(7); // Last 7 days
            let logs = self
                .resources
                .database
                .get_request_logs(
                    Some(&api_key.id),
                    Some(start_time),
                    Some(Utc::now()),
                    None,
                    None,
                )
                .await?;

            for log in logs.into_iter().take(limit as usize) {
                recent_activity.push(RecentActivity {
                    timestamp: log.timestamp,
                    api_key_name: log.api_key_name,
                    tool_name: log.tool_name,
                    status_code: log.status_code,
                    response_time_ms: log.response_time_ms,
                });
            }
        }

        // Sort by timestamp (newest first) and limit
        recent_activity.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        recent_activity.truncate(limit as usize);

        Ok(recent_activity)
    }

    /// Get top tools analysis
    async fn get_top_tools_analysis(
        &self,
        user_id: Uuid,
        start_date: chrono::DateTime<Utc>,
        end_date: chrono::DateTime<Utc>,
    ) -> AppResult<Vec<ToolUsage>> {
        let api_keys = self.resources.database.get_user_api_keys(user_id).await?;
        let mut tool_stats: std::collections::HashMap<String, (u64, u64, u64)> =
            std::collections::HashMap::new();

        // Aggregate tool usage across all user's API keys
        for api_key in api_keys {
            let stats = self
                .resources
                .database
                .get_api_key_usage_stats(&api_key.id, start_date, end_date)
                .await?;

            // Extract tool usage from the JSON
            if let Some(tool_usage_obj) = stats.tool_usage.as_object() {
                for (tool_name, count_val) in tool_usage_obj {
                    if let Some(count) = count_val.as_u64() {
                        let entry = tool_stats.entry(tool_name.clone()).or_insert((0, 0, 0));
                        entry.0 += count; // total requests
                        entry.1 += u64::from(stats.successful_requests); // successful requests
                        entry.2 += stats.total_response_time_ms; // total response time
                    }
                }
            }
        }

        // Convert to ToolUsage structs
        let mut tool_usage: Vec<ToolUsage> = tool_stats
            .into_iter()
            .map(
                |(tool_name, (total_requests, successful_requests, total_response_time))| {
                    let success_rate = if total_requests > 0 {
                        (f64::from(u32::try_from(successful_requests).unwrap_or(u32::MAX))
                            / f64::from(u32::try_from(total_requests).unwrap_or(u32::MAX)))
                            * 100.0
                    } else {
                        0.0
                    };

                    let average_response_time = if total_requests > 0 {
                        f64::from(u32::try_from(total_response_time).unwrap_or(u32::MAX))
                            / f64::from(u32::try_from(total_requests).unwrap_or(u32::MAX))
                    } else {
                        0.0
                    };

                    ToolUsage {
                        tool_name,
                        request_count: total_requests,
                        success_rate,
                        average_response_time,
                    }
                },
            )
            .collect();

        // Sort by request count (descending) and take top 10
        tool_usage.sort_by(|a, b| b.request_count.cmp(&a.request_count));
        tool_usage.truncate(10);

        Ok(tool_usage)
    }

    /// Get request logs with filtering
    ///
    /// # Errors
    /// Returns an error if API key access is denied, or database queries fail
    pub async fn get_request_logs(
        &self,
        auth: AuthResult,
        api_key_id: Option<&str>,
        time_range: Option<&str>,
        status: Option<&str>,
        tool: Option<&str>,
    ) -> AppResult<Vec<RequestLog>> {
        tracing::debug!("Dashboard request logs request received");

        let user_id = auth.user_id;

        // Validate user_id is not nil
        if user_id.is_nil() {
            return Err(AppError::invalid_input("Invalid user ID"));
        }

        tracing::info!(
            "Dashboard request logs access granted for user: {}",
            user_id
        );

        // Get user's API keys to filter by
        let api_keys = self.resources.database.get_user_api_keys(user_id).await?;

        // If specific API key is requested, verify user owns it
        if let Some(key_id) = api_key_id {
            if !api_keys.iter().any(|k| k.id == key_id) {
                return Err(AppError::not_found("API key not found or access denied"));
            }
        }

        // Parse time range
        let start_time = match time_range {
            Some("24h") => Utc::now() - Duration::hours(24),
            Some("7d") => Utc::now() - Duration::days(7),
            Some("30d") => Utc::now() - Duration::days(30),
            _ => Utc::now() - Duration::hours(1), // Default to 1 hour (includes "1h")
        };

        // Query real data from the database
        let logs = self
            .resources
            .database
            .get_request_logs(api_key_id, Some(start_time), Some(Utc::now()), status, tool)
            .await?;

        Ok(logs)
    }

    /// Get request statistics
    ///
    /// # Errors
    /// Returns an error if API key access is denied, or database queries fail
    pub async fn get_request_stats(
        &self,
        auth: AuthResult,
        api_key_id: Option<&str>,
        time_range: Option<&str>,
    ) -> AppResult<RequestStats> {
        tracing::debug!("Dashboard request stats request received");

        let user_id = auth.user_id;

        // Validate user_id is not nil
        if user_id.is_nil() {
            return Err(AppError::invalid_input("Invalid user ID"));
        }

        tracing::info!(
            "Dashboard request stats access granted for user: {}",
            user_id
        );

        // Get user's API keys
        let api_keys = self.resources.database.get_user_api_keys(user_id).await?;

        // If specific API key is requested, verify user owns it
        if let Some(key_id) = api_key_id {
            if !api_keys.iter().any(|k| k.id == key_id) {
                return Err(AppError::not_found("API key not found or access denied"));
            }
        }

        // Parse time range
        let (start_time, duration_minutes) = match time_range {
            Some("24h") => (Utc::now() - Duration::hours(24), 1440.0),
            Some("7d") => (Utc::now() - Duration::days(7), 10080.0),
            Some("30d") => (Utc::now() - Duration::days(30), 43200.0),
            _ => (Utc::now() - Duration::hours(1), 60.0), // Default to 1 hour (includes "1h")
        };

        // Calculate stats from user's API keys
        let mut total_requests = 0u64;
        let mut successful_requests = 0u64;
        let mut failed_requests = 0u64;
        let mut total_response_time = 0u64;

        let keys_to_check = if let Some(key_id) = api_key_id {
            api_keys.into_iter().filter(|k| k.id == key_id).collect()
        } else {
            api_keys
        };

        for api_key in keys_to_check {
            let stats = self
                .resources
                .database
                .get_api_key_usage_stats(&api_key.id, start_time, Utc::now())
                .await?;

            total_requests += u64::from(stats.total_requests);
            successful_requests += u64::from(stats.successful_requests);
            failed_requests += u64::from(stats.failed_requests);
            total_response_time += stats.total_response_time_ms;
        }

        let average_response_time = if total_requests > 0 {
            f64::from(u32::try_from(total_response_time).unwrap_or(u32::MAX))
                / f64::from(u32::try_from(total_requests).unwrap_or(u32::MAX))
        } else {
            0.0
        };

        let requests_per_minute =
            f64::from(u32::try_from(total_requests).unwrap_or(u32::MAX)) / duration_minutes;

        let error_rate = if total_requests > 0 {
            (f64::from(u32::try_from(failed_requests).unwrap_or(u32::MAX))
                / f64::from(u32::try_from(total_requests).unwrap_or(u32::MAX)))
                * 100.0
        } else {
            0.0
        };

        Ok(RequestStats {
            total_requests,
            successful_requests,
            failed_requests,
            average_response_time,
            min_response_time: None, // Not available in current stats
            max_response_time: None, // Not available in current stats
            requests_per_minute,
            error_rate,
        })
    }

    /// Get tool usage breakdown for analytics
    ///
    /// # Errors
    /// Returns an error if authentication fails or database queries fail
    pub async fn get_tool_usage_breakdown(
        &self,
        auth: AuthResult,
        _api_key_id: Option<&str>,
        time_range: Option<&str>,
    ) -> AppResult<Vec<ToolUsage>> {
        tracing::debug!("Dashboard tool usage breakdown request received");

        let user_id = auth.user_id;

        // Validate user_id is not nil
        if user_id.is_nil() {
            return Err(AppError::invalid_input("Invalid user ID"));
        }

        tracing::info!(
            "Dashboard tool usage breakdown access granted for user: {}",
            user_id
        );

        // Parse time range
        let start_time = match time_range {
            Some("1h") => Utc::now() - Duration::hours(1),
            Some("24h") => Utc::now() - Duration::hours(24),
            Some("30d") => Utc::now() - Duration::days(30),
            _ => Utc::now() - Duration::days(7), // Default to 7 days (includes "7d")
        };

        // Get tool usage analysis
        self.get_top_tools_analysis(user_id, start_time, Utc::now())
            .await
    }
}
