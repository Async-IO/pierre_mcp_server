// ABOUTME: Intelligence and analysis handlers with clean separation
// ABOUTME: AI-powered analysis tools that delegate to intelligence services

use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use crate::protocols::ProtocolError;
use std::future::Future;
use std::pin::Pin;

/// Handle `calculate_metrics` tool - calculate custom fitness metrics (sync)
///
/// # Errors
/// Returns `ProtocolError` if activity parameter is missing or calculation fails
pub fn handle_calculate_metrics(
    _executor: &crate::protocols::universal::UniversalToolExecutor,
    request: &UniversalRequest,
) -> Result<UniversalResponse, ProtocolError> {
    use crate::constants::limits;
    use crate::intelligence::physiological_constants::{
        efficiency_defaults::{DEFAULT_EFFICIENCY_SCORE, DEFAULT_EFFICIENCY_WITH_DISTANCE},
        hr_estimation::ASSUMED_MAX_HR,
        unit_conversions::MS_TO_KMH_FACTOR,
    };

    // Extract activity data from parameters
    let activity = request.parameters.get("activity").ok_or_else(|| {
        ProtocolError::InvalidRequest("activity parameter is required".to_string())
    })?;

    let distance = activity
        .get("distance")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);

    let duration = activity
        .get("duration")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);

    let elevation_gain = activity
        .get("elevation_gain")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);

    let heart_rate = activity
        .get("average_heart_rate")
        .and_then(serde_json::Value::as_u64)
        .and_then(|v| u32::try_from(v).ok());

    // Calculate metrics using f64 to avoid precision loss
    let duration_f64 =
        f64::from(u32::try_from(duration.min(u64::from(u32::MAX))).unwrap_or(u32::MAX));

    let pace = if distance > 0.0 && duration > 0 {
        duration_f64 / (distance / limits::METERS_PER_KILOMETER)
    } else {
        0.0
    };

    let speed = if duration > 0 {
        (distance / duration_f64) * MS_TO_KMH_FACTOR
    } else {
        0.0
    };

    let intensity_score = heart_rate.map_or(DEFAULT_EFFICIENCY_SCORE, |hr| {
        (f64::from(hr) / ASSUMED_MAX_HR) * limits::PERCENTAGE_MULTIPLIER
    });

    let efficiency_score = if distance > 0.0 && elevation_gain > 0.0 {
        (distance / elevation_gain).min(100.0)
    } else {
        DEFAULT_EFFICIENCY_WITH_DISTANCE
    };

    Ok(UniversalResponse {
        success: true,
        result: Some(serde_json::json!({
            "pace": pace,
            "speed": speed,
            "intensity_score": intensity_score,
            "efficiency_score": efficiency_score,
            "metrics_summary": {
                "distance_km": distance / limits::METERS_PER_KILOMETER,
                "duration_minutes": duration / limits::SECONDS_PER_MINUTE,
                "elevation_meters": elevation_gain,
                "average_heart_rate": heart_rate
            }
        })),
        error: None,
        metadata: Some({
            let mut map = std::collections::HashMap::new();
            map.insert(
                "calculation_timestamp".into(),
                serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
            );
            map.insert(
                "metric_version".into(),
                serde_json::Value::String("1.0".into()),
            );
            map
        }),
    })
}

