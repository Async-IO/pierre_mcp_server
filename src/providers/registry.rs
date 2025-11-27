// ABOUTME: Provider registry for managing all fitness data providers in a centralized way
// ABOUTME: Handles provider instantiation, configuration, and lookup with proper error handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::core::{FitnessProvider, ProviderConfig, ProviderFactory, TenantProvider};
use super::spi::{ProviderBundle, ProviderCapabilities, ProviderDescriptor};
use crate::constants::oauth_providers;
use crate::errors::{AppError, AppResult};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

// Conditional imports for provider-specific types
#[cfg(feature = "provider-garmin")]
use super::garmin_provider::GarminProviderFactory;
#[cfg(feature = "provider-garmin")]
use super::spi::GarminDescriptor;
#[cfg(feature = "provider-strava")]
use super::spi::StravaDescriptor;
#[cfg(feature = "provider-synthetic")]
use super::spi::SyntheticDescriptor;
#[cfg(feature = "provider-whoop")]
use super::spi::WhoopDescriptor;
#[cfg(feature = "provider-strava")]
use super::strava_provider::StravaProviderFactory;
#[cfg(feature = "provider-synthetic")]
use super::synthetic_provider::SyntheticProviderFactory;
#[cfg(feature = "provider-whoop")]
use super::whoop_provider::WhoopProviderFactory;

/// Factory wrapper for bundle-based provider registration
struct BundleFactory {
    factory_fn: super::spi::ProviderFactoryFn,
}

impl ProviderFactory for BundleFactory {
    fn create(&self, config: ProviderConfig) -> Box<dyn FitnessProvider> {
        (self.factory_fn)(config)
    }

    fn supported_providers(&self) -> &'static [&'static str] {
        &[] // Bundle-based providers don't use this method
    }
}

/// Global provider registry that manages all available fitness providers
pub struct ProviderRegistry {
    factories: HashMap<&'static str, Box<dyn ProviderFactory>>,
    default_configs: HashMap<&'static str, ProviderConfig>,
    descriptors: HashMap<&'static str, Box<dyn ProviderDescriptor>>,
}

impl ProviderRegistry {
    /// Create a new provider registry with default providers
    ///
    /// Providers are configured from environment variables with fallback to hardcoded defaults.
    /// See `crate::config::environment::load_provider_env_config()` for environment variable format.
    ///
    /// Long function: Provider registration requires individual configuration blocks per provider.
    /// This function grows linearly with the number of supported providers, which is expected.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn new() -> Self {
        let mut registry = Self {
            factories: HashMap::new(),
            default_configs: HashMap::new(),
            descriptors: HashMap::new(),
        };

        // Register Strava provider with environment-based configuration
        #[cfg(feature = "provider-strava")]
        {
            registry.register_factory(oauth_providers::STRAVA, Box::new(StravaProviderFactory));
            registry.register_descriptor(oauth_providers::STRAVA, Box::new(StravaDescriptor));
            let (_, _, auth_url, token_url, api_base_url, revoke_url, scopes) =
                crate::config::environment::load_provider_env_config(
                    oauth_providers::STRAVA,
                    "https://www.strava.com/oauth/authorize",
                    "https://www.strava.com/oauth/token",
                    "https://www.strava.com/api/v3",
                    Some("https://www.strava.com/oauth/deauthorize"),
                    &[oauth_providers::STRAVA_DEFAULT_SCOPES.to_owned()],
                );
            registry.set_default_config(
                oauth_providers::STRAVA,
                ProviderConfig {
                    name: oauth_providers::STRAVA.to_owned(),
                    auth_url,
                    token_url,
                    api_base_url,
                    revoke_url,
                    default_scopes: scopes,
                },
            );
        }

        // Register Garmin provider with environment-based configuration
        #[cfg(feature = "provider-garmin")]
        {
            registry.register_factory(oauth_providers::GARMIN, Box::new(GarminProviderFactory));
            registry.register_descriptor(oauth_providers::GARMIN, Box::new(GarminDescriptor));
            let (_, _, auth_url, token_url, api_base_url, revoke_url, scopes) =
                crate::config::environment::load_provider_env_config(
                    oauth_providers::GARMIN,
                    "https://connect.garmin.com/oauthConfirm",
                    "https://connectapi.garmin.com/oauth-service/oauth/access_token",
                    "https://apis.garmin.com/wellness-api/rest",
                    Some("https://connectapi.garmin.com/oauth-service/oauth/revoke"),
                    &crate::constants::oauth::GARMIN_DEFAULT_SCOPES
                        .split(',')
                        .map(str::to_owned)
                        .collect::<Vec<_>>(),
                );
            registry.set_default_config(
                oauth_providers::GARMIN,
                ProviderConfig {
                    name: oauth_providers::GARMIN.to_owned(),
                    auth_url,
                    token_url,
                    api_base_url,
                    revoke_url,
                    default_scopes: scopes,
                },
            );
        }

