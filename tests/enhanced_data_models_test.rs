// ABOUTME: Tests for enhanced data models supporting advanced intelligence engines
// ABOUTME: Validates power metrics, performance data, and activity modeling
//! Tests for enhanced data models supporting advanced intelligence engines

use chrono::Utc;
use pierre_mcp_server::models::*;

#[test]
fn test_enhanced_activity_with_power_metrics() {
    let activity = Activity {
        id: "power_test".to_string(),
        name: "Cycling Workout".to_string(),
        sport_type: SportType::Ride,
        start_date: Utc::now(),
        duration_seconds: 3600,         // 1 hour
        distance_meters: Some(40000.0), // 40km
        elevation_gain: Some(500.0),
        average_heart_rate: Some(160),
        max_heart_rate: Some(185),
        average_speed: Some(11.11), // 40km/h
        max_speed: Some(15.28),     // 55km/h
        calories: Some(800),
        steps: None,
        heart_rate_zones: None,

        // Power metrics
        average_power: Some(250),
        max_power: Some(450),
        normalized_power: Some(265),
        ftp: Some(280),
        power_zones: Some(vec![
            PowerZone {
                name: "Zone 1".to_string(),
                min_power: 0,
                max_power: 140,    // 50% FTP
                time_in_zone: 600, // 10 minutes
            },
            PowerZone {
                name: "Zone 2".to_string(),
                min_power: 140,
                max_power: 196,     // 70% FTP
                time_in_zone: 2400, // 40 minutes
            },
        ]),

        // Cadence
        average_cadence: Some(85),
        max_cadence: Some(120),

        // Environmental
        temperature: Some(22.0),
        humidity: Some(65.0),
        average_altitude: Some(150.0),
        wind_speed: Some(5.0),

        // Training metrics
        training_stress_score: Some(95.0),
        intensity_factor: Some(0.85),
        suffer_score: Some(120),

        // Other advanced metrics (None for this test)
        hrv_score: None,
        recovery_heart_rate: None,
        ground_contact_time: None,
        vertical_oscillation: None,
        stride_length: None,
        running_power: None,
        breathing_rate: None,
        spo2: None,
        time_series_data: None,

        start_latitude: Some(45.5017),
        start_longitude: Some(-73.5673),
        city: Some("Montreal".to_string()),
        region: Some("Quebec".to_string()),
        country: Some("Canada".to_string()),
        trail_name: Some("Lachine Canal".to_string()),
        provider: "strava".to_string(),
    };

    // Test serialization
    let json = serde_json::to_string(&activity).expect("Failed to serialize enhanced activity");
    assert!(json.contains("average_power"));
    assert!(json.contains("normalized_power"));
    assert!(json.contains("training_stress_score"));

    // Test deserialization
    let deserialized: Activity =
        serde_json::from_str(&json).expect("Failed to deserialize enhanced activity");
    assert_eq!(deserialized.average_power, Some(250));
    assert_eq!(deserialized.normalized_power, Some(265));
    assert_eq!(deserialized.training_stress_score, Some(95.0));
}

#[test]
fn test_running_activity_with_biomechanical_data() {
    let activity = Activity {
        id: "run_test".to_string(),
        name: "Tempo Run".to_string(),
        sport_type: SportType::Run,
        start_date: Utc::now(),
        duration_seconds: 2700,         // 45 minutes
        distance_meters: Some(10000.0), // 10km
        elevation_gain: Some(50.0),
        average_heart_rate: Some(165),
        max_heart_rate: Some(180),
        average_speed: Some(3.7), // ~4:30/km pace
        max_speed: Some(4.5),
        calories: Some(550),
        steps: Some(12000),
        heart_rate_zones: None,

        // Running-specific biomechanics
        average_cadence: Some(180), // steps per minute
        max_cadence: Some(200),
        ground_contact_time: Some(240),  // milliseconds
        vertical_oscillation: Some(8.5), // centimeters
        stride_length: Some(1.2),        // meters
        running_power: Some(280),        // watts

        // Respiratory
        breathing_rate: Some(35),
        spo2: Some(98.5),

        // HRV and recovery
        hrv_score: Some(45.2),
        recovery_heart_rate: Some(25), // HR drop in first minute

        // Power metrics (None for running)
        average_power: None,
        max_power: None,
        normalized_power: None,
        power_zones: None,
        ftp: None,

        // Environmental
        temperature: Some(18.0),
        humidity: Some(70.0),
        average_altitude: Some(100.0),
        wind_speed: Some(3.0),

        // Training
        training_stress_score: Some(75.0),
        intensity_factor: Some(0.78),
        suffer_score: Some(85),

        time_series_data: None,
        start_latitude: Some(45.5017),
        start_longitude: Some(-73.5673),
        city: Some("Montreal".to_string()),
        region: Some("Quebec".to_string()),
        country: Some("Canada".to_string()),
        trail_name: Some("Mount Royal".to_string()),
        provider: "garmin".to_string(),
    };

    // Verify running-specific metrics
    assert_eq!(activity.ground_contact_time, Some(240));
    assert_eq!(activity.vertical_oscillation, Some(8.5));
    assert_eq!(activity.stride_length, Some(1.2));
    assert_eq!(activity.running_power, Some(280));
    assert_eq!(activity.breathing_rate, Some(35));
    assert_eq!(activity.spo2, Some(98.5));
}

