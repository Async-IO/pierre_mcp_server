// ABOUTME: Verification test for synthetic data helpers and test utilities
// ABOUTME: Ensures the helpers module structure works correctly before writing full integration tests

mod helpers;

use helpers::synthetic_data::{SyntheticDataBuilder, TrainingPattern};
use helpers::test_utils::{
    assert_pace_within_range, assert_vdot_accurate, create_synthetic_provider_with_scenario,
    TestScenario,
};
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
fn test_validation_helpers() {
    // Verify validation helpers work correctly
    assert_pace_within_range(10.0, 10.0, 0.05);
    assert_pace_within_range(10.2, 10.0, 0.05);
    assert_pace_within_range(9.8, 10.0, 0.05);

    assert_vdot_accurate(50.0, 50.0, 0.06);
    assert_vdot_accurate(51.0, 50.0, 0.06);
    assert_vdot_accurate(49.0, 50.0, 0.06);
}

#[test]
#[should_panic(expected = "Pace out of tolerance")]
fn test_pace_validation_fails_outside_tolerance() {
    assert_pace_within_range(11.0, 10.0, 0.05);
}

#[test]
#[should_panic(expected = "VDOT out of tolerance")]
fn test_vdot_validation_fails_outside_tolerance() {
    assert_vdot_accurate(60.0, 50.0, 0.06);
}
