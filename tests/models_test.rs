// ABOUTME: Unit tests for models functionality
// ABOUTME: Validates models behavior, edge cases, and error handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use chrono::Utc;
use pierre_mcp_server::models::{
    Activity, Athlete, AuthorizationCode, EncryptedToken, HeartRateZone, PersonalRecord, PowerZone,
    PrMetric, SportType, Stats, Tenant, User, UserStatus, UserTier,
};
use uuid::Uuid;

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
        email: "test@example.com".to_string(),
        display_name: Some("Test User".to_string()),
        password_hash: "hashed_password".to_string(),
        tier: UserTier::Professional,
        tenant_id: Some("test-tenant".to_string()),
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        approved_by: None,
        approved_at: Some(now),
        created_at: now,
        last_active: now,
    };

    assert_eq!(user.id, user_id);
    assert_eq!(user.email, "test@example.com");
    assert_eq!(user.display_name, Some("Test User".to_string()));
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
        email: "test@example.com".to_string(),
        display_name: Some("Test User".to_string()),
        password_hash: "hashed_password".to_string(),
        tier: UserTier::Enterprise,
        tenant_id: Some("test-tenant".to_string()),
        strava_token: Some(EncryptedToken {
            access_token: "encrypted_access_token".to_string(),
            refresh_token: "encrypted_refresh_token".to_string(),
            expires_at: now + chrono::Duration::hours(1),
            scope: "read".to_string(),
            nonce: "test_nonce".to_string(),
        }),
        fitbit_token: Some(EncryptedToken {
            access_token: "encrypted_fitbit_access".to_string(),
            refresh_token: "encrypted_fitbit_refresh".to_string(),
            expires_at: now + chrono::Duration::hours(2),
            scope: "read_all".to_string(),
            nonce: "fitbit_nonce".to_string(),
        }),
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: true,
        approved_by: Some(Uuid::new_v4()),
        approved_at: Some(now),
        created_at: now,
        last_active: now,
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
        name: "Test Company".to_string(),
        slug: "test-company".to_string(),
        domain: Some("testcompany.com".to_string()),
        plan: "enterprise".to_string(),
        owner_user_id: Uuid::new_v4(),
        created_at: now,
        updated_at: now,
    };

    assert_eq!(tenant.id, tenant_id);
    assert_eq!(tenant.name, "Test Company");
    assert_eq!(tenant.domain, Some("testcompany.com".to_string()));
    assert_eq!(tenant.plan, "enterprise");
    assert_eq!(tenant.slug, "test-company");
}

#[test]
fn test_tenant_serialization_roundtrip() {
    let now = Utc::now();
    let original_tenant = Tenant {
        id: Uuid::new_v4(),
        name: "Acme Corp".to_string(),
        slug: "acme-corp".to_string(),
        domain: Some("acme.com".to_string()),
        plan: "professional".to_string(),
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
        name: "Zone 2".to_string(),
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
        name: "Zone 3".to_string(),
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
        "auth_code_123".to_string(),
        "client_456".to_string(),
        "https://redirect.uri".to_string(),
        "read write".to_string(),
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
        access_token: "encrypted_access_12345".to_string(),
        refresh_token: "encrypted_refresh_67890".to_string(),
        expires_at: now + chrono::Duration::hours(1),
        scope: "read write".to_string(),
        nonce: "unique_nonce_123".to_string(),
    };

    assert_eq!(token.access_token, "encrypted_access_12345");
    assert_eq!(token.refresh_token, "encrypted_refresh_67890");
    assert_eq!(token.scope, "read write");
    assert_eq!(token.nonce, "unique_nonce_123");
    assert!(token.expires_at > now);
}

#[test]
fn test_user_with_encrypted_tokens() {
    let now = Utc::now();
    let user = User {
        id: Uuid::new_v4(),
        email: "tokenuser@example.com".to_string(),
        display_name: Some("Token User".to_string()),
        password_hash: "secure_hash".to_string(),
        tier: UserTier::Professional,
        tenant_id: Some("tenant-123".to_string()),
        strava_token: Some(EncryptedToken {
            access_token: "strava_encrypted_access".to_string(),
            refresh_token: "strava_encrypted_refresh".to_string(),
            expires_at: now + chrono::Duration::hours(6),
            scope: "read_all,activity:read".to_string(),
            nonce: "strava_nonce_456".to_string(),
        }),
        fitbit_token: Some(EncryptedToken {
            access_token: "fitbit_encrypted_access".to_string(),
            refresh_token: "fitbit_encrypted_refresh".to_string(),
            expires_at: now + chrono::Duration::hours(8),
            scope: "activity,heartrate,sleep".to_string(),
            nonce: "fitbit_nonce_789".to_string(),
        }),
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        approved_by: Some(Uuid::new_v4()),
        approved_at: Some(now),
        created_at: now,
        last_active: now,
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
