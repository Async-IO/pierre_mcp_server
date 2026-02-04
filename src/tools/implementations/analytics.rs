// ABOUTME: Analytics tools for fitness data analysis and insights.
// ABOUTME: Uses intelligence module directly for clean, efficient analysis.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Analytics Tools
//!
//! This module provides tools for fitness analytics:
//! - `AnalyzeTrainingLoadTool` - Calculate CTL/ATL/TSB training metrics
//! - `DetectPatternsTool` - Detect training patterns and overtraining signs
//! - `CalculateFitnessScoreTool` - Calculate overall fitness score
//!
//! These tools use the intelligence module directly for efficient analysis.

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use tracing::info;

use crate::config::environment::default_provider;
use crate::errors::AppResult;
use crate::intelligence::{PatternDetector, RiskLevel, TrainingLoadCalculator, TrainingStatus};
use crate::mcp::schema::{JsonSchema, PropertySchema};
use crate::models::Activity;
use crate::protocols::universal::auth_service::AuthService;
use crate::providers::core::{ActivityQueryParams, FitnessProvider};
use crate::tools::context::ToolExecutionContext;
use crate::tools::result::ToolResult;
use crate::tools::traits::{McpTool, ToolCapabilities};

// ============================================================================
// Helper functions for provider creation and activity fetching
// ============================================================================

/// Create an authenticated provider from context
async fn create_provider(
    context: &ToolExecutionContext,
    provider_name: &str,
) -> Result<Box<dyn FitnessProvider>, ToolResult> {
    let auth_service = AuthService::new(context.resources.clone());
    let tenant_id = context.tenant_id.map(|id| id.to_string());

    auth_service
        .create_authenticated_provider(provider_name, context.user_id, tenant_id.as_deref())
        .await
        .map_err(|response| {
            ToolResult::error(json!({
                "error": response.error.unwrap_or_else(|| "Authentication failed".to_owned()),
                "provider": provider_name
            }))
        })
}

/// Fetch activities for a given time period
async fn fetch_activities(
    provider: &dyn FitnessProvider,
    after_timestamp: i64,
    limit: usize,
) -> Result<Vec<Activity>, String> {
    let query_params = ActivityQueryParams {
        limit: Some(limit),
        offset: None,
        before: None,
        after: Some(after_timestamp),
    };

    provider
        .get_activities_with_params(&query_params)
        .await
        .map_err(|e| format!("Failed to fetch activities: {e}"))
}

/// Build pattern detection JSON response
fn build_pattern_response(
    activities: &[Activity],
    weeks: i64,
    provider_name: &str,
) -> serde_json::Value {
    let hard_easy = PatternDetector::detect_hard_easy_pattern(activities);
    let weekly = PatternDetector::detect_weekly_schedule(activities);
    let volume = PatternDetector::detect_volume_progression(activities);
    let overtraining = PatternDetector::detect_overtraining_signals(activities);

    json!({
        "hard_easy_pattern": {
            "pattern_detected": hard_easy.pattern_detected,
            "description": hard_easy.pattern_description,
            "hard_percentage": hard_easy.hard_percentage,
            "easy_percentage": hard_easy.easy_percentage,
            "adequate_recovery": hard_easy.adequate_recovery,
            "recommendation": if hard_easy.adequate_recovery {
                "Good recovery balance between hard and easy days"
            } else if hard_easy.hard_percentage > 50.0 {
                "Too many hard days - add more easy/recovery days"
            } else {
                "Consider adding more intensity to some sessions"
            }
        },
        "weekly_schedule": {
            "most_common_days": weekly.most_common_days.iter()
                .map(|d| format!("{d:?}"))
                .collect::<Vec<_>>(),
            "day_frequencies": weekly.day_frequencies,
            "consistency_score": weekly.consistency_score,
            "avg_activities_per_week": weekly.avg_activities_per_week,
            "recommendation": if weekly.consistency_score > 70.0 {
                "Consistent training schedule - good for adaptation"
            } else {
                "Irregular schedule - try to establish a routine"
            }
        },
        "volume_progression": {
            "trend": format!("{:?}", volume.trend),
            "weekly_volumes_km": volume.weekly_volumes,
            "volume_spikes_detected": volume.volume_spikes_detected,
            "spike_weeks": volume.spike_weeks,
            "recommendation": volume.recommendation
        },
        "overtraining_signals": {
            "risk_level": format!("{:?}", overtraining.risk_level),
            "hr_drift_detected": overtraining.hr_drift_detected,
            "hr_drift_percent": overtraining.hr_drift_percent,
            "performance_decline": overtraining.performance_decline,
            "insufficient_recovery": overtraining.insufficient_recovery,
            "warnings": overtraining.warnings,
            "recommendation": match overtraining.risk_level {
                RiskLevel::Low => "Training load is manageable",
                RiskLevel::Moderate => "Monitor fatigue levels closely",
                RiskLevel::High => "Take rest - high risk of overtraining",
            }
        },
        "analysis_summary": {
            "weeks_analyzed": weeks,
            "activities_analyzed": activities.len(),
            "provider": provider_name
        }
    })
}

