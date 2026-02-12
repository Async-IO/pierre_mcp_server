// ABOUTME: Per-tenant and per-user LLM API key management for isolated multi-tenant operation
// ABOUTME: Handles secure storage, encryption, and retrieval of LLM provider credentials (Gemini, Groq, etc.)
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use pierre_core::models::TenantId;
use std::env;
use std::fmt;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::config::LlmProviderType;
use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::errors::{AppError, AppResult};

/// Environment variable names for LLM provider API keys
const GEMINI_API_KEY_ENV: &str = "GEMINI_API_KEY";
const GROQ_API_KEY_ENV: &str = "GROQ_API_KEY";
const LOCAL_LLM_API_KEY_ENV: &str = "LOCAL_LLM_API_KEY";
const LOCAL_LLM_BASE_URL_ENV: &str = "LOCAL_LLM_BASE_URL";
const LOCAL_LLM_MODEL_ENV: &str = "LOCAL_LLM_MODEL";

/// Supported LLM providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    /// Google Gemini
    Gemini,
    /// Groq (Llama, Mixtral via LPU)
    Groq,
    /// `OpenAI` API
    OpenAi,
    /// Anthropic Claude
    Anthropic,
    /// Local LLM (Ollama, vLLM, `LocalAI`)
    Local,
}

impl LlmProvider {
    /// Get provider name as string
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Gemini => "gemini",
            Self::Groq => "groq",
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
            Self::Local => "local",
        }
    }

    /// Parse provider name from string (case-insensitive)
    #[must_use]
    pub fn parse_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "gemini" | "google" => Some(Self::Gemini),
            "groq" => Some(Self::Groq),
            "openai" | "gpt" => Some(Self::OpenAi),
            "anthropic" | "claude" => Some(Self::Anthropic),
            "local" | "ollama" | "vllm" | "localai" => Some(Self::Local),
            _ => None,
        }
    }

    /// Get environment variable name for this provider's API key
    #[must_use]
    pub const fn env_var_name(&self) -> &'static str {
        match self {
            Self::Gemini => GEMINI_API_KEY_ENV,
            Self::Groq => GROQ_API_KEY_ENV,
            Self::OpenAi => "OPENAI_API_KEY",
            Self::Anthropic => "ANTHROPIC_API_KEY",
            Self::Local => LOCAL_LLM_API_KEY_ENV,
        }
    }

    /// Convert to `LlmProviderType` enum used by the LLM module
    #[must_use]
    pub const fn to_llm_provider_type(&self) -> Option<LlmProviderType> {
        match self {
            Self::Gemini => Some(LlmProviderType::Gemini),
            Self::Groq => Some(LlmProviderType::Groq),
            Self::Local => Some(LlmProviderType::Local),
            // OpenAI and Anthropic not yet supported in LlmProviderType
            Self::OpenAi | Self::Anthropic => None,
        }
    }
}

impl fmt::Display for LlmProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// LLM credentials for a specific provider
#[derive(Debug, Clone)]
pub struct LlmCredentials {
    /// Tenant ID these credentials belong to
    pub tenant_id: TenantId,
    /// User ID (None = tenant-level default)
    pub user_id: Option<Uuid>,
    /// LLM provider
    pub provider: LlmProvider,
    /// API key (decrypted)
    pub api_key: String,
    /// Base URL (for local providers)
    pub base_url: Option<String>,
    /// Default model override
    pub default_model: Option<String>,
    /// Source of credentials (for logging/debugging)
    pub source: CredentialSource,
}

/// Source of LLM credentials (for audit/debugging)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialSource {
    /// User-specific credentials from database
    UserSpecific,
    /// Tenant-level default from database
    TenantDefault,
    /// System-wide override from admin config
    SystemOverride,
    /// Environment variable fallback
    EnvironmentVariable,
}

impl fmt::Display for CredentialSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UserSpecific => write!(f, "user-specific"),
            Self::TenantDefault => write!(f, "tenant-default"),
            Self::SystemOverride => write!(f, "system-override"),
            Self::EnvironmentVariable => write!(f, "environment-variable"),
        }
    }
}

