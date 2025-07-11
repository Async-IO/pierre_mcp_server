use chrono::Utc;
use pierre_mcp_server::models::{Activity, Athlete, PersonalRecord, PrMetric, SportType, Stats};

/// Test data for creating sample activities
fn create_sample_activity() -> Activity {
    Activity {
        id: "12345".into(),
        name: "Morning Run".into(),
        sport_type: SportType::Run,
        start_date: Utc::now(),
        duration_seconds: 1800,        // 30 minutes
        distance_meters: Some(5000.0), // 5km
        elevation_gain: Some(100.0),
        average_heart_rate: Some(150),
        max_heart_rate: Some(175),
        average_speed: Some(2.78), // ~10 km/h
        max_speed: Some(4.17),     // ~15 km/h
        calories: Some(300),
        steps: Some(7500),
        heart_rate_zones: None,

        // Advanced metrics (all None for basic test)
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
        city: Some("Montreal".into()),
        region: Some("Quebec".into()),
        country: Some("Canada".into()),
        trail_name: Some("Mount Royal Trail".into()),
        provider: "strava".into(),
    }
}

/// Test data for creating sample athlete
fn create_sample_athlete() -> Athlete {
    Athlete {
        id: "67890".into(),
        username: "runner123".into(),
        firstname: Some("John".into()),
        lastname: Some("Doe".into()),
        profile_picture: Some("https://example.com/avatar.jpg".into()),
        provider: "strava".into(),
    }
}

#[test]
fn test_activity_creation() {
    let activity = create_sample_activity();
    assert_eq!(activity.id, "12345");
    assert_eq!(activity.name, "Morning Run");
    assert!(matches!(activity.sport_type, SportType::Run));
    assert_eq!(activity.duration_seconds, 1800);
    assert_eq!(activity.distance_meters, Some(5000.0));
    assert_eq!(activity.provider, "strava");
}

#[test]
fn test_activity_serialization() {
    let activity = create_sample_activity();

    // Test JSON serialization
    let json = serde_json::to_string(&activity).expect("Failed to serialize activity");
    assert!(json.contains("Morning Run"));
    assert!(json.contains("run")); // sport_type should be snake_case

    // Test JSON deserialization
    let deserialized: Activity =
        serde_json::from_str(&json).expect("Failed to deserialize activity");
    assert_eq!(deserialized.id, activity.id);
    assert_eq!(deserialized.name, activity.name);
    assert!(matches!(deserialized.sport_type, SportType::Run));
}

#[test]
fn test_sport_type_serialization() {
    // Test standard sport types
    assert_eq!(serde_json::to_string(&SportType::Run).unwrap(), "\"run\"");
    assert_eq!(serde_json::to_string(&SportType::Ride).unwrap(), "\"ride\"");
    assert_eq!(
        serde_json::to_string(&SportType::VirtualRun).unwrap(),
        "\"virtual_run\""
    );

    // Test Other variant
    let custom_sport = SportType::Other("CrossCountrySkiing".into());
    let json = serde_json::to_string(&custom_sport).unwrap();
    assert!(json.contains("CrossCountrySkiing"));

    // Test deserialization
    let sport: SportType = serde_json::from_str("\"run\"").unwrap();
    assert!(matches!(sport, SportType::Run));
}

#[test]
fn test_athlete_creation() {
    let athlete = create_sample_athlete();
    assert_eq!(athlete.id, "67890");
    assert_eq!(athlete.username, "runner123");
    assert_eq!(athlete.firstname, Some("John".into()));
    assert_eq!(athlete.lastname, Some("Doe".into()));
    assert_eq!(athlete.provider, "strava");
}

#[test]
fn test_athlete_serialization() {
    let athlete = create_sample_athlete();

    // Test JSON serialization
    let json = serde_json::to_string(&athlete).expect("Failed to serialize athlete");
    assert!(json.contains("runner123"));
    assert!(json.contains("John"));

    // Test JSON deserialization
    let deserialized: Athlete = serde_json::from_str(&json).expect("Failed to deserialize athlete");
    assert_eq!(deserialized.username, athlete.username);
    assert_eq!(deserialized.firstname, athlete.firstname);
}

#[test]
fn test_stats_creation() {
    let stats = Stats {
        total_activities: 150,
        total_distance: 1_500_000.0, // 1500 km
        total_duration: 540_000,     // 150 hours
        total_elevation_gain: 25000.0,
    };

    assert_eq!(stats.total_activities, 150);
    {
        assert!((stats.total_distance - 1_500_000.0).abs() < f64::EPSILON);
        assert!((stats.total_elevation_gain - 25000.0).abs() < f64::EPSILON);
    }
    assert_eq!(stats.total_duration, 540_000);
}

#[test]
fn test_stats_serialization() {
    let stats = Stats {
        total_activities: 100,
        total_distance: 1_000_000.0,
        total_duration: 360_000,
        total_elevation_gain: 15000.0,
    };

    let json = serde_json::to_string(&stats).expect("Failed to serialize stats");
    let deserialized: Stats = serde_json::from_str(&json).expect("Failed to deserialize stats");

    assert_eq!(deserialized.total_activities, stats.total_activities);
    assert!((deserialized.total_distance - stats.total_distance).abs() < f64::EPSILON);
}

#[test]
fn test_personal_record_creation() {
    let pr = PersonalRecord {
        activity_id: "12345".into(),
        metric: PrMetric::LongestDistance,
        value: 42195.0, // Marathon distance in meters
        date: Utc::now(),
    };

    assert_eq!(pr.activity_id, "12345");
    assert!(matches!(pr.metric, PrMetric::LongestDistance));
    assert!((pr.value - 42195.0).abs() < f64::EPSILON);
}

#[test]
fn test_pr_metric_serialization() {
    assert_eq!(
        serde_json::to_string(&PrMetric::FastestPace).unwrap(),
        "\"fastest_pace\""
    );
    assert_eq!(
        serde_json::to_string(&PrMetric::LongestDistance).unwrap(),
        "\"longest_distance\""
    );
    assert_eq!(
        serde_json::to_string(&PrMetric::HighestElevation).unwrap(),
        "\"highest_elevation\""
    );
    assert_eq!(
        serde_json::to_string(&PrMetric::FastestTime).unwrap(),
        "\"fastest_time\""
    );

    // Test deserialization
    let metric: PrMetric = serde_json::from_str("\"fastest_pace\"").unwrap();
    assert!(matches!(metric, PrMetric::FastestPace));
}

#[test]
fn test_activity_optional_fields() {
    let minimal_activity = Activity {
        id: "123".into(),
        name: "Quick Walk".into(),
        sport_type: SportType::Walk,
        start_date: Utc::now(),
        duration_seconds: 600, // 10 minutes
        distance_meters: None, // No distance tracking
        elevation_gain: None,
        average_heart_rate: None,
        max_heart_rate: None,
        average_speed: None,
        max_speed: None,
        calories: None,
        steps: None,
        heart_rate_zones: None,

        // Advanced metrics (all None)
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
        city: None,
        region: None,
        country: None,
        trail_name: None,
        provider: "manual".into(),
    };

    // Should serialize and deserialize correctly even with None values
    let json = serde_json::to_string(&minimal_activity).unwrap();
    let deserialized: Activity = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.distance_meters, None);
    assert_eq!(deserialized.calories, None);
    assert_eq!(deserialized.provider, "manual");
}
