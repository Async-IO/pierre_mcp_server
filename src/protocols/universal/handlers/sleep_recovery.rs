// ABOUTME: Sleep and recovery analysis tool handlers for MCP protocol
// ABOUTME: Implements 5 tools: analyze_sleep_quality, calculate_recovery_score, suggest_rest_day, track_sleep_trends, optimize_sleep_schedule
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use std::cmp::Ordering;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use chrono::{Duration, Utc};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::config::environment::ServerConfig;
use crate::config::intelligence::IntelligenceConfig;
use crate::intelligence::algorithms::RecoveryAggregationAlgorithm;
use crate::intelligence::{RecoveryCalculator, SleepAnalyzer, SleepData, TrainingLoadCalculator};
use crate::models::{Activity, SleepSession, SleepStageType};
use crate::protocols::universal::{UniversalRequest, UniversalResponse, UniversalToolExecutor};
use crate::protocols::ProtocolError;
use crate::providers::OAuth2Credentials;
use crate::utils::uuid::parse_user_id_for_protocol;

use super::{apply_format_to_response, extract_output_format};

/// Get OAuth credentials for a specific provider from configuration
///
/// Returns (`client_id`, `client_secret`) for the requested provider, or None if not configured.
fn get_provider_oauth_config(
    config: &ServerConfig,
    provider_name: &str,
) -> Option<(String, String)> {
    let oauth_config = match provider_name {
        "strava" => &config.oauth.strava,
        "fitbit" => &config.oauth.fitbit,
        "garmin" => &config.oauth.garmin,
        "whoop" => &config.oauth.whoop,
        "terra" => &config.oauth.terra,
        _ => return None,
    };

    let client_id = oauth_config.client_id.clone()?;
    let client_secret = oauth_config.client_secret.clone()?;

    if client_id.is_empty() || client_secret.is_empty() {
        return None;
    }

    Some((client_id, client_secret))
}

/// Get default OAuth scopes for a provider
fn get_provider_default_scopes(provider_name: &str) -> Vec<String> {
    match provider_name {
        "strava" => "activity:read_all".split(',').map(str::to_owned).collect(),
        "fitbit" => vec![
            "activity".to_owned(),
            "profile".to_owned(),
            "sleep".to_owned(),
            "heartrate".to_owned(),
        ],
        "garmin" => vec![
            "activity:read".to_owned(),
            "sleep:read".to_owned(),
            "health:read".to_owned(),
        ],
        "whoop" => vec![
            "offline".to_owned(),
            "read:profile".to_owned(),
            "read:workout".to_owned(),
            "read:sleep".to_owned(),
            "read:recovery".to_owned(),
        ],
        "terra" => vec!["activity".to_owned(), "sleep".to_owned(), "body".to_owned()],
        _ => vec![],
    }
}

/// Provider-agnostic activity fetcher
///
/// Fetches activities from any supported fitness provider based on the provider name.
/// Uses dynamic credential lookup and provider instantiation.
///
/// # Arguments
/// * `executor` - The tool executor with access to auth service and provider registry
/// * `user_uuid` - The user's UUID for token lookup
/// * `tenant_id` - Optional tenant ID for multi-tenant deployments
/// * `provider_name` - Name of the provider to fetch from (e.g., "strava", "garmin", "fitbit")
///
/// # Errors
/// Returns `UniversalResponse` with error if authentication fails or activities cannot be fetched
async fn fetch_provider_activities(
    executor: &UniversalToolExecutor,
    user_uuid: Uuid,
    tenant_id: Option<&str>,
    provider_name: &str,
) -> Result<Vec<Activity>, UniversalResponse> {
    // Validate provider is supported
    if !executor
        .resources
        .provider_registry
        .is_supported(provider_name)
    {
        let supported = executor
            .resources
            .provider_registry
            .supported_providers()
            .join(", ");
        return Err(UniversalResponse {
            success: false,
            result: None,
            error: Some(format!(
                "Provider '{provider_name}' is not supported. Available providers: {supported}"
            )),
            metadata: None,
        });
    }

    // Get valid OAuth token for the provider
    match executor
        .auth_service
        .get_valid_token(user_uuid, provider_name, tenant_id)
        .await
    {
        Ok(Some(token_data)) => {
            let provider = executor
                .resources
                .provider_registry
                .create_provider(provider_name)
                .map_err(|e| UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!("Failed to create provider '{provider_name}': {e}")),
                    metadata: None,
                })?;

            // Get OAuth config for this provider
            let (client_id, client_secret) =
                get_provider_oauth_config(&executor.resources.config, provider_name).ok_or_else(
                    || UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!(
                            "OAuth configuration missing for provider '{provider_name}'"
                        )),
                        metadata: None,
                    },
                )?;

            let credentials = OAuth2Credentials {
                client_id,
                client_secret,
                access_token: Some(token_data.access_token),
                refresh_token: Some(token_data.refresh_token),
                expires_at: Some(token_data.expires_at),
                scopes: get_provider_default_scopes(provider_name),
            };

            provider
                .set_credentials(credentials)
                .await
                .map_err(|e| UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!("Failed to set credentials for '{provider_name}': {e}")),
                    metadata: None,
                })?;

            #[allow(clippy::cast_possible_truncation)]
            provider
                .get_activities(
                    Some(executor.resources.config.sleep_tool_params.activity_limit as usize),
                    None,
                )
                .await
                .map_err(|e| UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!(
                        "Failed to fetch activities from '{provider_name}': {e}"
                    )),
                    metadata: None,
                })
        }
        Ok(None) => Err(UniversalResponse {
            success: false,
            result: None,
            error: Some(format!(
                "No valid {provider_name} token found. Please connect your {provider_name} account using the connect_provider tool with provider='{provider_name}'."
            )),
            metadata: None,
        }),
        Err(e) => Err(UniversalResponse {
            success: false,
            result: None,
            error: Some(format!("Authentication error for '{provider_name}': {e}")),
            metadata: None,
        }),
    }
}

