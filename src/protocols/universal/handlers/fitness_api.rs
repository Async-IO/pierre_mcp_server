// ABOUTME: Fitness provider API handlers for universal protocol
// ABOUTME: Provider-agnostic single responsibility handlers that delegate auth to AuthService
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::cache::{factory::Cache, CacheKey, CacheResource};
use crate::config::environment::default_provider;
use crate::formatters::{format_output, OutputFormat};
use crate::intelligence::physiological_constants::api_limits::{
    safe_limit_json_detailed, safe_limit_json_summary, safe_limit_toon_detailed,
    safe_limit_toon_summary, CLAUDE_CONTEXT_TOKENS, CONTEXT_WARNING_THRESHOLD_PERCENT,
    DEFAULT_ACTIVITY_LIMIT_U32, MAX_ACTIVITY_LIMIT, TOKENS_PER_ACTIVITY_DETAILED,
    TOKENS_PER_ACTIVITY_SUMMARY, USABLE_CONTEXT_TOKENS,
};
use crate::models::{Activity, Athlete, SportType, Stats};
use crate::protocols::universal::{UniversalRequest, UniversalResponse, UniversalToolExecutor};
use crate::protocols::ProtocolError;
use crate::providers::core::{ActivityQueryParams, FitnessProvider};
use crate::utils::uuid::parse_user_id_for_protocol;
use serde::Serialize;
use serde_json::{json, to_value, Value};
use std::cmp::Reverse;
use std::collections::HashMap;
use std::future::Future;
use std::hash::BuildHasher;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Activity summary with minimal fields for efficient list queries
/// Used when mode=summary to reduce payload size and preserve LLM context
#[derive(Debug, Clone, Serialize)]
pub struct ActivitySummary {
    /// Unique activity identifier
    pub id: String,
    /// Activity name/title
    pub name: String,
    /// Activity sport type (e.g., "run", "ride", "cross\_country\_skiing")
    pub sport_type: SportType,
    /// Start date/time in ISO 8601 format
    pub start_date: String,
    /// Distance in meters (0.0 if not available)
    pub distance_meters: f64,
    /// Duration in seconds
    pub duration_seconds: u64,
}

impl From<&Activity> for ActivitySummary {
    fn from(activity: &Activity) -> Self {
        Self {
            id: activity.id().to_owned(),
            name: activity.name().to_owned(),
            sport_type: activity.sport_type().clone(),
            start_date: activity.start_date().to_rfc3339(),
            distance_meters: activity.distance_meters().unwrap_or(0.0),
            duration_seconds: activity.duration_seconds(),
        }
    }
}

/// Format activities as a numbered human-readable list for LLM output
/// This helps smaller models include the list in their response without transforming JSON
/// Activities are sorted by date descending (newest first) for better user experience
fn format_activities_as_list(activities: &[Activity]) -> String {
    let mut lines = Vec::with_capacity(activities.len() + 2);
    lines.push("Your Activities:".to_owned());
    lines.push(String::new());

    // Sort activities by start_date descending (newest first)
    let mut sorted_activities: Vec<_> = activities.iter().collect();
    sorted_activities.sort_by_key(|a| Reverse(a.start_date()));

    for (i, activity) in sorted_activities.iter().enumerate() {
        let date = activity.start_date().format("%Y-%m-%d").to_string();
        // Format sport type cleanly - extract inner string for Other variant
        let sport = match activity.sport_type() {
            SportType::Other(s) => s.clone(),
            other => format!("{other:?}"),
        };
        let distance_km = activity.distance_meters().unwrap_or(0.0) / 1000.0;
        let duration_secs = activity.duration_seconds();
        let hours = duration_secs / 3600;
        let minutes = (duration_secs % 3600) / 60;
        let seconds = duration_secs % 60;

        let duration_str = if hours > 0 {
            format!("{hours}:{minutes:02}:{seconds:02}")
        } else {
            format!("{minutes}:{seconds:02}")
        };

        lines.push(format!(
            "{}. [{}] {} - {} - {:.2} km - {}",
            i + 1,
            sport,
            activity.name(),
            date,
            distance_km,
            duration_str
        ));
    }

    lines.join("\n")
}

/// Pagination metadata for list responses
/// Enables clients to intelligently paginate through large result sets
#[derive(Debug, Clone, Default)]
pub struct PaginationInfo {
    /// The offset that was requested (0 if not specified)
    pub offset: usize,
    /// The limit that was applied to the query
    pub limit: usize,
    /// Number of items actually returned in this response
    pub returned_count: usize,
    /// True if there are likely more results available (`returned_count` == `limit`)
    pub has_more: bool,
}

/// Token usage estimation for LLM context management
/// Helps users understand how much of their context window is being used
#[derive(Debug, Clone, Serialize)]
pub struct TokenEstimate {
    /// Estimated tokens for this response
    pub estimated_tokens: usize,
    /// Percentage of Claude's context used (out of 200K)
    pub context_usage_percent: f64,
    /// Percentage of usable context used (out of 150K, leaving room for prompts)
    pub usable_context_percent: f64,
    /// Whether context usage is above warning threshold
    pub context_warning: bool,
    /// Human-readable context guidance
    pub guidance: String,
}

