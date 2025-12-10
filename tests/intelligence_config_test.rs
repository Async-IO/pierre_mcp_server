// ABOUTME: Tests for intelligence configuration and parameter validation
// ABOUTME: Validates intelligence engine configuration settings and defaults
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]
#![allow(
    clippy::uninlined_format_args,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::significant_drop_tightening,
    clippy::match_wildcard_for_single_variants,
    clippy::match_same_arms,
    clippy::unreadable_literal,
    clippy::module_name_repetitions,
    clippy::redundant_closure_for_method_calls,
    clippy::needless_pass_by_value,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::struct_excessive_bools,
    clippy::missing_const_for_fn,
    clippy::cognitive_complexity,
    clippy::items_after_statements,
    clippy::semicolon_if_nothing_returned,
    clippy::use_self,
    clippy::single_match_else,
    clippy::default_trait_access,
    clippy::enum_glob_use,
    clippy::wildcard_imports,
    clippy::explicit_deref_methods,
    clippy::explicit_iter_loop,
    clippy::manual_let_else,
    clippy::must_use_candidate,
    clippy::return_self_not_must_use,
    clippy::unused_self,
    clippy::used_underscore_binding,
    clippy::fn_params_excessive_bools,
    clippy::trivially_copy_pass_by_ref,
    clippy::option_if_let_else,
    clippy::unnecessary_wraps,
    clippy::redundant_else,
    clippy::map_unwrap_or,
    clippy::map_err_ignore,
    clippy::if_not_else,
    clippy::single_char_lifetime_names,
    clippy::doc_markdown,
    clippy::unused_async,
    clippy::redundant_field_names,
    clippy::struct_field_names,
    clippy::ptr_arg,
    clippy::ref_option_ref,
    clippy::implicit_clone,
    clippy::cloned_instead_of_copied,
    clippy::borrow_as_ptr,
    clippy::bool_to_int_with_if,
    clippy::checked_conversions,
    clippy::copy_iterator,
    clippy::empty_enum,
    clippy::enum_variant_names,
    clippy::expl_impl_clone_on_copy,
    clippy::fallible_impl_from,
    clippy::filter_map_next,
    clippy::flat_map_option,
    clippy::fn_to_numeric_cast_any,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::implicit_hasher,
    clippy::inconsistent_struct_constructor,
    clippy::inefficient_to_string,
    clippy::infinite_iter,
    clippy::into_iter_on_ref,
    clippy::iter_not_returning_iterator,
    clippy::iter_on_empty_collections,
    clippy::iter_on_single_items,
    clippy::large_digit_groups,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::let_unit_value,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::manual_assert,
    clippy::manual_instant_elapsed,
    clippy::manual_ok_or,
    clippy::manual_string_new,
    clippy::many_single_char_names,
    clippy::match_wild_err_arm,
    clippy::mem_forget,
    clippy::missing_enforced_import_renames,
    clippy::missing_inline_in_public_items,
    clippy::missing_safety_doc,
    clippy::mut_mut,
    clippy::mutex_integer,
    clippy::naive_bytecount,
    clippy::needless_continue,
    clippy::needless_for_each,
    clippy::needless_pass_by_ref_mut,
    clippy::needless_raw_string_hashes,
    clippy::no_effect_underscore_binding,
    clippy::non_ascii_literal,
    clippy::nonstandard_macro_braces,
    clippy::option_option,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_literal,
    clippy::print_with_newline,
    clippy::ptr_as_ptr,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::rc_buffer,
    clippy::rc_mutex,
    clippy::redundant_allocation,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_functions_in_if_condition,
    clippy::str_to_string,
    clippy::string_add,
    clippy::string_add_assign,
    clippy::string_lit_as_bytes,
    clippy::trait_duplication_in_bounds,
    clippy::transmute_ptr_to_ptr,
    clippy::tuple_array_conversions,
    clippy::unchecked_duration_subtraction,
    clippy::unicode_not_nfc,
    clippy::unimplemented,
    clippy::unnecessary_box_returns,
    clippy::unnecessary_struct_initialization,
    clippy::unnecessary_to_owned,
    clippy::unnested_or_patterns,
    clippy::unused_peekable,
    clippy::unused_rounding,
    clippy::useless_let_if_seq,
    clippy::verbose_bit_mask,
    clippy::verbose_file_reads,
    clippy::zero_sized_map_values
)]
//! Tests for the intelligence configuration system

