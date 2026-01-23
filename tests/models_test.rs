// ABOUTME: Unit tests for models functionality
// ABOUTME: Validates models behavior, edge cases, and error handling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use chrono::Utc;
use pierre_mcp_server::models::{
    Activity, ActivityBuilder, Athlete, AuthorizationCode, EncryptedToken, HeartRateZone,
    PersonalRecord, PowerZone, PrMetric, SegmentEffort, SportType, Stats, Tenant, User, UserStatus,
    UserTier,
};
use pierre_mcp_server::permissions::UserRole;
use uuid::Uuid;

/// Test data for creating sample activities
fn create_sample_activity() -> Activity {
    ActivityBuilder::new(
        "12345",
        "Morning Run",
        SportType::Run,
        Utc::now(),
        1800,
        "strava",
    )
    .distance_meters(5000.0)
    .elevation_gain(100.0)
    .average_heart_rate(150)
    .max_heart_rate(175)
    .average_speed(2.78)
    .max_speed(4.17)
    .calories(300)
    .steps(7500)
    .start_latitude(45.5017)
    .start_longitude(-73.5673)
    .city("Montreal".to_owned())
    .region("Quebec".to_owned())
    .country("Canada".to_owned())
    .trail_name("Mount Royal Trail".to_owned())
    .workout_type(10)
    .sport_type_detail("TrailRun".to_owned())
    .build()
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
    assert_eq!(activity.id(), "12345");
    assert_eq!(activity.name(), "Morning Run");
    assert!(matches!(*activity.sport_type(), SportType::Run));
    assert_eq!(activity.duration_seconds(), 1800);
    assert_eq!(activity.distance_meters(), Some(5000.0));
    assert_eq!(activity.provider(), "strava");
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
    assert_eq!(deserialized.id(), activity.id());
    assert_eq!(deserialized.name(), activity.name());
    assert!(matches!(deserialized.sport_type(), SportType::Run));
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
    let minimal_activity = ActivityBuilder::new(
        "123",
        "Quick Walk",
        SportType::Walk,
        Utc::now(),
        600,
        "manual",
    )
    .start_latitude(45.5017)
    .start_longitude(-73.5673)
    .build();

    // Should serialize and deserialize correctly even with None values
    let json = serde_json::to_string(&minimal_activity).unwrap();
    let deserialized: Activity = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.distance_meters(), None);
    assert_eq!(deserialized.calories(), None);
    assert_eq!(deserialized.provider(), "manual");
}

// User and Authentication Model Tests

#[test]
fn test_user_tier_monthly_limits() {
    assert_eq!(UserTier::Starter.monthly_limit(), Some(10_000));
    assert_eq!(UserTier::Professional.monthly_limit(), Some(100_000));
    assert_eq!(UserTier::Enterprise.monthly_limit(), None);
}

#[test]
fn test_user_tier_serialization() {
    let starter = UserTier::Starter;
    let json = serde_json::to_string(&starter).unwrap();
    let deserialized: UserTier = serde_json::from_str(&json).unwrap();
    assert_eq!(starter, deserialized);

    let professional = UserTier::Professional;
    let json = serde_json::to_string(&professional).unwrap();
    let deserialized: UserTier = serde_json::from_str(&json).unwrap();
    assert_eq!(professional, deserialized);

    let enterprise = UserTier::Enterprise;
    let json = serde_json::to_string(&enterprise).unwrap();
    let deserialized: UserTier = serde_json::from_str(&json).unwrap();
    assert_eq!(enterprise, deserialized);
}

#[test]
fn test_user_status_can_login() {
    assert!(UserStatus::Active.can_login());
    assert!(!UserStatus::Pending.can_login());
    assert!(!UserStatus::Suspended.can_login());
}

#[test]
fn test_user_status_serialization() {
    let active = UserStatus::Active;
    let json = serde_json::to_string(&active).unwrap();
    let deserialized: UserStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(active, deserialized);

    let pending = UserStatus::Pending;
    let json = serde_json::to_string(&pending).unwrap();
    let deserialized: UserStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(pending, deserialized);

    let suspended = UserStatus::Suspended;
    let json = serde_json::to_string(&suspended).unwrap();
    let deserialized: UserStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(suspended, deserialized);
}

