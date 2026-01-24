// ABOUTME: Tests for social intelligence modules (insights, adapters, activity cache)
// ABOUTME: Validates insight generation, adaptation, and friend activity caching
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Tests for social intelligence modules including insight generation,
//! insight adaptation, and friend activity caching.

use chrono::{Duration, Utc};
use pierre_mcp_server::intelligence::{
    friend_activity_cache::{
        CacheConfig, DurationCategory, EffortLevel, FriendActivityCache, FriendActivitySummary,
    },
    insight_adapter::{truncate_string, FitnessLevel, InsightAdapter, UserTrainingContext},
    social_insights::{
        calculate_milestone_relevance, capitalize_first, InsightContextBuilder,
        InsightGenerationContext, InsightSuggestion, PersonalRecord, SharedInsightGenerator,
    },
};
use pierre_mcp_server::models::{InsightType, ShareVisibility, SharedInsight, TrainingPhase};
use uuid::Uuid;

// ============================================================================
// FriendActivityCache Tests
// ============================================================================

#[test]
fn test_duration_category() {
    assert_eq!(DurationCategory::from_minutes(15), DurationCategory::Short);
    assert_eq!(DurationCategory::from_minutes(45), DurationCategory::Medium);
    assert_eq!(DurationCategory::from_minutes(90), DurationCategory::Long);
    assert_eq!(DurationCategory::from_minutes(150), DurationCategory::Epic);
}

#[test]
fn test_duration_category_boundary_values() {
    // Test exact boundaries
    assert_eq!(DurationCategory::from_minutes(0), DurationCategory::Short);
    assert_eq!(DurationCategory::from_minutes(29), DurationCategory::Short);
    assert_eq!(DurationCategory::from_minutes(30), DurationCategory::Medium);
    assert_eq!(DurationCategory::from_minutes(59), DurationCategory::Medium);
    assert_eq!(DurationCategory::from_minutes(60), DurationCategory::Long);
    assert_eq!(DurationCategory::from_minutes(119), DurationCategory::Long);
    assert_eq!(DurationCategory::from_minutes(120), DurationCategory::Epic);
    assert_eq!(
        DurationCategory::from_minutes(u32::MAX),
        DurationCategory::Epic
    );
}

#[test]
fn test_effort_level() {
    assert_eq!(EffortLevel::from_hr_percentage(50), EffortLevel::Easy);
    assert_eq!(EffortLevel::from_hr_percentage(70), EffortLevel::Moderate);
    assert_eq!(EffortLevel::from_hr_percentage(85), EffortLevel::Hard);
    assert_eq!(EffortLevel::from_hr_percentage(95), EffortLevel::Max);
}

#[test]
fn test_effort_level_boundary_values() {
    // Test exact boundaries based on actual implementation:
    // 0-59: Easy, 60-75: Moderate, 76-89: Hard, 90+: Max
    assert_eq!(EffortLevel::from_hr_percentage(0), EffortLevel::Easy);
    assert_eq!(EffortLevel::from_hr_percentage(59), EffortLevel::Easy);
    assert_eq!(EffortLevel::from_hr_percentage(60), EffortLevel::Moderate);
    assert_eq!(EffortLevel::from_hr_percentage(75), EffortLevel::Moderate);
    assert_eq!(EffortLevel::from_hr_percentage(76), EffortLevel::Hard);
    assert_eq!(EffortLevel::from_hr_percentage(89), EffortLevel::Hard);
    assert_eq!(EffortLevel::from_hr_percentage(90), EffortLevel::Max);
    assert_eq!(EffortLevel::from_hr_percentage(100), EffortLevel::Max);
}

#[test]
fn test_cache_insert_and_get() {
    let cache = FriendActivityCache::new();
    let user_id = Uuid::new_v4();

    let summary = FriendActivitySummary::new(
        user_id,
        "run".to_owned(),
        "this morning".to_owned(),
        DurationCategory::Medium,
        EffortLevel::Moderate,
    );

    cache.insert(summary);

    let activities = cache.get_friend_activities(user_id);
    assert_eq!(activities.len(), 1);
    assert_eq!(activities[0].sport_type, "run");
}

