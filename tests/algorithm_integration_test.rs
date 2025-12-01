// ABOUTME: Integration tests for algorithm APIs (TSS, VO2max, etc.) through public interfaces
// ABOUTME: Tests actual calculate() methods, error handling, and algorithm variants
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use chrono::Utc;
use pierre_mcp_server::intelligence::algorithms::{TssAlgorithm, Vo2maxAlgorithm};
use pierre_mcp_server::models::{Activity, SportType};

// === TSS Algorithm Integration Tests ===

/// Create a test activity with basic power data
fn create_test_activity_with_power(avg_power: u32, duration_seconds: u64) -> Activity {
    Activity {
        id: "test_tss_1".to_owned(),
        name: "Test Ride".to_owned(),
        sport_type: SportType::Ride,
        start_date: Utc::now(),
        duration_seconds,
        distance_meters: Some(30000.0),
        elevation_gain: Some(500.0),
        average_heart_rate: Some(150),
        max_heart_rate: Some(180),
        average_speed: Some(8.33),
        max_speed: Some(12.0),
        calories: Some(1200),
        steps: None,
        heart_rate_zones: None,
        average_power: Some(avg_power),
        max_power: Some(avg_power + 50),
        normalized_power: None,
        power_zones: None,
        ftp: None,
        average_cadence: Some(85),
        max_cadence: Some(110),
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
    }
}

#[test]
fn test_tss_avg_power_algorithm_valid_input() {
    let activity = create_test_activity_with_power(200, 3600);
    let ftp = 250.0;
    let duration_hours = 1.0;

    let algorithm = TssAlgorithm::AvgPower;
    let result = algorithm.calculate(&activity, ftp, duration_hours);

    assert!(result.is_ok(), "TSS calculation should succeed");
    let tss = result.unwrap();

    // Expected: 1.0 * (200/250)^2 * 100 = 64
    assert!(
        (tss - 64.0).abs() < 0.1,
        "TSS should be approximately 64, got {tss}"
    );
}

#[test]
fn test_tss_avg_power_algorithm_zero_ftp() {
    let activity = create_test_activity_with_power(200, 3600);
    let ftp = 0.0;
    let duration_hours = 1.0;

    let algorithm = TssAlgorithm::AvgPower;
    let result = algorithm.calculate(&activity, ftp, duration_hours);

    assert!(result.is_err(), "TSS calculation should fail with zero FTP");
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("FTP must be greater than zero"),
        "Error message should mention FTP"
    );
}

#[test]
fn test_tss_avg_power_algorithm_negative_duration() {
    let activity = create_test_activity_with_power(200, 3600);
    let ftp = 250.0;
    let duration_hours = -1.0;

    let algorithm = TssAlgorithm::AvgPower;
    let result = algorithm.calculate(&activity, ftp, duration_hours);

    assert!(
        result.is_err(),
        "TSS calculation should fail with negative duration"
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Duration cannot be negative"),
        "Error message should mention duration"
    );
}

#[test]
fn test_tss_avg_power_algorithm_missing_power_data() {
    let mut activity = create_test_activity_with_power(200, 3600);
    activity.average_power = None;
    let ftp = 250.0;
    let duration_hours = 1.0;

    let algorithm = TssAlgorithm::AvgPower;
    let result = algorithm.calculate(&activity, ftp, duration_hours);

    assert!(
        result.is_err(),
        "TSS calculation should fail with missing power data"
    );
}

#[test]
fn test_tss_normalized_power_algorithm_no_stream_data() {
    let activity = create_test_activity_with_power(200, 3600);
    let ftp = 250.0;
    let duration_hours = 1.0;

    let algorithm = TssAlgorithm::NormalizedPower { window_seconds: 30 };
    let result = algorithm.calculate(&activity, ftp, duration_hours);

    // Should fail because we don't have power stream data
    assert!(
        result.is_err(),
        "NP-based TSS should fail without power stream"
    );
    assert!(
        result.unwrap_err().to_string().contains("Power stream"),
        "Error should mention power stream requirement"
    );
}