/// Provider-agnostic sleep data fetcher
///
/// Fetches sleep data from any provider that supports sleep tracking (Fitbit, Garmin, WHOOP, Terra).
/// Automatically converts provider-specific `SleepSession` to the unified `SleepData` format.
///
/// # Arguments
/// * `executor` - The tool executor with access to auth service and provider registry
/// * `user_uuid` - The user's UUID for token lookup
/// * `tenant_id` - Optional tenant ID for multi-tenant deployments
/// * `provider_name` - Name of the sleep-capable provider
/// * `days_back` - Number of days of sleep data to fetch (default: 1 for most recent night)
///
/// # Errors
/// Returns `UniversalResponse` with error if provider doesn't support sleep or fetch fails
// Long function: Provider fetcher requires validation, auth, credential setup, API call, and conversion
#[allow(clippy::too_many_lines)]
pub async fn fetch_provider_sleep_data(
    executor: &UniversalToolExecutor,
    user_uuid: Uuid,
    tenant_id: Option<&str>,
    provider_name: &str,
    days_back: u32,
) -> Result<SleepData, UniversalResponse> {
    // Check if provider supports sleep tracking
    let capabilities = executor
        .resources
        .provider_registry
        .get_capabilities(provider_name)
        .ok_or_else(|| UniversalResponse {
            success: false,
            result: None,
            error: Some(format!("Provider '{provider_name}' not found in registry")),
            metadata: None,
        })?;

    if !capabilities.supports_sleep() {
        return Err(UniversalResponse {
            success: false,
            result: None,
            error: Some(format!(
                "Provider '{provider_name}' does not support sleep tracking. \
                 Use a sleep-capable provider: fitbit, garmin, whoop, or terra."
            )),
            metadata: None,
        });
    }

    // Get valid OAuth token for the provider
    match executor
        .auth_service
        .get_valid_token(user_uuid, provider_name, tenant_id)
        .await
    {
        Ok(Some(token_data)) => {
            let provider = executor
                .resources
                .provider_registry
                .create_provider(provider_name)
                .map_err(|e| UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!("Failed to create provider '{provider_name}': {e}")),
                    metadata: None,
                })?;

            // Get OAuth config for this provider
            let (client_id, client_secret) =
                get_provider_oauth_config(&executor.resources.config, provider_name).ok_or_else(
                    || UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!(
                            "OAuth configuration missing for provider '{provider_name}'"
                        )),
                        metadata: None,
                    },
                )?;

            let credentials = OAuth2Credentials {
                client_id,
                client_secret,
                access_token: Some(token_data.access_token),
                refresh_token: Some(token_data.refresh_token),
                expires_at: Some(token_data.expires_at),
                scopes: get_provider_default_scopes(provider_name),
            };

            provider
                .set_credentials(credentials)
                .await
                .map_err(|e| UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!("Failed to set credentials for '{provider_name}': {e}")),
                    metadata: None,
                })?;

            // Fetch sleep sessions for the requested date range
            let end_date = Utc::now();
            let start_date = end_date - Duration::days(i64::from(days_back));

            let sessions = provider
                .get_sleep_sessions(start_date, end_date)
                .await
                .map_err(|e| UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!(
                        "Failed to fetch sleep data from '{provider_name}': {e}"
                    )),
                    metadata: None,
                })?;

            // Get most recent session and convert to SleepData
            let session = sessions.into_iter().next().ok_or_else(|| UniversalResponse {
                success: false,
                result: None,
                error: Some(format!(
                    "No sleep data available from '{provider_name}' for the last {days_back} day(s)"
                )),
                metadata: None,
            })?;

            Ok(convert_sleep_session_to_data(&session))
        }
        Ok(None) => Err(UniversalResponse {
            success: false,
            result: None,
            error: Some(format!(
                "No valid {provider_name} token found. Please connect your {provider_name} account using the connect_provider tool with provider='{provider_name}'."
            )),
            metadata: None,
        }),
        Err(e) => Err(UniversalResponse {
            success: false,
            result: None,
            error: Some(format!("Authentication error for '{provider_name}': {e}")),
            metadata: None,
        }),
    }
}

