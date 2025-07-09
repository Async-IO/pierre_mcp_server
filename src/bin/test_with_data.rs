// ABOUTME: Comprehensive testing utility for validating server functionality with real fitness data
// ABOUTME: End-to-end testing tool for MCP protocol, activity analysis, and data processing workflows
use anyhow::Result;
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

async fn test_initialize(
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
) -> Result<()> {
    println!("🔄 Initializing MCP connection...");
    let init_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": null,
        "id": 1
    });

    let request_str = format!("{init_request}\n");
    writer.write_all(request_str.as_bytes()).await?;

    let mut response = String::new();
    reader.read_line(&mut response).await?;
    let init_response: serde_json::Value = serde_json::from_str(&response)?;
    let tools_count = init_response["result"]["capabilities"]["tools"]
        .as_array()
        .map_or(0, std::vec::Vec::len);
    println!("Success Initialized! Available tools: {tools_count}");
    Ok(())
}

async fn test_athlete_profile(
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
) -> Result<()> {
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

    let request_str = format!("{athlete_request}\n");
    writer.write_all(request_str.as_bytes()).await?;

    let mut response = String::new();
    reader.read_line(&mut response).await?;
    let athlete_response: serde_json::Value = serde_json::from_str(&response)?;

    if let Some(result) = athlete_response["result"].as_object() {
        println!("Success Athlete Profile:");
        println!(
            "   Name: {} {}",
            result["firstname"].as_str().unwrap_or("N/A"),
            result["lastname"].as_str().unwrap_or("N/A")
        );
        println!(
            "   Username: {}",
            result["username"].as_str().unwrap_or("N/A")
        );
        println!(
            "   Provider: {}",
            result["provider"].as_str().unwrap_or("N/A")
        );
    } else {
        println!("Error Error getting athlete: {}", athlete_response["error"]);
    }
    Ok(())
}

async fn test_recent_activities(
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
) -> Result<()> {
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

    let request_str = format!("{activities_request}\n");
    writer.write_all(request_str.as_bytes()).await?;

    let mut response = String::new();
    reader.read_line(&mut response).await?;
    let activities_response: serde_json::Value = serde_json::from_str(&response)?;

    if let Some(activities) = activities_response["result"].as_array() {
        println!("Success Recent Activities ({} found):", activities.len());
        for (i, activity) in activities.iter().enumerate() {
            println!(
                "   {}. {} ({})",
                i + 1,
                activity["name"].as_str().unwrap_or("Unknown"),
                activity["sport_type"].as_str().unwrap_or("Unknown")
            );
            println!(
                "      Duration: {} seconds",
                activity["duration_seconds"].as_u64().unwrap_or(0)
            );
            if let Some(distance) = activity["distance_meters"].as_f64() {
                println!("      Distance: {:.2} km", distance / 1000.0);
            }
            println!(
                "      Date: {}",
                activity["start_date"].as_str().unwrap_or("N/A")
            );
            println!();
        }
    } else {
        println!(
            "Error Error getting activities: {}",
            activities_response["error"]
        );
    }
    Ok(())
}

async fn test_fitness_stats(
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
) -> Result<()> {
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

    let request_str = format!("{stats_request}\n");
    writer.write_all(request_str.as_bytes()).await?;

    let mut response = String::new();
    reader.read_line(&mut response).await?;
    let stats_response: serde_json::Value = serde_json::from_str(&response)?;

    if let Some(result) = stats_response["result"].as_object() {
        println!("Success Fitness Statistics:");
        println!(
            "   Total Activities: {}",
            result["total_activities"].as_u64().unwrap_or(0)
        );
        println!(
            "   Total Distance: {:.2} km",
            result["total_distance"].as_f64().unwrap_or(0.0) / 1000.0
        );
        println!(
            "   Total Duration: {} hours",
            result["total_duration"].as_u64().unwrap_or(0) / 3600
        );
        println!(
            "   Total Elevation: {:.0} m",
            result["total_elevation_gain"].as_f64().unwrap_or(0.0)
        );
    } else {
        println!("Error Error getting stats: {}", stats_response["error"]);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing MCP Server with Real Strava Data...\n");

    // Connect to the MCP server
    let stream = TcpStream::connect("127.0.0.1:8080").await?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Run all tests
    test_initialize(&mut writer, &mut reader).await?;
    test_athlete_profile(&mut writer, &mut reader).await?;
    test_recent_activities(&mut writer, &mut reader).await?;
    test_fitness_stats(&mut writer, &mut reader).await?;

    println!("\nComplete MCP Server test completed successfully!");

    Ok(())
}
