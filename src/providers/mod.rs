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

// Core provider system

/// Core provider traits and interfaces
pub mod core;
/// Provider error types and result aliases
pub mod errors;
/// Global provider registry and factory
pub mod registry;
/// Provider utility functions
pub mod utils;

// Provider implementations

/// Garmin Connect provider implementation
pub mod garmin_provider;
/// Strava API provider implementation
pub mod strava_provider;
/// Synthetic provider for development and testing
pub mod synthetic_provider;

// Re-export key types for convenience
pub use core::{
    FitnessProvider as CoreFitnessProvider, OAuth2Credentials, ProviderConfig, TenantProvider,
};
/// Re-export provider error types
pub use errors::{ProviderError, ProviderResult};
/// Re-export provider registry functions
pub use registry::{
    create_provider, create_tenant_provider, get_supported_providers, is_provider_supported,
    ProviderRegistry,
};
