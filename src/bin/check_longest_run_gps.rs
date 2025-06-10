use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🗺️  Checking GPS Coordinates for Longest Run");
    println!("============================================");
    
    // Connect to MCP server
    let mut stream = TcpStream::connect("127.0.0.1:8080")?;
    let mut reader = BufReader::new(stream.try_clone()?);
    
    // Send initialize request
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "clientInfo": {
                "name": "gps-check-client",
                "version": "0.1.0"
            }
        }
    });
    
    writeln!(stream, "{}", init_request)?;
    
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let _init_response: Value = serde_json::from_str(&line)?;
    println!("✅ MCP connection initialized");
    
    // Get activities to find the longest 2025 run
    println!("\n📊 Retrieving activities...");
    
    let mut all_activities: Vec<Value> = Vec::new();
    let mut page = 1;
    let limit = 50;
    
    // Get first few pages to find the longest run
    while page <= 3 {
        let offset = (page - 1) * limit;
        let activities_request = json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "get_activities",
                "arguments": {
                    "provider": "strava",
                    "limit": limit,
                    "offset": offset
                }
            },
            "id": page + 1
        });
        
        writeln!(stream, "{}", activities_request)?;
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let response: Value = serde_json::from_str(&line)?;
        
        if let Some(result) = response.get("result") {
            if let Some(activities) = result.as_array() {
                if activities.is_empty() {
                    break;
                }
                all_activities.extend(activities.clone());
                println!("📄 Retrieved page {} with {} activities", page, activities.len());
                page += 1;
            } else {
                break;
            }
        } else {
            println!("❌ Error retrieving activities: {:?}", response);
            return Ok(());
        }
    }
    
    // Find 2025 runs
    let mut runs_2025 = Vec::new();
    for activity in &all_activities {
        if let (Some(sport_type), Some(start_date)) = 
            (activity.get("sport_type"), activity.get("start_date")) {
            if sport_type == "run" && start_date.as_str().unwrap_or("").starts_with("2025") {
                runs_2025.push(activity);
            }
        }
    }
    
    println!("\n🏃 Found {} runs in 2025", runs_2025.len());
    
    // Find the longest run
    let longest_run = runs_2025.iter()
        .max_by(|a, b| {
            let dist_a = a.get("distance_meters").and_then(|d| d.as_f64()).unwrap_or(0.0);
            let dist_b = b.get("distance_meters").and_then(|d| d.as_f64()).unwrap_or(0.0);
            dist_a.partial_cmp(&dist_b).unwrap()
        })
        .unwrap();
    
    let distance_km = longest_run.get("distance_meters")
        .and_then(|d| d.as_f64())
        .unwrap_or(0.0) / 1000.0;
    
    let activity_id = longest_run.get("id").and_then(|id| id.as_str()).unwrap_or("");
    let name = longest_run.get("name").and_then(|n| n.as_str()).unwrap_or("");
    
    println!("\n🎯 LONGEST RUN IN 2025:");
    println!("   📛 Name: {}", name);
    println!("   📏 Distance: {:.2} km", distance_km);
    println!("   🆔 Activity ID: {}", activity_id);
    
    // Check GPS coordinates
    let start_lat = longest_run.get("start_latitude").and_then(|lat| lat.as_f64());
    let start_lon = longest_run.get("start_longitude").and_then(|lon| lon.as_f64());
    
    match (start_lat, start_lon) {
        (Some(lat), Some(lon)) => {
            println!("   📍 GPS Coordinates: {:.6}, {:.6}", lat, lon);
            println!("   ✅ Activity HAS GPS coordinates - location intelligence should work!");
            
            // Test location service directly
            println!("\n🧪 Testing Location Service...");
            let mut location_service = pierre_mcp_server::intelligence::location::LocationService::new();
            
            match location_service.get_location_from_coordinates(lat, lon).await {
                Ok(location_data) => {
                    println!("✅ Location data retrieved:");
                    println!("   📍 Display Name: {}", location_data.display_name);
                    if let Some(city) = &location_data.city {
                        println!("   🏙️  City: {}", city);
                    }
                    if let Some(region) = &location_data.region {
                        println!("   🗺️  Region: {}", region);
                    }
                    if let Some(country) = &location_data.country {
                        println!("   🌍 Country: {}", country);
                    }
                    if let Some(trail_name) = &location_data.trail_name {
                        println!("   🥾 Trail: {}", trail_name);
                    }
                }
                Err(e) => {
                    println!("❌ Failed to get location data: {}", e);
                    println!("   This could be due to API rate limiting or network issues");
                }
            }
        }
        _ => {
            println!("   ❌ No GPS coordinates available for this activity");
            println!("   📝 Raw activity data: {}", serde_json::to_string_pretty(longest_run)?);
        }
    }
    
    Ok(())
}