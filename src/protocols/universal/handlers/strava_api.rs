// ABOUTME: Strava API handlers for universal protocol
// ABOUTME: Single responsibility handlers that delegate auth to AuthService
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::constants::oauth_providers;
use crate::intelligence::physiological_constants::api_limits::{
    DEFAULT_ACTIVITY_LIMIT, MAX_ACTIVITY_LIMIT,
};
use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use crate::protocols::ProtocolError;
use crate::providers::core::FitnessProvider;
use crate::utils::uuid::parse_user_id_for_protocol;
use std::future::Future;
use std::pin::Pin;

/// Create and configure Strava provider with token credentials
async fn create_configured_strava_provider(
    token_data: &crate::oauth::TokenData,
) -> Result<Box<dyn FitnessProvider>, String> {
    let mut provider = crate::providers::create_provider(oauth_providers::STRAVA)
        .map_err(|e| format!("Failed to create provider: {e}"))?;

    let credentials = crate::providers::OAuth2Credentials {
        client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
        client_secret: std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default(),
        access_token: Some(token_data.access_token.clone()), // Safe: String ownership needed for OAuth credentials
        refresh_token: Some(token_data.refresh_token.clone()), // Safe: String ownership needed for OAuth credentials
        expires_at: Some(token_data.expires_at),
        scopes: crate::constants::oauth::STRAVA_DEFAULT_SCOPES
            .split(',')
            .map(str::to_string)
            .collect(),
    };

    provider
        .set_credentials(credentials)
        .await
        .map_err(|e| format!("Failed to set provider credentials: {e}"))?;

    Ok(provider)
}

/// Create standard no-token response
fn create_no_token_response() -> UniversalResponse {
    UniversalResponse {
        success: false,
        result: None,
        error: Some("No valid Strava token found. Please connect your Strava account using the connect_provider tool with provider='strava'.".to_string()),
        metadata: Some({
            let mut map = std::collections::HashMap::new();
            map.insert(
                "total_activities".to_string(),
                serde_json::Value::Number(0.into()),
            );
            map.insert(
                "authentication_required".to_string(),
                serde_json::Value::Bool(true),
            );
            map
        }),
    }
}

/// Create standard auth error response
fn create_auth_error_response(error: &str) -> UniversalResponse {
    UniversalResponse {
        success: true,
        result: Some(serde_json::json!({
            "activities": [],
            "message": format!("Authentication error: {error}"),
            "error": format!("Authentication error: {error}")
        })),
        error: None,
        metadata: Some({
            let mut map = std::collections::HashMap::new();
            map.insert(
                "authentication_error".to_string(),
                serde_json::Value::Bool(true),
            );
            map
        }),
    }
}

/// Create metadata for activity analysis responses
fn create_activity_metadata(
    activity_id: &str,
    user_uuid: uuid::Uuid,
    tenant_id: Option<&String>,
) -> std::collections::HashMap<String, serde_json::Value> {
    let mut map = std::collections::HashMap::new();
    map.insert(
        "activity_id".to_string(),
        serde_json::Value::String(activity_id.to_string()),
    );
    map.insert(
        "user_id".to_string(),
        serde_json::Value::String(user_uuid.to_string()),
    );
    map.insert(
        "tenant_id".to_string(),
        tenant_id.map_or(serde_json::Value::Null, |id| {
            serde_json::Value::String(id.clone()) // Safe: String ownership for JSON value
        }),
    );
    map
}

/// Process activity analysis when activity is found
async fn process_activity_analysis(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
    activity_id: &str,
    user_uuid: uuid::Uuid,
) -> Result<UniversalResponse, ProtocolError> {
    let analysis_response =
        super::intelligence::handle_get_activity_intelligence(executor, request).await?;
    let analysis = analysis_response
        .result
        .unwrap_or_else(|| serde_json::json!({}));

    Ok(UniversalResponse {
        success: true,
        result: Some(serde_json::to_value(analysis).map_err(|e| {
            ProtocolError::SerializationError(format!("Failed to serialize analysis: {e}"))
        })?),
        error: None,
        metadata: Some(create_activity_metadata(
            activity_id,
            user_uuid,
            analysis_response
                .metadata
                .as_ref()
                .and_then(|m| {
                    m.get("tenant_id")
                        .and_then(serde_json::Value::as_str)
                        .map(String::from)
                })
                .as_ref(),
        )),
    })
}