/// Request to store LLM credentials
#[derive(Debug, Clone)]
pub struct StoreLlmCredentialsRequest {
    /// LLM provider
    pub provider: LlmProvider,
    /// API key (plain text, will be encrypted)
    pub api_key: String,
    /// Base URL (for local providers)
    pub base_url: Option<String>,
    /// Default model
    pub default_model: Option<String>,
}

/// Database record for LLM credentials
#[derive(Debug, Clone)]
pub struct LlmCredentialRecord {
    /// Record ID
    pub id: Uuid,
    /// Tenant ID
    pub tenant_id: TenantId,
    /// User ID (None = tenant default)
    pub user_id: Option<Uuid>,
    /// Provider name
    pub provider: String,
    /// Encrypted API key
    pub api_key_encrypted: String,
    /// Base URL (for local providers)
    pub base_url: Option<String>,
    /// Default model
    pub default_model: Option<String>,
    /// Is this credential active
    pub is_active: bool,
    /// Created timestamp
    pub created_at: String,
    /// Updated timestamp
    pub updated_at: String,
    /// Created by user ID
    pub created_by: Uuid,
}

/// Manager for tenant and user LLM credentials
///
/// Resolution order:
/// 1. User-specific credentials (from `user_llm_credentials` table)
/// 2. Tenant-level default (from `user_llm_credentials` table with `user_id = NULL`)
/// 3. System-wide override (from `admin_config_overrides` table)
/// 4. Environment variable fallback
pub struct TenantLlmManager;

impl TenantLlmManager {
    /// Get LLM credentials for a provider with full resolution chain
    ///
    /// Resolution order:
    /// 1. User-specific credentials
    /// 2. Tenant-level default
    /// 3. System-wide override (admin config)
    /// 4. Environment variable fallback
    ///
    /// # Errors
    ///
    /// Returns an error if no credentials are found at any level
    pub async fn get_credentials(
        user_id: Option<Uuid>,
        tenant_id: TenantId,
        provider: LlmProvider,
        database: &Database,
    ) -> AppResult<LlmCredentials> {
        // Priority 1: User-specific credentials
        if let Some(uid) = user_id {
            if let Some(creds) =
                Self::try_user_credentials(uid, tenant_id, provider, database).await
            {
                return Ok(creds);
            }
        }

        // Priority 2: Tenant-level default
        if let Some(creds) = Self::try_tenant_credentials(tenant_id, provider, database).await {
            return Ok(creds);
        }

        // Priority 3: System-wide override (admin config)
        if let Some(creds) = Self::try_system_override(tenant_id, provider, database).await {
            return Ok(creds);
        }

        // Priority 4: Environment variable fallback
        if let Some(creds) = Self::try_environment_credentials(tenant_id, provider) {
            return Ok(creds);
        }

        // No credentials found
        Err(AppError::not_found(format!(
            "No {} API credentials configured. Set {} environment variable, \
             or configure per-tenant credentials in Settings → AI Provider.",
            provider,
            provider.env_var_name()
        )))
    }

    /// Store LLM credentials for a user or tenant
    ///
    /// # Arguments
    /// * `user_id` - User ID (None for tenant-level default)
    /// * `tenant_id` - Tenant ID
    /// * `request` - Credential details
    /// * `created_by` - User who created these credentials
    /// * `database` - Database connection
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails
    pub async fn store_credentials(
        user_id: Option<Uuid>,
        tenant_id: TenantId,
        request: StoreLlmCredentialsRequest,
        created_by: Uuid,
        database: &Database,
    ) -> AppResult<Uuid> {
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4();

        // Create AAD context for encryption
        let aad_context = Self::create_aad_context(tenant_id, user_id, request.provider);

        // Encrypt the API key
        let api_key_encrypted = database.encrypt_data_with_aad(&request.api_key, &aad_context)?;

        let record = LlmCredentialRecord {
            id,
            tenant_id,
            user_id,
            provider: request.provider.as_str().to_owned(),
            api_key_encrypted,
            base_url: request.base_url,
            default_model: request.default_model,
            is_active: true,
            created_at: now.clone(),
            updated_at: now,
            created_by,
        };

        database.store_llm_credentials(&record).await?;

        info!(
            "Stored {} LLM credentials for {} (tenant: {}, user: {:?})",
            request.provider,
            if user_id.is_some() { "user" } else { "tenant" },
            tenant_id,
            user_id
        );

        Ok(id)
    }