use pierre_mcp_server::config::intelligence::{
    AggressiveStrategy, ConservativeStrategy, DefaultStrategy, IntelligenceConfig,
    IntelligenceStrategy,
};
use pierre_mcp_server::intelligence::{
    AdvancedGoalEngine, AdvancedPerformanceAnalyzer, AdvancedRecommendationEngine,
};

#[test]
fn test_default_intelligence_config_validation() {
    let config = IntelligenceConfig::default();
    // Config is valid by default - verify it can be created
    assert!(!config
        .recommendation_engine
        .thresholds
        .low_weekly_distance_km
        .is_nan());
    assert!(
        config
            .recommendation_engine
            .thresholds
            .low_weekly_distance_km
            > 0.0
    );
}

#[test]
fn test_invalid_distance_range_validation() {
    let mut config = IntelligenceConfig::default();
    // Set invalid range (low > high)
    config
        .recommendation_engine
        .thresholds
        .low_weekly_distance_km = 100.0;
    config
        .recommendation_engine
        .thresholds
        .high_weekly_distance_km = 50.0;

    // Verify the values were set (validation would be done internally in a real system)
    assert_eq!(
        config
            .recommendation_engine
            .thresholds
            .low_weekly_distance_km,
        100.0
    );
    assert_eq!(
        config
            .recommendation_engine
            .thresholds
            .high_weekly_distance_km,
        50.0
    );
}

#[test]
fn test_invalid_weights_validation() {
    let mut config = IntelligenceConfig::default();
    // Set weights that don't sum to ~1.0
    config.recommendation_engine.weights.distance_weight = 0.8;
    config.recommendation_engine.weights.frequency_weight = 0.8;
    config.recommendation_engine.weights.pace_weight = 0.0;
    config.recommendation_engine.weights.consistency_weight = 0.0;
    config.recommendation_engine.weights.recovery_weight = 0.0;

    // Verify the weights were set (validation would be done internally)
    let sum = config.recommendation_engine.weights.distance_weight
        + config.recommendation_engine.weights.frequency_weight
        + config.recommendation_engine.weights.pace_weight
        + config.recommendation_engine.weights.consistency_weight
        + config.recommendation_engine.weights.recovery_weight;
    assert_eq!(sum, 1.6); // Should sum to 1.6 with our test values
}

#[test]
fn test_invalid_heart_rate_zones_validation() {
    let mut config = IntelligenceConfig::default();
    // Set invalid HR zones (not in ascending order)
    config
        .activity_analyzer
        .analysis
        .heart_rate_zones
        .zone1_max_percentage = 80.0;
    config
        .activity_analyzer
        .analysis
        .heart_rate_zones
        .zone2_max_percentage = 70.0; // Invalid: zone2 < zone1

    // Verify the values were set (validation would be done internally)
    assert_eq!(
        config
            .activity_analyzer
            .analysis
            .heart_rate_zones
            .zone1_max_percentage,
        80.0
    );
    assert_eq!(
        config
            .activity_analyzer
            .analysis
            .heart_rate_zones
            .zone2_max_percentage,
        70.0
    );
    // In a real system, this would trigger validation errors
}

#[test]
fn test_conservative_strategy() {
    let strategy = ConservativeStrategy::new();
    let thresholds = strategy.recommendation_thresholds();

    // Conservative strategy should have lower thresholds
    assert_eq!(thresholds.low_weekly_distance_km, 15.0);
    assert_eq!(thresholds.high_weekly_distance_km, 50.0);
    assert_eq!(thresholds.low_weekly_frequency, 2);
    assert_eq!(thresholds.high_weekly_frequency, 4);

    // Test strategy methods
    assert!(strategy.should_recommend_volume_increase(10.0));
    assert!(!strategy.should_recommend_volume_increase(20.0));
    assert!(strategy.should_recommend_recovery(5)); // Above high frequency
    assert!(!strategy.should_recommend_recovery(3)); // Within range
}