/// Convert a provider `SleepSession` to the intelligence layer `SleepData` format
fn convert_sleep_session_to_data(session: &SleepSession) -> SleepData {
    // Calculate stage durations from sleep stages
    let mut deep_minutes: u32 = 0;
    let mut rem_minutes: u32 = 0;
    let mut light_minutes: u32 = 0;
    let mut awake_minutes: u32 = 0;

    for stage in &session.stages {
        match stage.stage_type {
            SleepStageType::Deep => deep_minutes += stage.duration_minutes,
            SleepStageType::Rem => rem_minutes += stage.duration_minutes,
            SleepStageType::Light => light_minutes += stage.duration_minutes,
            SleepStageType::Awake => awake_minutes += stage.duration_minutes,
        }
    }

    // Convert minutes to hours
    let minutes_to_hours = |m: u32| -> Option<f64> {
        if m > 0 {
            Some(f64::from(m) / 60.0)
        } else {
            None
        }
    };

    SleepData {
        date: session.start_time,
        duration_hours: f64::from(session.total_sleep_time) / 60.0,
        deep_sleep_hours: minutes_to_hours(deep_minutes),
        rem_sleep_hours: minutes_to_hours(rem_minutes),
        light_sleep_hours: minutes_to_hours(light_minutes),
        awake_hours: minutes_to_hours(awake_minutes),
        efficiency_percent: Some(f64::from(session.sleep_efficiency)),
        hrv_rmssd_ms: session.hrv_during_sleep,
        resting_hr_bpm: None, // SleepSession doesn't include this directly
        provider_score: session.sleep_score.map(f64::from),
    }
}

