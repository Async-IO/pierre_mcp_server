// ABOUTME: Provider module re-exports from pierre-providers crate plus local modules
// ABOUTME: Preserves all existing import paths while delegating core to the extracted crate
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
//! use pierre_mcp_server::models::TenantId;
//! # use uuid::Uuid;
//! # let tenant_id = TenantId::new();
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
//! # Ok::<(), pierre_mcp_server::errors::AppError>(())
//! ```

// Re-export all types and modules from pierre-providers
#[cfg(feature = "provider-coros")]
pub use pierre_providers::coros_provider;
#[cfg(feature = "provider-fitbit")]
pub use pierre_providers::fitbit_provider;
#[cfg(feature = "provider-garmin")]
pub use pierre_providers::garmin_provider;
#[cfg(feature = "provider-strava")]
pub use pierre_providers::strava_provider;
#[cfg(feature = "provider-terra")]
pub use pierre_providers::terra;
#[cfg(feature = "provider-whoop")]
pub use pierre_providers::whoop_provider;
pub use pierre_providers::*;
pub use pierre_providers::{activity_iterator, circuit_breaker, core, http_client, spi, utils};

// Local modules that remain in the main crate (database/cache/config dependencies)

/// Caching decorator for transparent API response caching
pub mod caching_provider;
/// Provider error types and result aliases
pub mod errors;
/// Global provider registry and factory
pub mod registry;
/// Synthetic provider for development and testing
#[cfg(feature = "provider-synthetic")]
pub mod synthetic_provider;

// Re-export caching provider types
pub use caching_provider::{
    create_caching_provider, create_caching_provider_with_ttl, CachePolicy, CachingFitnessProvider,
};
// Re-export registry functions
#[cfg(feature = "provider-terra")]
pub use registry::global_terra_cache;
pub use registry::{
    create_caching_provider_global, create_caching_provider_with_admin_config_global,
    create_provider, create_registry_with_external_providers, create_tenant_provider,
    get_supported_providers, global_registry, is_provider_supported, ProviderRegistry,
};
#[cfg(feature = "provider-synthetic")]
pub use synthetic_provider::{get_synthetic_database_pool, set_synthetic_database_pool};