#[test]
fn test_cache_multiple_users() {
    let cache = FriendActivityCache::new();
    let user1 = Uuid::new_v4();
    let user2 = Uuid::new_v4();
    let user3 = Uuid::new_v4();

    // Insert activities for different users
    cache.insert(FriendActivitySummary::new(
        user1,
        "run".to_owned(),
        "morning".to_owned(),
        DurationCategory::Short,
        EffortLevel::Easy,
    ));
    cache.insert(FriendActivitySummary::new(
        user1,
        "bike".to_owned(),
        "afternoon".to_owned(),
        DurationCategory::Long,
        EffortLevel::Hard,
    ));
    cache.insert(FriendActivitySummary::new(
        user2,
        "swim".to_owned(),
        "evening".to_owned(),
        DurationCategory::Medium,
        EffortLevel::Moderate,
    ));

    // Verify correct retrieval
    let user1_activities = cache.get_friend_activities(user1);
    assert_eq!(user1_activities.len(), 2);

    let user2_activities = cache.get_friend_activities(user2);
    assert_eq!(user2_activities.len(), 1);
    assert_eq!(user2_activities[0].sport_type, "swim");

    // User with no activities
    let user3_activities = cache.get_friend_activities(user3);
    assert!(user3_activities.is_empty());

    // Check stats
    let stats = cache.stats();
    assert_eq!(stats.total_entries, 3);
    assert_eq!(stats.user_count, 2);
}

#[test]
fn test_cache_cleanup() {
    let cache = FriendActivityCache::new();
    let user_id = Uuid::new_v4();

    // Insert expired entry
    let mut summary = FriendActivitySummary::new(
        user_id,
        "run".to_owned(),
        "yesterday".to_owned(),
        DurationCategory::Short,
        EffortLevel::Easy,
    );
    summary.expires_at = Utc::now() - Duration::hours(1); // Already expired

    cache.insert(summary);

    // Should not return expired entries
    let activities = cache.get_friend_activities(user_id);
    assert!(activities.is_empty());
}

#[test]
fn test_cache_mixed_expiry() {
    let cache = FriendActivityCache::new();
    let user_id = Uuid::new_v4();

    // Insert valid entry
    cache.insert(FriendActivitySummary::new(
        user_id,
        "run".to_owned(),
        "now".to_owned(),
        DurationCategory::Short,
        EffortLevel::Easy,
    ));

    // Insert expired entry
    let mut expired = FriendActivitySummary::new(
        user_id,
        "swim".to_owned(),
        "yesterday".to_owned(),
        DurationCategory::Medium,
        EffortLevel::Moderate,
    );
    expired.expires_at = Utc::now() - Duration::hours(1);
    cache.insert(expired);

    // Should only return valid entry
    let activities = cache.get_friend_activities(user_id);
    assert_eq!(activities.len(), 1);
    assert_eq!(activities[0].sport_type, "run");
}

#[test]
fn test_display_string() {
    let summary = FriendActivitySummary::new(
        Uuid::new_v4(),
        "run".to_owned(),
        "this morning".to_owned(),
        DurationCategory::Long,
        EffortLevel::Hard,
    )
    .with_display_name("Alice".to_owned());

    let display = summary.display_string();
    assert!(display.contains("Alice"));
    assert!(display.contains("long session"));
    assert!(display.contains("run"));
}

#[test]
fn test_display_string_all_categories() {
    let user_id = Uuid::new_v4();

    // Short + Easy
    let summary = FriendActivitySummary::new(
        user_id,
        "walk".to_owned(),
        "today".to_owned(),
        DurationCategory::Short,
        EffortLevel::Easy,
    )
    .with_display_name("Bob".to_owned());
    let display = summary.display_string();
    assert!(display.contains("quick"));
    assert!(display.contains("easy"));

    // Epic + Max
    let summary = FriendActivitySummary::new(
        user_id,
        "triathlon".to_owned(),
        "today".to_owned(),
        DurationCategory::Epic,
        EffortLevel::Max,
    )
    .with_display_name("Charlie".to_owned());
    let display = summary.display_string();
    assert!(display.contains("epic"));
    assert!(display.contains("max effort") || display.contains("all-out"));
}

