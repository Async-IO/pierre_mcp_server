// ABOUTME: Sleep and recovery tools for rest optimization.
// ABOUTME: Implements analyze_sleep_quality, calculate_recovery_score, suggest_rest_day, track_sleep_trends, optimize_sleep_schedule.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Sleep and Recovery Tools
//!
//! This module provides tools for sleep and recovery analysis with direct business logic:
//! - `AnalyzeSleepQualityTool` - Analyze sleep patterns and generate quality scores
//! - `CalculateRecoveryScoreTool` - Calculate holistic recovery score
//! - `SuggestRestDayTool` - AI-powered rest day recommendation
//! - `TrackSleepTrendsTool` - Track sleep trends over time
//! - `OptimizeSleepScheduleTool` - Sleep schedule recommendations
//!
//! All tools use direct `SleepAnalyzer` and `RecoveryCalculator` access.

use std::cmp::Ordering;
use std::collections::HashMap;

use async_trait::async_trait;
use chrono::Utc;
use serde_json::{json, Value};
use tracing::warn;

use crate::config::intelligence::SleepRecoveryConfig;
use crate::config::IntelligenceConfig;
use crate::errors::{AppError, AppResult};
use crate::intelligence::algorithms::RecoveryAggregationAlgorithm;
use crate::intelligence::{RecoveryCalculator, SleepAnalyzer, SleepData, TrainingLoad};
use crate::mcp::schema::{JsonSchema, PropertySchema};
use crate::tools::context::ToolExecutionContext;
use crate::tools::result::ToolResult;
use crate::tools::traits::{McpTool, ToolCapabilities};

// ============================================================================
// Helper functions
// ============================================================================

/// Parse `SleepData` from JSON value
fn parse_sleep_data(value: &Value) -> AppResult<SleepData> {
    serde_json::from_value(value.clone())
        .map_err(|e| AppError::invalid_input(format!("Invalid sleep_data format: {e}")))
}

/// Parse sleep history from JSON array
fn parse_sleep_history(value: &Value) -> AppResult<Vec<SleepData>> {
    serde_json::from_value(value.clone())
        .map_err(|e| AppError::invalid_input(format!("Invalid sleep_history format: {e}")))
}

/// Parse HRV values from JSON array
fn parse_hrv_values(value: Option<&Value>) -> Vec<f64> {
    value
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_f64).collect())
        .unwrap_or_default()
}

/// Create default training load for TSB-only calculations when no activities available
const fn create_default_training_load() -> TrainingLoad {
    TrainingLoad {
        ctl: 0.0,
        atl: 0.0,
        tsb: 0.0,
        tss_history: vec![],
    }
}

/// Create training load from provided ctl/atl/tsb values
const fn create_training_load_from_values(ctl: f64, atl: f64, tsb: f64) -> TrainingLoad {
    TrainingLoad {
        ctl,
        atl,
        tsb,
        tss_history: vec![],
    }
}

/// Parse training load from optional JSON value
fn parse_training_load(args: &Value) -> TrainingLoad {
    args.get("training_load")
        .map_or_else(create_default_training_load, |tl_json| {
            let ctl = tl_json.get("ctl").and_then(Value::as_f64).unwrap_or(0.0);
            let atl = tl_json.get("atl").and_then(Value::as_f64).unwrap_or(0.0);
            let tsb = tl_json.get("tsb").and_then(Value::as_f64).unwrap_or(0.0);
            create_training_load_from_values(ctl, atl, tsb)
        })
}

// ============================================================================
// AnalyzeSleepQualityTool
// ============================================================================

/// Tool for analyzing sleep quality from sleep data.
pub struct AnalyzeSleepQualityTool;

