// ABOUTME: Unit tests for pattern_detection module
// ABOUTME: Tests pattern detection functionality with comprehensive test coverage
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use chrono::{Duration, Utc};
use pierre_mcp_server::intelligence::{PatternDetector, RiskLevel};
use pierre_mcp_server::models::Activity;

fn create_test_activity(days_ago: i64, distance_km: f64, avg_hr: Option<u32>) -> Activity {
    let date = Utc::now() - Duration::days(days_ago);
    Activity {
        id: format!("test_{}", date.timestamp()),
        name: "Test Activity".to_string(),
        sport_type: pierre_mcp_server::models::SportType::Run,
        start_date: date,
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        duration_seconds: (distance_km * 300.0) as u64, // ~5 min/km pace
        distance_meters: Some(distance_km * 1000.0),
        average_heart_rate: avg_hr,
        elevation_gain: None,
        max_heart_rate: None,
        average_speed: None,
        max_speed: None,
        calories: None,
        steps: None,
        heart_rate_zones: None,
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
        provider: "test".to_string(),
    }
}

#[test]
fn test_weekly_schedule_detection() {
    // Create consistent Monday/Wednesday/Friday pattern
    let mut activities = Vec::new();
    for week in 0..4 {
        activities.push(create_test_activity(week * 7, 10.0, Some(140))); // Monday
        activities.push(create_test_activity(week * 7 + 2, 8.0, Some(135))); // Wednesday
        activities.push(create_test_activity(week * 7 + 4, 12.0, Some(145)));
        // Friday
    }

    let pattern = PatternDetector::detect_weekly_schedule(&activities);

    // With 3 equally distributed days, consistency score ~= 33 (1/3² + 1/3² + 1/3²) * 100
    assert!(
        pattern.consistency_score > 30.0,
        "Expected consistency > 30, got {}",
        pattern.consistency_score
    );
    assert!(pattern.avg_activities_per_week > 2.5);
}

#[test]
fn test_hard_easy_pattern_detection() {
    // Create alternating hard/easy pattern
    let mut activities = Vec::new();
    for i in 0..10 {
        let is_hard = i % 2 == 0;
        let hr = if is_hard { 170 } else { 130 };
        activities.push(create_test_activity(i, 10.0, Some(hr)));
    }

    let pattern = PatternDetector::detect_hard_easy_pattern(&activities);

    assert!(pattern.pattern_detected);
    assert!(pattern.hard_percentage > 40.0 && pattern.hard_percentage < 60.0);
}

#[test]
fn test_hr_drift_detection() {
    // Create activities with increasing HR (fatigue signal)
    let mut activities = Vec::new();
    for i in 0..12 {
        let hr = 140 + (i * 2); // HR increases over time
        #[allow(clippy::cast_sign_loss)]
        activities.push(create_test_activity(i64::from(i), 10.0, Some(hr as u32)));
    }

    let signals = PatternDetector::detect_overtraining_signals(&activities);

    assert!(signals.hr_drift_detected);
    assert!(signals.risk_level != RiskLevel::Low);
}

#[test]
fn test_volume_spike_detection() {
    // Week 0: 30km (3 weeks ago, days 18-20)
    // Note: create_test_activity uses days_ago, so larger values = older dates
    let mut activities = vec![
        create_test_activity(20, 10.0, Some(140)),
        create_test_activity(19, 10.0, Some(140)),
        create_test_activity(18, 10.0, Some(140)),
        // Week 2: 60km (this week, days 2-7 - creating 100% spike)
        create_test_activity(7, 10.0, Some(140)),
        create_test_activity(6, 10.0, Some(140)),
        create_test_activity(5, 10.0, Some(140)),
        create_test_activity(4, 10.0, Some(140)),
        create_test_activity(3, 10.0, Some(140)),
        create_test_activity(2, 10.0, Some(140)),
    ];

    // Sort by date to ensure chronological order (oldest first)
    activities.sort_by_key(|a| a.start_date);

    let pattern = PatternDetector::detect_volume_progression(&activities);

    // Debug output if test fails
    if !pattern.volume_spikes_detected {
        eprintln!("Weekly volumes: {:?}", pattern.weekly_volumes);
        eprintln!("Week numbers: {:?}", pattern.week_numbers);
        eprintln!("Spike weeks: {:?}", pattern.spike_weeks);
    }

    assert!(
        pattern.volume_spikes_detected,
        "Expected to detect 100% volume spike (30km to 60km)"
    );
    assert!(
        !pattern.spike_weeks.is_empty(),
        "Expected spike weeks to be non-empty"
    );
}
