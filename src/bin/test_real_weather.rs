// ABOUTME: Weather API testing utility for validating real-time weather data integration
// ABOUTME: Tests OpenWeatherMap API connectivity and historical weather data retrieval
use chrono::Utc;
use pierre_mcp_server::config::fitness_config::WeatherApiConfig;
use pierre_mcp_server::intelligence::weather::WeatherService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌍 Testing Real Weather API Integration");
    println!("======================================");

    check_api_key_status();

    let weather_result = test_weather_functionality().await;

    display_comparison_summary();

    weather_result
}

/// Check and display API key status
fn check_api_key_status() {
    if let Ok(key) = std::env::var("OPENWEATHER_API_KEY") {
        println!(
            "✅ API Key Found: {}...{}",
            &key[..8],
            &key[key.len() - 4..]
        );
    } else {
        println!("⚠️  No OPENWEATHER_API_KEY found - will use mock weather");
        println!("   Set with: export OPENWEATHER_API_KEY=\"your_key_here\"");
    }
}

/// Test core weather functionality including analysis and multi-location testing
async fn test_weather_functionality() -> Result<(), Box<dyn std::error::Error>> {
    // Create weather service
    let config = WeatherApiConfig::default();
    let mut weather_service =
        WeatherService::new(config, std::env::var("OPENWEATHER_API_KEY").ok());

    println!("\n📅 Testing Historical Weather Data");
    println!("==================================");

    // Test historical date (1 week ago) - Montreal coordinates
    let historical_date = Utc::now() - chrono::Duration::days(7);
    let latitude = 45.5017; // Montreal
    let longitude = -73.5673;

    println!("📍 Location: Montreal, Canada ({latitude}, {longitude})");
    println!("📅 Date: {}", historical_date.format("%Y-%m-%d %H:%M UTC"));

    println!("\n🌤️  Fetching Historical Weather...");

    match weather_service
        .get_weather_at_time(latitude, longitude, historical_date)
        .await
    {
        Ok(weather) => {
            display_weather_results(&weather, &mut weather_service, historical_date).await;
        }
        Err(e) => {
            handle_weather_error(&e);
        }
    }

    Ok(())
}

/// Display detailed weather results and run additional tests
async fn display_weather_results(
    weather: &pierre_mcp_server::intelligence::WeatherConditions,
    weather_service: &mut WeatherService,
    historical_date: chrono::DateTime<chrono::Utc>,
) {
    println!("✅ Real Weather Data Retrieved:");
    println!("   🌡️  Temperature: {:.1}°C", weather.temperature_celsius);
    println!("   🌦️  Conditions: {}", weather.conditions);

    if let Some(humidity) = weather.humidity_percentage {
        println!("   💧 Humidity: {humidity:.1}%");
    }

    if let Some(wind_speed) = weather.wind_speed_kmh {
        println!("   💨 Wind Speed: {wind_speed:.1} km/h");
    }

    // Analyze weather impact
    println!("\n📊 Weather Impact Analysis:");
    let impact = weather_service.analyze_weather_impact(weather);
    println!("   🎯 Difficulty Level: {:?}", impact.difficulty_level);
    println!(
        "   📈 Performance Adjustment: {:.1}%",
        impact.performance_adjustment
    );

    if !impact.impact_factors.is_empty() {
        println!("   🔍 Impact Factors:");
        for factor in &impact.impact_factors {
            println!("     • {factor}");
        }
    }

    test_multiple_locations(weather_service, historical_date).await;
}

/// Test weather service with multiple global locations
async fn test_multiple_locations(
    weather_service: &mut WeatherService,
    historical_date: chrono::DateTime<chrono::Utc>,
) {
    println!("\n🌐 Testing Different Locations");
    println!("==============================");

    let locations = vec![
        (40.7128, -74.0060, "New York, USA"),
        (51.5074, -0.1278, "London, UK"),
        (35.6762, 139.6503, "Tokyo, Japan"),
    ];

    for (lat, lon, city) in locations {
        println!("\n📍 Testing: {city}");
        match weather_service
            .get_weather_at_time(lat, lon, historical_date)
            .await
        {
            Ok(weather) => {
                println!(
                    "   🌡️  {:.1}°C, {}",
                    weather.temperature_celsius, weather.conditions
                );
            }
            Err(e) => {
                println!("   ❌ Failed: {e}");
            }
        }
    }
}

/// Handle and display weather API errors with troubleshooting guidance
fn handle_weather_error(e: &pierre_mcp_server::intelligence::weather::WeatherError) {
    println!("❌ Weather API Error: {e}");
    println!("\n🔍 Troubleshooting:");

    if e.to_string().contains("API key") {
        println!("   • Check OPENWEATHER_API_KEY environment variable");
        println!("   • Verify API key is correct");
        println!("   • Ensure you have an OpenWeatherMap account");
    } else if e.to_string().contains("401") {
        println!("   • API key is invalid or expired");
        println!("   • Check your OpenWeatherMap account status");
    } else if e.to_string().contains("429") {
        println!("   • Rate limit exceeded (1000 calls/day on free tier)");
        println!("   • Wait or upgrade your plan");
    } else {
        println!("   • Network connectivity issue");
        println!("   • OpenWeatherMap service might be down");
    }

    println!("\n❌ Weather API failed and no fallback available");
    println!("   Configure weather API properly for production use");
}

/// Display comparison between real and mock weather features
fn display_comparison_summary() {
    println!("\n✨ Real vs Mock Weather Comparison");
    println!("==================================");
    println!("📊 Real Weather Features:");
    println!("   • Actual historical data from weather stations");
    println!("   • Precise temperature, humidity, wind measurements");
    println!("   • Real precipitation and weather conditions");
    println!("   • Location-specific climate patterns");

    println!("\n🎭 Mock Weather Features:");
    println!("   • Realistic seasonal patterns");
    println!("   • Time-of-day temperature variations");
    println!("   • Varied weather conditions");
    println!("   • No API calls or rate limits");

    println!("\n🎯 Recommendation:");
    if std::env::var("OPENWEATHER_API_KEY").is_ok() {
        println!("   ✅ Use real weather for production (you have API key)");
        println!("   💡 Monitor API usage: 1000 calls/day free limit");
    } else {
        println!("   🎭 Mock weather works great for development");
        println!("   💡 Get free API key at: https://openweathermap.org/api");
    }
}