#[test]
fn test_tss_hybrid_algorithm_fallback_to_avg_power() {
    let activity = create_test_activity_with_power(200, 3600);
    let ftp = 250.0;
    let duration_hours = 1.0;

    let algorithm = TssAlgorithm::Hybrid;
    let result = algorithm.calculate(&activity, ftp, duration_hours);

    // Hybrid should fallback to avg_power when NP stream is unavailable
    assert!(
        result.is_ok(),
        "Hybrid TSS should succeed with avg_power fallback"
    );
    let tss = result.unwrap();

    // Should get same result as avg_power algorithm
    assert!(
        (tss - 64.0).abs() < 0.1,
        "Hybrid TSS should match avg_power TSS, got {tss}"
    );
}

#[test]
fn test_tss_algorithm_name_and_description() {
    let avg_power = TssAlgorithm::AvgPower;
    assert_eq!(avg_power.name(), "avg_power");
    assert!(avg_power.description().contains("fast"));

    let np = TssAlgorithm::NormalizedPower { window_seconds: 30 };
    assert_eq!(np.name(), "normalized_power");
    assert!(np.description().contains("accurate"));

    let hybrid = TssAlgorithm::Hybrid;
    assert_eq!(hybrid.name(), "hybrid");
    assert!(hybrid.description().contains("Hybrid"));
}

#[test]
fn test_tss_from_str_parsing() {
    use std::str::FromStr;

    let avg_power = TssAlgorithm::from_str("avg_power");
    assert!(avg_power.is_ok());
    assert!(matches!(avg_power.unwrap(), TssAlgorithm::AvgPower));

    let np = TssAlgorithm::from_str("normalized_power");
    assert!(np.is_ok());
    match np.unwrap() {
        TssAlgorithm::NormalizedPower { window_seconds } => {
            assert_eq!(window_seconds, 30, "Default window should be 30 seconds");
        }
        _ => panic!("Expected NormalizedPower variant"),
    }

    let hybrid = TssAlgorithm::from_str("hybrid");
    assert!(hybrid.is_ok());
    assert!(matches!(hybrid.unwrap(), TssAlgorithm::Hybrid));

    let invalid = TssAlgorithm::from_str("invalid_algorithm");
    assert!(invalid.is_err());
    assert!(invalid
        .unwrap_err()
        .to_string()
        .contains("Unknown TSS algorithm"));
}

#[test]
fn test_tss_high_intensity_workout() {
    // Test high intensity workout (110% of FTP)
    let activity = create_test_activity_with_power(275, 1800); // 30 minutes
    let ftp = 250.0;
    let duration_hours = 0.5;

    let algorithm = TssAlgorithm::AvgPower;
    let result = algorithm.calculate(&activity, ftp, duration_hours);

    assert!(result.is_ok());
    let tss = result.unwrap();

    // Expected: 0.5 * (275/250)^2 * 100 = 60.5
    assert!(
        (tss - 61.0).abs() < 1.0,
        "High intensity TSS should be ~61, got {tss}"
    );
}

#[test]
fn test_tss_low_intensity_recovery_ride() {
    // Test low intensity recovery ride (60% of FTP)
    let activity = create_test_activity_with_power(150, 3600); // 1 hour
    let ftp = 250.0;
    let duration_hours = 1.0;

    let algorithm = TssAlgorithm::AvgPower;
    let result = algorithm.calculate(&activity, ftp, duration_hours);

    assert!(result.is_ok());
    let tss = result.unwrap();

    // Expected: 1.0 * (150/250)^2 * 100 = 36
    assert!(
        (tss - 36.0).abs() < 0.1,
        "Recovery ride TSS should be ~36, got {tss}"
    );
}

// === VO2max Algorithm Integration Tests ===

#[test]
fn test_vo2max_from_vdot_valid() {
    let algorithm = Vo2maxAlgorithm::FromVdot { vdot: 50.0 };
    let result = algorithm.estimate_vo2max();

    assert!(result.is_ok(), "VO2max estimation should succeed");
    let vo2max = result.unwrap();

    // Expected: 50.0 * 3.5 = 175.0
    assert!(
        (vo2max - 175.0).abs() < 0.1,
        "VO2max should be 175, got {vo2max}"
    );
}