#[test]
fn test_sleep_session_model() {
    let sleep_session = SleepSession {
        id: "sleep_123".to_string(),
        start_time: Utc::now() - chrono::Duration::hours(8),
        end_time: Utc::now(),
        time_in_bed: 480,      // 8 hours
        total_sleep_time: 420, // 7 hours
        sleep_efficiency: 87.5,
        sleep_score: Some(82.0),
        stages: vec![
            SleepStage {
                stage_type: SleepStageType::Awake,
                start_time: Utc::now() - chrono::Duration::hours(8),
                duration_minutes: 15,
            },
            SleepStage {
                stage_type: SleepStageType::Light,
                start_time: Utc::now() - chrono::Duration::hours(7) - chrono::Duration::minutes(45),
                duration_minutes: 180, // 3 hours
            },
            SleepStage {
                stage_type: SleepStageType::Deep,
                start_time: Utc::now() - chrono::Duration::hours(4) - chrono::Duration::minutes(45),
                duration_minutes: 120, // 2 hours
            },
            SleepStage {
                stage_type: SleepStageType::Rem,
                start_time: Utc::now() - chrono::Duration::hours(2) - chrono::Duration::minutes(45),
                duration_minutes: 105, // 1.75 hours
            },
        ],
        hrv_during_sleep: Some(42.5),
        respiratory_rate: Some(16.0),
        temperature_variation: Some(0.8),
        wake_count: Some(2),
        sleep_onset_latency: Some(12),
        provider: "oura".to_string(),
    };

    // Test stage summary
    let summary = sleep_session.stage_summary();
    assert_eq!(summary.get(&SleepStageType::Light), Some(&180));
    assert_eq!(summary.get(&SleepStageType::Deep), Some(&120));
    assert_eq!(summary.get(&SleepStageType::Rem), Some(&105));

    // Test sleep percentages
    let deep_percentage = sleep_session.deep_sleep_percentage();
    let rem_percentage = sleep_session.rem_sleep_percentage();

    assert!((deep_percentage - 28.57).abs() < 0.1); // 120/420 * 100 ≈ 28.57%
    assert!((rem_percentage - 25.0).abs() < 0.1); // 105/420 * 100 = 25%

    // Test serialization
    let json = serde_json::to_string(&sleep_session).expect("Failed to serialize sleep session");
    assert!(json.contains("sleep_efficiency"));
    assert!(json.contains("hrv_during_sleep"));
}

#[test]
fn test_recovery_metrics_model() {
    let recovery_metrics = RecoveryMetrics {
        date: Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc(),
        recovery_score: Some(78.0),
        readiness_score: Some(82.0),
        hrv_status: Some("Balanced".to_string()),
        sleep_score: Some(85.0),
        stress_level: Some(25.0), // Low stress
        training_load: Some(65.0),
        resting_heart_rate: Some(52),
        body_temperature: Some(36.7),
        resting_respiratory_rate: Some(14.0),
        provider: "whoop".to_string(),
    };

    // Test readiness for training
    assert!(recovery_metrics.is_ready_for_training());

    // Test wellness score calculation
    let wellness = recovery_metrics.wellness_score().unwrap();
    // Should be average of recovery (78), sleep (85), and inverted stress (75) = 79.33
    assert!((wellness - 79.33).abs() < 0.1);

    // Test serialization (before creating poor_recovery to avoid borrow checker issue)
    let json =
        serde_json::to_string(&recovery_metrics).expect("Failed to serialize recovery metrics");
    assert!(json.contains("recovery_score"));
    assert!(json.contains("readiness_score"));

    // Test with poor recovery
    let poor_recovery = RecoveryMetrics {
        recovery_score: Some(45.0),
        readiness_score: Some(40.0),
        stress_level: Some(80.0), // High stress
        ..recovery_metrics
    };

    assert!(!poor_recovery.is_ready_for_training());
}

