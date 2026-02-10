// ABOUTME: Fitness data provider implementations for Strava, Garmin, Fitbit, WHOOP, COROS, Terra
// ABOUTME: Core provider traits, circuit breaker, retry utilities, and streaming activity iteration
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Fitness data provider implementations and core abstractions.
//!
//! This crate provides the unified provider system for integrating with external
//! fitness data sources (Strava, Garmin, Fitbit, WHOOP, COROS, Terra).

// Re-export pierre-core modules so moved files can keep `use crate::errors::*` etc.
pub use pierre_core::constants;
pub use pierre_core::errors;
pub use pierre_core::models;
pub use pierre_core::pagination;

// Core provider infrastructure
/// Streaming activity iterator for memory-efficient paginated fetching
pub mod activity_iterator;
/// Circuit breaker pattern for provider resilience
pub mod circuit_breaker;
/// Core provider traits and interfaces
pub mod core;
/// Shared HTTP client for provider API calls
pub mod http_client;
/// Service Provider Interface for external providers
pub mod spi;
/// Provider utility functions (retry, type conversion)
pub mod utils;

// Provider implementations (conditionally compiled)

/// COROS provider for GPS sports watch data
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
/// Terra unified API provider (150+ wearables)
#[cfg(feature = "provider-terra")]
pub mod terra;
/// WHOOP provider for sleep, recovery, and workout data
#[cfg(feature = "provider-whoop")]
pub mod whoop_provider;

// Re-export key types for convenience

pub use activity_iterator::{
    create_activity_stream, ActivityStream, ActivityStreamExt, StreamConfig, DEFAULT_PAGE_SIZE,
    MAX_PAGE_SIZE, MIN_PAGE_SIZE,
};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
pub use core::{
    ActivityQueryParams, FitnessProvider as CoreFitnessProvider, OAuth2Credentials, ProviderConfig,
    ProviderFactory, TenantProvider,
};
pub use http_client::{initialize_shared_client, shared_client};
pub use pierre_core::errors::provider::{ProviderError, ProviderResult};
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
#[cfg(feature = "provider-synthetic")]
pub use spi::SyntheticSleepDescriptor;
#[cfg(feature = "provider-whoop")]
pub use spi::WhoopDescriptor;
pub use spi::{OAuthEndpoints, ProviderBundle, ProviderCapabilities, ProviderDescriptor};
#[cfg(feature = "provider-terra")]
pub use terra::{
    TerraDataCache, TerraDescriptor, TerraProvider, TerraProviderFactory, TerraWebhookHandler,
};
pub use utils::{
    with_retry, with_retry_default, RetryBackoffConfig, ENV_RETRY_BASE_DELAY_MS,
    ENV_RETRY_JITTER_FACTOR, ENV_RETRY_MAX_ATTEMPTS, ENV_RETRY_MAX_DELAY_MS,
};
