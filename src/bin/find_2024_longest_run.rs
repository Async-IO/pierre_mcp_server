// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Find 2024 Longest Run - Example MCP Client
//!
//! This binary serves as a comprehensive example of how to interact with the Pierre MCP Server
//! to query fitness data. It demonstrates the complete workflow from connection establishment
//! to data analysis.
//!
//! ## Purpose
//!
//! This example connects to a running Pierre MCP Server and analyzes a user's Strava activities
//! to find their longest run in 2024. It showcases:
//!
//! - MCP protocol communication (JSON-RPC over TCP)
//! - Paginated data retrieval from fitness providers
//! - Data filtering and analysis
//! - Error handling and connection management
//! - Performance optimization for large datasets
//!
//! ## Usage
//!
//! 1. Start the Pierre MCP Server:
//!    ```bash
//!    cargo run --bin pierre-mcp-server
//!    ```
//!
//! 2. Run this example client:
//!    ```bash
//!    cargo run --bin find-2024-longest-run
//!    ```
//!
//! ## Prerequisites
//!
//! - A running Pierre MCP Server on localhost:8080
//! - Configured Strava provider with valid authentication
//! - Activities data from 2024 in the user's Strava account
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐     JSON-RPC     ┌─────────────────┐     HTTP/OAuth2     ┌─────────────────┐
//! │  Example Client │ ←→ over TCP  ←→  │  Pierre MCP     │ ←→ API Calls    ←→  │  Strava API     │
//! │ (this binary)   │                  │  Server         │                     │                 │
//! └─────────────────┘                  └─────────────────┘                     └─────────────────┘
//! ```
//!
//! ## Key Features Demonstrated
//!
//! - **Pagination**: Efficiently retrieves large datasets in chunks
//! - **Filtering**: Processes activities by year and sport type
//! - **Analysis**: Calculates metrics like pace and duration
//! - **Error Handling**: Graceful handling of network and data errors
//! - **Data Presentation**: User-friendly output formatting

use anyhow::Result;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