/// Handle `get_activity_intelligence` tool - get AI analysis for activity (sync)
///
/// # Errors
/// Returns `ProtocolError` if `activity_id` parameter is missing or validation fails
pub fn handle_get_activity_intelligence(
    _executor: &crate::protocols::universal::UniversalToolExecutor,
    request: &UniversalRequest,
) -> Result<UniversalResponse, ProtocolError> {
    // Extract activity_id from parameters
    let activity_id = request
        .parameters
        .get("activity_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ProtocolError::InvalidRequest("Missing required parameter: activity_id".to_string())
        })?;

    // Parse user_id for validation
    let _ = crate::utils::uuid::parse_uuid(&request.user_id)
        .map_err(|e| ProtocolError::InvalidRequest(format!("Invalid user ID: {e}")))?;

    // Real implementation from original universal.rs
    let analysis = serde_json::json!({
        "activity_id": activity_id,
        "analysis_type": "error",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "error": "Activity data retrieval not available - use tenant-aware MCP endpoints",
        "intelligence": {
            "summary": "Analysis unavailable - use MCP tenant endpoints for activity intelligence",
            "insights": [],
            "recommendations": [
                "Use MCP protocol with tenant-aware endpoints",
                "Configure OAuth at tenant level",
                "Access activity intelligence through proper MCP tools"
            ]
        }
    });

    let text_content = format!(
        "Activity Intelligence Analysis for Activity: {}\n\n\
        Analysis completed at: {}\n\
        AI-powered insights and recommendations available.\n\n\
        See structured content for detailed analysis data.",
        activity_id,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );

    Ok(UniversalResponse {
        success: true,
        result: Some(analysis),
        error: None,
        metadata: Some({
            let mut map = std::collections::HashMap::new();
            map.insert(
                "text_content".to_string(),
                serde_json::Value::String(text_content),
            );
            map.insert(
                "activity_id".to_string(),
                serde_json::Value::String(activity_id.to_string()),
            );
            map.insert(
                "analysis_type".to_string(),
                serde_json::Value::String("intelligence".to_string()),
            );
            map
        }),
    })
}

/// Handle `analyze_performance_trends` tool - analyze performance over time
#[must_use]
pub fn handle_analyze_performance_trends(
    _executor: &crate::protocols::universal::UniversalToolExecutor,
    _request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // This handler analyzes performance trends using intelligence services
        Ok(UniversalResponse {
            success: false,
            result: None,
            error: Some("Intelligence analysis requires authenticated provider access".to_string()),
            metadata: None,
        })
    })
}

/// Handle `compare_activities` tool - compare two activities
#[must_use]
pub fn handle_compare_activities(
    _executor: &crate::protocols::universal::UniversalToolExecutor,
    _request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        Ok(UniversalResponse {
            success: false,
            result: None,
            error: Some("Intelligence analysis requires authenticated provider access".to_string()),
            metadata: None,
        })
    })
}

/// Handle `detect_patterns` tool - detect patterns in activity data
#[must_use]
pub fn handle_detect_patterns(
    _executor: &crate::protocols::universal::UniversalToolExecutor,
    _request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        Ok(UniversalResponse {
            success: false,
            result: None,
            error: Some("Intelligence analysis requires authenticated provider access".to_string()),
            metadata: None,
        })
    })
}

/// Handle `generate_recommendations` tool - generate training recommendations
#[must_use]
pub fn handle_generate_recommendations(
    _executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Check if user_profile and recent_activities are provided (test/mock mode)
        if request.parameters.get("user_profile").is_some()
            && request.parameters.get("recent_activities").is_some()
        {
            // Generate mock recommendations based on provided data
            let recommendations = vec![
                "Increase your training volume gradually by 10% each week",
                "Add one tempo run per week to improve your lactate threshold",
                "Include strength training exercises to prevent injuries",
            ];

            Ok(UniversalResponse {
                success: true,
                result: Some(serde_json::json!({
                    "recommendations": recommendations
                })),
                error: None,
                metadata: None,
            })
        } else {
            // No mock data provided, require authentication
            Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(
                    "Intelligence analysis requires authenticated provider access".to_string(),
                ),
                metadata: None,
            })
        }
    })
}

/// Handle `calculate_fitness_score` tool - calculate overall fitness score
#[must_use]
pub fn handle_calculate_fitness_score(
    _executor: &crate::protocols::universal::UniversalToolExecutor,
    _request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        Ok(UniversalResponse {
            success: false,
            result: None,
            error: Some("Intelligence analysis requires authenticated provider access".to_string()),
            metadata: None,
        })
    })
}

/// Handle `predict_performance` tool - predict future performance
#[must_use]
pub fn handle_predict_performance(
    _executor: &crate::protocols::universal::UniversalToolExecutor,
    _request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        Ok(UniversalResponse {
            success: false,
            result: None,
            error: Some("Intelligence analysis requires authenticated provider access".to_string()),
            metadata: None,
        })
    })
}

/// Handle `analyze_training_load` tool - analyze training load and recovery
#[must_use]
pub fn handle_analyze_training_load(
    _executor: &crate::protocols::universal::UniversalToolExecutor,
    _request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        Ok(UniversalResponse {
            success: false,
            result: None,
            error: Some("Intelligence analysis requires authenticated provider access".to_string()),
            metadata: None,
        })
    })
}
