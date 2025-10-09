// ABOUTME: Tenant-aware fitness provider factory for multi-tenant OAuth credential management
// ABOUTME: Routes provider requests through tenant-specific OAuth credentials and rate limiting
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::database_plugins::DatabaseProvider;
use crate::models::{Activity, Athlete, PersonalRecord, Stats};
use crate::tenant::{TenantContext, TenantOAuthClient};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;

/// Tenant-aware fitness provider that wraps existing providers with tenant context
#[async_trait]
pub trait TenantFitnessProvider: Send + Sync {
    /// Authenticate using tenant-specific OAuth credentials
    async fn authenticate_tenant(
        &mut self,
        tenant_context: &TenantContext,
        provider: &str,
        database: &dyn DatabaseProvider,
    ) -> Result<()>;

    /// Get athlete information for the authenticated tenant user
    async fn get_athlete(&self) -> Result<Athlete>;

    /// Get activities for the authenticated tenant user
    async fn get_activities(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Activity>>;

    /// Get specific activity by ID
    async fn get_activity(&self, id: &str) -> Result<Activity>;

    /// Get stats for the authenticated tenant user
    async fn get_stats(&self) -> Result<Stats>;

    /// Get personal records for the authenticated tenant user
    async fn get_personal_records(&self) -> Result<Vec<PersonalRecord>>;

    /// Get provider name
    fn provider_name(&self) -> &'static str;
}

/// Factory for creating tenant-aware fitness providers
pub struct TenantProviderFactory {
    oauth_client: Arc<TenantOAuthClient>,
}

impl TenantProviderFactory {
    /// Create new tenant provider factory
    #[must_use]
    pub const fn new(oauth_client: Arc<TenantOAuthClient>) -> Self {
        Self { oauth_client }
    }

    /// Create tenant-aware provider for the specified type
    ///
    /// # Errors
    ///
    /// Returns an error if the provider type is not supported
    pub fn create_tenant_provider(
        &self,
        provider_type: &str,
    ) -> Result<Box<dyn TenantFitnessProvider>> {
        match provider_type.to_lowercase().as_str() {
            "strava" => Ok(Box::new(super::strava_tenant::TenantStravaProvider::new(
                self.oauth_client.clone(),
            ))),
            _ => Err(anyhow!(
                "Unknown tenant provider: {provider_type}. Currently supported: strava"
            )),
        }
    }
}
