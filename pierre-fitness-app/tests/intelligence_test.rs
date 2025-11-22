// ABOUTME: Unit tests for activity intelligence and performance analysis features
// ABOUTME: Tests intelligence module components including metrics, trends, and contextual factors
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::intelligence::{
    ActivityIntelligence, ContextualFactors, PerformanceMetrics, TimeOfDay, TrendDirection,
    TrendIndicators, ZoneDistribution,
};

#[test]
fn test_activity_intelligence_creation() {
    let intelligence = ActivityIntelligence::new(
        "Great morning run!".into(),
        vec![],
        PerformanceMetrics {
            relative_effort: Some(5.0),
            zone_distribution: None,
            personal_records: vec![],
            efficiency_score: Some(85.0),
            trend_indicators: TrendIndicators {
                pace_trend: TrendDirection::Improving,
                effort_trend: TrendDirection::Stable,
                distance_trend: TrendDirection::Stable,
                consistency_score: 90.0,
            },
        },
        ContextualFactors {
            weather: None,
            location: None,
            time_of_day: TimeOfDay::Morning,
            days_since_last_activity: Some(1),
            weekly_load: None,
        },
    );

    assert_eq!(intelligence.summary, "Great morning run!");
    assert_eq!(
        intelligence.performance_indicators.relative_effort,
        Some(5.0)
    );
}

#[test]
fn test_zone_distribution() {
    let zones = ZoneDistribution {
        zone1_recovery: 10.0,
        zone2_endurance: 65.0,
        zone3_tempo: 20.0,
        zone4_threshold: 5.0,
        zone5_vo2max: 0.0,
    };

    assert!((zones.zone2_endurance - 65.0).abs() < f32::EPSILON);

    // Total should be 100%
    let total = zones.zone1_recovery
        + zones.zone2_endurance
        + zones.zone3_tempo
        + zones.zone4_threshold
        + zones.zone5_vo2max;
    assert!((total - 100.0).abs() < f32::EPSILON);
}
