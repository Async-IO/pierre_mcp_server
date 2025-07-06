// ABOUTME: Data analysis utility for finding the longest running activity in 2024 dataset
// ABOUTME: Fitness data mining tool to identify peak distance achievements from activity records
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

    let connection_result = establish_mcp_connection().await?;
    let (mut reader, mut writer) = connection_result;

    let all_activities = fetch_activities_data(&mut reader, &mut writer).await?;

    if all_activities.is_empty() {
        println!("❌ Failed to get activities");
    } else {
        analyze_activities(&all_activities);
    }

    Ok(())
}

/// Establish TCP connection and initialize MCP protocol session
async fn establish_mcp_connection() -> Result<(
    BufReader<tokio::net::tcp::OwnedReadHalf>,
    tokio::net::tcp::OwnedWriteHalf,
)> {
    // Step 1: Establish TCP connection to the MCP server
    let stream = TcpStream::connect("127.0.0.1:8080").await?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Step 2: Initialize MCP protocol session
    let init_request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {},
        "id": 1
    });

    writer
        .write_all(format!("{init_request}\n").as_bytes())
        .await?;
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    println!("✅ Connected to MCP server");

    Ok((reader, writer))
}

/// Fetch activities data using pagination through MCP server
async fn fetch_activities_data(
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
) -> Result<Vec<Value>> {
    let mut all_activities = Vec::new();
    let mut line = String::new();

    // Paginate through multiple pages to get historical data
    for page in 1..=3 {
        let activities_request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "get_activities",
                "arguments": {
                    "provider": "strava",
                    "limit": 200,
                    "offset": (page - 1) * 200
                }
            },
            "id": page + 1
        });

        writer
            .write_all(format!("{activities_request}\n").as_bytes())
            .await?;
        line.clear();
        reader.read_line(&mut line).await?;

        let response: Value = serde_json::from_str(&line)?;

        if let Some(activities) = response["result"].as_array() {
            if activities.is_empty() {
                break;
            }
            for activity in activities {
                all_activities.push(activity.clone());
            }
            println!(
                "📄 Got page {}: {} activities (total: {})",
                page,
                activities.len(),
                all_activities.len()
            );
        } else {
            println!("❌ Failed to get page {page}");
            break;
        }
    }

    Ok(all_activities)
}

/// Analyze activities to find longest 2024 run and display statistics
fn analyze_activities(all_activities: &[Value]) {
    println!("📊 Analyzing {} activities...", all_activities.len());

    let mut longest_run_2024: Option<&Value> = None;
    let mut longest_distance_2024 = 0.0;
    let mut total_runs_2024 = 0;
    let mut total_run_distance_2024 = 0.0;

    // Find 2024 runs and track statistics
    for activity in all_activities {
        if let Some(date_str) = activity["start_date"].as_str() {
            if date_str.starts_with("2024") {
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

    display_run_statistics(total_runs_2024, total_run_distance_2024);

    if let Some(run) = longest_run_2024 {
        display_longest_run_details(run, longest_distance_2024);
    } else {
        println!("\n❌ No runs found in 2024 activities");
    }

    display_other_runs_sample(all_activities);
}

/// Display overall 2024 run statistics
fn display_run_statistics(total_runs: i32, total_distance: f64) {
    println!("\n🏃 2024 Run Statistics:");
    println!("   Total runs in 2024: {total_runs}");
    println!(
        "   Total run distance in 2024: {:.2} km",
        total_distance / 1000.0
    );
}

/// Display detailed information about the longest run
fn display_longest_run_details(run: &Value, distance: f64) {
    println!("\n🏆 LONGEST RUN IN 2024:");
    println!("   Distance: {:.2} km", distance / 1000.0);

    if let Some(name) = run["name"].as_str() {
        println!("   Name: {name}");
    }

    if let Some(date) = run["start_date"].as_str() {
        println!("   Date: {date}");
    }

    if let Some(duration) = run["duration_seconds"].as_u64() {
        display_duration_and_pace(duration, distance);
    }

    if let Some(elevation) = run["elevation_gain"].as_f64() {
        println!("   Elevation gain: {elevation:.0} m");
    }

    if let Some(avg_hr) = run["average_heart_rate"].as_u64() {
        println!("   Average heart rate: {avg_hr} bpm");
    }
}

/// Display duration and calculate pace for a run
fn display_duration_and_pace(duration: u64, distance: f64) {
    let hours = duration / 3600;
    let minutes = (duration % 3600) / 60;
    let seconds = duration % 60;
    println!("   Duration: {hours}h {minutes}m {seconds}s");

    // Calculate pace
    if distance > 0.0 {
        let duration_u32 = u32::try_from(duration).unwrap_or(u32::MAX);
        let pace_per_km_f64 = f64::from(duration_u32) / (distance / 1000.0);
        let pace_minutes = (pace_per_km_f64 / 60.0).floor().clamp(0.0, 59.0);
        let pace_seconds = (pace_per_km_f64 % 60.0).floor().clamp(0.0, 59.0);

        println!("   Pace: {pace_minutes:.0}:{pace_seconds:02.0} min/km");
    }
}

/// Display a sample of other 2024 runs for context
fn display_other_runs_sample(all_activities: &[Value]) {
    println!("\n📋 Other 2024 runs:");
    let mut run_count = 0;
    for activity in all_activities {
        if let Some(date_str) = activity["start_date"].as_str() {
            if date_str.starts_with("2024") {
                if let Some(sport_type) = activity["sport_type"].as_str() {
                    if sport_type == "run" && run_count < 5 {
                        if let Some(distance_meters) = activity["distance_meters"].as_f64() {
                            if let Some(name) = activity["name"].as_str() {
                                println!(
                                    "   {:.2} km - {} ({})",
                                    distance_meters / 1000.0,
                                    name,
                                    &date_str[0..10]
                                );
                                run_count += 1;
                            }
                        }
                    }
                }
            }
        }
    }
}