#[test]
fn test_user_creation_with_required_fields() {
    let now = Utc::now();
    let user_id = Uuid::new_v4();

    let user = User {
        id: user_id,
        email: "test@example.com".to_owned(),
        display_name: Some("Test User".to_owned()),
        password_hash: "hashed_password".to_owned(),
        tier: UserTier::Professional,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        role: UserRole::User,
        approved_by: None,
        approved_at: Some(now),
        created_at: now,
        last_active: now,
        firebase_uid: None,
        auth_provider: String::new(),
    };

    assert_eq!(user.id, user_id);
    assert_eq!(user.email, "test@example.com");
    assert_eq!(user.display_name, Some("Test User".to_owned()));
    assert_eq!(user.tier, UserTier::Professional);
    assert!(user.is_active);
    assert_eq!(user.user_status, UserStatus::Active);
    assert!(!user.is_admin);
}

#[test]
fn test_user_serialization_roundtrip() {
    let now = Utc::now();
    let original_user = User {
        id: Uuid::new_v4(),
        email: "test@example.com".to_owned(),
        display_name: Some("Test User".to_owned()),
        password_hash: "hashed_password".to_owned(),
        tier: UserTier::Enterprise,
        strava_token: Some(EncryptedToken {
            access_token: "encrypted_access_token".to_owned(),
            refresh_token: "encrypted_refresh_token".to_owned(),
            expires_at: now + chrono::Duration::hours(1),
            scope: "read".to_owned(),
        }),
        fitbit_token: Some(EncryptedToken {
            access_token: "encrypted_fitbit_access".to_owned(),
            refresh_token: "encrypted_fitbit_refresh".to_owned(),
            expires_at: now + chrono::Duration::hours(2),
            scope: "read_all".to_owned(),
        }),
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: true,
        role: UserRole::Admin,
        approved_by: Some(Uuid::new_v4()),
        approved_at: Some(now),
        created_at: now,
        last_active: now,
        firebase_uid: None,
        auth_provider: String::new(),
    };

    let json = serde_json::to_string(&original_user).unwrap();
    let deserialized_user: User = serde_json::from_str(&json).unwrap();

    assert_eq!(original_user.id, deserialized_user.id);
    assert_eq!(original_user.email, deserialized_user.email);
    assert_eq!(original_user.display_name, deserialized_user.display_name);
    assert_eq!(original_user.tier, deserialized_user.tier);
    assert_eq!(original_user.user_status, deserialized_user.user_status);
    assert_eq!(original_user.is_admin, deserialized_user.is_admin);
    // Note: EncryptedToken doesn't implement PartialEq, so we test individual fields
    assert!(original_user.strava_token.is_some());
    assert!(deserialized_user.strava_token.is_some());
}

#[test]
fn test_tenant_creation() {
    let now = Utc::now();
    let tenant_id = Uuid::new_v4();

    let tenant = Tenant {
        id: tenant_id,
        name: "Test Company".to_owned(),
        slug: "test-company".to_owned(),
        domain: Some("testcompany.com".to_owned()),
        plan: "enterprise".to_owned(),
        owner_user_id: Uuid::new_v4(),
        created_at: now,
        updated_at: now,
    };

    assert_eq!(tenant.id, tenant_id);
    assert_eq!(tenant.name, "Test Company");
    assert_eq!(tenant.domain, Some("testcompany.com".to_owned()));
    assert_eq!(tenant.plan, "enterprise");
    assert_eq!(tenant.slug, "test-company");
}

#[test]
fn test_tenant_serialization_roundtrip() {
    let now = Utc::now();
    let original_tenant = Tenant {
        id: Uuid::new_v4(),
        name: "Acme Corp".to_owned(),
        slug: "acme-corp".to_owned(),
        domain: Some("acme.com".to_owned()),
        plan: "professional".to_owned(),
        owner_user_id: Uuid::new_v4(),
        created_at: now,
        updated_at: now,
    };

    let json = serde_json::to_string(&original_tenant).unwrap();
    let deserialized_tenant: Tenant = serde_json::from_str(&json).unwrap();

    assert_eq!(original_tenant.id, deserialized_tenant.id);
    assert_eq!(original_tenant.name, deserialized_tenant.name);
    assert_eq!(original_tenant.domain, deserialized_tenant.domain);
    assert_eq!(original_tenant.plan, deserialized_tenant.plan);
    assert_eq!(original_tenant.slug, deserialized_tenant.slug);
}