/// Fetch sleep history from a provider for trend analysis
///
/// Returns multiple sleep sessions converted to `SleepData` for trend tracking.
async fn fetch_provider_sleep_history(
    executor: &UniversalToolExecutor,
    user_uuid: Uuid,
    tenant_id: Option<&str>,
    provider_name: &str,
    days: u32,
) -> Result<Vec<SleepData>, UniversalResponse> {
    // Check if provider supports sleep tracking
    let capabilities = executor
        .resources
        .provider_registry
        .get_capabilities(provider_name)
        .ok_or_else(|| UniversalResponse {
            success: false,
            result: None,
            error: Some(format!("Provider '{provider_name}' not found in registry")),
            metadata: None,
        })?;

    if !capabilities.supports_sleep() {
        return Err(UniversalResponse {
            success: false,
            result: None,
            error: Some(format!(
                "Provider '{provider_name}' does not support sleep tracking."
            )),
            metadata: None,
        });
    }

    // Get valid OAuth token for the provider
    match executor
        .auth_service
        .get_valid_token(user_uuid, provider_name, tenant_id)
        .await
    {
        Ok(Some(token_data)) => {
            let provider = executor
                .resources
                .provider_registry
                .create_provider(provider_name)
                .map_err(|e| UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!("Failed to create provider '{provider_name}': {e}")),
                    metadata: None,
                })?;

            let (client_id, client_secret) =
                get_provider_oauth_config(&executor.resources.config, provider_name).ok_or_else(
                    || UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!(
                            "OAuth configuration missing for provider '{provider_name}'"
                        )),
                        metadata: None,
                    },
                )?;

            let credentials = OAuth2Credentials {
                client_id,
                client_secret,
                access_token: Some(token_data.access_token),
                refresh_token: Some(token_data.refresh_token),
                expires_at: Some(token_data.expires_at),
                scopes: get_provider_default_scopes(provider_name),
            };

            provider
                .set_credentials(credentials)
                .await
                .map_err(|e| UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!("Failed to set credentials: {e}")),
                    metadata: None,
                })?;

            let end_date = Utc::now();
            let start_date = end_date - Duration::days(i64::from(days));

            let sessions = provider
                .get_sleep_sessions(start_date, end_date)
                .await
                .map_err(|e| UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!("Failed to fetch sleep history: {e}")),
                    metadata: None,
                })?;

            Ok(sessions.iter().map(convert_sleep_session_to_data).collect())
        }
        Ok(None) => Err(UniversalResponse {
            success: false,
            result: None,
            error: Some(format!("No valid {provider_name} token found.")),
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

/// Select the best available activity provider for a user
///
/// Checks connected providers and returns the first one that supports activities.
/// Priority order: strava > garmin > fitbit > whoop > terra
async fn select_activity_provider(
    executor: &UniversalToolExecutor,
    user_uuid: Uuid,
    tenant_id: Option<&str>,
) -> Option<String> {
    // Activity provider priority (Strava is best for activities)
    let priority = ["strava", "garmin", "fitbit", "whoop", "terra"];

    for provider in priority {
        if let Some(caps) = executor
            .resources
            .provider_registry
            .get_capabilities(provider)
        {
            if caps.supports_activities() {
                // Check if user has a valid token
                if matches!(
                    executor
                        .auth_service
                        .get_valid_token(user_uuid, provider, tenant_id)
                        .await,
                    Ok(Some(_))
                ) {
                    return Some(provider.to_owned());
                }
            }
        }
    }
    None
}

/// Select the best available sleep provider for a user
///
/// Checks connected providers and returns the first one that supports sleep tracking.
/// Priority order: whoop > garmin > fitbit > terra (Strava excluded - no sleep support)
pub async fn select_sleep_provider(
    executor: &UniversalToolExecutor,
    user_uuid: Uuid,
    tenant_id: Option<&str>,
) -> Option<String> {
    // Sleep provider priority (WHOOP is best for recovery/sleep metrics)
    let priority = ["whoop", "garmin", "fitbit", "terra"];

    for provider in priority {
        if let Some(caps) = executor
            .resources
            .provider_registry
            .get_capabilities(provider)
        {
            if caps.supports_sleep() {
                // Check if user has a valid token
                if matches!(
                    executor
                        .auth_service
                        .get_valid_token(user_uuid, provider, tenant_id)
                        .await,
                    Ok(Some(_))
                ) {
                    return Some(provider.to_owned());
                }
            }
        }
    }
    None
}

/// Handle `analyze_sleep_quality` tool - analyze sleep data from fitness providers
///
/// Analyzes sleep duration, stages (deep/REM/light), efficiency, and generates quality score.
///
/// Supports two modes:
/// 1. **Provider mode**: Specify `sleep_provider` to auto-fetch data from connected provider
/// 2. **Manual mode**: Provide `sleep_data` JSON directly (fallback when no provider)
///
/// # Parameters
/// - `sleep_provider` (optional): Provider to fetch sleep data from (e.g., "whoop", "fitbit", "garmin")
/// - `sleep_data` (optional): Manual sleep data JSON (used if `sleep_provider` not specified)
/// - `recent_hrv_values` (optional): Array of recent HRV values for trend analysis
/// - `baseline_hrv` (optional): User's baseline HRV for comparison
///
/// # Errors
/// Returns `ProtocolError` if sleep data is missing or invalid
#[must_use]
// Long function: Protocol handler with async data fetching, HRV analysis, response formatting
#[allow(clippy::too_many_lines)]
pub fn handle_analyze_sleep_quality(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use parse_user_id_for_protocol;

        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "handle_analyze_sleep_quality cancelled by user".to_owned(),
                ));
            }
        }

        // Extract output format parameter: "json" (default) or "toon"
        let output_format = extract_output_format(&request);

        // Get sleep data from provider or manual input
        let sleep_data: SleepData = if let Some(provider_name) = request
            .parameters
            .get("sleep_provider")
            .and_then(serde_json::Value::as_str)
        {
            // Provider mode: fetch from connected provider
            let user_uuid = parse_user_id_for_protocol(&request.user_id)?;
            match fetch_provider_sleep_data(
                executor,
                user_uuid,
                request.tenant_id.as_deref(),
                provider_name,
                1, // Most recent night
            )
            .await
            {
                Ok(data) => data,
                Err(response) => return Ok(response),
            }
        } else if let Some(sleep_data_json) = request.parameters.get("sleep_data") {
            // Manual mode: parse provided JSON
            serde_json::from_value(sleep_data_json.clone()).map_err(|e| {
                ProtocolError::InvalidRequest(format!("Invalid sleep_data format: {e}"))
            })?
        } else {
            // No data source specified
            return Err(ProtocolError::InvalidRequest(
                "Either 'sleep_provider' or 'sleep_data' parameter is required. \
                 Use sleep_provider to auto-fetch from a connected provider (whoop, fitbit, garmin, terra), \
                 or provide sleep_data JSON directly."
                    .to_owned(),
            ));
        };

        // Get sleep/recovery config
        let config = &IntelligenceConfig::global().sleep_recovery;

        // Calculate sleep quality using foundation module
        let sleep_quality =
            SleepAnalyzer::calculate_sleep_quality(&sleep_data, config).map_err(|e| {
                ProtocolError::InternalError(format!(
                    "sleep_analyzer: Sleep quality calculation failed: {e}"
                ))
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
                            debug!(
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
                        ProtocolError::InternalError(format!(
                            "sleep_analyzer: HRV analysis failed: {e}"
                        ))
                    })?,
            )
        } else {
            None
        };

        let result = UniversalResponse {
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
        };

        // Apply format transformation
        Ok(apply_format_to_response(
            result,
            "sleep_quality",
            output_format,
        ))
    })
}