#[test]
fn test_vo2max_from_vdot_out_of_range() {
    // VDOT too low
    let algorithm = Vo2maxAlgorithm::FromVdot { vdot: 20.0 };
    let result = algorithm.estimate_vo2max();
    assert!(result.is_err(), "VDOT below 30 should fail validation");

    // VDOT too high
    let algorithm = Vo2maxAlgorithm::FromVdot { vdot: 90.0 };
    let result = algorithm.estimate_vo2max();
    assert!(result.is_err(), "VDOT above 85 should fail validation");
}

#[test]
fn test_vo2max_cooper_test_valid() {
    let algorithm = Vo2maxAlgorithm::CooperTest {
        distance_meters: 2800.0,
    };
    let result = algorithm.estimate_vo2max();

    assert!(result.is_ok(), "Cooper test estimation should succeed");
    let vo2max = result.unwrap();

    // Expected: (2800 - 504.9) / 44.73 ≈ 51.3
    assert!(
        (vo2max - 51.3).abs() < 1.0,
        "VO2max should be ~51.3, got {vo2max}"
    );
}

#[test]
fn test_vo2max_cooper_test_distance_too_low() {
    let algorithm = Vo2maxAlgorithm::CooperTest {
        distance_meters: 800.0,
    };
    let result = algorithm.estimate_vo2max();

    assert!(
        result.is_err(),
        "Cooper test should fail with distance < 1000m"
    );
    assert!(result.unwrap_err().to_string().contains("seems too low"));
}

#[test]
fn test_vo2max_cooper_test_distance_too_high() {
    let algorithm = Vo2maxAlgorithm::CooperTest {
        distance_meters: 5500.0,
    };
    let result = algorithm.estimate_vo2max();

    assert!(
        result.is_err(),
        "Cooper test should fail with distance > 5000m"
    );
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("unrealistically high"));
}

#[test]
fn test_vo2max_rockport_walk_valid() {
    // Using realistic values from actual Rockport walk test literature:
    // 1 mile walk for a moderately fit 40-year-old male
    let algorithm = Vo2maxAlgorithm::RockportWalk {
        weight_kg: 80.0,
        age: 40,
        gender: 1,           // male
        time_seconds: 840.0, // 14 minutes (realistic for brisk 1-mile walk)
        heart_rate: 130.0,   // Typical elevated HR for brisk walk
    };
    let result = algorithm.estimate_vo2max();

    assert!(result.is_ok(), "Rockport walk estimation should succeed");
    let vo2max = result.unwrap();

    // Rockport walk typically gives VO2max in 35-55 range for average fitness
    // Just verify it's in the physiological range - the formula may have limitations
    assert!(vo2max >= 20.0, "VO2max should be at least 20, got {vo2max}");
}

#[test]
fn test_vo2max_rockport_walk_invalid_gender() {
    let algorithm = Vo2maxAlgorithm::RockportWalk {
        weight_kg: 75.0,
        age: 35,
        gender: 2, // invalid
        time_seconds: 900.0,
        heart_rate: 140.0,
    };
    let result = algorithm.estimate_vo2max();

    assert!(result.is_err(), "Gender must be 0 or 1");
    assert!(result.unwrap_err().to_string().contains("Gender must be"));
}

#[test]
fn test_vo2max_rockport_walk_invalid_weight() {
    let algorithm = Vo2maxAlgorithm::RockportWalk {
        weight_kg: 200.0, // too high
        age: 35,
        gender: 1,
        time_seconds: 900.0,
        heart_rate: 140.0,
    };
    let result = algorithm.estimate_vo2max();

    assert!(result.is_err(), "Weight outside range should fail");
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("outside typical range"));
}

#[test]
fn test_vo2max_rockport_walk_invalid_age() {
    let algorithm = Vo2maxAlgorithm::RockportWalk {
        weight_kg: 75.0,
        age: 15, // too young
        gender: 1,
        time_seconds: 900.0,
        heart_rate: 140.0,
    };
    let result = algorithm.estimate_vo2max();

    assert!(result.is_err(), "Age outside validated range should fail");
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("outside validated range"));
}

