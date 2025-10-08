// ABOUTME: Unified fitness data provider system with clean abstractions and multi-tenant support
// ABOUTME: Replaces the previous fragmented provider implementations with a single, extensible architecture

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

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
//! let mut provider = create_provider(oauth_providers::STRAVA)?;
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
pub mod core;
pub mod registry;

// Provider implementations
pub mod strava_provider;

// Re-export key types for convenience
pub use core::{
    FitnessProvider as CoreFitnessProvider, OAuth2Credentials, ProviderConfig, TenantProvider,
};
pub use registry::{
    create_provider, create_tenant_provider, get_supported_providers, is_provider_supported,
    ProviderRegistry,
};