/// Calculate fitness score components and build response
fn build_fitness_score_response(activities: &[Activity], provider_name: &str) -> serde_json::Value {
    let weeks_active = 6.0_f64;
    #[allow(clippy::cast_precision_loss)]
    let avg_per_week = activities.len() as f64 / weeks_active;
    let consistency_score = (avg_per_week / 5.0 * 100.0).min(100.0);

    // Training load score (using CTL)
    let calculator = TrainingLoadCalculator::new();
    let load_score = calculator
        .calculate_ctl(activities, None, None, None, None, None)
        .map_or(0.0, |ctl| (ctl / 100.0 * 100.0).clamp(0.0, 100.0));

    // Volume score (duration_seconds is u64, acceptable precision loss for hour conversion)
    #[allow(clippy::cast_precision_loss)]
    let total_hours: f64 = activities
        .iter()
        .map(|a| a.duration_seconds() as f64 / 3600.0)
        .sum();
    let avg_hours_per_week = total_hours / weeks_active;
    let volume_score = (avg_hours_per_week / 10.0 * 100.0).min(100.0);

    // Balance score
    let hard_easy = PatternDetector::detect_hard_easy_pattern(activities);
    let balance_score = if hard_easy.adequate_recovery {
        100.0
    } else {
        60.0
    };

    // Calculate overall fitness score using fused multiply-add for accuracy
    let fitness_score = consistency_score
        .mul_add(
            0.25,
            load_score.mul_add(0.35, volume_score.mul_add(0.25, balance_score * 0.15)),
        )
        .round();

    // fitness_score is clamped to 0-100 range, safe for u32 match
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let fitness_level = match fitness_score as u32 {
        90..=100 => "Elite",
        75..=89 => "Excellent",
        60..=74 => "Good",
        40..=59 => "Moderate",
        20..=39 => "Developing",
        _ => "Beginner",
    };

    json!({
        "fitness_score": fitness_score,
        "fitness_level": fitness_level,
        "components": {
            "consistency": {
                "score": consistency_score.round(),
                "avg_sessions_per_week": (avg_per_week * 10.0).round() / 10.0,
                "weight": "25%",
                "description": "Based on training frequency"
            },
            "training_load": {
                "score": load_score.round(),
                "weight": "35%",
                "description": "Based on chronic training load (CTL)"
            },
            "volume": {
                "score": volume_score.round(),
                "avg_hours_per_week": (avg_hours_per_week * 10.0).round() / 10.0,
                "weight": "25%",
                "description": "Based on total training volume"
            },
            "balance": {
                "score": balance_score.round(),
                "adequate_recovery": hard_easy.adequate_recovery,
                "weight": "15%",
                "description": "Based on training intensity distribution"
            }
        },
        "analysis_period": {
            "weeks": 6,
            "activities_analyzed": activities.len()
        },
        "provider": provider_name
    })
}