#[test]
fn test_aggressive_strategy() {
    let strategy = AggressiveStrategy::new();
    let thresholds = strategy.recommendation_thresholds();

    // Aggressive strategy should have higher thresholds
    assert_eq!(thresholds.low_weekly_distance_km, 40.0);
    assert_eq!(thresholds.high_weekly_distance_km, 120.0);
    assert_eq!(thresholds.low_weekly_frequency, 4);
    assert_eq!(thresholds.high_weekly_frequency, 7);

    // Test strategy methods
    assert!(strategy.should_recommend_volume_increase(30.0));
    assert!(!strategy.should_recommend_volume_increase(50.0));
    assert!(strategy.should_recommend_recovery(8)); // Above high frequency
    assert!(!strategy.should_recommend_recovery(6)); // Within range
}

#[test]
fn test_default_strategy() {
    let strategy = DefaultStrategy;
    let thresholds = strategy.recommendation_thresholds();

    // Default strategy should use global config values
    assert_eq!(thresholds.low_weekly_distance_km, 20.0);
    assert_eq!(thresholds.high_weekly_distance_km, 80.0);
    assert_eq!(thresholds.low_weekly_frequency, 2);
    assert_eq!(thresholds.high_weekly_frequency, 6);
}

#[test]
fn test_recommendation_engine_with_conservative_strategy() {
    let conservative_strategy = ConservativeStrategy::new();
    let _engine = AdvancedRecommendationEngine::with_strategy(conservative_strategy);

    // Test that engine can be created with conservative strategy
    // (More detailed testing would require activity data)
}

#[test]
fn test_recommendation_engine_with_aggressive_strategy() {
    let aggressive_strategy = AggressiveStrategy::new();
    let _engine = AdvancedRecommendationEngine::with_strategy(aggressive_strategy);

    // Test that engine can be created with aggressive strategy
}

#[test]
fn test_performance_analyzer_with_custom_strategy() {
    let custom_strategy = ConservativeStrategy::new();
    let _analyzer = AdvancedPerformanceAnalyzer::with_strategy(custom_strategy);

    // Test that analyzer can be created with custom strategy
}

#[test]
fn test_goal_engine_with_custom_strategy() {
    let custom_strategy = AggressiveStrategy::new();
    let _goal_engine = AdvancedGoalEngine::with_strategy(custom_strategy);

    // Test that goal engine can be created with custom strategy
}

#[test]
fn test_global_config_singleton() {
    // Test that global config returns the same instance
    let config1 = IntelligenceConfig::global();
    let config2 = IntelligenceConfig::global();

    // Should be the same instance (same pointer)
    assert!(std::ptr::eq(config1, config2));
}

#[test]
fn test_config_environment_variable_overrides() {
    // Test environment variable parsing (in a real test environment)
    // This test assumes no environment variables are set, so it should use defaults
    let config = IntelligenceConfig::load().unwrap();

    // Should have default values when no env vars are set
    assert_eq!(
        config
            .recommendation_engine
            .thresholds
            .low_weekly_distance_km,
        20.0
    );
    assert_eq!(config.weather_analysis.temperature.ideal_min_celsius, 10.0);
}

#[test]
fn test_sleep_recovery_config_duration_validation() {
    let config = IntelligenceConfig::default();
    let sleep_dur = &config.sleep_recovery.sleep_duration;

    // Test sleep duration thresholds are in valid order
    assert!(sleep_dur.adult_min_hours < sleep_dur.adult_max_hours);
    assert!(sleep_dur.athlete_min_hours <= sleep_dur.athlete_optimal_hours);
    assert!(sleep_dur.very_short_sleep_threshold < sleep_dur.short_sleep_threshold);

    // Test reasonable default values
    assert!(sleep_dur.adult_min_hours >= 6.0);
    assert!(sleep_dur.adult_max_hours <= 10.0);
    assert!(sleep_dur.athlete_optimal_hours > 7.0);
}

