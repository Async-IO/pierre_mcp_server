// ABOUTME: Integration tests for FitnessAnalysisAgent
// ABOUTME: Tests A2A protocol communication and end-to-end analysis workflows
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use anyhow::Result;
use fitness_analyzer::a2a_client::{A2AClient, Activity};
use fitness_analyzer::analyzer::FitnessAnalyzer;
use fitness_analyzer::config::AgentConfig;
use serde_json::json;
use std::collections::HashMap;
use tokio_test;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Create a mock activity for testing
fn create_mock_activity(id: &str, sport: &str, distance: Option<f64>, duration: Option<u32>) -> Activity {
    Activity {
        id: id.to_string(),
        name: format!("Test {} Activity", sport),
        sport_type: sport.to_string(),
        distance_meters: distance,
        duration_seconds: duration,
        elevation_gain: Some(100.0),
        average_heart_rate: Some(150),
        max_heart_rate: Some(180),
        start_date: "2024-01-15T10:00:00Z".to_string(),
        provider: "strava".to_string(),
    }
}

/// Create a test config for integration tests
fn create_test_config(server_url: &str) -> AgentConfig {
    AgentConfig {
        server_url: server_url.to_string(),
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        analysis_interval_hours: 1,
        development_mode: true,
        max_activities_per_analysis: 10,
        generate_reports: false,
        report_output_dir: "/tmp/test_reports".to_string(),
    }
}

#[tokio::test]
async fn test_a2a_authentication_flow() -> Result<()> {
    let mock_server = MockServer::start().await;

    // Mock authentication endpoint
    Mock::given(method("POST"))
        .and(path("/a2a/auth"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "test_access_token_123",
            "expires_in": 3600,
            "token_type": "Bearer"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let mut client = A2AClient::new(
        mock_server.uri(),
        "test_client".to_string(),
        "test_secret".to_string(),
    );

    // Test authentication
    let result = client.authenticate().await;
    assert!(result.is_ok(), "Authentication should succeed");

    Ok(())
}

#[tokio::test]
async fn test_a2a_get_activities() -> Result<()> {
    let mock_server = MockServer::start().await;

    // Mock authentication
    Mock::given(method("POST"))
        .and(path("/a2a/auth"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "test_token",
            "expires_in": 3600,
            "token_type": "Bearer"
        })))
        .mount(&mock_server)
        .await;

    // Mock activities endpoint
    Mock::given(method("POST"))
        .and(path("/a2a/execute"))
        .and(header("authorization", "Bearer test_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "result": [
                {
                    "id": "123",
                    "name": "Morning Run",
                    "sport_type": "Run",
                    "distance_meters": 5000.0,
                    "duration_seconds": 1800,
                    "elevation_gain": 50.0,
                    "start_date": "2024-01-15T08:00:00Z",
                    "provider": "strava"
                }
            ],
            "id": "test-request-id"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let mut client = A2AClient::new(
        mock_server.uri(),
        "test_client".to_string(),
        "test_secret".to_string(),
    );

    client.authenticate().await?;
    let activities = client.get_activities("strava", 10).await?;

    assert_eq!(activities.len(), 1);
    assert_eq!(activities[0].id, "123");
    assert_eq!(activities[0].sport_type, "Run");
    assert_eq!(activities[0].distance_meters, Some(5000.0));

    Ok(())
}

#[tokio::test]
async fn test_a2a_json_rpc_error_handling() -> Result<()> {
    let mock_server = MockServer::start().await;

    // Mock authentication
    Mock::given(method("POST"))
        .and(path("/a2a/auth"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "test_token",
            "expires_in": 3600,
            "token_type": "Bearer"
        })))
        .mount(&mock_server)
        .await;

    // Mock error response
    Mock::given(method("POST"))
        .and(path("/a2a/execute"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32600,
                "message": "Invalid Request",
                "data": {"details": "Missing required parameter"}
            },
            "id": "test-request-id"
        })))
        .mount(&mock_server)
        .await;

    let mut client = A2AClient::new(
        mock_server.uri(),
        "test_client".to_string(),
        "test_secret".to_string(),
    );

    client.authenticate().await?;
    let result = client.get_activities("strava", 10).await;

    assert!(result.is_err());
    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("Invalid Request"));

    Ok(())
}

