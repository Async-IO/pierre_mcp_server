// ABOUTME: API endpoints for per-tenant and per-user LLM provider settings
// ABOUTME: Enables configuration of Gemini, Groq, and local LLM API keys via the frontend
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use pierre_core::models::TenantId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::auth::AuthResult;
use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::errors::AppError;
use crate::llm::ChatProvider;
use crate::mcp::resources::ServerResources;
use crate::security::cookies::get_cookie_value;
use crate::tenant::llm_manager::{
    CredentialSource, LlmCredentialSummary, LlmCredentials, LlmProvider,
    StoreLlmCredentialsRequest, TenantLlmManager,
};

/// Request to save LLM credentials
#[derive(Debug, Deserialize)]
pub struct SaveLlmCredentialsRequest {
    /// Provider name (gemini, groq, local)
    pub provider: String,
    /// API key
    pub api_key: String,
    /// Base URL (for local providers only)
    #[serde(default)]
    pub base_url: Option<String>,
    /// Default model (optional override)
    #[serde(default)]
    pub default_model: Option<String>,
    /// Scope: "user" for user-specific, "tenant" for tenant-wide default
    #[serde(default = "default_scope")]
    pub scope: String,
}

fn default_scope() -> String {
    "user".to_owned()
}

/// Request to validate LLM credentials (without saving)
#[derive(Debug, Deserialize)]
pub struct ValidateLlmCredentialsRequest {
    /// Provider name (gemini, groq, local)
    pub provider: String,
    /// API key to validate
    pub api_key: String,
    /// Base URL (for local providers only)
    #[serde(default)]
    pub base_url: Option<String>,
}

/// Response for LLM settings
#[derive(Debug, Serialize)]
pub struct LlmSettingsResponse {
    /// Current effective provider
    pub current_provider: Option<String>,
    /// Available providers with their configuration status
    pub providers: Vec<ProviderStatus>,
    /// User-specific credentials
    pub user_credentials: Vec<LlmCredentialSummary>,
    /// Tenant-level credentials (visible to admins)
    pub tenant_credentials: Vec<LlmCredentialSummary>,
}

/// Status of a provider
#[derive(Debug, Serialize)]
pub struct ProviderStatus {
    /// Provider name
    pub name: String,
    /// Display name
    pub display_name: String,
    /// Whether credentials are configured at any level
    pub has_credentials: bool,
    /// Credential source (user, tenant, environment)
    pub credential_source: Option<String>,
    /// Whether this provider is currently active
    pub is_active: bool,
}

/// Response for validation
#[derive(Debug, Serialize)]
pub struct ValidationResponse {
    /// Whether the credentials are valid
    pub valid: bool,
    /// Provider name on success
    pub provider: Option<String>,
    /// Available models on success
    pub models: Option<Vec<String>>,
    /// Error message on failure
    pub error: Option<String>,
}

/// Response for save operation
#[derive(Debug, Serialize)]
pub struct SaveCredentialsResponse {
    /// Whether save was successful
    pub success: bool,
    /// Credential ID
    pub id: Option<String>,
    /// Message
    pub message: String,
}

/// LLM settings routes container
pub struct LlmSettingsRoutes;

impl LlmSettingsRoutes {
    /// Create all LLM settings routes
    pub fn routes(resources: Arc<ServerResources>) -> Router {
        Router::new()
            .route("/api/user/llm-settings", get(Self::get_llm_settings))
            .route("/api/user/llm-settings", put(Self::save_llm_credentials))
            .route(
                "/api/user/llm-settings/validate",
                post(Self::validate_llm_credentials),
            )
            .route(
                "/api/user/llm-settings/:provider",
                delete(Self::delete_llm_credentials),
            )
            .with_state(resources)
    }

    /// Extract and authenticate user from authorization header or cookie
    async fn authenticate(
        headers: &HeaderMap,
        resources: &Arc<ServerResources>,
    ) -> Result<AuthResult, AppError> {
        // Try Authorization header first, then fall back to auth_token cookie
        let auth_value =
            if let Some(auth_header) = headers.get("authorization").and_then(|h| h.to_str().ok()) {
                auth_header.to_owned()
            } else if let Some(token) = get_cookie_value(headers, "auth_token") {
                // Fall back to auth_token cookie, format as Bearer token
                format!("Bearer {token}")
            } else {
                return Err(AppError::auth_invalid(
                    "Missing authorization header or cookie",
                ));
            };

        resources
            .auth_middleware
            .authenticate_request(Some(&auth_value))
            .await
            .map_err(|e| AppError::auth_invalid(format!("Authentication failed: {e}")))
    }