impl TokenEstimate {
    /// Create token estimate based on activity count and mode
    /// Token counts are small enough that f64 precision loss is negligible
    #[allow(clippy::cast_precision_loss)]
    fn from_activities(count: usize, mode: &str) -> Self {
        let tokens_per_activity = if mode == "summary" {
            TOKENS_PER_ACTIVITY_SUMMARY
        } else {
            TOKENS_PER_ACTIVITY_DETAILED
        };

        let estimated_tokens = count * tokens_per_activity;
        let context_usage_percent =
            (estimated_tokens as f64 / CLAUDE_CONTEXT_TOKENS as f64) * 100.0;
        let usable_context_percent =
            (estimated_tokens as f64 / USABLE_CONTEXT_TOKENS as f64) * 100.0;
        let context_warning = usable_context_percent > CONTEXT_WARNING_THRESHOLD_PERCENT as f64;

        let guidance = if context_warning {
            format!(
                "Using {usable_context_percent:.1}% of usable context. Consider filtering by sport_type or using a smaller time range."
            )
        } else if usable_context_percent > 25.0 {
            format!("Using {usable_context_percent:.1}% of usable context. Plenty of room for analysis.")
        } else {
            format!(
                "Using {usable_context_percent:.1}% of usable context. Excellent - lots of room for detailed analysis."
            )
        };

        Self {
            estimated_tokens,
            context_usage_percent,
            usable_context_percent,
            context_warning,
            guidance,
        }
    }
}

/// Parameters for trying to get cached activities
struct CachedActivitiesParams<'a> {
    cache: &'a Arc<Cache>,
    cache_key: &'a CacheKey,
    user_uuid: uuid::Uuid,
    tenant_id: Option<String>,
    provider_name: &'a str,
    mode: &'a str,
    output_format: OutputFormat,
    limit: usize,
    offset: usize,
    default_time_window_applied: bool,
}

/// Create metadata for activity analysis responses
fn create_activity_metadata(
    activity_id: &str,
    user_uuid: Uuid,
    tenant_id: Option<&String>,
) -> HashMap<String, Value> {
    let mut map = HashMap::new();
    map.insert(
        "activity_id".to_owned(),
        Value::String(activity_id.to_owned()),
    );
    map.insert("user_id".to_owned(), Value::String(user_uuid.to_string()));
    map.insert(
        "tenant_id".to_owned(),
        tenant_id.map_or(Value::Null, |id| {
            Value::String(id.clone()) // Safe: String ownership for JSON value
        }),
    );
    map
}

/// Extract output format parameter from request
/// Returns `OutputFormat::Json` as default for backwards compatibility
pub fn extract_output_format(request: &UniversalRequest) -> OutputFormat {
    request
        .parameters
        .get("format")
        .and_then(|v| v.as_str())
        .map_or(OutputFormat::Json, OutputFormat::from_str_param)
}

/// Apply format transformation to an existing `UniversalResponse`.
///
/// This is useful for handlers that delegate to internal functions returning `UniversalResponse`.
/// If the response is successful and has a result, formats it according to `output_format`.
pub fn apply_format_to_response(
    mut response: UniversalResponse,
    data_key: &str,
    output_format: OutputFormat,
) -> UniversalResponse {
    // Only apply formatting to successful responses with data
    if !response.success || response.result.is_none() {
        return response;
    }

    // JSON is the default, no transformation needed
    if matches!(output_format, OutputFormat::Json) {
        // Add format metadata
        if let Some(ref mut metadata) = response.metadata {
            metadata.insert("format".to_owned(), Value::String("json".to_owned()));
        }
        return response;
    }

    // Apply TOON formatting - result presence was verified by guard above
    let Some(result_value) = response.result.take() else {
        // Defensive: return unchanged if result is unexpectedly None
        return response;
    };

    match format_output(&result_value, OutputFormat::Toon) {
        Ok(formatted) => {
            let toon_key = format!("{data_key}_toon");
            response.result = Some(json!({
                toon_key: formatted.data,
                "format": "toon"
            }));
            if let Some(ref mut metadata) = response.metadata {
                metadata.insert("format".to_owned(), Value::String("toon".to_owned()));
            }
        }
        Err(e) => {
            // Fall back to JSON on encoding error
            warn!("TOON encoding failed, falling back to JSON: {}", e);
            response.result = Some(json!({
                data_key: result_value,
                "format": "json",
                "format_fallback": true,
                "format_error": e.to_string()
            }));
            if let Some(ref mut metadata) = response.metadata {
                metadata.insert("format".to_owned(), Value::String("json".to_owned()));
                metadata.insert("format_fallback".to_owned(), Value::Bool(true));
            }
        }
    }

    response
}

/// Build a formatted response with format support (JSON or TOON).
///
/// Generic helper for all data-returning handlers.
///
/// # Errors
///
/// Returns `ProtocolError::SerializationError` if:
/// - Data serialization to JSON fails
/// - TOON encoding fails (falls back to JSON with metadata flag)
pub fn build_formatted_response<T, S>(
    data: &T,
    data_key: &str,
    output_format: OutputFormat,
    metadata: HashMap<String, Value, S>,
) -> Result<UniversalResponse, ProtocolError>
where
    T: Serialize,
    S: BuildHasher,
{
    // Convert to standard HashMap for UniversalResponse compatibility
    let mut metadata: HashMap<String, Value> = metadata.into_iter().collect();

    // Add format to metadata
    metadata.insert(
        "format".to_owned(),
        Value::String(output_format.as_str().to_owned()),
    );

    let result_json = match output_format {
        OutputFormat::Toon => {
            // Convert data to JSON value first for TOON encoding
            let data_value = to_value(data).map_err(|e| {
                ProtocolError::SerializationError(format!("Failed to serialize data: {e}"))
            })?;

            match format_output(&data_value, OutputFormat::Toon) {
                Ok(formatted) => {
                    // Use _toon suffix for the data key to indicate TOON format
                    let toon_key = format!("{data_key}_toon");
                    json!({
                        toon_key: formatted.data,
                        "format": "toon"
                    })
                }
                Err(e) => {
                    // Fall back to JSON if TOON serialization fails
                    warn!("TOON serialization failed, falling back to JSON: {}", e);
                    metadata.insert("format".to_owned(), Value::String("json".to_owned()));
                    metadata.insert("format_fallback".to_owned(), Value::Bool(true));
                    metadata.insert("format_error".to_owned(), Value::String(e.to_string()));
                    json!({
                        data_key: to_value(data).map_err(|e| {
                            ProtocolError::SerializationError(format!("Failed to serialize data: {e}"))
                        })?,
                        "format": "json"
                    })
                }
            }
        }
        OutputFormat::Json => {
            json!({
                data_key: to_value(data).map_err(|e| {
                    ProtocolError::SerializationError(format!("Failed to serialize data: {e}"))
                })?,
                "format": "json"
            })
        }
    };

    Ok(UniversalResponse {
        success: true,
        result: Some(result_json),
        error: None,
        metadata: Some(metadata),
    })
}