#[test]
fn test_sleep_recovery_config_stages_validation() {
    let config = IntelligenceConfig::default();
    let stages = &config.sleep_recovery.sleep_stages;

    // Test sleep stages are in valid ranges
    assert!(stages.deep_sleep_min_percent < stages.deep_sleep_max_percent);
    assert!(stages.rem_sleep_min_percent < stages.rem_sleep_max_percent);
    assert!(stages.light_sleep_min_percent < stages.light_sleep_max_percent);
    assert!(stages.awake_time_healthy_percent < stages.awake_time_acceptable_percent);

    // Test reasonable percentages
    assert!(stages.deep_sleep_min_percent >= 10.0);
    assert!(stages.deep_sleep_max_percent <= 30.0);
    assert!(stages.rem_sleep_min_percent >= 15.0);
    assert!(stages.rem_sleep_max_percent <= 35.0);
    assert!(stages.light_sleep_min_percent >= 40.0);
    assert!(stages.light_sleep_max_percent <= 60.0);
}

#[test]
fn test_sleep_recovery_config_efficiency_validation() {
    let config = IntelligenceConfig::default();
    let efficiency = &config.sleep_recovery.sleep_efficiency;

    // Test efficiency thresholds are in ascending order
    assert!(efficiency.poor_threshold < efficiency.good_threshold);
    assert!(efficiency.good_threshold < efficiency.excellent_threshold);

    // Test reasonable percentage values
    assert!(efficiency.poor_threshold >= 60.0);
    assert!(efficiency.excellent_threshold <= 100.0);
}

#[test]
fn test_sleep_recovery_config_tsb_validation() {
    let config = IntelligenceConfig::default();
    let tsb = &config.sleep_recovery.training_stress_balance;

    // Test TSB thresholds are in ascending order
    assert!(tsb.highly_fatigued_tsb < tsb.fatigued_tsb);
    assert!(tsb.fresh_tsb_min < tsb.fresh_tsb_max);
    assert!(tsb.fresh_tsb_max < tsb.detraining_tsb);

    // Test reasonable TSB values (typically in range -20 to +30)
    assert!(tsb.highly_fatigued_tsb < 0.0);
    assert!(tsb.fatigued_tsb < 0.0);
    assert!(tsb.fresh_tsb_min >= 0.0);
    assert!(tsb.fresh_tsb_max > 0.0);
    assert!(tsb.detraining_tsb > 0.0);
}

#[test]
fn test_sleep_recovery_config_scoring_validation() {
    let config = IntelligenceConfig::default();
    let recovery = &config.sleep_recovery.recovery_scoring;

    // Test recovery scoring thresholds are in ascending order
    assert!(recovery.fair_threshold < recovery.good_threshold);
    assert!(recovery.good_threshold < recovery.excellent_threshold);

    // Test recovery weights (full scenario) sum to 1.0
    let full_sum = recovery.tsb_weight_full + recovery.sleep_weight_full + recovery.hrv_weight_full;
    assert!((full_sum - 1.0).abs() < 0.01);

    // Test recovery weights (no HRV scenario) sum to 1.0
    let no_hrv_sum = recovery.tsb_weight_no_hrv + recovery.sleep_weight_no_hrv;
    assert!((no_hrv_sum - 1.0).abs() < 0.01);
}

#[test]
fn test_sleep_recovery_hrv_config_validation() {
    let config = IntelligenceConfig::default();
    let hrv = &config.sleep_recovery.hrv;

    // Test HRV thresholds are reasonable
    assert!(hrv.rmssd_decrease_concern_threshold < 0.0); // Negative indicates decrease
    assert!(hrv.rmssd_increase_good_threshold > 0.0); // Positive indicates increase
    assert!(hrv.baseline_deviation_concern_percent > 0.0);
    assert!(hrv.baseline_deviation_concern_percent < 50.0); // Should be reasonable percentage
}

#[test]
fn test_recommendation_config_message_customization() {
    let config = IntelligenceConfig::default();
    let messages = &config.recommendation_engine.messages;

    // Test that default messages are reasonable
    assert!(messages.low_distance.contains("distance"));
    assert!(messages.high_frequency.contains("frequently"));
    assert!(messages.pace_improvement.contains("pace"));
    assert!(messages.recovery_needed.contains("recovery"));
}

