// ABOUTME: Intelligence module re-exports for algorithm types and policies
// ABOUTME: Contains MaxHrAlgorithm and InsightSharingPolicy used by models

/// Maximum heart rate estimation algorithms
pub mod algorithms;

/// Insight sharing policy for social features
mod insight_sharing_policy;

pub use insight_sharing_policy::InsightSharingPolicy;