/// Try to get activities from cache
async fn try_get_cached_activities(
    params: CachedActivitiesParams<'_>,
) -> Option<UniversalResponse> {
    if let Ok(Some(cached_activities)) = params.cache.get::<Vec<Activity>>(params.cache_key).await {
        info!(
            "Cache hit for activities (count={}, mode={}, format={:?})",
            cached_activities.len(),
            params.mode,
            params.output_format
        );

        // Sort by start_date descending (newest first) for consistent ordering
        let mut sorted_activities = cached_activities;
        sorted_activities.sort_by_key(|a| Reverse(a.start_date()));

        // Create pagination info from cached results
        let pagination = PaginationInfo {
            offset: params.offset,
            limit: params.limit,
            returned_count: sorted_activities.len(),
            has_more: sorted_activities.len() == params.limit,
        };
        // Use the same response builder as the non-cached path to apply mode/format
        let mut response = build_activities_success_response(ActivitiesResponseParams {
            activities: &sorted_activities,
            user_uuid: params.user_uuid,
            tenant_id: params.tenant_id,
            provider_name: params.provider_name,
            mode: params.mode,
            output_format: params.output_format,
            pagination: Some(&pagination),
            default_time_window_applied: params.default_time_window_applied,
        });
        // Mark as cached in metadata
        if let Some(ref mut metadata) = response.metadata {
            metadata.insert("cached".to_owned(), Value::Bool(true));
        }
        return Some(response);
    }
    info!("Cache miss for activities");
    None
}

/// Cache activities after fetching from API
async fn cache_activities_result(
    cache: &Arc<Cache>,
    cache_key: &CacheKey,
    activities: &Vec<Activity>,
    per_page: u32,
) {
    let ttl = CacheResource::ActivityList {
        page: 1,
        per_page,
        before: None,
        after: None,
    }
    .recommended_ttl();
    if let Err(e) = cache.set(cache_key, activities, ttl).await {
        warn!("Failed to cache activities: {}", e);
    } else {
        info!("Cached {} activities with TTL {:?}", activities.len(), ttl);
    }
}

/// Filter activities by sport type (case-insensitive)
/// Handles both standard sport types (serialized as strings like "run")
/// and Other variants (serialized as {"other":"NordicSki"})
fn filter_activities_by_sport_type(
    activities: Vec<Activity>,
    sport_type_filter: Option<&str>,
) -> Vec<Activity> {
    match sport_type_filter {
        Some(filter) => {
            let filter_lower = filter.to_lowercase();
            activities
                .into_iter()
                .filter(|a| {
                    // Serialize sport_type to JSON and compare case-insensitively
                    let Ok(v) = to_value(a.sport_type()) else {
                        return false;
                    };

                    // Standard sport types serialize as simple strings (e.g., "run", "ride")
                    if let Some(s) = v.as_str() {
                        return s.to_lowercase() == filter_lower;
                    }

                    // Other(String) variants serialize as {"other":"value"}
                    if let Some(obj) = v.as_object() {
                        if let Some(other_value) = obj.get("other").and_then(|v| v.as_str()) {
                            return other_value.to_lowercase() == filter_lower;
                        }
                    }

                    false
                })
                .collect()
        }
        None => activities,
    }
}

/// Build metadata for activities response
fn build_activities_metadata(
    count: usize,
    user_uuid: Uuid,
    tenant_id: Option<String>,
    mode_used: &str,
    format_used: &str,
    pagination: Option<&PaginationInfo>,
) -> HashMap<String, Value> {
    let mut map = HashMap::new();
    map.insert("returned_count".to_owned(), Value::Number(count.into()));
    map.insert("user_id".to_owned(), Value::String(user_uuid.to_string()));
    map.insert(
        "tenant_id".to_owned(),
        tenant_id.map_or(Value::Null, Value::String),
    );
    map.insert("cached".to_owned(), Value::Bool(false));
    map.insert("mode".to_owned(), Value::String(mode_used.to_owned()));
    map.insert("format".to_owned(), Value::String(format_used.to_owned()));
    // Add pagination metadata when available
    if let Some(page_info) = pagination {
        map.insert("offset".to_owned(), Value::Number(page_info.offset.into()));
        map.insert("limit".to_owned(), Value::Number(page_info.limit.into()));
        map.insert("has_more".to_owned(), Value::Bool(page_info.has_more));
    }
    map
}

/// Prepare activity data for response based on mode (summary or detailed)
/// Returns the JSON value and mode string, or error message string if serialization fails
fn prepare_activity_data(
    activities: &[Activity],
    mode: &str,
) -> Result<(Value, &'static str), String> {
    if mode == "summary" {
        let summaries: Vec<ActivitySummary> =
            activities.iter().map(ActivitySummary::from).collect();
        to_value(&summaries)
            .map(|v| (v, "summary"))
            .map_err(|e| format!("Failed to serialize activity summaries: {e}"))
    } else {
        to_value(activities)
            .map(|v| (v, "detailed"))
            .map_err(|e| format!("Failed to serialize activities: {e}"))
    }
}

