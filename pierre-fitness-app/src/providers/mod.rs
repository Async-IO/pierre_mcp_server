// ABOUTME: FitnessProvider trait and provider core abstractions
// ABOUTME: Re-exports framework provider types until full migration

// Re-export framework provider types (temporary during migration)
pub use pierre_mcp_server::providers::config::{OAuth2Credentials, ProviderConfig};
pub use pierre_mcp_server::providers::{CoreFitnessProvider as FitnessProvider, ProviderFactory, TenantProvider};