/// Handle `calculate_recovery_score` tool - holistic recovery assessment
///
/// Combines Training Stress Balance (TSB), sleep quality, and HRV into overall recovery score.
///
/// Supports cross-provider integration:
/// - Use `activity_provider` to specify where to fetch training data (e.g., "strava", "garmin")
/// - Use `sleep_provider` to specify where to fetch sleep/HRV data (e.g., "whoop", "fitbit")
/// - Falls back to manual `sleep_data` JSON if no `sleep_provider` specified
/// - Auto-selects best available provider if not specified
///
/// # Parameters
/// - `activity_provider` (optional): Provider for activities (default: auto-select or strava)
/// - `sleep_provider` (optional): Provider for sleep data (e.g., "whoop", "fitbit", "garmin")
/// - `sleep_data` (optional): Manual sleep data JSON (fallback if no `sleep_provider`)
/// - `user_config` (optional): User physiological parameters (FTP, LTHR, max HR, etc.)
/// - `recent_hrv_values` (optional): Array of recent HRV values
/// - `baseline_hrv` (optional): User's baseline HRV
/// - `algorithm` (optional): Recovery aggregation algorithm to use
///
/// # Errors
/// Returns `ProtocolError` if required data is missing or calculation fails
#[must_use]
// Long function: Protocol handler inherently long due to async auth, param extraction, calculation, response formatting
#[allow(clippy::too_many_lines)]
pub fn handle_calculate_recovery_score(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use parse_user_id_for_protocol;

        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "handle_calculate_recovery_score cancelled by user".to_owned(),
                ));
            }
        }

        // Extract output format parameter: "json" (default) or "toon"
        let output_format = extract_output_format(&request);

        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Determine activity provider: explicit param > auto-select > strava fallback
        let activity_provider = if let Some(provider) = request
            .parameters
            .get("activity_provider")
            .and_then(serde_json::Value::as_str)
        {
            provider.to_owned()
        } else {
            // Auto-select best available activity provider
            select_activity_provider(executor, user_uuid, request.tenant_id.as_deref())
                .await
                .unwrap_or_else(|| "strava".to_owned())
        };

        // Fetch activities from the selected provider
        let activities = match fetch_provider_activities(
            executor,
            user_uuid,
            request.tenant_id.as_deref(),
            &activity_provider,
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
                ProtocolError::InternalError(format!(
                    "sleep_analyzer: Training load calculation failed: {e}"
                ))
            })?;

        // Get sleep data: from provider or manual input (optional - TSB-only fallback available)
        let sleep_result: Option<(SleepData, Option<String>)> = if let Some(provider_name) = request
            .parameters
            .get("sleep_provider")
            .and_then(serde_json::Value::as_str)
        {
            // Fetch from specified sleep provider
            match fetch_provider_sleep_data(
                executor,
                user_uuid,
                request.tenant_id.as_deref(),
                provider_name,
                1,
            )
            .await
            {
                Ok(data) => Some((data, Some(provider_name.to_owned()))),
                Err(response) => return Ok(response),
            }
        } else if let Some(sleep_data_json) = request.parameters.get("sleep_data") {
            // Manual sleep data provided
            let data = serde_json::from_value(sleep_data_json.clone()).map_err(|e| {
                ProtocolError::InvalidRequest(format!("Invalid sleep_data format: {e}"))
            })?;
            Some((data, None))
        } else {
            // Try auto-selecting a sleep provider
            if let Some(provider_name) =
                select_sleep_provider(executor, user_uuid, request.tenant_id.as_deref()).await
            {
                fetch_provider_sleep_data(
                    executor,
                    user_uuid,
                    request.tenant_id.as_deref(),
                    &provider_name,
                    1,
                )
                .await
                .ok()
                .map(|data| (data, Some(provider_name)))
            } else {
                // No sleep provider available - will use TSB-only mode
                None
            }
        };

        // Get sleep/recovery config
        let config = &IntelligenceConfig::global().sleep_recovery;

        // Calculate recovery score based on available data
        let (recovery_score, sleep_quality_score, hrv_status, sleep_provider_used) =
            if let Some((sleep_data, sleep_provider)) = sleep_result {
                // Full mode: calculate with sleep data
                let sleep_quality = SleepAnalyzer::calculate_sleep_quality(&sleep_data, config)
                    .map_err(|e| {
                        ProtocolError::InternalError(format!(
                            "sleep_analyzer: Sleep quality calculation failed: {e}"
                        ))
                    })?;

                // Get HRV analysis if available
                let hrv_analysis = if let Some(rmssd) = sleep_data.hrv_rmssd_ms {
                    let recent_hrv = request
                        .parameters
                        .get("recent_hrv_values")
                        .and_then(|v| {
                            serde_json::from_value::<Vec<f64>>(v.clone())
                            .inspect_err(|e| {
                                debug!(
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
                                ProtocolError::InternalError(format!(
                                    "sleep_analyzer: HRV analysis failed: {e}"
                                ))
                            })?,
                    )
                } else {
                    None
                };

                // Get recovery aggregation algorithm (default to WeightedAverage with config weights)
                let algorithm = request
                    .parameters
                    .get("algorithm")
                    .and_then(|v| {
                        serde_json::from_value::<RecoveryAggregationAlgorithm>(v.clone()).ok()
                    })
                    .unwrap_or(RecoveryAggregationAlgorithm::WeightedAverage {
                        tsb_weight_full: config.recovery_scoring.tsb_weight_full,
                        sleep_weight_full: config.recovery_scoring.sleep_weight_full,
                        hrv_weight_full: config.recovery_scoring.hrv_weight_full,
                        tsb_weight_no_hrv: config.recovery_scoring.tsb_weight_no_hrv,
                        sleep_weight_no_hrv: config.recovery_scoring.sleep_weight_no_hrv,
                    });

                // Calculate holistic recovery score
                let score = RecoveryCalculator::calculate_recovery_score(
                    &training_load,
                    &sleep_quality,
                    hrv_analysis.as_ref(),
                    config,
                    &algorithm,
                )
                .map_err(|e| {
                    ProtocolError::InternalError(format!(
                        "sleep_analyzer: Recovery score calculation failed: {e}"
                    ))
                })?;

                let hrv_status = hrv_analysis.as_ref().map(|h| h.recovery_status);
                (
                    score,
                    Some(sleep_quality.overall_score),
                    hrv_status,
                    sleep_provider,
                )
            } else {
                // TSB-only fallback mode: no sleep data available
                debug!("No sleep data available, using TSB-only recovery calculation");
                let score =
                    RecoveryCalculator::calculate_recovery_score_tsb_only(&training_load, config)
                        .map_err(|e| {
                        ProtocolError::InternalError(format!(
                            "sleep_analyzer: TSB-only recovery score calculation failed: {e}"
                        ))
                    })?;

                (score, None, None, None)
            };

        let result = UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "recovery_score": recovery_score,
                "training_load": {
                    "ctl": training_load.ctl,
                    "atl": training_load.atl,
                    "tsb": training_load.tsb,
                },
                "sleep_quality_score": sleep_quality_score,
                "hrv_status": hrv_status,
                "providers_used": {
                    "activity_provider": activity_provider,
                    "sleep_provider": sleep_provider_used,
                },
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
                map.insert(
                    "activity_provider".into(),
                    serde_json::Value::String(activity_provider),
                );
                if let Some(ref provider) = sleep_provider_used {
                    map.insert(
                        "sleep_provider".into(),
                        serde_json::Value::String(provider.clone()),
                    );
                }
                map.insert(
                    "data_completeness".into(),
                    serde_json::to_value(recovery_score.data_completeness)
                        .unwrap_or(serde_json::Value::String("unknown".to_owned())),
                );
                map
            }),
        };

        // Apply format transformation
        Ok(apply_format_to_response(result, "recovery", output_format))
    })
}

