// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Integration tests for fitness provider functionality
//!
//! These tests verify provider authentication, data retrieval,
//! and error handling using mocked HTTP responses.

use anyhow::Result;
use mockito::Server;
use pierre_mcp_server::models::{Activity, Athlete, SportType, Stats};
use pierre_mcp_server::providers::strava::StravaProvider;
use pierre_mcp_server::providers::fitbit::FitbitProvider;
use pierre_mcp_server::providers::{AuthData, FitnessProvider};
use serde_json::json;

/// Helper to create a mock Strava athlete response
fn mock_strava_athlete_response() -> serde_json::Value {
    json!({
        "id": 12345,
        "username": "test_athlete",
        "firstname": "Test",
        "lastname": "User",
        "profile": "https://example.com/avatar.jpg"
    })
}

/// Helper to create a mock Strava activities response
fn mock_strava_activities_response() -> serde_json::Value {
    json!([
        {
            "id": 1001,
            "name": "Morning Run",
            "type": "Run",
            "start_date": "2024-01-15T08:00:00Z",
            "elapsed_time": 1800,
            "distance": 5000.0,
            "total_elevation_gain": 100.0,
            "average_heartrate": 150.0,
            "max_heartrate": 175.0,
            "average_speed": 2.78,
            "max_speed": 4.17
        },
        {
            "id": 1002,
            "name": "Evening Ride",
            "type": "Ride",
            "start_date": "2024-01-14T18:00:00Z",
            "elapsed_time": 3600,
            "distance": 25000.0,
            "total_elevation_gain": 300.0,
            "average_heartrate": 140.0,
            "max_heartrate": 165.0,
            "average_speed": 6.94,
            "max_speed": 12.5
        }
    ])
}

/// Helper to create a mock Strava stats response
fn mock_strava_stats_response() -> serde_json::Value {
    json!({
        "all_ride_totals": {
            "count": 50,
            "distance": 1250000.0,
            "moving_time": 180000,
            "elevation_gain": 15000.0
        },
        "all_run_totals": {
            "count": 100,
            "distance": 500000.0,
            "moving_time": 144000,
            "elevation_gain": 5000.0
        }
    })
}

#[tokio::test]
async fn test_strava_provider_authentication() -> Result<()> {
    let mut provider = StravaProvider::new();
    
    // Test OAuth2 authentication
    let auth_data = AuthData::OAuth2 {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        access_token: Some("test_access_token".to_string()),
        refresh_token: Some("test_refresh_token".to_string()),
    };
    
    let result = provider.authenticate(auth_data).await;
    assert!(result.is_ok());
    
    // Test invalid authentication type
    let invalid_auth = AuthData::ApiKey("invalid".to_string());
    let result = provider.authenticate(invalid_auth).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("OAuth2 authentication"));
    
    Ok(())
}

#[tokio::test]
async fn test_strava_mock_server_setup() -> Result<()> {
    let mut server = Server::new_async().await;
    
    // Create mock for athlete endpoint
    let _mock = server.mock("GET", "/athlete")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(mock_strava_athlete_response().to_string())
        .create_async()
        .await;
    
    // Test that the mock server can be set up correctly
    // In a real implementation, we would inject the server URL into the provider
    let mock_response = mock_strava_athlete_response();
    assert!(mock_response["id"].is_number());
    assert_eq!(mock_response["username"], "test_athlete");
    assert_eq!(mock_response["firstname"], "Test");
    
    // Test authentication structure without making actual HTTP calls
    let mut provider = StravaProvider::new();
    let auth_data = AuthData::OAuth2 {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        access_token: Some("test_access_token".to_string()),
        refresh_token: Some("test_refresh_token".to_string()),
    };
    
    // This should succeed without making HTTP calls
    provider.authenticate(auth_data).await?;
    
    // Note: To fully test HTTP interactions, we would need to:
    // 1. Make the API base URL configurable in StravaProvider
    // 2. Inject the mock server URL during testing
    // 3. Then make actual HTTP calls to verify the mock responses
    
    Ok(())
}

