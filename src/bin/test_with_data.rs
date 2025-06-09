use anyhow::Result;
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing MCP Server with Real Strava Data...\n");
    
    // Connect to the MCP server
    let stream = TcpStream::connect("127.0.0.1:8080").await?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    
    // Test 1: Initialize
    println!("🔄 Initializing MCP connection...");
    let init_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": null,
        "id": 1
    });
    
    let request_str = format!("{}\n", init_request.to_string());
    writer.write_all(request_str.as_bytes()).await?;
    
    let mut response = String::new();
    reader.read_line(&mut response).await?;
    let init_response: serde_json::Value = serde_json::from_str(&response)?;
    println!("✅ Initialized! Available tools: {:?}", 
        init_response["result"]["capabilities"]["tools"].as_array().unwrap().len());
    
    // Test 2: Get Athlete Profile
    println!("\n🔄 Getting athlete profile...");
    let athlete_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_athlete",
            "arguments": {
                "provider": "strava"
            }
        },
        "id": 2
    });
    
    let request_str = format!("{}\n", athlete_request.to_string());
    writer.write_all(request_str.as_bytes()).await?;
    
    let mut response = String::new();
    reader.read_line(&mut response).await?;
    let athlete_response: serde_json::Value = serde_json::from_str(&response)?;
    
    if let Some(result) = athlete_response["result"].as_object() {
        println!("✅ Athlete Profile:");
        println!("   Name: {} {}", 
            result["firstname"].as_str().unwrap_or("N/A"),
            result["lastname"].as_str().unwrap_or("N/A"));
        println!("   Username: {}", result["username"].as_str().unwrap_or("N/A"));
        println!("   Provider: {}", result["provider"].as_str().unwrap_or("N/A"));
    } else {
        println!("❌ Error getting athlete: {}", athlete_response["error"]);
    }
    
    // Test 3: Get Recent Activities
    println!("\n🔄 Getting recent activities...");
    let activities_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_activities",
            "arguments": {
                "provider": "strava",
                "limit": 5
            }
        },
        "id": 3
    });
    
    let request_str = format!("{}\n", activities_request.to_string());
    writer.write_all(request_str.as_bytes()).await?;
    
    let mut response = String::new();
    reader.read_line(&mut response).await?;
    let activities_response: serde_json::Value = serde_json::from_str(&response)?;
    
    if let Some(activities) = activities_response["result"].as_array() {
        println!("✅ Recent Activities ({} found):", activities.len());
        for (i, activity) in activities.iter().enumerate() {
            println!("   {}. {} ({})", 
                i + 1,
                activity["name"].as_str().unwrap_or("Unknown"),
                activity["sport_type"].as_str().unwrap_or("Unknown"));
            println!("      Duration: {} seconds", 
                activity["duration_seconds"].as_u64().unwrap_or(0));
            if let Some(distance) = activity["distance_meters"].as_f64() {
                println!("      Distance: {:.2} km", distance / 1000.0);
            }
            println!("      Date: {}", activity["start_date"].as_str().unwrap_or("N/A"));
            println!();
        }
    } else {
        println!("❌ Error getting activities: {}", activities_response["error"]);
    }
    
    // Test 4: Get Stats
    println!("🔄 Getting fitness statistics...");
    let stats_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_stats",
            "arguments": {
                "provider": "strava"
            }
        },
        "id": 4
    });
    
    let request_str = format!("{}\n", stats_request.to_string());
    writer.write_all(request_str.as_bytes()).await?;
    
    let mut response = String::new();
    reader.read_line(&mut response).await?;
    let stats_response: serde_json::Value = serde_json::from_str(&response)?;
    
    if let Some(result) = stats_response["result"].as_object() {
        println!("✅ Fitness Statistics:");
        println!("   Total Activities: {}", result["total_activities"].as_u64().unwrap_or(0));
        println!("   Total Distance: {:.2} km", 
            result["total_distance"].as_f64().unwrap_or(0.0) / 1000.0);
        println!("   Total Duration: {} hours", 
            result["total_duration"].as_u64().unwrap_or(0) / 3600);
        println!("   Total Elevation: {:.0} m", 
            result["total_elevation_gain"].as_f64().unwrap_or(0.0));
    } else {
        println!("❌ Error getting stats: {}", stats_response["error"]);
    }
    
    println!("\n🎉 MCP Server test completed successfully!");
    
    Ok(())
}