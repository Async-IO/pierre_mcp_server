// Test Activity Intelligence for the longest 2025 run
use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

fn main() -> Result<()> {
    println!("🧠 Testing Activity Intelligence for Longest 2025 Run");
    println!("======================================================");
    
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
                "name": "test-intelligence-client",
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
    println!("\n📊 Retrieving activities to find longest 2025 run...");
    
    let mut all_activities: Vec<Value> = Vec::new();
    let mut page = 1;
    let limit = 50;
    
    // Get multiple pages of activities
    loop {
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
                    break; // No more activities
                }
                all_activities.extend(activities.clone());
                println!("📄 Retrieved page {} with {} activities", page, activities.len());
                page += 1;
                
                // Limit to reasonable number to avoid rate limits
                if page > 10 {
                    break;
                }
            } else {
                break;
            }
        } else {
            println!("❌ Error retrieving activities: {:?}", response);
            return Ok(());
        }
    }
    
    println!("📊 Total activities retrieved: {}", all_activities.len());
    
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
    
    println!("🏃 Found {} runs in 2025", runs_2025.len());
    
    if runs_2025.is_empty() {
        println!("❌ No runs found in 2025");
        return Ok(());
    }
    
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
    let start_date = longest_run.get("start_date").and_then(|d| d.as_str()).unwrap_or("");
    
    println!("\n🎯 LONGEST RUN IN 2025:");
    println!("   📛 Name: {}", name);
    println!("   📏 Distance: {:.2} km", distance_km);
    println!("   🆔 Activity ID: {}", activity_id);
    println!("   📅 Date: {}", start_date);
    
    // Now get Activity Intelligence for this run
    println!("\n🧠 Generating Activity Intelligence with Weather Analysis...");
    
    let intelligence_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_activity_intelligence",
            "arguments": {
                "provider": "strava",
                "activity_id": activity_id,
                "include_weather": true
            }
        },
        "id": 100
    });
    
    writeln!(stream, "{}", intelligence_request)?;
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let response: Value = serde_json::from_str(&line)?;
    
    if let Some(result) = response.get("result") {
        println!("✅ Activity Intelligence Generated!");
        println!("{}", "=".repeat(50));
        
        // Display the intelligence summary
        if let Some(summary) = result.get("summary").and_then(|s| s.as_str()) {
            println!("📝 Summary: {}", summary);
        }
        
        // Display performance indicators
        if let Some(perf) = result.get("performance_indicators") {
            println!("\n📊 Performance Indicators:");
            
            if let Some(effort) = perf.get("relative_effort").and_then(|e| e.as_f64()) {
                println!("   🎯 Relative Effort: {:.1}/10", effort);
            }
            
            if let Some(efficiency) = perf.get("efficiency_score").and_then(|e| e.as_f64()) {
                println!("   ⚡ Efficiency Score: {:.1}/100", efficiency);
            }
            
            // Personal records
            if let Some(prs) = perf.get("personal_records").and_then(|p| p.as_array()) {
                if !prs.is_empty() {
                    println!("   🏆 Personal Records: {}", prs.len());
                    for pr in prs {
                        if let (Some(record_type), Some(value), Some(unit)) = (
                            pr.get("record_type").and_then(|r| r.as_str()),
                            pr.get("value").and_then(|v| v.as_f64()),
                            pr.get("unit").and_then(|u| u.as_str())
                        ) {
                            println!("     • {}: {:.2} {}", record_type, value, unit);
                        }
                    }
                }
            }
            
            // Zone distribution
            if let Some(zones) = perf.get("zone_distribution") {
                println!("   📈 Heart Rate Zones:");
                if let Some(z2) = zones.get("zone2_endurance").and_then(|z| z.as_f64()) {
                    println!("     • Endurance Zone: {:.1}%", z2);
                }
                if let Some(z4) = zones.get("zone4_threshold").and_then(|z| z.as_f64()) {
                    println!("     • Threshold Zone: {:.1}%", z4);
                }
            }
        }
        
        // Display contextual factors
        if let Some(context) = result.get("contextual_factors") {
            println!("\n🌍 Contextual Factors:");
            
            if let Some(time_of_day) = context.get("time_of_day").and_then(|t| t.as_str()) {
                println!("   🕐 Time of Day: {}", time_of_day);
            }
            
            // Weather information
            if let Some(weather) = context.get("weather") {
                println!("   🌦️  Weather:");
                
                if let Some(temp) = weather.get("temperature_celsius").and_then(|t| t.as_f64()) {
                    println!("     🌡️  Temperature: {:.1}°C", temp);
                }
                
                if let Some(conditions) = weather.get("conditions").and_then(|c| c.as_str()) {
                    println!("     ☁️  Conditions: {}", conditions);
                }
                
                if let Some(humidity) = weather.get("humidity_percentage").and_then(|h| h.as_f64()) {
                    println!("     💧 Humidity: {:.1}%", humidity);
                }
                
                if let Some(wind) = weather.get("wind_speed_kmh").and_then(|w| w.as_f64()) {
                    println!("     💨 Wind Speed: {:.1} km/h", wind);
                }
            }
        }
        
        // Display key insights
        if let Some(insights) = result.get("key_insights").and_then(|i| i.as_array()) {
            if !insights.is_empty() {
                println!("\n💡 Key Insights:");
                for insight in insights {
                    if let Some(message) = insight.get("message").and_then(|m| m.as_str()) {
                        println!("   • {}", message);
                    }
                }
            }
        }
        
        if let Some(generated_at) = result.get("generated_at").and_then(|g| g.as_str()) {
            println!("\n📅 Analysis Generated: {}", generated_at);
        }
        
        if let Some(status) = result.get("status").and_then(|s| s.as_str()) {
            println!("✅ Status: {}", status);
        }
        
        println!("\n🎉 Activity Intelligence Complete!");
        println!("   This analysis includes weather context, performance metrics,");
        println!("   heart rate zones, and AI-powered insights for your longest");
        println!("   run in 2025!");
        
    } else {
        println!("❌ Error generating intelligence: {:?}", response);
    }
    
    Ok(())
}