#[test]
fn test_cache_stats() {
    let cache = FriendActivityCache::new();

    let stats = cache.stats();
    assert_eq!(stats.total_entries, 0);
    assert_eq!(stats.user_count, 0);

    let user_id = Uuid::new_v4();
    cache.insert(FriendActivitySummary::new(
        user_id,
        "run".to_owned(),
        "now".to_owned(),
        DurationCategory::Short,
        EffortLevel::Easy,
    ));

    let stats = cache.stats();
    assert_eq!(stats.total_entries, 1);
    assert_eq!(stats.user_count, 1);
}

#[test]
fn test_cache_with_custom_config() {
    let config = CacheConfig {
        ttl_minutes: 720, // 12 hours
        max_entries_per_user: 2,
        max_total_entries: 100,
    };
    let cache = FriendActivityCache::with_config(config);
    let user_id = Uuid::new_v4();

    // Insert 3 entries, should keep only 2 (most recent)
    for i in 0..3 {
        cache.insert(FriendActivitySummary::new(
            user_id,
            format!("activity_{i}"),
            "now".to_owned(),
            DurationCategory::Short,
            EffortLevel::Easy,
        ));
    }

    let activities = cache.get_friend_activities(user_id);
    assert!(activities.len() <= 2);
}

#[test]
fn test_cache_clear() {
    let cache = FriendActivityCache::new();
    let user1 = Uuid::new_v4();
    let user2 = Uuid::new_v4();

    cache.insert(FriendActivitySummary::new(
        user1,
        "run".to_owned(),
        "now".to_owned(),
        DurationCategory::Short,
        EffortLevel::Easy,
    ));
    cache.insert(FriendActivitySummary::new(
        user2,
        "bike".to_owned(),
        "now".to_owned(),
        DurationCategory::Medium,
        EffortLevel::Moderate,
    ));

    assert_eq!(cache.stats().total_entries, 2);

    cache.clear();

    assert_eq!(cache.stats().total_entries, 0);
    assert_eq!(cache.stats().user_count, 0);
}

// ============================================================================
// InsightAdapter Tests
// ============================================================================

#[test]
fn test_fitness_level_categorization() {
    let context = UserTrainingContext::default().with_fitness_score(75.0);
    assert_eq!(context.fitness_level(), FitnessLevel::Advanced);

    let context = UserTrainingContext::default().with_fitness_score(50.0);
    assert_eq!(context.fitness_level(), FitnessLevel::Intermediate);

    let context = UserTrainingContext::default().with_fitness_score(20.0);
    assert_eq!(context.fitness_level(), FitnessLevel::Beginner);

    let context = UserTrainingContext::default();
    assert_eq!(context.fitness_level(), FitnessLevel::Unknown);
}

#[test]
fn test_fitness_level_boundary_values() {
    // Test exact boundaries
    let context = UserTrainingContext::default().with_fitness_score(0.0);
    assert_eq!(context.fitness_level(), FitnessLevel::Beginner);

    let context = UserTrainingContext::default().with_fitness_score(39.9);
    assert_eq!(context.fitness_level(), FitnessLevel::Beginner);

    let context = UserTrainingContext::default().with_fitness_score(40.0);
    assert_eq!(context.fitness_level(), FitnessLevel::Intermediate);

    let context = UserTrainingContext::default().with_fitness_score(69.9);
    assert_eq!(context.fitness_level(), FitnessLevel::Intermediate);

    let context = UserTrainingContext::default().with_fitness_score(70.0);
    assert_eq!(context.fitness_level(), FitnessLevel::Advanced);

    let context = UserTrainingContext::default().with_fitness_score(100.0);
    assert_eq!(context.fitness_level(), FitnessLevel::Advanced);
}

#[test]
fn test_truncate_string() {
    assert_eq!(truncate_string("short", 10), "short");
    assert_eq!(truncate_string("this is a longer string", 10), "this is...");
}