#[test]
fn test_heart_rate_zone_creation() {
    let zone = HeartRateZone {
        name: "Zone 2".to_owned(),
        min_hr: 120,
        max_hr: 140,
        minutes: 25,
    };

    assert_eq!(zone.name, "Zone 2");
    assert_eq!(zone.min_hr, 120);
    assert_eq!(zone.max_hr, 140);
    assert_eq!(zone.minutes, 25);
}

#[test]
fn test_power_zone_creation() {
    let zone = PowerZone {
        name: "Zone 3".to_owned(),
        min_power: 200,
        max_power: 250,
        time_in_zone: 15,
    };

    assert_eq!(zone.name, "Zone 3");
    assert_eq!(zone.min_power, 200);
    assert_eq!(zone.max_power, 250);
    assert_eq!(zone.time_in_zone, 15);
}

#[test]
fn test_authorization_code_new() {
    let code = AuthorizationCode::new(
        "auth_code_123".to_owned(),
        "client_456".to_owned(),
        "https://redirect.uri".to_owned(),
        "read write".to_owned(),
        Some(Uuid::new_v4()),
    );

    assert_eq!(code.code, "auth_code_123");
    assert_eq!(code.client_id, "client_456");
    assert_eq!(code.redirect_uri, "https://redirect.uri");
    assert_eq!(code.scope, "read write");
    assert!(!code.is_used);

    // Code should expire in 10 minutes
    let now = Utc::now();
    let expected_expiry = now + chrono::Duration::minutes(10);
    let expiry_diff = (code.expires_at - expected_expiry).num_seconds().abs();
    assert!(
        expiry_diff < 5,
        "Expiry time should be within 5 seconds of expected"
    );
}

#[test]
fn test_encrypted_token_creation() {
    let now = Utc::now();
    let token = EncryptedToken {
        access_token: "encrypted_access_12345".to_owned(),
        refresh_token: "encrypted_refresh_67890".to_owned(),
        expires_at: now + chrono::Duration::hours(1),
        scope: "read write".to_owned(),
    };

    assert_eq!(token.access_token, "encrypted_access_12345");
    assert_eq!(token.refresh_token, "encrypted_refresh_67890");
    assert_eq!(token.scope, "read write");
    assert!(token.expires_at > now);
}

#[test]
fn test_user_with_encrypted_tokens() {
    let now = Utc::now();
    let user = User {
        id: Uuid::new_v4(),
        email: "tokenuser@example.com".to_owned(),
        display_name: Some("Token User".to_owned()),
        password_hash: "secure_hash".to_owned(),
        tier: UserTier::Professional,
        strava_token: Some(EncryptedToken {
            access_token: "strava_encrypted_access".to_owned(),
            refresh_token: "strava_encrypted_refresh".to_owned(),
            expires_at: now + chrono::Duration::hours(6),
            scope: "read_all,activity:read".to_owned(),
        }),
        fitbit_token: Some(EncryptedToken {
            access_token: "fitbit_encrypted_access".to_owned(),
            refresh_token: "fitbit_encrypted_refresh".to_owned(),
            expires_at: now + chrono::Duration::hours(8),
            scope: "activity,heartrate,sleep".to_owned(),
        }),
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        role: UserRole::User,
        approved_by: Some(Uuid::new_v4()),
        approved_at: Some(now),
        created_at: now,
        last_active: now,
        firebase_uid: None,
        auth_provider: String::new(),
    };

    // Verify tokens are present
    assert!(user.strava_token.is_some());
    assert!(user.fitbit_token.is_some());

    if let Some(strava_token) = &user.strava_token {
        assert_eq!(strava_token.scope, "read_all,activity:read");
        assert!(strava_token.expires_at > now);
    }

    if let Some(fitbit_token) = &user.fitbit_token {
        assert_eq!(fitbit_token.scope, "activity,heartrate,sleep");
        assert!(fitbit_token.expires_at > now);
    }
}

