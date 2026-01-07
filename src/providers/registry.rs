// ABOUTME: Provider registry for managing all fitness data providers in a centralized way
// ABOUTME: Handles provider instantiation, configuration, and lookup with proper error handling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::caching_provider::CachingFitnessProvider;
use super::core::{FitnessProvider, ProviderConfig, ProviderFactory, TenantProvider};
use super::spi::{ProviderBundle, ProviderCapabilities, ProviderDescriptor};
use crate::cache::memory::InMemoryCache;
use crate::cache::{CacheConfig, CacheTtlConfig};
use crate::config::admin::service::AdminConfigService;
use crate::config::environment::load_provider_env_config;
use crate::constants::oauth::GARMIN_DEFAULT_SCOPES;
use crate::constants::oauth_providers;
use crate::errors::{AppError, AppResult};
use std::{
    collections::HashMap,
    convert::AsRef,
    sync::{Arc, OnceLock},
};
use tracing::info;
use uuid::Uuid;

// Conditional imports for provider-specific types
#[cfg(feature = "provider-coros")]
use super::coros_provider::CorosProviderFactory;
#[cfg(feature = "provider-fitbit")]
use super::fitbit_provider::FitbitProviderFactory;
#[cfg(feature = "provider-garmin")]
use super::garmin_provider::GarminProviderFactory;
#[cfg(feature = "provider-coros")]
use super::spi::CorosDescriptor;
#[cfg(feature = "provider-fitbit")]
use super::spi::FitbitDescriptor;
#[cfg(feature = "provider-garmin")]
use super::spi::GarminDescriptor;
#[cfg(feature = "provider-strava")]
use super::spi::StravaDescriptor;
#[cfg(feature = "provider-synthetic")]
use super::spi::SyntheticDescriptor;
#[cfg(feature = "provider-synthetic")]
use super::spi::SyntheticSleepDescriptor;
#[cfg(feature = "provider-whoop")]
use super::spi::WhoopDescriptor;
#[cfg(feature = "provider-strava")]
use super::strava_provider::StravaProviderFactory;
#[cfg(feature = "provider-synthetic")]
use super::synthetic_provider::{SyntheticProviderFactory, SyntheticSleepProviderFactory};
#[cfg(feature = "provider-terra")]
use super::terra::{TerraDataCache, TerraDescriptor, TerraProviderFactory};
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
    /// See `load_provider_env_config()` for environment variable format.
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self {
            factories: HashMap::new(),
            default_configs: HashMap::new(),
            descriptors: HashMap::new(),
        };

        // Register all enabled providers
        Self::register_strava(&mut registry);
        Self::register_garmin(&mut registry);
        Self::register_fitbit(&mut registry);
        Self::register_terra(&mut registry);
        Self::register_whoop(&mut registry);
        Self::register_coros(&mut registry);
        Self::register_synthetic(&mut registry);

        // Log registered providers at startup
        let providers = registry.supported_providers().join(", ");
        info!(
            "Provider registry initialized with {} provider(s): [{}]",
            registry.factories.len(),
            providers
        );

        registry
    }

    /// Register Strava provider with environment-based configuration
    #[cfg(feature = "provider-strava")]
    fn register_strava(registry: &mut Self) {
        registry.register_factory(oauth_providers::STRAVA, Box::new(StravaProviderFactory));
        registry.register_descriptor(oauth_providers::STRAVA, Box::new(StravaDescriptor));
        let (_, _, auth_url, token_url, api_base_url, revoke_url, scopes) =
            load_provider_env_config(
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

    #[cfg(not(feature = "provider-strava"))]
    fn register_strava(_registry: &mut Self) {}

    /// Register Garmin provider with environment-based configuration
    #[cfg(feature = "provider-garmin")]
    fn register_garmin(registry: &mut Self) {
        registry.register_factory(oauth_providers::GARMIN, Box::new(GarminProviderFactory));
        registry.register_descriptor(oauth_providers::GARMIN, Box::new(GarminDescriptor));
        let (_, _, auth_url, token_url, api_base_url, revoke_url, scopes) =
            load_provider_env_config(
                oauth_providers::GARMIN,
                "https://connect.garmin.com/oauthConfirm",
                "https://connectapi.garmin.com/oauth-service/oauth/access_token",
                "https://apis.garmin.com/wellness-api/rest",
                Some("https://connectapi.garmin.com/oauth-service/oauth/revoke"),
                &GARMIN_DEFAULT_SCOPES
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

    #[cfg(not(feature = "provider-garmin"))]
    fn register_garmin(_registry: &mut Self) {}

    /// Register Fitbit provider with environment-based configuration
    #[cfg(feature = "provider-fitbit")]
    fn register_fitbit(registry: &mut Self) {
        registry.register_factory(oauth_providers::FITBIT, Box::new(FitbitProviderFactory));
        registry.register_descriptor(oauth_providers::FITBIT, Box::new(FitbitDescriptor));
        let (_, _, auth_url, token_url, api_base_url, revoke_url, scopes) =
            load_provider_env_config(
                oauth_providers::FITBIT,
                "https://www.fitbit.com/oauth2/authorize",
                "https://api.fitbit.com/oauth2/token",
                "https://api.fitbit.com/1",
                Some("https://api.fitbit.com/oauth2/revoke"),
                &oauth_providers::FITBIT_DEFAULT_SCOPES
                    .split(' ')
                    .map(str::to_owned)
                    .collect::<Vec<_>>(),
            );
        registry.set_default_config(
            oauth_providers::FITBIT,
            ProviderConfig {
                name: oauth_providers::FITBIT.to_owned(),
                auth_url,
                token_url,
                api_base_url,
                revoke_url,
                default_scopes: scopes,
            },
        );
    }

    #[cfg(not(feature = "provider-fitbit"))]
    fn register_fitbit(_registry: &mut Self) {}

    /// Register Terra provider with environment-based configuration
    #[cfg(feature = "provider-terra")]
    fn register_terra(registry: &mut Self) {
        let terra_cache = global_terra_cache();
        registry.register_factory(
            oauth_providers::TERRA,
            Box::new(TerraProviderFactory::new(terra_cache)),
        );
        registry.register_descriptor(oauth_providers::TERRA, Box::new(TerraDescriptor));
        registry.set_default_config(
            oauth_providers::TERRA,
            ProviderConfig {
                name: oauth_providers::TERRA.to_owned(),
                auth_url: "https://api.tryterra.co/v2/auth/generateWidgetSession".to_owned(),
                token_url: "https://api.tryterra.co/v2/auth/token".to_owned(),
                api_base_url: "https://api.tryterra.co/v2".to_owned(),
                revoke_url: Some("https://api.tryterra.co/v2/auth/deauthenticateUser".to_owned()),
                default_scopes: oauth_providers::TERRA_DEFAULT_SCOPES
                    .split(',')
                    .map(str::to_owned)
                    .collect(),
            },
        );
    }

    #[cfg(not(feature = "provider-terra"))]
    fn register_terra(_registry: &mut Self) {}

    /// Register WHOOP provider with environment-based configuration
    #[cfg(feature = "provider-whoop")]
    fn register_whoop(registry: &mut Self) {
        registry.register_factory(oauth_providers::WHOOP, Box::new(WhoopProviderFactory));
        registry.register_descriptor(oauth_providers::WHOOP, Box::new(WhoopDescriptor));
        let (_, _, auth_url, token_url, api_base_url, revoke_url, scopes) =
            load_provider_env_config(
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

    #[cfg(not(feature = "provider-whoop"))]
    fn register_whoop(_registry: &mut Self) {}

    /// Register COROS provider with environment-based configuration
    ///
    /// Note: COROS API documentation is private. OAuth endpoints are placeholders
    /// until official documentation is received.
    #[cfg(feature = "provider-coros")]
    fn register_coros(registry: &mut Self) {
        registry.register_factory(oauth_providers::COROS, Box::new(CorosProviderFactory));
        registry.register_descriptor(oauth_providers::COROS, Box::new(CorosDescriptor));
        let (_, _, auth_url, token_url, api_base_url, revoke_url, scopes) =
            load_provider_env_config(
                oauth_providers::COROS,
                // Placeholder URLs - update when COROS provides official API documentation
                "https://open.coros.com/oauth2/authorize",
                "https://open.coros.com/oauth2/token",
                "https://open.coros.com/api/v1",
                Some("https://open.coros.com/oauth2/revoke"),
                &oauth_providers::COROS_DEFAULT_SCOPES
                    .split(' ')
                    .map(str::to_owned)
                    .collect::<Vec<_>>(),
            );
        registry.set_default_config(
            oauth_providers::COROS,
            ProviderConfig {
                name: oauth_providers::COROS.to_owned(),
                auth_url,
                token_url,
                api_base_url,
                revoke_url,
                default_scopes: scopes,
            },
        );
    }

    #[cfg(not(feature = "provider-coros"))]
    fn register_coros(_registry: &mut Self) {}

    /// Register Synthetic provider for development and testing
    #[cfg(feature = "provider-synthetic")]
    fn register_synthetic(registry: &mut Self) {
        // Register the primary synthetic provider (for activities)
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

        // Register the synthetic_sleep provider for cross-provider testing
        registry.register_factory(
            oauth_providers::SYNTHETIC_SLEEP,
            Box::new(SyntheticSleepProviderFactory),
        );
        registry.register_descriptor(
            oauth_providers::SYNTHETIC_SLEEP,
            Box::new(SyntheticSleepDescriptor),
        );
        registry.set_default_config(
            oauth_providers::SYNTHETIC_SLEEP,
            ProviderConfig {
                name: oauth_providers::SYNTHETIC_SLEEP.to_owned(),
                auth_url: "http://localhost/synthetic_sleep/auth".to_owned(),
                token_url: "http://localhost/synthetic_sleep/token".to_owned(),
                api_base_url: "http://localhost/synthetic_sleep/api".to_owned(),
                revoke_url: None,
                default_scopes: vec!["sleep:read".to_owned()],
            },
        );
    }

    #[cfg(not(feature = "provider-synthetic"))]
    fn register_synthetic(_registry: &mut Self) {}

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

        info!("Registered external provider: {}", static_name);
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
        self.descriptors.get(provider_name).map(AsRef::as_ref)
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

    /// Create a caching provider with default configuration
    ///
    /// This wraps a provider with transparent caching using the cache-aside pattern.
    /// The cache backend is determined by the `CacheConfig` (Redis if URL provided,
    /// otherwise in-memory).
    ///
    /// # Errors
    ///
    /// Returns an error if the provider is not supported, cache initialization fails,
    /// or no default configuration exists.
    pub async fn create_caching_provider(
        &self,
        provider_name: &str,
        cache_config: CacheConfig,
        tenant_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<CachingFitnessProvider<InMemoryCache>> {
        let provider = self.create_provider(provider_name)?;
        super::caching_provider::create_caching_provider(provider, cache_config, tenant_id, user_id)
            .await
    }

    /// Create a caching provider with TTL configuration from admin config service
    ///
    /// This wraps a provider with transparent caching, loading TTL values from
    /// the admin configuration service. This allows runtime configuration of
    /// cache TTLs through the admin UI.
    ///
    /// # Arguments
    ///
    /// * `provider_name` - Name of the provider to wrap (e.g., "strava", "garmin")
    /// * `cache_config` - Base cache configuration (capacity, cleanup interval, etc.)
    /// * `tenant_id` - Tenant ID for multi-tenant cache isolation
    /// * `user_id` - User ID for per-user cache isolation
    /// * `admin_config` - Admin configuration service for loading TTL values
    ///
    /// # Errors
    ///
    /// Returns an error if the provider is not supported or cache initialization fails.
    pub async fn create_caching_provider_with_admin_config(
        &self,
        provider_name: &str,
        cache_config: CacheConfig,
        tenant_id: Uuid,
        user_id: Uuid,
        admin_config: &AdminConfigService,
    ) -> AppResult<CachingFitnessProvider<InMemoryCache>> {
        let provider = self.create_provider(provider_name)?;
        let ttl_config =
            CacheTtlConfig::from_admin_config(admin_config, Some(&tenant_id.to_string())).await;
        super::caching_provider::create_caching_provider_with_ttl(
            provider,
            cache_config,
            tenant_id,
            user_id,
            ttl_config,
        )
        .await
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
static REGISTRY: OnceLock<Arc<ProviderRegistry>> = OnceLock::new();

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

/// Convenience function to create a caching provider using the global registry
///
/// This wraps a provider with transparent caching using the cache-aside pattern.
/// For test isolation, prefer creating a local `ProviderRegistry` instance.
///
/// # Errors
///
/// Returns an error if the provider is not supported or cache initialization fails.
pub async fn create_caching_provider_global(
    provider_name: &str,
    cache_config: CacheConfig,
    tenant_id: Uuid,
    user_id: Uuid,
) -> AppResult<CachingFitnessProvider<InMemoryCache>> {
    global_registry()
        .create_caching_provider(provider_name, cache_config, tenant_id, user_id)
        .await
}

/// Convenience function to create a caching provider with admin config TTLs
///
/// Uses the global registry and loads TTL configuration from the admin config service.
/// For test isolation, prefer creating a local `ProviderRegistry` instance.
///
/// # Errors
///
/// Returns an error if the provider is not supported or cache initialization fails.
pub async fn create_caching_provider_with_admin_config_global(
    provider_name: &str,
    cache_config: CacheConfig,
    tenant_id: Uuid,
    user_id: Uuid,
    admin_config: &AdminConfigService,
) -> AppResult<CachingFitnessProvider<InMemoryCache>> {
    global_registry()
        .create_caching_provider_with_admin_config(
            provider_name,
            cache_config,
            tenant_id,
            user_id,
            admin_config,
        )
        .await
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

// ============================================================================
// Terra Global Cache (conditionally compiled)
// ============================================================================

/// Global Terra data cache instance
///
/// Terra uses a webhook-based model where data is pushed to your endpoint.
/// This global cache stores webhook data and makes it available to `TerraProvider`
/// instances for the `FitnessProvider` trait implementation.
#[cfg(feature = "provider-terra")]
static TERRA_CACHE: OnceLock<Arc<TerraDataCache>> = OnceLock::new();

/// Get the global Terra data cache
///
/// Returns a shared reference to the Terra webhook data cache.
/// Use this cache with `TerraWebhookHandler` to store incoming webhook data.
#[cfg(feature = "provider-terra")]
#[must_use]
pub fn global_terra_cache() -> Arc<TerraDataCache> {
    TERRA_CACHE
        .get_or_init(|| Arc::new(TerraDataCache::new_in_memory()))
        .clone() // Safe: Arc clone for shared cache access
}
