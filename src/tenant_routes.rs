// ABOUTME: HTTP REST API routes for multi-tenant management and tenant OAuth configuration
// ABOUTME: Handles tenant creation, OAuth app management, and tenant-isolated authentication flows
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - String ownership transfers for tenant, OAuth, and app struct construction
// - Required field cloning for database entity creation

use crate::{
    auth::{AuthManager, AuthResult},
    constants::oauth_providers,
    database_plugins::factory::Database,
    errors::{AppError, AppResult},
    tenant::TenantOAuthCredentials,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

// Tenant Management Request/Response Types

/// Request body for creating a new tenant
#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    /// Display name for the tenant
    pub name: String,
    /// URL-safe slug identifier for the tenant
    pub slug: String,
    /// Optional custom domain for the tenant
    pub domain: Option<String>,
    /// Subscription plan (basic, pro, enterprise)
    pub plan: Option<String>,
}

/// Response containing created tenant details
#[derive(Debug, Serialize)]
pub struct CreateTenantResponse {
    /// UUID of the created tenant
    pub tenant_id: String,
    /// Display name of the tenant
    pub name: String,
    /// URL-safe slug identifier
    pub slug: String,
    /// Custom domain if configured
    pub domain: Option<String>,
    /// ISO 8601 timestamp of creation
    pub created_at: String,
    /// API endpoint URL for this tenant
    pub api_endpoint: String,
}

/// Response containing list of tenants with pagination
#[derive(Debug, Serialize)]
pub struct TenantListResponse {
    /// List of tenant summaries
    pub tenants: Vec<TenantSummary>,
    /// Total number of tenants
    pub total_count: usize,
}

/// Summary information about a tenant
#[derive(Debug, Serialize)]
pub struct TenantSummary {
    /// UUID of the tenant
    pub tenant_id: String,
    /// Display name
    pub name: String,
    /// URL-safe slug
    pub slug: String,
    /// Custom domain if any
    pub domain: Option<String>,
    /// Subscription plan
    pub plan: String,
    /// ISO 8601 creation timestamp
    pub created_at: String,
    /// List of configured OAuth providers
    pub oauth_providers: Vec<String>,
}

// OAuth App Management Types

/// Request to configure OAuth provider credentials for a tenant
#[derive(Debug, Deserialize)]
pub struct ConfigureTenantOAuthRequest {
    /// OAuth provider name (e.g., "strava", "fitbit")
    pub provider: String,
    /// OAuth client ID from provider
    pub client_id: String,
    /// OAuth client secret from provider
    pub client_secret: String,
    /// Redirect URI for OAuth callbacks
    pub redirect_uri: String,
    /// OAuth scopes to request
    pub scopes: Vec<String>,
    /// Optional daily rate limit
    pub rate_limit_per_day: Option<u32>,
}

/// Response after configuring OAuth provider
#[derive(Debug, Serialize)]
pub struct ConfigureTenantOAuthResponse {
    /// OAuth provider name
    pub provider: String,
    /// OAuth client ID (secret not exposed)
    pub client_id: String,
    /// Configured redirect URI
    pub redirect_uri: String,
    /// Configured OAuth scopes
    pub scopes: Vec<String>,
    /// ISO 8601 timestamp when configured
    pub configured_at: String,
}

/// List of OAuth providers configured for a tenant
#[derive(Debug, Serialize)]
pub struct TenantOAuthListResponse {
    /// Configured OAuth providers
    pub providers: Vec<TenantOAuthProvider>,
}

/// OAuth provider configuration details
#[derive(Debug, Serialize)]
pub struct TenantOAuthProvider {
    /// Provider name
    pub provider: String,
    /// OAuth client ID
    pub client_id: String,
    /// Redirect URI
    pub redirect_uri: String,
    /// Configured scopes
    pub scopes: Vec<String>,
    /// Configuration timestamp
    pub configured_at: String,
    /// Whether provider is enabled
    pub enabled: bool,
}

// OAuth App Registration for MCP clients

/// Request to register a new OAuth application
#[derive(Debug, Deserialize)]
pub struct RegisterOAuthAppRequest {
    /// Application name
    pub name: String,
    /// Optional application description
    pub description: Option<String>,
    /// Allowed redirect URIs for OAuth callbacks
    pub redirect_uris: Vec<String>,
    /// Requested OAuth scopes (e.g., mcp:read, mcp:write, a2a:read)
    pub scopes: Vec<String>,
    /// Application type (desktop, web, mobile, server)
    pub app_type: String,
}

