// ABOUTME: Sleep and recovery analysis tool handlers for MCP protocol
// ABOUTME: Implements 5 tools: analyze_sleep_quality, calculate_recovery_score, suggest_rest_day, track_sleep_trends, optimize_sleep_schedule
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::intelligence::algorithms::RecoveryAggregationAlgorithm;
use crate::intelligence::{RecoveryCalculator, SleepAnalyzer, SleepData, TrainingLoadCalculator};
use crate::models::Activity;
use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use crate::protocols::ProtocolError;
use chrono::Utc;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

/// Helper function to fetch activities from Strava provider
///
/// # Errors
/// Returns `UniversalResponse` with error if authentication fails or activities cannot be fetched
async fn fetch_strava_activities(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    user_uuid: Uuid,
    tenant_id: Option<&str>,
) -> Result<Vec<Activity>, UniversalResponse> {
    use crate::constants::oauth_providers;

    match executor
        .auth_service
        .get_valid_token(user_uuid, oauth_providers::STRAVA, tenant_id)
        .await
    {
        Ok(Some(token_data)) => {
            let provider = executor
                .resources
                .provider_registry
                .create_provider(oauth_providers::STRAVA)
                .map_err(|e| UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!("Failed to create provider: {e}")),
                    metadata: None,
                })?;

            let credentials = crate::providers::OAuth2Credentials {
                client_id: executor.resources.config.oauth.strava.client_id.clone().unwrap_or_default(),
                client_secret: executor.resources.config.oauth.strava.client_secret.clone().unwrap_or_default(),
                access_token: Some(token_data.access_token),
                refresh_token: Some(token_data.refresh_token),
                expires_at: Some(token_data.expires_at),
                scopes: crate::constants::oauth::STRAVA_DEFAULT_SCOPES
                    .split(',')
                    .map(str::to_string)
                    .collect(),
            };

            provider.set_credentials(credentials).await.map_err(|e| UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Failed to set credentials: {e}")),
                metadata: None,
            })?;

            provider.get_activities(Some(executor.resources.config.sleep_recovery.activity_limit as usize), None).await
                .map_err(|e| UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!("Failed to fetch activities: {e}")),
                    metadata: None,
                })
        }
        Ok(None) => Err(UniversalResponse {
            success: false,
            result: None,
            error: Some("No valid Strava token found. Please connect your Strava account using the connect_provider tool with provider='strava'.".to_string()),
            metadata: None,
        }),
        Err(e) => Err(UniversalResponse {
            success: false,
            result: None,
            error: Some(format!("Authentication error: {e}")),
            metadata: None,
        }),
    }
}

/// Handle `analyze_sleep_quality` tool - analyze sleep data from fitness providers
///
/// Analyzes sleep duration, stages (deep/REM/light), efficiency, and generates quality score.
///
/// # Errors
/// Returns `ProtocolError` if sleep data is missing or invalid
#[must_use]
pub fn handle_analyze_sleep_quality(
    _executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Extract sleep data from parameters
        let sleep_data_json = request.parameters.get("sleep_data").ok_or_else(|| {
            ProtocolError::InvalidRequest("sleep_data parameter is required".to_string())
        })?;

        // Parse sleep data
        let sleep_data: SleepData =
            serde_json::from_value(sleep_data_json.clone()).map_err(|e| {
                ProtocolError::InvalidRequest(format!("Invalid sleep_data format: {e}"))
            })?;

        // Get sleep/recovery config
        let config =
            &crate::config::intelligence_config::IntelligenceConfig::global().sleep_recovery;

        // Calculate sleep quality using foundation module
        let sleep_quality =
            SleepAnalyzer::calculate_sleep_quality(&sleep_data, config).map_err(|e| {
                ProtocolError::InternalError(format!("Sleep quality calculation failed: {e}"))
            })?;

        // Analyze HRV if available
        let hrv_analysis = if let Some(rmssd) = sleep_data.hrv_rmssd_ms {
            // Get recent HRV values if provided
            let recent_hrv = request
                .parameters
                .get("recent_hrv_values")
                .and_then(|v| {
                    serde_json::from_value::<Vec<f64>>(v.clone())
                        .inspect_err(|e| {
                            tracing::debug!(
                                error = %e,
                                "Failed to deserialize recent_hrv_values, using empty default"
                            );
                        })
                        .ok()
                })
                .unwrap_or_default();

            let baseline_hrv = request
                .parameters
                .get("baseline_hrv")
                .and_then(serde_json::Value::as_f64);

            Some(
                SleepAnalyzer::analyze_hrv_trends(rmssd, &recent_hrv, baseline_hrv, config)
                    .map_err(|e| {
                        ProtocolError::InternalError(format!("HRV analysis failed: {e}"))
                    })?,
            )
        } else {
            None
        };

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "sleep_quality": sleep_quality,
                "hrv_analysis": hrv_analysis,
                "analysis_date": sleep_data.date,
                "provider_score": sleep_data.provider_score,
            })),
            error: None,
            metadata: Some({
                let mut map = HashMap::new();
                map.insert(
                    "analysis_timestamp".into(),
                    serde_json::Value::String(Utc::now().to_rfc3339()),
                );
                map.insert(
                    "scientific_references".into(),
                    serde_json::Value::Array(vec![
                        serde_json::Value::String(
                            "Watson et al. (2015) - NSF Sleep Guidelines".into(),
                        ),
                        serde_json::Value::String(
                            "Hirshkowitz et al. (2015) - Sleep Duration Recommendations".into(),
                        ),
                    ]),
                );
                map
            }),
        })
    })
}

