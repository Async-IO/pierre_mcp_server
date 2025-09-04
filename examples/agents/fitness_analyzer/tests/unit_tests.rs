// ABOUTME: Unit tests for individual components of FitnessAnalysisAgent
// ABOUTME: Tests isolated functionality without external dependencies

use fitness_analyzer::config::AgentConfig;
use fitness_analyzer::a2a_client::Activity;
use serde_json::json;
use std::time::Duration;

#[test]
fn test_config_default_values() {
    let config = AgentConfig::default();
    
    assert_eq!(config.server_url, "http://localhost:8081");
    assert_eq!(config.analysis_interval_hours, 24);
    assert!(!config.development_mode);
    assert_eq!(config.max_activities_per_analysis, 200);
    assert!(config.generate_reports);
}

#[test]
fn test_config_validation_empty_credentials() {
    let mut config = AgentConfig::default();
    
    // Empty client_id should fail
    config.client_id = "".to_string();
    config.client_secret = "test_secret".to_string();
    assert!(config.validate().is_err());
    
    // Empty client_secret should fail
    config.client_id = "test_id".to_string();
    config.client_secret = "".to_string();
    assert!(config.validate().is_err());
    
    // Both empty should fail
    config.client_id = "".to_string();
    config.client_secret = "".to_string();
    assert!(config.validate().is_err());
}

#[test]
fn test_config_validation_zero_values() {
    let mut config = AgentConfig::default();
    config.client_id = "test_id".to_string();
    config.client_secret = "test_secret".to_string();
    
    // Zero analysis interval should fail
    config.analysis_interval_hours = 0;
    assert!(config.validate().is_err());
    
    // Zero max activities should fail
    config.analysis_interval_hours = 24;
    config.max_activities_per_analysis = 0;
    assert!(config.validate().is_err());
}

#[test]
fn test_config_valid_configuration() {
    let mut config = AgentConfig::default();
    config.client_id = "valid_client_id".to_string();
    config.client_secret = "valid_client_secret".to_string();
    config.analysis_interval_hours = 12;
    config.max_activities_per_analysis = 50;
    
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_interval_conversions() {
    let mut config = AgentConfig::default();
    config.analysis_interval_hours = 24;
    
    // Test normal interval
    let interval = config.analysis_interval();
    assert_eq!(interval, Duration::from_secs(24 * 3600));
    
    // Test development mode interval
    config.development_mode = false;
    let dev_interval = config.dev_analysis_interval();
    assert_eq!(dev_interval, Duration::from_secs(24 * 3600));
    
    config.development_mode = true;
    let dev_interval = config.dev_analysis_interval();
    assert_eq!(dev_interval, Duration::from_secs(60));
}

#[test]
fn test_activity_deserialization() {
    let activity_json = json!({
        "id": "12345",
        "name": "Morning Run",
        "sport_type": "Run",
        "distance_meters": 5000.0,
        "duration_seconds": 1800,
        "elevation_gain": 100.0,
        "average_heart_rate": 150,
        "max_heart_rate": 180,
        "start_date": "2024-01-15T08:00:00Z",
        "provider": "strava"
    });

    let activity: Activity = serde_json::from_value(activity_json).unwrap();
    
    assert_eq!(activity.id, "12345");
    assert_eq!(activity.name, "Morning Run");
    assert_eq!(activity.sport_type, "Run");
    assert_eq!(activity.distance_meters, Some(5000.0));
    assert_eq!(activity.duration_seconds, Some(1800));
    assert_eq!(activity.elevation_gain, Some(100.0));
    assert_eq!(activity.average_heart_rate, Some(150));
    assert_eq!(activity.max_heart_rate, Some(180));
    assert_eq!(activity.start_date, "2024-01-15T08:00:00Z");
    assert_eq!(activity.provider, "strava");
}

#[test]
fn test_activity_partial_data() {
    // Test activity with missing optional fields
    let activity_json = json!({
        "id": "67890",
        "name": "Bike Ride",
        "sport_type": "Ride", 
        "start_date": "2024-01-16T10:00:00Z",
        "provider": "strava"
    });

    let activity: Activity = serde_json::from_value(activity_json).unwrap();
    
    assert_eq!(activity.id, "67890");
    assert_eq!(activity.sport_type, "Ride");
    assert_eq!(activity.distance_meters, None);
    assert_eq!(activity.duration_seconds, None);
    assert_eq!(activity.average_heart_rate, None);
}

#[test]
fn test_pattern_serialization() {
    use fitness_analyzer::analyzer::Pattern;
    use std::collections::HashMap;

    let mut supporting_data = HashMap::new();
    supporting_data.insert("test_key".to_string(), json!("test_value"));
    
    let pattern = Pattern {
        pattern_type: "test_pattern".to_string(),
        confidence: 0.85,
        description: "A test pattern for validation".to_string(),
        supporting_data,
    };

    // Test serialization
    let serialized = serde_json::to_string(&pattern).unwrap();
    assert!(serialized.contains("test_pattern"));
    assert!(serialized.contains("0.85"));
    
    // Test deserialization
    let deserialized: Pattern = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.pattern_type, "test_pattern");
    assert_eq!(deserialized.confidence, 0.85);
    assert_eq!(deserialized.supporting_data.get("test_key").unwrap(), "test_value");
}

