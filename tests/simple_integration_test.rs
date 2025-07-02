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
        name: "Integration Test Run".to_string(),
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
        start_latitude: Some(45.5017), // Montreal
        start_longitude: Some(-73.5673),
        city: Some("Montreal".to_string()),
        region: Some("Quebec".to_string()),
        country: Some("Canada".to_string()),
        trail_name: Some("Test Trail".to_string()),
        provider: "test".to_string(),
    };

    // Analyze the activity
    let analysis = analyzer.analyze_activity(&activity, None).await?;

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
            sport_type: sport.clone(),
            ..Activity::default()
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
                id: format!("concurrent_test_{}", i),
                name: format!("Concurrent Test {}", i),
                duration_seconds: 3600 + (i as u64 * 300),
                distance_meters: Some(5000.0 + (i as f64 * 1000.0)),
                ..Activity::default()
            };

            let analyzer_local = ActivityAnalyzer::new();
            analyzer_local.analyze_activity(&activity, None).await
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