/// Add common fields (pagination, token estimate, time window flag) to activity response JSON
fn add_common_response_fields(
    json_val: &mut Value,
    pagination: Option<&PaginationInfo>,
    token_estimate: &TokenEstimate,
    default_time_window_applied: bool,
) {
    if let Some(page_info) = pagination {
        json_val["offset"] = json!(page_info.offset);
        json_val["limit"] = json!(page_info.limit);
        json_val["has_more"] = json!(page_info.has_more);
    }
    json_val["token_estimate"] = json!(token_estimate);
    json_val["default_time_window_applied"] = json!(default_time_window_applied);
}

/// Parameters for building an activities success response
struct ActivitiesResponseParams<'a> {
    activities: &'a [Activity],
    user_uuid: Uuid,
    tenant_id: Option<String>,
    provider_name: &'a str,
    mode: &'a str,
    output_format: OutputFormat,
    pagination: Option<&'a PaginationInfo>,
    default_time_window_applied: bool,
}

/// Build success response for activities with mode and format support
/// `mode="summary"` returns minimal fields (id, name, `sport_type`, `start_date`, distance, duration)
/// `mode="detailed"` returns full activity data (default for backwards compatibility when not specified)
/// `format="json"` (default) or `format="toon"` for token-efficient LLM output
/// `pagination` enables clients to paginate through large result sets
/// `default_time_window_applied` indicates if the 90-day default was used
fn build_activities_success_response(params: ActivitiesResponseParams<'_>) -> UniversalResponse {
    let ActivitiesResponseParams {
        activities,
        user_uuid,
        tenant_id,
        provider_name,
        mode,
        output_format,
        pagination,
        default_time_window_applied,
    } = params;

    // Prepare the data based on mode
    let (data_value, mode_used) = match prepare_activity_data(activities, mode) {
        Ok(result) => result,
        Err(error) => {
            return UniversalResponse {
                success: false,
                result: None,
                error: Some(error),
                metadata: None,
            }
        }
    };

    // Create pre-formatted activity list for LLM output (helps models include the list)
    let activity_list = format_activities_as_list(activities);

    // Calculate token estimate for context management
    let token_estimate = TokenEstimate::from_activities(activities.len(), mode_used);

    // Format the activities data according to the requested format
    let (result_json, format_used) = match output_format {
        OutputFormat::Toon => match format_output(&data_value, OutputFormat::Toon) {
            Ok(formatted) => {
                let mut json_val = json!({
                    "activity_list": activity_list,
                    "activities_toon": formatted.data,
                    "provider": provider_name,
                    "count": activities.len(),
                    "mode": mode_used,
                    "format": "toon"
                });
                add_common_response_fields(
                    &mut json_val,
                    pagination,
                    &token_estimate,
                    default_time_window_applied,
                );
                (json_val, "toon")
            }
            Err(e) => {
                warn!("TOON serialization failed, falling back to JSON: {e}");
                let mut json_val = json!({
                    "activity_list": activity_list,
                    "activities": data_value,
                    "provider": provider_name,
                    "count": activities.len(),
                    "mode": mode_used,
                    "format": "json",
                    "format_fallback": true,
                    "format_error": e.to_string()
                });
                add_common_response_fields(
                    &mut json_val,
                    pagination,
                    &token_estimate,
                    default_time_window_applied,
                );
                (json_val, "json")
            }
        },
        OutputFormat::Json => {
            let mut json_val = json!({
                "activity_list": activity_list,
                "activities": data_value,
                "provider": provider_name,
                "count": activities.len(),
                "mode": mode_used,
                "format": "json"
            });
            add_common_response_fields(
                &mut json_val,
                pagination,
                &token_estimate,
                default_time_window_applied,
            );
            (json_val, "json")
        }
    };

    let metadata = build_activities_metadata(
        activities.len(),
        user_uuid,
        tenant_id,
        mode_used,
        format_used,
        pagination,
    );
    UniversalResponse {
        success: true,
        result: Some(result_json),
        error: None,
        metadata: Some(metadata),
    }
}

/// Try to get athlete from cache
async fn try_get_cached_athlete(
    cache: &Arc<Cache>,
    cache_key: &CacheKey,
    user_uuid: Uuid,
    tenant_id: Option<&String>,
    output_format: OutputFormat,
) -> Result<Option<UniversalResponse>, ProtocolError> {
    if let Ok(Some(cached_athlete)) = cache.get::<Athlete>(cache_key).await {
        info!("Cache hit for athlete profile");
        let mut metadata = HashMap::new();
        metadata.insert("user_id".to_owned(), Value::String(user_uuid.to_string()));
        metadata.insert(
            "tenant_id".to_owned(),
            tenant_id.map_or(Value::Null, |id| Value::String(id.clone())),
        );
        metadata.insert("cached".to_owned(), Value::Bool(true));

        return Ok(Some(build_formatted_response(
            &cached_athlete,
            "athlete",
            output_format,
            metadata,
        )?));
    }
    info!("Cache miss for athlete profile");
    Ok(None)
}

/// Cache athlete profile after fetching from API
async fn cache_athlete_result(cache: &Arc<Cache>, cache_key: &CacheKey, athlete: &Athlete) {
    let ttl = CacheResource::AthleteProfile.recommended_ttl();
    if let Err(e) = cache.set(cache_key, athlete, ttl).await {
        warn!("Failed to cache athlete profile: {}", e);
    } else {
        info!("Cached athlete profile with TTL {:?}", ttl);
    }
}