    /// Get user's `tenant_id` for the current request
    ///
    /// Uses `active_tenant_id` from JWT claims (user's selected tenant) when available,
    /// falling back to the user's first tenant, or `user_id` if no tenant exists.
    async fn get_tenant_id(
        auth: &AuthResult,
        resources: &Arc<ServerResources>,
    ) -> Result<TenantId, AppError> {
        // Prefer active_tenant_id from JWT claims (user's selected tenant)
        if let Some(tenant_id) = auth.active_tenant_id {
            return Ok(TenantId::from(tenant_id));
        }
        // Fall back to user's first tenant (single-tenant users or tokens without active_tenant_id)
        let tenants = resources
            .database
            .list_tenants_for_user(auth.user_id)
            .await?;
        Ok(tenants
            .first()
            .map_or_else(|| TenantId::from(auth.user_id), |t| t.id))
    }

    /// Get current LLM settings for the authenticated user
    async fn get_llm_settings(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Json<LlmSettingsResponse>, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let user_id = auth.user_id;
        let tenant_id = Self::get_tenant_id(&auth, &resources).await?;
        let database = &*resources.database;

        // Get user's credentials
        let all_credentials =
            TenantLlmManager::list_tenant_credentials(tenant_id, database).await?;

        let user_credentials: Vec<_> = all_credentials
            .iter()
            .filter(|c| c.user_id == Some(user_id))
            .cloned()
            .collect();

        let tenant_credentials: Vec<_> = all_credentials
            .iter()
            .filter(|c| c.user_id.is_none())
            .cloned()
            .collect();

        // Build provider status list
        let providers = vec![
            Self::build_provider_status(
                "gemini",
                "Google Gemini",
                LlmProvider::Gemini,
                user_id,
                tenant_id,
                database,
            )
            .await,
            Self::build_provider_status(
                "groq",
                "Groq (Llama/Mixtral)",
                LlmProvider::Groq,
                user_id,
                tenant_id,
                database,
            )
            .await,
            Self::build_provider_status(
                "local",
                "Local LLM (Ollama/vLLM)",
                LlmProvider::Local,
                user_id,
                tenant_id,
                database,
            )
            .await,
        ];

        // Determine current effective provider (first one with credentials)
        let current_provider = providers
            .iter()
            .find(|p| p.has_credentials)
            .map(|p| p.name.clone());

        Ok(Json(LlmSettingsResponse {
            current_provider,
            providers,
            user_credentials,
            tenant_credentials,
        }))
    }

    /// Build status for a single provider
    async fn build_provider_status(
        name: &str,
        display_name: &str,
        provider: LlmProvider,
        user_id: Uuid,
        tenant_id: TenantId,
        database: &Database,
    ) -> ProviderStatus {
        let has_credentials =
            TenantLlmManager::has_credentials(Some(user_id), tenant_id, provider, database).await;

        // Determine credential source if available
        let credential_source = if has_credentials {
            match TenantLlmManager::get_credentials(Some(user_id), tenant_id, provider, database)
                .await
            {
                Ok(creds) => Some(creds.source.to_string()),
                Err(_) => None,
            }
        } else {
            None
        };

        ProviderStatus {
            name: name.to_owned(),
            display_name: display_name.to_owned(),
            has_credentials,
            credential_source,
            is_active: false, // Will be set based on current selection
        }
    }

    /// Save LLM credentials
    async fn save_llm_credentials(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Json(request): Json<SaveLlmCredentialsRequest>,
    ) -> Result<Json<SaveCredentialsResponse>, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let user_id = auth.user_id;
        let tenant_id = Self::get_tenant_id(&auth, &resources).await?;
        let database = &*resources.database;

        // Parse provider
        let provider = LlmProvider::parse_str(&request.provider).ok_or_else(|| {
            AppError::invalid_input(format!(
                "Invalid provider '{}'. Use: gemini, groq, or local",
                request.provider
            ))
        })?;

        // Validate API key is not empty
        if request.api_key.trim().is_empty() {
            return Err(AppError::invalid_input("API key cannot be empty"));
        }

