// ABOUTME: OAuth flow management for multi-tenant MCP server
// ABOUTME: Handles authorization requests, callbacks, token processing, and template rendering
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::database_plugins::DatabaseProvider;
use crate::mcp::resources::ServerResources;
use crate::tenant::{TenantContext, TenantRole};
use crate::utils::json_responses::api_error;
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
    ) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
        // Parse user_id - return HTML error if invalid
        let Ok(user_id) = Uuid::parse_str(&user_id_str) else {
            error!("Invalid user_id format: {}", user_id_str);
            return match Self::create_html_error_response(
                &provider,
                "Invalid user ID format",
                Some("The user ID provided in the URL is not a valid UUID format."),
            ) {
                Ok(reply) => Ok(Box::new(reply) as Box<dyn warp::Reply>),
                Err(e) => Err(e),
            };
        };

        // Get user - return HTML error if not found
        let Ok(user) = self.get_user_with_tenant(user_id).await else {
            error!("User {} not found", user_id);
            return match Self::create_html_error_response(
                &provider,
                "User account not found",
                Some("The user account associated with this OAuth request could not be found."),
            ) {
                Ok(reply) => Ok(Box::new(reply) as Box<dyn warp::Reply>),
                Err(e) => Err(e),
            };
        };

        // Extract tenant_id - return HTML error if missing
        let Ok(tenant_id) = Self::extract_tenant_id(&user) else {
            error!("Missing or invalid tenant for user {}", user_id);
            return match Self::create_html_error_response(
                &provider,
                "Tenant configuration error",
                Some("User does not belong to any tenant or tenant ID is invalid."),
            ) {
                Ok(reply) => Ok(Box::new(reply) as Box<dyn warp::Reply>),
                Err(e) => Err(e),
            };
        };

        // Process credentials - return HTML error if fails
        if self
            .process_oauth_credentials(&headers, &provider, tenant_id, user_id)
            .await
            .is_err()
        {
            error!("Failed to process OAuth credentials for user {}", user_id);
            return match Self::create_html_error_response(
                &provider,
                "OAuth configuration error",
                Some("Failed to store OAuth credentials. Please check your configuration."),
            ) {
                Ok(reply) => Ok(Box::new(reply) as Box<dyn warp::Reply>),
                Err(e) => Err(e),
            };
        }

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
            .and_then(|h| {
                h.to_str()
                    .inspect_err(|e| {
                        tracing::debug!(
                            provider = %provider,
                            error = ?e,
                            "Failed to parse OAuth client ID header as UTF-8"
                        );
                    })
                    .ok()
            })
            .map(str::to_owned);

        let user_client_secret = headers
            .get(format!("x-{}-client-secret", provider.to_lowercase()))
            .and_then(|h| {
                h.to_str()
                    .inspect_err(|e| {
                        tracing::debug!(
                            provider = %provider,
                            error = ?e,
                            "Failed to parse OAuth client secret header as UTF-8"
                        );
                    })
                    .ok()
            })
            .map(str::to_owned);

        // Only store credentials if both client_id and client_secret are provided
        if let (Some(client_id), Some(client_secret)) = (user_client_id, user_client_secret) {
            info!(
                "Using user-provided OAuth credentials for tenant {} and provider {}",
                tenant_id, provider
            );

            let redirect_uri = self.get_provider_redirect_uri(provider);
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
    ) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
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
                auth_url.parse::<warp::http::Uri>().map_or_else(
                    |_| {
                        error!("Invalid authorization URL format: {}", auth_url);
                        match Self::create_html_error_response(
                            provider,
                            "Invalid authorization URL",
                            Some("The generated authorization URL has an invalid format."),
                        ) {
                            Ok(reply) => Ok(Box::new(reply) as Box<dyn warp::Reply>),
                            Err(e) => Err(e),
                        }
                    },
                    |uri| Ok(Box::new(warp::redirect::found(uri)) as Box<dyn warp::Reply>),
                )
            }
            Err(e) => {
                error!(
                    "Failed to generate authorization URL for provider {}: {}",
                    provider, e
                );
                match Self::create_html_error_response(
                    provider,
                    "Failed to generate authorization URL",
                    Some("Could not generate the authorization URL. Please check your OAuth configuration."),
                ) {
                    Ok(reply) => Ok(Box::new(reply) as Box<dyn warp::Reply>),
                    Err(e) => Err(e),
                }
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

        tenant_id_str.parse().map_err(|e| {
            error!(
                "Invalid tenant ID format - user_id: {}, email: {}, tenant_id_str: {}, error: {}. Failed to parse tenant ID as UUID.",
                user.id, user.email, tenant_id_str, e
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
                "Unknown Tenant".to_owned()
            }
        }
    }

    /// Get provider redirect URI
    fn get_provider_redirect_uri(&self, provider: &str) -> String {
        match provider {
            "strava" => self
                .resources
                .config
                .oauth
                .strava
                .redirect_uri
                .clone() // Safe: Config string ownership for OAuth request setup
                .unwrap_or_else(|| {
                    format!(
                        "http://localhost:{}/api/oauth/callback/strava",
                        self.resources.config.http_port
                    )
                }),
            "fitbit" => self
                .resources
                .config
                .oauth
                .fitbit
                .redirect_uri
                .clone() // Safe: Config string ownership for OAuth request setup
                .unwrap_or_else(|| {
                    format!(
                        "http://localhost:{}/api/oauth/callback/fitbit",
                        self.resources.config.http_port
                    )
                }),
            _ => format!(
                "http://localhost:{}/api/oauth/callback/unknown",
                self.resources.config.http_port
            ),
        }
    }

    /// Get default scopes for OAuth provider
    fn get_provider_scopes(provider: &str) -> Vec<String> {
        match provider {
            crate::constants::oauth_providers::STRAVA => {
                crate::constants::oauth::STRAVA_DEFAULT_SCOPES
                    .split(',')
                    .map(str::to_owned)
                    .collect()
            }
            crate::constants::oauth_providers::FITBIT => vec![
                "activity".to_owned(),
                "heartrate".to_owned(),
                "location".to_owned(),
                "nutrition".to_owned(),
                "profile".to_owned(),
                "settings".to_owned(),
                "sleep".to_owned(),
                "social".to_owned(),
                "weight".to_owned(),
            ],
            _ => vec!["read".to_owned()], // Default scope
        }
    }

    /// Create HTML error response using the OAuth template renderer
    fn create_html_error_response(
        provider: &str,
        error: &str,
        description: Option<&str>,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        match OAuthTemplateRenderer::render_error_template(provider, error, description) {
            Ok(html) => Ok(warp::reply::with_status(
                warp::reply::html(html),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            )),
            Err(e) => {
                error!("Failed to render OAuth error template: {}", e);
                Err(warp::reject::custom(ApiError(api_error(
                    "Failed to render error page",
                ))))
            }
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
    ) -> Result<String, Box<dyn std::error::Error>> {
        const TEMPLATE: &str = include_str!("../../templates/oauth_success.html");

        let rendered = TEMPLATE
            .replace("{{PROVIDER}}", provider)
            .replace("{{USER_ID}}", &callback_response.user_id);

        Ok(rendered)
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
        const TEMPLATE: &str = include_str!("../../templates/oauth_error.html");

        let description_html = description
            .map(|d| format!("<div class=\"description\"><strong>Description:</strong> {d}</div>"))
            .unwrap_or_default();

        let rendered = TEMPLATE
            .replace("{{PROVIDER}}", provider)
            .replace("{{ERROR}}", error)
            .replace("{{DESCRIPTION}}", &description_html);

        Ok(rendered)
    }
}

/// OAuth notification handler for async notification processing
pub struct OAuthNotificationHandler {
    resources: Arc<ServerResources>,
}

impl OAuthNotificationHandler {
    /// Creates a new OAuth notification handler
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
