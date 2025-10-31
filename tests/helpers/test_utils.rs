// ABOUTME: Test utilities for synthetic provider and intelligence validation
// ABOUTME: Provides scenario builders and validation assertion helpers for automated testing

use pierre_mcp_server::models::Activity;

use super::synthetic_data::{SyntheticDataBuilder, TrainingPattern};
use super::synthetic_provider::SyntheticProvider;

/// Test scenarios for intelligence testing
#[derive(Debug, Clone, Copy)]
pub enum TestScenario {
    /// Beginner runner showing 35% improvement over 6 weeks
    BeginnerRunnerImproving,
    /// Experienced cyclist with stable, consistent performance - Reserved for future cyclist tests
    #[allow(dead_code)]
    ExperiencedCyclistConsistent,
    /// Athlete showing signs of overtraining (TSB < -30) - Reserved for future overtraining tests
    #[allow(dead_code)]
    OvertrainingRisk,
    /// Return from injury with gradual progression - Reserved for future injury recovery tests
    #[allow(dead_code)]
    InjuryRecovery,
    /// Pre-race taper with volume reduction - Reserved for future tapering algorithm tests
    #[allow(dead_code)]
    PeakingForRace,
    /// Aerobic base building phase - Reserved for future base building tests
    #[allow(dead_code)]
    BaseBuilding,
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
            Self::PeakingForRace => TrainingPattern::PeakingForRace,
            Self::BaseBuilding => TrainingPattern::BaseBuilding,
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

/// Create a synthetic provider with custom activities
/// Reserved for future custom activity generation tests
#[must_use]
#[allow(dead_code)]
pub fn create_synthetic_provider_with_activities(activities: Vec<Activity>) -> SyntheticProvider {
    SyntheticProvider::with_activities(activities)
}

/// Create an empty synthetic provider
/// Reserved for future empty state handling tests
#[must_use]
#[allow(dead_code)]
pub fn create_empty_synthetic_provider() -> SyntheticProvider {
    SyntheticProvider::new()
}

// ================================================================================================
// Validation Helpers for Intelligence Testing
// ================================================================================================

/// Assert that a pace value is within acceptable tolerance
///
/// Reserved for future pace-based performance validation tests
///
/// # Arguments
/// * `actual` - The calculated pace (seconds per meter)
/// * `expected` - The expected pace (seconds per meter)
/// * `tolerance` - Acceptable difference as a percentage (e.g., 0.05 for 5%)
///
/// # Panics
/// Panics if the actual value is outside the tolerance range
#[allow(dead_code)]
pub fn assert_pace_within_range(actual: f64, expected: f64, tolerance: f64) {
    let diff = (actual - expected).abs();
    let max_diff = expected * tolerance;
    assert!(
        diff <= max_diff,
        "Pace out of tolerance: actual={:.4} s/m, expected={:.4} s/m, diff={:.4} s/m, max_diff={:.4} s/m ({}%)",
        actual,
        expected,
        diff,
        max_diff,
        tolerance * 100.0
    );
}

/// Assert that a VDOT calculation is accurate within tolerance
///
/// Reserved for future `VDOT` methodology validation tests
///
/// `VDOT` methodology allows ±6% variance per `methodology.md`
///
/// # Arguments
/// * `actual` - The calculated `VDOT` value
/// * `expected` - The expected `VDOT` value
/// * `tolerance` - Acceptable difference as a percentage (typically 0.06 for 6%)
///
/// # Panics
/// Panics if the actual `VDOT` is outside the tolerance range
#[allow(dead_code)]
pub fn assert_vdot_accurate(actual: f64, expected: f64, tolerance: f64) {
    let diff = (actual - expected).abs();
    let max_diff = expected * tolerance;
    assert!(
        diff <= max_diff,
        "VDOT out of tolerance: actual={:.2}, expected={:.2}, diff={:.2}, max_diff={:.2} ({}%)",
        actual,
        expected,
        diff,
        max_diff,
        tolerance * 100.0
    );
}

/// Assert that a `CTL` (Chronic Training Load) calculation is accurate
///
/// Reserved for future `CTL` algorithm validation tests
///
/// `CTL` uses 42-day exponential moving average of `TSS`
///
/// # Arguments
/// * `actual` - The calculated `CTL` value
/// * `expected` - The expected `CTL` value
/// * `tolerance` - Acceptable difference as absolute value (not percentage)
///
/// # Panics
/// Panics if the actual `CTL` differs from expected by more than tolerance
#[allow(dead_code)]
pub fn assert_ctl_calculation(actual: f64, expected: f64, tolerance: f64) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tolerance,
        "CTL out of tolerance: actual={actual:.2}, expected={expected:.2}, diff={diff:.2}, max_diff={tolerance:.2}"
    );
}

