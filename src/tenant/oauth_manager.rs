// ABOUTME: Per-tenant OAuth credential management for isolated multi-tenant operation
// ABOUTME: Handles secure storage, encryption, and retrieval of tenant-specific OAuth applications

use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

/// Credential configuration for storing OAuth credentials
#[derive(Debug, Clone)]
pub struct CredentialConfig {
    /// OAuth client ID (public)
    pub client_id: String,
    /// OAuth client secret (to be encrypted)
    pub client_secret: String,
    /// OAuth redirect URI
    pub redirect_uri: String,
    /// OAuth scopes
    pub scopes: Vec<String>,
    /// User who configured these credentials
    pub configured_by: Uuid,
}

/// Per-tenant OAuth credentials with decrypted secret
#[derive(Debug, Clone)]
pub struct TenantOAuthCredentials {
    /// Tenant ID that owns these credentials
    pub tenant_id: Uuid,
    /// OAuth provider name
    pub provider: String,
    /// OAuth client ID (public)
    pub client_id: String,
    /// OAuth client secret (decrypted)
    pub client_secret: String,
    /// OAuth redirect URI
    pub redirect_uri: String,
    /// OAuth scopes
    pub scopes: Vec<String>,
    /// Daily rate limit for this tenant
    pub rate_limit_per_day: u32,
}

/// Manager for tenant-specific OAuth credentials
///
/// Note: This is a simplified implementation for initial multi-tenant support.
/// In production, credentials would be encrypted and stored in the database.
pub struct TenantOAuthManager {
    // In-memory storage for now - would be database-backed in production
    credentials: HashMap<(Uuid, String), TenantOAuthCredentials>,
    usage_tracking: HashMap<(Uuid, String, chrono::NaiveDate), u32>,
}

impl TenantOAuthManager {
    /// Create new OAuth manager
    #[must_use]
    pub fn new() -> Self {
        Self {
            credentials: HashMap::new(),
            usage_tracking: HashMap::new(),
        }
    }

    /// Load OAuth credentials for a specific tenant and provider
    ///
    /// # Errors
    ///
    /// Returns an error if credential lookup fails
    pub fn get_credentials(
        &self,
        tenant_id: Uuid,
        provider: &str,
    ) -> Result<Option<TenantOAuthCredentials>> {
        Ok(self
            .credentials
            .get(&(tenant_id, provider.to_string()))
            .cloned())
    }

    /// Store OAuth credentials for a tenant
    ///
    /// # Errors
    ///
    /// Returns an error if credential storage fails
    pub fn store_credentials(
        &mut self,
        tenant_id: Uuid,
        provider: &str,
        config: CredentialConfig,
    ) -> Result<()> {
        let credentials = TenantOAuthCredentials {
            tenant_id,
            provider: provider.to_string(),
            client_id: config.client_id,
            client_secret: config.client_secret,
            redirect_uri: config.redirect_uri,
            scopes: config.scopes,
            rate_limit_per_day: 15000, // Default Strava rate limit
        };

        self.credentials
            .insert((tenant_id, provider.to_string()), credentials);
        Ok(())
    }

    /// Check tenant's daily rate limit usage
    ///
    /// # Errors
    ///
    /// Returns an error if rate limit check fails
    pub fn check_rate_limit(&self, tenant_id: Uuid, provider: &str) -> Result<(u32, u32)> {
        let today = Utc::now().date_naive();
        let usage = self
            .usage_tracking
            .get(&(tenant_id, provider.to_string(), today))
            .copied()
            .unwrap_or(0);

        // Get tenant's rate limit
        let daily_limit = self
            .credentials
            .get(&(tenant_id, provider.to_string()))
            .map_or(15000, |c| c.rate_limit_per_day);

        Ok((usage, daily_limit))
    }

    /// Increment tenant's usage counter
    ///
    /// # Errors
    ///
    /// Returns an error if usage increment fails
    pub fn increment_usage(
        &mut self,
        tenant_id: Uuid,
        provider: &str,
        successful_requests: u32,
        _failed_requests: u32,
    ) -> Result<()> {
        let today = Utc::now().date_naive();
        let key = (tenant_id, provider.to_string(), today);
        let current = self.usage_tracking.get(&key).copied().unwrap_or(0);
        self.usage_tracking
            .insert(key, current + successful_requests);
        Ok(())
    }
}

impl Default for TenantOAuthManager {
    fn default() -> Self {
        Self::new()
    }
}