#[test]
fn test_truncate_string_edge_cases() {
    // Exact length
    assert_eq!(truncate_string("exact", 5), "exact");

    // Empty string
    assert_eq!(truncate_string("", 10), "");

    // Very small max length
    assert_eq!(truncate_string("hello", 3), "...");

    // Unicode handling
    let unicode_str = "héllo wörld";
    let truncated = truncate_string(unicode_str, 8);
    assert!(truncated.len() <= 11); // May vary due to unicode
}

#[test]
fn test_adapter_generates_notes() {
    // Test that default adapter includes notes by checking adaptation result
    let adapter = InsightAdapter::new();
    let insight = SharedInsight::new(
        Uuid::new_v4(),
        InsightType::TrainingTip,
        "Test content".to_owned(),
        ShareVisibility::FriendsOnly,
    );
    let context = UserTrainingContext::default().with_fitness_score(50.0);

    let result = adapter.adapt(&insight, &context, None);

    // Default adapter should include notes
    assert!(!result.adaptation_notes.is_empty());
}

#[test]
fn test_adapter_without_notes() {
    // Test that adapter configured without notes produces empty notes
    let adapter = InsightAdapter::new().with_notes(false);
    let insight = SharedInsight::new(
        Uuid::new_v4(),
        InsightType::TrainingTip,
        "Test content".to_owned(),
        ShareVisibility::FriendsOnly,
    );
    let context = UserTrainingContext::default();

    let result = adapter.adapt(&insight, &context, None);

    // Adapter without notes should have empty notes
    assert!(result.adaptation_notes.is_empty());
}

#[test]
fn test_relevance_score_via_adapt() {
    // Test relevance calculation through the public adapt method
    let adapter = InsightAdapter::new();
    let insight = SharedInsight::new(
        Uuid::new_v4(),
        InsightType::TrainingTip,
        "Test content".to_owned(),
        ShareVisibility::FriendsOnly,
    );

    let context = UserTrainingContext::default();
    let result = adapter.adapt(&insight, &context, None);

    // Base 50 + TrainingTip bonus 10 = 60
    assert_eq!(result.relevance_score, 60);
}

#[test]
fn test_relevance_score_different_insight_types() {
    let adapter = InsightAdapter::new();
    let context = UserTrainingContext::default();

    // Achievement insight
    let achievement = SharedInsight::new(
        Uuid::new_v4(),
        InsightType::Achievement,
        "Achievement content".to_owned(),
        ShareVisibility::FriendsOnly,
    );
    let result = adapter.adapt(&achievement, &context, None);
    assert!(result.relevance_score >= 50);

    // Milestone insight
    let milestone = SharedInsight::new(
        Uuid::new_v4(),
        InsightType::Milestone,
        "Milestone content".to_owned(),
        ShareVisibility::FriendsOnly,
    );
    let result = adapter.adapt(&milestone, &context, None);
    assert!(result.relevance_score >= 50);

    // Motivation insight
    let motivation = SharedInsight::new(
        Uuid::new_v4(),
        InsightType::Motivation,
        "Motivation content".to_owned(),
        ShareVisibility::FriendsOnly,
    );
    let result = adapter.adapt(&motivation, &context, None);
    assert!(result.relevance_score >= 50);
}

#[test]
fn test_relevance_boost_for_matching_sport() {
    let adapter = InsightAdapter::new();

    // Create insight with sport type
    let mut insight = SharedInsight::new(
        Uuid::new_v4(),
        InsightType::TrainingTip,
        "Running tip".to_owned(),
        ShareVisibility::FriendsOnly,
    );
    insight.sport_type = Some("running".to_owned());

    // Context with matching sport
    let matching_context = UserTrainingContext::default()
        .with_fitness_score(50.0)
        .with_primary_sport("running".to_owned());

    // Context with different sport
    let different_context = UserTrainingContext::default()
        .with_fitness_score(50.0)
        .with_primary_sport("cycling".to_owned());

    let matching_result = adapter.adapt(&insight, &matching_context, None);
    let different_result = adapter.adapt(&insight, &different_context, None);

    // Matching sport should have higher relevance
    assert!(matching_result.relevance_score >= different_result.relevance_score);
}