        // Determine scope (user-specific or tenant-level)
        // For tenant-level credentials, user must be the tenant owner or admin
        // For now, we allow any authenticated user to set their own credentials
        // and tenant owners to set tenant-level defaults
        let scope_user_id = if request.scope == "tenant" {
            // Allow tenant-level credentials for tenant admins/owners.
            // In single-tenant mode, tenant_id == user_id (always allowed).
            // In multi-tenant mode, check the user's role in the tenant.
            if tenant_id.as_uuid() != user_id {
                let role = resources
                    .database
                    .get_user_tenant_role(user_id, tenant_id)
                    .await
                    .map_err(|e| AppError::database(format!("Failed to check tenant role: {e}")))?;

                let is_tenant_admin = role
                    .as_deref()
                    .is_some_and(|r| r == "owner" || r == "admin");

                if !is_tenant_admin {
                    return Err(AppError::auth_invalid(
                        "Only tenant administrators can set tenant-level credentials",
                    ));
                }
            }
            None
        } else {
            Some(user_id)
        };

        // Store credentials
        let store_request = StoreLlmCredentialsRequest {
            provider,
            api_key: request.api_key,
            base_url: request.base_url,
            default_model: request.default_model,
        };

        let id = TenantLlmManager::store_credentials(
            scope_user_id,
            tenant_id,
            store_request,
            user_id,
            database,
        )
        .await?;

        info!(
            "Stored {} credentials for user {:?} in tenant {}",
            provider, scope_user_id, tenant_id
        );

        Ok(Json(SaveCredentialsResponse {
            success: true,
            id: Some(id.to_string()),
            message: format!(
                "{} API key saved successfully",
                provider.as_str().to_uppercase()
            ),
        }))
    }

    /// Validate LLM credentials without saving
    async fn validate_llm_credentials(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Json(request): Json<ValidateLlmCredentialsRequest>,
    ) -> Result<Json<ValidationResponse>, AppError> {
        // Require authentication to prevent unauthenticated abuse
        let auth = Self::authenticate(&headers, &resources).await?;

        // Parse provider
        let provider = LlmProvider::parse_str(&request.provider).ok_or_else(|| {
            AppError::invalid_input(format!(
                "Invalid provider '{}'. Use: gemini, groq, or local",
                request.provider
            ))
        })?;

        let tenant_id = Self::get_tenant_id(&auth, &resources).await?;

        // Create credentials for validation
        let credentials = LlmCredentials {
            tenant_id,
            user_id: Some(auth.user_id),
            provider,
            api_key: request.api_key,
            base_url: request.base_url,
            default_model: None,
            source: CredentialSource::UserSpecific,
        };

        // Try to create provider and run health check
        match ChatProvider::from_credentials(credentials) {
            Ok(chat_provider) => {
                // Run health check
                match chat_provider.health_check().await {
                    Ok(true) => {
                        let models: Vec<String> = chat_provider
                            .available_models()
                            .iter()
                            .map(|s| (*s).to_owned())
                            .collect();
                        Ok(Json(ValidationResponse {
                            valid: true,
                            provider: Some(request.provider),
                            models: Some(models),
                            error: None,
                        }))
                    }
                    Ok(false) => Ok(Json(ValidationResponse {
                        valid: false,
                        provider: None,
                        models: None,
                        error: Some("Health check failed - API key may be invalid".to_owned()),
                    })),
                    Err(e) => {
                        warn!("Validation failed for {}: {}", request.provider, e);
                        Ok(Json(ValidationResponse {
                            valid: false,
                            provider: None,
                            models: None,
                            error: Some(format!("Validation failed: {e}")),
                        }))
                    }
                }
            }
            Err(e) => {
                warn!("Failed to create provider for validation: {}", e);
                Ok(Json(ValidationResponse {
                    valid: false,
                    provider: None,
                    models: None,
                    error: Some(format!("Invalid configuration: {e}")),
                }))
            }
        }
    }

    /// Delete LLM credentials
    async fn delete_llm_credentials(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(provider_name): Path<String>,
    ) -> Result<impl IntoResponse, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let user_id = auth.user_id;
        let tenant_id = Self::get_tenant_id(&auth, &resources).await?;
        let database = &*resources.database;

        // Parse provider
        let provider = LlmProvider::parse_str(&provider_name).ok_or_else(|| {
            AppError::invalid_input(format!(
                "Invalid provider '{provider_name}'. Use: gemini, groq, or local"
            ))
        })?;

        // Delete user's credentials for this provider
        let deleted =
            TenantLlmManager::delete_credentials(Some(user_id), tenant_id, provider, database)
                .await?;

        if deleted {
            info!(
                "Deleted {} credentials for user {} in tenant {}",
                provider, user_id, tenant_id
            );
            Ok((
                StatusCode::OK,
                Json(serde_json::json!({
                    "success": true,
                    "message": format!("{} API key deleted", provider.as_str().to_uppercase())
                })),
            ))
        } else {
            Ok((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "success": false,
                    "message": format!("No {} credentials found", provider.as_str().to_uppercase())
                })),
            ))
        }
    }
}
