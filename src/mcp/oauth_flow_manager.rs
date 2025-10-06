// ABOUTME: OAuth flow management for multi-tenant MCP server
// ABOUTME: Handles authorization requests, callbacks, token processing, and template rendering

use super::resources::ServerResources;
use crate::database_plugins::DatabaseProvider;
use crate::tenant::{TenantContext, TenantRole};
use crate::utils::json_responses::{api_error, oauth_error};
use anyhow::Result;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;
use warp::http::HeaderMap;

/// Manages OAuth authorization flows for multiple providers and tenants
pub struct OAuthFlowManager {
    resources: Arc<ServerResources>,
}

impl OAuthFlowManager {
    /// Create a new OAuth flow manager
    #[must_use]
    pub const fn new(resources: Arc<ServerResources>) -> Self {
        Self { resources }
    }

    /// Handle OAuth authorization request for a specific provider
    ///
    /// # Errors
    /// Returns an error if user validation, credential processing, or URL generation fails
    pub async fn handle_authorization_request(
        &self,
        provider: String,
        user_id_str: String,
        headers: HeaderMap,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let user_id = Uuid::parse_str(&user_id_str)
            .map_err(|_| warp::reject::custom(ApiError(api_error("Invalid user ID format"))))?;

        let user = self.get_user_with_tenant(user_id).await?;
        let tenant_id = Self::extract_tenant_id(&user)?;

        self.process_oauth_credentials(&headers, &provider, tenant_id, user_id)
            .await?;

        let tenant_name = self.get_tenant_name(tenant_id).await;
        let tenant_context = TenantContext {
            tenant_id,
            user_id,
            tenant_name,
            user_role: TenantRole::Member,
        };

        self.generate_authorization_redirect(&provider, &tenant_context)
            .await
    }

    /// Process OAuth credentials from request headers (optional)
    async fn process_oauth_credentials(
        &self,
        headers: &HeaderMap,
        provider: &str,
        tenant_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), warp::Rejection> {
        // Look for provider-specific headers (e.g., x-strava-client-id)
        let user_client_id = headers
            .get(format!("x-{}-client-id", provider.to_lowercase()))
            .and_then(|h| h.to_str().ok())
            .map(std::string::ToString::to_string);

        let user_client_secret = headers
            .get(format!("x-{}-client-secret", provider.to_lowercase()))
            .and_then(|h| h.to_str().ok())
            .map(std::string::ToString::to_string);

        // Only store credentials if both client_id and client_secret are provided
        if let (Some(client_id), Some(client_secret)) = (user_client_id, user_client_secret) {
            info!(
                "Using user-provided OAuth credentials for tenant {} and provider {}",
                tenant_id, provider
            );

            let redirect_uri = Self::get_provider_redirect_uri(provider);
            let scopes = Self::get_provider_scopes(provider);

            let request = crate::tenant::oauth_client::StoreCredentialsRequest {
                client_id,
                client_secret,
                redirect_uri,
                scopes,
                configured_by: user_id,
            };

            self.resources
                .tenant_oauth_client
                .store_credentials(tenant_id, provider, request)
                .await
                .map_err(|e| {
                    error!(
                        "Failed to store user OAuth credentials for tenant {} and provider {}: {}",
                        tenant_id, provider, e
                    );
                    let error = api_error(&format!("Failed to store OAuth credentials: {e}"));
                    warp::reject::custom(ApiError(error))
                })?;
        }

        Ok(())
    }

    /// Generate OAuth authorization redirect URL
    async fn generate_authorization_redirect(
        &self,
        provider: &str,
        tenant_context: &TenantContext,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let tenant_oauth_client = &self.resources.tenant_oauth_client;
        let state = format!("{}:{}", tenant_context.user_id, uuid::Uuid::new_v4());

        match tenant_oauth_client
            .get_authorization_url(
                tenant_context,
                provider,
                &state,
                self.resources.database.as_ref(),
            )
            .await
        {
            Ok(auth_url) => {
                info!(
                    "Generated authorization URL for provider {} and tenant {}: {}",
                    provider, tenant_context.tenant_id, auth_url
                );

                // Return redirect response with 302 status (expected by tests)
                let uri = auth_url.parse::<warp::http::Uri>().map_err(|_| {
                    warp::reject::custom(ApiError(oauth_error(
                        "Invalid authorization URL",
                        "URL parse error",
                        Some(provider),
                    )))
                })?;
                Ok(warp::redirect::found(uri))
            }
            Err(e) => {
                error!(
                    "Failed to generate authorization URL for provider {}: {}",
                    provider, e
                );
                Err(warp::reject::custom(ApiError(oauth_error(
                    "Failed to generate authorization URL",
                    &e.to_string(),
                    Some(provider),
                ))))
            }
        }
    }