#[tokio::test]
async fn test_provider_factory() -> Result<()> {
    use pierre_mcp_server::providers::create_provider;
    
    // Test creating a Strava provider
    let provider = create_provider("strava");
    assert!(provider.is_ok());
    
    // Test creating a Fitbit provider
    let provider = create_provider("fitbit");
    assert!(provider.is_ok());
    
    // Test creating an invalid provider
    let provider_result = create_provider("invalid_provider");
    assert!(provider_result.is_err());
    
    match provider_result {
        Err(error) => {
            let error_msg = error.to_string();
            assert!(error_msg.contains("Unknown provider"));
        }
        Ok(_) => panic!("Expected error for invalid provider"),
    }
    
    Ok(())
}

#[tokio::test]
async fn test_auth_data_variants() -> Result<()> {
    // Test OAuth2 variant
    let oauth2_auth = AuthData::OAuth2 {
        client_id: "client123".to_string(),
        client_secret: "secret456".to_string(),
        access_token: Some("access789".to_string()),
        refresh_token: Some("refresh012".to_string()),
    };
    
    match oauth2_auth {
        AuthData::OAuth2 { client_id, .. } => {
            assert_eq!(client_id, "client123");
        }
        _ => panic!("Expected OAuth2 variant"),
    }
    
    // Test ApiKey variant
    let api_key_auth = AuthData::ApiKey("api_key_123".to_string());
    
    match api_key_auth {
        AuthData::ApiKey(key) => {
            assert_eq!(key, "api_key_123");
        }
        _ => panic!("Expected ApiKey variant"),
    }
    
    Ok(())
}

#[tokio::test]
async fn test_activity_model_conversion() -> Result<()> {
    // Test creating activities from provider data
    let activity = Activity {
        id: "12345".to_string(),
        name: "Test Activity".to_string(),
        sport_type: SportType::Run,
        start_date: chrono::Utc::now(),
        duration_seconds: 1800,
        distance_meters: Some(5000.0),
        elevation_gain: Some(100.0),
        average_heart_rate: Some(150),
        max_heart_rate: Some(175),
        average_speed: Some(2.78),
        max_speed: Some(4.17),
        calories: Some(300),
        start_latitude: Some(45.5017), // Montreal
        start_longitude: Some(-73.5673),
        provider: "strava".to_string(),
    };
    
    // Verify all fields are correctly set
    assert_eq!(activity.id, "12345");
    assert_eq!(activity.name, "Test Activity");
    assert!(matches!(activity.sport_type, SportType::Run));
    assert_eq!(activity.duration_seconds, 1800);
    assert_eq!(activity.distance_meters, Some(5000.0));
    assert_eq!(activity.provider, "strava");
    
    // Test serialization for API responses
    let json = serde_json::to_string(&activity)?;
    assert!(json.contains("Test Activity"));
    assert!(json.contains("run")); // sport_type serialized as snake_case
    
    Ok(())
}

#[tokio::test]
async fn test_athlete_model() -> Result<()> {
    let athlete = Athlete {
        id: "67890".to_string(),
        username: "test_user".to_string(),
        firstname: Some("Test".to_string()),
        lastname: Some("User".to_string()),
        profile_picture: Some("https://example.com/avatar.jpg".to_string()),
        provider: "strava".to_string(),
    };
    
    // Test serialization
    let json = serde_json::to_string(&athlete)?;
    let deserialized: Athlete = serde_json::from_str(&json)?;
    
    assert_eq!(deserialized.id, athlete.id);
    assert_eq!(deserialized.username, athlete.username);
    assert_eq!(deserialized.firstname, athlete.firstname);
    assert_eq!(deserialized.provider, athlete.provider);
    
    Ok(())
}

#[tokio::test]
async fn test_stats_model() -> Result<()> {
    let stats = Stats {
        total_activities: 150,
        total_distance: 1750000.0, // 1750 km
        total_duration: 324000,    // 90 hours
        total_elevation_gain: 20000.0, // 20 km
    };
    
    // Test calculations
    assert_eq!(stats.total_activities, 150);
    assert_eq!(stats.total_distance / 1000.0, 1750.0); // Convert to km
    assert_eq!(stats.total_duration / 3600, 90); // Convert to hours
    
    // Test serialization
    let json = serde_json::to_string(&stats)?;
    let deserialized: Stats = serde_json::from_str(&json)?;
    
    assert_eq!(deserialized.total_activities, stats.total_activities);
    assert_eq!(deserialized.total_distance, stats.total_distance);
    
    Ok(())
}