    /// Delete LLM credentials
    ///
    /// # Errors
    ///
    /// Returns an error if deletion fails
    pub async fn delete_credentials(
        user_id: Option<Uuid>,
        tenant_id: TenantId,
        provider: LlmProvider,
        database: &Database,
    ) -> AppResult<bool> {
        let deleted = database
            .delete_llm_credentials(tenant_id, user_id, provider.as_str())
            .await?;

        if deleted {
            info!(
                "Deleted {} LLM credentials for {} (tenant: {}, user: {:?})",
                provider,
                if user_id.is_some() { "user" } else { "tenant" },
                tenant_id,
                user_id
            );
        }

        Ok(deleted)
    }

    /// List all LLM credentials for a tenant (for admin UI)
    ///
    /// # Errors
    ///
    /// Returns an error if listing fails
    pub async fn list_tenant_credentials(
        tenant_id: TenantId,
        database: &Database,
    ) -> AppResult<Vec<LlmCredentialSummary>> {
        database.list_llm_credentials(tenant_id).await
    }

    /// Check if credentials exist for a provider (without decrypting)
    pub async fn has_credentials(
        user_id: Option<Uuid>,
        tenant_id: TenantId,
        provider: LlmProvider,
        database: &Database,
    ) -> bool {
        // Check user-specific
        if let Some(uid) = user_id {
            if database
                .get_llm_credentials(tenant_id, Some(uid), provider.as_str())
                .await
                .ok()
                .flatten()
                .is_some()
            {
                return true;
            }
        }

        // Check tenant-level
        if database
            .get_llm_credentials(tenant_id, None, provider.as_str())
            .await
            .ok()
            .flatten()
            .is_some()
        {
            return true;
        }

        // Check environment variable
        env::var(provider.env_var_name()).is_ok()
    }

    // ========================================
    // Private resolution methods
    // ========================================

    /// Decrypt a credential record and build `LlmCredentials`
    fn decrypt_record(
        record: &LlmCredentialRecord,
        tenant_id: TenantId,
        user_id: Option<Uuid>,
        provider: LlmProvider,
        source: CredentialSource,
        database: &Database,
    ) -> Option<LlmCredentials> {
        let aad_context = Self::create_aad_context(tenant_id, user_id, provider);
        match database.decrypt_data_with_aad(&record.api_key_encrypted, &aad_context) {
            Ok(api_key) => Some(LlmCredentials {
                tenant_id,
                user_id,
                provider,
                api_key,
                base_url: record.base_url.clone(),
                default_model: record.default_model.clone(),
                source,
            }),
            Err(e) => {
                warn!("Failed to decrypt {provider} credentials: {e}");
                None
            }
        }
    }

    /// Try to load user-specific credentials
    async fn try_user_credentials(
        user_id: Uuid,
        tenant_id: TenantId,
        provider: LlmProvider,
        database: &Database,
    ) -> Option<LlmCredentials> {
        let result = database
            .get_llm_credentials(tenant_id, Some(user_id), provider.as_str())
            .await;

        match result {
            Ok(Some(record)) => {
                let creds = Self::decrypt_record(
                    &record,
                    tenant_id,
                    Some(user_id),
                    provider,
                    CredentialSource::UserSpecific,
                    database,
                );
                if creds.is_some() {
                    info!(
                        "Using user-specific {provider} credentials for user {user_id} in tenant {tenant_id}"
                    );
                }
                creds
            }
            Ok(None) => {
                debug!(
                    "No user-specific {provider} credentials for user {user_id} in tenant {tenant_id}"
                );
                None
            }
            Err(e) => {
                warn!("Error fetching user {provider} credentials for user {user_id}: {e}");
                None
            }
        }
    }