/// Handle `calculate_recovery_score` tool - holistic recovery assessment
///
/// Combines Training Stress Balance (TSB), sleep quality, and HRV into overall recovery score.
///
/// # Errors
/// Returns `ProtocolError` if required data is missing or calculation fails
#[must_use]
// Long function: Protocol handler inherently long due to async auth, param extraction, calculation, response formatting
#[allow(clippy::too_many_lines)]
pub fn handle_calculate_recovery_score(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use crate::utils::uuid::parse_user_id_for_protocol;

        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Get valid token and fetch activities from provider
        let activities = match fetch_strava_activities(
            executor,
            user_uuid,
            request.tenant_id.as_deref(),
        )
        .await
        {
            Ok(activities) => activities,
            Err(response) => return Ok(response),
        };

        // Get user configuration for physiological parameters
        let user_config = request
            .parameters
            .get("user_config")
            .and_then(|v| {
                serde_json::from_value::<HashMap<String, serde_json::Value>>(v.clone()).ok()
            })
            .unwrap_or_default();

        let ftp = user_config.get("ftp").and_then(serde_json::Value::as_f64);
        let lthr = user_config.get("lthr").and_then(serde_json::Value::as_f64);
        let max_hr = user_config
            .get("max_hr")
            .and_then(serde_json::Value::as_f64);
        let resting_hr = user_config
            .get("resting_hr")
            .and_then(serde_json::Value::as_f64);
        let weight_kg = user_config
            .get("weight_kg")
            .and_then(serde_json::Value::as_f64);

        // Calculate training load (TSB)
        let training_load_calculator = TrainingLoadCalculator::new();
        let training_load = training_load_calculator
            .calculate_training_load(&activities, ftp, lthr, max_hr, resting_hr, weight_kg)
            .map_err(|e| {
                ProtocolError::InternalError(format!("Training load calculation failed: {e}"))
            })?;

        // Get sleep quality data
        let sleep_data_json = request.parameters.get("sleep_data").ok_or_else(|| {
            ProtocolError::InvalidRequest("sleep_data parameter is required".to_string())
        })?;

        let sleep_data: SleepData =
            serde_json::from_value(sleep_data_json.clone()).map_err(|e| {
                ProtocolError::InvalidRequest(format!("Invalid sleep_data format: {e}"))
            })?;

        // Get sleep/recovery config
        let config =
            &crate::config::intelligence_config::IntelligenceConfig::global().sleep_recovery;

        let sleep_quality =
            SleepAnalyzer::calculate_sleep_quality(&sleep_data, config).map_err(|e| {
                ProtocolError::InternalError(format!("Sleep quality calculation failed: {e}"))
            })?;

        // Get HRV analysis if available
        let hrv_analysis = if let Some(rmssd) = sleep_data.hrv_rmssd_ms {
            let recent_hrv = request
                .parameters
                .get("recent_hrv_values")
                .and_then(|v| {
                    serde_json::from_value::<Vec<f64>>(v.clone())
                        .inspect_err(|e| {
                            tracing::debug!(
                                error = %e,
                                "Failed to deserialize recent_hrv_values, using empty default"
                            );
                        })
                        .ok()
                })
                .unwrap_or_default();

            let baseline_hrv = request
                .parameters
                .get("baseline_hrv")
                .and_then(serde_json::Value::as_f64);

            Some(
                SleepAnalyzer::analyze_hrv_trends(rmssd, &recent_hrv, baseline_hrv, config)
                    .map_err(|e| {
                        ProtocolError::InternalError(format!("HRV analysis failed: {e}"))
                    })?,
            )
        } else {
            None
        };

        // Get recovery aggregation algorithm (default to WeightedAverage with config weights)
        let algorithm = request
            .parameters
            .get("algorithm")
            .and_then(|v| serde_json::from_value::<RecoveryAggregationAlgorithm>(v.clone()).ok())
            .unwrap_or(RecoveryAggregationAlgorithm::WeightedAverage {
                tsb_weight_full: config.recovery_scoring.tsb_weight_full,
                sleep_weight_full: config.recovery_scoring.sleep_weight_full,
                hrv_weight_full: config.recovery_scoring.hrv_weight_full,
                tsb_weight_no_hrv: config.recovery_scoring.tsb_weight_no_hrv,
                sleep_weight_no_hrv: config.recovery_scoring.sleep_weight_no_hrv,
            });

        // Calculate holistic recovery score
        let recovery_score = RecoveryCalculator::calculate_recovery_score(
            &training_load,
            &sleep_quality,
            hrv_analysis.as_ref(),
            config,
            &algorithm,
        )
        .map_err(|e| {
            ProtocolError::InternalError(format!("Recovery score calculation failed: {e}"))
        })?;

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "recovery_score": recovery_score,
                "training_load": {
                    "ctl": training_load.ctl,
                    "atl": training_load.atl,
                    "tsb": training_load.tsb,
                },
                "sleep_quality_score": sleep_quality.overall_score,
                "hrv_status": hrv_analysis.as_ref().map(|h| &h.recovery_status),
            })),
            error: None,
            metadata: Some({
                let mut map = HashMap::new();
                map.insert(
                    "calculation_timestamp".into(),
                    serde_json::Value::String(Utc::now().to_rfc3339()),
                );
                map.insert(
                    "components_used".into(),
                    serde_json::Value::Number(serde_json::Number::from(
                        recovery_score.components.components_available,
                    )),
                );
                map
            }),
        })
    })
}

