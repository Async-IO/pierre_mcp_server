// ABOUTME: Integration tests for basic intelligence tools using synthetic data
// ABOUTME: Tests get_athlete, get_activities, get_activity, get_stats, and compare_activities tools
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

mod helpers;

use helpers::test_utils::{create_synthetic_provider_with_scenario, TestScenario};
use pierre_mcp_server::models::SportType;
use pierre_mcp_server::pagination::PaginationParams;
use pierre_mcp_server::providers::core::FitnessProvider;

#[tokio::test]
async fn test_get_athlete_with_synthetic_data() {
    // Create synthetic provider with beginner runner data
    let provider = create_synthetic_provider_with_scenario(TestScenario::BeginnerRunnerImproving);

    // Verify provider returns athlete info
    let athlete = provider.get_athlete().await;
    assert!(athlete.is_ok(), "Should successfully get athlete info");

    let athlete = athlete.unwrap();
    assert_eq!(athlete.id, "synthetic_athlete_001");
    assert_eq!(athlete.username, "test_athlete");
}

#[tokio::test]
async fn test_get_activities_with_beginner_runner() {
    let provider = create_synthetic_provider_with_scenario(TestScenario::BeginnerRunnerImproving);

    // Get activities from provider
    let result = provider.get_activities(Some(50), None).await;

    assert!(result.is_ok(), "Should successfully get activities");

    let activities = result.unwrap();
    assert!(!activities.is_empty(), "Should have activities");
    assert!(
        activities.len() >= 20,
        "Beginner runner pattern should generate at least 20 activities"
    );

    // Verify activities are running activities
    let run_count = activities
        .iter()
        .filter(|a| a.sport_type == SportType::Run)
        .count();
    assert!(
        run_count > 0,
        "Should have running activities for runner pattern"
    );
}

#[tokio::test]
async fn test_get_activities_cursor_pagination() {
    let provider =
        create_synthetic_provider_with_scenario(TestScenario::ExperiencedCyclistConsistent);

    // Get first page with limit using cursor-based pagination
    let params = PaginationParams::forward(None, 10);
    let result = provider.get_activities_cursor(&params).await;
    assert!(result.is_ok(), "First page should succeed");

    let page = result.unwrap();
    assert_eq!(page.items.len(), 10, "Should return exactly 10 activities");
    assert!(page.has_more, "Should indicate more data available");

    // Get second page using cursor
    if let Some(next_cursor) = &page.next_cursor {
        let params2 = PaginationParams::forward(Some(next_cursor.clone()), 10);
        let result2 = provider.get_activities_cursor(&params2).await;
        assert!(result2.is_ok(), "Second page should succeed");

        let page2 = result2.unwrap();
        assert_eq!(page2.items.len(), 10, "Second page should have 10 items");

        // Verify no duplicate activities
        let first_ids: Vec<_> = page.items.iter().map(|a| &a.id).collect();
        let second_ids: Vec<_> = page2.items.iter().map(|a| &a.id).collect();

        for id in &second_ids {
            assert!(
                !first_ids.contains(id),
                "Pages should not contain duplicate activities"
            );
        }
    }
}

#[tokio::test]
async fn test_get_activity_by_id() {
    let provider = create_synthetic_provider_with_scenario(TestScenario::BeginnerRunnerImproving);

    // Get all activities first
    let activities = provider
        .get_activities(Some(50), None)
        .await
        .expect("Should get activities");

    assert!(!activities.is_empty(), "Should have activities to test");

    // Get first activity by ID
    let first_activity_id = &activities[0].id;
    let result = provider.get_activity(first_activity_id).await;

    assert!(
        result.is_ok(),
        "Should successfully get activity by ID: {first_activity_id}"
    );

    let activity = result.unwrap();
    assert_eq!(&activity.id, first_activity_id, "IDs should match");
    assert!(
        activity.duration_seconds > 0,
        "Activity should have duration"
    );
    assert!(activity.distance_meters.is_some(), "Should have distance");
}

