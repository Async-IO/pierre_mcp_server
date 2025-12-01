// ABOUTME: Verification test for synthetic data helpers and test utilities
// ABOUTME: Ensures the helpers module structure works correctly before writing full integration tests
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

mod helpers;

use helpers::synthetic_data::{SyntheticDataBuilder, TrainingPattern};
use helpers::test_utils::{create_synthetic_provider_with_scenario, TestScenario};
use pierre_mcp_server::providers::core::FitnessProvider;

#[test]
fn test_helpers_module_structure() {
    // Verify we can create a synthetic data builder
    let mut builder = SyntheticDataBuilder::new(42);
    let activities = builder.generate_pattern(TrainingPattern::BeginnerRunnerImproving);
    assert!(!activities.is_empty(), "Should generate activities");

    // Verify we can create a provider with a scenario
    let provider = create_synthetic_provider_with_scenario(TestScenario::BeginnerRunnerImproving);

    assert_eq!(
        provider.name(),
        "synthetic",
        "Provider name should be synthetic"
    );
}

#[test]
fn test_all_test_scenarios() {
    // Test all TestScenario variants to ensure they work correctly
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
        assert!(
            !activities.is_empty(),
            "Scenario should generate activities"
        );
    }
}
