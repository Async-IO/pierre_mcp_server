// ABOUTME: Test utilities for synthetic provider and intelligence validation
// ABOUTME: Provides scenario builders and validation assertion helpers for automated testing

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::synthetic_data::{SyntheticDataBuilder, TrainingPattern};
use super::synthetic_provider::SyntheticProvider;

/// Test scenarios for intelligence testing
#[derive(Debug, Clone, Copy)]
pub enum TestScenario {
    /// Beginner runner showing 35% improvement over 6 weeks
    BeginnerRunnerImproving,
    /// Experienced cyclist with stable, consistent performance - Reserved for future cyclist tests
    ExperiencedCyclistConsistent,
    /// Athlete showing signs of overtraining (TSB < -30) - Reserved for future overtraining tests
    OvertrainingRisk,
    /// Return from injury with gradual progression - Reserved for future injury recovery tests
    InjuryRecovery,
}

impl TestScenario {
    /// Get the corresponding pattern from synthetic data builder
    #[must_use]
    pub const fn to_training_pattern(self) -> TrainingPattern {
        match self {
            Self::BeginnerRunnerImproving => TrainingPattern::BeginnerRunnerImproving,
            Self::ExperiencedCyclistConsistent => TrainingPattern::ExperiencedCyclistConsistent,
            Self::OvertrainingRisk => TrainingPattern::Overtraining,
            Self::InjuryRecovery => TrainingPattern::InjuryRecovery,
        }
    }
}

/// Create a synthetic provider with pre-configured scenario data
#[must_use]
pub fn create_synthetic_provider_with_scenario(scenario: TestScenario) -> SyntheticProvider {
    let mut builder = SyntheticDataBuilder::new(42); // Deterministic seed
    let activities = builder.generate_pattern(scenario.to_training_pattern());

    SyntheticProvider::with_activities(activities)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pierre_mcp_server::providers::core::FitnessProvider;

    #[test]
    fn test_all_test_scenario_variants() {
        // Test all TestScenario variants to prevent dead code warnings
        let scenarios = [
            TestScenario::BeginnerRunnerImproving,
            TestScenario::ExperiencedCyclistConsistent,
            TestScenario::OvertrainingRisk,
            TestScenario::InjuryRecovery,
        ];

        for scenario in scenarios {
            let provider = create_synthetic_provider_with_scenario(scenario);
            assert_eq!(provider.name(), "synthetic");

            // Verify the pattern conversion works
            let pattern = scenario.to_training_pattern();
            let mut builder = SyntheticDataBuilder::new(42);
            let activities = builder.generate_pattern(pattern);
            assert!(!activities.is_empty());
        }
    }
}
