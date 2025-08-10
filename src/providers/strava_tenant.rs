// ABOUTME: Tenant-aware Strava provider implementation with isolated OAuth credentials
// ABOUTME: Provides Strava API integration respecting tenant boundaries and rate limits

use super::tenant_provider::TenantFitnessProvider;
use crate::models::{Activity, Athlete, PersonalRecord, Stats};
use crate::providers::{strava::StravaProvider, AuthData, FitnessProvider};
use crate::tenant::{TenantContext, TenantOAuthClient};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;

/// Tenant-aware Strava provider
pub struct TenantStravaProvider {
    oauth_client: Arc<TenantOAuthClient>,
    inner_provider: Option<StravaProvider>,
}

impl TenantStravaProvider {
    /// Create new tenant-aware Strava provider
    #[must_use]
    pub const fn new(oauth_client: Arc<TenantOAuthClient>) -> Self {
        Self {
            oauth_client,
            inner_provider: None,
        }
    }

    /// Get the authenticated inner provider, returning an error if not authenticated
    fn get_authenticated_provider(&self) -> Result<&StravaProvider> {
        self.inner_provider
            .as_ref()
            .ok_or_else(|| anyhow!("Provider not authenticated. Call authenticate_tenant first."))
    }
}

#[async_trait]
impl TenantFitnessProvider for TenantStravaProvider {
    async fn authenticate_tenant(
        &mut self,
        tenant_context: &TenantContext,
        provider: &str,
    ) -> Result<()> {
        // Get tenant credentials
        let credentials = self
            .oauth_client
            .get_tenant_credentials(tenant_context.tenant_id, provider)
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "No OAuth credentials found for tenant {} and provider {}",
                    tenant_context.tenant_id,
                    provider
                )
            })?;

        // Create inner Strava provider and authenticate
        let mut strava_provider = StravaProvider::new();
        let auth_data = AuthData::OAuth2 {
            client_id: credentials.client_id,
            client_secret: credentials.client_secret,
            access_token: None, // Will be populated during OAuth flow
            refresh_token: None,
        };

        strava_provider.authenticate(auth_data).await?;
        self.inner_provider = Some(strava_provider);

        Ok(())
    }

    async fn get_athlete(&self) -> Result<Athlete> {
        self.get_authenticated_provider()?.get_athlete().await
    }

    async fn get_activities(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Activity>> {
        self.get_authenticated_provider()?
            .get_activities(limit, offset)
            .await
    }

    async fn get_activity(&self, id: &str) -> Result<Activity> {
        self.get_authenticated_provider()?.get_activity(id).await
    }

    async fn get_stats(&self) -> Result<Stats> {
        self.get_authenticated_provider()?.get_stats().await
    }

    async fn get_personal_records(&self) -> Result<Vec<PersonalRecord>> {
        self.get_authenticated_provider()?
            .get_personal_records()
            .await
    }

    fn provider_name(&self) -> &'static str {
        "strava"
    }
}
