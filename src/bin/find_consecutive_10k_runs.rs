// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use anyhow::Result;
use chrono::NaiveDate;
use pierre_mcp_server::models::{Activity, SportType};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

fn main() -> Result<()> {
    println!("🏃 Finding longest consecutive running streak with 10km+ runs...");
    
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
                "name": "find-consecutive-10k-runs",
                "version": "1.0.0"
            }
        }
    });
    
    writeln!(stream, "{}", init_request)?;
    
    // Read initialize response
    let mut response = String::new();
    reader.read_line(&mut response)?;
    println!("✅ Connected to MCP server");
    
    // Send initialized notification
    let initialized = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    
    writeln!(stream, "{}", initialized)?;
    
    // Request all activities with aggressive pagination to get full history
    let mut all_activities = Vec::new();
    let mut page = 1;
    let per_page = 200; // Larger pages for efficiency
    
    println!("📡 Retrieving activity history...");
    
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
            let activities: Vec<Activity> = if result.is_array() {
                serde_json::from_value(result.clone())?
            } else if let Some(content) = result.get("content") {
                if let Some(activities_json) = content.get(0).and_then(|c| c.get("text")) {
                    serde_json::from_str(activities_json.as_str().unwrap())?
                } else {
                    println!("❌ Unexpected content format");
                    break;
                }
            } else {
                println!("❌ Unexpected result format");
                break;
            };
            
            if activities.is_empty() {
                println!("📄 No more activities found on page {}", page);
                break;
            }
            
            let activities_count = activities.len();
            println!("📄 Retrieved {} activities on page {}", activities_count, page);
            all_activities.extend(activities);
            
            if activities_count < per_page {
                println!("📄 Reached end of activities");
                break;
            }
            
            page += 1;
            
            // Safety limit to avoid excessive API calls
            if page > 20 {
                println!("📄 Limiting to first 20 pages to avoid rate limits");
                break;
            }
        } else {
            println!("❌ Failed to get activities: {}", response);
            break;
        }
    }
    
    println!("📊 Total activities retrieved: {}", all_activities.len());
    
    // Filter for runs with 10km+ distance
    let qualifying_runs: Vec<&Activity> = all_activities
        .iter()
        .filter(|activity| {
            // Check if it's a run
            let is_run = matches!(activity.sport_type, SportType::Run) || 
                         matches!(activity.sport_type, SportType::Other(ref s) if s.to_lowercase().contains("run"));
            
            // Check if distance is 10km or more
            let is_10k_plus = activity.distance_meters
                .map(|d| d >= 10000.0)
                .unwrap_or(false);
            
            is_run && is_10k_plus
        })
        .collect();
    
    println!("🏃 Found {} runs with 10km+ distance", qualifying_runs.len());
    
    if qualifying_runs.is_empty() {
        println!("❌ No runs with 10km+ distance found");
        return Ok(());
    }
    
    // Group runs by date (only keep the longest run per day)
    let mut runs_by_date: HashMap<NaiveDate, &Activity> = HashMap::new();
    
    for run in &qualifying_runs {
        let date = run.start_date.date_naive();
        
        // Keep only the longest run for each day
        if let Some(existing_run) = runs_by_date.get(&date) {
            let existing_distance = existing_run.distance_meters.unwrap_or(0.0);
            let current_distance = run.distance_meters.unwrap_or(0.0);
            
            if current_distance > existing_distance {
                runs_by_date.insert(date, run);
            }
        } else {
            runs_by_date.insert(date, run);
        }
    }
    
    println!("📅 Found {} unique days with 10km+ runs", runs_by_date.len());
    
    // Convert to sorted vector by date
    let mut daily_runs: Vec<(NaiveDate, &Activity)> = runs_by_date.into_iter().collect();
    daily_runs.sort_by_key(|(date, _)| *date);
    
    // Find longest consecutive streak
    let mut longest_streak = 0;
    let mut current_streak = 0;
    let mut longest_streak_start: Option<NaiveDate> = None;
    let mut longest_streak_end: Option<NaiveDate> = None;
    let mut current_streak_start: Option<NaiveDate> = None;
    let mut longest_streak_activities: Vec<&Activity> = Vec::new();
    
    for i in 0..daily_runs.len() {
        let (current_date, _current_activity) = daily_runs[i];
        
        if i == 0 {
            // First run starts a streak
            current_streak = 1;
            current_streak_start = Some(current_date);
        } else {
            let (prev_date, _) = daily_runs[i - 1];
            
            // Check if current date is consecutive to previous date
            if current_date == prev_date + chrono::Duration::days(1) {
                // Continue streak
                current_streak += 1;
            } else {
                // Check if current streak is the longest
                if current_streak > longest_streak {
                    longest_streak = current_streak;
                    longest_streak_start = current_streak_start;
                    longest_streak_end = Some(prev_date);
                    
                    // Collect activities from the longest streak
                    longest_streak_activities.clear();
                    let start_idx = i - current_streak;
                    for j in start_idx..i {
                        longest_streak_activities.push(daily_runs[j].1);
                    }
                }
                
                // Start new streak
                current_streak = 1;
                current_streak_start = Some(current_date);
            }
        }
    }
    
    // Check the final streak
    if current_streak > longest_streak {
        longest_streak = current_streak;
        longest_streak_start = current_streak_start;
        longest_streak_end = Some(daily_runs.last().unwrap().0);
        
        // Collect activities from the final streak
        longest_streak_activities.clear();
        let start_idx = daily_runs.len() - current_streak;
        for j in start_idx..daily_runs.len() {
            longest_streak_activities.push(daily_runs[j].1);
        }
    }
    
    // Display results
    println!("\n🏆 LONGEST CONSECUTIVE 10KM+ RUNNING STREAK:");
    println!("   📈 Streak Length: {} days", longest_streak);
    
    if let (Some(start), Some(end)) = (longest_streak_start, longest_streak_end) {
        println!("   📅 Period: {} to {}", start.format("%Y-%m-%d"), end.format("%Y-%m-%d"));
        
        // Calculate total distance and time for the streak
        let total_distance: f64 = longest_streak_activities
            .iter()
            .map(|a| a.distance_meters.unwrap_or(0.0))
            .sum();
        
        let total_time: u64 = longest_streak_activities
            .iter()
            .map(|a| a.duration_seconds)
            .sum();
        
        println!("   📏 Total Distance: {:.2} km", total_distance / 1000.0);
        println!("   ⏱️  Total Time: {:.2} hours", total_time as f64 / 3600.0);
        println!("   📊 Average Distance: {:.2} km/day", (total_distance / 1000.0) / longest_streak as f64);
        
        println!("\n📋 Streak Details:");
        for (i, activity) in longest_streak_activities.iter().enumerate() {
            let distance_km = activity.distance_meters.unwrap_or(0.0) / 1000.0;
            let duration_hours = activity.duration_seconds as f64 / 3600.0;
            
            println!("   Day {}: {} - {:.2}km in {:.2}h - \"{}\"", 
                i + 1,
                activity.start_date.format("%Y-%m-%d"),
                distance_km,
                duration_hours,
                activity.name
            );
        }
    }
    
    if longest_streak == 0 {
        println!("❌ No consecutive days with 10km+ runs found");
    } else if longest_streak == 1 {
        println!("ℹ️  Maximum streak is 1 day (no consecutive days found)");
    }
    
    // Show some additional statistics
    println!("\n📊 Additional Statistics:");
    println!("   🏃 Total 10km+ runs: {}", qualifying_runs.len());
    println!("   📅 Total days with 10km+ runs: {}", daily_runs.len());
    
    // Find the longest single run
    if let Some(longest_run) = qualifying_runs.iter().max_by(|a, b| {
        let dist_a = a.distance_meters.unwrap_or(0.0);
        let dist_b = b.distance_meters.unwrap_or(0.0);
        dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
    }) {
        println!("   🎯 Longest single run: {:.2}km - \"{}\"", 
            longest_run.distance_meters.unwrap_or(0.0) / 1000.0,
            longest_run.name
        );
    }
    
    println!("\n✅ Analysis completed successfully!");
    
    Ok(())
}