/// Handle `suggest_rest_day` tool - AI-powered rest day recommendation
///
/// Analyzes recovery score, training load, and sleep to recommend rest or training.
///
/// Supports cross-provider integration:
/// - Use `activity_provider` to specify where to fetch training data
/// - Use `sleep_provider` to specify where to fetch sleep/HRV data
/// - Auto-selects best available providers if not specified
///
/// # Parameters
/// - `activity_provider` (optional): Provider for activities (default: auto-select)
/// - `sleep_provider` (optional): Provider for sleep data
/// - `sleep_data` (optional): Manual sleep data JSON (fallback)
/// - `user_config` (optional): User physiological parameters
///
/// # Errors
/// Returns `ProtocolError` if required data is missing or analysis fails
#[must_use]
// Long function: Protocol handler inherently long due to async auth, param extraction, calculation, response formatting
#[allow(clippy::too_many_lines)]
pub fn handle_suggest_rest_day(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use parse_user_id_for_protocol;

        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "handle_suggest_rest_day cancelled by user".to_owned(),
                ));
            }
        }

        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Determine activity provider
        let activity_provider = if let Some(provider) = request
            .parameters
            .get("activity_provider")
            .and_then(serde_json::Value::as_str)
        {
            provider.to_owned()
        } else {
            select_activity_provider(executor, user_uuid, request.tenant_id.as_deref())
                .await
                .unwrap_or_else(|| "strava".to_owned())
        };

        // Get recent activities from selected provider
        let activities = match fetch_provider_activities(
            executor,
            user_uuid,
            request.tenant_id.as_deref(),
            &activity_provider,
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
                ProtocolError::InternalError(format!(
                    "sleep_analyzer: Training load calculation failed: {e}"
                ))
            })?;

        // Get sleep data from provider or manual input (optional - TSB-only fallback available)
        let sleep_result: Option<SleepData> = if let Some(provider_name) = request
            .parameters
            .get("sleep_provider")
            .and_then(serde_json::Value::as_str)
        {
            match fetch_provider_sleep_data(
                executor,
                user_uuid,
                request.tenant_id.as_deref(),
                provider_name,
                1,
            )
            .await
            {
                Ok(data) => Some(data),
                Err(response) => return Ok(response),
            }
        } else if let Some(sleep_data_json) = request.parameters.get("sleep_data") {
            let data = serde_json::from_value(sleep_data_json.clone()).map_err(|e| {
                ProtocolError::InvalidRequest(format!("Invalid sleep_data format: {e}"))
            })?;
            Some(data)
        } else if let Some(provider_name) =
            select_sleep_provider(executor, user_uuid, request.tenant_id.as_deref()).await
        {
            // If provider fetch fails, fall back to TSB-only mode
            fetch_provider_sleep_data(
                executor,
                user_uuid,
                request.tenant_id.as_deref(),
                &provider_name,
                1,
            )
            .await
            .ok()
        } else {
            // No sleep provider available - will use TSB-only mode
            None
        };

        // Get sleep/recovery config
        let config = &IntelligenceConfig::global().sleep_recovery;

        // Calculate recovery score and generate recommendation based on available data
        let (recovery_score, recommendation, sleep_quality_score, sleep_hours, hrv_status) =
            if let Some(sleep_data) = sleep_result {
                // Full mode: calculate with sleep data
                let sleep_quality = SleepAnalyzer::calculate_sleep_quality(&sleep_data, config)
                    .map_err(|e| {
                        ProtocolError::InternalError(format!(
                            "sleep_analyzer: Sleep quality calculation failed: {e}"
                        ))
                    })?;

                // HRV analysis
                let hrv_analysis = if let Some(rmssd) = sleep_data.hrv_rmssd_ms {
                    let recent_hrv = request
                        .parameters
                        .get("recent_hrv_values")
                        .and_then(|v| {
                            serde_json::from_value::<Vec<f64>>(v.clone())
                                .inspect_err(|e| {
                                    debug!(
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
                                ProtocolError::InternalError(format!(
                                    "sleep_analyzer: HRV analysis failed: {e}"
                                ))
                            })?,
                    )
                } else {
                    None
                };

                // Get recovery aggregation algorithm
                let algorithm = request
                    .parameters
                    .get("algorithm")
                    .and_then(|v| {
                        serde_json::from_value::<RecoveryAggregationAlgorithm>(v.clone()).ok()
                    })
                    .unwrap_or(RecoveryAggregationAlgorithm::WeightedAverage {
                        tsb_weight_full: config.recovery_scoring.tsb_weight_full,
                        sleep_weight_full: config.recovery_scoring.sleep_weight_full,
                        hrv_weight_full: config.recovery_scoring.hrv_weight_full,
                        tsb_weight_no_hrv: config.recovery_scoring.tsb_weight_no_hrv,
                        sleep_weight_no_hrv: config.recovery_scoring.sleep_weight_no_hrv,
                    });

                // Calculate recovery score
                let score = RecoveryCalculator::calculate_recovery_score(
                    &training_load,
                    &sleep_quality,
                    hrv_analysis.as_ref(),
                    config,
                    &algorithm,
                )
                .map_err(|e| {
                    ProtocolError::InternalError(format!(
                        "sleep_analyzer: Recovery score calculation failed: {e}"
                    ))
                })?;

                // Generate rest day recommendation
                let rec = RecoveryCalculator::recommend_rest_day(
                    &score,
                    &sleep_data,
                    &training_load,
                    config,
                )
                .map_err(|e| {
                    ProtocolError::InternalError(format!(
                        "sleep_analyzer: Rest day recommendation failed: {e}"
                    ))
                })?;

                let hrv_status = hrv_analysis.as_ref().map(|h| h.recovery_status);
                (
                    score,
                    rec,
                    Some(sleep_quality.overall_score),
                    Some(sleep_data.duration_hours),
                    hrv_status,
                )
            } else {
                // TSB-only fallback mode: no sleep data available
                debug!("No sleep data available, using TSB-only rest day recommendation");
                let score =
                    RecoveryCalculator::calculate_recovery_score_tsb_only(&training_load, config)
                        .map_err(|e| {
                        ProtocolError::InternalError(format!(
                            "sleep_analyzer: TSB-only recovery score calculation failed: {e}"
                        ))
                    })?;

                let rec =
                    RecoveryCalculator::recommend_rest_day_tsb_only(&score, &training_load, config)
                        .map_err(|e| {
                            ProtocolError::InternalError(format!(
                                "sleep_analyzer: TSB-only rest day recommendation failed: {e}"
                            ))
                        })?;

                (score, rec, None, None, None)
            };

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "recommendation": recommendation,
                "recovery_summary": {
                    "overall_score": recovery_score.overall_score,
                    "category": recovery_score.recovery_category,
                    "training_readiness": recovery_score.training_readiness,
                    "data_completeness": recovery_score.data_completeness,
                    "limitations": recovery_score.limitations,
                },
                "key_factors": {
                    "tsb": training_load.tsb,
                    "sleep_score": sleep_quality_score,
                    "sleep_hours": sleep_hours,
                    "hrv_status": hrv_status,
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
                    warn!(
                        confidence = recommendation.confidence,
                        "Invalid confidence value (NaN/Infinity), omitting from metadata"
                    );
                }
                map.insert(
                    "data_completeness".into(),
                    serde_json::to_value(recovery_score.data_completeness)
                        .unwrap_or(serde_json::Value::String("unknown".to_owned())),
                );
                map
            }),
        })
    })
}