/// Create test activity with detailed segment efforts for validation
fn create_test_activity_with_segments() -> Activity {
    ActivityBuilder::new(
        "test_123",
        "Trail Run with Segments",
        SportType::Run,
        Utc::now(),
        3600,
        "strava",
    )
    .distance_meters(10_000.0)
    .elevation_gain(250.0)
    .average_heart_rate(155)
    .max_heart_rate(180)
    .average_speed(2.78)
    .max_speed(4.5)
    .calories(600)
    .steps(12_000)
    .average_cadence(180)
    .max_cadence(195)
    .temperature(15.0)
    .humidity(60.0)
    .average_altitude(300.0)
    .ground_contact_time(220)
    .vertical_oscillation(8.5)
    .stride_length(1.25)
    .running_power(250)
    .suffer_score(85)
    .start_latitude(45.5017)
    .start_longitude(-73.5673)
    .city("Saint-Hippolyte".to_owned())
    .region("Quebec".to_owned())
    .country("Canada".to_owned())
    .trail_name("Mont Rigaud Trail".to_owned())
    .workout_type(10)
    .sport_type_detail("TrailRun".to_owned())
    .segment_efforts(vec![
        SegmentEffort {
            id: "seg_001".into(),
            name: "Steep Climb".into(),
            elapsed_time: 600,
            moving_time: Some(590),
            start_date: Utc::now(),
            distance: 1200.0,
            average_heart_rate: Some(170),
            max_heart_rate: Some(185),
            average_cadence: Some(165),
            average_watts: None,
            kom_rank: Some(5),
            pr_rank: Some(2),
            climb_category: Some(3),
            average_grade: Some(8.5),
            elevation_gain: Some(102.0),
        },
        SegmentEffort {
            id: "seg_002".into(),
            name: "Fast Descent".into(),
            elapsed_time: 300,
            moving_time: Some(295),
            start_date: Utc::now(),
            distance: 1500.0,
            average_heart_rate: Some(145),
            max_heart_rate: Some(160),
            average_cadence: Some(190),
            average_watts: None,
            kom_rank: Some(12),
            pr_rank: Some(1),
            climb_category: None,
            average_grade: Some(-6.5),
            elevation_gain: None,
        },
    ])
    .build()
}

/// Validate climb segment fields
#[allow(clippy::float_cmp)] // Test assertions with exact literal float values
fn validate_climb_segment(climb: &SegmentEffort) {
    assert_eq!(climb.name, "Steep Climb");
    assert_eq!(climb.elapsed_time, 600);
    assert_eq!(climb.distance, 1200.0);
    assert_eq!(climb.kom_rank, Some(5));
    assert_eq!(climb.pr_rank, Some(2));
    assert_eq!(climb.climb_category, Some(3));
    assert_eq!(climb.average_grade, Some(8.5));
    assert_eq!(climb.elevation_gain, Some(102.0));
}

/// Validate descent segment fields
#[allow(clippy::float_cmp)] // Test assertions with exact literal float values
fn validate_descent_segment(descent: &SegmentEffort) {
    assert_eq!(descent.name, "Fast Descent");
    assert_eq!(descent.elapsed_time, 300);
    assert_eq!(descent.pr_rank, Some(1), "Should be PR!");
    assert_eq!(
        descent.average_grade,
        Some(-6.5),
        "Should be negative grade for descent"
    );
    assert_eq!(
        descent.climb_category, None,
        "Descents don't have climb category"
    );
}

#[test]
#[allow(clippy::float_cmp)] // Test assertions with exact literal float values
fn test_activity_detailed_fields() {
    let activity = create_test_activity_with_segments();

    // Validate workout_type
    assert_eq!(activity.workout_type(), Some(10));
    assert!(
        activity.workout_type().is_some(),
        "workout_type should be present"
    );

    // Validate sport_type_detail
    assert_eq!(activity.sport_type_detail(), Some("TrailRun"));
    assert!(
        activity.sport_type_detail().is_some(),
        "sport_type_detail should be present"
    );

    // Validate segment_efforts
    assert!(
        activity.segment_efforts().is_some(),
        "segment_efforts should be present"
    );
    let segments = activity.segment_efforts().unwrap();
    assert_eq!(segments.len(), 2, "Should have 2 segment efforts");

    // Validate segments using helper functions
    validate_climb_segment(&segments[0]);
    validate_descent_segment(&segments[1]);

    // Validate location fields work together
    assert_eq!(activity.city(), Some("Saint-Hippolyte"));
    assert_eq!(activity.region(), Some("Quebec"));
    assert_eq!(activity.country(), Some("Canada"));
}