#[tokio::test]
async fn test_fitness_analyzer_with_mock_data() {
    let mock_server = MockServer::start().await;
    
    // Mock authentication
    Mock::given(method("POST"))
        .and(path("/a2a/auth"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "test_token",
            "expires_in": 3600,
            "token_type": "Bearer"
        })))
        .mount(&mock_server)
        .await;

    // Mock activities endpoint with multiple activities
    Mock::given(method("POST"))
        .and(path("/a2a/execute"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "result": [
                {
                    "id": "1", "name": "Run 1", "sport_type": "Run",
                    "distance_meters": 5000.0, "duration_seconds": 1800,
                    "start_date": "2024-01-15T08:00:00Z", "provider": "strava"
                },
                {
                    "id": "2", "name": "Run 2", "sport_type": "Run", 
                    "distance_meters": 6000.0, "duration_seconds": 2000,
                    "start_date": "2024-01-16T08:00:00Z", "provider": "strava"
                },
                {
                    "id": "3", "name": "Bike Ride", "sport_type": "Ride",
                    "distance_meters": 20000.0, "duration_seconds": 3600,
                    "start_date": "2024-01-17T09:00:00Z", "provider": "strava"
                }
            ],
            "id": "test-request-id"
        })))
        .mount(&mock_server)
        .await;

    let client = A2AClient::new(
        mock_server.uri(),
        "test_client".to_string(),
        "test_secret".to_string(),
    );

    let mut analyzer = FitnessAnalyzer::new(client);
    let results = analyzer.analyze("strava", 10).await.unwrap();

    assert_eq!(results.activities_analyzed, 3);
    assert!(results.patterns.len() > 0, "Should detect some patterns");
    assert!(results.performance_trends.overall_trend != "insufficient_data");
}

#[tokio::test]
async fn test_pattern_detection_frequency() {
    let activities = vec![
        create_mock_activity("1", "Run", Some(5000.0), Some(1800)),
        create_mock_activity("2", "Run", Some(5500.0), Some(1900)),
        create_mock_activity("3", "Run", Some(6000.0), Some(2000)),
        create_mock_activity("4", "Ride", Some(20000.0), Some(3600)),
        create_mock_activity("5", "Run", Some(5200.0), Some(1850)),
    ];

    // Create a mock client (won't be used for this test)
    let mock_client = A2AClient::new(
        "http://localhost:8081".to_string(),
        "test".to_string(),
        "test".to_string(),
    );

    let analyzer = FitnessAnalyzer::new(mock_client);
    let patterns = analyzer.detect_patterns(&activities).unwrap();

    // Should detect sport preference pattern (4/5 are runs = 80%)
    let sport_pattern = patterns.iter()
        .find(|p| p.pattern_type.contains("sport"));
    assert!(sport_pattern.is_some(), "Should detect sport pattern");

    // Should detect some frequency pattern
    let frequency_pattern = patterns.iter()
        .find(|p| p.pattern_type.contains("frequency"));
    assert!(frequency_pattern.is_some(), "Should detect frequency pattern");
}

#[tokio::test]
async fn test_risk_assessment_volume_spike() {
    // Create activities with sudden volume increase
    let mut activities = Vec::new();
    
    // Previous 2 weeks: short activities
    for i in 0..14 {
        activities.push(Activity {
            id: format!("old_{}", i),
            name: "Short Run".to_string(),
            sport_type: "Run".to_string(),
            distance_meters: Some(3000.0),
            duration_seconds: Some(1200), // 20 minutes
            elevation_gain: None,
            average_heart_rate: None,
            max_heart_rate: None,
            start_date: format!("2024-01-{:02}T08:00:00Z", i + 1),
            provider: "strava".to_string(),
        });
    }
    
    // Recent 2 weeks: long activities (volume spike)
    for i in 0..14 {
        activities.push(Activity {
            id: format!("new_{}", i),
            name: "Long Run".to_string(),
            sport_type: "Run".to_string(),
            distance_meters: Some(8000.0),
            duration_seconds: Some(3600), // 60 minutes (3x increase)
            elevation_gain: None,
            average_heart_rate: None,
            max_heart_rate: None,
            start_date: format!("2024-01-{:02}T08:00:00Z", i + 15),
            provider: "strava".to_string(),
        });
    }

    let mock_client = A2AClient::new(
        "http://localhost:8081".to_string(),
        "test".to_string(),
        "test".to_string(),
    );

    let analyzer = FitnessAnalyzer::new(mock_client);
    let risks = analyzer.assess_risks(&activities).unwrap();

    // Should detect volume spike risk
    let volume_risk = risks.iter()
        .find(|r| r.risk_type == "volume_spike");
    assert!(volume_risk.is_some(), "Should detect volume spike risk");
    
    if let Some(risk) = volume_risk {
        assert!(risk.probability > 0.5, "Volume spike risk probability should be significant");
    }
}