/// Handle `get_activities` tool - retrieve user's fitness activities
#[must_use]
pub fn handle_get_activities(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Parse user ID from request
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Extract limit parameter with bounds checking
        let requested_limit = request
            .parameters
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(10)
            .try_into()
            .unwrap_or(DEFAULT_ACTIVITY_LIMIT);

        let limit = requested_limit.min(MAX_ACTIVITY_LIMIT);

        // Get valid Strava token (with automatic refresh if needed)
        match executor
            .auth_service
            .get_valid_token(
                user_uuid,
                oauth_providers::STRAVA,
                request.tenant_id.as_deref(),
            )
            .await
        {
            Ok(Some(token_data)) => {
                // Create and configure Strava provider
                match create_configured_strava_provider(&token_data).await {
                    Ok(provider) => {
                        // Get activities from provider
                        match provider.get_activities(Some(limit), None).await {
                            Ok(activities) => Ok(UniversalResponse {
                                success: true,
                                result: Some(serde_json::json!({
                                    "activities": activities,
                                    "provider": "strava",
                                    "count": activities.len()
                                })),
                                error: None,
                                metadata: Some({
                                    let mut map = std::collections::HashMap::new();
                                    map.insert(
                                        "total_activities".to_string(),
                                        serde_json::Value::Number(activities.len().into()),
                                    );
                                    map.insert(
                                        "user_id".to_string(),
                                        serde_json::Value::String(user_uuid.to_string()),
                                    );
                                    map.insert(
                                        "tenant_id".to_string(),
                                        request.tenant_id.map_or(
                                            serde_json::Value::Null,
                                            serde_json::Value::String,
                                        ),
                                    );
                                    map
                                }),
                            }),
                            Err(e) => Ok(UniversalResponse {
                                success: false,
                                result: None,
                                error: Some(format!("Failed to fetch activities: {e}")),
                                metadata: None,
                            }),
                        }
                    }
                    Err(e) => Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(e),
                        metadata: None,
                    }),
                }
            }
            Ok(None) => Ok(create_no_token_response()),
            Err(e) => Ok(create_auth_error_response(&e.to_string())),
        }
    })
}

/// Handle `get_athlete` tool - retrieve user's athlete profile
#[must_use]
pub fn handle_get_athlete(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Parse user ID from request
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Get valid Strava token (with automatic refresh if needed)
        match executor
            .auth_service
            .get_valid_token(
                user_uuid,
                oauth_providers::STRAVA,
                request.tenant_id.as_deref(),
            )
            .await
        {
            Ok(Some(token_data)) => {
                // Create Strava provider with token
                match crate::providers::create_provider(oauth_providers::STRAVA) {
                    Ok(mut provider) => {
                        // Set credentials using the token data
                        let credentials = crate::providers::OAuth2Credentials {
                            client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                            client_secret: std::env::var("STRAVA_CLIENT_SECRET")
                                .unwrap_or_default(),
                            access_token: Some(token_data.access_token),
                            refresh_token: Some(token_data.refresh_token),
                            expires_at: Some(token_data.expires_at),
                            scopes: crate::constants::oauth::STRAVA_DEFAULT_SCOPES
                                .split(',')
                                .map(str::to_string)
                                .collect(),
                        };

                        if let Err(e) = provider.set_credentials(credentials).await {
                            return Ok(UniversalResponse {
                                success: false,
                                result: None,
                                error: Some(format!("Failed to set provider credentials: {e}")),
                                metadata: None,
                            });
                        }

                        // Get athlete profile from provider
                        match provider.get_athlete().await {
                            Ok(athlete) => Ok(UniversalResponse {
                                success: true,
                                result: Some(serde_json::to_value(&athlete).map_err(|e| {
                                    ProtocolError::SerializationError(format!(
                                        "Failed to serialize athlete: {e}"
                                    ))
                                })?),
                                error: None,
                                metadata: Some({
                                    let mut map = std::collections::HashMap::new();
                                    map.insert(
                                        "user_id".to_string(),
                                        serde_json::Value::String(user_uuid.to_string()),
                                    );
                                    map.insert(
                                        "tenant_id".to_string(),
                                        request.tenant_id.map_or(
                                            serde_json::Value::Null,
                                            serde_json::Value::String,
                                        ),
                                    );
                                    map
                                }),
                            }),
                            Err(e) => Ok(UniversalResponse {
                                success: false,
                                result: None,
                                error: Some(format!("Failed to fetch athlete profile: {e}")),
                                metadata: None,
                            }),
                        }
                    }
                    Err(e) => Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(e.to_string()),
                        metadata: None,
                    }),
                }
            }
            Ok(None) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(
                    "No valid Strava token found. Please connect your Strava account.".to_string(),
                ),
                metadata: None,
            }),
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Authentication error: {e}")),
                metadata: None,
            }),
        }
    })
}

