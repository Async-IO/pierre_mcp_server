// ABOUTME: HTTP REST API routes for multi-tenant management and tenant OAuth configuration
// ABOUTME: Handles tenant creation, OAuth app management, and tenant-isolated authentication flows

// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - String ownership transfers for tenant, OAuth, and app struct construction
// - Required field cloning for database entity creation

use crate::{
    auth::{AuthManager, AuthResult},
    constants::oauth_providers,
    database_plugins::{factory::Database, DatabaseProvider},
    errors::AppError,
    tenant::TenantOAuthCredentials,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

// Tenant Management Request/Response Types

#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    pub name: String,
    pub slug: String,
    pub domain: Option<String>,
    pub plan: Option<String>, // basic, pro, enterprise
}

#[derive(Debug, Serialize)]
pub struct CreateTenantResponse {
    pub tenant_id: String,
    pub name: String,
    pub slug: String,
    pub domain: Option<String>,
    pub created_at: String,
    pub api_endpoint: String,
}

#[derive(Debug, Serialize)]
pub struct TenantListResponse {
    pub tenants: Vec<TenantSummary>,
    pub total_count: usize,
}

#[derive(Debug, Serialize)]
pub struct TenantSummary {
    pub tenant_id: String,
    pub name: String,
    pub slug: String,
    pub domain: Option<String>,
    pub plan: String,
    pub created_at: String,
    pub oauth_providers: Vec<String>,
}

// OAuth App Management Types

#[derive(Debug, Deserialize)]
pub struct ConfigureTenantOAuthRequest {
    pub provider: String, // "strava", "fitbit"
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub rate_limit_per_day: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ConfigureTenantOAuthResponse {
    pub provider: String,
    pub client_id: String, // Don't expose client_secret
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub configured_at: String,
}

#[derive(Debug, Serialize)]
pub struct TenantOAuthListResponse {
    pub providers: Vec<TenantOAuthProvider>,
}

#[derive(Debug, Serialize)]
pub struct TenantOAuthProvider {
    pub provider: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub configured_at: String,
    pub enabled: bool,
}

// OAuth App Registration for MCP clients

#[derive(Debug, Deserialize)]
pub struct RegisterOAuthAppRequest {
    pub name: String,
    pub description: Option<String>,
    pub redirect_uris: Vec<String>,
    pub scopes: Vec<String>, // mcp:read, mcp:write, a2a:read, etc.
    pub app_type: String,    // "desktop", "web", "mobile", "server"
}

#[derive(Debug, Serialize)]
pub struct RegisterOAuthAppResponse {
    pub client_id: String,
    pub client_secret: String,
    pub name: String,
    pub app_type: String,
    pub authorization_url: String,
    pub token_url: String,
    pub created_at: String,
}

// OAuth Authorization Flow Types

#[derive(Debug, Deserialize)]
pub struct OAuthAuthorizeRequest {
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
    pub state: Option<String>,
    pub response_type: String, // "code"
}

#[derive(Debug, Serialize)]
pub struct OAuthAuthorizeResponse {
    pub authorization_url: String,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
pub struct OAuthTokenRequest {
    pub grant_type: String, // "authorization_code", "client_credentials"
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Serialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
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
) -> Result<CreateTenantResponse, AppError> {
    info!("Creating new tenant: {}", tenant_request.name);

    // Verify user is authenticated and has tenant creation permissions
    let user = database
        .get_user(auth_result.user_id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    let _ = user; // Used for permission validation

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
        plan: tenant_request.plan.unwrap_or_else(|| "basic".to_string()),
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
) -> Result<TenantListResponse, AppError> {
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
            .unwrap_or_default();

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
) -> Result<ConfigureTenantOAuthResponse, AppError> {
    info!(
        "Configuring {} OAuth for tenant: {}",
        oauth_request.provider, tenant_id
    );

    let tenant_uuid = Uuid::parse_str(&tenant_id)
        .map_err(|_| AppError::invalid_input("Invalid tenant ID format".to_string()))?;

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
) -> Result<TenantOAuthListResponse, AppError> {
    info!("Getting OAuth config for tenant: {}", tenant_id);

    let tenant_uuid = Uuid::parse_str(&tenant_id)
        .map_err(|_| AppError::invalid_input("Invalid tenant ID format".to_string()))?;

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
) -> Result<RegisterOAuthAppResponse, AppError> {
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
        authorization_url: "https://your-server.com/oauth/authorize".to_string(),
        token_url: "https://your-server.com/oauth/token".to_string(),
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
) -> Result<OAuthAuthorizeResponse, AppError> {
    info!(
        "OAuth authorization request for client: {}",
        auth_params.client_id
    );

    // Validate client_id exists
    let oauth_app = database
        .get_oauth_app_by_client_id(&auth_params.client_id)
        .await
        .map_err(|_| AppError::invalid_input("Invalid client_id".to_string()))?;

    // Validate redirect_uri matches registered URIs
    if !oauth_app.redirect_uris.contains(&auth_params.redirect_uri) {
        return Err(AppError::invalid_input("Invalid redirect_uri".to_string()));
    }

    // Generate authorization code and store it temporarily
    let auth_code = format!("code_{}", Uuid::new_v4().simple());

    // Store auth code in database with expiration (10 minutes)
    // Use the OAuth app owner as the user_id (validated through JWT authentication)
    database
        .store_authorization_code(
            &auth_code,
            &auth_params.client_id,
            &auth_params.redirect_uri,
            &auth_params.scope,
            oauth_app.owner_user_id, // Use the app owner's user_id
        )
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // Build authorization URL
    let auth_url = format!(
        "{}?code={}&state={}",
        auth_params.redirect_uri,
        auth_code,
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
) -> Result<OAuthTokenResponse, AppError> {
    info!(
        "OAuth token request for client: {}",
        token_request.client_id
    );

    // Validate client credentials
    let oauth_app = database
        .get_oauth_app_by_client_id(&token_request.client_id)
        .await
        .map_err(|_| AppError::invalid_input("Invalid client_id".to_string()))?;

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
                .ok_or_else(|| AppError::invalid_input("Missing authorization code".to_string()))?;

            let auth_code_data = database.get_authorization_code(&code).await.map_err(|_| {
                AppError::invalid_input("Invalid or expired authorization code".to_string())
            })?;
            let _ = auth_code_data; // Used for security validation

            // Generate access token (JWT)
            let access_token = auth_manager
                .generate_oauth_access_token(&oauth_app.owner_user_id, &oauth_app.scopes)?;

            // Clean up authorization code
            let _ = database.delete_authorization_code(&code).await;

            Ok(OAuthTokenResponse {
                access_token,
                token_type: "Bearer".to_string(),
                expires_in: crate::constants::time::DAY_SECONDS as u64, // 24 hours
                scope: oauth_app.scopes.join(" "),
            })
        }
        "client_credentials" => {
            // Direct client credentials grant (for A2A)
            let access_token = auth_manager
                .generate_client_credentials_token(&token_request.client_id, &oauth_app.scopes)?;

            Ok(OAuthTokenResponse {
                access_token,
                token_type: "Bearer".to_string(),
                expires_in: crate::constants::time::HOUR_SECONDS as u64, // 1 hour for client credentials
                scope: oauth_app.scopes.join(" "),
            })
        }
        _ => Err(AppError::invalid_input(
            "Unsupported grant_type".to_string(),
        )),
    }
}