/// Handle `track_sleep_trends` tool - analyze sleep patterns over time
///
/// Correlates sleep quality with performance and training load.
///
/// Supports two modes:
/// 1. **Provider mode**: Specify `sleep_provider` and `days` to auto-fetch history
/// 2. **Manual mode**: Provide `sleep_history` JSON array directly
///
/// # Parameters
/// - `sleep_provider` (optional): Provider to fetch sleep history from
/// - `days` (optional): Number of days of history to fetch (default: 14)
/// - `sleep_history` (optional): Manual sleep history JSON array
///
/// # Errors
/// Returns `ProtocolError` if data is insufficient or analysis fails
#[must_use]
// Long function: Protocol handler inherently long due to trend calculation, statistics aggregation, response formatting
#[allow(clippy::too_many_lines)]
pub fn handle_track_sleep_trends(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use parse_user_id_for_protocol;

        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "handle_track_sleep_trends cancelled by user".to_owned(),
                ));
            }
        }

        // Extract output format parameter: "json" (default) or "toon"
        let output_format = extract_output_format(&request);

        // Get sleep history from provider or manual input
        let sleep_history: Vec<SleepData> = if let Some(provider_name) = request
            .parameters
            .get("sleep_provider")
            .and_then(serde_json::Value::as_str)
        {
            // Provider mode: fetch from connected provider
            let user_uuid = parse_user_id_for_protocol(&request.user_id)?;
            // Safe: days parameter is user input, clamped to u32::MAX (reasonable for date ranges)
            #[allow(clippy::cast_possible_truncation)]
            let days = request
                .parameters
                .get("days")
                .and_then(serde_json::Value::as_u64)
                .map_or(14, |d| d.min(u64::from(u32::MAX)) as u32);

            match fetch_provider_sleep_history(
                executor,
                user_uuid,
                request.tenant_id.as_deref(),
                provider_name,
                days,
            )
            .await
            {
                Ok(history) => history,
                Err(response) => return Ok(response),
            }
        } else if let Some(sleep_history_json) = request.parameters.get("sleep_history") {
            // Manual mode: parse provided JSON
            serde_json::from_value(sleep_history_json.clone()).map_err(|e| {
                ProtocolError::InvalidRequest(format!("Invalid sleep_history format: {e}"))
            })?
        } else {
            // Try auto-selecting a sleep provider
            let user_uuid = parse_user_id_for_protocol(&request.user_id)?;
            if let Some(provider_name) =
                select_sleep_provider(executor, user_uuid, request.tenant_id.as_deref()).await
            {
                // Safe: days parameter is user input, clamped to u32::MAX (reasonable for date ranges)
                #[allow(clippy::cast_possible_truncation)]
                let days = request
                    .parameters
                    .get("days")
                    .and_then(serde_json::Value::as_u64)
                    .map_or(14, |d| d.min(u64::from(u32::MAX)) as u32);

                match fetch_provider_sleep_history(
                    executor,
                    user_uuid,
                    request.tenant_id.as_deref(),
                    &provider_name,
                    days,
                )
                .await
                {
                    Ok(history) => history,
                    Err(response) => return Ok(response),
                }
            } else {
                return Err(ProtocolError::InvalidRequest(
                    "Either 'sleep_provider' or 'sleep_history' parameter is required. \
                     Use sleep_provider to auto-fetch from a connected provider, \
                     or provide sleep_history JSON directly."
                        .to_owned(),
                ));
            }
        };

        let trend_min_days = executor.resources.config.sleep_tool_params.trend_min_days;
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
        let config = &IntelligenceConfig::global().sleep_recovery;

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
            .sleep_tool_params
            .trend_improving_threshold;
        let declining_threshold = executor
            .resources
            .config
            .sleep_tool_params
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
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
        let worst_night = quality_scores
            .iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));

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

        let result = UniversalResponse {
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
        };

        // Apply format transformation
        Ok(apply_format_to_response(
            result,
            "sleep_trends",
            output_format,
        ))
    })
}