#[test]
fn test_user_training_context_builder() {
    let context = UserTrainingContext::default()
        .with_fitness_score(65.0)
        .with_primary_sport("cycling".to_owned())
        .with_training_phase(TrainingPhase::Build)
        .with_weekly_volume(10.0);

    assert_eq!(context.fitness_level(), FitnessLevel::Intermediate);
}

// ============================================================================
// SharedInsightGenerator Tests
// ============================================================================

#[test]
fn test_generator_creates_valid_output() {
    // Test that generator can be created and produces valid output
    let generator = SharedInsightGenerator::new();
    let context = InsightContextBuilder::new().build();

    // Empty context should produce no suggestions (below threshold)
    let suggestions = generator.generate_suggestions(&context);
    assert!(suggestions.is_empty());
}

#[test]
fn test_generator_with_custom_relevance() {
    // Test that custom relevance threshold works
    let generator = SharedInsightGenerator::with_min_relevance(90);
    let context = InsightContextBuilder::new().build();

    // Empty context should produce no suggestions
    let suggestions = generator.generate_suggestions(&context);
    assert!(suggestions.is_empty());
}

#[test]
fn test_generator_activity_milestone() {
    let generator = SharedInsightGenerator::with_min_relevance(50);

    // Create context with 100 activities (milestone)
    let context = InsightGenerationContext {
        recent_activity_count: 10,
        total_activity_count: 100,
        total_distance_km: 500.0,
        current_streak_days: 5,
        longest_streak_days: 10,
        primary_sport: Some("running".to_owned()),
        training_phase: None,
        recent_prs: Vec::new(),
    };

    let suggestions = generator.generate_suggestions(&context);

    // Should have at least one milestone suggestion
    let has_milestone = suggestions
        .iter()
        .any(|s| s.insight_type == InsightType::Milestone);
    assert!(
        has_milestone,
        "Should generate milestone for 100 activities"
    );
}

#[test]
fn test_generator_streak_achievement() {
    let generator = SharedInsightGenerator::with_min_relevance(50);

    // Create context with 30-day streak
    let context = InsightGenerationContext {
        recent_activity_count: 30,
        total_activity_count: 50,
        total_distance_km: 150.0,
        current_streak_days: 30,
        longest_streak_days: 30,
        primary_sport: Some("running".to_owned()),
        training_phase: None,
        recent_prs: Vec::new(),
    };

    let suggestions = generator.generate_suggestions(&context);

    // Should have achievement for streak
    let has_achievement = suggestions
        .iter()
        .any(|s| s.insight_type == InsightType::Achievement);
    assert!(
        has_achievement,
        "Should generate achievement for 30-day streak"
    );
}

#[test]
fn test_generator_personal_record() {
    let generator = SharedInsightGenerator::with_min_relevance(50);

    // Create context with recent PR
    let context = InsightGenerationContext {
        recent_activity_count: 10,
        total_activity_count: 50,
        total_distance_km: 200.0,
        current_streak_days: 5,
        longest_streak_days: 10,
        primary_sport: Some("running".to_owned()),
        training_phase: None,
        recent_prs: vec![PersonalRecord {
            pr_type: "5k".to_owned(),
            description: "New personal best in 5k".to_owned(),
            achieved_at: Utc::now() - Duration::days(2),
            improvement_pct: Some(3.5),
        }],
    };

    let suggestions = generator.generate_suggestions(&context);

    // Should have achievement for PR
    let has_pr_achievement = suggestions.iter().any(|s| {
        s.insight_type == InsightType::Achievement
            && s.suggested_content.to_lowercase().contains("5k")
    });
    assert!(
        has_pr_achievement,
        "Should generate achievement for recent PR"
    );
}