/// Handle `suggest_rest_day` tool - AI-powered rest day recommendation
///
/// Analyzes recovery score, training load, and sleep to recommend rest or training.
///
/// # Errors
/// Returns `ProtocolError` if required data is missing or analysis fails
#[must_use]
// Long function: Protocol handler inherently long due to async auth, param extraction, calculation, response formatting
#[allow(clippy::too_many_lines)]
pub fn handle_suggest_rest_day(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use crate::utils::uuid::parse_user_id_for_protocol;

        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Get recent activities from provider
        let activities = match fetch_strava_activities(
            executor,
            user_uuid,
            request.tenant_id.as_deref(),
        )
        .await
        {
            Ok(activities) => activities,
            Err(response) => return Ok(response),
        };

        // Get user configuration
        let user_config = request
            .parameters
            .get("user_config")
            .and_then(|v| {
                serde_json::from_value::<HashMap<String, serde_json::Value>>(v.clone()).ok()
            })
            .unwrap_or_default();

        let ftp = user_config.get("ftp").and_then(serde_json::Value::as_f64);
        let lthr = user_config.get("lthr").and_then(serde_json::Value::as_f64);
        let max_hr = user_config
            .get("max_hr")
            .and_then(serde_json::Value::as_f64);
        let resting_hr = user_config
            .get("resting_hr")
            .and_then(serde_json::Value::as_f64);
        let weight_kg = user_config
            .get("weight_kg")
            .and_then(serde_json::Value::as_f64);

        // Calculate training load
        let training_load_calculator = TrainingLoadCalculator::new();
        let training_load = training_load_calculator
            .calculate_training_load(&activities, ftp, lthr, max_hr, resting_hr, weight_kg)
            .map_err(|e| {
                ProtocolError::InternalError(format!("Training load calculation failed: {e}"))
            })?;

        // Get sleep data
        let sleep_data_json = request.parameters.get("sleep_data").ok_or_else(|| {
            ProtocolError::InvalidRequest("sleep_data parameter is required".to_string())
        })?;

        let sleep_data: SleepData =
            serde_json::from_value(sleep_data_json.clone()).map_err(|e| {
                ProtocolError::InvalidRequest(format!("Invalid sleep_data format: {e}"))
            })?;

        // Get sleep/recovery config
        let config =
            &crate::config::intelligence_config::IntelligenceConfig::global().sleep_recovery;

        let sleep_quality =
            SleepAnalyzer::calculate_sleep_quality(&sleep_data, config).map_err(|e| {
                ProtocolError::InternalError(format!("Sleep quality calculation failed: {e}"))
            })?;

        // HRV analysis
        let hrv_analysis = if let Some(rmssd) = sleep_data.hrv_rmssd_ms {
            let recent_hrv = request
                .parameters
                .get("recent_hrv_values")
                .and_then(|v| {
                    serde_json::from_value::<Vec<f64>>(v.clone())
                        .inspect_err(|e| {
                            tracing::debug!(
                                error = %e,
                                "Failed to deserialize recent_hrv_values, using empty default"
                            );
                        })
                        .ok()
                })
                .unwrap_or_default();

            let baseline_hrv = request
                .parameters
                .get("baseline_hrv")
                .and_then(serde_json::Value::as_f64);

            Some(
                SleepAnalyzer::analyze_hrv_trends(rmssd, &recent_hrv, baseline_hrv, config)
                    .map_err(|e| {
                        ProtocolError::InternalError(format!("HRV analysis failed: {e}"))
                    })?,
            )
        } else {
            None
        };

        // Get recovery aggregation algorithm (default to WeightedAverage with config weights)
        let algorithm = request
            .parameters
            .get("algorithm")
            .and_then(|v| serde_json::from_value::<RecoveryAggregationAlgorithm>(v.clone()).ok())
            .unwrap_or(RecoveryAggregationAlgorithm::WeightedAverage {
                tsb_weight_full: config.recovery_scoring.tsb_weight_full,
                sleep_weight_full: config.recovery_scoring.sleep_weight_full,
                hrv_weight_full: config.recovery_scoring.hrv_weight_full,
                tsb_weight_no_hrv: config.recovery_scoring.tsb_weight_no_hrv,
                sleep_weight_no_hrv: config.recovery_scoring.sleep_weight_no_hrv,
            });

        // Calculate recovery score
        let recovery_score = RecoveryCalculator::calculate_recovery_score(
            &training_load,
            &sleep_quality,
            hrv_analysis.as_ref(),
            config,
            &algorithm,
        )
        .map_err(|e| {
            ProtocolError::InternalError(format!("Recovery score calculation failed: {e}"))
        })?;

        // Generate rest day recommendation
        let recommendation = RecoveryCalculator::recommend_rest_day(
            &recovery_score,
            &sleep_data,
            &training_load,
            config,
        )
        .map_err(|e| {
            ProtocolError::InternalError(format!("Rest day recommendation failed: {e}"))
        })?;

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "recommendation": recommendation,
                "recovery_summary": {
                    "overall_score": recovery_score.overall_score,
                    "category": recovery_score.recovery_category,
                    "training_readiness": recovery_score.training_readiness,
                },
                "key_factors": {
                    "tsb": training_load.tsb,
                    "sleep_score": sleep_quality.overall_score,
                    "sleep_hours": sleep_data.duration_hours,
                    "hrv_status": hrv_analysis.as_ref().map(|h| &h.recovery_status),
                },
            })),
            error: None,
            metadata: Some({
                let mut map = HashMap::new();
                map.insert(
                    "recommendation_timestamp".into(),
                    serde_json::Value::String(Utc::now().to_rfc3339()),
                );
                // Only include confidence if it's a valid f64 value
                if let Some(confidence_number) =
                    serde_json::Number::from_f64(recommendation.confidence)
                {
                    map.insert(
                        "confidence_percent".into(),
                        serde_json::Value::Number(confidence_number),
                    );
                } else {
                    tracing::warn!(
                        confidence = recommendation.confidence,
                        "Invalid confidence value (NaN/Infinity), omitting from metadata"
                    );
                }
                map
            }),
        })
    })
}