        // Register WHOOP provider with environment-based configuration
        #[cfg(feature = "provider-whoop")]
        {
            registry.register_factory(oauth_providers::WHOOP, Box::new(WhoopProviderFactory));
            registry.register_descriptor(oauth_providers::WHOOP, Box::new(WhoopDescriptor));
            let (_, _, auth_url, token_url, api_base_url, revoke_url, scopes) =
                crate::config::environment::load_provider_env_config(
                    oauth_providers::WHOOP,
                    "https://api.prod.whoop.com/oauth/oauth2/auth",
                    "https://api.prod.whoop.com/oauth/oauth2/token",
                    "https://api.prod.whoop.com/developer/v1",
                    Some("https://api.prod.whoop.com/oauth/oauth2/revoke"),
                    &oauth_providers::WHOOP_DEFAULT_SCOPES
                        .split(' ')
                        .map(str::to_owned)
                        .collect::<Vec<_>>(),
                );
            registry.set_default_config(
                oauth_providers::WHOOP,
                ProviderConfig {
                    name: oauth_providers::WHOOP.to_owned(),
                    auth_url,
                    token_url,
                    api_base_url,
                    revoke_url,
                    default_scopes: scopes,
                },
            );
        }

        // Register Synthetic provider (for development and testing)
        #[cfg(feature = "provider-synthetic")]
        {
            registry.register_factory(
                oauth_providers::SYNTHETIC,
                Box::new(SyntheticProviderFactory),
            );
            registry.register_descriptor(oauth_providers::SYNTHETIC, Box::new(SyntheticDescriptor));
            registry.set_default_config(
                oauth_providers::SYNTHETIC,
                ProviderConfig {
                    name: oauth_providers::SYNTHETIC.to_owned(),
                    auth_url: "http://localhost/synthetic/auth".to_owned(),
                    token_url: "http://localhost/synthetic/token".to_owned(),
                    api_base_url: "http://localhost/synthetic/api".to_owned(),
                    revoke_url: None,
                    default_scopes: vec!["activity:read_all".to_owned()],
                },
            );
        }

        // Future providers can be added with their own feature flags

        // Log registered providers at startup
        let providers = registry.supported_providers().join(", ");
        tracing::info!(
            "Provider registry initialized with {} provider(s): [{}]",
            registry.factories.len(),
            providers
        );

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

    /// Register a provider descriptor
    pub fn register_descriptor(
        &mut self,
        provider_name: &'static str,
        descriptor: Box<dyn ProviderDescriptor>,
    ) {
        self.descriptors.insert(provider_name, descriptor);
    }

    /// Register a complete provider bundle (factory + descriptor + config)
    ///
    /// This is the preferred method for external provider crates to register their providers.
    /// It handles factory registration, descriptor storage, and default configuration.
    pub fn register_provider_bundle(&mut self, bundle: ProviderBundle) {
        let name = bundle.name();
        // We need to leak the string to get a &'static str
        // This is safe because provider names are expected to live for the program's lifetime
        let static_name: &'static str = Box::leak(name.to_owned().into_boxed_str());

        self.factories.insert(
            static_name,
            Box::new(BundleFactory {
                factory_fn: bundle.factory,
            }),
        );
        self.default_configs
            .insert(static_name, bundle.descriptor.to_config());
        self.descriptors.insert(static_name, bundle.descriptor);

        tracing::info!("Registered external provider: {}", static_name);
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

    /// Check if a provider requires OAuth authentication
    #[must_use]
    pub fn requires_oauth(&self, provider_name: &str) -> bool {
        self.descriptors
            .get(provider_name)
            .is_some_and(|d| d.requires_oauth())
    }

    /// Check if a provider supports sleep tracking
    #[must_use]
    pub fn supports_sleep(&self, provider_name: &str) -> bool {
        self.descriptors
            .get(provider_name)
            .is_some_and(|d| d.supports_sleep())
    }

    /// Check if a provider supports recovery metrics
    #[must_use]
    pub fn supports_recovery(&self, provider_name: &str) -> bool {
        self.descriptors
            .get(provider_name)
            .is_some_and(|d| d.supports_recovery())
    }

    /// Get provider capabilities
    #[must_use]
    pub fn get_capabilities(&self, provider_name: &str) -> Option<ProviderCapabilities> {
        self.descriptors
            .get(provider_name)
            .map(|d| d.capabilities())
    }

    /// Get provider display name
    #[must_use]
    pub fn get_display_name(&self, provider_name: &str) -> Option<&'static str> {
        self.descriptors
            .get(provider_name)
            .map(|d| d.display_name())
    }