/// Fetch athlete from API and cache result
async fn fetch_and_cache_athlete(
    provider: &dyn FitnessProvider,
    cache: &Arc<Cache>,
    cache_key: &CacheKey,
    user_uuid: Uuid,
    tenant_id: Option<String>,
    output_format: OutputFormat,
) -> Result<UniversalResponse, ProtocolError> {
    match provider.get_athlete().await {
        Ok(athlete) => {
            cache_athlete_result(cache, cache_key, &athlete).await;

            let mut metadata = HashMap::new();
            metadata.insert("user_id".to_owned(), Value::String(user_uuid.to_string()));
            metadata.insert(
                "tenant_id".to_owned(),
                tenant_id.map_or(Value::Null, Value::String),
            );
            metadata.insert("cached".to_owned(), Value::Bool(false));

            build_formatted_response(&athlete, "athlete", output_format, metadata)
        }
        Err(e) => Ok(UniversalResponse {
            success: false,
            result: None,
            error: Some(format!("Failed to fetch athlete profile: {e}")),
            metadata: None,
        }),
    }
}

/// Process activity analysis when activity is found
async fn process_activity_analysis(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
    activity_id: &str,
    user_uuid: Uuid,
) -> Result<UniversalResponse, ProtocolError> {
    let analysis_response =
        super::intelligence::handle_get_activity_intelligence(executor, request).await?;
    let analysis = analysis_response.result.unwrap_or_else(|| json!({}));

    Ok(UniversalResponse {
        success: true,
        result: Some(to_value(analysis).map_err(|e| {
            ProtocolError::SerializationError(format!("Failed to serialize analysis: {e}"))
        })?),
        error: None,
        metadata: Some(create_activity_metadata(
            activity_id,
            user_uuid,
            analysis_response
                .metadata
                .as_ref()
                .and_then(|m| m.get("tenant_id").and_then(Value::as_str).map(String::from))
                .as_ref(),
        )),
    })
}

/// Handle `get_activities` tool - retrieve user's fitness activities
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn handle_get_activities(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "get_activities cancelled by user".to_owned(),
                ));
            }
        }

        // Parse user ID from request
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Extract provider from request parameters
        let provider_name = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .map_or_else(default_provider, String::from);

        // Extract mode parameter: "summary" (default) or "detailed"
        // Parse mode/format FIRST to determine format-aware default limit
        let mode = request
            .parameters
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("summary");

        // Extract output format parameter: "json" (default) or "toon"
        let output_format = request
            .parameters
            .get("format")
            .and_then(|v| v.as_str())
            .map_or(OutputFormat::Json, OutputFormat::from_str_param);

        // Determine format-aware safe default limit based on mode and format
        // These defaults prevent LLM context overflow when limit is not specified
        // Configurable via SAFE_LIMIT_* environment variables
        let format_aware_default = match (output_format, mode) {
            (OutputFormat::Toon, "summary") => safe_limit_toon_summary(),
            (OutputFormat::Toon, _) => safe_limit_toon_detailed(),
            (OutputFormat::Json, "summary") => safe_limit_json_summary(),
            (OutputFormat::Json, _) => safe_limit_json_detailed(),
        };

        // Extract limit parameter - use format-aware default if not specified
        let user_limit = request.parameters.get("limit").and_then(Value::as_u64);

        let limit = user_limit
            .and_then(|v| usize::try_from(v).ok())
            .unwrap_or(format_aware_default)
            .min(MAX_ACTIVITY_LIMIT);

        // Extract optional offset parameter (handle both integer and float JSON numbers)
        // MCP clients may send numbers as floats (e.g., 100.0 instead of 100)
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let offset = request.parameters.get("offset").and_then(|v| {
            v.as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .or_else(|| v.as_f64().map(|f| f as usize))
        });

        // Extract optional before/after timestamp parameters for time-based filtering
        // Note: We intentionally do NOT apply a default 'after' timestamp because:
        // - Strava API returns activities in reverse chronological order (newest first) by default
        // - Using 'after=X' causes Strava to return the OLDEST activities after X first
        // - The 'limit' parameter already prevents overwhelming LLM context
        // - Users can explicitly pass 'after' if they need time-based filtering
        let before = request.parameters.get("before").and_then(Value::as_i64);
        let after = request.parameters.get("after").and_then(Value::as_i64);
        let default_time_window_applied = false;

        // Extract sport_type filter parameter (case-insensitive)
        let sport_type_filter = request
            .parameters
            .get("sport_type")
            .and_then(|v| v.as_str())
            .map(str::to_owned);

        // Build query params
        let query_params = ActivityQueryParams {
            limit: Some(limit),
            offset,
            before,
            after,
        };

        // Create cache key for activities
        let tenant_uuid = request
            .tenant_id
            .as_ref()
            .and_then(|t| {
                Uuid::parse_str(t)
                    .inspect_err(|e| {
                        debug!(
                            tenant_id_str = %t,
                            error = %e,
                            "Failed to parse tenant ID for activities cache key - using nil UUID"
                        );
                    })
                    .ok()
            })
            .unwrap_or_else(Uuid::nil);

        // For caching activities, include time filters in cache key to avoid
        // returning cached results that don't match the requested time range
        // Safe: limit is bounded by MAX_ACTIVITY_LIMIT which fits in u32
        let per_page = u32::try_from(limit).unwrap_or(DEFAULT_ACTIVITY_LIMIT_U32);
        // Calculate page from offset for cache key (same formula as strava_provider.rs)
        let offset_val = offset.unwrap_or(0);
        #[allow(clippy::cast_possible_truncation)]
        let page = if offset_val > 0 {
            (offset_val / limit + 1) as u32
        } else {
            1
        };
        let cache_key = CacheKey::new(
            tenant_uuid,
            user_uuid,
            provider_name.clone(),
            CacheResource::ActivityList {
                page,
                per_page,
                before,
                after,
            },
        );

        // Create pagination info for response metadata
        // Note: has_more and returned_count are set after we get results
        let create_pagination = |returned_count: usize| PaginationInfo {
            offset: offset.unwrap_or(0),
            limit,
            returned_count,
            has_more: returned_count == limit,
        };

        // Try to get from cache first
        if let Some(cached_response) = try_get_cached_activities(CachedActivitiesParams {
            cache: &executor.resources.cache,
            cache_key: &cache_key,
            user_uuid,
            tenant_id: request.tenant_id.clone(),
            provider_name: &provider_name,
            mode,
            output_format,
            limit,
            offset: offset.unwrap_or(0),
            default_time_window_applied,
        })
        .await
        {
            // Report completion if we got from cache
            if let Some(reporter) = &request.progress_reporter {
                reporter.report(
                    100.0,
                    Some(100.0),
                    Some("Activities loaded from cache".to_owned()),
                );
            }
            return Ok(cached_response);
        }

        // Report progress after cache miss
        if let Some(reporter) = &request.progress_reporter {
            reporter.report(
                25.0,
                Some(100.0),
                Some("Checking authentication...".to_owned()),
            );
        }

        // Check cancellation before expensive auth operation
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "get_activities cancelled before authentication".to_owned(),
                ));
            }
        }

        // Create authenticated provider
        match executor
            .auth_service
            .create_authenticated_provider(&provider_name, user_uuid, request.tenant_id.as_deref())
            .await
        {
            Ok(provider) => {
                // Report progress after successful auth
                if let Some(reporter) = &request.progress_reporter {
                    reporter.report(
                        50.0,
                        Some(100.0),
                        Some(format!(
                            "Authenticated - fetching activities from {provider_name}..."
                        )),
                    );
                }

                // Check cancellation before API call
                if let Some(token) = &request.cancellation_token {
                    if token.is_cancelled().await {
                        return Err(ProtocolError::OperationCancelled(
                            "get_activities cancelled before API call".to_owned(),
                        ));
                    }
                }

                // Get activities from provider with full query params
                match provider.get_activities_with_params(&query_params).await {
                    Ok(activities) => {
                        // Apply sport_type filter if specified (server-side filtering)
                        let mut filtered_activities = filter_activities_by_sport_type(
                            activities,
                            sport_type_filter.as_deref(),
                        );

                        // Sort by start_date descending (newest first) for consistent ordering
                        filtered_activities.sort_by_key(|a| Reverse(a.start_date()));

                        // Report completion
                        if let Some(reporter) = &request.progress_reporter {
                            reporter.report(
                                100.0,
                                Some(100.0),
                                Some(format!(
                                    "Successfully fetched {} activities{}",
                                    filtered_activities.len(),
                                    sport_type_filter
                                        .as_ref()
                                        .map_or(String::new(), |st| format!(" (filtered by {st})"))
                                )),
                            );
                        }

                        // Cache the original unfiltered activities
                        cache_activities_result(
                            &executor.resources.cache,
                            &cache_key,
                            &filtered_activities,
                            per_page,
                        )
                        .await;

                        // Create pagination info for fresh results
                        let pagination = create_pagination(filtered_activities.len());

                        Ok(build_activities_success_response(
                            ActivitiesResponseParams {
                                activities: &filtered_activities,
                                user_uuid,
                                tenant_id: request.tenant_id,
                                provider_name: &provider_name,
                                mode,
                                output_format,
                                pagination: Some(&pagination),
                                default_time_window_applied,
                            },
                        ))
                    }
                    Err(e) => Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Failed to fetch activities: {e}")),
                        metadata: None,
                    }),
                }
            }
            Err(response) => Ok(response),
        }
    })
}

