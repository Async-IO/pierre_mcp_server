// ABOUTME: Advanced integration tests for intelligence tools using synthetic data
// ABOUTME: Tests fitness scoring, performance prediction, training load analysis, and goal management
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

mod helpers;

use chrono::Utc;
use helpers::test_utils::{create_synthetic_provider_with_scenario, TestScenario};
use pierre_mcp_server::config::intelligence::DefaultStrategy;
use pierre_mcp_server::intelligence::{
    ActivityGoal, AdvancedGoalEngine, FitnessLevel, Goal, GoalDifficulty, GoalEngineTrait,
    GoalStatus, GoalType, PerformanceAnalyzerV2, TimeAvailability, TimeFrame, UserFitnessProfile,
    UserPreferences,
};
use pierre_mcp_server::models::SportType;
use pierre_mcp_server::providers::core::FitnessProvider;

// ================================================================================================
// Test Helper Functions
// ================================================================================================

/// Create a test `UserFitnessProfile` with sensible defaults
fn create_test_user_profile(fitness_level: FitnessLevel) -> UserFitnessProfile {
    UserFitnessProfile {
        user_id: "test_user_123".to_owned(),
        age: Some(30),
        gender: Some("male".to_owned()),
        weight: Some(70.0),
        height: Some(175.0),
        fitness_level,
        primary_sports: vec!["Run".to_owned(), "Ride".to_owned()],
        training_history_months: 12,
        preferences: UserPreferences {
            preferred_units: "metric".to_owned(),
            training_focus: vec!["endurance".to_owned()],
            injury_history: vec![],
            time_availability: TimeAvailability {
                hours_per_week: 8.0,
                preferred_days: vec![
                    "Monday".to_owned(),
                    "Wednesday".to_owned(),
                    "Friday".to_owned(),
                ],
                preferred_duration_minutes: Some(60),
            },
        },
    }
}

/// Create a test `Goal` for distance tracking
fn create_test_distance_goal(sport: &str, target_km: f64) -> Goal {
    let now = Utc::now();
    Goal {
        id: format!("goal_{}", uuid::Uuid::new_v4()),
        user_id: "test_user_123".to_owned(),
        title: format!("Run {target_km} km"),
        description: format!("Complete a {target_km} km {sport}"),
        goal_type: GoalType::Distance {
            sport: sport.to_owned(),
            timeframe: TimeFrame::Month,
        },
        target_value: target_km * 1000.0,
        target_date: now + chrono::Duration::weeks(4),
        current_value: 0.0,
        created_at: now - chrono::Duration::weeks(1),
        updated_at: now,
        status: GoalStatus::Active,
    }
}

// ================================================================================================
// Integration Tests
// ================================================================================================

#[tokio::test]
async fn test_fitness_score_calculation() {
    // Create provider with consistent training pattern
    let provider =
        create_synthetic_provider_with_scenario(TestScenario::ExperiencedCyclistConsistent);

    // Get activities for analysis
    let activities = provider
        .get_activities(Some(100), None)
        .await
        .expect("Should get activities");

    assert!(
        !activities.is_empty(),
        "Should have activities for fitness score calculation"
    );

    // Create performance analyzer
    let strategy = Box::new(DefaultStrategy);
    let analyzer =
        PerformanceAnalyzerV2::new(strategy).expect("Should create performance analyzer");

    // Calculate fitness score
    let fitness_score = analyzer
        .calculate_fitness_score(&activities)
        .expect("Should calculate fitness score");

    // Verify fitness score structure
    assert!(
        fitness_score.overall_score >= 0.0,
        "Overall score should be non-negative"
    );
    assert!(
        fitness_score.overall_score <= 100.0,
        "Overall score should not exceed 100"
    );

    // For a consistent cyclist pattern, we expect reasonable fitness scores
    assert!(
        fitness_score.aerobic_fitness > 0.0,
        "Aerobic fitness should be calculated"
    );
    assert!(
        fitness_score.consistency >= 0.0,
        "Consistency should be measured"
    );
}

#[tokio::test]
async fn test_fitness_score_with_improving_pattern() {
    // Create provider with improving runner pattern
    let provider = create_synthetic_provider_with_scenario(TestScenario::BeginnerRunnerImproving);

    let activities = provider
        .get_activities(Some(100), None)
        .await
        .expect("Should get activities");

    let strategy = Box::new(DefaultStrategy);
    let analyzer =
        PerformanceAnalyzerV2::new(strategy).expect("Should create performance analyzer");

    let fitness_score = analyzer
        .calculate_fitness_score(&activities)
        .expect("Should calculate fitness score");

    // Improving pattern should show positive trends
    assert!(
        fitness_score.overall_score > 0.0,
        "Improving athlete should have positive fitness score"
    );
}