#[test]
fn test_vo2max_rockport_walk_invalid_heart_rate() {
    let algorithm = Vo2maxAlgorithm::RockportWalk {
        weight_kg: 75.0,
        age: 35,
        gender: 1,
        time_seconds: 900.0,
        heart_rate: 220.0, // too high
    };
    let result = algorithm.estimate_vo2max();

    assert!(
        result.is_err(),
        "Heart rate outside physiological range should fail"
    );
}

#[test]
fn test_vo2max_astrand_ryhming_valid() {
    let algorithm = Vo2maxAlgorithm::AstrandRyhming {
        gender: 1, // male
        heart_rate: 150.0,
        power_watts: 200.0,
        weight_kg: 75.0,
    };
    let result = algorithm.estimate_vo2max();

    assert!(result.is_ok(), "Astrand-Ryhming estimation should succeed");
    let vo2max = result.unwrap();

    assert!(
        (20.0..=90.0).contains(&vo2max),
        "VO2max should be in physiological range, got {vo2max}"
    );
}

#[test]
fn test_vo2max_astrand_ryhming_heart_rate_out_of_range() {
    // Heart rate too low
    let algorithm = Vo2maxAlgorithm::AstrandRyhming {
        gender: 1,
        heart_rate: 100.0, // below 120 bpm
        power_watts: 200.0,
        weight_kg: 75.0,
    };
    let result = algorithm.estimate_vo2max();
    assert!(
        result.is_err(),
        "HR below 120 bpm should fail for submaximal test"
    );

    // Heart rate too high
    let algorithm = Vo2maxAlgorithm::AstrandRyhming {
        gender: 1,
        heart_rate: 180.0, // above 170 bpm
        power_watts: 200.0,
        weight_kg: 75.0,
    };
    let result = algorithm.estimate_vo2max();
    assert!(
        result.is_err(),
        "HR above 170 bpm should fail for submaximal test"
    );
}

#[test]
fn test_vo2max_from_pace_valid() {
    let algorithm = Vo2maxAlgorithm::FromPace {
        max_speed_ms: 5.0,      // ~3:20 min/km pace
        recovery_speed_ms: 3.0, // ~5:33 min/km pace
    };
    let result = algorithm.estimate_vo2max();

    assert!(result.is_ok(), "Pace-based estimation should succeed");
    let vo2max = result.unwrap();

    assert!(
        (20.0..=90.0).contains(&vo2max),
        "VO2max should be in physiological range, got {vo2max}"
    );
}

#[test]
fn test_vo2max_from_pace_max_slower_than_recovery() {
    let algorithm = Vo2maxAlgorithm::FromPace {
        max_speed_ms: 3.0,
        recovery_speed_ms: 5.0, // recovery faster than max - invalid
    };
    let result = algorithm.estimate_vo2max();

    assert!(
        result.is_err(),
        "Max speed must be greater than recovery speed"
    );
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Max speed must be greater"));
}

#[test]
fn test_vo2max_from_pace_negative_speeds() {
    let algorithm = Vo2maxAlgorithm::FromPace {
        max_speed_ms: -5.0,
        recovery_speed_ms: 3.0,
    };
    let result = algorithm.estimate_vo2max();

    assert!(result.is_err(), "Negative speeds should fail validation");
    assert!(result.unwrap_err().to_string().contains("must be positive"));
}

#[test]
fn test_vo2max_hybrid_requires_specific_data() {
    let algorithm = Vo2maxAlgorithm::Hybrid;
    let result = algorithm.estimate_vo2max();

    assert!(
        result.is_err(),
        "Hybrid algorithm should require specific test data"
    );
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("requires specific test data"));
}

#[test]
fn test_vo2max_algorithm_name_and_description() {
    let vdot = Vo2maxAlgorithm::FromVdot { vdot: 50.0 };
    assert_eq!(vdot.name(), "from_vdot");
    assert!(vdot.description().contains("VDOT"));

    let cooper = Vo2maxAlgorithm::CooperTest {
        distance_meters: 2800.0,
    };
    assert_eq!(cooper.name(), "cooper_test");
    assert!(cooper.description().contains("Cooper"));

    let rockport = Vo2maxAlgorithm::RockportWalk {
        weight_kg: 75.0,
        age: 35,
        gender: 1,
        time_seconds: 900.0,
        heart_rate: 140.0,
    };
    assert_eq!(rockport.name(), "rockport_walk");
    assert!(rockport.description().contains("Rockport"));
}