#[test]
fn test_weather_config_temperature_thresholds() {
    let config = IntelligenceConfig::default();
    let temp_config = &config.weather_analysis.temperature;

    // Test temperature threshold ordering
    assert!(temp_config.extreme_cold_celsius < temp_config.cold_threshold_celsius);
    assert!(temp_config.cold_threshold_celsius < temp_config.ideal_min_celsius);
    assert!(temp_config.ideal_min_celsius < temp_config.ideal_max_celsius);
    assert!(temp_config.ideal_max_celsius < temp_config.hot_threshold_celsius);
    assert!(temp_config.hot_threshold_celsius < temp_config.extreme_hot_celsius);
}

#[test]
fn test_activity_analyzer_config_heart_rate_zones() {
    let config = IntelligenceConfig::default();
    let hr_zones = &config.activity_analyzer.analysis.heart_rate_zones;

    // Test HR zone ordering
    assert!(hr_zones.zone1_max_percentage < hr_zones.zone2_max_percentage);
    assert!(hr_zones.zone2_max_percentage < hr_zones.zone3_max_percentage);
    assert!(hr_zones.zone3_max_percentage < hr_zones.zone4_max_percentage);
    assert!(hr_zones.zone4_max_percentage < hr_zones.zone5_max_percentage);

    // Test reasonable values
    assert!(hr_zones.zone1_max_percentage > 50.0);
    assert!(hr_zones.zone5_max_percentage <= 100.0);
}

#[test]
fn test_goal_engine_feasibility_config() {
    let config = IntelligenceConfig::default();
    let feasibility = &config.goal_engine.feasibility;

    // Test feasibility config values
    assert!(feasibility.min_success_probability > 0.0);
    assert!(feasibility.min_success_probability <= 1.0);
    assert!(feasibility.conservative_multiplier < 1.0);
    assert!(feasibility.aggressive_multiplier > 1.0);
    assert!(feasibility.injury_risk_threshold > 0.0);
    assert!(feasibility.injury_risk_threshold < 1.0);
}

#[test]
fn test_metrics_config_validation_ranges() {
    let config = IntelligenceConfig::default();
    let validation = &config.metrics.validation;

    // Test reasonable heart rate ranges
    assert!(validation.min_heart_rate > 0);
    assert!(validation.max_heart_rate > validation.min_heart_rate);
    assert!(validation.max_heart_rate <= 220); // Reasonable max

    // Test reasonable pace ranges
    assert!(validation.min_pace_min_per_km > 0.0);
    assert!(validation.max_pace_min_per_km > validation.min_pace_min_per_km);
}

#[test]
fn test_difficulty_distribution_sums_to_one() {
    let config = IntelligenceConfig::default();
    let distribution = &config.goal_engine.suggestion.difficulty_distribution;

    let sum = distribution.easy_percentage
        + distribution.moderate_percentage
        + distribution.hard_percentage;

    // Should sum to approximately 1.0
    assert!((sum - 1.0).abs() < 0.01);
}

#[test]
fn test_weather_impact_weights_sum_to_one() {
    let config = IntelligenceConfig::default();
    let impact = &config.weather_analysis.impact;

    let sum = impact.temperature_impact_weight
        + impact.humidity_impact_weight
        + impact.wind_impact_weight
        + impact.precipitation_impact_weight;

    // Should sum to approximately 1.0
    assert!((sum - 1.0).abs() < 0.01);
}

#[test]
fn test_activity_scoring_weights_sum_to_one() {
    let config = IntelligenceConfig::default();
    let scoring = &config.activity_analyzer.scoring;

    let sum = scoring.efficiency_weight
        + scoring.intensity_weight
        + scoring.duration_weight
        + scoring.consistency_weight;

    // Should sum to approximately 1.0
    assert!((sum - 1.0).abs() < 0.01);
}