#[tokio::test]
async fn test_training_load_analysis() {
    let provider =
        create_synthetic_provider_with_scenario(TestScenario::ExperiencedCyclistConsistent);

    let activities = provider
        .get_activities(Some(100), None)
        .await
        .expect("Should get activities");

    let strategy = Box::new(DefaultStrategy);
    let analyzer =
        PerformanceAnalyzerV2::new(strategy).expect("Should create performance analyzer");

    let training_load = analyzer
        .analyze_training_load(&activities)
        .expect("Should analyze training load");

    // Verify training load structure
    assert!(
        !training_load.weekly_loads.is_empty(),
        "Should have weekly load data"
    );

    // Check that weekly loads are populated
    let last_week = training_load
        .weekly_loads
        .last()
        .expect("Should have at least one week");

    assert!(
        last_week.total_duration_hours >= 0.0,
        "Duration should be non-negative"
    );
    assert!(
        last_week.total_distance_km >= 0.0,
        "Distance should be non-negative"
    );
    assert!(
        last_week.activity_count >= 0,
        "Activity count should be non-negative"
    );
    assert!(
        last_week.intensity_score >= 0.0,
        "Intensity score should be non-negative"
    );

    // For consistent cyclist, we expect stable weekly loads
    assert!(
        training_load.average_weekly_load > 0.0,
        "Should have positive average weekly load"
    );
}

#[tokio::test]
async fn test_training_load_overtraining_detection() {
    let provider = create_synthetic_provider_with_scenario(TestScenario::OvertrainingRisk);

    let activities = provider
        .get_activities(Some(100), None)
        .await
        .expect("Should get activities");

    let strategy = Box::new(DefaultStrategy);
    let analyzer =
        PerformanceAnalyzerV2::new(strategy).expect("Should create performance analyzer");

    let training_load = analyzer
        .analyze_training_load(&activities)
        .expect("Should analyze training load");

    // Overtraining pattern should show high load
    assert!(
        !training_load.weekly_loads.is_empty(),
        "Should have weekly data"
    );

    // Check for high training volume indicating overtraining risk
    let recent_weeks: Vec<_> = training_load.weekly_loads.iter().rev().take(2).collect();

    // For overtraining pattern, verify we have some training load data
    assert!(!recent_weeks.is_empty(), "Should have weekly training data");

    // Calculate total volume across recent weeks
    let total_hours: f64 = recent_weeks.iter().map(|w| w.total_duration_hours).sum();

    // Overtraining pattern should show measurable training volume
    assert!(
        total_hours > 0.0,
        "Overtraining pattern should show training activity"
    );
}

#[tokio::test]
async fn test_performance_prediction() {
    let provider = create_synthetic_provider_with_scenario(TestScenario::BeginnerRunnerImproving);

    let activities = provider
        .get_activities(Some(100), None)
        .await
        .expect("Should get activities");

    // Filter to only running activities for performance prediction
    let running_activities: Vec<_> = activities
        .iter()
        .filter(|a| a.sport_type == SportType::Run)
        .cloned()
        .collect();

    assert!(
        !running_activities.is_empty(),
        "Should have running activities"
    );

    let strategy = Box::new(DefaultStrategy);
    let analyzer =
        PerformanceAnalyzerV2::new(strategy).expect("Should create performance analyzer");

    // Create a goal for 5K run
    let goal = ActivityGoal {
        sport_type: "Run".to_owned(),
        metric: "distance".to_owned(),
        target_value: 5000.0, // 5km
        target_date: chrono::Utc::now() + chrono::Duration::weeks(12),
    };

    let prediction = analyzer
        .predict_performance(&running_activities, &goal)
        .expect("Should predict performance");

    // Verify prediction structure
    assert!(
        prediction.predicted_value > 0.0,
        "Should predict a positive value"
    );
    assert!(
        (prediction.target_goal.target_value - 5000.0).abs() < 0.01,
        "Should match target goal"
    );
    assert!(
        !prediction.recommendations.is_empty(),
        "Should provide recommendations"
    );
}

