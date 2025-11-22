// ABOUTME: Connection management handlers for OAuth providers
// ABOUTME: Handle connection status, disconnection, and connection initiation
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

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
        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "handle_get_connection_status cancelled by user".to_owned(),
                ));
            }
        }

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
                        "user_id".to_owned(),
                        serde_json::Value::String(user_uuid.to_string()),
                    );
                    map.insert(
                        "provider".to_owned(),
                        serde_json::Value::String(specific_provider.to_owned()),
                    );
                    map.insert(
                        "tenant_id".to_owned(),
                        request
                            .tenant_id
                            .map_or(serde_json::Value::Null, serde_json::Value::String),
                    );
                    map
                }),
            })
        } else {
            // Multi-provider mode - check all supported providers from registry
            let providers_to_check = executor.resources.provider_registry.supported_providers();
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
                    provider.to_owned(),
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
                        "user_id".to_owned(),
                        serde_json::Value::String(user_uuid.to_string()),
                    );
                    map.insert(
                        "tenant_id".to_owned(),
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
        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "handle_disconnect_provider cancelled by user".to_owned(),
                ));
            }
        }

        // Parse user ID from request
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Extract provider from parameters (required)
        let Some(provider) = request
            .parameters
            .get("provider")
            .and_then(serde_json::Value::as_str)
        else {
            let supported = executor
                .resources
                .provider_registry
                .supported_providers()
                .join(", ");
            return Ok(connection_error(format!(
                "Missing required 'provider' parameter. Supported providers: {supported}"
            )));
        };

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
                        "user_id".to_owned(),
                        serde_json::Value::String(user_uuid.to_string()),
                    );
                    map.insert(
                        "provider".to_owned(),
                        serde_json::Value::String(provider.to_owned()),
                    );
                    map.insert(
                        "tenant_id".to_owned(),
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
                        "user_id".to_owned(),
                        serde_json::Value::String(user_uuid.to_string()),
                    );
                    map.insert(
                        "provider".to_owned(),
                        serde_json::Value::String(provider.to_owned()),
                    );
                    map.insert(
                        "tenant_id".to_owned(),
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

/// Validate that provider is supported using provider registry
fn is_provider_supported(
    provider: &str,
    provider_registry: &crate::providers::ProviderRegistry,
) -> bool {
    provider_registry.is_supported(provider)
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
                "user_id".to_owned(),
                serde_json::Value::String(user_uuid.to_string()),
            );
            map.insert(
                "tenant_id".to_owned(),
                serde_json::Value::String(tenant_id.to_string()),
            );
            map.insert(
                "provider".to_owned(),
                serde_json::Value::String(provider.to_owned()),
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
                "error_type".to_owned(),
                serde_json::Value::String("oauth_configuration_error".to_owned()),
            );
            map.insert(
                "provider".to_owned(),
                serde_json::Value::String(provider.to_owned()),
            );
            map
        }),
    }
}

/// Create error response for connection operations
#[inline]
fn connection_error(message: impl Into<String>) -> UniversalResponse {
    UniversalResponse {
        success: false,
        result: None,
        error: Some(message.into()),
        metadata: None,
    }
}

/// Handle `connect_provider` tool - initiate OAuth connection flow
#[must_use]
pub fn handle_connect_provider(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "handle_connect_provider cancelled by user".to_owned(),
                ));
            }
        }
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;
        let registry = &executor.resources.provider_registry;
        let db = &executor.resources.database;

        // Extract and validate provider parameter
        let Some(provider) = request.parameters.get("provider").and_then(|v| v.as_str()) else {
            let supported = registry.supported_providers().join(", ");
            return Ok(connection_error(format!(
                "Missing required 'provider' parameter. Supported providers: {supported}"
            )));
        };
        if !is_provider_supported(provider, registry) {
            let supported = registry.supported_providers().join(", ");
            return Ok(connection_error(format!(
                "Provider '{provider}' is not supported. Supported providers: {supported}"
            )));
        }

        // Get user and extract tenant context
        let user = match db.get_user(user_uuid).await {
            Ok(Some(u)) => u,
            Ok(None) => return Ok(connection_error(format!("User {user_uuid} not found"))),
            Err(e) => return Ok(connection_error(format!("Database error: {e}"))),
        };
        let Some(tenant_id) = user
            .tenant_id
            .as_ref()
            .and_then(|t| uuid::Uuid::parse_str(t).ok())
        else {
            return Ok(connection_error(
                "User does not belong to any tenant".to_owned(),
            ));
        };
        let tenant_name = db
            .get_tenant_by_id(tenant_id)
            .await
            .map_or_else(|_| "Unknown Tenant".to_owned(), |t| t.name);
        let ctx = TenantContext {
            tenant_id,
            user_id: user_uuid,
            tenant_name,
            user_role: crate::tenant::TenantRole::Member,
        };
        let state = format!("{}:{}", user_uuid, uuid::Uuid::new_v4());

        // Generate OAuth authorization URL
        match executor
            .resources
            .tenant_oauth_client
            .get_authorization_url(&ctx, provider, &state, db.as_ref())
            .await
        {
            Ok(url) => {
                tracing::info!(
                    "Generated OAuth URL for user {} provider {}",
                    user_uuid,
                    provider
                );
                Ok(build_oauth_success_response(
                    user_uuid, tenant_id, provider, &url, &state,
                ))
            }
            Err(e) => {
                tracing::error!("OAuth URL generation failed for {}: {}", provider, e);
                Ok(build_oauth_error_response(provider, &e.to_string()))
            }
        }
    })
}