#[tokio::test]
async fn test_get_activity_nonexistent_id() {
    let provider = create_synthetic_provider_with_scenario(TestScenario::BeginnerRunnerImproving);

    let result = provider.get_activity("nonexistent_id_12345").await;

    assert!(result.is_err(), "Should return error for nonexistent ID");
}

#[tokio::test]
async fn test_get_stats_with_synthetic_data() {
    let provider =
        create_synthetic_provider_with_scenario(TestScenario::ExperiencedCyclistConsistent);

    let stats = provider.get_stats().await;
    assert!(stats.is_ok(), "Should successfully get stats");

    let stats = stats.unwrap();

    // Verify basic stats structure
    assert!(
        stats.total_activities > 0,
        "Should have total activities count"
    );
    assert!(stats.total_distance > 0.0, "Should have total distance");
    assert!(stats.total_duration > 0, "Should have total duration");
    assert!(
        stats.total_elevation_gain >= 0.0,
        "Should have elevation gain data"
    );
}

#[tokio::test]
async fn test_compare_activities_different_patterns() {
    let provider = create_synthetic_provider_with_scenario(TestScenario::BeginnerRunnerImproving);

    // Get activities
    let activities = provider
        .get_activities(Some(50), None)
        .await
        .expect("Should get activities");

    assert!(
        activities.len() >= 2,
        "Need at least 2 activities to compare"
    );

    // Compare first and last activities (should show improvement for beginner pattern)
    let first = &activities[0];
    let last = &activities[activities.len() - 1];

    // Verify both are running activities with comparable data
    assert_eq!(
        first.sport_type, last.sport_type,
        "Should compare same sport"
    );
    assert!(
        first.duration_seconds > 0 && last.duration_seconds > 0,
        "Both should have duration"
    );

    // For beginner improving pattern, later activities should show better performance
    if let (Some(first_dist), Some(last_dist)) = (first.distance_meters, last.distance_meters) {
        // Duration in seconds, precision loss acceptable for pace calculation
        #[allow(clippy::cast_precision_loss)]
        let first_pace = first.duration_seconds as f64 / first_dist;
        #[allow(clippy::cast_precision_loss)]
        let last_pace = last.duration_seconds as f64 / last_dist;

        // Beginner improving should show faster pace over time (lower seconds per meter)
        assert!(
            last_pace < first_pace * 1.2,
            "Pace should improve or stay within 20% for improving pattern"
        );
    }
}

#[tokio::test]
async fn test_overtraining_pattern_detection() {
    let provider = create_synthetic_provider_with_scenario(TestScenario::OvertrainingRisk);

    let activities = provider
        .get_activities(Some(50), None)
        .await
        .expect("Should get activities");

    assert!(
        activities.len() >= 10,
        "Should have enough activities to detect pattern"
    );

    // Overtraining pattern should show high volume with declining performance
    // activities are already sorted descending (most recent first), so just take first 7
    let recent_activities: Vec<_> = activities.iter().take(7).collect();

    let total_duration: u64 = recent_activities.iter().map(|a| a.duration_seconds).sum();

    // Overtraining pattern should have high weekly training volume
    assert!(
        total_duration > 7 * 3600,
        "Overtraining pattern should show high volume (>7 hours/week)"
    );
}

#[tokio::test]
async fn test_injury_recovery_pattern() {
    let provider = create_synthetic_provider_with_scenario(TestScenario::InjuryRecovery);

    let activities = provider
        .get_activities(Some(50), None)
        .await
        .expect("Should get activities");

    // Injury recovery pattern should show a gap followed by gradual return
    let timestamps: Vec<_> = activities
        .iter()
        .map(|a| a.start_date.timestamp())
        .collect();

    // Check for gaps in training (indicators of recovery period)
    let mut max_gap = 0i64;
    for window in timestamps.windows(2) {
        let gap = (window[1] - window[0]).abs();
        if gap > max_gap {
            max_gap = gap;
        }
    }

    // Should have at least one significant gap (7+ days)
    assert!(
        max_gap > 7 * 24 * 3600,
        "Injury recovery pattern should show training gap"
    );
}