/// Handle `track_sleep_trends` tool - analyze sleep patterns over time
///
/// Correlates sleep quality with performance and training load.
///
/// # Errors
/// Returns `ProtocolError` if data is insufficient or analysis fails
#[must_use]
// Long function: Protocol handler inherently long due to trend calculation, statistics aggregation, response formatting
#[allow(clippy::too_many_lines)]
pub fn handle_track_sleep_trends(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Get sleep history
        let sleep_history_json = request.parameters.get("sleep_history").ok_or_else(|| {
            ProtocolError::InvalidRequest("sleep_history parameter is required".to_string())
        })?;

        let sleep_history: Vec<SleepData> = serde_json::from_value(sleep_history_json.clone())
            .map_err(|e| {
                ProtocolError::InvalidRequest(format!("Invalid sleep_history format: {e}"))
            })?;

        let trend_min_days = executor.resources.config.sleep_recovery.trend_min_days;
        if sleep_history.len() < trend_min_days {
            return Err(ProtocolError::InvalidRequest(format!(
                "At least {trend_min_days} days of sleep data required for trend analysis"
            )));
        }

        // Calculate average sleep metrics
        #[allow(clippy::cast_precision_loss)]
        // Safe: sleep_history.len() validated >= 7, well within f64 precision
        let avg_duration = sleep_history.iter().map(|s| s.duration_hours).sum::<f64>()
            / sleep_history.len() as f64;

        #[allow(clippy::cast_precision_loss)]
        // Safe: count of sleep records with efficiency, small number, well within f64 precision
        let avg_efficiency = sleep_history
            .iter()
            .filter_map(|s| s.efficiency_percent)
            .sum::<f64>()
            / sleep_history
                .iter()
                .filter(|s| s.efficiency_percent.is_some())
                .count() as f64;

        // Get sleep/recovery config
        let config =
            &crate::config::intelligence_config::IntelligenceConfig::global().sleep_recovery;

        // Calculate quality scores for each day
        let mut quality_scores = Vec::new();
        for sleep in &sleep_history {
            if let Ok(quality) = SleepAnalyzer::calculate_sleep_quality(sleep, config) {
                quality_scores.push((sleep.date, quality.overall_score));
            }
        }

        // Detect trends
        let recent_n_days = &quality_scores[quality_scores.len().saturating_sub(trend_min_days)..];
        let previous_n_days = if quality_scores.len() >= trend_min_days * 2 {
            &quality_scores[quality_scores.len().saturating_sub(trend_min_days * 2)
                ..quality_scores.len().saturating_sub(trend_min_days)]
        } else {
            recent_n_days
        };

        #[allow(clippy::cast_precision_loss)]
        // Safe: len() is config trend_min_days, well within f64 precision
        let recent_avg = recent_n_days.iter().map(|(_, score)| score).sum::<f64>()
            / recent_n_days.len().max(1) as f64;
        #[allow(clippy::cast_precision_loss)]
        // Safe: len() is config trend_min_days, well within f64 precision
        let previous_avg = previous_n_days.iter().map(|(_, score)| score).sum::<f64>()
            / previous_n_days.len().max(1) as f64;

        let improving_threshold = executor
            .resources
            .config
            .sleep_recovery
            .trend_improving_threshold;
        let declining_threshold = executor
            .resources
            .config
            .sleep_recovery
            .trend_declining_threshold;
        let trend = if recent_avg > previous_avg + improving_threshold {
            "improving"
        } else if recent_avg < previous_avg - declining_threshold {
            "declining"
        } else {
            "stable"
        };

        // Identify best and worst nights
        let best_night = quality_scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let worst_night = quality_scores
            .iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Generate insights
        let mut insights = Vec::new();
        insights.push(format!(
            "Average sleep duration: {avg_duration:.1}h over {} days",
            sleep_history.len()
        ));
        insights.push(format!("Average sleep efficiency: {avg_efficiency:.1}%"));
        insights.push(format!("Sleep quality trend: {trend}"));

        let athlete_min_hours = config.sleep_duration.athlete_min_hours;
        if avg_duration < athlete_min_hours {
            insights.push(format!(
                "Sleep duration below athlete recommendation ({avg_duration:.1}h < {athlete_min_hours:.1}h)"
            ));
        }

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "trends": {
                    "average_duration_hours": avg_duration,
                    "average_efficiency_percent": avg_efficiency,
                    "quality_trend": trend,
                    "recent_7day_avg": recent_avg,
                    "previous_7day_avg": previous_avg,
                },
                "highlights": {
                    "best_night": best_night.map(|(date, score)| {
                        serde_json::json!({ "date": date, "score": score })
                    }),
                    "worst_night": worst_night.map(|(date, score)| {
                        serde_json::json!({ "date": date, "score": score })
                    }),
                },
                "insights": insights,
                "data_points": quality_scores.len(),
            })),
            error: None,
            metadata: Some({
                let mut map = HashMap::new();
                map.insert(
                    "analysis_timestamp".into(),
                    serde_json::Value::String(Utc::now().to_rfc3339()),
                );
                map.insert(
                    "days_analyzed".into(),
                    serde_json::Value::Number(serde_json::Number::from(sleep_history.len())),
                );
                map
            }),
        })
    })
}

