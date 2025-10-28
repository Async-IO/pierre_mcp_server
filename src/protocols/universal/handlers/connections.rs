// ABOUTME: Connection management handlers for OAuth providers
// ABOUTME: Handle connection status, disconnection, and connection initiation
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::constants::oauth_providers;
use crate::database_plugins::DatabaseProvider;
use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use crate::protocols::ProtocolError;
use crate::tenant::TenantContext;
use crate::utils::uuid::parse_user_id_for_protocol;
use std::future::Future;
use std::pin::Pin;

/// Handle `get_connection_status` tool - check OAuth connection status
#[must_use]
pub fn handle_get_connection_status(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Parse user ID from request
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Check if a specific provider is requested
        if let Some(specific_provider) = request
            .parameters
            .get("provider")
            .and_then(serde_json::Value::as_str)
        {
            // Single provider mode
            let is_connected = matches!(
                executor
                    .auth_service
                    .get_valid_token(user_uuid, specific_provider, request.tenant_id.as_deref())
                    .await,
                Ok(Some(_))
            );

            let status = if is_connected {
                "connected"
            } else {
                "disconnected"
            };

            Ok(UniversalResponse {
                success: true,
                result: Some(serde_json::json!({
                    "provider": specific_provider,
                    "status": status,
                    "connected": is_connected
                })),
                error: None,
                metadata: Some({
                    let mut map = std::collections::HashMap::new();
                    map.insert(
                        "user_id".to_string(),
                        serde_json::Value::String(user_uuid.to_string()),
                    );
                    map.insert(
                        "provider".to_string(),
                        serde_json::Value::String(specific_provider.to_string()),
                    );
                    map.insert(
                        "tenant_id".to_string(),
                        request
                            .tenant_id
                            .map_or(serde_json::Value::Null, serde_json::Value::String),
                    );
                    map
                }),
            })
        } else {
            // Multi-provider mode - check all supported providers
            let providers_to_check = [oauth_providers::STRAVA, "fitbit"];
            let mut providers_status = serde_json::Map::new();

            for provider in providers_to_check {
                let is_connected = matches!(
                    executor
                        .auth_service
                        .get_valid_token(user_uuid, provider, request.tenant_id.as_deref())
                        .await,
                    Ok(Some(_))
                );

                let status = if is_connected {
                    "connected"
                } else {
                    "disconnected"
                };

                providers_status.insert(
                    provider.to_string(),
                    serde_json::json!({
                        "connected": is_connected,
                        "status": status
                    }),
                );
            }

            Ok(UniversalResponse {
                success: true,
                result: Some(serde_json::json!({
                    "providers": providers_status
                })),
                error: None,
                metadata: Some({
                    let mut map = std::collections::HashMap::new();
                    map.insert(
                        "user_id".to_string(),
                        serde_json::Value::String(user_uuid.to_string()),
                    );
                    map.insert(
                        "tenant_id".to_string(),
                        request
                            .tenant_id
                            .map_or(serde_json::Value::Null, serde_json::Value::String),
                    );
                    map
                }),
            })
        }
    })
}

/// Handle `disconnect_provider` tool - disconnect user from OAuth provider
#[must_use]
pub fn handle_disconnect_provider(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Parse user ID from request
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Extract provider from parameters (default to Strava)
        let provider = request
            .parameters
            .get("provider")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(oauth_providers::STRAVA);

        // Disconnect by deleting the token directly
        let tenant_id_str = request.tenant_id.as_deref().unwrap_or("default");
        match (*executor.resources.database)
            .delete_user_oauth_token(user_uuid, tenant_id_str, provider)
            .await
        {
            Ok(()) => Ok(UniversalResponse {
                success: true,
                result: Some(serde_json::json!({
                    "provider": provider,
                    "status": "disconnected",
                    "message": format!("Successfully disconnected from {provider}")
                })),
                error: None,
                metadata: Some({
                    let mut map = std::collections::HashMap::new();
                    map.insert(
                        "user_id".to_string(),
                        serde_json::Value::String(user_uuid.to_string()),
                    );
                    map.insert(
                        "provider".to_string(),
                        serde_json::Value::String(provider.to_string()),
                    );
                    map.insert(
                        "tenant_id".to_string(),
                        request
                            .tenant_id
                            .map_or(serde_json::Value::Null, serde_json::Value::String),
                    );
                    map
                }),
            }),
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Failed to disconnect from {provider}: {e}")),
                metadata: Some({
                    let mut map = std::collections::HashMap::new();
                    map.insert(
                        "user_id".to_string(),
                        serde_json::Value::String(user_uuid.to_string()),
                    );
                    map.insert(
                        "provider".to_string(),
                        serde_json::Value::String(provider.to_string()),
                    );
                    map.insert(
                        "tenant_id".to_string(),
                        request
                            .tenant_id
                            .map_or(serde_json::Value::Null, serde_json::Value::String),
                    );
                    map
                }),
            }),
        }
    })
}

