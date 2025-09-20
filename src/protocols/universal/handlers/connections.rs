// ABOUTME: Connection management handlers for OAuth providers
// ABOUTME: Handle connection status and disconnection operations

use crate::constants::oauth_providers;
use crate::database_plugins::DatabaseProvider;
use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use crate::protocols::ProtocolError;
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
