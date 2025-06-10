// Test weather integration with real activity data
use pierre_mcp_server::intelligence::weather::WeatherService;
use pierre_mcp_server::config::fitness_config::WeatherApiConfig;
use pierre_mcp_server::models::{Activity, SportType};
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌦️  Testing Weather Integration");
    println!("================================");
    
    // Create test activity with GPS coordinates (Montreal)
    let activity = Activity {
        id: "test_weather".to_string(),
        name: "Test Weather Integration".to_string(),
        sport_type: SportType::Run,
        start_date: Utc::now(),
        duration_seconds: 3600, // 1 hour
        distance_meters: Some(10000.0), // 10km
        elevation_gain: Some(100.0),
        average_heart_rate: Some(155),
        max_heart_rate: Some(180),
        average_speed: Some(2.78), // ~10 km/h
        max_speed: Some(4.17),
        calories: Some(500),
        start_latitude: Some(45.5017), // Montreal
        start_longitude: Some(-73.5673),
        provider: "test".to_string(),
    };
    
    // Test with default configuration (will use mock weather)
    println!("\n📍 Activity Location: Montreal, Canada ({}, {})", 
             activity.start_latitude.unwrap(), 
             activity.start_longitude.unwrap());
    
    // Create weather service with default config
    let config = WeatherApiConfig::default();
    let mut weather_service = WeatherService::new(config);
    
    println!("\n🔧 Weather Service Configuration:");
    println!("   Provider: {}", weather_service.get_config().provider);
    println!("   Enabled: {}", weather_service.get_config().enabled);
    println!("   Fallback to Mock: {}", weather_service.get_config().fallback_to_mock);
    
    // Test weather retrieval
    println!("\n🌤️  Fetching Weather Data...");
    
    match weather_service.get_weather_for_activity(
        activity.start_latitude,
        activity.start_longitude,
        activity.start_date
    ).await {
        Ok(Some(weather)) => {
            println!("✅ Weather Data Retrieved:");
            println!("   Temperature: {:.1}°C", weather.temperature_celsius);
            println!("   Conditions: {}", weather.conditions);
            
            if let Some(humidity) = weather.humidity_percentage {
                println!("   Humidity: {:.1}%", humidity);
            }
            
            if let Some(wind_speed) = weather.wind_speed_kmh {
                println!("   Wind Speed: {:.1} km/h", wind_speed);
            }
            
            // Test weather impact analysis
            println!("\n📊 Weather Impact Analysis:");
            let impact = weather_service.analyze_weather_impact(&weather);
            println!("   Difficulty Level: {:?}", impact.difficulty_level);
            println!("   Performance Adjustment: {:.1}%", impact.performance_adjustment);
            
            if !impact.impact_factors.is_empty() {
                println!("   Impact Factors:");
                for factor in &impact.impact_factors {
                    println!("     • {}", factor);
                }
            }
        }
        Ok(None) => {
            println!("ℹ️  No weather data available (missing GPS coordinates)");
        }
        Err(e) => {
            println!("⚠️  Weather fetch failed: {}", e);
            println!("   This is expected if OPENWEATHER_API_KEY is not set");
            println!("   The system should fall back to mock weather data");
        }
    }
    
    // Test mock weather generation
    println!("\n🎭 Mock Weather Generation:");
    let mock_weather = weather_service.generate_mock_weather();
    println!("   Mock Temperature: {:.1}°C", mock_weather.temperature_celsius);
    println!("   Mock Conditions: {}", mock_weather.conditions);
    
    println!("\n✅ Weather Integration Test Complete!");
    println!("\n📝 Next Steps:");
    println!("   1. Set OPENWEATHER_API_KEY environment variable for real weather");
    println!("   2. Test with real Strava activities using: cargo run --bin test-with-data");
    println!("   3. Check activity intelligence includes weather context");
    
    Ok(())
}