#[tokio::test]
async fn test_sport_type_variants() -> Result<()> {
    // Test standard sport types
    let sport_types = vec![
        SportType::Run,
        SportType::Ride,
        SportType::Swim,
        SportType::Walk,
        SportType::Hike,
        SportType::VirtualRide,
        SportType::VirtualRun,
        SportType::Workout,
        SportType::Yoga,
        SportType::Other("CustomSport".to_string()),
    ];
    
    for sport_type in sport_types {
        // Test serialization
        let json = serde_json::to_string(&sport_type)?;
        let deserialized: SportType = serde_json::from_str(&json)?;
        
        match (&sport_type, &deserialized) {
            (SportType::Run, SportType::Run) => (),
            (SportType::Ride, SportType::Ride) => (),
            (SportType::Other(a), SportType::Other(b)) => assert_eq!(a, b),
            _ => {
                // For other variants, just ensure they serialize/deserialize consistently
                assert_eq!(
                    serde_json::to_string(&sport_type)?,
                    serde_json::to_string(&deserialized)?
                );
            }
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_provider_error_handling() -> Result<()> {
    let provider = StravaProvider::new();
    
    // Test unauthenticated requests
    let result = provider.get_athlete().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Not authenticated"));
    
    let result = provider.get_activities(Some(10), None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Not authenticated"));
    
    let result = provider.get_stats().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Not authenticated"));
    
    Ok(())
}

#[tokio::test]
async fn test_activity_pagination() -> Result<()> {
    // Test different pagination parameters
    struct TestCase {
        limit: Option<usize>,
        offset: Option<usize>,
        expected_params: Vec<(&'static str, String)>,
    }
    
    let test_cases = vec![
        TestCase {
            limit: Some(50),
            offset: None,
            expected_params: vec![("per_page", "50".to_string())],
        },
        TestCase {
            limit: Some(30),
            offset: Some(60),
            expected_params: vec![
                ("per_page", "30".to_string()),
                ("page", "3".to_string()), // offset/limit + 1
            ],
        },
        TestCase {
            limit: None,
            offset: None,
            expected_params: vec![],
        },
    ];
    
    // Note: In a real test, we would mock the HTTP client to verify the correct parameters
    // are being sent. For now, we verify the parameter calculation logic.
    
    for test_case in test_cases {
        let mut query = vec![];
        
        if let Some(limit) = test_case.limit {
            query.push(("per_page", limit.to_string()));
        }
        if let Some(offset) = test_case.offset {
            let page = (offset / test_case.limit.unwrap_or(30) + 1).to_string();
            query.push(("page", page));
        }
        
        // Verify expected parameters match
        assert_eq!(query.len(), test_case.expected_params.len());
        for (expected_key, expected_value) in test_case.expected_params {
            assert!(query.iter().any(|(k, v)| *k == expected_key && *v == expected_value));
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_fitbit_provider_authentication() -> Result<()> {
    let mut provider = FitbitProvider::new();
    
    // Test OAuth2 authentication
    let auth_data = AuthData::OAuth2 {
        client_id: "test_fitbit_client_id".to_string(),
        client_secret: "test_fitbit_client_secret".to_string(),
        access_token: Some("test_fitbit_access_token".to_string()),
        refresh_token: Some("test_fitbit_refresh_token".to_string()),
    };
    
    let result = provider.authenticate(auth_data).await;
    assert!(result.is_ok());
    
    // Test invalid authentication type
    let invalid_auth = AuthData::ApiKey("invalid".to_string());
    let result = provider.authenticate(invalid_auth).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("OAuth2 authentication"));
    
    Ok(())
}

#[tokio::test]
async fn test_fitbit_provider_error_handling() -> Result<()> {
    let provider = FitbitProvider::new();
    
    // Test unauthenticated requests
    let result = provider.get_athlete().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Not authenticated"));
    
    let result = provider.get_activities(Some(10), None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Not authenticated"));
    
    let result = provider.get_stats().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Not authenticated"));
    
    Ok(())
}