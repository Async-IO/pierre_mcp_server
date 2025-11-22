// ABOUTME: Fitness-specific configuration modules
// ABOUTME: Re-exports framework fitness configs until full migration

// Re-export framework config modules (temporary during migration)
pub use pierre_mcp_server::config::fitness_config;
pub use pierre_mcp_server::config::intelligence_config;

// Re-export key types for convenience
pub use fitness_config::FitnessConfig;
pub use intelligence_config::IntelligenceConfig;
