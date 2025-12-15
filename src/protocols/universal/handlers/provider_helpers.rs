// ABOUTME: Shared helper functions for provider-agnostic handler operations
// ABOUTME: Consolidates provider extraction, configuration, and creation logic
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::config::environment::{default_provider, get_oauth_config, OAuthProviderConfig};
use crate::models::Activity;
use crate::protocols::universal::auth_service::TokenData;
use crate::protocols::universal::{UniversalResponse, UniversalToolExecutor};
use crate::providers::core::FitnessProvider;
use crate::providers::{OAuth2Credentials, ProviderRegistry};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

/// Extract provider name from request parameters, falling back to default provider
///
/// Returns the provider name from request parameters if specified, otherwise returns
/// the configured default provider (from `PIERRE_DEFAULT_PROVIDER` env var or "synthetic").
#[must_use]
pub fn extract_provider(parameters: &serde_json::Map<String, JsonValue>) -> String {
    parameters
        .get("provider")
        .and_then(JsonValue::as_str)
        .map_or_else(default_provider, String::from)
}

/// Create and configure a provider with OAuth credentials
///
/// This is the provider-agnostic version of provider creation that works with
/// any registered provider. It:
/// 1. Creates a provider instance from the registry
/// 2. Loads provider-specific OAuth configuration
/// 3. Sets credentials from the token data
///
/// # Arguments
/// * `provider_name` - Name of the provider (e.g., "strava", "garmin")
/// * `provider_registry` - Registry for creating provider instances
/// * `token_data` - OAuth token data for authentication
///
/// # Errors
/// Returns an error string if provider creation fails or credentials cannot be set.
pub async fn create_configured_provider(
    provider_name: &str,
    provider_registry: &Arc<ProviderRegistry>,
    token_data: &TokenData,
) -> Result<Box<dyn FitnessProvider>, String> {
    // Create provider instance
    let provider = provider_registry
        .create_provider(provider_name)
        .map_err(|e| format!("Failed to create {provider_name} provider: {e}"))?;

    // Load provider-specific OAuth config
    let config = get_oauth_config(provider_name);

    // Build credentials
    let credentials = OAuth2Credentials {
        client_id: config.client_id.clone().unwrap_or_default(),
        client_secret: config.client_secret.clone().unwrap_or_default(),
        access_token: Some(token_data.access_token.clone()),
        refresh_token: Some(token_data.refresh_token.clone()),
        expires_at: Some(token_data.expires_at),
        scopes: config.scopes.clone(),
    };

    // Set credentials on provider
    provider
        .set_credentials(credentials)
        .await
        .map_err(|e| format!("Failed to set {provider_name} provider credentials: {e}"))?;

    Ok(provider)
}

/// Create a no-token response for a specific provider
///
/// Returns a standardized response when no OAuth token is found for the provider.
#[must_use]
pub fn create_no_token_response(provider_name: &str) -> UniversalResponse {
    UniversalResponse {
        success: false,
        result: None,
        error: Some(format!(
            "No valid {provider_name} token found. Please connect your account using the connect_provider tool with provider='{provider_name}'."
        )),
        metadata: Some({
            let mut map = HashMap::new();
            map.insert(
                "total_activities".to_owned(),
                JsonValue::Number(0.into()),
            );
            map.insert(
                "authentication_required".to_owned(),
                JsonValue::Bool(true),
            );
            map.insert(
                "provider".to_owned(),
                JsonValue::String(provider_name.to_owned()),
            );
            map
        }),
    }
}

/// Create a standard auth error response
#[must_use]
pub fn create_auth_error_response(provider_name: &str, error: &str) -> UniversalResponse {
    UniversalResponse {
        success: true,
        result: Some(json!({
            "activities": [],
            "message": format!("Authentication error for {provider_name}: {error}"),
            "error": format!("Authentication error: {error}"),
            "provider": provider_name
        })),
        error: None,
        metadata: Some({
            let mut map = HashMap::new();
            map.insert("authentication_error".to_owned(), JsonValue::Bool(true));
            map.insert(
                "provider".to_owned(),
                JsonValue::String(provider_name.to_owned()),
            );
            map
        }),
    }
}

/// Build success response for activities from any provider
pub fn build_activities_success_response(
    activities: &[Activity],
    provider_name: &str,
    user_uuid: uuid::Uuid,
    tenant_id: Option<String>,
) -> UniversalResponse {
    UniversalResponse {
        success: true,
        result: Some(json!({
            "activities": activities,
            "provider": provider_name,
            "count": activities.len()
        })),
        error: None,
        metadata: Some({
            let mut map = HashMap::new();
            map.insert(
                "total_activities".to_owned(),
                JsonValue::Number(activities.len().into()),
            );
            map.insert(
                "user_id".to_owned(),
                JsonValue::String(user_uuid.to_string()),
            );
            map.insert(
                "tenant_id".to_owned(),
                tenant_id.map_or(JsonValue::Null, JsonValue::String),
            );
            map.insert(
                "provider".to_owned(),
                JsonValue::String(provider_name.to_owned()),
            );
            map.insert("cached".to_owned(), JsonValue::Bool(false));
            map
        }),
    }
}