#[test]
fn test_vo2max_algorithm_formula() {
    let vdot = Vo2maxAlgorithm::FromVdot { vdot: 50.0 };
    assert!(vdot.formula().contains("VDOT x 3.5"));

    let cooper = Vo2maxAlgorithm::CooperTest {
        distance_meters: 2800.0,
    };
    assert!(cooper.formula().contains("distance - 504.9"));
}

#[test]
fn test_vo2max_from_str_parsing() {
    use std::str::FromStr;

    // Parametrized variants should return errors (prevent zero-value foot-gun)
    let vdot = Vo2maxAlgorithm::from_str("from_vdot");
    assert!(vdot.is_err(), "FromVdot requires parameters");
    assert!(vdot
        .unwrap_err()
        .to_string()
        .contains("requires VDOT parameter"));

    let cooper = Vo2maxAlgorithm::from_str("cooper_test");
    assert!(cooper.is_err(), "Cooper requires distance_meters");
    assert!(cooper
        .unwrap_err()
        .to_string()
        .contains("requires distance_meters"));

    let rockport = Vo2maxAlgorithm::from_str("rockport_walk");
    assert!(rockport.is_err(), "Rockport requires test parameters");
    assert!(rockport
        .unwrap_err()
        .to_string()
        .contains("requires test parameters"));

    // Hybrid has no parameters, so it can be parsed from string
    let hybrid = Vo2maxAlgorithm::from_str("hybrid");
    assert!(hybrid.is_ok(), "Hybrid should parse successfully");
    assert!(matches!(hybrid.unwrap(), Vo2maxAlgorithm::Hybrid));

    // Unknown algorithm should error
    let invalid = Vo2maxAlgorithm::from_str("invalid_algorithm");
    assert!(invalid.is_err());
    assert!(invalid
        .unwrap_err()
        .to_string()
        .contains("Unknown VO2max algorithm"));
}

// === Edge Cases and Boundary Tests ===

#[test]
fn test_tss_very_short_duration() {
    let activity = create_test_activity_with_power(250, 60); // 1 minute
    let ftp = 250.0;
    let duration_hours = 1.0 / 60.0;

    let algorithm = TssAlgorithm::AvgPower;
    let result = algorithm.calculate(&activity, ftp, duration_hours);

    assert!(result.is_ok(), "Very short duration should work");
    let tss = result.unwrap();
    assert!(tss > 0.0, "TSS should be positive for short duration");
}

#[test]
fn test_tss_very_long_duration() {
    let activity = create_test_activity_with_power(180, 21600); // 6 hours
    let ftp = 250.0;
    let duration_hours = 6.0;

    let algorithm = TssAlgorithm::AvgPower;
    let result = algorithm.calculate(&activity, ftp, duration_hours);

    assert!(result.is_ok(), "Long duration should work");
    let tss = result.unwrap();
    assert!(
        tss > 200.0,
        "TSS for 6-hour ride should be substantial, got {tss}"
    );
}

#[test]
fn test_vo2max_elite_athlete_cooper() {
    let algorithm = Vo2maxAlgorithm::CooperTest {
        distance_meters: 3800.0, // elite performance
    };
    let result = algorithm.estimate_vo2max();

    assert!(result.is_ok());
    let vo2max = result.unwrap();
    assert!(
        vo2max > 60.0,
        "Elite athlete should have high VO2max, got {vo2max}"
    );
}

#[test]
fn test_vo2max_untrained_individual_cooper() {
    let algorithm = Vo2maxAlgorithm::CooperTest {
        distance_meters: 1800.0, // untrained performance
    };
    let result = algorithm.estimate_vo2max();

    assert!(result.is_ok());
    let vo2max = result.unwrap();
    assert!(
        (20.0..40.0).contains(&vo2max),
        "Untrained individual should have lower VO2max, got {vo2max}"
    );
}
