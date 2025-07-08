// ABOUTME: Weather integration testing utility for validating environmental data correlation with activities
// ABOUTME: Integration testing tool for weather API connectivity and activity-weather data correlation
use chrono::Utc;
use pierre_mcp_server::config::fitness_config::WeatherApiConfig;
use pierre_mcp_server::intelligence::weather::WeatherService;
use pierre_mcp_server::models::{Activity, SportType};

fn create_test_activity() -> Activity {
    Activity {
        id: "test_weather".into(),
        name: "Test Weather Integration".into(),
        sport_type: SportType::Run,
        start_date: Utc::now(),
        duration_seconds: 3600,         // 1 hour
        distance_meters: Some(10000.0), // 10km
        elevation_gain: Some(100.0),
        average_heart_rate: Some(155),
        max_heart_rate: Some(180),
        average_speed: Some(2.78), // ~10 km/h
        max_speed: Some(4.17),
        calories: Some(500),
        steps: None,
        heart_rate_zones: None,

        // Advanced metrics (all None for test)
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
        provider: "test".into(),
    }
}

async fn test_weather_service(activity: &Activity) -> Result<(), Box<dyn std::error::Error>> {
    // Test with default configuration (will use mock weather)
    println!(
        "\nLocation Activity Location: Montreal, Canada ({}, {})",
        activity.start_latitude.unwrap(),
        activity.start_longitude.unwrap()
    );

    // Create weather service with default config
    let config = WeatherApiConfig::default();
    let mut weather_service =
        WeatherService::new(config, std::env::var("OPENWEATHER_API_KEY").ok());

    println!("\nTool Weather Service Configuration:");
    println!("   Provider: {}", weather_service.get_config().provider);
    println!("   Enabled: {}", weather_service.get_config().enabled);
    println!(
        "   Cache Duration: {} hours",
        weather_service.get_config().cache_duration_hours
    );

    // Test weather retrieval
    println!("\nWeather  Fetching Weather Data...");

    match weather_service
        .get_weather_for_activity(
            activity.start_latitude,
            activity.start_longitude,
            activity.start_date,
        )
        .await
    {
        Ok(Some(weather)) => {
            println!("Success Weather Data Retrieved:");
            println!("   Temperature: {:.1}°C", weather.temperature_celsius);
            println!("   Conditions: {}", weather.conditions);

            if let Some(humidity) = weather.humidity_percentage {
                println!("   Humidity: {humidity:.1}%");
            }

            if let Some(wind_speed) = weather.wind_speed_kmh {
                println!("   Wind Speed: {wind_speed:.1} km/h");
            }

            // Test weather impact analysis
            println!("\nData Weather Impact Analysis:");
            let impact = weather_service.analyze_weather_impact(&weather);
            println!("   Difficulty Level: {:?}", impact.difficulty_level);
            println!(
                "   Performance Adjustment: {:.1}%",
                impact.performance_adjustment
            );

            if !impact.impact_factors.is_empty() {
                println!("   Impact Factors:");
                for factor in &impact.impact_factors {
                    println!("     • {factor}");
                }
            }
        }
        Ok(None) => {
            println!("Info  No weather data available (missing GPS coordinates)");
        }
        Err(e) => {
            println!("Warning  Weather fetch failed: {e}");
            println!("   This is expected if OPENWEATHER_API_KEY is not set or API is disabled");
        }
    }

    println!("\nSuccess Weather Integration Test Complete!");
    println!("\nSummary Next Steps:");
    println!("   1. Set OPENWEATHER_API_KEY environment variable for real weather");
    println!("   2. Test with real Strava activities using: cargo run --bin test-with-data");
    println!("   3. Check activity intelligence includes weather context");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Weather  Testing Weather Integration");
    println!("================================");

    let activity = create_test_activity();
    test_weather_service(&activity).await?;

    Ok(())
}
