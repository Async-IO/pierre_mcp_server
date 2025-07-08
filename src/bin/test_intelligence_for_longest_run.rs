// ABOUTME: Intelligence analysis utility for testing AI insights on longest running activities
// ABOUTME: Validates fitness intelligence algorithms using peak performance activity data
use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

fn main() -> Result<()> {
    println!("AI Testing Activity Intelligence for Longest 2025 Run");
    println!("======================================================");

    let (mut stream, mut reader) = connect_to_mcp_server()?;
    initialize_mcp_connection(&mut stream, &mut reader)?;

    let all_activities = retrieve_all_activities(&mut stream, &mut reader)?;
    let longest_run = find_longest_2025_run(&all_activities)?;

    display_run_info(&longest_run);
    generate_and_display_intelligence(&mut stream, &mut reader, &longest_run)?;

    Ok(())
}

fn connect_to_mcp_server() -> Result<(TcpStream, BufReader<TcpStream>)> {
    let stream = TcpStream::connect("127.0.0.1:8080")?;
    let reader = BufReader::new(stream.try_clone()?);
    Ok((stream, reader))
}

fn initialize_mcp_connection(
    stream: &mut TcpStream,
    reader: &mut BufReader<TcpStream>,
) -> Result<()> {
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

    Ok(())
}

fn retrieve_all_activities(
    stream: &mut TcpStream,
    reader: &mut BufReader<TcpStream>,
) -> Result<Vec<Value>> {
    println!("\nData Retrieving activities to find longest 2025 run...");

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

        writeln!(stream, "{activities_request}")?;
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let response: Value = serde_json::from_str(&line)?;

        if let Some(result) = response.get("result") {
            if let Some(activities) = result.as_array() {
                if activities.is_empty() {
                    break; // No more activities
                }
                all_activities.extend(activities.clone());
                println!(
                    "Page Retrieved page {page} with {} activities",
                    activities.len()
                );
                page += 1;

                // Limit to reasonable number to avoid rate limits
                if page > 10 {
                    break;
                }
            } else {
                break;
            }
        } else {
            println!("Error Error retrieving activities: {response:?}");
            return Ok(all_activities);
        }
    }

    println!("Data Total activities retrieved: {}", all_activities.len());

    Ok(all_activities)
}

fn find_longest_2025_run(all_activities: &[Value]) -> Result<Value> {
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

    println!("Run Found {} runs in 2025", runs_2025.len());

    if runs_2025.is_empty() {
        anyhow::bail!("No runs found in 2025");
    }

    let longest_run = runs_2025
        .iter()
        .max_by(|a, b| {
            let dist_a = a
                .get("distance_meters")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let dist_b = b
                .get("distance_meters")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            dist_a
                .partial_cmp(&dist_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .ok_or_else(|| anyhow::anyhow!("No longest run found"))?;

    Ok((*longest_run).clone())
}

fn display_run_info(longest_run: &Value) {
    let distance_km = longest_run
        .get("distance_meters")
        .and_then(Value::as_f64)
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
    let start_date = longest_run
        .get("start_date")
        .and_then(|d| d.as_str())
        .unwrap_or("");

    println!("\nTarget LONGEST RUN IN 2025:");
    println!("   Name Name: {name}");
    println!("   Distance Distance: {distance_km:.2} km");
    println!("   ID Activity ID: {activity_id}");
    println!("   Date Date: {start_date}");
}

fn generate_and_display_intelligence(
    stream: &mut TcpStream,
    reader: &mut BufReader<TcpStream>,
    longest_run: &Value,
) -> Result<()> {
    let activity_id = longest_run
        .get("id")
        .and_then(|id| id.as_str())
        .unwrap_or("");

    // Now get Activity Intelligence for this run
    println!("\nAI Generating Activity Intelligence with Weather and Location Analysis...");

    let intelligence_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_activity_intelligence",
            "arguments": {
                "provider": "strava",
                "activity_id": activity_id,
                "include_weather": true,
                "include_location": true
            }
        },
        "id": 100
    });

    writeln!(stream, "{intelligence_request}")?;
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let response: Value = serde_json::from_str(&line)?;

    if let Some(result) = response.get("result") {
        println!("Success Activity Intelligence Generated!");
        println!("{}", "=".repeat(50));

        // Display the intelligence summary
        if let Some(summary) = result.get("summary").and_then(|s| s.as_str()) {
            println!("Summary Summary: {summary}");
        }

        display_performance_indicators(result);
        display_contextual_factors(result);
        display_key_insights(result);
        display_metadata(result);

        println!("\nComplete Activity Intelligence Complete!");
        println!("   This analysis includes weather context, location intelligence,");
        println!("   performance metrics, heart rate zones, and AI-powered insights");
        println!("   for your longest run in 2025!");
    } else {
        println!("Error Error generating intelligence: {response:?}");
    }

    Ok(())
}