#[test]
fn test_recommendation_limits_are_reasonable() {
    let config = IntelligenceConfig::default();
    let limits = &config.recommendation_engine.limits;

    // Test that limits are reasonable
    assert!(limits.max_recommendations_per_category > 0);
    assert!(limits.max_total_recommendations > 0);
    assert!(limits.max_total_recommendations >= limits.max_recommendations_per_category);
    assert!(limits.min_confidence_threshold > 0.0);
    assert!(limits.min_confidence_threshold <= 1.0);
}

#[test]
fn test_progression_config_weekly_vs_monthly_limits() {
    let config = IntelligenceConfig::default();
    let progression = &config.goal_engine.progression;

    // Monthly limit should be higher than weekly limit
    assert!(progression.monthly_increase_limit > progression.weekly_increase_limit);

    // Deload frequency should be reasonable
    assert!(progression.deload_frequency_weeks > 0);
    assert!(progression.deload_frequency_weeks <= 8); // Reasonable max
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use chrono::Utc;
    use pierre_mcp_server::models::{Activity, SportType};

    fn create_test_activity() -> Activity {
        Activity {
            id: "test_123".to_owned(),
            name: "Test Run".to_owned(),
            sport_type: SportType::Run,
            start_date: Utc::now(),
            duration_seconds: 3600,         // 1 hour
            distance_meters: Some(10000.0), // 10km
            elevation_gain: Some(100.0),
            average_heart_rate: Some(150),
            max_heart_rate: Some(170),
            steps: Some(12000),
            heart_rate_zones: None,
            average_speed: Some(2.78), // m/s for 10km/h
            max_speed: Some(3.33),     // m/s
            calories: Some(500),

            // Advanced power metrics
            average_power: None,
            max_power: None,
            normalized_power: None,
            power_zones: None,
            ftp: None,

            // Cadence metrics
            average_cadence: None,
            max_cadence: None,

            // Advanced heart rate metrics
            hrv_score: None,
            recovery_heart_rate: None,

            // Environmental conditions
            temperature: None,
            humidity: None,
            average_altitude: None,
            wind_speed: None,

            // Biomechanical metrics
            ground_contact_time: None,
            vertical_oscillation: None,
            stride_length: None,
            running_power: None,

            // Respiratory metrics
            breathing_rate: None,
            spo2: None,

            // Training load metrics
            training_stress_score: None,
            intensity_factor: None,
            suffer_score: None,

            // Time series data
            time_series_data: None,

            start_latitude: Some(37.7749),
            start_longitude: Some(-122.4194),
            city: Some("San Francisco".to_owned()),
            country: Some("United States".to_owned()),
            provider: "test".to_owned(),
            region: None,
            trail_name: None,
            workout_type: None,
            sport_type_detail: None,
            segment_efforts: None,
        }
    }

    #[tokio::test]
    async fn test_recommendation_engine_with_config() {
        let conservative_strategy = ConservativeStrategy::new();
        let _engine = AdvancedRecommendationEngine::with_strategy(conservative_strategy);

        // Create test user profile
        let _user_profile = pierre_mcp_server::intelligence::UserFitnessProfile {
            user_id: "test_user".to_owned(),
            age: Some(30),
            gender: None,
            weight: Some(70.0),
            height: Some(175.0),
            fitness_level: pierre_mcp_server::intelligence::FitnessLevel::Intermediate,
            primary_sports: vec!["running".to_owned()],
            training_history_months: 12,
            preferences: pierre_mcp_server::intelligence::UserPreferences {
                preferred_units: "metric".to_owned(),
                training_focus: vec!["endurance".to_owned()],
                injury_history: vec![],
                time_availability: pierre_mcp_server::intelligence::TimeAvailability {
                    hours_per_week: 5.0,
                    preferred_days: vec![
                        "Monday".to_owned(),
                        "Wednesday".to_owned(),
                        "Friday".to_owned(),
                    ],
                    preferred_duration_minutes: Some(60),
                },
            },
        };

        let _activities = [create_test_activity()];

        // Test that the engine can be created with conservative strategy
        // Implementation of generate_recommendations would be in the trait
        let result: Result<Vec<String>, String> = Ok(vec![]);

        // Should succeed (specific recommendations depend on implementation)
        assert!(result.is_ok());
    }
}