// ============================================================================
// AnalyzeTrainingLoadTool - Calculate CTL/ATL/TSB
// ============================================================================

/// Tool for analyzing training load using CTL/ATL/TSB metrics.
pub struct AnalyzeTrainingLoadTool;

#[async_trait]
impl McpTool for AnalyzeTrainingLoadTool {
    fn name(&self) -> &'static str {
        "analyze_training_load"
    }

    fn description(&self) -> &'static str {
        "Analyze training load using CTL (chronic training load), ATL (acute training load), and TSB (training stress balance) metrics to assess fitness, fatigue, and form"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "provider".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Fitness provider to query (e.g., 'strava'). Defaults to configured provider."
                        .to_owned(),
                ),
            },
        );
        properties.insert(
            "days".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some(
                    "Number of days of history to analyze. Default: 42 (6 weeks).".to_owned(),
                ),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: None,
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, context: &ToolExecutionContext) -> AppResult<ToolResult> {
        let provider_name = args
            .get("provider")
            .and_then(Value::as_str)
            .map_or_else(default_provider, String::from);

        let days = args
            .get("days")
            .and_then(Value::as_i64)
            .unwrap_or(42)
            .min(180);

        let provider = match create_provider(context, &provider_name).await {
            Ok(p) => p,
            Err(result) => return Ok(result),
        };

        let after = Utc::now() - Duration::days(days);
        let activities = match fetch_activities(provider.as_ref(), after.timestamp(), 500).await {
            Ok(acts) => acts,
            Err(e) => {
                return Ok(ToolResult::error(json!({
                    "error": e,
                    "provider": provider_name
                })));
            }
        };

        if activities.is_empty() {
            return Ok(ToolResult::ok(json!({
                "message": "No activities found in the analysis period",
                "days_analyzed": days,
                "provider": provider_name
            })));
        }

        let calculator = TrainingLoadCalculator::new();
        let load =
            match calculator.calculate_training_load(&activities, None, None, None, None, None) {
                Ok(l) => l,
                Err(e) => {
                    return Ok(ToolResult::error(json!({
                        "error": format!("Failed to calculate training load: {e}"),
                        "provider": provider_name
                    })));
                }
            };

        let status = match load.tsb {
            tsb if tsb > 10.0 => TrainingStatus::Detraining,
            tsb if tsb > 0.0 => TrainingStatus::Fresh,
            tsb if tsb > -10.0 => TrainingStatus::Productive,
            _ => TrainingStatus::Overreaching,
        };

        info!(
            "Training load analysis: CTL={:.1}, ATL={:.1}, TSB={:.1}, Status={:?}",
            load.ctl, load.atl, load.tsb, status
        );

        Ok(ToolResult::ok(json!({
            "training_load": {
                "ctl": load.ctl,
                "atl": load.atl,
                "tsb": load.tsb,
                "ctl_description": "Chronic Training Load - your long-term fitness level",
                "atl_description": "Acute Training Load - your recent training stress/fatigue",
                "tsb_description": "Training Stress Balance - your current form (positive = fresh, negative = fatigued)"
            },
            "status": format!("{status:?}"),
            "status_description": match status {
                TrainingStatus::Fresh => "Well rested, ready for hard training or racing",
                TrainingStatus::Productive => "Good balance of fitness and fatigue, optimal training zone",
                TrainingStatus::Overreaching => "High fatigue, prioritize recovery",
                TrainingStatus::Detraining => "Very fresh but risk of fitness loss without training",
            },
            "analysis_period": {
                "days": days,
                "activities_analyzed": activities.len()
            },
            "provider": provider_name
        })))
    }
}

// ============================================================================
// DetectPatternsTool - Detect training patterns
// ============================================================================

/// Tool for detecting training patterns and potential issues.
pub struct DetectPatternsTool;