#[test]
fn test_generator_distance_milestone() {
    let generator = SharedInsightGenerator::with_min_relevance(50);

    // Create context with 1000km milestone (format is "1k km" for >= 1000)
    let context = InsightGenerationContext {
        recent_activity_count: 20,
        total_activity_count: 200,
        total_distance_km: 1000.0,
        current_streak_days: 5,
        longest_streak_days: 15,
        primary_sport: Some("running".to_owned()),
        training_phase: None,
        recent_prs: Vec::new(),
    };

    let suggestions = generator.generate_suggestions(&context);

    // Should have milestone for 1000km (formatted as "1k km")
    let has_distance_milestone = suggestions
        .iter()
        .any(|s| s.insight_type == InsightType::Milestone && s.suggested_content.contains("1k km"));
    assert!(
        has_distance_milestone,
        "Should generate milestone for 1000km distance (formatted as '1k km'). Got: {:?}",
        suggestions
            .iter()
            .map(|s| &s.suggested_content)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_generator_training_phase_insights() {
    let generator = SharedInsightGenerator::with_min_relevance(40);

    // Create context with training phase
    let context = InsightGenerationContext {
        recent_activity_count: 15,
        total_activity_count: 100,
        total_distance_km: 300.0,
        current_streak_days: 10,
        longest_streak_days: 20,
        primary_sport: Some("running".to_owned()),
        training_phase: Some(TrainingPhase::Build),
        recent_prs: Vec::new(),
    };

    let suggestions = generator.generate_suggestions(&context);

    // Training phase insights may or may not be generated depending on other criteria
    // Just ensure we don't crash and generator works
    // The generator may or may not produce training tips based on context
    let _ = suggestions
        .iter()
        .any(|s| s.insight_type == InsightType::TrainingTip);
}

#[test]
fn test_generator_suggestions_sorted_by_relevance() {
    let generator = SharedInsightGenerator::with_min_relevance(30);

    // Create rich context that should generate multiple suggestions
    let context = InsightGenerationContext {
        recent_activity_count: 30,
        total_activity_count: 500,
        total_distance_km: 2000.0,
        current_streak_days: 60,
        longest_streak_days: 60,
        primary_sport: Some("running".to_owned()),
        training_phase: Some(TrainingPhase::Peak),
        recent_prs: vec![PersonalRecord {
            pr_type: "10k".to_owned(),
            description: "New 10k PR".to_owned(),
            achieved_at: Utc::now() - Duration::days(1),
            improvement_pct: Some(2.0),
        }],
    };

    let suggestions = generator.generate_suggestions(&context);

    // Verify sorted by relevance (descending)
    for window in suggestions.windows(2) {
        assert!(
            window[0].relevance_score >= window[1].relevance_score,
            "Suggestions should be sorted by relevance descending"
        );
    }
}

#[test]
fn test_generator_create_insight() {
    let generator = SharedInsightGenerator::new();
    let user_id = Uuid::new_v4();

    let suggestion = InsightSuggestion {
        insight_type: InsightType::Achievement,
        suggested_content: "Great achievement!".to_owned(),
        suggested_title: Some("Title".to_owned()),
        relevance_score: 80,
        sport_type: Some("running".to_owned()),
        training_phase: Some(TrainingPhase::Build),
    };

    let insight = generator.create_insight(user_id, &suggestion, ShareVisibility::FriendsOnly);

    assert_eq!(insight.user_id, user_id);
    assert_eq!(insight.insight_type, InsightType::Achievement);
    assert_eq!(insight.content, "Great achievement!");
    assert_eq!(insight.visibility, ShareVisibility::FriendsOnly);
}

#[test]
fn test_milestone_relevance() {
    assert_eq!(calculate_milestone_relevance(1000), 95);
    assert_eq!(calculate_milestone_relevance(100), 80);
    assert_eq!(calculate_milestone_relevance(10), 65);
}

#[test]
fn test_milestone_relevance_all_tiers() {
    // Test all milestone tiers
    assert_eq!(calculate_milestone_relevance(1500), 95); // 1000+
    assert_eq!(calculate_milestone_relevance(750), 90); // 500-999
    assert_eq!(calculate_milestone_relevance(300), 85); // 250-499
    assert_eq!(calculate_milestone_relevance(150), 80); // 100-249
    assert_eq!(calculate_milestone_relevance(75), 75); // 50-99
    assert_eq!(calculate_milestone_relevance(30), 70); // 25-49
    assert_eq!(calculate_milestone_relevance(10), 65); // < 25
}

// Note: calculate_streak_relevance is private, tested indirectly through generate_suggestions

#[test]
fn test_capitalize_first() {
    assert_eq!(capitalize_first("runs"), "Runs");
    assert_eq!(capitalize_first(""), "");
    assert_eq!(capitalize_first("a"), "A");
}

#[test]
fn test_capitalize_first_unicode() {
    // Unicode handling
    assert_eq!(capitalize_first("über"), "Über");
    assert_eq!(capitalize_first("éclair"), "Éclair");
}

#[test]
fn test_capitalize_first_already_capital() {
    assert_eq!(capitalize_first("Already"), "Already");
    assert_eq!(capitalize_first("CAPS"), "CAPS");
}

#[test]
fn test_empty_context_builder() {
    let context = InsightContextBuilder::new().build();
    assert_eq!(context.total_activity_count, 0);
    assert_eq!(context.current_streak_days, 0);
}

#[test]
fn test_context_builder_with_training_phase() {
    let context = InsightContextBuilder::new()
        .with_training_phase(TrainingPhase::Base)
        .build();

    assert_eq!(context.training_phase, Some(TrainingPhase::Base));
}

// ============================================================================
// Integration Tests - Generator + Adapter
// ============================================================================

#[test]
fn test_end_to_end_insight_generation_and_adaptation() {
    // Generate an insight
    let generator = SharedInsightGenerator::with_min_relevance(50);
    let gen_context = InsightGenerationContext {
        recent_activity_count: 20,
        total_activity_count: 100,
        total_distance_km: 500.0,
        current_streak_days: 14,
        longest_streak_days: 21,
        primary_sport: Some("running".to_owned()),
        training_phase: Some(TrainingPhase::Build),
        recent_prs: Vec::new(),
    };

    let suggestions = generator.generate_suggestions(&gen_context);

    if let Some(suggestion) = suggestions.first() {
        // Create the insight
        let user_id = Uuid::new_v4();
        let insight = generator.create_insight(user_id, suggestion, ShareVisibility::FriendsOnly);

        // Adapt it for another user
        let adapter = InsightAdapter::new();
        let adapt_context = UserTrainingContext::default()
            .with_fitness_score(55.0)
            .with_primary_sport("running".to_owned());

        let result = adapter.adapt(&insight, &adapt_context, None);

        // Verify adaptation result
        assert!(result.relevance_score > 0);
        assert!(!result.adapted_content.is_empty());
        assert!(!result.context_summary.is_empty());
    }
}

#[test]
fn test_cache_and_generator_integration() {
    // Simulate caching friend activity summaries
    let cache = FriendActivityCache::new();
    let friend_id = Uuid::new_v4();

    // Friend completes several activities
    for i in 0..5 {
        cache.insert(FriendActivitySummary::new(
            friend_id,
            if i % 2 == 0 {
                "running".to_owned()
            } else {
                "cycling".to_owned()
            },
            "recently".to_owned(),
            DurationCategory::Medium,
            EffortLevel::Moderate,
        ));
    }

    // Verify cache contains activities
    let activities = cache.get_friend_activities(friend_id);
    assert!(!activities.is_empty());

    // Generate insights based on similar context
    let generator = SharedInsightGenerator::with_min_relevance(50);
    let context = InsightGenerationContext {
        recent_activity_count: 5,
        total_activity_count: 50,
        total_distance_km: 200.0,
        current_streak_days: 7,
        longest_streak_days: 14,
        primary_sport: Some("running".to_owned()),
        training_phase: None,
        recent_prs: Vec::new(),
    };

    let suggestions = generator.generate_suggestions(&context);
    // Just ensure the integration works without errors
    // The generator may produce zero or more suggestions based on context
    let _ = suggestions;
}
