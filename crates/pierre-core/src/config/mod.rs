// ABOUTME: Configuration types shared across the workspace
// ABOUTME: Contains FitnessConfig and FitnessLevel used by models

/// Fitness-specific configuration for training zones, thresholds, and sport parameters
pub mod fitness;

/// User profile configuration including FitnessLevel
pub mod profiles;

pub use fitness::FitnessConfig;
