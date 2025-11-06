// ABOUTME: Unit tests for configuration profiles functionality
// ABOUTME: Validates configuration profiles behavior, edge cases, and error handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::configuration::profiles::{ConfigProfile, FitnessLevel, ProfileTemplates};

#[test]
fn test_profile_names() {
    assert_eq!(ConfigProfile::Default.name(), "default");
    assert_eq!(
        ConfigProfile::Elite {
            performance_factor: 1.1,
            recovery_sensitivity: 1.2
        }
        .name(),
        "elite"
    );
}

#[test]
fn test_elite_from_vo2_max() {
    let profile = ConfigProfile::elite_from_vo2_max(65.0);
    if let ConfigProfile::Elite {
        performance_factor, ..
    } = profile
    {
        assert!((performance_factor - 1.15).abs() < f64::EPSILON);
    } else {
        panic!("Expected Elite profile");
    }
}

#[test]
fn test_fitness_level_from_vo2_max() {
    assert_eq!(
        FitnessLevel::from_vo2_max(35.0, None, Some("M")),
        FitnessLevel::Recreational
    );
    assert_eq!(
        FitnessLevel::from_vo2_max(55.0, None, Some("M")),
        FitnessLevel::Elite
    );
    assert_eq!(
        FitnessLevel::from_vo2_max(45.0, None, Some("F")),
        FitnessLevel::Advanced
    );
}

#[test]
fn test_profile_adjustments() {
    let profile = ConfigProfile::Beginner {
        threshold_reduction: 0.85,
        simplified_metrics: true,
    };

    let adjustments = profile.get_adjustments();
    assert_eq!(adjustments.get("threshold_multiplier"), Some(&0.85));
    assert_eq!(adjustments.get("achievement_sensitivity"), Some(&1.2));
}

#[test]
fn test_profile_templates() {
    let templates = ProfileTemplates::all();
    assert!(templates.len() >= 9);

    let research = ProfileTemplates::get("research");
    assert!(research.is_some());
    assert!(matches!(research.unwrap(), ConfigProfile::Research { .. }));
}
