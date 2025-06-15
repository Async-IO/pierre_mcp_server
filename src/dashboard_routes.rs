// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Dashboard routes for the API Key Management System frontend

use crate::auth::AuthManager;
use crate::database::Database;
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
        
        tracing::info!("Dashboard overview data access granted for user: {}", user_id);

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
        
        tracing::info!("Dashboard analytics data access granted for user: {} (timeframe: {} days)", user_id, days);

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
                total_response_time += stats.total_response_time_ms as u64;
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
        
        tracing::info!("Dashboard rate limit data access granted for user: {}", user_id);

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
                    tracing::warn!("Dashboard access denied for token validation failure: {}", e);
                    Err(e)
                }
            }
        } else {
            tracing::warn!("Dashboard access denied: Invalid authorization header format (expected 'Bearer ...')");
            Err(anyhow::anyhow!("Invalid authorization header format"))
        }
    }

    /// Get recent activity for user
    async fn get_recent_activity(
        &self,
        _user_id: Uuid,
        _limit: u32,
    ) -> Result<Vec<RecentActivity>> {
        // This would require a more complex query to join API keys with usage
        // For now, return empty vector - would need to enhance database layer
        Ok(Vec::new())
    }

    /// Get top tools analysis
    async fn get_top_tools_analysis(
        &self,
        _user_id: Uuid,
        _start_date: chrono::DateTime<Utc>,
        _end_date: chrono::DateTime<Utc>,
    ) -> Result<Vec<ToolUsage>> {
        // This would require enhanced database queries
        // For now, return empty vector - would need to enhance database layer
        Ok(Vec::new())
    }
}
