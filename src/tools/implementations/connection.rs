// ABOUTME: Connection management tools implementing the McpTool trait.
// ABOUTME: Provides connect_provider, get_connection_status, disconnect_provider tools.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Connection Management Tools
//!
//! This module contains tools for managing provider connections:
//! - `ConnectProviderTool` - Initiate OAuth flow for a provider
//! - `GetConnectionStatusTool` - Check provider connection status
//! - `DisconnectProviderTool` - Disconnect and revoke OAuth tokens

use std::collections::HashMap;

use async_trait::async_trait;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use std::env;

use chrono::{Duration, Utc};
use serde_json::{json, Map, Value};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::constants::oauth_config::AUTHORIZATION_EXPIRES_MINUTES;
use crate::database_plugins::factory::Database;
use crate::database_plugins::DatabaseProvider;
use crate::errors::{AppError, AppResult, ErrorCode};
use crate::mcp::schema::{JsonSchema, PropertySchema};
use crate::oauth2_client::OAuthClientState;
use crate::protocols::universal::auth_service::AuthService;
use crate::tenant::{TenantContext, TenantRole};
use crate::tools::context::ToolExecutionContext;
use crate::tools::result::ToolResult;
use crate::tools::traits::{McpTool, ToolCapabilities};

// ============================================================================
// Helper functions for connection tools
// ============================================================================

/// Validate redirect URL scheme for mobile OAuth flows
fn validate_redirect_url(url: &str) -> bool {
    url.starts_with("pierre://")
        || url.starts_with("exp://")
        || url.starts_with("http://localhost")
        || url.starts_with("https://")
}

/// Build OAuth state string with optional redirect URL
fn build_oauth_state(user_id: Uuid, redirect_url: Option<&str>) -> String {
    redirect_url.map_or_else(
        || format!("{}:{}", user_id, Uuid::new_v4()),
        |url| {
            let encoded_url = URL_SAFE_NO_PAD.encode(url.as_bytes());
            format!("{}:{}:{}", user_id, Uuid::new_v4(), encoded_url)
        },
    )
}

/// Build tenant context from user and request context
async fn build_tenant_context(
    database: &Database,
    user_id: Uuid,
    user_tenant_id: Option<&str>,
    context_tenant_id: Option<Uuid>,
) -> TenantContext {
    let tenant_id = user_tenant_id
        .and_then(|t| Uuid::parse_str(t).ok())
        .or(context_tenant_id)
        .unwrap_or(user_id);

    let tenant_name = database
        .get_tenant_by_id(tenant_id)
        .await
        .map_or_else(|_| "Unknown Tenant".to_owned(), |t| t.name);

    TenantContext {
        tenant_id,
        user_id,
        tenant_name,
        user_role: TenantRole::Member,
    }
}

/// Build successful OAuth authorization response
fn build_oauth_success_response(provider: &str, url: &str, state: &str) -> ToolResult {
    ToolResult::ok(json!({
        "provider": provider,
        "authorization_url": url,
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
    }))
}

/// Build OAuth error response
fn build_oauth_error_response(provider: &str, error: &AppError) -> ToolResult {
    ToolResult::error(json!({
        "error": format!(
            "Failed to generate authorization URL: {}. \
             Please check that OAuth credentials are configured for provider '{}'.",
            error, provider
        ),
        "error_type": "oauth_configuration_error",
        "provider": provider
    }))
}

// ============================================================================
// ConnectProviderTool - Initiate OAuth connection flow
// ============================================================================

/// Tool for initiating OAuth connection flow with a fitness provider.
///
/// Generates an authorization URL that the user can visit to authenticate
/// with the provider. Supports optional redirect URL for mobile app flows.
pub struct ConnectProviderTool;