/// Handle `optimize_sleep_schedule` tool - recommend optimal sleep based on training
///
/// Suggests sleep duration and timing based on upcoming workouts and recovery needs.
///
/// Supports cross-provider integration:
/// - Use `activity_provider` to specify where to fetch training load data
/// - Auto-selects best available provider if not specified
///
/// # Parameters
/// - `activity_provider` (optional): Provider for activities (default: auto-select)
/// - `user_config` (optional): User physiological parameters
/// - `upcoming_workout_intensity` (optional): "low", "moderate", or "high"
/// - `typical_wake_time` (optional): Wake time in "HH:MM" format (default: "06:00")
///
/// # Errors
/// Returns `ProtocolError` if required parameters are missing
#[must_use]
// Long function: Protocol handler inherently long due to async auth, param extraction, calculation, response formatting
#[allow(clippy::too_many_lines)]
pub fn handle_optimize_sleep_schedule(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use parse_user_id_for_protocol;

        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Determine activity provider
        let activity_provider = if let Some(provider) = request
            .parameters
            .get("activity_provider")
            .and_then(serde_json::Value::as_str)
        {
            provider.to_owned()
        } else {
            select_activity_provider(executor, user_uuid, request.tenant_id.as_deref())
                .await
                .unwrap_or_else(|| "strava".to_owned())
        };

        // Get recent activities for training load from selected provider
        let activities = match fetch_provider_activities(
            executor,
            user_uuid,
            request.tenant_id.as_deref(),
            &activity_provider,
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
                ProtocolError::InternalError(format!(
                    "sleep_analyzer: Training load calculation failed: {e}"
                ))
            })?;

        // Get sleep/recovery config
        let config = &IntelligenceConfig::global().sleep_recovery;

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
            base_recommendation
                + executor
                    .resources
                    .config
                    .sleep_tool_params
                    .fatigue_bonus_hours
        } else if training_load.atl
            > executor
                .resources
                .config
                .sleep_tool_params
                .high_load_atl_threshold
        {
            // High acute load: prioritize recovery
            base_recommendation
                + executor
                    .resources
                    .config
                    .sleep_tool_params
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
            executor
                .resources
                .config
                .sleep_tool_params
                .wind_down_minutes,
            executor.resources.config.sleep_tool_params.minutes_per_day,
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
        warn!(
            wake_time = wake_time,
            "Invalid wake_time format (expected HH:MM), using default 06:00"
        );
        return "22:00".to_owned(); // Default fallback
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