/// Main entry point for the longest run finder example
///
/// This function demonstrates a complete MCP client workflow:
/// 1. Establishes connection to the MCP server
/// 2. Initializes the MCP protocol session
/// 3. Retrieves activities data through pagination
/// 4. Filters activities by year and sport type
/// 5. Analyzes data to find the longest run
/// 6. Presents results in a user-friendly format
///
/// # Returns
///
/// - `Ok(())` if the analysis completes successfully
/// - `Err` if connection, data retrieval, or analysis fails
///
/// # Example Output
///
/// ```text
/// 🏆 LONGEST RUN IN 2024:
///    Distance: 46.97 km
///    Name: Beluga Ultra Trail 45 km 🕺
///    Date: 2024-09-14T10:04:01Z
///    Duration: 7h 30m 12s
///    Pace: 9:35 min/km
///    Elevation gain: 2044 m
///    Average heart rate: 131 bpm
/// ```
#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Finding longest run in 2024 for Strava user...\n");
    
    // Step 1: Establish TCP connection to the MCP server
    // The Pierre MCP Server listens on localhost:8080 by default
    let stream = TcpStream::connect("127.0.0.1:8080").await?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    
    // Step 2: Initialize MCP protocol session
    // This is required by the MCP specification to establish capabilities
    // and protocol version before any tool calls can be made
    let init_request = serde_json::json!({
        "jsonrpc": "2.0",           // JSON-RPC 2.0 protocol
        "method": "initialize",     // MCP initialization method
        "params": {},               // No parameters needed for init
        "id": 1                     // Request ID for response correlation
    });
    
    writer.write_all(format!("{}\n", init_request).as_bytes()).await?;
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    println!("✅ Connected to MCP server");
    
    // Step 3: Retrieve activities data using pagination
    // Since we're looking for 2024 data, we need to go back in history.
    // Strava returns activities in reverse chronological order (newest first),
    // so we need to paginate through multiple pages to find 2024 activities.
    let mut all_activities = Vec::new();
    
    // Paginate through multiple pages to get historical data
    // Each page returns up to 200 activities with an offset
    for page in 1..=3 {
        let activities_request = serde_json::json!({
            "jsonrpc": "2.0",           // JSON-RPC 2.0 protocol
            "method": "tools/call",     // MCP tool call method
            "params": {
                "name": "get_activities",   // Tool name from server capabilities
                "arguments": {
                    "provider": "strava",       // Which fitness provider to use
                    "limit": 200,               // Max activities per request
                    "offset": (page - 1) * 200 // Pagination offset
                }
            },
            "id": page + 1              // Unique ID for this request
        });
        
        writer.write_all(format!("{}\n", activities_request).as_bytes()).await?;
        line.clear();
        reader.read_line(&mut line).await?;
        
        let response: Value = serde_json::from_str(&line)?;
        
        if let Some(activities) = response["result"].as_array() {
            if activities.is_empty() {
                break; // No more activities
            }
            for activity in activities {
                all_activities.push(activity.clone());
            }
            println!("📄 Got page {}: {} activities (total: {})", page, activities.len(), all_activities.len());
        } else {
            println!("❌ Failed to get page {}", page);
            break;
        }
    }
    
    if !all_activities.is_empty() {
        println!("📊 Analyzing {} activities...", all_activities.len());
        
        let mut longest_run_2024: Option<&Value> = None;
        let mut longest_distance_2024 = 0.0;
        let mut total_runs_2024 = 0;
        let mut total_run_distance_2024 = 0.0;
        
        for activity in &all_activities {
            // Check if it's from 2024
            if let Some(date_str) = activity["start_date"].as_str() {
                if date_str.starts_with("2024") {
                    // Check if it's a run
                    if let Some(sport_type) = activity["sport_type"].as_str() {
                        if sport_type == "run" {
                            total_runs_2024 += 1;
                            
                            if let Some(distance_meters) = activity["distance_meters"].as_f64() {
                                total_run_distance_2024 += distance_meters;
                                
                                if distance_meters > longest_distance_2024 {
                                    longest_distance_2024 = distance_meters;
                                    longest_run_2024 = Some(activity);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        println!("\n🏃 2024 Run Statistics:");
        println!("   Total runs in 2024: {}", total_runs_2024);
        println!("   Total run distance in 2024: {:.2} km", total_run_distance_2024 / 1000.0);
        
        if let Some(run) = longest_run_2024 {
            println!("\n🏆 LONGEST RUN IN 2024:");
            println!("   Distance: {:.2} km", longest_distance_2024 / 1000.0);
            
            if let Some(name) = run["name"].as_str() {
                println!("   Name: {}", name);
            }
            
            if let Some(date) = run["start_date"].as_str() {
                println!("   Date: {}", date);
            }
            
            if let Some(duration) = run["duration_seconds"].as_u64() {
                let hours = duration / 3600;
                let minutes = (duration % 3600) / 60;
                let seconds = duration % 60;
                println!("   Duration: {}h {}m {}s", hours, minutes, seconds);
                
                // Calculate pace
                if longest_distance_2024 > 0.0 {
                    let pace_per_km = duration as f64 / (longest_distance_2024 / 1000.0);
                    let pace_minutes = (pace_per_km / 60.0) as u64;
                    let pace_seconds = (pace_per_km % 60.0) as u64;
                    println!("   Pace: {}:{:02} min/km", pace_minutes, pace_seconds);
                }
            }
            
            if let Some(elevation) = run["elevation_gain"].as_f64() {
                println!("   Elevation gain: {:.0} m", elevation);
            }
            
            if let Some(avg_hr) = run["average_heart_rate"].as_u64() {
                println!("   Average heart rate: {} bpm", avg_hr);
            }
        } else {
            println!("\n❌ No runs found in 2024 activities");
        }
        
        // Also show some 2024 runs for context
        println!("\n📋 Other 2024 runs:");
        let mut run_count = 0;
        for activity in &all_activities {
            if let Some(date_str) = activity["start_date"].as_str() {
                if date_str.starts_with("2024") {
                    if let Some(sport_type) = activity["sport_type"].as_str() {
                        if sport_type == "run" && run_count < 5 {
                            if let Some(distance_meters) = activity["distance_meters"].as_f64() {
                                if let Some(name) = activity["name"].as_str() {
                                    println!("   {:.2} km - {} ({})", 
                                        distance_meters / 1000.0, 
                                        name, 
                                        &date_str[0..10]);
                                    run_count += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        println!("❌ Failed to get activities");
    }
    
    Ok(())
}