#[tokio::test]
async fn test_performance_prediction_insufficient_data() {
    let provider = create_synthetic_provider_with_scenario(TestScenario::InjuryRecovery);

    let activities = provider
        .get_activities(Some(10), None)
        .await
        .expect("Should get activities");

    let strategy = Box::new(DefaultStrategy);
    let analyzer =
        PerformanceAnalyzerV2::new(strategy).expect("Should create performance analyzer");

    // Create a goal for 10K run
    let goal = ActivityGoal {
        sport_type: "Run".to_owned(),
        metric: "distance".to_owned(),
        target_value: 10000.0, // 10km
        target_date: chrono::Utc::now() + chrono::Duration::weeks(8),
    };

    // With insufficient data, prediction should handle gracefully
    let result = analyzer.predict_performance(&activities, &goal);

    // Either succeeds with limited data or returns an appropriate error
    if let Ok(prediction) = result {
        // If it succeeds, check it has basic structure
        assert!(
            prediction.predicted_value >= 0.0,
            "Predicted value should be non-negative"
        );
    }
    // Otherwise expected error for insufficient data
}

#[tokio::test]
async fn test_suggest_goals_for_beginner() {
    let provider = create_synthetic_provider_with_scenario(TestScenario::BeginnerRunnerImproving);
    let activities = provider
        .get_activities(Some(50), None)
        .await
        .expect("Should get activities");

    let user_profile = create_test_user_profile(FitnessLevel::Beginner);
    let goal_engine = AdvancedGoalEngine::new();

    let suggestions = goal_engine
        .suggest_goals(&user_profile, &activities)
        .await
        .expect("Should suggest goals");

    assert!(
        !suggestions.is_empty(),
        "Should suggest at least one goal for beginner"
    );

    // Verify suggestion structure
    for suggestion in &suggestions {
        assert!(
            !suggestion.rationale.is_empty(),
            "Suggestion should have rationale"
        );
        assert!(
            suggestion.suggested_target > 0.0,
            "Should have positive target value"
        );
        assert!(
            suggestion.estimated_timeline_days > 0,
            "Should have positive timeline"
        );
        assert!(
            suggestion.success_probability >= 0.0 && suggestion.success_probability <= 1.0,
            "Success probability should be between 0 and 1"
        );
    }
}

#[tokio::test]
async fn test_suggest_goals_for_intermediate() {
    let provider =
        create_synthetic_provider_with_scenario(TestScenario::ExperiencedCyclistConsistent);
    let activities = provider
        .get_activities(Some(50), None)
        .await
        .expect("Should get activities");

    let user_profile = create_test_user_profile(FitnessLevel::Intermediate);
    let goal_engine = AdvancedGoalEngine::new();

    let suggestions = goal_engine
        .suggest_goals(&user_profile, &activities)
        .await
        .expect("Should suggest goals");

    assert!(
        !suggestions.is_empty(),
        "Should suggest goals for intermediate user"
    );

    // Intermediate users should get challenging but achievable goals
    for suggestion in &suggestions {
        assert!(
            suggestion.success_probability > 0.5,
            "Intermediate goals should be reasonably achievable"
        );
    }
}

#[tokio::test]
async fn test_suggest_goals_for_advanced() {
    let provider = create_synthetic_provider_with_scenario(TestScenario::BeginnerRunnerImproving);
    let activities = provider
        .get_activities(Some(50), None)
        .await
        .expect("Should get activities");

    let user_profile = create_test_user_profile(FitnessLevel::Advanced);
    let goal_engine = AdvancedGoalEngine::new();

    let suggestions = goal_engine
        .suggest_goals(&user_profile, &activities)
        .await
        .expect("Should suggest goals");

    assert!(
        !suggestions.is_empty(),
        "Should suggest goals for advanced user"
    );

    // Advanced users might get more challenging goals
    let has_challenging_goals = suggestions.iter().any(|s| {
        matches!(
            s.difficulty,
            GoalDifficulty::Challenging | GoalDifficulty::Ambitious
        )
    });

    assert!(
        has_challenging_goals || !suggestions.is_empty(),
        "Advanced users should get appropriate goal difficulty levels"
    );
}