#[test]
fn test_time_series_data_model() {
    let time_series = TimeSeriesData {
        timestamps: vec![0, 30, 60, 90, 120], // Every 30 seconds for 2 minutes
        heart_rate: Some(vec![120, 135, 150, 165, 160]),
        power: Some(vec![200, 250, 280, 320, 300]),
        cadence: Some(vec![80, 85, 90, 95, 88]),
        speed: Some(vec![8.0, 9.5, 11.0, 12.5, 11.8]),
        altitude: Some(vec![100.0, 105.0, 110.0, 115.0, 118.0]),
        temperature: Some(vec![20.0, 20.2, 20.5, 20.8, 21.0]),
        gps_coordinates: Some(vec![
            (45.5017, -73.5673),
            (45.5020, -73.5670),
            (45.5023, -73.5667),
            (45.5026, -73.5664),
            (45.5029, -73.5661),
        ]),
    };

    // Verify data integrity
    assert_eq!(time_series.timestamps.len(), 5);
    assert_eq!(time_series.heart_rate.as_ref().unwrap().len(), 5);
    assert_eq!(time_series.power.as_ref().unwrap().len(), 5);
    assert_eq!(time_series.gps_coordinates.as_ref().unwrap().len(), 5);

    // Test serialization
    let json = serde_json::to_string(&time_series).expect("Failed to serialize time series data");
    assert!(json.contains("timestamps"));
    assert!(json.contains("gps_coordinates"));

    // Test deserialization
    let deserialized: TimeSeriesData =
        serde_json::from_str(&json).expect("Failed to deserialize time series");
    assert_eq!(deserialized.timestamps, time_series.timestamps);
    assert_eq!(deserialized.heart_rate, time_series.heart_rate);
}

#[test]
fn test_health_metrics_model() {
    let health_metrics = HealthMetrics {
        date: Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc(),
        weight: Some(70.5),
        body_fat_percentage: Some(12.5),
        muscle_mass: Some(58.2),
        bone_mass: Some(3.1),
        body_water_percentage: Some(62.8),
        bmr: Some(1750),
        blood_pressure: Some((120, 80)),
        blood_glucose: Some(95.0),
        vo2_max: Some(52.8),
        provider: "garmin".to_string(),
    };

    // Test serialization
    let json = serde_json::to_string(&health_metrics).expect("Failed to serialize health metrics");
    assert!(json.contains("body_fat_percentage"));
    assert!(json.contains("blood_pressure"));
    assert!(json.contains("vo2_max"));

    // Test deserialization
    let deserialized: HealthMetrics =
        serde_json::from_str(&json).expect("Failed to deserialize health metrics");
    assert_eq!(deserialized.weight, Some(70.5));
    assert_eq!(deserialized.blood_pressure, Some((120, 80)));
    assert_eq!(deserialized.vo2_max, Some(52.8));
}

#[test]
fn test_power_zone_model() {
    let power_zones = vec![
        PowerZone {
            name: "Active Recovery".to_string(),
            min_power: 0,
            max_power: 140,    // <50% FTP
            time_in_zone: 300, // 5 minutes
        },
        PowerZone {
            name: "Endurance".to_string(),
            min_power: 140,
            max_power: 196,     // 50-70% FTP
            time_in_zone: 1800, // 30 minutes
        },
        PowerZone {
            name: "Tempo".to_string(),
            min_power: 196,
            max_power: 238,    // 70-85% FTP
            time_in_zone: 600, // 10 minutes
        },
    ];

    // Test serialization
    let json = serde_json::to_string(&power_zones).expect("Failed to serialize power zones");
    assert!(json.contains("Active Recovery"));
    assert!(json.contains("time_in_zone"));

    // Test deserialization
    let deserialized: Vec<PowerZone> =
        serde_json::from_str(&json).expect("Failed to deserialize power zones");
    assert_eq!(deserialized.len(), 3);
    assert_eq!(deserialized[0].name, "Active Recovery");
    assert_eq!(deserialized[1].time_in_zone, 1800);
}

#[test]
fn test_backward_compatibility() {
    // Test that existing Activity model still works without advanced metrics
    let basic_activity = Activity {
        id: "basic_test".to_string(),
        name: "Simple Run".to_string(),
        sport_type: SportType::Run,
        start_date: Utc::now(),
        duration_seconds: 1800,
        distance_meters: Some(5000.0),
        elevation_gain: Some(100.0),
        average_heart_rate: Some(150),
        max_heart_rate: Some(175),
        average_speed: Some(2.78),
        max_speed: Some(4.17),
        calories: Some(300),
        steps: Some(7500),
        heart_rate_zones: None,

        // All advanced metrics should be None
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

        start_latitude: Some(45.5017),
        start_longitude: Some(-73.5673),
        city: Some("Montreal".to_string()),
        region: Some("Quebec".to_string()),
        country: Some("Canada".to_string()),
        trail_name: Some("Mount Royal Trail".to_string()),
        provider: "strava".to_string(),
    };

    // Should serialize and deserialize correctly
    let json = serde_json::to_string(&basic_activity).expect("Failed to serialize basic activity");
    let deserialized: Activity =
        serde_json::from_str(&json).expect("Failed to deserialize basic activity");

    assert_eq!(deserialized.id, "basic_test");
    assert_eq!(deserialized.average_power, None);
    assert!(deserialized.time_series_data.is_none());
}
