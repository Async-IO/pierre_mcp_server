// ABOUTME: Simple integration tests for basic system functionality
// ABOUTME: Tests core integration points and basic workflows
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
use anyhow::Result;
use chrono::{Duration, Utc};
use pierre_mcp_server::{
    errors::ErrorCode,
    intelligence::ActivityAnalyzer,
    models::{Activity, SportType},
};
use uuid::Uuid;

#[tokio::test]
async fn test_intelligence_analysis_integration() -> Result<()> {
    let analyzer = ActivityAnalyzer::new();

    // Create a test activity using the correct structure
    let activity = Activity {
        id: format!("test_{}", Uuid::new_v4().simple()),
        name: "Integration Test Run".to_owned(),
        sport_type: SportType::Run,
        start_date: Utc::now() - Duration::hours(1),
        duration_seconds: 3600,         // 1 hour
        distance_meters: Some(10000.0), // 10km
        elevation_gain: Some(100.0),
        average_heart_rate: Some(150),
        max_heart_rate: Some(180),
        average_speed: Some(2.78), // ~10 km/h
        max_speed: Some(3.33),
        calories: Some(400),
        steps: Some(12000),
        heart_rate_zones: None,

        // Advanced power metrics (all None for basic test)
        average_power: None,
        max_power: None,
        normalized_power: None,
        power_zones: None,
        ftp: None,
        average_cadence: None,
        max_cadence: None,
        hrv_score: None,
        recovery_heart_rate: None,
        temperature: None,
        humidity: None,
        average_altitude: None,
        wind_speed: None,
        ground_contact_time: None,
        vertical_oscillation: None,
        stride_length: None,
        running_power: None,
        breathing_rate: None,
        spo2: None,
        training_stress_score: None,
        intensity_factor: None,
        suffer_score: None,
        time_series_data: None,

        start_latitude: Some(45.5017), // Montreal
        start_longitude: Some(-73.5673),
        city: Some("Montreal".to_owned()),
        region: Some("Quebec".to_owned()),
        country: Some("Canada".to_owned()),
        trail_name: Some("Test Trail".to_owned()),
        workout_type: None,
        sport_type_detail: None,
        segment_efforts: None,
        provider: "test".to_owned(),
    };

    // Analyze the activity
    let analysis = analyzer.analyze_activity(&activity, None)?;

    // Verify analysis results
    assert!(!analysis.summary.is_empty());
    assert!(!analysis.key_insights.is_empty());
    assert!(
        analysis
            .performance_indicators
            .relative_effort
            .unwrap_or(0.0)
            > 0.0
    );

    Ok(())
}

#[tokio::test]
async fn test_error_code_mappings() -> Result<()> {
    // Test that error codes map to correct HTTP statuses
    assert_eq!(ErrorCode::AuthRequired.http_status(), 401);
    assert_eq!(ErrorCode::AuthInvalid.http_status(), 401);
    assert_eq!(ErrorCode::PermissionDenied.http_status(), 403);
    assert_eq!(ErrorCode::ResourceNotFound.http_status(), 404);
    assert_eq!(ErrorCode::RateLimitExceeded.http_status(), 429);
    assert_eq!(ErrorCode::InternalError.http_status(), 500);

    Ok(())
}

#[tokio::test]
async fn test_activity_model_creation() -> Result<()> {
    // Test that we can create activities for different sports
    let sports = [
        SportType::Run,
        SportType::Ride,
        SportType::Swim,
        SportType::Hike,
    ];

    for sport in sports {
        let activity = Activity {
            id: format!("sport_test_{sport:?}"),
            name: format!("Test {:?} Activity", sport),
            sport_type: sport.clone(),
            start_date: Utc::now(),
            duration_seconds: 1800, // 30 minutes
            distance_meters: Some(5000.0),
            elevation_gain: Some(50.0),
            average_heart_rate: Some(140),
            max_heart_rate: Some(160),
            average_speed: Some(3.0),
            max_speed: Some(4.0),
            calories: Some(200),
            steps: Some(10000),
            heart_rate_zones: None,

            // All advanced metrics as None
            average_power: None,
            max_power: None,
            normalized_power: None,
            power_zones: None,
            ftp: None,
            average_cadence: None,
            max_cadence: None,
            hrv_score: None,
            recovery_heart_rate: None,
            temperature: None,
            humidity: None,
            average_altitude: None,
            wind_speed: None,
            ground_contact_time: None,
            vertical_oscillation: None,
            stride_length: None,
            running_power: None,
            breathing_rate: None,
            spo2: None,
            training_stress_score: None,
            intensity_factor: None,
            suffer_score: None,
            time_series_data: None,

            start_latitude: None,
            start_longitude: None,
            city: None,
            region: None,
            country: None,
            trail_name: None,
            workout_type: None,
            sport_type_detail: None,
            segment_efforts: None,
            provider: "test".to_owned(),
        };

        assert_eq!(activity.sport_type, sport);
        assert!(activity.duration_seconds > 0);
    }

    Ok(())
}

#[tokio::test]
async fn test_concurrent_analysis() -> Result<()> {
    let _analyzer = ActivityAnalyzer::new();

    // Create multiple activities and analyze them concurrently
    let mut handles = Vec::new();

    for i in 0..5 {
        let handle = tokio::spawn(async move {
            let activity = Activity {
                id: format!("concurrent_test_{i}"),
                name: format!("Concurrent Test {i}"),
                sport_type: SportType::Run,
                start_date: Utc::now(),
                duration_seconds: 3600 + (i as u64 * 300),
                distance_meters: Some(f64::from(i).mul_add(1000.0, 5000.0)),
                elevation_gain: Some(50.0),
                average_heart_rate: Some(150),
                max_heart_rate: Some(170),
                average_speed: Some(3.0),
                max_speed: Some(4.0),
                calories: Some(300),
                steps: Some(8000 + (i as u32 * 1000)),
                heart_rate_zones: None,

                // All advanced metrics as None
                average_power: None,
                max_power: None,
                normalized_power: None,
                power_zones: None,
                ftp: None,
                average_cadence: None,
                max_cadence: None,
                hrv_score: None,
                recovery_heart_rate: None,
                temperature: None,
                humidity: None,
                average_altitude: None,
                wind_speed: None,
                ground_contact_time: None,
                vertical_oscillation: None,
                stride_length: None,
                running_power: None,
                breathing_rate: None,
                spo2: None,
                training_stress_score: None,
                intensity_factor: None,
                suffer_score: None,
                time_series_data: None,

                start_latitude: None,
                start_longitude: None,
                city: None,
                region: None,
                country: None,
                trail_name: None,
                workout_type: None,
                sport_type_detail: None,
                segment_efforts: None,
                provider: "test".to_owned(),
            };

            let analyzer_local = ActivityAnalyzer::new();
            analyzer_local.analyze_activity(&activity, None)
        });

        handles.push(handle);
    }

    // Wait for all analyses to complete
    for handle in handles {
        let result = handle.await?;
        assert!(result.is_ok(), "Concurrent analysis should succeed");

        let analysis = result.unwrap();
        assert!(!analysis.summary.is_empty());
    }

    Ok(())
}