#[tokio::test]
async fn test_performance_trend_analysis() {
    let activities = vec![
        // Improving distances over time
        create_mock_activity("1", "Run", Some(4000.0), Some(1800)),
        create_mock_activity("2", "Run", Some(4500.0), Some(1850)),
        create_mock_activity("3", "Run", Some(5000.0), Some(1900)),
        create_mock_activity("4", "Run", Some(5500.0), Some(1950)),
        create_mock_activity("5", "Run", Some(6000.0), Some(2000)),
    ];

    let mock_client = A2AClient::new(
        "http://localhost:8081".to_string(),
        "test".to_string(),
        "test".to_string(),
    );

    let analyzer = FitnessAnalyzer::new(mock_client);
    let trends = analyzer.analyze_performance_trends(&activities).unwrap();

    assert_ne!(trends.overall_trend, "insufficient_data");
    assert!(trends.distance_trend.is_some(), "Should calculate distance trend");
    
    if let Some(distance_trend) = trends.distance_trend {
        assert!(distance_trend > 0.0, "Distance trend should be positive (increasing)");
    }
}

#[tokio::test]
async fn test_recommendation_generation() {
    let mock_server = MockServer::start().await;
    
    // Mock authentication
    Mock::given(method("POST"))
        .and(path("/a2a/auth"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "test_token",
            "expires_in": 3600,
            "token_type": "Bearer"
        })))
        .mount(&mock_server)
        .await;

    // Mock recommendations endpoint
    Mock::given(method("POST"))
        .and(path("/a2a/execute"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "result": {
                "training_recommendations": [
                    {
                        "title": "Increase Weekly Mileage",
                        "description": "Gradually increase your weekly running distance",
                        "priority": "medium"
                    }
                ]
            },
            "id": "test-request-id"
        })))
        .mount(&mock_server)
        .await;

    let client = A2AClient::new(
        mock_server.uri(),
        "test_client".to_string(),
        "test_secret".to_string(),
    );

    let mut analyzer = FitnessAnalyzer::new(client);

    // Create activities showing low frequency pattern
    let activities = vec![
        create_mock_activity("1", "Run", Some(5000.0), Some(1800)),
        create_mock_activity("2", "Run", Some(5000.0), Some(1800)),
    ];

    // Create patterns that should generate recommendations
    let patterns = vec![
        fitness_analyzer::analyzer::Pattern {
            pattern_type: "low_frequency".to_string(),
            confidence: 0.8,
            description: "Low training frequency detected".to_string(),
            supporting_data: HashMap::new(),
        }
    ];

    let recommendations = analyzer.generate_recommendations(&activities, &patterns).await.unwrap();

    assert!(!recommendations.is_empty(), "Should generate recommendations");
    
    // Should have pattern-based recommendation for low frequency
    let pattern_rec = recommendations.iter()
        .find(|r| r.category == "training_volume");
    assert!(pattern_rec.is_some(), "Should generate training volume recommendation");
}

#[tokio::test]
async fn test_config_validation() {
    let mut config = AgentConfig::default();
    
    // Should fail with empty credentials
    assert!(config.validate().is_err());
    
    // Should pass with valid config
    config.client_id = "test_id".to_string();
    config.client_secret = "test_secret".to_string();
    assert!(config.validate().is_ok());
    
    // Test interval conversion
    config.analysis_interval_hours = 24;
    let interval = config.analysis_interval();
    assert_eq!(interval.as_secs(), 24 * 3600);
    
    // Test development mode interval
    config.development_mode = true;
    let dev_interval = config.dev_analysis_interval();
    assert_eq!(dev_interval.as_secs(), 60); // 1 minute in dev mode
}

#[tokio::test]
async fn test_empty_activities_analysis() {
    let mock_client = A2AClient::new(
        "http://localhost:8081".to_string(),
        "test".to_string(),
        "test".to_string(),
    );

    let analyzer = FitnessAnalyzer::new(mock_client);
    let empty_activities: Vec<Activity> = vec![];
    
    let patterns = analyzer.detect_patterns(&empty_activities).unwrap();
    assert!(patterns.is_empty(), "No patterns should be detected for empty activities");
    
    let risks = analyzer.assess_risks(&empty_activities).unwrap();
    assert!(risks.is_empty(), "No risks should be assessed for empty activities");
    
    let trends = analyzer.analyze_performance_trends(&empty_activities).unwrap();
    assert_eq!(trends.overall_trend, "insufficient_data");
}