#[test]
fn test_activity_detailed_fields_serialization() {
    // Test that detailed fields serialize/deserialize correctly
    let segment = SegmentEffort {
        id: "seg_test".into(),
        name: "Test Segment".into(),
        elapsed_time: 180,
        moving_time: Some(175),
        start_date: Utc::now(),
        distance: 800.0,
        average_heart_rate: Some(160),
        max_heart_rate: Some(175),
        average_cadence: Some(170),
        average_watts: Some(220),
        kom_rank: Some(1),
        pr_rank: Some(1),
        climb_category: Some(4),
        average_grade: Some(5.2),
        elevation_gain: Some(42.0),
    };

    let activity = ActivityBuilder::new(
        "serialize_test",
        "Serialization Test",
        SportType::Run,
        Utc::now(),
        1800,
        "strava",
    )
    .distance_meters(5000.0)
    .elevation_gain(100.0)
    .average_heart_rate(150)
    .max_heart_rate(170)
    .average_speed(2.78)
    .max_speed(3.5)
    .calories(300)
    .steps(7500)
    .city("Montreal".to_owned())
    .region("Quebec".to_owned())
    .country("Canada".to_owned())
    .workout_type(11)
    .sport_type_detail("RoadRun".to_owned())
    .segment_efforts(vec![segment])
    .build();

    // Serialize to JSON
    let json = serde_json::to_string(&activity).expect("Should serialize to JSON");

    // Verify JSON contains the new fields
    assert!(
        json.contains("workout_type"),
        "JSON should contain workout_type"
    );
    assert!(
        json.contains("sport_type_detail"),
        "JSON should contain sport_type_detail"
    );
    assert!(
        json.contains("segment_efforts"),
        "JSON should contain segment_efforts"
    );
    assert!(
        json.contains("RoadRun"),
        "JSON should contain sport_type_detail value"
    );

    // Deserialize back
    let deserialized: Activity = serde_json::from_str(&json).expect("Should deserialize from JSON");

    // Verify fields roundtrip correctly
    assert_eq!(deserialized.workout_type(), Some(11));
    assert_eq!(deserialized.sport_type_detail(), Some("RoadRun"));
    assert!(deserialized.segment_efforts().is_some());
    assert_eq!(deserialized.segment_efforts().unwrap().len(), 1);

    let segment_check = &deserialized.segment_efforts().unwrap()[0];
    assert_eq!(segment_check.id, "seg_test");
    assert_eq!(segment_check.kom_rank, Some(1));
    assert_eq!(segment_check.climb_category, Some(4));
}

#[test]
fn test_activity_without_detailed_fields() {
    // Test that activities without the new fields still work (backward compatibility)
    let activity = ActivityBuilder::new(
        "basic_test",
        "Basic Activity",
        SportType::Ride,
        Utc::now(),
        3600,
        "garmin",
    )
    .distance_meters(30_000.0)
    .elevation_gain(500.0)
    .average_heart_rate(140)
    .max_heart_rate(165)
    .average_speed(8.33)
    .max_speed(12.5)
    .calories(800)
    .average_power(200)
    .max_power(400)
    .normalized_power(215)
    .ftp(250)
    .average_cadence(90)
    .max_cadence(110)
    .build();

    // Validate basic fields work
    assert_eq!(activity.id(), "basic_test");
    assert!(matches!(activity.sport_type(), SportType::Ride));
    assert_eq!(activity.provider(), "garmin");

    // Validate new fields can be None
    assert!(activity.workout_type().is_none());
    assert!(activity.sport_type_detail().is_none());
    assert!(activity.segment_efforts().is_none());

    // Serialize should still work
    let json = serde_json::to_string(&activity).expect("Should serialize");
    let deserialized: Activity = serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(deserialized.id(), activity.id());
    assert!(deserialized.workout_type().is_none());
    assert!(deserialized.segment_efforts().is_none());
}
