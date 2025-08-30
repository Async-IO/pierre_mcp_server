// ABOUTME: Core provider traits and interfaces for unified fitness data access
// ABOUTME: Defines the foundational abstractions for all fitness data providers

use crate::models::{Activity, Athlete, PersonalRecord, Stats};
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Authentication credentials for OAuth2 providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Credentials {
    pub client_id: String,
    pub client_secret: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub scopes: Vec<String>,
}

/// Provider configuration containing all necessary endpoints and settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub auth_url: String,
    pub token_url: String,
    pub api_base_url: String,
    pub revoke_url: Option<String>,
    pub default_scopes: Vec<String>,
}

/// Core fitness data provider trait - single interface for all providers
#[async_trait]
pub trait FitnessProvider: Send + Sync {
    /// Get provider name (e.g., "strava", "fitbit")
    fn name(&self) -> &'static str;
    
    /// Get provider configuration
    fn config(&self) -> &ProviderConfig;
    
    /// Set OAuth2 credentials for this provider
    async fn set_credentials(&mut self, credentials: OAuth2Credentials) -> Result<()>;
    
    /// Check if provider has valid authentication
    async fn is_authenticated(&self) -> bool;
    
    /// Refresh access token if needed
    async fn refresh_token_if_needed(&mut self) -> Result<()>;
    
    /// Get user's athlete profile
    async fn get_athlete(&self) -> Result<Athlete>;
    
    /// Get user's activities with pagination
    async fn get_activities(&self, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<Activity>>;
    
    /// Get specific activity by ID
    async fn get_activity(&self, id: &str) -> Result<Activity>;
    
    /// Get user's aggregate statistics
    async fn get_stats(&self) -> Result<Stats>;
    
    /// Get user's personal records
    async fn get_personal_records(&self) -> Result<Vec<PersonalRecord>>;
    
    /// Revoke access tokens (disconnect)
    async fn disconnect(&mut self) -> Result<()>;
}

/// Provider factory for creating instances
pub trait ProviderFactory: Send + Sync {
    /// Create a new provider instance with the given configuration
    fn create(&self, config: ProviderConfig) -> Box<dyn FitnessProvider>;
    
    /// Get supported provider names
    fn supported_providers(&self) -> &'static [&'static str];
}

/// Tenant-aware provider wrapper that handles multi-tenancy
pub struct TenantProvider {
    inner: Box<dyn FitnessProvider>,
    tenant_id: Uuid,
    user_id: Uuid,
}

impl TenantProvider {
    /// Create a new tenant-aware provider
    pub fn new(inner: Box<dyn FitnessProvider>, tenant_id: Uuid, user_id: Uuid) -> Self {
        Self {
            inner,
            tenant_id,
            user_id,
        }
    }
    
    /// Get tenant ID
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }
    
    /// Get user ID
    pub fn user_id(&self) -> Uuid {
        self.user_id
    }
}

#[async_trait]
impl FitnessProvider for TenantProvider {
    fn name(&self) -> &'static str {
        self.inner.name()
    }
    
    fn config(&self) -> &ProviderConfig {
        self.inner.config()
    }
    
    async fn set_credentials(&mut self, credentials: OAuth2Credentials) -> Result<()> {
        // Add tenant-specific logging/metrics here
        tracing::info!(
            "Setting credentials for provider {} in tenant {} for user {}",
            self.name(),
            self.tenant_id,
            self.user_id
        );
        self.inner.set_credentials(credentials).await
    }
    
    async fn is_authenticated(&self) -> bool {
        self.inner.is_authenticated().await
    }
    
    async fn refresh_token_if_needed(&mut self) -> Result<()> {
        self.inner.refresh_token_if_needed().await
    }
    
    async fn get_athlete(&self) -> Result<Athlete> {
        self.inner.get_athlete().await
    }
    
    async fn get_activities(&self, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<Activity>> {
        self.inner.get_activities(limit, offset).await
    }
    
    async fn get_activity(&self, id: &str) -> Result<Activity> {
        self.inner.get_activity(id).await
    }
    
    async fn get_stats(&self) -> Result<Stats> {
        self.inner.get_stats().await
    }
    
    async fn get_personal_records(&self) -> Result<Vec<PersonalRecord>> {
        self.inner.get_personal_records().await
    }
    
    async fn disconnect(&mut self) -> Result<()> {
        self.inner.disconnect().await
    }
}