/// Handle `get_stats` tool - retrieve user's activity statistics
#[must_use]
pub fn handle_get_stats(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Parse user ID from request
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Get valid Strava token (with automatic refresh if needed)
        match executor
            .auth_service
            .get_valid_token(
                user_uuid,
                oauth_providers::STRAVA,
                request.tenant_id.as_deref(),
            )
            .await
        {
            Ok(Some(token_data)) => {
                // Create Strava provider with token
                match crate::providers::create_provider(oauth_providers::STRAVA) {
                    Ok(mut provider) => {
                        // Set credentials using the token data
                        let credentials = crate::providers::OAuth2Credentials {
                            client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                            client_secret: std::env::var("STRAVA_CLIENT_SECRET")
                                .unwrap_or_default(),
                            access_token: Some(token_data.access_token),
                            refresh_token: Some(token_data.refresh_token),
                            expires_at: Some(token_data.expires_at),
                            scopes: crate::constants::oauth::STRAVA_DEFAULT_SCOPES
                                .split(',')
                                .map(str::to_string)
                                .collect(),
                        };

                        if let Err(e) = provider.set_credentials(credentials).await {
                            return Ok(UniversalResponse {
                                success: false,
                                result: None,
                                error: Some(format!("Failed to set provider credentials: {e}")),
                                metadata: None,
                            });
                        }

                        // Get stats from provider
                        match provider.get_stats().await {
                            Ok(stats) => Ok(UniversalResponse {
                                success: true,
                                result: Some(serde_json::to_value(&stats).map_err(|e| {
                                    ProtocolError::SerializationError(format!(
                                        "Failed to serialize stats: {e}"
                                    ))
                                })?),
                                error: None,
                                metadata: Some({
                                    let mut map = std::collections::HashMap::new();
                                    map.insert(
                                        "user_id".to_string(),
                                        serde_json::Value::String(user_uuid.to_string()),
                                    );
                                    map.insert(
                                        "tenant_id".to_string(),
                                        request.tenant_id.map_or(
                                            serde_json::Value::Null,
                                            serde_json::Value::String,
                                        ),
                                    );
                                    map
                                }),
                            }),
                            Err(e) => Ok(UniversalResponse {
                                success: false,
                                result: None,
                                error: Some(format!("Failed to fetch stats: {e}")),
                                metadata: None,
                            }),
                        }
                    }
                    Err(e) => Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(e.to_string()),
                        metadata: None,
                    }),
                }
            }
            Ok(None) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(
                    "No valid Strava token found. Please connect your Strava account.".to_string(),
                ),
                metadata: None,
            }),
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Authentication error: {e}")),
                metadata: None,
            }),
        }
    })
}

/// Handle `analyze_activity` tool - analyze specific activity with intelligence
#[must_use]
pub fn handle_analyze_activity(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Parse user ID and extract activity ID from request
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;
        let activity_id = request
            .parameters
            .get("activity_id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string) // Safe: String ownership needed to avoid borrowing issues
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("activity_id parameter required".to_string())
            })?;

        // Get valid Strava token (with automatic refresh if needed)
        match executor
            .auth_service
            .get_valid_token(
                user_uuid,
                oauth_providers::STRAVA,
                request.tenant_id.as_deref(),
            )
            .await
        {
            Ok(Some(token_data)) => {
                // Create and configure Strava provider
                match create_configured_strava_provider(&token_data).await {
                    Ok(provider) => {
                        // Get activities to find the target activity
                        match provider
                            .get_activities(Some(DEFAULT_ACTIVITY_LIMIT), None)
                            .await
                        {
                            Ok(activities) => {
                                if activities.iter().any(|a| a.id == activity_id) {
                                    // Activity found - process analysis
                                    process_activity_analysis(
                                        executor,
                                        request,
                                        &activity_id,
                                        user_uuid,
                                    )
                                    .await
                                } else {
                                    // Activity not found
                                    Ok(UniversalResponse {
                                        success: false,
                                        result: None,
                                        error: Some(format!("Activity {activity_id} not found")),
                                        metadata: Some({
                                            let mut map = std::collections::HashMap::new();
                                            map.insert(
                                                "activity_id".to_string(),
                                                serde_json::Value::String(activity_id.to_string()),
                                            );
                                            map.insert(
                                                "provider".to_string(),
                                                serde_json::Value::String("strava".to_string()),
                                            );
                                            map
                                        }),
                                    })
                                }
                            }
                            Err(e) => Ok(UniversalResponse {
                                success: false,
                                result: None,
                                error: Some(format!("Failed to fetch activities: {e}")),
                                metadata: None,
                            }),
                        }
                    }
                    Err(e) => Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(e),
                        metadata: None,
                    }),
                }
            }
            Ok(None) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(
                    "No valid Strava token found. Please connect your Strava account.".to_string(),
                ),
                metadata: None,
            }),
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Authentication error: {e}")),
                metadata: None,
            }),
        }
    })
}
