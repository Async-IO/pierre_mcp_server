// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use anyhow::Result;
use chrono::Datelike;
use pierre_mcp_server::models::{Activity, SportType};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

fn main() -> Result<()> {
    println!("🔍 Finding longest run in 2025...");
    
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
                "name": "find-2025-longest-run",
                "version": "1.0.0"
            }
        }
    });
    
    writeln!(stream, "{}", init_request)?;
    
    // Read initialize response
    let mut response = String::new();
    reader.read_line(&mut response)?;
    println!("Initialize response: {}", response.trim());
    
    // Send initialized notification
    let initialized = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    
    writeln!(stream, "{}", initialized)?;
    
    // Request activities from Strava with pagination to get more activities
    let mut all_activities = Vec::new();
    let mut page = 1;
    let per_page = 50;
    
    loop {
        let activities_request = json!({
            "jsonrpc": "2.0",
            "id": page + 1,
            "method": "tools/call",
            "params": {
                "name": "get_activities",
                "arguments": {
                    "provider": "strava",
                    "limit": per_page,
                    "offset": (page - 1) * per_page
                }
            }
        });
        
        writeln!(stream, "{}", activities_request)?;
        
        // Read activities response
        let mut response = String::new();
        reader.read_line(&mut response)?;
        
        let response_json: Value = serde_json::from_str(&response)?;
        
        if let Some(result) = response_json.get("result") {
            // Handle both old and new response formats
            let activities: Vec<Activity> = if result.is_array() {
                // New format: direct array response
                serde_json::from_value(result.clone())?
            } else if let Some(content) = result.get("content") {
                // Old format: wrapped in content
                if let Some(activities_json) = content.get(0).and_then(|c| c.get("text")) {
                    serde_json::from_str(activities_json.as_str().unwrap())?
                } else {
                    println!("❌ Unexpected content format: {}", response);
                    break;
                }
            } else {
                println!("❌ Unexpected result format: {}", response);
                break;
            };
            
            if activities.is_empty() {
                println!("📄 No more activities found on page {}", page);
                break;
            }
            
            let activities_count = activities.len();
            println!("📄 Found {} activities on page {}", activities_count, page);
            all_activities.extend(activities);
            
            if activities_count < per_page {
                println!("📄 Reached end of activities (partial page)");
                break;
            }
            
            page += 1;
            
            // Limit to reasonable number of pages to avoid rate limits
            if page > 10 {
                println!("📄 Limiting to first 10 pages to avoid rate limits");
                break;
            }
        } else {
            println!("❌ Failed to get activities: {}", response);
            break;
        }
    }
    
    println!("📊 Total activities retrieved: {}", all_activities.len());
    
    // Filter for 2025 runs
    let year_2025_runs: Vec<&Activity> = all_activities
        .iter()
        .filter(|activity| {
            // Check if it's a run (including trail runs, etc.)
            let is_run = matches!(activity.sport_type, SportType::Run) || 
                         matches!(activity.sport_type, SportType::Other(ref s) if s.to_lowercase().contains("run"));
            
            // Check if it's from 2025
            let is_2025 = activity.start_date.year() == 2025;
            
            is_run && is_2025
        })
        .collect();
    
    println!("🏃 Found {} runs in 2025", year_2025_runs.len());
    
    if year_2025_runs.is_empty() {
        println!("❌ No runs found in 2025");
        
        // Show some sample activities for debugging
        println!("\n🔍 Sample activities for debugging:");
        for activity in all_activities.iter().take(5) {
            println!("  - {} ({:?}) on {} - Distance: {:.2}km", 
                activity.name, 
                activity.sport_type,
                activity.start_date.format("%Y-%m-%d"),
                activity.distance_meters.unwrap_or(0.0) / 1000.0
            );
        }
        
        // Show activities by year
        let mut years: std::collections::HashMap<i32, usize> = std::collections::HashMap::new();
        for activity in &all_activities {
            *years.entry(activity.start_date.year()).or_insert(0) += 1;
        }
        
        println!("\n📅 Activities by year:");
        let mut sorted_years: Vec<_> = years.into_iter().collect();
        sorted_years.sort_by_key(|&(year, _)| year);
        for (year, count) in sorted_years {
            println!("  - {}: {} activities", year, count);
        }
        
        return Ok(());
    }
    
    // Find the longest run
    let longest_run = year_2025_runs
        .iter()
        .max_by(|a, b| {
            let dist_a = a.distance_meters.unwrap_or(0.0);
            let dist_b = b.distance_meters.unwrap_or(0.0);
            dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap();
    
    let distance_km = longest_run.distance_meters.unwrap_or(0.0) / 1000.0;
    let duration_hours = longest_run.duration_seconds as f64 / 3600.0;
    
    println!("\n🎯 LONGEST RUN IN 2025:");
    println!("   📛 Name: {}", longest_run.name);
    println!("   📏 Distance: {:.2} km", distance_km);
    println!("   ⏱️  Duration: {:.2} hours", duration_hours);
    println!("   📅 Date: {}", longest_run.start_date.format("%Y-%m-%d %H:%M:%S UTC"));
    println!("   🏃 Type: {:?}", longest_run.sport_type);
    
    if let Some(elevation) = longest_run.elevation_gain {
        println!("   ⛰️  Elevation Gain: {:.0} m", elevation);
    }
    
    if let Some(avg_speed) = longest_run.average_speed {
        let pace_min_per_km = 1000.0 / (avg_speed * 60.0);
        println!("   🏃 Average Pace: {:.2} min/km", pace_min_per_km);
    }
    
    println!("\n✅ Test completed successfully!");
    
    Ok(())
}