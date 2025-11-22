// ABOUTME: FitnessProvider trait and provider core abstractions
// ABOUTME: Domain-specific provider interface for fitness data sources

pub mod config;
pub mod core;

// Re-export key types
pub use config::{OAuth2Credentials, ProviderConfig};
pub use core::{FitnessProvider, ProviderFactory, TenantProvider};