/// Handle `optimize_sleep_schedule` tool - recommend optimal sleep based on training
///
/// Suggests sleep duration and timing based on upcoming workouts and recovery needs.
///
/// # Errors
/// Returns `ProtocolError` if required parameters are missing
#[must_use]
// Long function: Protocol handler inherently long due to async auth, param extraction, calculation, response formatting
#[allow(clippy::too_many_lines)]
pub fn handle_optimize_sleep_schedule(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use crate::utils::uuid::parse_user_id_for_protocol;

        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Get recent activities for training load from provider
        let activities = match fetch_strava_activities(
            executor,
            user_uuid,
            request.tenant_id.as_deref(),
        )
        .await
        {
            Ok(activities) => activities,
            Err(response) => return Ok(response),
        };

        // Get user configuration
        let user_config = request
            .parameters
            .get("user_config")
            .and_then(|v| {
                serde_json::from_value::<HashMap<String, serde_json::Value>>(v.clone()).ok()
            })
            .unwrap_or_default();

        let ftp = user_config.get("ftp").and_then(serde_json::Value::as_f64);
        let lthr = user_config.get("lthr").and_then(serde_json::Value::as_f64);
        let max_hr = user_config
            .get("max_hr")
            .and_then(serde_json::Value::as_f64);
        let resting_hr = user_config
            .get("resting_hr")
            .and_then(serde_json::Value::as_f64);
        let weight_kg = user_config
            .get("weight_kg")
            .and_then(serde_json::Value::as_f64);

        // Calculate training load
        let training_load_calculator = TrainingLoadCalculator::new();
        let training_load = training_load_calculator
            .calculate_training_load(&activities, ftp, lthr, max_hr, resting_hr, weight_kg)
            .map_err(|e| {
                ProtocolError::InternalError(format!("Training load calculation failed: {e}"))
            })?;

        // Get sleep/recovery config
        let config =
            &crate::config::intelligence_config::IntelligenceConfig::global().sleep_recovery;

        // Get upcoming workout info (optional)
        let upcoming_workout_intensity = request
            .parameters
            .get("upcoming_workout_intensity")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("moderate");

        // Calculate recommended sleep duration
        let base_recommendation = config.sleep_duration.athlete_optimal_hours;
        let fatigued_tsb = config.training_stress_balance.fatigued_tsb;

        let recommended_hours = if training_load.tsb < fatigued_tsb {
            // High fatigue: add extra sleep
            base_recommendation + executor.resources.config.sleep_recovery.fatigue_bonus_hours
        } else if training_load.atl
            > executor
                .resources
                .config
                .sleep_recovery
                .high_load_atl_threshold
        {
            // High acute load: prioritize recovery
            base_recommendation
                + executor
                    .resources
                    .config
                    .sleep_recovery
                    .high_load_bonus_hours
        } else if upcoming_workout_intensity == "high" {
            // Hard workout tomorrow: ensure quality sleep
            base_recommendation
        } else {
            // Normal conditions
            base_recommendation
        };

        // Calculate recommended sleep window
        let wake_time = request
            .parameters
            .get("typical_wake_time")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("06:00");

        let bedtime = calculate_bedtime(
            wake_time,
            recommended_hours,
            executor.resources.config.sleep_recovery.wind_down_minutes,
            executor.resources.config.sleep_recovery.minutes_per_day,
        );

        // Generate recommendations
        let mut recommendations = Vec::new();
        recommendations.push(format!(
            "Target {recommended_hours:.1} hours of sleep tonight"
        ));
        recommendations.push(format!("Recommended bedtime: {bedtime}"));

        if training_load.tsb < fatigued_tsb {
            recommendations.push(
                "Extra sleep needed due to accumulated training fatigue (negative TSB)".to_string(),
            );
        }

        if upcoming_workout_intensity == "high" {
            recommendations.push(
                "High-intensity workout planned - prioritize sleep quality tonight".to_string(),
            );
        }

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
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
                },
                "tips": recommendations,
            })),
            error: None,
            metadata: Some({
                let mut map = HashMap::new();
                map.insert(
                    "calculation_timestamp".into(),
                    serde_json::Value::String(Utc::now().to_rfc3339()),
                );
                map
            }),
        })
    })
}

