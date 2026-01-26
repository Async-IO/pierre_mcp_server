// ABOUTME: Tests for social insights configuration validation
// ABOUTME: Validates config defaults, relevance scoring, and validation rules
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::config::social::{
    DistanceRelevanceScores, MilestoneRelevanceScores, SocialInsightsConfig, StreakRelevanceScores,
};

#[test]
fn test_default_config_is_valid() {
    let config = SocialInsightsConfig::default();
    assert!(config.validate().is_ok());
}

#[test]
fn test_milestone_relevance_scores() {
    let scores = MilestoneRelevanceScores::default();
    assert_eq!(scores.score_for_milestone(1500), 95);
    assert_eq!(scores.score_for_milestone(500), 90);
    assert_eq!(scores.score_for_milestone(250), 85);
    assert_eq!(scores.score_for_milestone(100), 80);
    assert_eq!(scores.score_for_milestone(50), 75);
    assert_eq!(scores.score_for_milestone(25), 70);
    assert_eq!(scores.score_for_milestone(10), 65);
}

#[test]
fn test_distance_relevance_scores() {
    let scores = DistanceRelevanceScores::default();
    assert_eq!(scores.score_for_distance(15000.0), 95);
    assert_eq!(scores.score_for_distance(5000.0), 90);
    assert_eq!(scores.score_for_distance(2500.0), 85);
    assert_eq!(scores.score_for_distance(1000.0), 80);
    assert_eq!(scores.score_for_distance(500.0), 75);
    assert_eq!(scores.score_for_distance(100.0), 70);
}

#[test]
fn test_streak_relevance_scores() {
    let scores = StreakRelevanceScores::default();
    assert_eq!(scores.score_for_streak(400), 95);
    assert_eq!(scores.score_for_streak(200), 90);
    assert_eq!(scores.score_for_streak(100), 85);
    assert_eq!(scores.score_for_streak(70), 80);
    assert_eq!(scores.score_for_streak(40), 75);
    assert_eq!(scores.score_for_streak(10), 70);
}

#[test]
fn test_invalid_milestone_order() {
    let mut config = SocialInsightsConfig::default();
    config.milestone_thresholds.activity_counts = vec![100, 50, 25]; // Wrong order
    assert!(config.validate().is_err());
}

#[test]
fn test_invalid_distance_order() {
    let mut config = SocialInsightsConfig::default();
    config.distance_milestones.thresholds_km = vec![1000.0, 500.0]; // Wrong order
    assert!(config.validate().is_err());
}

#[test]
fn test_invalid_streak_order() {
    let mut config = SocialInsightsConfig::default();
    config.streak_config.milestone_days = vec![30, 14, 7]; // Wrong order
    assert!(config.validate().is_err());
}

#[test]
fn test_zero_limits_invalid() {
    let mut config = SocialInsightsConfig::default();
    config.activity_fetch_limits.insight_context_limit = 0;
    assert!(config.validate().is_err());
}
