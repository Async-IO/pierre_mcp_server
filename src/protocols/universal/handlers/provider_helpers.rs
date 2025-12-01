// ABOUTME: Shared helper functions for provider-agnostic handler operations
// ABOUTME: Consolidates provider extraction, configuration, and creation logic
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::config::environment::{default_provider, OAuthProviderConfig};
use crate::protocols::universal::auth_service::TokenData;
use crate::providers::core::FitnessProvider;
use crate::providers::ProviderRegistry;
use std::sync::Arc;

/// Extract provider name from request parameters, falling back to default provider
///
/// Returns the provider name from request parameters if specified, otherwise returns
/// the configured default provider (from `PIERRE_DEFAULT_PROVIDER` env var or "synthetic").
#[must_use]
pub fn extract_provider(parameters: &serde_json::Map<String, serde_json::Value>) -> String {
    parameters
        .get("provider")
        .and_then(serde_json::Value::as_str)
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
    let config = crate::config::environment::get_oauth_config(provider_name);

    // Build credentials
    let credentials = crate::providers::OAuth2Credentials {
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
pub fn create_no_token_response(
    provider_name: &str,
) -> crate::protocols::universal::UniversalResponse {
    crate::protocols::universal::UniversalResponse {
        success: false,
        result: None,
        error: Some(format!(
            "No valid {provider_name} token found. Please connect your account using the connect_provider tool with provider='{provider_name}'."
        )),
        metadata: Some({
            let mut map = std::collections::HashMap::new();
            map.insert(
                "total_activities".to_owned(),
                serde_json::Value::Number(0.into()),
            );
            map.insert(
                "authentication_required".to_owned(),
                serde_json::Value::Bool(true),
            );
            map.insert(
                "provider".to_owned(),
                serde_json::Value::String(provider_name.to_owned()),
            );
            map
        }),
    }
}

/// Create a standard auth error response
#[must_use]
pub fn create_auth_error_response(
    provider_name: &str,
    error: &str,
) -> crate::protocols::universal::UniversalResponse {
    crate::protocols::universal::UniversalResponse {
        success: true,
        result: Some(serde_json::json!({
            "activities": [],
            "message": format!("Authentication error for {provider_name}: {error}"),
            "error": format!("Authentication error: {error}"),
            "provider": provider_name
        })),
        error: None,
        metadata: Some({
            let mut map = std::collections::HashMap::new();
            map.insert(
                "authentication_error".to_owned(),
                serde_json::Value::Bool(true),
            );
            map.insert(
                "provider".to_owned(),
                serde_json::Value::String(provider_name.to_owned()),
            );
            map
        }),
    }
}

/// Build success response for activities from any provider
pub fn build_activities_success_response(
    activities: &[crate::models::Activity],
    provider_name: &str,
    user_uuid: uuid::Uuid,
    tenant_id: Option<String>,
) -> crate::protocols::universal::UniversalResponse {
    crate::protocols::universal::UniversalResponse {
        success: true,
        result: Some(serde_json::json!({
            "activities": activities,
            "provider": provider_name,
            "count": activities.len()
        })),
        error: None,
        metadata: Some({
            let mut map = std::collections::HashMap::new();
            map.insert(
                "total_activities".to_owned(),
                serde_json::Value::Number(activities.len().into()),
            );
            map.insert(
                "user_id".to_owned(),
                serde_json::Value::String(user_uuid.to_string()),
            );
            map.insert(
                "tenant_id".to_owned(),
                tenant_id.map_or(serde_json::Value::Null, serde_json::Value::String),
            );
            map.insert(
                "provider".to_owned(),
                serde_json::Value::String(provider_name.to_owned()),
            );
            map.insert("cached".to_owned(), serde_json::Value::Bool(false));
            map
        }),
    }
}

/// Get OAuth config for a provider, with logging
pub fn get_provider_oauth_config(provider_name: &str) -> OAuthProviderConfig {
    let config = crate::config::environment::get_oauth_config(provider_name);
    tracing::debug!(
        provider = provider_name,
        has_client_id = config.client_id.is_some(),
        "Loaded OAuth config for provider"
    );
    config
}