/// Handle `get_athlete` tool - retrieve user's athlete profile
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn handle_get_athlete(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "get_athlete cancelled by user".to_owned(),
                ));
            }
        }

        // Parse user ID from request
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Extract provider from request parameters
        let provider_name = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .map_or_else(default_provider, String::from);

        // Extract output format parameter: "json" (default) or "toon"
        let output_format = extract_output_format(&request);

        // Create cache key for athlete profile
        let tenant_uuid = request
            .tenant_id
            .as_ref()
            .and_then(|t| {
                Uuid::parse_str(t)
                    .inspect_err(|e| {
                        debug!(
                            tenant_id_str = %t,
                            error = %e,
                            "Failed to parse tenant ID for cache key - using nil UUID"
                        );
                    })
                    .ok()
            })
            .unwrap_or_else(Uuid::nil);

        let cache_key = CacheKey::new(
            tenant_uuid,
            user_uuid,
            provider_name.clone(),
            CacheResource::AthleteProfile,
        );

        // Try to get from cache first
        if let Some(cached_response) = try_get_cached_athlete(
            &executor.resources.cache,
            &cache_key,
            user_uuid,
            request.tenant_id.as_ref(),
            output_format,
        )
        .await?
        {
            // Report completion if loaded from cache
            if let Some(reporter) = &request.progress_reporter {
                reporter.report(
                    100.0,
                    Some(100.0),
                    Some("Athlete profile loaded from cache".to_owned()),
                );
            }
            return Ok(cached_response);
        }

        // Report progress after cache miss
        if let Some(reporter) = &request.progress_reporter {
            reporter.report(
                25.0,
                Some(100.0),
                Some("Checking authentication...".to_owned()),
            );
        }

        // Check cancellation before auth
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "get_athlete cancelled before authentication".to_owned(),
                ));
            }
        }

        // Create authenticated provider
        match executor
            .auth_service
            .create_authenticated_provider(&provider_name, user_uuid, request.tenant_id.as_deref())
            .await
        {
            Ok(provider) => {
                // Report progress after auth
                if let Some(reporter) = &request.progress_reporter {
                    reporter.report(
                        50.0,
                        Some(100.0),
                        Some("Authenticated - fetching athlete profile...".to_owned()),
                    );
                }

                // Check cancellation before fetch
                if let Some(token) = &request.cancellation_token {
                    if token.is_cancelled().await {
                        return Err(ProtocolError::OperationCancelled(
                            "get_athlete cancelled before fetch".to_owned(),
                        ));
                    }
                }

                let result = fetch_and_cache_athlete(
                    provider.as_ref(),
                    &executor.resources.cache,
                    &cache_key,
                    user_uuid,
                    request.tenant_id,
                    output_format,
                )
                .await;

                // Report completion on success
                if result.is_ok() {
                    if let Some(reporter) = &request.progress_reporter {
                        reporter.report(
                            100.0,
                            Some(100.0),
                            Some("Athlete profile fetched successfully".to_owned()),
                        );
                    }
                }

                result
            }
            Err(response) => Ok(response),
        }
    })
}