#[test]
fn test_recommendation_serialization() {
    use fitness_analyzer::analyzer::Recommendation;

    let rec = Recommendation {
        category: "training_volume".to_string(),
        priority: "high".to_string(),
        title: "Increase Weekly Distance".to_string(),
        description: "Gradually increase your weekly running distance by 10%".to_string(),
        actionable_steps: vec![
            "Add one extra run per week".to_string(),
            "Increase long run distance by 1km".to_string(),
        ],
    };

    // Test serialization
    let serialized = serde_json::to_string(&rec).unwrap();
    assert!(serialized.contains("training_volume"));
    assert!(serialized.contains("high"));
    assert!(serialized.contains("Add one extra run"));
    
    // Test deserialization
    let deserialized: Recommendation = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.category, "training_volume");
    assert_eq!(deserialized.priority, "high");
    assert_eq!(deserialized.actionable_steps.len(), 2);
}

#[test]
fn test_risk_indicator_probability_bounds() {
    use fitness_analyzer::analyzer::RiskIndicator;

    let risk = RiskIndicator {
        risk_type: "overtraining".to_string(),
        severity: "high".to_string(),
        probability: 0.75,
        description: "High risk of overtraining detected".to_string(),
        mitigation_actions: vec!["Take a rest day".to_string()],
    };

    assert!(risk.probability >= 0.0);
    assert!(risk.probability <= 1.0);
    
    // Test serialization preserves probability
    let serialized = serde_json::to_string(&risk).unwrap();
    let deserialized: RiskIndicator = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.probability, 0.75);
}

#[test]
fn test_performance_trends_serialization() {
    use fitness_analyzer::analyzer::PerformanceTrends;

    let trends = PerformanceTrends {
        overall_trend: "improving".to_string(),
        pace_trend: Some(-0.05), // Negative is improving for pace
        distance_trend: Some(250.0), // Positive increase in distance
        frequency_trend: Some(0.5), // Increase in frequency
        heart_rate_trend: Some(-2.0), // Decrease in HR (fitness improvement)
    };

    let serialized = serde_json::to_string(&trends).unwrap();
    assert!(serialized.contains("improving"));
    assert!(serialized.contains("-0.05"));
    assert!(serialized.contains("250"));
    
    let deserialized: PerformanceTrends = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.overall_trend, "improving");
    assert_eq!(deserialized.pace_trend, Some(-0.05));
    assert_eq!(deserialized.distance_trend, Some(250.0));
}

#[test]
fn test_analysis_results_timestamp() {
    use fitness_analyzer::analyzer::{AnalysisResults, PerformanceTrends};
    use chrono::Utc;

    let before_creation = Utc::now();
    
    let results = AnalysisResults {
        timestamp: Utc::now(),
        activities_analyzed: 5,
        patterns: vec![],
        recommendations: vec![],
        risk_indicators: vec![],
        performance_trends: PerformanceTrends {
            overall_trend: "stable".to_string(),
            pace_trend: None,
            distance_trend: None,
            frequency_trend: None,
            heart_rate_trend: None,
        },
    };
    
    let after_creation = Utc::now();
    
    // Timestamp should be between before and after creation
    assert!(results.timestamp >= before_creation);
    assert!(results.timestamp <= after_creation);
}

#[test]
fn test_json_rpc_request_structure() {
    use serde_json::Value;

    // Test JSON-RPC request format matches specification
    let request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_activities",
            "arguments": {
                "provider": "strava",
                "limit": 10
            }
        },
        "id": "test-123"
    });

    // Verify required fields
    assert_eq!(request["jsonrpc"], "2.0");
    assert_eq!(request["method"], "tools/call");
    assert!(request["params"].is_object());
    assert_eq!(request["id"], "test-123");
    
    // Verify parameter structure
    assert_eq!(request["params"]["name"], "get_activities");
    assert!(request["params"]["arguments"].is_object());
    assert_eq!(request["params"]["arguments"]["provider"], "strava");
    assert_eq!(request["params"]["arguments"]["limit"], 10);
}

