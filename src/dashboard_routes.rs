// ABOUTME: Dashboard web interface routes for user fitness data visualization
// ABOUTME: Provides HTTP endpoints for dashboard UI, charts, and interactive fitness analytics
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Dashboard routes for the API Key Management System frontend

use crate::auth::AuthManager;
use crate::database_plugins::{factory::Database, DatabaseProvider};
use anyhow::Result;
use chrono::{Datelike, Duration, Timelike, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct DashboardOverview {
    pub total_api_keys: u32,
    pub active_api_keys: u32,
    pub total_requests_today: u64,
    pub total_requests_this_month: u64,
    pub current_month_usage_by_tier: Vec<TierUsage>,
    pub recent_activity: Vec<RecentActivity>,
}

#[derive(Debug, Serialize)]
pub struct TierUsage {
    pub tier: String,
    pub key_count: u32,
    pub total_requests: u64,
    pub average_requests_per_key: f64,
}

#[derive(Debug, Serialize)]
pub struct RecentActivity {
    pub timestamp: chrono::DateTime<Utc>,
    pub api_key_name: String,
    pub tool_name: String,
    pub status_code: u16,
    pub response_time_ms: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct UsageAnalytics {
    pub time_series: Vec<UsageDataPoint>,
    pub top_tools: Vec<ToolUsage>,
    pub error_rate: f64,
    pub average_response_time: f64,
}

#[derive(Debug, Serialize)]
pub struct UsageDataPoint {
    pub timestamp: chrono::DateTime<Utc>,
    pub request_count: u64,
    pub error_count: u64,
    pub average_response_time: f64,
}

#[derive(Debug, Serialize)]
pub struct ToolUsage {
    pub tool_name: String,
    pub request_count: u64,
    pub success_rate: f64,
    pub average_response_time: f64,
}

#[derive(Debug, Serialize)]
pub struct RateLimitOverview {
    pub api_key_id: String,
    pub api_key_name: String,
    pub tier: String,
    pub current_usage: u64,
    pub limit: Option<u64>,
    pub usage_percentage: f64,
    pub reset_date: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct RequestLog {
    pub id: String,
    pub timestamp: chrono::DateTime<Utc>,
    pub api_key_id: String,
    pub api_key_name: String,
    pub tool_name: String,
    pub status_code: u16,
    pub response_time_ms: Option<u32>,
    pub error_message: Option<String>,
    pub request_size_bytes: Option<u32>,
    pub response_size_bytes: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct RequestStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time: f64,
    pub min_response_time: Option<u32>,
    pub max_response_time: Option<u32>,
    pub requests_per_minute: f64,
    pub error_rate: f64,
}

#[derive(Clone)]
pub struct DashboardRoutes {
    database: Database,
    auth_manager: AuthManager,
}

impl DashboardRoutes {
    pub fn new(database: Database, auth_manager: AuthManager) -> Self {
        Self {
            database,
            auth_manager,
        }
    }

    /// Get dashboard overview data
    pub async fn get_dashboard_overview(
        &self,
        auth_header: Option<&str>,
    ) -> Result<DashboardOverview> {
        tracing::debug!("Dashboard overview request received");

        // Authenticate user
        let claims = self.validate_auth_header(auth_header)?;
        let user_id = Uuid::parse_str(&claims.sub)?;

        tracing::info!(
            "Dashboard overview data access granted for user: {}",
            user_id
        );

        // Get user's API keys
        let api_keys = self.database.get_user_api_keys(user_id).await?;
        let total_api_keys = api_keys.len() as u32;
        let active_api_keys = api_keys.iter().filter(|k| k.is_active).count() as u32;

        // Calculate time ranges
        let today_start = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        let month_start = Utc::now()
            .date_naive()
            .with_day(1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();

        // Get usage statistics
        let mut total_requests_today = 0u64;
        let mut total_requests_this_month = 0u64;

        for api_key in &api_keys {
            // Today's usage
            let today_stats = self
                .database
                .get_api_key_usage_stats(&api_key.id, today_start, Utc::now())
                .await?;
            total_requests_today += today_stats.total_requests as u64;

            // This month's usage
            let month_stats = self
                .database
                .get_api_key_usage_stats(&api_key.id, month_start, Utc::now())
                .await?;
            total_requests_this_month += month_stats.total_requests as u64;
        }

        // Group by tier
        let mut tier_map: std::collections::HashMap<String, (u32, u64)> =
            std::collections::HashMap::new();
        for api_key in &api_keys {
            let tier_name = format!("{:?}", api_key.tier).to_lowercase();
            let month_stats = self
                .database
                .get_api_key_usage_stats(&api_key.id, month_start, Utc::now())
                .await?;

            let entry = tier_map.entry(tier_name).or_insert((0, 0));
            entry.0 += 1; // key count
            entry.1 += month_stats.total_requests as u64; // total requests
        }

        let current_month_usage_by_tier: Vec<TierUsage> = tier_map
            .into_iter()
            .map(|(tier, (key_count, total_requests))| TierUsage {
                tier,
                key_count,
                total_requests,
                average_requests_per_key: if key_count > 0 {
                    total_requests as f64 / key_count as f64
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
    pub async fn get_usage_analytics(
        &self,
        auth_header: Option<&str>,
        days: u32,
    ) -> Result<UsageAnalytics> {
        tracing::debug!("Dashboard analytics request received for {} days", days);

        let claims = self.validate_auth_header(auth_header)?;
        let user_id = Uuid::parse_str(&claims.sub)?;

        tracing::info!(
            "Dashboard analytics data access granted for user: {} (timeframe: {} days)",
            user_id,
            days
        );

        let api_keys = self.database.get_user_api_keys(user_id).await?;
        let start_date = Utc::now() - Duration::days(days as i64);

        // Time series data (daily aggregates)
        let mut time_series = Vec::new();
        for day in 0..days {
            let day_start = start_date + Duration::days(day as i64);
            let day_end = day_start + Duration::days(1);

            let mut total_requests = 0u64;
            let mut total_errors = 0u64;
            let mut total_response_time = 0u64;
            let mut response_count = 0u64;

            for api_key in &api_keys {
                let stats = self
                    .database
                    .get_api_key_usage_stats(&api_key.id, day_start, day_end)
                    .await?;

                total_requests += stats.total_requests as u64;
                total_errors += stats.failed_requests as u64;
                total_response_time += stats.total_response_time_ms;
                response_count += stats.total_requests as u64;
            }

            time_series.push(UsageDataPoint {
                timestamp: day_start,
                request_count: total_requests,
                error_count: total_errors,
                average_response_time: if response_count > 0 {
                    total_response_time as f64 / response_count as f64
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
            (total_errors as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };

        let average_response_time = if !time_series.is_empty() {
            time_series
                .iter()
                .map(|d| d.average_response_time)
                .sum::<f64>()
                / time_series.len() as f64
        } else {
            0.0
        };

        Ok(UsageAnalytics {
            time_series,
            top_tools,
            error_rate,
            average_response_time,
        })
    }

    /// Get rate limit overview for all user's API keys
    pub async fn get_rate_limit_overview(
        &self,
        auth_header: Option<&str>,
    ) -> Result<Vec<RateLimitOverview>> {
        tracing::debug!("Dashboard rate limit overview request received");

        let claims = self.validate_auth_header(auth_header)?;
        let user_id = Uuid::parse_str(&claims.sub)?;

        tracing::info!(
            "Dashboard rate limit data access granted for user: {}",
            user_id
        );

        let api_keys = self.database.get_user_api_keys(user_id).await?;
        let mut overview = Vec::new();

        for api_key in api_keys {
            let current_usage = self.database.get_api_key_current_usage(&api_key.id).await?;

            let limit = if api_key.tier == crate::api_keys::ApiKeyTier::Enterprise {
                None
            } else {
                Some(api_key.rate_limit_requests as u64)
            };

            let usage_percentage = if let Some(limit_val) = limit {
                if limit_val > 0 {
                    (current_usage as f64 / limit_val as f64) * 100.0
                } else {
                    0.0
                }
            } else {
                0.0 // Unlimited
            };

            // Calculate reset date (first day of next month)
            let now = Utc::now();
            let reset_date = if now.month() == 12 {
                now.with_year(now.year() + 1)
                    .unwrap()
                    .with_month(1)
                    .unwrap()
            } else {
                now.with_month(now.month() + 1).unwrap()
            }
            .with_day(1)
            .unwrap()
            .with_hour(0)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap();

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

    /// Validate authentication header and return claims
    fn validate_auth_header(&self, auth_header: Option<&str>) -> Result<crate::auth::Claims> {
        tracing::debug!("Dashboard endpoint authentication attempt");

        let auth_str = match auth_header {
            Some(header) => header,
            None => {
                tracing::warn!("Dashboard access denied: Missing authorization header");
                return Err(anyhow::anyhow!("Missing authorization header"));
            }
        };

        if let Some(token) = auth_str.strip_prefix("Bearer ") {
            tracing::debug!("Validating JWT token for dashboard access");
            match self.auth_manager.validate_token(token) {
                Ok(claims) => {
                    tracing::info!("Dashboard access granted for user: {}", claims.sub);
                    Ok(claims)
                }
                Err(e) => {
                    tracing::warn!(
                        "Dashboard access denied for token validation failure: {}",
                        e
                    );
                    Err(e)
                }
            }
        } else {
            tracing::warn!("Dashboard access denied: Invalid authorization header format (expected 'Bearer ...')");
            Err(anyhow::anyhow!("Invalid authorization header format"))
        }
    }

    /// Get recent activity for user
    async fn get_recent_activity(&self, user_id: Uuid, limit: u32) -> Result<Vec<RecentActivity>> {
        let api_keys = self.database.get_user_api_keys(user_id).await?;
        let mut recent_activity = Vec::new();

        // Get recent usage for all user's API keys
        for api_key in api_keys {
            let start_time = Utc::now() - Duration::days(7); // Last 7 days
            let logs = self
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
    ) -> Result<Vec<ToolUsage>> {
        let api_keys = self.database.get_user_api_keys(user_id).await?;
        let mut tool_stats: std::collections::HashMap<String, (u64, u64, u64)> =
            std::collections::HashMap::new();

        // Aggregate tool usage across all user's API keys
        for api_key in api_keys {
            let stats = self
                .database
                .get_api_key_usage_stats(&api_key.id, start_date, end_date)
                .await?;

            // Extract tool usage from the JSON
            if let Some(tool_usage_obj) = stats.tool_usage.as_object() {
                for (tool_name, count_val) in tool_usage_obj {
                    if let Some(count) = count_val.as_u64() {
                        let entry = tool_stats.entry(tool_name.clone()).or_insert((0, 0, 0));
                        entry.0 += count; // total requests
                        entry.1 += stats.successful_requests as u64; // successful requests
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
                        (successful_requests as f64 / total_requests as f64) * 100.0
                    } else {
                        0.0
                    };

                    let average_response_time = if total_requests > 0 {
                        total_response_time as f64 / total_requests as f64
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
    pub async fn get_request_logs(
        &self,
        auth_header: Option<&str>,
        api_key_id: Option<&str>,
        time_range: Option<&str>,
        status: Option<&str>,
        tool: Option<&str>,
    ) -> Result<Vec<RequestLog>> {
        tracing::debug!("Dashboard request logs request received");

        let claims = self.validate_auth_header(auth_header)?;
        let user_id = Uuid::parse_str(&claims.sub)?;

        tracing::info!(
            "Dashboard request logs access granted for user: {}",
            user_id
        );

        // Get user's API keys to filter by
        let api_keys = self.database.get_user_api_keys(user_id).await?;

        // If specific API key is requested, verify user owns it
        if let Some(key_id) = api_key_id {
            if !api_keys.iter().any(|k| k.id == key_id) {
                return Err(anyhow::anyhow!("API key not found or access denied"));
            }
        }

        // Parse time range
        let start_time = match time_range {
            Some("1h") => Utc::now() - Duration::hours(1),
            Some("24h") => Utc::now() - Duration::hours(24),
            Some("7d") => Utc::now() - Duration::days(7),
            Some("30d") => Utc::now() - Duration::days(30),
            _ => Utc::now() - Duration::hours(1), // Default to 1 hour
        };

        // Query real data from the database
        let logs = self
            .database
            .get_request_logs(api_key_id, Some(start_time), Some(Utc::now()), status, tool)
            .await?;

        Ok(logs)
    }

    /// Get request statistics
    pub async fn get_request_stats(
        &self,
        auth_header: Option<&str>,
        api_key_id: Option<&str>,
        time_range: Option<&str>,
    ) -> Result<RequestStats> {
        tracing::debug!("Dashboard request stats request received");

        let claims = self.validate_auth_header(auth_header)?;
        let user_id = Uuid::parse_str(&claims.sub)?;

        tracing::info!(
            "Dashboard request stats access granted for user: {}",
            user_id
        );

        // Get user's API keys
        let api_keys = self.database.get_user_api_keys(user_id).await?;

        // If specific API key is requested, verify user owns it
        if let Some(key_id) = api_key_id {
            if !api_keys.iter().any(|k| k.id == key_id) {
                return Err(anyhow::anyhow!("API key not found or access denied"));
            }
        }

        // Parse time range
        let (start_time, duration_minutes) = match time_range {
            Some("1h") => (Utc::now() - Duration::hours(1), 60.0),
            Some("24h") => (Utc::now() - Duration::hours(24), 1440.0),
            Some("7d") => (Utc::now() - Duration::days(7), 10080.0),
            Some("30d") => (Utc::now() - Duration::days(30), 43200.0),
            _ => (Utc::now() - Duration::hours(1), 60.0), // Default to 1 hour
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
                .database
                .get_api_key_usage_stats(&api_key.id, start_time, Utc::now())
                .await?;

            total_requests += stats.total_requests as u64;
            successful_requests += stats.successful_requests as u64;
            failed_requests += stats.failed_requests as u64;
            total_response_time += stats.total_response_time_ms;
        }

        let average_response_time = if total_requests > 0 {
            total_response_time as f64 / total_requests as f64
        } else {
            0.0
        };

        let requests_per_minute = total_requests as f64 / duration_minutes;

        let error_rate = if total_requests > 0 {
            (failed_requests as f64 / total_requests as f64) * 100.0
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
    pub async fn get_tool_usage_breakdown(
        &self,
        auth_header: Option<&str>,
        _api_key_id: Option<&str>,
        time_range: Option<&str>,
    ) -> Result<Vec<ToolUsage>> {
        tracing::debug!("Dashboard tool usage breakdown request received");

        let claims = self.validate_auth_header(auth_header)?;
        let user_id = Uuid::parse_str(&claims.sub)?;

        tracing::info!(
            "Dashboard tool usage breakdown access granted for user: {}",
            user_id
        );

        // Parse time range
        let start_time = match time_range {
            Some("1h") => Utc::now() - Duration::hours(1),
            Some("24h") => Utc::now() - Duration::hours(24),
            Some("7d") => Utc::now() - Duration::days(7),
            Some("30d") => Utc::now() - Duration::days(30),
            _ => Utc::now() - Duration::days(7), // Default to 7 days
        };

        // Get tool usage analysis
        self.get_top_tools_analysis(user_id, start_time, Utc::now())
            .await
    }
}