/// Try to get athlete ID from cached athlete profile
async fn try_get_athlete_id_from_cache(
    cache: &Arc<Cache>,
    athlete_cache_key: &CacheKey,
) -> Option<u64> {
    if let Ok(Some(athlete)) = cache.get::<Athlete>(athlete_cache_key).await {
        return athlete
            .id
            .parse::<u64>()
            .inspect_err(|e| {
                debug!(
                    athlete_id_str = %athlete.id,
                    error = %e,
                    "Failed to parse athlete ID from cache as u64"
                );
            })
            .ok();
    }
    None
}

/// Try to get stats from cache
async fn try_get_cached_stats(
    cache: &Arc<Cache>,
    stats_cache_key: &CacheKey,
    user_uuid: Uuid,
    tenant_id: Option<&String>,
    output_format: OutputFormat,
) -> Result<Option<UniversalResponse>, ProtocolError> {
    if let Ok(Some(cached_stats)) = cache.get::<Stats>(stats_cache_key).await {
        info!("Cache hit for stats");
        let mut metadata = HashMap::new();
        metadata.insert("user_id".to_owned(), Value::String(user_uuid.to_string()));
        metadata.insert(
            "tenant_id".to_owned(),
            tenant_id.map_or(Value::Null, |id| Value::String(id.clone())),
        );
        metadata.insert("cached".to_owned(), Value::Bool(true));

        return Ok(Some(build_formatted_response(
            &cached_stats,
            "stats",
            output_format,
            metadata,
        )?));
    }
    info!("Cache miss for stats");
    Ok(None)
}

/// Create metadata for stats responses
fn create_stats_metadata(
    user_uuid: Uuid,
    tenant_uuid: Uuid,
    cached: bool,
) -> HashMap<String, Value> {
    let mut map = HashMap::new();
    map.insert("user_id".to_owned(), Value::String(user_uuid.to_string()));
    map.insert(
        "tenant_id".to_owned(),
        Value::String(tenant_uuid.to_string()),
    );
    map.insert("cached".to_owned(), Value::Bool(cached));
    map
}

/// Cache a single item with TTL, logging errors
async fn cache_item<T: serde::Serialize + Send + Sync>(
    cache: &Arc<Cache>,
    key: &CacheKey,
    item: &T,
    ttl: Duration,
    item_name: &str,
) {
    if let Err(e) = cache.set(key, item, ttl).await {
        warn!("Failed to cache {}: {}", item_name, e);
    }
}

/// Cache athlete and stats data
async fn cache_athlete_and_stats(
    cache: &Arc<Cache>,
    athlete_cache_key: &CacheKey,
    athlete: &Athlete,
    stats: &Stats,
    tenant_uuid: Uuid,
    user_uuid: Uuid,
    provider_name: &str,
) {
    let Some(athlete_id) = athlete
        .id
        .parse::<u64>()
        .inspect_err(|e| debug!("Failed to parse athlete ID: {e}"))
        .ok()
    else {
        return;
    };

    // Cache athlete
    let athlete_ttl = CacheResource::AthleteProfile.recommended_ttl();
    cache_item(cache, athlete_cache_key, athlete, athlete_ttl, "athlete").await;

    // Cache stats
    let stats_cache_key = CacheKey::new(
        tenant_uuid,
        user_uuid,
        provider_name.to_owned(),
        CacheResource::Stats { athlete_id },
    );
    let stats_ttl = CacheResource::Stats { athlete_id }.recommended_ttl();
    cache_item(cache, &stats_cache_key, stats, stats_ttl, "stats").await;
    info!("Cached stats with TTL {:?}", stats_ttl);
}

/// Fetch stats from API and cache both athlete and stats
async fn fetch_and_cache_stats(
    provider: &dyn FitnessProvider,
    cache: &Arc<Cache>,
    athlete_cache_key: &CacheKey,
    tenant_uuid: Uuid,
    user_uuid: Uuid,
    provider_name: &str,
    output_format: OutputFormat,
) -> Result<UniversalResponse, ProtocolError> {
    let stats = match provider.get_stats().await {
        Ok(stats) => stats,
        Err(e) => {
            return Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Failed to fetch stats: {e}")),
                metadata: None,
            });
        }
    };

    // Get athlete to extract athlete_id for caching
    if let Ok(athlete) = provider.get_athlete().await {
        cache_athlete_and_stats(
            cache,
            athlete_cache_key,
            &athlete,
            &stats,
            tenant_uuid,
            user_uuid,
            provider_name,
        )
        .await;
    }

    let metadata = create_stats_metadata(user_uuid, tenant_uuid, false);
    build_formatted_response(&stats, "stats", output_format, metadata)
}

