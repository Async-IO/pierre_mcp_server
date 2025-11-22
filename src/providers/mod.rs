// ABOUTME: Unified fitness data provider system with clean abstractions and multi-tenant support
// ABOUTME: Replaces the previous fragmented provider implementations with a single, extensible architecture

//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

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

// Generic provider infrastructure (domain-agnostic)

/// Generic provider configuration (OAuth credentials, endpoints)
pub mod config;
/// Core provider traits and interfaces
pub mod core;
/// Provider error types and result aliases
pub mod errors;
/// Generic provider registry (for external providers)
pub mod registry;
/// Service Provider Interface (SPI) for external providers
pub mod spi;
/// Provider utility functions
pub mod utils;

// Re-export key types for convenience

/// Re-export configuration types
pub use config::{OAuth2Credentials, ProviderConfig};
/// Re-export core provider traits
pub use core::{FitnessProvider as CoreFitnessProvider, ProviderFactory, TenantProvider};
/// Re-export provider error types
pub use errors::{ProviderError, ProviderResult};
/// Re-export registry types
pub use registry::{
    create_provider, create_registry_with_external_providers, create_tenant_provider,
    get_supported_providers, global_registry, is_provider_supported, ProviderBundle,
    ProviderFactoryFn, ProviderRegistry,
};
/// Re-export SPI types for external provider development
pub use spi::{OAuthEndpoints, ProviderCapabilities, ProviderDescriptor};