#[async_trait]
impl McpTool for AnalyzeSleepQualityTool {
    fn name(&self) -> &'static str {
        "analyze_sleep_quality"
    }

    fn description(&self) -> &'static str {
        "Analyze sleep data to generate quality scores and insights"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "sleep_data".to_owned(),
            PropertySchema {
                property_type: "object".to_owned(),
                description: Some(
                    "Sleep data with fields: duration_hours, deep_sleep_hours, rem_sleep_hours, \
                     light_sleep_hours, awake_hours, efficiency_percent, hrv_rmssd_ms"
                        .to_owned(),
                ),
            },
        );
        properties.insert(
            "recent_hrv_values".to_owned(),
            PropertySchema {
                property_type: "array".to_owned(),
                description: Some("Array of recent HRV values for trend analysis".to_owned()),
            },
        );
        properties.insert(
            "baseline_hrv".to_owned(),
            PropertySchema {
                property_type: "number".to_owned(),
                description: Some("User's baseline HRV for comparison".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["sleep_data".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, _ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let sleep_data_json = args
            .get("sleep_data")
            .ok_or_else(|| AppError::invalid_input("sleep_data is required"))?;

        let sleep_data = parse_sleep_data(sleep_data_json)?;
        let config = &IntelligenceConfig::global().sleep_recovery;

        // Calculate sleep quality using intelligence module
        let sleep_quality = SleepAnalyzer::calculate_sleep_quality(&sleep_data, config)
            .map_err(|e| AppError::internal(format!("Sleep quality calculation failed: {e}")))?;

        // Analyze HRV if available
        let hrv_analysis = if let Some(rmssd) = sleep_data.hrv_rmssd_ms {
            let recent_hrv = parse_hrv_values(args.get("recent_hrv_values"));
            let baseline_hrv = args.get("baseline_hrv").and_then(Value::as_f64);

            Some(
                SleepAnalyzer::analyze_hrv_trends(rmssd, &recent_hrv, baseline_hrv, config)
                    .map_err(|e| AppError::internal(format!("HRV analysis failed: {e}")))?,
            )
        } else {
            None
        };

        Ok(ToolResult::ok(json!({
            "sleep_quality": {
                "overall_score": sleep_quality.overall_score,
                "category": format!("{:?}", sleep_quality.quality_category),
                "duration_score": sleep_quality.duration_score,
                "stage_quality_score": sleep_quality.stage_quality_score,
                "efficiency_score": sleep_quality.efficiency_score,
                "insights": sleep_quality.insights,
                "recommendations": sleep_quality.recommendations,
            },
            "hrv_analysis": hrv_analysis.map(|h| json!({
                "current_rmssd": h.current_rmssd,
                "baseline_deviation_percent": h.baseline_deviation_percent,
                "trend": format!("{:?}", h.trend),
                "recovery_status": format!("{:?}", h.recovery_status),
                "insights": h.insights,
            })),
            "analysis_date": sleep_data.date.to_rfc3339(),
            "provider_score": sleep_data.provider_score,
            "analyzed_at": Utc::now().to_rfc3339(),
        })))
    }
}

// ============================================================================
// CalculateRecoveryScoreTool
// ============================================================================

/// Tool for calculating holistic recovery score.
pub struct CalculateRecoveryScoreTool;

#[async_trait]
impl McpTool for CalculateRecoveryScoreTool {
    fn name(&self) -> &'static str {
        "calculate_recovery_score"
    }

    fn description(&self) -> &'static str {
        "Calculate holistic recovery score combining training stress, sleep, and HRV"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "sleep_data".to_owned(),
            PropertySchema {
                property_type: "object".to_owned(),
                description: Some("Sleep data for recovery calculation".to_owned()),
            },
        );
        properties.insert(
            "training_load".to_owned(),
            PropertySchema {
                property_type: "object".to_owned(),
                description: Some(
                    "Training load data with ctl, atl, tsb values (optional)".to_owned(),
                ),
            },
        );
        properties.insert(
            "recent_hrv_values".to_owned(),
            PropertySchema {
                property_type: "array".to_owned(),
                description: Some("Array of recent HRV values".to_owned()),
            },
        );
        properties.insert(
            "baseline_hrv".to_owned(),
            PropertySchema {
                property_type: "number".to_owned(),
                description: Some("User's baseline HRV".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["sleep_data".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        tracing::debug!(user_id = %ctx.user_id, "Calculating recovery score");

        let sleep_data_json = args
            .get("sleep_data")
            .ok_or_else(|| AppError::invalid_input("sleep_data is required"))?;

        let sleep_data = parse_sleep_data(sleep_data_json)?;
        let config = &IntelligenceConfig::global().sleep_recovery;

        // Calculate sleep quality
        let sleep_quality = SleepAnalyzer::calculate_sleep_quality(&sleep_data, config)
            .map_err(|e| AppError::internal(format!("Sleep quality calculation failed: {e}")))?;

        // Get or calculate training load
        let training_load = parse_training_load(&args);

        // Analyze HRV if available
        let hrv_analysis = if let Some(rmssd) = sleep_data.hrv_rmssd_ms {
            let recent_hrv = parse_hrv_values(args.get("recent_hrv_values"));
            let baseline_hrv = args.get("baseline_hrv").and_then(Value::as_f64);

            Some(
                SleepAnalyzer::analyze_hrv_trends(rmssd, &recent_hrv, baseline_hrv, config)
                    .map_err(|e| AppError::internal(format!("HRV analysis failed: {e}")))?,
            )
        } else {
            None
        };

        // Get recovery aggregation algorithm with config weights
        let algorithm = RecoveryAggregationAlgorithm::WeightedAverage {
            tsb_weight_full: config.recovery_scoring.tsb_weight_full,
            sleep_weight_full: config.recovery_scoring.sleep_weight_full,
            hrv_weight_full: config.recovery_scoring.hrv_weight_full,
            tsb_weight_no_hrv: config.recovery_scoring.tsb_weight_no_hrv,
            sleep_weight_no_hrv: config.recovery_scoring.sleep_weight_no_hrv,
        };

        // Calculate holistic recovery score
        let recovery_score = RecoveryCalculator::calculate_recovery_score(
            &training_load,
            &sleep_quality,
            hrv_analysis.as_ref(),
            config,
            &algorithm,
        )
        .map_err(|e| AppError::internal(format!("Recovery score calculation failed: {e}")))?;

        Ok(ToolResult::ok(json!({
            "recovery_score": {
                "overall_score": recovery_score.overall_score,
                "category": format!("{:?}", recovery_score.recovery_category),
                "training_readiness": format!("{:?}", recovery_score.training_readiness),
                "data_completeness": format!("{:?}", recovery_score.data_completeness),
                "recommendations": recovery_score.recommendations,
                "limitations": recovery_score.limitations,
            },
            "components": {
                "tsb_score": recovery_score.components.tsb_score,
                "sleep_score": recovery_score.components.sleep_score,
                "hrv_score": recovery_score.components.hrv_score,
                "components_available": recovery_score.components.components_available,
            },
            "training_load": {
                "ctl": training_load.ctl,
                "atl": training_load.atl,
                "tsb": training_load.tsb,
            },
            "sleep_quality_score": sleep_quality.overall_score,
            "hrv_status": hrv_analysis.as_ref().map(|h| format!("{:?}", h.recovery_status)),
            "calculated_at": Utc::now().to_rfc3339(),
        })))
    }
}

// ============================================================================
// SuggestRestDayTool
// ============================================================================

/// Tool for AI-powered rest day recommendation.
pub struct SuggestRestDayTool;

#[async_trait]
impl McpTool for SuggestRestDayTool {
    fn name(&self) -> &'static str {
        "suggest_rest_day"
    }

    fn description(&self) -> &'static str {
        "Get AI-powered recommendation on whether to rest or train"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "sleep_data".to_owned(),
            PropertySchema {
                property_type: "object".to_owned(),
                description: Some("Last night's sleep data".to_owned()),
            },
        );
        properties.insert(
            "training_load".to_owned(),
            PropertySchema {
                property_type: "object".to_owned(),
                description: Some("Training load data (ctl, atl, tsb)".to_owned()),
            },
        );
        properties.insert(
            "recent_hrv_values".to_owned(),
            PropertySchema {
                property_type: "array".to_owned(),
                description: Some("Recent HRV values for trend analysis".to_owned()),
            },
        );
        properties.insert(
            "baseline_hrv".to_owned(),
            PropertySchema {
                property_type: "number".to_owned(),
                description: Some("User's baseline HRV".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["sleep_data".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        tracing::debug!(user_id = %ctx.user_id, "Generating rest day recommendation");

        let sleep_data_json = args
            .get("sleep_data")
            .ok_or_else(|| AppError::invalid_input("sleep_data is required"))?;

        let sleep_data = parse_sleep_data(sleep_data_json)?;
        let config = &IntelligenceConfig::global().sleep_recovery;

        // Calculate sleep quality
        let sleep_quality = SleepAnalyzer::calculate_sleep_quality(&sleep_data, config)
            .map_err(|e| AppError::internal(format!("Sleep quality calculation failed: {e}")))?;

        // Get or create training load
        let training_load = parse_training_load(&args);

        // Analyze HRV if available
        let hrv_analysis = if let Some(rmssd) = sleep_data.hrv_rmssd_ms {
            let recent_hrv = parse_hrv_values(args.get("recent_hrv_values"));
            let baseline_hrv = args.get("baseline_hrv").and_then(Value::as_f64);

            Some(
                SleepAnalyzer::analyze_hrv_trends(rmssd, &recent_hrv, baseline_hrv, config)
                    .map_err(|e| AppError::internal(format!("HRV analysis failed: {e}")))?,
            )
        } else {
            None
        };

        // Calculate recovery score
        let algorithm = RecoveryAggregationAlgorithm::WeightedAverage {
            tsb_weight_full: config.recovery_scoring.tsb_weight_full,
            sleep_weight_full: config.recovery_scoring.sleep_weight_full,
            hrv_weight_full: config.recovery_scoring.hrv_weight_full,
            tsb_weight_no_hrv: config.recovery_scoring.tsb_weight_no_hrv,
            sleep_weight_no_hrv: config.recovery_scoring.sleep_weight_no_hrv,
        };

        let recovery_score = RecoveryCalculator::calculate_recovery_score(
            &training_load,
            &sleep_quality,
            hrv_analysis.as_ref(),
            config,
            &algorithm,
        )
        .map_err(|e| AppError::internal(format!("Recovery score calculation failed: {e}")))?;

        // Generate rest day recommendation
        let recommendation = RecoveryCalculator::recommend_rest_day(
            &recovery_score,
            &sleep_data,
            &training_load,
            config,
        )
        .map_err(|e| AppError::internal(format!("Rest day recommendation failed: {e}")))?;

        Ok(ToolResult::ok(json!({
            "recommendation": {
                "rest_recommended": recommendation.rest_recommended,
                "confidence": recommendation.confidence,
                "recovery_score": recommendation.recovery_score,
                "primary_reasons": recommendation.primary_reasons,
                "supporting_factors": recommendation.supporting_factors,
                "alternatives": recommendation.alternatives,
            },
            "recovery_summary": {
                "overall_score": recovery_score.overall_score,
                "category": format!("{:?}", recovery_score.recovery_category),
                "training_readiness": format!("{:?}", recovery_score.training_readiness),
                "data_completeness": format!("{:?}", recovery_score.data_completeness),
            },
            "key_factors": {
                "tsb": training_load.tsb,
                "sleep_score": sleep_quality.overall_score,
                "sleep_hours": sleep_data.duration_hours,
                "hrv_status": hrv_analysis.as_ref().map(|h| format!("{:?}", h.recovery_status)),
            },
            "recommended_at": Utc::now().to_rfc3339(),
        })))
    }
}

// ============================================================================
// Sleep Trends Helpers
// ============================================================================

/// Sleep averages calculated from history
struct SleepAverages {
    duration: f64,
    efficiency: f64,
    efficiency_count: usize,
}

/// Calculate sleep averages from history
fn calculate_sleep_averages(sleep_history: &[SleepData]) -> SleepAverages {
    #[allow(clippy::cast_precision_loss)]
    let duration =
        sleep_history.iter().map(|s| s.duration_hours).sum::<f64>() / sleep_history.len() as f64;

    let efficiency_count = sleep_history
        .iter()
        .filter(|s| s.efficiency_percent.is_some())
        .count();

    #[allow(clippy::cast_precision_loss)]
    let efficiency = if efficiency_count > 0 {
        sleep_history
            .iter()
            .filter_map(|s| s.efficiency_percent)
            .sum::<f64>()
            / efficiency_count as f64
    } else {
        0.0
    };

    SleepAverages {
        duration,
        efficiency,
        efficiency_count,
    }
}

/// Determine sleep quality trend from recent vs previous scores
fn detect_trend(
    recent_avg: f64,
    previous_avg: f64,
    improving_threshold: f64,
    declining_threshold: f64,
) -> &'static str {
    if recent_avg > previous_avg + improving_threshold {
        "improving"
    } else if recent_avg < previous_avg - declining_threshold {
        "declining"
    } else {
        "stable"
    }
}

/// Calculate trend averages from quality scores
fn calculate_trend_averages(
    quality_scores: &[(chrono::DateTime<Utc>, f64)],
    min_days: usize,
) -> (f64, f64) {
    let recent_n_days = &quality_scores[quality_scores.len().saturating_sub(min_days)..];
    let previous_n_days = if quality_scores.len() >= min_days * 2 {
        &quality_scores[quality_scores.len().saturating_sub(min_days * 2)
            ..quality_scores.len().saturating_sub(min_days)]
    } else {
        recent_n_days
    };

    #[allow(clippy::cast_precision_loss)]
    let recent_avg = recent_n_days.iter().map(|(_, score)| score).sum::<f64>()
        / recent_n_days.len().max(1) as f64;

    #[allow(clippy::cast_precision_loss)]
    let previous_avg = previous_n_days.iter().map(|(_, score)| score).sum::<f64>()
        / previous_n_days.len().max(1) as f64;

    (recent_avg, previous_avg)
}

/// Generate insights from sleep trend analysis
fn generate_trend_insights(
    averages: &SleepAverages,
    trend: &str,
    sleep_history: &[SleepData],
    config: &SleepRecoveryConfig,
) -> Vec<String> {
    let mut insights = Vec::new();
    insights.push(format!(
        "Average sleep duration: {:.1}h over {} days",
        averages.duration,
        sleep_history.len()
    ));
    if averages.efficiency_count > 0 {
        insights.push(format!(
            "Average sleep efficiency: {:.1}%",
            averages.efficiency
        ));
    }
    insights.push(format!("Sleep quality trend: {trend}"));

    let athlete_min_hours = config.sleep_duration.athlete_min_hours;
    if averages.duration < athlete_min_hours {
        insights.push(format!(
            "Sleep duration below athlete recommendation ({:.1}h < {athlete_min_hours:.1}h)",
            averages.duration
        ));
    }
    insights
}

// ============================================================================
// TrackSleepTrendsTool
// ============================================================================

/// Tool for tracking sleep trends over time.
pub struct TrackSleepTrendsTool;

#[async_trait]
impl McpTool for TrackSleepTrendsTool {
    fn name(&self) -> &'static str {
        "track_sleep_trends"
    }

    fn description(&self) -> &'static str {
        "Analyze sleep patterns over time to identify trends and insights"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "sleep_history".to_owned(),
            PropertySchema {
                property_type: "array".to_owned(),
                description: Some("Array of sleep data objects (minimum 7 days)".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["sleep_history".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let sleep_history_json = args
            .get("sleep_history")
            .ok_or_else(|| AppError::invalid_input("sleep_history is required"))?;

        let sleep_history = parse_sleep_history(sleep_history_json)?;
        let config = &IntelligenceConfig::global().sleep_recovery;
        let sleep_params = &ctx.resources.config.sleep_tool_params;

        if sleep_history.len() < sleep_params.trend_min_days {
            return Err(AppError::invalid_input(format!(
                "At least {} days of sleep data required for trend analysis",
                sleep_params.trend_min_days
            )));
        }

        let averages = calculate_sleep_averages(&sleep_history);

        // Calculate quality scores for each day
        let quality_scores: Vec<_> = sleep_history
            .iter()
            .filter_map(|sleep| {
                SleepAnalyzer::calculate_sleep_quality(sleep, config)
                    .ok()
                    .map(|q| (sleep.date, q.overall_score))
            })
            .collect();

        // Calculate trend averages
        let (recent_avg, previous_avg) =
            calculate_trend_averages(&quality_scores, sleep_params.trend_min_days);
        let trend = detect_trend(
            recent_avg,
            previous_avg,
            sleep_params.trend_improving_threshold,
            sleep_params.trend_declining_threshold,
        );

        // Identify best and worst nights
        let best_night = quality_scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
        let worst_night = quality_scores
            .iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));

        // Generate insights
        let insights = generate_trend_insights(&averages, trend, &sleep_history, config);

        Ok(ToolResult::ok(json!({
            "trends": {
                "average_duration_hours": (averages.duration * 10.0).round() / 10.0,
                "average_efficiency_percent": (averages.efficiency * 10.0).round() / 10.0,
                "quality_trend": trend,
                "recent_avg_score": (recent_avg * 10.0).round() / 10.0,
                "previous_avg_score": (previous_avg * 10.0).round() / 10.0,
            },
            "highlights": {
                "best_night": best_night.map(|(date, score)| json!({"date": date.to_rfc3339(), "score": score})),
                "worst_night": worst_night.map(|(date, score)| json!({"date": date.to_rfc3339(), "score": score})),
            },
            "insights": insights,
            "data_points": quality_scores.len(),
            "analyzed_at": Utc::now().to_rfc3339(),
        })))
    }
}

// ============================================================================
// OptimizeSleepScheduleTool
// ============================================================================

/// Tool for recommending optimal sleep schedule.
pub struct OptimizeSleepScheduleTool;

#[async_trait]
impl McpTool for OptimizeSleepScheduleTool {
    fn name(&self) -> &'static str {
        "optimize_sleep_schedule"
    }

    fn description(&self) -> &'static str {
        "Get personalized sleep schedule recommendations based on training and recovery needs"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "training_load".to_owned(),
            PropertySchema {
                property_type: "object".to_owned(),
                description: Some("Training load data (ctl, atl, tsb)".to_owned()),
            },
        );
        properties.insert(
            "upcoming_workout_intensity".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("low, moderate, or high".to_owned()),
            },
        );
        properties.insert(
            "typical_wake_time".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("Wake time in HH:MM format (default: 06:00)".to_owned()),
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

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let config = &IntelligenceConfig::global().sleep_recovery;

        // Get training load
        let training_load = parse_training_load(&args);

        let upcoming_workout_intensity = args
            .get("upcoming_workout_intensity")
            .and_then(Value::as_str)
            .unwrap_or("moderate");

        let wake_time = args
            .get("typical_wake_time")
            .and_then(Value::as_str)
            .unwrap_or("06:00");

        // Calculate recommended sleep duration
        let base_recommendation = config.sleep_duration.athlete_optimal_hours;
        let fatigued_tsb = config.training_stress_balance.fatigued_tsb;

        let sleep_params = &ctx.resources.config.sleep_tool_params;

        let recommended_hours = if training_load.tsb < fatigued_tsb {
            base_recommendation + sleep_params.fatigue_bonus_hours
        } else if training_load.atl > sleep_params.high_load_atl_threshold {
            base_recommendation + sleep_params.high_load_bonus_hours
        } else {
            base_recommendation
        };

        // Calculate bedtime
        let bedtime = calculate_bedtime(
            wake_time,
            recommended_hours,
            sleep_params.wind_down_minutes,
            sleep_params.minutes_per_day,
        );

        // Generate recommendations
        let mut recommendations = Vec::new();
        recommendations.push(format!(
            "Target {recommended_hours:.1} hours of sleep tonight"
        ));
        recommendations.push(format!("Recommended bedtime: {bedtime}"));

        if training_load.tsb < fatigued_tsb {
            recommendations.push(
                "Extra sleep needed due to accumulated training fatigue (negative TSB)".to_owned(),
            );
        }

        if upcoming_workout_intensity == "high" {
            recommendations.push(
                "High-intensity workout planned - prioritize sleep quality tonight".to_owned(),
            );
        }

        Ok(ToolResult::ok(json!({
            "recommendations": {
                "target_hours": recommended_hours,
                "recommended_bedtime": bedtime,
                "wake_time": wake_time,
            },
            "rationale": {
                "training_load": {
                    "tsb": training_load.tsb,
                    "atl": training_load.atl,
                    "ctl": training_load.ctl,
                },
                "upcoming_intensity": upcoming_workout_intensity,
                "base_recommendation_hours": base_recommendation,
            },
            "tips": recommendations,
            "calculated_at": Utc::now().to_rfc3339(),
        })))
    }
}