    /// Try to load tenant-level default credentials
    async fn try_tenant_credentials(
        tenant_id: TenantId,
        provider: LlmProvider,
        database: &Database,
    ) -> Option<LlmCredentials> {
        let result = database
            .get_llm_credentials(tenant_id, None, provider.as_str())
            .await;

        match result {
            Ok(Some(record)) => {
                let creds = Self::decrypt_record(
                    &record,
                    tenant_id,
                    None,
                    provider,
                    CredentialSource::TenantDefault,
                    database,
                );
                if creds.is_some() {
                    info!("Using tenant-level {provider} credentials for tenant {tenant_id}");
                }
                creds
            }
            Ok(None) => {
                debug!("No tenant-level {provider} credentials for tenant {tenant_id}");
                None
            }
            Err(e) => {
                warn!("Error fetching tenant {provider} credentials for tenant {tenant_id}: {e}");
                None
            }
        }
    }

    /// Try to load from system-wide admin config override
    async fn try_system_override(
        tenant_id: TenantId,
        provider: LlmProvider,
        database: &Database,
    ) -> Option<LlmCredentials> {
        // Config key format: llm.{provider}_api_key
        let config_key = format!("llm.{}_api_key", provider.as_str());

        match database.get_admin_config_override(&config_key, None).await {
            Ok(Some(value)) => {
                info!(
                    "Using system-wide {} API key override for tenant {}",
                    provider, tenant_id
                );

                // Also try to get base_url and model for local provider
                let base_url = if provider == LlmProvider::Local {
                    database
                        .get_admin_config_override("llm.local_base_url", None)
                        .await
                        .ok()
                        .flatten()
                } else {
                    None
                };

                let default_model = if provider == LlmProvider::Local {
                    database
                        .get_admin_config_override("llm.local_model", None)
                        .await
                        .ok()
                        .flatten()
                } else {
                    None
                };

                Some(LlmCredentials {
                    tenant_id,
                    user_id: None,
                    provider,
                    api_key: value,
                    base_url,
                    default_model,
                    source: CredentialSource::SystemOverride,
                })
            }
            Ok(None) => None,
            Err(e) => {
                debug!("Error checking system override for {}: {}", provider, e);
                None
            }
        }
    }

    /// Try to load from environment variables (final fallback)
    fn try_environment_credentials(
        tenant_id: TenantId,
        provider: LlmProvider,
    ) -> Option<LlmCredentials> {
        let api_key = env::var(provider.env_var_name()).ok()?;

        info!(
            "Using environment variable {} for tenant {}",
            provider.env_var_name(),
            tenant_id
        );

        // For local provider, also check base URL and model env vars
        let (base_url, default_model) = if provider == LlmProvider::Local {
            (
                env::var(LOCAL_LLM_BASE_URL_ENV).ok(),
                env::var(LOCAL_LLM_MODEL_ENV).ok(),
            )
        } else {
            (None, None)
        };

        Some(LlmCredentials {
            tenant_id,
            user_id: None,
            provider,
            api_key,
            base_url,
            default_model,
            source: CredentialSource::EnvironmentVariable,
        })
    }

    /// Create AAD context for encryption/decryption
    ///
    /// Format: `"{tenant_id}|{user_id}|{provider}|user_llm_credentials"`
    fn create_aad_context(
        tenant_id: TenantId,
        user_id: Option<Uuid>,
        provider: LlmProvider,
    ) -> String {
        let user_part = user_id.map_or_else(|| "tenant-default".to_owned(), |u| u.to_string());
        format!(
            "{}|{}|{}|user_llm_credentials",
            tenant_id,
            user_part,
            provider.as_str()
        )
    }
}

/// Summary of LLM credentials (for listing, without decrypted key)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCredentialSummary {
    /// Record ID
    pub id: Uuid,
    /// User ID (None = tenant default)
    pub user_id: Option<Uuid>,
    /// Provider name
    pub provider: String,
    /// Whether this is a user-specific or tenant-level credential
    pub scope: String,
    /// Base URL (for local providers)
    pub base_url: Option<String>,
    /// Default model
    pub default_model: Option<String>,
    /// Is active
    pub is_active: bool,
    /// Created timestamp
    pub created_at: String,
    /// Updated timestamp
    pub updated_at: String,
}
