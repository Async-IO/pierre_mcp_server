// ABOUTME: Provider registry for managing all fitness data providers in a centralized way
// ABOUTME: Handles provider instantiation, configuration, and lookup with proper error handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::core::{FitnessProvider, ProviderConfig, ProviderFactory, TenantProvider};
use super::strava_provider::StravaProvider;
use crate::constants::oauth_providers;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Factory for creating Strava providers
pub struct StravaProviderFactory;

impl ProviderFactory for StravaProviderFactory {
    fn create(&self, config: ProviderConfig) -> Box<dyn FitnessProvider> {
        Box::new(StravaProvider::with_config(config))
    }

    fn supported_providers(&self) -> &'static [&'static str] {
        &[oauth_providers::STRAVA]
    }
}

/// Global provider registry that manages all available fitness providers
pub struct ProviderRegistry {
    factories: HashMap<&'static str, Box<dyn ProviderFactory>>,
    default_configs: HashMap<&'static str, ProviderConfig>,
}

impl ProviderRegistry {
    /// Create a new provider registry with default providers
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self {
            factories: HashMap::new(),
            default_configs: HashMap::new(),
        };

        // Register Strava provider
        registry.register_factory(oauth_providers::STRAVA, Box::new(StravaProviderFactory));
        registry.set_default_config(
            oauth_providers::STRAVA,
            ProviderConfig {
                name: oauth_providers::STRAVA.to_string(),
                auth_url: "https://www.strava.com/oauth/authorize".to_string(),
                token_url: "https://www.strava.com/oauth/token".to_string(),
                api_base_url: "https://www.strava.com/api/v3".to_string(),
                revoke_url: Some("https://www.strava.com/oauth/deauthorize".to_string()),
                default_scopes: crate::constants::oauth::STRAVA_DEFAULT_SCOPES
                    .split(',')
                    .map(str::to_string)
                    .collect(),
            },
        );

        // Future: Add Fitbit provider when implemented
        // registry.register_factory(oauth_providers::FITBIT, Box::new(FitbitProviderFactory));

        registry
    }

    /// Register a provider factory
    pub fn register_factory(
        &mut self,
        provider_name: &'static str,
        factory: Box<dyn ProviderFactory>,
    ) {
        self.factories.insert(provider_name, factory);
    }

    /// Set default configuration for a provider
    pub fn set_default_config(&mut self, provider_name: &'static str, config: ProviderConfig) {
        self.default_configs.insert(provider_name, config);
    }

    /// Get list of supported provider names
    #[must_use]
    pub fn supported_providers(&self) -> Vec<&'static str> {
        self.factories.keys().copied().collect()
    }

    /// Check if a provider is supported
    #[must_use]
    pub fn is_supported(&self, provider_name: &str) -> bool {
        self.factories.contains_key(provider_name)
    }

    /// Create a provider instance with default configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the provider is not supported or no default configuration exists.
    pub fn create_provider(&self, provider_name: &str) -> Result<Box<dyn FitnessProvider>> {
        let factory = self
            .factories
            .get(provider_name)
            .with_context(|| format!("Unsupported provider: {provider_name}"))?;

        let config = self
            .default_configs
            .get(provider_name)
            .with_context(|| format!("No default configuration for provider: {provider_name}"))?
            .clone();

        Ok(factory.create(config))
    }

    /// Create a provider instance with custom configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the provider is not supported.
    pub fn create_provider_with_config(
        &self,
        provider_name: &str,
        config: ProviderConfig,
    ) -> Result<Box<dyn FitnessProvider>> {
        let factory = self
            .factories
            .get(provider_name)
            .with_context(|| format!("Unsupported provider: {provider_name}"))?;

        Ok(factory.create(config))
    }

    /// Create a tenant-aware provider
    ///
    /// # Errors
    ///
    /// Returns an error if the provider is not supported or no default configuration exists.
    pub fn create_tenant_provider(
        &self,
        provider_name: &str,
        tenant_id: Uuid,
        user_id: Uuid,
    ) -> Result<TenantProvider> {
        let provider = self.create_provider(provider_name)?;
        Ok(TenantProvider::new(provider, tenant_id, user_id))
    }

    /// Create a tenant-aware provider with custom configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the provider is not supported.
    pub fn create_tenant_provider_with_config(
        &self,
        provider_name: &str,
        config: ProviderConfig,
        tenant_id: Uuid,
        user_id: Uuid,
    ) -> Result<TenantProvider> {
        let provider = self.create_provider_with_config(provider_name, config)?;
        Ok(TenantProvider::new(provider, tenant_id, user_id))
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global provider registry instance (singleton)
static REGISTRY: std::sync::OnceLock<Arc<ProviderRegistry>> = std::sync::OnceLock::new();

/// Get the global provider registry
#[must_use]
pub fn global_registry() -> Arc<ProviderRegistry> {
    REGISTRY
        .get_or_init(|| Arc::new(ProviderRegistry::new()))
        .clone() // Safe: Arc clone for provider registry access
}

/// Convenience function to create a provider
///
/// # Errors
///
/// Returns an error if the provider is not supported or no default configuration exists.
pub fn create_provider(provider_name: &str) -> Result<Box<dyn FitnessProvider>> {
    global_registry().create_provider(provider_name)
}

/// Convenience function to create a tenant provider
///
/// # Errors
///
/// Returns an error if the provider is not supported or no default configuration exists.
pub fn create_tenant_provider(
    provider_name: &str,
    tenant_id: Uuid,
    user_id: Uuid,
) -> Result<TenantProvider> {
    global_registry().create_tenant_provider(provider_name, tenant_id, user_id)
}

/// Convenience function to check if a provider is supported
#[must_use]
pub fn is_provider_supported(provider_name: &str) -> bool {
    global_registry().is_supported(provider_name)
}

/// Convenience function to get all supported providers
#[must_use]
pub fn get_supported_providers() -> Vec<&'static str> {
    global_registry().supported_providers()
}
