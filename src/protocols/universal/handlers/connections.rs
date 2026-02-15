// ABOUTME: Connection management handlers for OAuth providers
// ABOUTME: Handle connection status, disconnection, and connection initiation
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::constants::oauth_config::AUTHORIZATION_EXPIRES_MINUTES;
use crate::database_plugins::DatabaseProvider;
use crate::models::TenantId;
use crate::oauth2_client::OAuthClientState;
use crate::protocols::universal::{UniversalRequest, UniversalResponse, UniversalToolExecutor};
use crate::protocols::ProtocolError;
use crate::tenant::{TenantContext, TenantRole};
use crate::utils::uuid::parse_user_id_for_protocol;
use chrono::{Duration, Utc};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::env;
use std::future::Future;
use std::pin::Pin;
use tracing::{error, info, warn};

/// Handle `get_connection_status` tool - check OAuth connection status
#[must_use]
pub fn handle_get_connection_status(
    executor: &UniversalToolExecutor,
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
        if let Some(specific_provider) = request.parameters.get("provider").and_then(Value::as_str)
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
                result: Some(json!({
                    "provider": specific_provider,
                    "status": status,
                    "connected": is_connected
                })),
                error: None,
                metadata: Some({
                    let mut map = HashMap::new();
                    map.insert("user_id".to_owned(), Value::String(user_uuid.to_string()));
                    map.insert(
                        "provider".to_owned(),
                        Value::String(specific_provider.to_owned()),
                    );
                    map.insert(
                        "tenant_id".to_owned(),
                        request.tenant_id.map_or(Value::Null, Value::String),
                    );
                    map
                }),
            })
        } else {
            // Multi-provider mode - check all supported providers from registry
            let providers_to_check = executor.resources.provider_registry.supported_providers();
            let mut providers_status = Map::new();

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
                    json!({
                        "connected": is_connected,
                        "status": status
                    }),
                );
            }

            Ok(UniversalResponse {
                success: true,
                result: Some(json!({
                    "providers": providers_status
                })),
                error: None,
                metadata: Some({
                    let mut map = HashMap::new();
                    map.insert("user_id".to_owned(), Value::String(user_uuid.to_string()));
                    map.insert(
                        "tenant_id".to_owned(),
                        request.tenant_id.map_or(Value::Null, Value::String),
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
    executor: &UniversalToolExecutor,
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
        let Some(provider) = request.parameters.get("provider").and_then(Value::as_str) else {
            let supported = executor
                .resources
                .provider_registry
                .supported_providers()
                .join(", ");
            return Ok(connection_error(format!(
                "Missing required 'provider' parameter. Supported providers: {supported}"
            )));
        };

        // Resolve tenant ID: prefer request.tenant_id (user's selected tenant from JWT),
        // falling back to user's first tenant for clients without active_tenant_id.
        let tenant_id: TenantId = if let Some(tid) = request.tenant_id.as_deref() {
            tid.parse::<TenantId>()
                .map_err(|_| ProtocolError::InvalidRequest(format!("Invalid tenant_id: {tid}")))?
        } else {
            // No active tenant in request - fall back to user's first tenant
            let tenants = executor
                .resources
                .database
                .list_tenants_for_user(user_uuid)
                .await
                .unwrap_or_default();
            match tenants.first() {
                Some(t) => t.id,
                None => {
                    return Ok(connection_error(
                        "Cannot disconnect: user does not belong to any tenant",
                    ));
                }
            }
        };

        // Disconnect by deleting the token
        match (*executor.resources.database)
            .delete_user_oauth_token(user_uuid, tenant_id, provider)
            .await
        {
            Ok(()) => Ok(UniversalResponse {
                success: true,
                result: Some(json!({
                    "provider": provider,
                    "status": "disconnected",
                    "message": format!("Successfully disconnected from {provider}")
                })),
                error: None,
                metadata: Some({
                    let mut map = HashMap::new();
                    map.insert("user_id".to_owned(), Value::String(user_uuid.to_string()));
                    map.insert("provider".to_owned(), Value::String(provider.to_owned()));
                    map.insert(
                        "tenant_id".to_owned(),
                        request.tenant_id.map_or(Value::Null, Value::String),
                    );
                    map
                }),
            }),
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Failed to disconnect from {provider}: {e}")),
                metadata: Some({
                    let mut map = HashMap::new();
                    map.insert("user_id".to_owned(), Value::String(user_uuid.to_string()));
                    map.insert("provider".to_owned(), Value::String(provider.to_owned()));
                    map.insert(
                        "tenant_id".to_owned(),
                        request.tenant_id.map_or(Value::Null, Value::String),
                    );
                    map
                }),
            }),
        }
    })
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
        result: Some(json!({
            "provider": provider,
            "authorization_url": authorization_url,
            "state": state,
            "instructions": format!(
                "To connect your {} account:\n\
                 1. Visit the authorization URL\n\
                 2. Log in to {} and approve the connection\n\
                 3. You will be redirected back to complete the connection\n\
                 4. Once connected, you can access your {} data through Pierre",
                provider, provider, provider
            ),
            "expires_in_minutes": AUTHORIZATION_EXPIRES_MINUTES,
            "status": "pending_authorization"
        })),
        error: None,
        metadata: Some({
            let mut map = HashMap::new();
            map.insert("user_id".to_owned(), Value::String(user_uuid.to_string()));
            map.insert("tenant_id".to_owned(), Value::String(tenant_id.to_string()));
            map.insert("provider".to_owned(), Value::String(provider.to_owned()));
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
            let mut map = HashMap::new();
            map.insert(
                "error_type".to_owned(),
                Value::String("oauth_configuration_error".to_owned()),
            );
            map.insert("provider".to_owned(), Value::String(provider.to_owned()));
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

/// Validate redirect URL scheme for OAuth mobile flows
fn validate_redirect_url_scheme(url: &str) -> bool {
    url.starts_with("pierre://")
        || url.starts_with("exp://")
        || url.starts_with("http://localhost")
        || url.starts_with("https://")
}

/// Build OAuth state string with optional redirect URL
fn build_oauth_state(user_uuid: uuid::Uuid, redirect_url: Option<&str>) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    redirect_url.map_or_else(
        || format!("{}:{}", user_uuid, uuid::Uuid::new_v4()),
        |url| {
            let encoded_url = URL_SAFE_NO_PAD.encode(url.as_bytes());
            format!("{}:{}:{}", user_uuid, uuid::Uuid::new_v4(), encoded_url)
        },
    )
}

/// Handle `connect_provider` tool - initiate OAuth connection flow
///
/// Accepts optional `redirect_url` parameter for mobile app OAuth flows.
/// When provided, the redirect URL is base64 encoded and included in the OAuth state,
/// allowing the server to redirect back to the mobile app after OAuth completes.
#[must_use]
pub fn handle_connect_provider(
    executor: &UniversalToolExecutor,
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
        if !registry.is_supported(provider) {
            let supported = registry.supported_providers().join(", ");
            return Ok(connection_error(format!(
                "Provider '{provider}' is not supported. Supported providers: {supported}"
            )));
        }

        // Extract and validate optional redirect_url for mobile OAuth flows
        let redirect_url = request
            .parameters
            .get("redirect_url")
            .and_then(Value::as_str);
        if let Some(url) = redirect_url {
            if !validate_redirect_url_scheme(url) {
                return Ok(connection_error(
                    "Invalid redirect_url scheme. Allowed: pierre://, exp://, http://localhost, https://",
                ));
            }
        }

        // Verify user exists
        match db.get_user(user_uuid).await {
            Ok(Some(_)) => {}
            Ok(None) => return Ok(connection_error(format!("User {user_uuid} not found"))),
            Err(e) => return Ok(connection_error(format!("Database error: {e}"))),
        }

        // Get tenant context: prefer request.tenant_id (user's selected tenant from JWT),
        // falling back to user's first tenant for clients without active_tenant_id.
        // Security: always verify membership before using request.tenant_id,
        // as it would allow a caller to use another tenant's OAuth credentials/rate limits.
        let tenants = db
            .list_tenants_for_user(user_uuid)
            .await
            .unwrap_or_default();
        let tenant_id: TenantId = request
            .tenant_id
            .as_ref()
            .and_then(|t| t.parse::<TenantId>().ok())
            .map_or_else(
                // No tenant_id in request; use first membership or user_uuid
                || {
                    tenants
                        .first()
                        .map_or_else(|| TenantId::from(user_uuid), |t| t.id)
                },
                |requested_tid| {
                    // Verify the user is a member of the requested tenant
                    if tenants.iter().any(|t| t.id == requested_tid) {
                        requested_tid
                    } else {
                        // User is not a member of the requested tenant
                        tenants
                            .first()
                            .map_or_else(|| TenantId::from(user_uuid), |t| t.id)
                    }
                },
            );
        let tenant_name = db
            .get_tenant_by_id(tenant_id)
            .await
            .map_or_else(|_| "Unknown Tenant".to_owned(), |t| t.name);
        let ctx = TenantContext {
            tenant_id,
            user_id: user_uuid,
            tenant_name,
            user_role: TenantRole::Member,
        };

        let state = build_oauth_state(user_uuid, redirect_url);

        match executor
            .resources
            .tenant_oauth_client
            .get_authorization_url(&ctx, provider, &state, db.as_ref())
            .await
        {
            Ok(url) => {
                // Store state server-side for CSRF protection with 10-minute TTL
                let now = Utc::now();
                let base_url = env::var("BASE_URL").unwrap_or_else(|_| {
                    format!("http://localhost:{}", executor.resources.config.http_port)
                });
                let oauth_callback_uri = format!("{base_url}/api/oauth/callback/{provider}");
                let client_state = OAuthClientState {
                    state: state.clone(),
                    provider: provider.to_owned(),
                    user_id: Some(user_uuid),
                    tenant_id: Some(tenant_id.to_string()),
                    redirect_uri: oauth_callback_uri,
                    scope: None,
                    pkce_code_verifier: None,
                    created_at: now,
                    expires_at: now + Duration::minutes(i64::from(AUTHORIZATION_EXPIRES_MINUTES)),
                    used: false,
                };

                if let Err(e) = db.store_oauth_client_state(&client_state).await {
                    warn!("Failed to store OAuth state for CSRF protection: {}", e);
                    return Ok(build_oauth_error_response(
                        provider,
                        &format!("Failed to initiate OAuth flow: {e}"),
                    ));
                }

                let flow_type = if redirect_url.is_some() {
                    " (mobile flow)"
                } else {
                    ""
                };
                info!(
                    "Generated OAuth URL for user {} provider {}{}",
                    user_uuid, provider, flow_type
                );
                Ok(build_oauth_success_response(
                    user_uuid,
                    tenant_id.as_uuid(),
                    provider,
                    &url,
                    &state,
                ))
            }
            Err(e) => {
                error!("OAuth URL generation failed for {}: {}", provider, e);
                Ok(build_oauth_error_response(provider, &e.to_string()))
            }
        }
    })
}
