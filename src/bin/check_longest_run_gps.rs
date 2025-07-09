// ABOUTME: GPS data validation utility for verifying location accuracy in longest running activities
// ABOUTME: Diagnostic tool for checking GPS track quality and geographic data integrity
use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

fn initialize_connection() -> Result<(TcpStream, BufReader<TcpStream>)> {
    println!("Location  Checking GPS Coordinates for Longest Run");
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

    writeln!(stream, "{init_request}")?;

    let mut line = String::new();
    reader.read_line(&mut line)?;
    let init_response: Value = serde_json::from_str(&line)?;
    println!("Success MCP connection initialized");

    // Validate initialization response
    if let Some(result) = init_response.get("result") {
        if let Some(server_info) = result.get("serverInfo") {
            if let Some(name) = server_info.get("name") {
                println!("   Server: {name}");
            }
            if let Some(version) = server_info.get("version") {
                println!("   Version: {version}");
            }
        }
    }

    Ok((stream, reader))
}

fn fetch_activities(
    stream: &mut TcpStream,
    reader: &mut BufReader<TcpStream>,
) -> Result<Vec<Value>> {
    println!("\nData Retrieving activities...");

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

        writeln!(stream, "{activities_request}")?;
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let response: Value = serde_json::from_str(&line)?;

        if let Some(result) = response.get("result") {
            if let Some(activities) = result.as_array() {
                if activities.is_empty() {
                    break;
                }
                all_activities.extend(activities.iter().cloned());
                println!(
                    "Page Retrieved page {page} with {} activities",
                    activities.len()
                );
                page += 1;
            } else {
                break;
            }
        } else {
            println!("Error Error retrieving activities: {response:?}");
            return Ok(all_activities);
        }
    }

    Ok(all_activities)
}

fn find_longest_2025_run(all_activities: &[Value]) -> Option<&Value> {
    // Find 2025 runs
    let mut runs_2025 = Vec::new();
    for activity in all_activities {
        if let (Some(sport_type), Some(start_date)) =
            (activity.get("sport_type"), activity.get("start_date"))
        {
            if sport_type == "run" && start_date.as_str().unwrap_or("").starts_with("2025") {
                runs_2025.push(activity);
            }
        }
    }

    println!("\nRun Found {} runs in 2025", runs_2025.len());

    // Find the longest run
    runs_2025
        .iter()
        .max_by(|a, b| {
            let dist_a = a
                .get("distance_meters")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let dist_b = b
                .get("distance_meters")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            dist_a.partial_cmp(&dist_b).unwrap()
        })
        .copied()
}

async fn test_location_service(lat: f64, lon: f64) -> Result<()> {
    println!("\nTest Testing Location Service...");
    let mut location_service = pierre_mcp_server::intelligence::location::LocationService::new();

    match location_service
        .get_location_from_coordinates(lat, lon)
        .await
    {
        Ok(location_data) => {
            println!("Success Location data retrieved:");
            println!("   Location Display Name: {}", location_data.display_name);
            if let Some(city) = &location_data.city {
                println!("   City  City: {city}");
            }
            if let Some(region) = &location_data.region {
                println!("   Location  Region: {region}");
            }
            if let Some(country) = &location_data.country {
                println!("   Country Country: {country}");
            }
            if let Some(trail_name) = &location_data.trail_name {
                println!("   Trail Trail: {trail_name}");
            }
        }
        Err(e) => {
            println!("Error Failed to get location data: {e}");
            println!("   This could be due to API rate limiting or network issues");
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize connection
    let (mut stream, mut reader) = initialize_connection()?;

    // Fetch activities
    let all_activities = fetch_activities(&mut stream, &mut reader)?;

    // Find longest 2025 run
    let longest_run = find_longest_2025_run(&all_activities);

    let Some(longest_run) = longest_run else {
        println!("Error No runs found in 2025");
        return Ok(());
    };

    let distance_km = longest_run
        .get("distance_meters")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0)
        / 1000.0;

    let activity_id = longest_run
        .get("id")
        .and_then(|id| id.as_str())
        .unwrap_or("");
    let name = longest_run
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("");

    println!("\nTarget LONGEST RUN IN 2025:");
    println!("   Name Name: {name}");
    println!("   Distance Distance: {distance_km:.2} km");
    println!("   ID Activity ID: {activity_id}");

    // Check GPS coordinates
    let start_lat = longest_run
        .get("start_latitude")
        .and_then(serde_json::Value::as_f64);
    let start_lon = longest_run
        .get("start_longitude")
        .and_then(serde_json::Value::as_f64);

    if let (Some(lat), Some(lon)) = (start_lat, start_lon) {
        println!("   Location GPS Coordinates: {lat:.6}, {lon:.6}");
        println!("   Success Activity HAS GPS coordinates - location intelligence should work!");

        test_location_service(lat, lon).await?;
    } else {
        println!("   Error No GPS coordinates available for this activity");
        println!(
            "   Summary Raw activity data: {}",
            serde_json::to_string_pretty(longest_run)?
        );
    }

    Ok(())
}