/// Get OAuth config for a provider, with logging
pub fn get_provider_oauth_config(provider_name: &str) -> OAuthProviderConfig {
    let config = get_oauth_config(provider_name);
    debug!(
        provider = provider_name,
        has_client_id = config.client_id.is_some(),
        "Loaded OAuth config for provider"
    );
    config
}

/// Fetch activities from any supported provider
///
/// This is a provider-agnostic activity fetcher that handles:
/// - Provider validation
/// - OAuth token retrieval and validation
/// - Provider creation and credential setup
/// - Activity fetching with optional limit
///
/// # Arguments
/// * `executor` - The universal tool executor with server resources
/// * `user_uuid` - User identifier
/// * `tenant_id` - Optional tenant identifier for multi-tenant isolation
/// * `provider_name` - Name of the provider (e.g., "strava", "garmin")
/// * `limit` - Optional limit on number of activities to fetch
///
/// # Errors
/// Returns `UniversalResponse` error if provider is not supported, authentication fails,
/// or activity fetching fails.
pub async fn fetch_provider_activities(
    executor: &UniversalToolExecutor,
    user_uuid: Uuid,
    tenant_id: Option<&str>,
    provider_name: &str,
    limit: Option<usize>,
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
            // Create and configure provider
            let provider = match create_configured_provider(
                provider_name,
                &executor.resources.provider_registry,
                &token_data,
            )
            .await
            {
                Ok(p) => p,
                Err(e) => {
                    return Err(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Failed to configure {provider_name} provider: {e}")),
                        metadata: None,
                    });
                }
            };

            // Fetch activities
            match provider.get_activities(limit, None).await {
                Ok(activities) => Ok(activities),
                Err(e) => Err(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!(
                        "Failed to fetch activities from {provider_name}: {e}"
                    )),
                    metadata: None,
                }),
            }
        }
        Ok(None) => Err(create_no_token_response(provider_name)),
        Err(e) => Err(UniversalResponse {
            success: false,
            result: None,
            error: Some(format!("Authentication error for {provider_name}: {e}")),
            metadata: None,
        }),
    }
}

/// Infer workout intensity from recent activities
///
/// Analyzes recent training data to determine current workout intensity:
/// - High: Average > 2 hours/day or high heart rate zones
/// - Moderate: Average 1-2 hours/day
/// - Low: Average < 1 hour/day
///
/// # Arguments
/// * `activities` - List of recent activities to analyze
/// * `days_back` - Number of days the activities span
///
/// # Returns
/// Inferred intensity as "low", "moderate", or "high"
#[must_use]
pub fn infer_workout_intensity(activities: &[Activity], days_back: u32) -> String {
    if activities.is_empty() || days_back == 0 {
        return "moderate".to_owned(); // Default when no data
    }

    // Calculate total training hours
    // Safe: total_seconds is sum of activity durations (typically < 10^9 seconds), well within f64 precision
    let total_seconds: u64 = activities.iter().map(|a| a.duration_seconds).sum();
    #[allow(clippy::cast_precision_loss)]
    let total_hours = total_seconds as f64 / 3600.0;
    let avg_hours_per_day = total_hours / f64::from(days_back);

    // Calculate average heart rate if available
    let hr_activities: Vec<_> = activities
        .iter()
        .filter_map(|a| a.average_heart_rate)
        .collect();
    let avg_hr = if hr_activities.is_empty() {
        None
    } else {
        // Safe: activity count is bounded by fetch limit (typically 50), well within u32 range
        #[allow(clippy::cast_possible_truncation)]
        let count = hr_activities.len() as u32;
        Some(hr_activities.iter().sum::<u32>() / count)
    };

    // Intensity inference logic
    // High intensity: > 2 hours/day OR high avg HR (> 150 bpm)
    // Moderate: 1-2 hours/day OR moderate avg HR (130-150 bpm)
    // Low: < 1 hour/day AND low avg HR (< 130 bpm)
    if avg_hours_per_day > 2.0 || avg_hr.is_some_and(|hr| hr > 150) {
        "high".to_owned()
    } else if avg_hours_per_day >= 1.0 || avg_hr.is_some_and(|hr| hr >= 130) {
        "moderate".to_owned()
    } else {
        "low".to_owned()
    }
}