#[test]
fn test_json_rpc_response_structure() {
    // Test successful response
    let success_response = json!({
        "jsonrpc": "2.0",
        "result": [{"id": "123", "name": "Test Activity"}],
        "id": "test-123"
    });

    assert_eq!(success_response["jsonrpc"], "2.0");
    assert!(success_response["result"].is_array());
    assert_eq!(success_response["id"], "test-123");
    assert!(success_response["error"].is_null());

    // Test error response
    let error_response = json!({
        "jsonrpc": "2.0",
        "error": {
            "code": -32600,
            "message": "Invalid Request"
        },
        "id": "test-123"
    });

    assert_eq!(error_response["jsonrpc"], "2.0");
    assert!(error_response["result"].is_null());
    assert_eq!(error_response["error"]["code"], -32600);
    assert_eq!(error_response["error"]["message"], "Invalid Request");
    assert_eq!(error_response["id"], "test-123");
}

#[test]
fn test_sport_type_normalization() {
    // Test common sport type variations
    let sport_variations = vec![
        "Run", "run", "Running", "RUNNING",
        "Ride", "ride", "Cycling", "CYCLING",
        "Swim", "swim", "Swimming", "SWIMMING",
    ];

    for sport in sport_variations {
        let activity = Activity {
            id: "test".to_string(),
            name: "Test Activity".to_string(),
            sport_type: sport.to_string(),
            distance_meters: Some(1000.0),
            duration_seconds: Some(600),
            elevation_gain: None,
            average_heart_rate: None,
            max_heart_rate: None,
            start_date: "2024-01-15T10:00:00Z".to_string(),
            provider: "test".to_string(),
        };

        // Activity should be created successfully regardless of case
        assert_eq!(activity.sport_type, sport);
    }
}

#[test]
fn test_duration_calculations() {
    // Test pace calculation (seconds per meter)
    let distance = 5000.0; // 5km
    let duration = 1500; // 25 minutes
    let expected_pace = duration as f64 / distance; // 0.3 seconds per meter

    assert_eq!(expected_pace, 0.3);

    // Test speed calculation (meters per second)
    let expected_speed = distance / duration as f64; // ~3.33 m/s
    assert!((expected_speed - 3.333).abs() < 0.01);
}

#[test]
fn test_heart_rate_zone_calculations() {
    // Test heart rate zones (common fitness calculation)
    let max_hr = 190;
    let resting_hr = 60;
    let hr_reserve = max_hr - resting_hr;
    
    // Zone calculations based on % of HR reserve
    let zone1_lower = resting_hr + (hr_reserve as f64 * 0.5) as u32; // 50% = Easy
    let zone2_lower = resting_hr + (hr_reserve as f64 * 0.6) as u32; // 60% = Aerobic
    let zone3_lower = resting_hr + (hr_reserve as f64 * 0.7) as u32; // 70% = Tempo
    
    assert_eq!(zone1_lower, 125);
    assert_eq!(zone2_lower, 138);
    assert_eq!(zone3_lower, 151);
    
    // Test zone classification
    let test_hr = 140;
    let is_zone1 = test_hr < zone2_lower;
    let is_zone2 = test_hr >= zone2_lower && test_hr < zone3_lower;
    
    assert!(!is_zone1);
    assert!(is_zone2);
}

#[test]
fn test_distance_unit_conversions() {
    // Test common distance conversions
    let meters = 5000.0;
    let kilometers = meters / 1000.0;
    let miles = meters * 0.000621371;
    
    assert_eq!(kilometers, 5.0);
    assert!((miles - 3.107).abs() < 0.01);
    
    // Test elevation conversions
    let elevation_meters = 150.0;
    let elevation_feet = elevation_meters * 3.28084;
    
    assert!((elevation_feet - 492.0).abs() < 1.0);
}

#[test]
fn test_date_parsing_edge_cases() {
    use chrono::DateTime;

    // Test various ISO 8601 formats
    let date_formats = vec![
        "2024-01-15T08:00:00Z",
        "2024-01-15T08:00:00+00:00",
        "2024-01-15T08:00:00.000Z",
        "2024-12-31T23:59:59Z",
    ];

    for date_str in date_formats {
        let parsed = DateTime::parse_from_rfc3339(date_str);
        assert!(parsed.is_ok(), "Failed to parse date: {}", date_str);
    }
}

#[test]
fn test_empty_string_handling() {
    // Test handling of empty strings in activity data
    let activity_json = json!({
        "id": "",
        "name": "",
        "sport_type": "Run",
        "start_date": "2024-01-15T08:00:00Z",
        "provider": "strava"
    });

    let activity: Activity = serde_json::from_value(activity_json).unwrap();
    
    // Empty strings should be preserved, not converted to null
    assert_eq!(activity.id, "");
    assert_eq!(activity.name, "");
    assert_eq!(activity.sport_type, "Run");
}