/// Handle `get_stats` tool - retrieve user's activity statistics
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn handle_get_stats(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "get_stats cancelled by user".to_owned(),
                ));
            }
        }

        // Parse user ID from request
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Extract provider from request parameters
        let provider_name = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .map_or_else(default_provider, String::from);

        // Extract output format parameter: "json" (default) or "toon"
        let output_format = extract_output_format(&request);

        // Create cache key for stats (need athlete_id from athlete profile)
        let tenant_uuid = request
            .tenant_id
            .as_ref()
            .and_then(|t| {
                Uuid::parse_str(t)
                    .inspect_err(|e| {
                        debug!(
                            tenant_id_str = %t,
                            error = %e,
                            "Failed to parse tenant ID for cache key - using nil UUID"
                        );
                    })
                    .ok()
            })
            .unwrap_or_else(Uuid::nil);

        let athlete_cache_key = CacheKey::new(
            tenant_uuid,
            user_uuid,
            provider_name.clone(),
            CacheResource::AthleteProfile,
        );

        // Try to get athlete_id from cache and then stats
        if let Some(athlete_id) =
            try_get_athlete_id_from_cache(&executor.resources.cache, &athlete_cache_key).await
        {
            let stats_cache_key = CacheKey::new(
                tenant_uuid,
                user_uuid,
                provider_name.clone(),
                CacheResource::Stats { athlete_id },
            );

            if let Some(cached_response) = try_get_cached_stats(
                &executor.resources.cache,
                &stats_cache_key,
                user_uuid,
                request.tenant_id.as_ref(),
                output_format,
            )
            .await?
            {
                // Report completion if loaded from cache
                if let Some(reporter) = &request.progress_reporter {
                    reporter.report(
                        100.0,
                        Some(100.0),
                        Some("Stats loaded from cache".to_owned()),
                    );
                }
                return Ok(cached_response);
            }
        }

        // Report progress after cache miss
        if let Some(reporter) = &request.progress_reporter {
            reporter.report(
                25.0,
                Some(100.0),
                Some("Checking authentication...".to_owned()),
            );
        }

        // Check cancellation before auth
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "get_stats cancelled before authentication".to_owned(),
                ));
            }
        }

        // Create authenticated provider
        match executor
            .auth_service
            .create_authenticated_provider(&provider_name, user_uuid, request.tenant_id.as_deref())
            .await
        {
            Ok(provider) => {
                // Report progress after auth
                if let Some(reporter) = &request.progress_reporter {
                    reporter.report(
                        50.0,
                        Some(100.0),
                        Some("Authenticated - fetching stats...".to_owned()),
                    );
                }

                // Check cancellation before fetch
                if let Some(token) = &request.cancellation_token {
                    if token.is_cancelled().await {
                        return Err(ProtocolError::OperationCancelled(
                            "get_stats cancelled before fetch".to_owned(),
                        ));
                    }
                }

                let result = fetch_and_cache_stats(
                    provider.as_ref(),
                    &executor.resources.cache,
                    &athlete_cache_key,
                    tenant_uuid,
                    user_uuid,
                    &provider_name,
                    output_format,
                )
                .await;

                // Report completion on success
                if result.is_ok() {
                    if let Some(reporter) = &request.progress_reporter {
                        reporter.report(
                            100.0,
                            Some(100.0),
                            Some("Stats fetched successfully".to_owned()),
                        );
                    }
                }

                result
            }
            Err(response) => Ok(response),
        }
    })
}

/// Handle `analyze_activity` tool - analyze specific activity with intelligence
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn handle_analyze_activity(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "analyze_activity cancelled by user".to_owned(),
                ));
            }
        }

        // Parse user ID and extract activity ID from request
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Extract provider from request parameters
        let provider_name = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .map_or_else(default_provider, String::from);

        let activity_id = request
            .parameters
            .get("activity_id")
            .and_then(Value::as_str)
            .map(str::to_owned) // Safe: String ownership needed to avoid borrowing issues
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("activity_id parameter required".to_owned())
            })?;

        // Report progress - starting authentication
        if let Some(reporter) = &request.progress_reporter {
            reporter.report(
                20.0,
                Some(100.0),
                Some("Checking authentication...".to_owned()),
            );
        }

        // Check cancellation before auth
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "analyze_activity cancelled before authentication".to_owned(),
                ));
            }
        }

        // Create authenticated provider
        match executor
            .auth_service
            .create_authenticated_provider(&provider_name, user_uuid, request.tenant_id.as_deref())
            .await
        {
            Ok(provider) => {
                // Report progress after auth
                if let Some(reporter) = &request.progress_reporter {
                    reporter.report(
                        40.0,
                        Some(100.0),
                        Some("Authenticated - fetching activity...".to_owned()),
                    );
                }

                // Check cancellation before provider creation
                if let Some(token) = &request.cancellation_token {
                    if token.is_cancelled().await {
                        return Err(ProtocolError::OperationCancelled(
                            "analyze_activity cancelled before fetch".to_owned(),
                        ));
                    }
                }

                // Fetch the specific activity directly - efficient single API call
                match provider.get_activity(&activity_id).await {
                    Ok(_activity) => {
                        // Report progress before analysis
                        if let Some(reporter) = &request.progress_reporter {
                            reporter.report(
                                60.0,
                                Some(100.0),
                                Some("Activity retrieved - analyzing...".to_owned()),
                            );
                        }

                        // Activity found - process analysis
                        // Note: process_activity_analysis takes ownership of request
                        process_activity_analysis(executor, request, &activity_id, user_uuid).await
                    }
                    Err(e) => {
                        // Activity not found or API error
                        Ok(UniversalResponse {
                            success: false,
                            result: None,
                            error: Some(format!("Activity {activity_id} not found: {e}")),
                            metadata: Some({
                                let mut map = HashMap::new();
                                map.insert(
                                    "activity_id".to_owned(),
                                    Value::String(activity_id.clone()),
                                );
                                map.insert(
                                    "provider".to_owned(),
                                    Value::String(provider_name.clone()),
                                );
                                map
                            }),
                        })
                    }
                }
            }
            Err(response) => Ok(response),
        }
    })
}
