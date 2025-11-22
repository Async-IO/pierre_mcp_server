// ABOUTME: FitnessProvider trait and provider core abstractions
// ABOUTME: Domain-specific provider interface for fitness data sources

pub mod core;

// Re-export key types
pub use core::{
    FitnessProvider, OAuth2Credentials, ProviderConfig, ProviderFactory, TenantProvider,
};