#[async_trait]
impl McpTool for ConnectProviderTool {
    fn name(&self) -> &'static str {
        "connect_provider"
    }

    fn description(&self) -> &'static str {
        "Initiate OAuth connection flow to connect a fitness data provider like Strava, Fitbit, or Garmin"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "provider".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Provider to connect (e.g., 'strava', 'fitbit', 'garmin')".to_owned(),
                ),
            },
        );
        properties.insert(
            "redirect_url".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Optional redirect URL for mobile app OAuth flows (supports pierre://, exp://, http://localhost, https://)".to_owned(),
                ),
            },
        );

        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["provider".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::REQUIRES_TENANT
    }

    async fn execute(&self, args: Value, context: &ToolExecutionContext) -> AppResult<ToolResult> {
        let registry = context.provider_registry();
        let database = context.database();

        // Extract and validate provider parameter
        let provider =
            args.get("provider")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    let supported = registry.supported_providers().join(", ");
                    AppError::new(
                ErrorCode::MissingRequiredField,
                format!("Missing required 'provider' parameter. Supported providers: {supported}"),
            )
                })?;

        if !registry.is_supported(provider) {
            let supported = registry.supported_providers().join(", ");
            return Ok(ToolResult::error(json!({
                "error": format!("Provider '{}' is not supported. Supported providers: {}", provider, supported)
            })));
        }

        // Validate redirect URL if provided
        let redirect_url = args.get("redirect_url").and_then(Value::as_str);
        if let Some(url) = redirect_url {
            if !validate_redirect_url(url) {
                return Ok(ToolResult::error(json!({
                    "error": "Invalid redirect_url scheme. Allowed schemes: pierre://, exp://, http://localhost, https://"
                })));
            }
        }

        // Get user and build tenant context
        // Verify user exists
        database.get_user(context.user_id).await?.ok_or_else(|| {
            AppError::new(
                ErrorCode::ResourceNotFound,
                format!("User {} not found", context.user_id),
            )
        })?;

        // Get user's tenant from tenant_users junction table
        let tenants = database.list_tenants_for_user(context.user_id).await.ok();
        let user_tenant_id = tenants.and_then(|t| t.first().map(|tenant| tenant.id.to_string()));

        let tenant_context = build_tenant_context(
            database,
            context.user_id,
            user_tenant_id.as_deref(),
            context.tenant_id,
        )
        .await;

        // Build OAuth state and generate authorization URL
        let state = build_oauth_state(context.user_id, redirect_url);

        match context
            .resources
            .tenant_oauth_client
            .get_authorization_url(&tenant_context, provider, &state, database)
            .await
        {
            Ok(url) => {
                // Store state server-side for CSRF protection with 10-minute TTL
                let now = Utc::now();
                let base_url = env::var("BASE_URL").unwrap_or_else(|_| {
                    format!("http://localhost:{}", context.resources.config.http_port)
                });
                let oauth_callback_uri = format!("{base_url}/api/oauth/callback/{provider}");
                let client_state = OAuthClientState {
                    state: state.clone(),
                    provider: provider.to_owned(),
                    user_id: Some(context.user_id),
                    tenant_id: context
                        .tenant_id
                        .as_ref()
                        .map(ToString::to_string)
                        .or_else(|| Some(tenant_context.tenant_id.to_string())),
                    redirect_uri: oauth_callback_uri,
                    scope: None,
                    pkce_code_verifier: None,
                    created_at: now,
                    expires_at: now + Duration::minutes(i64::from(AUTHORIZATION_EXPIRES_MINUTES)),
                    used: false,
                };

                if let Err(e) = database.store_oauth_client_state(&client_state).await {
                    warn!("Failed to store OAuth state for CSRF protection: {}", e);
                    return Ok(build_oauth_error_response(
                        provider,
                        &AppError::internal(format!("Failed to initiate OAuth flow: {e}")),
                    ));
                }

                let flow_type = if redirect_url.is_some() {
                    " (mobile flow)"
                } else {
                    ""
                };
                info!(
                    "Generated OAuth URL for user {} provider {}{}",
                    context.user_id, provider, flow_type
                );
                Ok(build_oauth_success_response(provider, &url, &state))
            }
            Err(e) => {
                error!("OAuth URL generation failed for {}: {}", provider, e);
                Ok(build_oauth_error_response(provider, &e))
            }
        }
    }
}