#[async_trait]
impl McpTool for DetectPatternsTool {
    fn name(&self) -> &'static str {
        "detect_patterns"
    }

    fn description(&self) -> &'static str {
        "Detect training patterns including hard/easy day balance, weekly schedule consistency, volume progression, and overtraining warning signs"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "provider".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Fitness provider to query. Defaults to configured provider.".to_owned(),
                ),
            },
        );
        properties.insert(
            "weeks".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some(
                    "Number of weeks to analyze for patterns. Default: 4.".to_owned(),
                ),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: None,
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, context: &ToolExecutionContext) -> AppResult<ToolResult> {
        let provider_name = args
            .get("provider")
            .and_then(Value::as_str)
            .map_or_else(default_provider, String::from);

        let weeks = args
            .get("weeks")
            .and_then(Value::as_i64)
            .unwrap_or(4)
            .min(12);

        let provider = match create_provider(context, &provider_name).await {
            Ok(p) => p,
            Err(result) => return Ok(result),
        };

        let after = Utc::now() - Duration::weeks(weeks);
        let activities = match fetch_activities(provider.as_ref(), after.timestamp(), 200).await {
            Ok(acts) => acts,
            Err(e) => {
                return Ok(ToolResult::error(json!({
                    "error": e,
                    "provider": provider_name
                })));
            }
        };

        if activities.len() < 3 {
            return Ok(ToolResult::ok(json!({
                "message": "Insufficient activities for pattern detection (need at least 3)",
                "weeks_analyzed": weeks,
                "activities_found": activities.len(),
                "provider": provider_name
            })));
        }

        info!(
            "Pattern detection: {} activities over {} weeks",
            activities.len(),
            weeks
        );

        Ok(ToolResult::ok(build_pattern_response(
            &activities,
            weeks,
            &provider_name,
        )))
    }
}

// ============================================================================
// CalculateFitnessScoreTool - Calculate overall fitness score
// ============================================================================

/// Tool for calculating an overall fitness score.
pub struct CalculateFitnessScoreTool;

#[async_trait]
impl McpTool for CalculateFitnessScoreTool {
    fn name(&self) -> &'static str {
        "calculate_fitness_score"
    }

    fn description(&self) -> &'static str {
        "Calculate an overall fitness score (0-100) based on training consistency, CTL, training volume, and recovery balance"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "provider".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Fitness provider to query. Defaults to configured provider.".to_owned(),
                ),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: None,
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, context: &ToolExecutionContext) -> AppResult<ToolResult> {
        let provider_name = args
            .get("provider")
            .and_then(Value::as_str)
            .map_or_else(default_provider, String::from);

        let provider = match create_provider(context, &provider_name).await {
            Ok(p) => p,
            Err(result) => return Ok(result),
        };

        let after = Utc::now() - Duration::weeks(6);
        let activities = match fetch_activities(provider.as_ref(), after.timestamp(), 200).await {
            Ok(acts) => acts,
            Err(e) => {
                return Ok(ToolResult::error(json!({
                    "error": e,
                    "provider": provider_name
                })));
            }
        };

        if activities.is_empty() {
            return Ok(ToolResult::ok(json!({
                "fitness_score": 0,
                "fitness_level": "No Data",
                "message": "No recent activities to calculate fitness score",
                "provider": provider_name
            })));
        }

        info!(
            "Fitness score calculated for user {} ({} activities)",
            context.user_id,
            activities.len()
        );

        Ok(ToolResult::ok(build_fitness_score_response(
            &activities,
            &provider_name,
        )))
    }
}

// ============================================================================
// Module exports
// ============================================================================

/// Create all analytics tools for registration
#[must_use]
pub fn create_analytics_tools() -> Vec<Box<dyn McpTool>> {
    vec![
        Box::new(AnalyzeTrainingLoadTool),
        Box::new(DetectPatternsTool),
        Box::new(CalculateFitnessScoreTool),
    ]
}