/// Assert that an `ATL` (Acute Training Load) calculation is accurate
///
/// Reserved for future `ATL` algorithm validation tests
///
/// `ATL` uses 7-day exponential moving average of `TSS`
///
/// # Arguments
/// * `actual` - The calculated `ATL` value
/// * `expected` - The expected `ATL` value
/// * `tolerance` - Acceptable difference as absolute value (not percentage)
///
/// # Panics
/// Panics if the actual `ATL` differs from expected by more than tolerance
#[allow(dead_code)]
pub fn assert_atl_calculation(actual: f64, expected: f64, tolerance: f64) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tolerance,
        "ATL out of tolerance: actual={actual:.2}, expected={expected:.2}, diff={diff:.2}, max_diff={tolerance:.2}"
    );
}

/// Assert that a `TSB` (Training Stress Balance) calculation is accurate
///
/// Reserved for future `TSB` algorithm validation tests
///
/// `TSB` = `CTL` - `ATL`
///
/// # Arguments
/// * `actual` - The calculated `TSB` value
/// * `expected` - The expected `TSB` value
/// * `tolerance` - Acceptable difference as absolute value (not percentage)
///
/// # Panics
/// Panics if the actual `TSB` differs from expected by more than tolerance
#[allow(dead_code)]
pub fn assert_tsb_calculation(actual: f64, expected: f64, tolerance: f64) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tolerance,
        "TSB out of tolerance: actual={actual:.2}, expected={expected:.2}, diff={diff:.2}, max_diff={tolerance:.2}"
    );
}

/// Assert that a trend slope is within expected range
///
/// Reserved for future performance trend analysis validation tests
///
/// Used for performance trend analysis to verify improving/declining/stable patterns
///
/// # Arguments
/// * `actual` - The calculated slope
/// * `expected` - The expected slope
/// * `tolerance` - Acceptable difference as absolute value
///
/// # Panics
/// Panics if the actual slope differs from expected by more than tolerance
#[allow(dead_code)]
pub fn assert_trend_slope(actual: f64, expected: f64, tolerance: f64) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tolerance,
        "Trend slope out of tolerance: actual={actual:.6}, expected={expected:.6}, diff={diff:.6}, max_diff={tolerance:.6}"
    );
}

/// Assert that an `R²` (coefficient of determination) value indicates good fit
///
/// Reserved for future regression analysis validation tests
///
/// `R²` ranges from 0 to 1, where 1 is perfect fit
///
/// # Arguments
/// * `r_squared` - The calculated `R²` value
/// * `min_acceptable` - Minimum acceptable `R²` (typically 0.85 for good fit)
///
/// # Panics
/// Panics if `R²` is below the minimum acceptable value
#[allow(dead_code)]
pub fn assert_r_squared_good_fit(r_squared: f64, min_acceptable: f64) {
    assert!(
        r_squared >= min_acceptable,
        "R² indicates poor fit: actual={r_squared:.4}, minimum={min_acceptable:.4}"
    );
}
