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
//! ```rust
//! use crate::providers::{create_provider, create_tenant_provider};
//! use crate::constants::oauth_providers;
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
//! ```

// Core provider system
pub mod core;
pub mod registry;

// Provider implementations
pub mod strava_provider;

// Legacy modules - marked for removal
#[deprecated(note = "Use the new provider system via registry::create_provider instead")]
pub mod fitbit;
#[deprecated(note = "Use the new provider system via registry::create_provider instead")]
pub mod strava;
#[deprecated(note = "Use the new provider system via registry::create_tenant_provider instead")]
pub mod strava_tenant;
#[deprecated(note = "Use the new provider system via registry::create_tenant_provider instead")]  
pub mod tenant_provider;

// Re-export key types for convenience  
pub use core::{FitnessProvider as CoreFitnessProvider, OAuth2Credentials, ProviderConfig, TenantProvider};
pub use registry::{
    create_provider, create_tenant_provider, get_supported_providers, 
    is_provider_supported, ProviderRegistry
};

// Legacy re-exports for backward compatibility - will be removed
#[deprecated(note = "Use registry::create_tenant_provider instead")]
pub use strava_tenant::TenantStravaProvider;
#[deprecated(note = "Use registry::create_tenant_provider instead")]
pub use tenant_provider::{TenantFitnessProvider, TenantProviderFactory};

// Legacy trait for backward compatibility - will be removed  
#[deprecated(note = "Use core::FitnessProvider instead")]
#[async_trait::async_trait]
pub trait FitnessProvider: Send + Sync {
    async fn authenticate(&mut self, auth_data: AuthData) -> anyhow::Result<()>;
    async fn get_athlete(&self) -> anyhow::Result<crate::models::Athlete>;
    async fn get_activities(&self, limit: Option<usize>, offset: Option<usize>) -> anyhow::Result<Vec<crate::models::Activity>>;
    async fn get_activity(&self, id: &str) -> anyhow::Result<crate::models::Activity>;
    async fn get_stats(&self) -> anyhow::Result<crate::models::Stats>;
    async fn get_personal_records(&self) -> anyhow::Result<Vec<crate::models::PersonalRecord>>;
    fn provider_name(&self) -> &'static str;
}

#[deprecated(note = "Use core::OAuth2Credentials instead")]
#[derive(Debug, Clone)]
pub enum AuthData {
    OAuth2 {
        client_id: String,
        client_secret: String,
        access_token: Option<String>,
        refresh_token: Option<String>,
    },
    ApiKey(String),
}