    /// Get user with tenant information
    async fn get_user_with_tenant(
        &self,
        user_id: Uuid,
    ) -> Result<crate::models::User, warp::Rejection> {
        self.resources
            .database
            .get_user(user_id)
            .await
            .map_err(|e| {
                error!("Failed to get user {}: {}", user_id, e);
                warp::reject::custom(ApiError(api_error("User not found")))
            })?
            .ok_or_else(|| warp::reject::custom(ApiError(api_error("User not found"))))
    }

    /// Extract tenant ID from user
    fn extract_tenant_id(user: &crate::models::User) -> Result<Uuid, warp::Rejection> {
        let tenant_id_str = user.tenant_id
            .clone()
            .ok_or_else(|| {
                error!(
                    "Missing tenant for user - user_id: {}, email: {}. User does not belong to any tenant.",
                    user.id, user.email
                );
                warp::reject::custom(ApiError(api_error("User does not belong to any tenant")))
            })?;

        tenant_id_str.parse().map_err(|_| {
            error!(
                "Invalid tenant ID format - user_id: {}, email: {}, tenant_id_str: {}. Failed to parse tenant ID as UUID.",
                user.id, user.email, tenant_id_str
            );
            warp::reject::custom(ApiError(api_error("Invalid tenant ID format")))
        })
    }

    /// Get tenant name
    async fn get_tenant_name(&self, tenant_id: Uuid) -> String {
        match self.resources.database.get_tenant_by_id(tenant_id).await {
            Ok(tenant) => tenant.name,
            Err(e) => {
                warn!(
                    "Failed to get tenant {}: {}, using default name",
                    tenant_id, e
                );
                "Unknown Tenant".to_string()
            }
        }
    }

    /// Get provider redirect URI
    fn get_provider_redirect_uri(provider: &str) -> String {
        match provider {
            "strava" => crate::constants::env_config::strava_redirect_uri(),
            "fitbit" => crate::constants::env_config::fitbit_redirect_uri(),
            _ => format!(
                "{}/api/oauth/callback/unknown",
                crate::constants::env_config::base_url()
            ),
        }
    }

    /// Get default scopes for OAuth provider
    fn get_provider_scopes(provider: &str) -> Vec<String> {
        match provider {
            crate::constants::oauth_providers::STRAVA => {
                crate::constants::oauth::STRAVA_DEFAULT_SCOPES
                    .split(',')
                    .map(str::to_string)
                    .collect()
            }
            crate::constants::oauth_providers::FITBIT => vec![
                "activity".to_string(),
                "heartrate".to_string(),
                "location".to_string(),
                "nutrition".to_string(),
                "profile".to_string(),
                "settings".to_string(),
                "sleep".to_string(),
                "social".to_string(),
                "weight".to_string(),
            ],
            _ => vec!["read".to_string()], // Default scope
        }
    }
}

/// Template renderer for OAuth success and error pages
pub struct OAuthTemplateRenderer;

