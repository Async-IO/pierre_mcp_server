// ABOUTME: Weather API diagnostic utility for troubleshooting external weather service integration
// ABOUTME: Network connectivity and API configuration testing tool for weather services
use chrono::{Duration, Utc};
use reqwest::Client;
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 OpenWeatherMap API Diagnostics");
    println!("=================================");

    let api_key = check_api_key()?;
    let client = Client::new();
    
    test_current_weather(&client, &api_key).await?;
    test_historical_weather(&client, &api_key).await?;
    test_error_conditions(&client, &api_key).await?;
    
    println!("\n✅ All diagnostics completed successfully!");
    Ok(())
}

fn check_api_key() -> Result<String, Box<dyn std::error::Error>> {
    std::env::var("OPENWEATHER_API_KEY").map_or_else(|_| {
        println!("❌ No OPENWEATHER_API_KEY environment variable found");
        Err("Missing API key".into())
    }, |key| {
        println!(
            "✅ API Key Found: {}...{}",
            &key[..8],
            &key[key.len() - 4..]
        );
        Ok(key)
    })
}

async fn test_current_weather(client: &Client, api_key: &str) -> Result<(), Box<dyn std::error::Error>> {
    let lat = 45.5017; // Montreal
    let lon = -73.5673;

    println!("\n📍 Test Location: Montreal, Canada ({lat}, {lon})");
    println!("\n🌤️  Test 1: Current Weather API (Free)");
    println!("=====================================");

    let current_url = format!(
        "https://api.openweathermap.org/data/2.5/weather?lat={lat}&lon={lon}&appid={api_key}&units=metric"
    );

    println!("🔗 URL: {current_url}");

    match client.get(&current_url).send().await {
        Ok(response) => {
            println!("📊 Status: {}", response.status());

            if response.status().is_success() {
                match response.json::<Value>().await {
                    Ok(data) => {
                        println!("✅ Current Weather Success!");
                        if let Some(main) = data.get("main") {
                            if let Some(temp) = main.get("temp") {
                                println!("   🌡️  Temperature: {temp}°C");
                            }
                        }
                        if let Some(weather) = data.get("weather").and_then(|w| w.get(0)) {
                            if let Some(desc) = weather.get("description") {
                                println!("   🌦️  Conditions: {desc}");
                            }
                        }
                    }
                    Err(e) => println!("❌ JSON Parse Error: {e}"),
                }
            } else {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown".into());
                println!("❌ API Error: {error_text}");
            }
        }
        Err(e) => println!("❌ Network Error: {e}"),
    }
    Ok(())
}

async fn test_historical_weather(client: &Client, api_key: &str) -> Result<(), Box<dyn std::error::Error>> {
    let lat = 45.5017; // Montreal
    let lon = -73.5673;

    println!("\n📅 Test 2: Historical Weather API (Paid)");
    println!("=========================================");

    let historical_timestamp = (Utc::now() - Duration::days(7)).timestamp();
    let historical_url = format!(
        "https://api.openweathermap.org/data/3.0/onecall/timemachine?lat={lat}&lon={lon}&dt={historical_timestamp}&appid={api_key}&units=metric"
    );

    println!("🔗 URL: {historical_url}");
    println!(
        "📅 Timestamp: {historical_timestamp} ({})",
        Utc::now() - Duration::days(7)
    );

    match client.get(&historical_url).send().await {
        Ok(response) => {
            println!("📊 Status: {}", response.status());

            if response.status().is_success() {
                match response.json::<Value>().await {
                    Ok(data) => {
                        println!("✅ Historical Weather Success!");
                        if let Some(data_array) = data.get("data").and_then(|d| d.as_array()) {
                            if let Some(first_entry) = data_array.first() {
                                if let Some(temp) = first_entry.get("temp") {
                                    println!("   🌡️  Historical Temperature: {temp}°C");
                                }
                            }
                        }
                    }
                    Err(e) => println!("❌ JSON Parse Error: {e}"),
                }
            } else {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown".into());
                println!("❌ Historical API Error: {error_text}");

                analyze_error_status(status);
            }
        }
        Err(e) => println!("❌ Network Error: {e}"),
    }
    Ok(())
}

async fn test_error_conditions(client: &Client, api_key: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔍 Test 3: Error Condition Tests");
    println!("=================================");

    // Test with invalid coordinates
    let invalid_url = format!(
        "https://api.openweathermap.org/data/2.5/weather?lat=999&lon=999&appid={api_key}&units=metric"
    );

    println!("🧪 Testing invalid coordinates...");
    match client.get(&invalid_url).send().await {
        Ok(response) => {
            println!("📊 Status: {}", response.status());
            if !response.status().is_success() {
                println!("✅ Correctly rejected invalid coordinates");
            }
        }
        Err(e) => println!("❌ Network Error: {e}"),
    }

    // Test with invalid API key
    let bad_key_url = "https://api.openweathermap.org/data/2.5/weather?lat=45.5017&lon=-73.5673&appid=invalid_key&units=metric";

    println!("🧪 Testing invalid API key...");
    match client.get(&bad_key_url).send().await {
        Ok(response) => {
            println!("📊 Status: {}", response.status());
            if response.status() == 401 {
                println!("✅ Correctly rejected invalid API key");
            }
        }
        Err(e) => println!("❌ Network Error: {e}"),
    }

    Ok(())
}

fn analyze_error_status(status: reqwest::StatusCode) {
    match status.as_u16() {
        401 => {
            println!("   💡 This usually means:");
            println!("      • API key is invalid");
            println!("      • Historical data requires One Call API 3.0 subscription");
        },
        403 => {
            println!("   💡 This usually means:");
            println!("      • Historical data not included in your plan");
            println!("      • Upgrade to One Call API 3.0 required");
        },
        429 => {
            println!("   💡 Rate limit exceeded (1000/day on free tier)");
        },
        _ => {
            println!("   💡 Unexpected error code: {status}");
        }
    }
}