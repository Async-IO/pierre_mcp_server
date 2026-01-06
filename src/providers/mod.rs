// ABOUTME: Unified fitness data provider system with clean abstractions and multi-tenant support
// ABOUTME: Replaces the previous fragmented provider implementations with a single, extensible architecture

//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Fitness Data Provider System
//!
//! This module provides a unified, extensible system for integrating with fitness data providers
//! like Strava, Fitbit, and others. The architecture is designed around clean abstractions that
//! support multi-tenancy, OAuth2 authentication, and consistent error handling.
//!
//! ## Architecture
//!
//! - `core` - Core traits and interfaces that all providers implement
//! - `registry` - Global registry for managing provider factories and configurations
//! - `strava_provider` - Clean Strava API implementation
//!
//! ## Usage
//!
//! ```rust,no_run
//! use pierre_mcp_server::providers::{create_provider, create_tenant_provider};
//! use pierre_mcp_server::constants::oauth_providers;
//! # use uuid::Uuid;
//! # let tenant_id = Uuid::new_v4();
//! # let user_id = Uuid::new_v4();
//!
//! // Create a basic provider
//! let provider = create_provider(oauth_providers::STRAVA)?;
//!
//! // Or create a tenant-aware provider
//! let tenant_provider = create_tenant_provider(
//!     oauth_providers::STRAVA,
//!     tenant_id,
//!     user_id
//! )?;
//! # Ok::<(), anyhow::Error>(())
//! ```

// Core provider system

/// Streaming activity iterator for memory-efficient paginated fetching
pub mod activity_iterator;
/// Circuit breaker pattern for provider resilience
pub mod circuit_breaker;
/// Core provider traits and interfaces
pub mod core;
/// Provider error types and result aliases
pub mod errors;
/// Global provider registry and factory
pub mod registry;
/// Service Provider Interface (SPI) for external providers
pub mod spi;
/// Provider utility functions
pub mod utils;

// Provider implementations (conditionally compiled based on feature flags)

/// COROS provider for GPS sports watch data (activities, sleep, daily summaries)
#[cfg(feature = "provider-coros")]
pub mod coros_provider;
/// Fitbit API provider implementation
#[cfg(feature = "provider-fitbit")]
pub mod fitbit_provider;
/// Garmin Connect provider implementation
#[cfg(feature = "provider-garmin")]
pub mod garmin_provider;
/// Strava API provider implementation
#[cfg(feature = "provider-strava")]
pub mod strava_provider;
/// Synthetic provider for development and testing
#[cfg(feature = "provider-synthetic")]
pub mod synthetic_provider;
/// Terra unified API provider (150+ wearables)
#[cfg(feature = "provider-terra")]
pub mod terra;
/// WHOOP provider for sleep, recovery, and workout data
#[cfg(feature = "provider-whoop")]
pub mod whoop_provider;

// Re-export key types for convenience
/// Re-export activity iterator for memory-efficient streaming
pub use activity_iterator::{
    create_activity_stream, ActivityStream, ActivityStreamExt, StreamConfig, DEFAULT_PAGE_SIZE,
    MAX_PAGE_SIZE, MIN_PAGE_SIZE,
};
/// Re-export circuit breaker types for provider resilience
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
pub use core::{
    ActivityQueryParams, FitnessProvider as CoreFitnessProvider, OAuth2Credentials, ProviderConfig,
    ProviderFactory, TenantProvider,
};
/// Re-export provider error types
pub use errors::{ProviderError, ProviderResult};
#[cfg(feature = "provider-terra")]
pub use registry::global_terra_cache;
/// Re-export provider registry functions
pub use registry::{
    create_provider, create_registry_with_external_providers, create_tenant_provider,
    get_supported_providers, global_registry, is_provider_supported, ProviderRegistry,
};
#[cfg(feature = "provider-coros")]
pub use spi::CorosDescriptor;
#[cfg(feature = "provider-fitbit")]
pub use spi::FitbitDescriptor;
#[cfg(feature = "provider-garmin")]
pub use spi::GarminDescriptor;
#[cfg(feature = "provider-strava")]
pub use spi::StravaDescriptor;
#[cfg(feature = "provider-synthetic")]
pub use spi::SyntheticDescriptor;
#[cfg(feature = "provider-whoop")]
pub use spi::WhoopDescriptor;
/// Re-export SPI types for external provider development
pub use spi::{OAuthEndpoints, ProviderBundle, ProviderCapabilities, ProviderDescriptor};
/// Re-export Terra types
#[cfg(feature = "provider-terra")]
pub use terra::{
    TerraDataCache, TerraDescriptor, TerraProvider, TerraProviderFactory, TerraWebhookHandler,
};
/// Re-export retry utilities for production resilience
pub use utils::{
    with_retry, with_retry_default, RetryBackoffConfig, ENV_RETRY_BASE_DELAY_MS,
    ENV_RETRY_JITTER_FACTOR, ENV_RETRY_MAX_ATTEMPTS, ENV_RETRY_MAX_DELAY_MS,
};