/// Validate that provider is supported
fn is_provider_supported(provider: &str) -> bool {
    matches!(provider, oauth_providers::STRAVA | oauth_providers::FITBIT)
}

/// Build successful OAuth connection response
fn build_oauth_success_response(
    user_uuid: uuid::Uuid,
    tenant_id: uuid::Uuid,
    provider: &str,
    authorization_url: &str,
    state: &str,
) -> UniversalResponse {
    UniversalResponse {
        success: true,
        result: Some(serde_json::json!({
            "provider": provider,
            "authorization_url": authorization_url,
            "state": state,
            "instructions": format!(
                "To connect your {} account:\n\
                 1. Visit the authorization URL\n\
                 2. Log in to {} and approve the connection\n\
                 3. You will be redirected back to complete the connection\n\
                 4. Once connected, you can access your {} data through MCP tools",
                provider, provider, provider
            ),
            "expires_in_minutes": crate::constants::oauth_config::AUTHORIZATION_EXPIRES_MINUTES,
            "status": "pending_authorization"
        })),
        error: None,
        metadata: Some({
            let mut map = std::collections::HashMap::new();
            map.insert(
                "user_id".to_string(),
                serde_json::Value::String(user_uuid.to_string()),
            );
            map.insert(
                "tenant_id".to_string(),
                serde_json::Value::String(tenant_id.to_string()),
            );
            map.insert(
                "provider".to_string(),
                serde_json::Value::String(provider.to_string()),
            );
            map
        }),
    }
}

/// Build OAuth error response
fn build_oauth_error_response(provider: &str, error: &str) -> UniversalResponse {
    UniversalResponse {
        success: false,
        result: None,
        error: Some(format!(
            "Failed to generate authorization URL: {error}. \
             Please check that OAuth credentials are configured for provider '{provider}'."
        )),
        metadata: Some({
            let mut map = std::collections::HashMap::new();
            map.insert(
                "error_type".to_string(),
                serde_json::Value::String("oauth_configuration_error".to_string()),
            );
            map.insert(
                "provider".to_string(),
                serde_json::Value::String(provider.to_string()),
            );
            map
        }),
    }
}

/// Handle `connect_provider` tool - initiate OAuth connection flow
#[must_use]
pub fn handle_connect_provider(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        let provider = request
            .parameters
            .get("provider")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(oauth_providers::STRAVA);

        if !is_provider_supported(provider) {
            return Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!(
                    "Provider '{provider}' is not supported. Supported providers: strava, fitbit"
                )),
                metadata: None,
            });
        }

        let user = match executor.resources.database.get_user(user_uuid).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                return Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!("User {user_uuid} not found")),
                    metadata: None,
                });
            }
            Err(e) => {
                return Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!("Database error: {e}")),
                    metadata: None,
                });
            }
        };

        let Some(tenant_id) = user
            .tenant_id
            .as_ref()
            .and_then(|t| uuid::Uuid::parse_str(t).ok())
        else {
            return Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some("User does not belong to any tenant".to_string()),
                metadata: None,
            });
        };

        let tenant_name = match executor
            .resources
            .database
            .get_tenant_by_id(tenant_id)
            .await
        {
            Ok(tenant) => tenant.name,
            Err(_) => "Unknown Tenant".to_string(),
        };

        let tenant_context = TenantContext {
            tenant_id,
            user_id: user_uuid,
            tenant_name,
            user_role: crate::tenant::TenantRole::Member,
        };

        let state = format!("{}:{}", user_uuid, uuid::Uuid::new_v4());

        match executor
            .resources
            .tenant_oauth_client
            .get_authorization_url(
                &tenant_context,
                provider,
                &state,
                executor.resources.database.as_ref(),
            )
            .await
        {
            Ok(authorization_url) => {
                tracing::info!(
                    "Generated OAuth authorization URL for user {} and provider {}",
                    user_uuid,
                    provider
                );
                Ok(build_oauth_success_response(
                    user_uuid,
                    tenant_id,
                    provider,
                    &authorization_url,
                    &state,
                ))
            }
            Err(e) => {
                tracing::error!(
                    "Failed to generate OAuth authorization URL for user {} and provider {}: {}",
                    user_uuid,
                    provider,
                    e
                );
                Ok(build_oauth_error_response(provider, &e.to_string()))
            }
        }
    })
}