impl OAuthTemplateRenderer {
    /// Render OAuth success template
    ///
    /// # Errors
    /// Returns an error if template formatting fails
    pub fn render_success_template(
        provider: &str,
        callback_response: &crate::routes::OAuthCallbackResponse,
        oauth_callback_port: u16,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let template = format!(
            r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>OAuth Success - {}</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; background-color: #f5f5f5; }}
        .container {{ max-width: 600px; margin: 0 auto; background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }}
        .success {{ color: #27ae60; font-size: 24px; margin-bottom: 20px; }}
        .info {{ color: #2c3e50; margin: 10px 0; }}
        .code {{ background: #ecf0f1; padding: 10px; border-radius: 4px; font-family: monospace; }}
    </style>
    <script>
        // Attempt to focus Claude Desktop before closing
        async function focusClaudeDesktop() {{
            try {{
                // Try to trigger focus recovery via bridge communication
                await fetch('http://localhost:{}/oauth/focus-recovery', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ action: 'focus_claude_desktop' }})
                }}).catch(() => {{
                    // Ignore fetch errors - focus recovery is best-effort
                }});
            }} catch (error) {{
                // Silently ignore errors
            }}

            // Close the window after a short delay
            setTimeout(() => {{
                window.close();
            }}, 1500);
        }}

        window.onload = function() {{
            // Start focus recovery immediately
            focusClaudeDesktop();
        }};
    </script>
</head>
<body>
    <div class="container">
        <h1 class="success">✓ OAuth Authorization Successful</h1>
        <div class="info"><strong>Provider:</strong> {}</div>
        <div class="info"><strong>Status:</strong> Connected successfully</div>
        <div class="info"><strong>User ID:</strong> {}</div>
        <div class="info"><strong>Status:</strong> <span class="code">Connected</span></div>
        <p>You can now close this window and return to your MCP client.</p>
        <p><small>Attempting to return focus to your MCP client automatically...</small></p>
    </div>
</body>
</html>
"#,
            provider, oauth_callback_port, provider, callback_response.user_id
        );

        Ok(template)
    }

    /// Render OAuth error template
    ///
    /// # Errors
    /// Returns an error if template formatting fails
    pub fn render_error_template(
        provider: &str,
        error: &str,
        description: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let template = format!(
            r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>OAuth Error - {}</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; background-color: #f5f5f5; }}
        .container {{ max-width: 600px; margin: 0 auto; background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }}
        .error {{ color: #e74c3c; font-size: 24px; margin-bottom: 20px; }}
        .info {{ color: #2c3e50; margin: 10px 0; }}
        .description {{ background: #ffeaa7; padding: 15px; border-radius: 4px; margin: 15px 0; }}
    </style>
</head>
<body>
    <div class="container">
        <h1 class="error">✗ OAuth Authorization Failed</h1>
        <div class="info"><strong>Provider:</strong> {}</div>
        <div class="info"><strong>Error:</strong> {}</div>
        {}
        <p>Please try again or contact support if the problem persists.</p>
    </div>
</body>
</html>
"#,
            provider,
            provider,
            error,
            description
                .map(|d| format!(
                    "<div class=\"description\"><strong>Description:</strong> {d}</div>"
                ))
                .unwrap_or_default()
        );

        Ok(template)
    }
}

/// OAuth notification handler for async notification processing
pub struct OAuthNotificationHandler {
    resources: Arc<ServerResources>,
}

impl OAuthNotificationHandler {
    #[must_use]
    pub const fn new(resources: Arc<ServerResources>) -> Self {
        Self { resources }
    }

    /// Handle OAuth completion notification
    ///
    /// # Errors
    /// Returns an error if notification processing fails
    pub fn handle_notification(
        &self,
        notification: &crate::mcp::schema::OAuthCompletedNotification,
    ) -> Result<()> {
        info!(
            "Processing OAuth notification for user {} with provider {}",
            notification.params.user_id.as_deref().unwrap_or("unknown"),
            notification.params.provider
        );

        // Use resources field by referencing the config
        if notification.params.success {
            info!(
                "OAuth notification processed successfully with HTTP port: {}",
                self.resources.config.http_port
            );
        }

        Ok(())
    }

    /// Spawn a background task to handle OAuth notifications
    pub fn spawn_notification_handler(
        resources: Arc<ServerResources>,
        mut notification_receiver: tokio::sync::broadcast::Receiver<
            crate::mcp::schema::OAuthCompletedNotification,
        >,
    ) {
        let handler = Self::new(resources);

        tokio::spawn(async move {
            info!("OAuth notification handler started");

            while let Ok(notification) = notification_receiver.recv().await {
                if let Err(e) = handler.handle_notification(&notification) {
                    error!("Failed to handle OAuth notification: {}", e);
                }
            }

            warn!("OAuth notification handler ended");
        });
    }
}

// Helper struct for API errors
#[derive(Debug)]
struct ApiError(serde_json::Value);

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "API Error: {}", self.0)
    }
}

impl std::error::Error for ApiError {}

impl warp::reject::Reject for ApiError {}