// ============================================================================
// Helper functions for sleep schedule
// ============================================================================

/// Parse hour component from wake time string
fn parse_hour(hour_str: &str) -> i64 {
    match hour_str.parse() {
        Ok(h) if (0..24).contains(&h) => h,
        Ok(h) => {
            warn!(hour = h, "Invalid hour value, using default 6");
            6
        }
        Err(e) => {
            warn!(
                hour_str = hour_str,
                error = %e,
                "Failed to parse hour, using default 6"
            );
            6
        }
    }
}

/// Parse minute component from wake time string
fn parse_minute(minute_str: &str) -> i64 {
    match minute_str.parse() {
        Ok(m) if (0..60).contains(&m) => m,
        Ok(m) => {
            warn!(minute = m, "Invalid minute value, using default 0");
            0
        }
        Err(e) => {
            warn!(
                minute_str = minute_str,
                error = %e,
                "Failed to parse minute, using default 0"
            );
            0
        }
    }
}

/// Calculate recommended bedtime based on wake time and target sleep hours
fn calculate_bedtime(
    wake_time: &str,
    target_hours: f64,
    wind_down_minutes: i64,
    minutes_per_day: i64,
) -> String {
    let parts: Vec<&str> = wake_time.split(':').collect();
    if parts.len() != 2 {
        warn!(
            wake_time = wake_time,
            "Invalid wake_time format (expected HH:MM), using default 06:00"
        );
        return "22:00".to_owned();
    }

    let wake_hour = parse_hour(parts[0]);
    let wake_minute = parse_minute(parts[1]);

    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    let total_minutes =
        (wake_hour * 60 + wake_minute) - ((target_hours * 60.0) as i64) - wind_down_minutes;

    let bedtime_minutes = if total_minutes < 0 {
        minutes_per_day + total_minutes
    } else {
        total_minutes
    };

    let bedtime_hour = bedtime_minutes / 60;
    let bedtime_min = bedtime_minutes % 60;

    format!("{bedtime_hour:02}:{bedtime_min:02}")
}

// ============================================================================
// Module exports
// ============================================================================

/// Create all sleep tools for registration
#[must_use]
pub fn create_sleep_tools() -> Vec<Box<dyn McpTool>> {
    vec![
        Box::new(AnalyzeSleepQualityTool),
        Box::new(CalculateRecoveryScoreTool),
        Box::new(SuggestRestDayTool),
        Box::new(TrackSleepTrendsTool),
        Box::new(OptimizeSleepScheduleTool),
    ]
}