/// Parse hour component from wake time string
fn parse_hour(hour_str: &str) -> i64 {
    match hour_str.parse() {
        Ok(h) if (0..24).contains(&h) => h,
        Ok(h) => {
            tracing::warn!(hour = h, "Invalid hour value, using default 6");
            6
        }
        Err(e) => {
            tracing::warn!(
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
            tracing::warn!(minute = m, "Invalid minute value, using default 0");
            0
        }
        Err(e) => {
            tracing::warn!(
                minute_str = minute_str,
                error = %e,
                "Failed to parse minute, using default 0"
            );
            0
        }
    }
}

/// Helper function to calculate recommended bedtime
// Cognitive complexity reduced by extracting parse_hour and parse_minute helper functions
fn calculate_bedtime(
    wake_time: &str,
    target_hours: f64,
    wind_down_minutes: i64,
    minutes_per_day: i64,
) -> String {
    // Parse wake time (format: "HH:MM")
    let parts: Vec<&str> = wake_time.split(':').collect();
    if parts.len() != 2 {
        tracing::warn!(
            wake_time = wake_time,
            "Invalid wake_time format (expected HH:MM), using default 06:00"
        );
        return "22:00".to_string(); // Default fallback
    }

    let wake_hour = parse_hour(parts[0]);
    let wake_minute = parse_minute(parts[1]);

    // Calculate bedtime (wake time - target hours - wind-down minutes)
    #[allow(clippy::cast_precision_loss)] // Safe: target_hours is sleep duration (7-9h), well within f64→i64 range
    #[allow(clippy::cast_possible_truncation)]
    // Safe: target_hours * 60.0 is sleep minutes (420-540), no truncation
    let total_minutes =
        (wake_hour * 60 + wake_minute) - ((target_hours * 60.0) as i64) - wind_down_minutes;
    let bedtime_minutes = if total_minutes < 0 {
        minutes_per_day + total_minutes // Wrap to previous day
    } else {
        total_minutes
    };

    let bedtime_hour = bedtime_minutes / 60;
    let bedtime_min = bedtime_minutes % 60;

    format!("{bedtime_hour:02}:{bedtime_min:02}")
}