    /// Get provider descriptor for OAuth and API configuration
    #[must_use]
    pub fn get_descriptor(&self, provider_name: &str) -> Option<&dyn ProviderDescriptor> {
        self.descriptors
            .get(provider_name)
            .map(std::convert::AsRef::as_ref)
    }

    /// Get all providers that support OAuth
    #[must_use]
    pub fn oauth_providers(&self) -> Vec<&'static str> {
        self.descriptors
            .iter()
            .filter(|(_, d)| d.requires_oauth())
            .map(|(name, _)| *name)
            .collect()
    }

    /// Get all providers that support sleep tracking
    #[must_use]
    pub fn sleep_providers(&self) -> Vec<&'static str> {
        self.descriptors
            .iter()
            .filter(|(_, d)| d.supports_sleep())
            .map(|(name, _)| *name)
            .collect()
    }

    /// Create a provider instance with default configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the provider is not supported or no default configuration exists.
    pub fn create_provider(&self, provider_name: &str) -> AppResult<Box<dyn FitnessProvider>> {
        let factory = self.factories.get(provider_name).ok_or_else(|| {
            AppError::invalid_input(format!("Unsupported provider: {provider_name}"))
        })?;

        let config = self
            .default_configs
            .get(provider_name)
            .ok_or_else(|| {
                AppError::invalid_input(format!(
                    "No default configuration for provider: {provider_name}"
                ))
            })?
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
    ) -> AppResult<Box<dyn FitnessProvider>> {
        let factory = self.factories.get(provider_name).ok_or_else(|| {
            AppError::invalid_input(format!("Unsupported provider: {provider_name}"))
        })?;

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
    ) -> AppResult<TenantProvider> {
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
    ) -> AppResult<TenantProvider> {
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
///
/// Note: For test isolation, prefer creating local `ProviderRegistry::new()` instances
/// instead of using this global singleton. Tests that use the global singleton will
/// share state and cannot customize provider configuration per-test.
static REGISTRY: std::sync::OnceLock<Arc<ProviderRegistry>> = std::sync::OnceLock::new();

/// Get the global provider registry
///
/// This should be used in production code for convenience. For tests requiring isolation,
/// use `ProviderRegistry::new()` directly to create test-specific instances.
#[must_use]
pub fn global_registry() -> Arc<ProviderRegistry> {
    REGISTRY
        .get_or_init(|| Arc::new(ProviderRegistry::new()))
        .clone() // Safe: Arc clone for provider registry access
}

/// Convenience function to create a provider using the global registry
///
/// For test isolation, prefer creating a local `ProviderRegistry` instance and calling
/// `registry.create_provider()` instead of using this global function.
///
/// # Errors
///
/// Returns an error if the provider is not supported or no default configuration exists.
pub fn create_provider(provider_name: &str) -> AppResult<Box<dyn FitnessProvider>> {
    global_registry().create_provider(provider_name)
}

/// Convenience function to create a tenant provider using the global registry
///
/// For test isolation, prefer creating a local `ProviderRegistry` instance and calling
/// `registry.create_tenant_provider()` instead of using this global function.
///
/// # Errors
///
/// Returns an error if the provider is not supported or no default configuration exists.
pub fn create_tenant_provider(
    provider_name: &str,
    tenant_id: Uuid,
    user_id: Uuid,
) -> AppResult<TenantProvider> {
    global_registry().create_tenant_provider(provider_name, tenant_id, user_id)
}

/// Convenience function to check if a provider is supported
///
/// Uses the global registry. For test isolation, create a local `ProviderRegistry` instance.
#[must_use]
pub fn is_provider_supported(provider_name: &str) -> bool {
    global_registry().is_supported(provider_name)
}

/// Convenience function to get all supported providers
///
/// Uses the global registry. For test isolation, create a local `ProviderRegistry` instance.
#[must_use]
pub fn get_supported_providers() -> Vec<&'static str> {
    global_registry().supported_providers()
}

/// Create a new provider registry with external provider bundles
///
/// This function creates a new registry instance with both built-in providers
/// (based on feature flags) and any additional external provider bundles.
///
/// # Example
///
/// ```rust,no_run
/// use pierre_mcp_server::providers::{create_registry_with_external_providers, ProviderBundle};
///
/// // External provider crate would provide a function like:
/// // fn whoop_provider_bundle() -> ProviderBundle { ... }
///
/// let external_bundles = vec![
///     // whoop_provider_bundle(),
/// ];
/// let registry = create_registry_with_external_providers(external_bundles);
/// ```
#[must_use]
pub fn create_registry_with_external_providers(bundles: Vec<ProviderBundle>) -> ProviderRegistry {
    let mut registry = ProviderRegistry::new();
    for bundle in bundles {
        registry.register_provider_bundle(bundle);
    }
    registry
}