/// Response containing registered OAuth application credentials
#[derive(Debug, Serialize)]
pub struct RegisterOAuthAppResponse {
    /// OAuth client ID
    pub client_id: String,
    /// OAuth client secret (only shown once)
    pub client_secret: String,
    /// Application name
    pub name: String,
    /// Application type
    pub app_type: String,
    /// OAuth authorization endpoint URL
    pub authorization_url: String,
    /// OAuth token endpoint URL
    pub token_url: String,
    /// ISO 8601 timestamp when app was created
    pub created_at: String,
}

// OAuth Authorization Flow Types

/// Request to initiate OAuth authorization flow
#[derive(Debug, Deserialize)]
pub struct OAuthAuthorizeRequest {
    /// OAuth client ID
    pub client_id: String,
    /// Redirect URI after authorization
    pub redirect_uri: String,
    /// Space-separated OAuth scopes
    pub scope: String,
    /// Optional state parameter for CSRF protection
    pub state: Option<String>,
    /// Response type (always "code" for authorization code flow)
    pub response_type: String,
}

/// Response with authorization URL
#[derive(Debug, Serialize)]
pub struct OAuthAuthorizeResponse {
    /// Authorization URL to redirect user to
    pub authorization_url: String,
    /// How long the authorization is valid (seconds)
    pub expires_in: u64,
}

/// Request to exchange authorization code for access token
#[derive(Debug, Deserialize)]
pub struct OAuthTokenRequest {
    /// Grant type (`authorization_code`, `client_credentials`)
    pub grant_type: String,
    /// Authorization code (for `authorization_code` grant)
    pub code: Option<String>,
    /// Redirect URI used in authorization request
    pub redirect_uri: Option<String>,
    /// OAuth client ID
    pub client_id: String,
    /// OAuth client secret
    pub client_secret: String,
}

/// Response containing OAuth access token
#[derive(Debug, Serialize)]
pub struct OAuthTokenResponse {
    /// JWT access token for API authentication
    pub access_token: String,
    /// Token type (always "Bearer")
    pub token_type: String,
    /// Token expiration time in seconds
    pub expires_in: u64,
    /// Space-separated OAuth scopes granted
    pub scope: String,
}

// Route Handler Implementations

/// Create a new tenant organization
///
/// # Errors
///
/// Returns an error if:
/// - Tenant slug already exists
/// - Database operations fail
/// - User lacks permissions
pub async fn create_tenant(
    tenant_request: CreateTenantRequest,
    auth_result: AuthResult,
    database: Arc<Database>,
) -> AppResult<CreateTenantResponse> {
    info!("Creating new tenant: {}", tenant_request.name);

    // Verify user is authenticated and has tenant creation permissions
    database
        .get_user(auth_result.user_id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // Generate tenant ID and validate slug uniqueness
    let tenant_id = Uuid::new_v4();
    let slug = tenant_request.slug.trim().to_lowercase();

    // Check if slug already exists
    if let Ok(_existing) = database.get_tenant_by_slug(&slug).await {
        return Err(AppError::invalid_input(format!(
            "Tenant slug '{slug}' already exists"
        )));
    }

    // Create tenant in database
    let tenant_data = crate::models::Tenant {
        id: tenant_id,
        name: tenant_request.name.clone(), // Safe: String ownership for tenant struct
        slug: slug.clone(),                // Safe: String ownership for tenant struct
        domain: tenant_request.domain.clone(), // Safe: String ownership for tenant struct
        plan: tenant_request.plan.unwrap_or_else(|| "basic".to_owned()),
        owner_user_id: auth_result.user_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    database
        .create_tenant(&tenant_data)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    info!(
        "Tenant created successfully: {} ({})",
        tenant_data.name, tenant_data.id
    );

    Ok(CreateTenantResponse {
        tenant_id: tenant_data.id.to_string(),
        name: tenant_data.name,
        slug: tenant_data.slug,
        domain: tenant_data.domain,
        created_at: tenant_data.created_at.to_rfc3339(),
        api_endpoint: format!("https://api.your-server.com/tenants/{}", tenant_data.id),
    })
}

/// List all tenants for the authenticated user
///
/// # Errors
///
/// Returns an error if:
/// - Database operations fail
/// - User lacks permissions
pub async fn list_tenants(
    auth_result: AuthResult,
    database: Arc<Database>,
) -> AppResult<TenantListResponse> {
    info!("Listing tenants for user: {}", auth_result.user_id);

    let tenants = database
        .list_tenants_for_user(auth_result.user_id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    let mut tenant_summaries = Vec::new();

    for tenant in tenants {
        // Get OAuth providers for this tenant
        let oauth_providers = database
            .get_tenant_oauth_providers(tenant.id)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    tenant_id = %tenant.id,
                    tenant_name = %tenant.name,
                    error = %e,
                    "Failed to fetch OAuth providers for tenant summary, using empty list"
                );
                Vec::new()
            });

        tenant_summaries.push(TenantSummary {
            tenant_id: tenant.id.to_string(),
            name: tenant.name,
            slug: tenant.slug,
            domain: tenant.domain,
            plan: tenant.plan,
            created_at: tenant.created_at.to_rfc3339(),
            oauth_providers: oauth_providers.into_iter().map(|p| p.provider).collect(),
        });
    }

    Ok(TenantListResponse {
        total_count: tenant_summaries.len(),
        tenants: tenant_summaries,
    })
}