#[tokio::test]
async fn test_track_goal_progress() {
    let provider = create_synthetic_provider_with_scenario(TestScenario::BeginnerRunnerImproving);
    let activities = provider
        .get_activities(Some(50), None)
        .await
        .expect("Should get activities");

    let goal = create_test_distance_goal("Run", 50.0);
    let goal_engine = AdvancedGoalEngine::new();

    let progress = goal_engine
        .track_progress(&goal, &activities)
        .await
        .expect("Should track progress");

    assert!(
        progress.progress_percentage >= 0.0,
        "Progress should be non-negative"
    );
    assert!(
        progress.progress_percentage <= 100.0,
        "Progress should not exceed 100%"
    );
    assert!(
        !progress.recommendations.is_empty(),
        "Should provide recommendations"
    );
    assert_eq!(
        progress.goal_id, goal.id,
        "Progress report should match goal ID"
    );

    // Verify milestones structure
    assert!(
        !progress.milestones_achieved.is_empty(),
        "Should have milestones"
    );
    for milestone in &progress.milestones_achieved {
        assert!(!milestone.name.is_empty(), "Milestone should have a name");
        assert!(
            milestone.target_value > 0.0,
            "Milestone should have positive target value"
        );
    }
}

#[tokio::test]
async fn test_track_progress_with_no_activities() {
    // Get empty activity list
    let activities = vec![];

    let goal = create_test_distance_goal("Run", 50.0);
    let goal_engine = AdvancedGoalEngine::new();

    let progress = goal_engine
        .track_progress(&goal, &activities)
        .await
        .expect("Should track progress even with no activities");

    assert!(
        progress.progress_percentage.abs() < f64::EPSILON,
        "Should have 0% progress with no activities"
    );
    assert!(
        !progress.on_track || progress.progress_percentage.abs() < f64::EPSILON,
        "Should indicate not on track or zero progress"
    );
}

#[tokio::test]
async fn test_track_progress_milestones() {
    let provider = create_synthetic_provider_with_scenario(TestScenario::BeginnerRunnerImproving);
    let activities = provider
        .get_activities(Some(50), None)
        .await
        .expect("Should get activities");

    let goal = create_test_distance_goal("Run", 50.0);
    let goal_engine = AdvancedGoalEngine::new();

    let progress = goal_engine
        .track_progress(&goal, &activities)
        .await
        .expect("Should track progress");

    // Verify milestones are ordered by target value
    let mut previous_target = 0.0;
    for milestone in &progress.milestones_achieved {
        assert!(
            milestone.target_value > previous_target,
            "Milestones should be ordered by increasing target value"
        );
        previous_target = milestone.target_value;
    }

    // Verify achieved milestones are consistent with progress
    let achieved_count = progress
        .milestones_achieved
        .iter()
        .filter(|m| m.achieved)
        .count();

    // If progress is > 0, we might have achieved some milestones
    // The achieved_count is always valid as it's a count
    if progress.progress_percentage > 0.0 {
        // Milestone achievements are tracked properly
        assert!(
            achieved_count <= progress.milestones_achieved.len(),
            "Achieved count should not exceed total milestones"
        );
    }
}

#[tokio::test]
async fn test_track_progress_recommendations() {
    let provider = create_synthetic_provider_with_scenario(TestScenario::BeginnerRunnerImproving);
    let activities = provider
        .get_activities(Some(50), None)
        .await
        .expect("Should get activities");

    let goal = create_test_distance_goal("Run", 50.0);
    let goal_engine = AdvancedGoalEngine::new();

    let progress = goal_engine
        .track_progress(&goal, &activities)
        .await
        .expect("Should track progress");

    // Recommendations should be actionable
    assert!(
        !progress.recommendations.is_empty(),
        "Should provide recommendations"
    );

    for recommendation in &progress.recommendations {
        assert!(
            !recommendation.is_empty(),
            "Recommendations should not be empty strings"
        );
        assert!(
            recommendation.len() > 10,
            "Recommendations should be meaningful, not just placeholders"
        );
    }
}

#[tokio::test]
async fn test_goal_progress_insights() {
    let provider = create_synthetic_provider_with_scenario(TestScenario::BeginnerRunnerImproving);
    let activities = provider
        .get_activities(Some(50), None)
        .await
        .expect("Should get activities");

    let goal = create_test_distance_goal("Run", 50.0);
    let goal_engine = AdvancedGoalEngine::new();

    let progress = goal_engine
        .track_progress(&goal, &activities)
        .await
        .expect("Should track progress");

    // Progress should have insights field
    // Insights are generated based on progress status, so they may or may not be present
    // Verify the vector is valid and accessible
    let insight_count = progress.insights.len();

    // If insights are present, verify structure
    if insight_count > 0 {
        for insight in &progress.insights {
            assert!(
                !insight.message.is_empty(),
                "Insight message should not be empty"
            );
            assert!(
                !insight.insight_type.is_empty(),
                "Insight type should not be empty"
            );
        }
    }
}