// ============================================================================
// GetConnectionStatusTool - Check OAuth connection status
// ============================================================================

/// Tool for checking the connection status of fitness providers.
///
/// Can check a single provider's status or all supported providers.
pub struct GetConnectionStatusTool;

#[async_trait]
impl McpTool for GetConnectionStatusTool {
    fn name(&self) -> &'static str {
        "get_connection_status"
    }

    fn description(&self) -> &'static str {
        "Check the connection status of fitness data providers. If no provider is specified, returns status for all supported providers."
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "provider".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Optional: specific provider to check (e.g., 'strava'). If omitted, checks all providers.".to_owned(),
                ),
            },
        );

        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: None,
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, context: &ToolExecutionContext) -> AppResult<ToolResult> {
        let registry = context.provider_registry();
        let auth_service = AuthService::new(context.resources.clone());
        let tenant_id_str = context.tenant_id.map(|id| id.to_string());

        if let Some(specific_provider) = args.get("provider").and_then(Value::as_str) {
            // Single provider mode
            let is_connected = matches!(
                auth_service
                    .get_valid_token(context.user_id, specific_provider, tenant_id_str.as_deref())
                    .await,
                Ok(Some(_))
            );

            let status = if is_connected {
                "connected"
            } else {
                "disconnected"
            };

            Ok(ToolResult::ok(json!({
                "provider": specific_provider,
                "status": status,
                "connected": is_connected
            })))
        } else {
            // Multi-provider mode - check all supported providers
            let mut providers_status = Map::new();

            for provider in registry.supported_providers() {
                let is_connected = matches!(
                    auth_service
                        .get_valid_token(context.user_id, provider, tenant_id_str.as_deref())
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
                    json!({ "connected": is_connected, "status": status }),
                );
            }

            Ok(ToolResult::ok(json!({ "providers": providers_status })))
        }
    }
}

// ============================================================================
// DisconnectProviderTool - Disconnect OAuth provider
// ============================================================================

/// Tool for disconnecting from a fitness provider by removing OAuth tokens.
pub struct DisconnectProviderTool;

#[async_trait]
impl McpTool for DisconnectProviderTool {
    fn name(&self) -> &'static str {
        "disconnect_provider"
    }

    fn description(&self) -> &'static str {
        "Disconnect from a fitness data provider by removing stored OAuth tokens"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "provider".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Provider to disconnect (e.g., 'strava', 'fitbit', 'garmin')".to_owned(),
                ),
            },
        );

        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["provider".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::WRITES_DATA
    }

    async fn execute(&self, args: Value, context: &ToolExecutionContext) -> AppResult<ToolResult> {
        let registry = context.provider_registry();
        let database = context.database();

        // Extract provider from parameters (required)
        let provider =
            args.get("provider")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    let supported = registry.supported_providers().join(", ");
                    AppError::new(
                ErrorCode::MissingRequiredField,
                format!("Missing required 'provider' parameter. Supported providers: {supported}"),
            )
                })?;

        // Get tenant ID string, default to "default" if not present
        let tenant_id_str = context
            .tenant_id
            .map_or_else(|| "default".to_owned(), |id| id.to_string());

        // Disconnect by deleting the token directly
        match database
            .delete_user_oauth_token(context.user_id, &tenant_id_str, provider)
            .await
        {
            Ok(()) => Ok(ToolResult::ok(json!({
                "provider": provider,
                "status": "disconnected",
                "message": format!("Successfully disconnected from {}", provider)
            }))),
            Err(e) => Ok(ToolResult::error(json!({
                "error": format!("Failed to disconnect from {}: {}", provider, e),
                "provider": provider
            }))),
        }
    }
}

// ============================================================================
// Module exports
// ============================================================================

/// Create all connection tools for registration
#[must_use]
pub fn create_connection_tools() -> Vec<Box<dyn McpTool>> {
    vec![
        Box::new(ConnectProviderTool),
        Box::new(GetConnectionStatusTool),
        Box::new(DisconnectProviderTool),
    ]
}