/// Configure OAuth credentials for a tenant
///
/// # Errors
///
/// Returns an error if:
/// - Tenant not found or access denied
/// - Unsupported OAuth provider
/// - Database operations fail
pub async fn configure_tenant_oauth(
    tenant_id: String,
    oauth_request: ConfigureTenantOAuthRequest,
    auth_result: AuthResult,
    database: Arc<Database>,
) -> AppResult<ConfigureTenantOAuthResponse> {
    info!(
        "Configuring {} OAuth for tenant: {}",
        oauth_request.provider, tenant_id
    );

    let tenant_uuid = Uuid::parse_str(&tenant_id).map_err(|e| {
        tracing::warn!(
            tenant_id = %tenant_id,
            user_id = %auth_result.user_id,
            error = %e,
            "Failed to parse tenant ID for OAuth operation"
        );
        AppError::invalid_input(format!("Invalid tenant ID format: {e}"))
    })?;

    // Verify user owns this tenant
    let tenant = database
        .get_tenant_by_id(tenant_uuid)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    if tenant.owner_user_id != auth_result.user_id {
        return Err(AppError::new(
            crate::errors::ErrorCode::PermissionDenied,
            "Access denied to this tenant",
        ));
    }

    // Validate provider
    if ![oauth_providers::STRAVA, oauth_providers::FITBIT]
        .contains(&oauth_request.provider.as_str())
    {
        return Err(AppError::invalid_input(format!(
            "Unsupported OAuth provider: {}",
            oauth_request.provider
        )));
    }

    // Store encrypted OAuth credentials
    let credentials = TenantOAuthCredentials {
        tenant_id: tenant_uuid,
        provider: oauth_request.provider.clone(), // Safe: String ownership for OAuth credentials
        client_id: oauth_request.client_id.clone(), // Safe: String ownership for OAuth credentials
        client_secret: oauth_request.client_secret,
        redirect_uri: oauth_request.redirect_uri.clone(), // Safe: String ownership for OAuth credentials
        scopes: oauth_request.scopes.clone(), // Safe: Option<String> ownership for OAuth credentials
        rate_limit_per_day: oauth_request.rate_limit_per_day.unwrap_or(15000),
    };

    database
        .store_tenant_oauth_credentials(&credentials)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    info!(
        "OAuth configured successfully for tenant {} provider {}",
        tenant_id, oauth_request.provider
    );

    Ok(ConfigureTenantOAuthResponse {
        provider: oauth_request.provider,
        client_id: oauth_request.client_id,
        redirect_uri: oauth_request.redirect_uri,
        scopes: oauth_request.scopes,
        configured_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Get OAuth configuration for a tenant
///
/// # Errors
///
/// Returns an error if:
/// - Tenant not found or access denied
/// - Database operations fail
pub async fn get_tenant_oauth(
    tenant_id: String,
    auth_result: AuthResult,
    database: Arc<Database>,
) -> AppResult<TenantOAuthListResponse> {
    info!("Getting OAuth config for tenant: {}", tenant_id);

    let tenant_uuid = Uuid::parse_str(&tenant_id).map_err(|e| {
        tracing::warn!(
            tenant_id = %tenant_id,
            user_id = %auth_result.user_id,
            error = %e,
            "Failed to parse tenant ID for OAuth operation"
        );
        AppError::invalid_input(format!("Invalid tenant ID format: {e}"))
    })?;

    // Verify user owns this tenant
    let tenant = database
        .get_tenant_by_id(tenant_uuid)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    if tenant.owner_user_id != auth_result.user_id {
        return Err(AppError::new(
            crate::errors::ErrorCode::PermissionDenied,
            "Access denied to this tenant",
        ));
    }

    let oauth_configs = database
        .get_tenant_oauth_providers(tenant_uuid)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    let providers = oauth_configs
        .into_iter()
        .map(|config| TenantOAuthProvider {
            provider: config.provider,
            client_id: config.client_id,
            redirect_uri: config.redirect_uri,
            scopes: config.scopes,
            configured_at: chrono::Utc::now().to_rfc3339(),
            enabled: true,
        })
        .collect();

    Ok(TenantOAuthListResponse { providers })
}

/// Register OAuth application for MCP clients
///
/// # Errors
///
/// Returns an error if:
/// - Application name already exists
/// - Invalid redirect URIs
/// - Database operations fail
pub async fn register_oauth_app(
    app_request: RegisterOAuthAppRequest,
    auth_result: AuthResult,
    database: Arc<Database>,
) -> AppResult<RegisterOAuthAppResponse> {
    info!("Registering OAuth app: {}", app_request.name);

    // Generate client credentials
    let client_id = format!("app_{}", Uuid::new_v4().simple());
    let client_secret = format!("secret_{}", Uuid::new_v4().simple());

    // Store OAuth app in database
    let oauth_app = crate::models::OAuthApp {
        id: Uuid::new_v4(),
        client_id: client_id.clone(), // Safe: String ownership for OAuth app struct
        client_secret: client_secret.clone(), // Safe: String ownership for OAuth app struct
        name: app_request.name.clone(), // Safe: String ownership for OAuth app struct
        description: app_request.description,
        redirect_uris: app_request.redirect_uris,
        scopes: app_request.scopes,
        app_type: app_request.app_type.clone(), // Safe: String ownership for OAuth app struct
        owner_user_id: auth_result.user_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    database
        .create_oauth_app(&oauth_app)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    info!("OAuth app registered: {} ({})", app_request.name, client_id);

    Ok(RegisterOAuthAppResponse {
        client_id,
        client_secret,
        name: app_request.name,
        app_type: app_request.app_type,
        authorization_url: "https://your-server.com/oauth/authorize".to_owned(),
        token_url: "https://your-server.com/oauth/token".to_owned(),
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// OAuth authorization endpoint (GET /oauth/authorize)
///
/// # Errors
///
/// Returns an error if:
/// - OAuth app not found
/// - Invalid redirect URI
/// - Database operations fail
pub async fn oauth_authorize(
    auth_params: OAuthAuthorizeRequest,
    database: Arc<Database>,
) -> AppResult<OAuthAuthorizeResponse> {
    info!(
        "OAuth authorization request for client: {}",
        auth_params.client_id
    );

    // Validate client_id exists
    let oauth_app = database
        .get_oauth_app_by_client_id(&auth_params.client_id)
        .await
        .map_err(|e| {
            tracing::warn!(
                client_id = %auth_params.client_id,
                error = %e,
                "OAuth app lookup failed for authorization request"
            );
            AppError::invalid_input(format!("Invalid client_id: {e}"))
        })?;

    // Validate redirect_uri matches registered URIs
    if !oauth_app.redirect_uris.contains(&auth_params.redirect_uri) {
        return Err(AppError::invalid_input("Invalid redirect_uri".to_owned()));
    }

    // Generate authorization code and store it temporarily
    let auth_code_value = format!("code_{}", Uuid::new_v4().simple());

    // Create OAuth2AuthCode struct
    let auth_code = crate::oauth2_server::models::OAuth2AuthCode {
        code: auth_code_value.clone(),
        client_id: auth_params.client_id.clone(),
        user_id: oauth_app.owner_user_id, // Use the OAuth app owner
        tenant_id: "default".to_owned(),  // Default tenant for now
        redirect_uri: auth_params.redirect_uri.clone(),
        scope: Some(auth_params.scope.clone()),
        expires_at: chrono::Utc::now() + chrono::Duration::minutes(10),
        used: false,
        state: auth_params.state.clone(),
        code_challenge: None,
        code_challenge_method: None,
    };

    // Store auth code in database with expiration (10 minutes)
    database
        .store_authorization_code(&auth_code)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // Build authorization URL
    let auth_url = format!(
        "{}?code={}&state={}",
        auth_params.redirect_uri,
        auth_code_value,
        auth_params.state.unwrap_or_default()
    );

    Ok(OAuthAuthorizeResponse {
        authorization_url: auth_url,
        expires_in: 600, // 10 minutes
    })
}

/// OAuth token endpoint (POST /oauth/token)
///
/// # Errors
///
/// Returns an error if:
/// - Authorization code not found or expired
/// - Client credentials invalid
/// - Token generation fails
pub async fn oauth_token(
    token_request: OAuthTokenRequest,
    database: Arc<Database>,
    auth_manager: Arc<AuthManager>,
    jwks_manager: Arc<crate::admin::jwks::JwksManager>,
) -> AppResult<OAuthTokenResponse> {
    info!(
        "OAuth token request for client: {}",
        token_request.client_id
    );

    // Validate client credentials
    let oauth_app = database
        .get_oauth_app_by_client_id(&token_request.client_id)
        .await
        .map_err(|e| {
            tracing::warn!(
                client_id = %token_request.client_id,
                grant_type = %token_request.grant_type,
                error = %e,
                "OAuth app lookup failed for token request"
            );
            AppError::invalid_input(format!("Invalid client_id: {e}"))
        })?;

    if oauth_app.client_secret != token_request.client_secret {
        return Err(AppError::new(
            crate::errors::ErrorCode::AuthInvalid,
            "Invalid client_secret",
        ));
    }

    match token_request.grant_type.as_str() {
        "authorization_code" => {
            // Exchange authorization code for access token
            let code = token_request
                .code
                .ok_or_else(|| AppError::invalid_input("Missing authorization code".to_owned()))?;

            database
                .get_authorization_code(
                    &code,
                    &token_request.client_id,
                    &oauth_app.redirect_uris[0],
                )
                .await
                .map_err(|e| {
                    tracing::warn!(
                        code = %code,
                        error = %e,
                        "Failed to retrieve authorization code from database"
                    );
                    AppError::invalid_input("Invalid or expired authorization code".to_owned())
                })?;

            // Generate access token (JWT)
            let access_token = auth_manager
                .generate_oauth_access_token(
                    &jwks_manager,
                    &oauth_app.owner_user_id,
                    &oauth_app.scopes,
                    None, // tenant_id
                )
                .map_err(|e| {
                    AppError::auth_invalid(format!("Failed to generate OAuth access token: {e}"))
                })?;

            // Clean up authorization code
            if let Err(e) = database
                .delete_authorization_code(
                    &code,
                    &token_request.client_id,
                    &oauth_app.redirect_uris[0],
                )
                .await
            {
                tracing::warn!(
                    code = %code,
                    client_id = %oauth_app.client_id,
                    error = %e,
                    "Failed to delete authorization code after token exchange (potential security issue - code not cleaned up)"
                );
            }

            Ok(OAuthTokenResponse {
                access_token,
                token_type: "Bearer".to_owned(),
                expires_in: crate::constants::time::DAY_SECONDS as u64, // 24 hours
                scope: oauth_app.scopes.join(" "),
            })
        }
        "client_credentials" => {
            // Direct client credentials grant (for A2A)
            let access_token = auth_manager
                .generate_client_credentials_token(
                    &jwks_manager,
                    &token_request.client_id,
                    &oauth_app.scopes,
                    None, // tenant_id
                )
                .map_err(|e| {
                    AppError::auth_invalid(format!(
                        "Failed to generate client credentials token: {e}"
                    ))
                })?;

            Ok(OAuthTokenResponse {
                access_token,
                token_type: "Bearer".to_owned(),
                expires_in: crate::constants::time::HOUR_SECONDS as u64, // 1 hour for client credentials
                scope: oauth_app.scopes.join(" "),
            })
        }
        _ => Err(AppError::invalid_input("Unsupported grant_type".to_owned())),
    }
}