fn display_performance_indicators(result: &Value) {
    if let Some(perf) = result.get("performance_indicators") {
        println!("\nData Performance Indicators:");

        if let Some(effort) = perf.get("relative_effort").and_then(Value::as_f64) {
            println!("   Target Relative Effort: {effort:.1}/10");
        }

        if let Some(efficiency) = perf.get("efficiency_score").and_then(Value::as_f64) {
            println!("   Efficiency Efficiency Score: {efficiency:.1}/100");
        }

        if let Some(prs) = perf.get("personal_records").and_then(Value::as_array) {
            if !prs.is_empty() {
                println!("   Record Personal Records: {}", prs.len());
                for pr in prs {
                    if let (Some(record_type), Some(value), Some(unit)) = (
                        pr.get("record_type").and_then(|r| r.as_str()),
                        pr.get("value").and_then(Value::as_f64),
                        pr.get("unit").and_then(|u| u.as_str()),
                    ) {
                        println!("     • {record_type}: {value:.2} {unit}");
                    }
                }
            }
        }

        if let Some(zones) = perf.get("zone_distribution") {
            println!("   Performance Heart Rate Zones:");
            if let Some(z2) = zones.get("zone2_endurance").and_then(Value::as_f64) {
                println!("     • Endurance Zone: {z2:.1}%");
            }
            if let Some(z4) = zones.get("zone4_threshold").and_then(Value::as_f64) {
                println!("     • Threshold Zone: {z4:.1}%");
            }
        }
    }
}

fn display_contextual_factors(result: &Value) {
    if let Some(context) = result.get("contextual_factors") {
        println!("\nCountry Contextual Factors:");

        if let Some(time_of_day) = context.get("time_of_day").and_then(|t| t.as_str()) {
            println!("   Time Time of Day: {time_of_day}");
        }

        display_weather_info(context);
        display_location_info(context);
    }
}

fn display_weather_info(context: &Value) {
    if let Some(weather) = context.get("weather") {
        println!("   Weather  Weather:");

        if let Some(temp) = weather.get("temperature_celsius").and_then(Value::as_f64) {
            println!("     Temperature  Temperature: {temp:.1}°C");
        }

        if let Some(conditions) = weather.get("conditions").and_then(Value::as_str) {
            println!("     Conditions  Conditions: {conditions}");
        }

        if let Some(humidity) = weather.get("humidity_percentage").and_then(Value::as_f64) {
            println!("     Humidity Humidity: {humidity:.1}%");
        }

        if let Some(wind) = weather.get("wind_speed_kmh").and_then(Value::as_f64) {
            println!("     Wind Wind Speed: {wind:.1} km/h");
        }
    }
}

fn display_location_info(context: &Value) {
    if let Some(location) = context.get("location") {
        println!("   Location  Location:");

        if let Some(display_name) = location.get("display_name").and_then(Value::as_str) {
            println!("     Location Location: {display_name}");
        }

        if let Some(city) = location.get("city").and_then(Value::as_str) {
            println!("     City  City: {city}");
        }

        if let Some(region) = location.get("region").and_then(Value::as_str) {
            println!("     Location  Region: {region}");
        }

        if let Some(country) = location.get("country").and_then(Value::as_str) {
            println!("     Country Country: {country}");
        }

        if let Some(trail_name) = location.get("trail_name").and_then(Value::as_str) {
            println!("     Trail Trail: {trail_name}");
        }

        if let Some(terrain_type) = location.get("terrain_type").and_then(Value::as_str) {
            println!("     Terrain  Terrain: {terrain_type}");
        }
    }
}

fn display_key_insights(result: &Value) {
    if let Some(insights) = result.get("key_insights").and_then(Value::as_array) {
        if !insights.is_empty() {
            println!("\nTip Key Insights:");
            for insight in insights {
                if let Some(message) = insight.get("message").and_then(Value::as_str) {
                    println!("   • {message}");
                }
            }
        }
    }
}

fn display_metadata(result: &Value) {
    if let Some(generated_at) = result.get("generated_at").and_then(Value::as_str) {
        println!("\nDate Analysis Generated: {generated_at}");
    }

    if let Some(status) = result.get("status").and_then(Value::as_str) {
        println!("Success Status: {status}");
    }
}
