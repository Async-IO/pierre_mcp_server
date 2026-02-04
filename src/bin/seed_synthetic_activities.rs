// ABOUTME: Synthetic activity seeder for Pierre MCP Server testing without OAuth
// ABOUTME: Generates 100+ diverse activities (nordic ski, MTB, trail run, etc.) for any user
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Synthetic activity seeder for Pierre MCP Server.
//!
//! This binary populates the database with diverse synthetic activities for testing
//! without requiring Strava or other OAuth providers.
//!
//! Usage:
//! ```bash
//! # Seed activities for the default test user (user@example.com)
//! cargo run --bin seed-synthetic-activities
//!
//! # Seed for a specific user
//! cargo run --bin seed-synthetic-activities -- --email alice@example.com
//!
//! # Generate more activities (default: 100)
//! cargo run --bin seed-synthetic-activities -- --count 200
//!
//! # Spread over more days (default: 90)
//! cargo run --bin seed-synthetic-activities -- --days 180
//!
//! # Reset activities before seeding
//! cargo run --bin seed-synthetic-activities -- --reset
//!
//! # Verbose output
//! cargo run --bin seed-synthetic-activities -- -v
//! ```

use chrono::{Duration, Utc};
use clap::Parser;
use rand::prelude::SliceRandom;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tracing::info;
use uuid::Uuid;

/// CLI-specific error type for the seed binary
#[derive(Error, Debug)]
enum SeedError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("{0}")]
    Validation(String),
}

type SeedResult<T> = Result<T, SeedError>;

#[derive(Parser)]
#[command(
    name = "seed-synthetic-activities",
    about = "Pierre MCP Server Synthetic Activity Seeder",
    long_about = "Populate the database with diverse synthetic activities for testing without OAuth"
)]
struct SeedArgs {
    /// User email to seed activities for (default: user@example.com)
    #[arg(long, default_value = "user@example.com")]
    email: String,

    /// Database URL override
    #[arg(long)]
    database_url: Option<String>,

    /// Number of activities to generate
    #[arg(long, default_value = "100")]
    count: u32,

    /// Number of days to spread activities over
    #[arg(long, default_value = "90")]
    days: u32,

    /// Reset synthetic activities before seeding
    #[arg(long)]
    reset: bool,

    /// Enable verbose logging
    #[arg(long, short = 'v')]
    verbose: bool,

    /// Random seed for reproducible data (optional)
    #[arg(long)]
    seed: Option<u64>,
}

/// Sport type configuration for activity generation
struct SportConfig {
    sport_type: &'static str,
    display_name: &'static str,
    /// Weight for random selection (higher = more common)
    weight: u32,
    /// Duration range in seconds (min, max)
    duration_range: (u64, u64),
    /// Distance range in meters (min, max), None for non-distance activities
    distance_range: Option<(f64, f64)>,
    /// Elevation gain range in meters (min, max), None for flat activities
    elevation_range: Option<(f64, f64)>,
    /// Average heart rate range
    heart_rate_range: (u32, u32),
    /// Activity name templates
    names: &'static [&'static str],
}

/// Get all sport configurations with realistic parameters
fn get_sport_configs() -> Vec<SportConfig> {
    vec![
        // Running activities (most common)
        SportConfig {
            sport_type: "run",
            display_name: "Run",
            weight: 25,
            duration_range: (1200, 7200),            // 20 min - 2 hours
            distance_range: Some((3000.0, 25000.0)), // 3km - 25km
            elevation_range: Some((20.0, 300.0)),
            heart_rate_range: (140, 175),
            names: &[
                "Morning Run",
                "Easy Run",
                "Tempo Run",
                "Long Run",
                "Recovery Run",
                "Interval Session",
            ],
        },
        SportConfig {
            sport_type: "trail_run",
            display_name: "Trail Run",
            weight: 8,
            duration_range: (2400, 10800), // 40 min - 3 hours
            distance_range: Some((5000.0, 30000.0)),
            elevation_range: Some((200.0, 1500.0)), // More elevation
            heart_rate_range: (145, 180),
            names: &[
                "Trail Adventure",
                "Mountain Trail",
                "Forest Run",
                "Technical Trail",
                "Ridge Run",
            ],
        },
        // Cycling activities
        SportConfig {
            sport_type: "ride",
            display_name: "Ride",
            weight: 20,
            duration_range: (1800, 18000), // 30 min - 5 hours
            distance_range: Some((15_000.0, 150_000.0)), // 15km - 150km
            elevation_range: Some((100.0, 2000.0)),
            heart_rate_range: (130, 170),
            names: &[
                "Morning Ride",
                "Endurance Ride",
                "Tempo Ride",
                "Group Ride",
                "Solo Spin",
            ],
        },
        SportConfig {
            sport_type: "mountain_bike_ride",
            display_name: "Mountain Bike",
            weight: 8,
            duration_range: (2400, 14400), // 40 min - 4 hours
            distance_range: Some((10000.0, 60000.0)),
            elevation_range: Some((300.0, 2500.0)), // Lots of climbing
            heart_rate_range: (140, 180),
            names: &[
                "MTB Session",
                "Single Track",
                "Trail Ride",
                "Downhill Fun",
                "Technical Climb",
            ],
        },
        SportConfig {
            sport_type: "gravel_ride",
            display_name: "Gravel Ride",
            weight: 5,
            duration_range: (3600, 21600), // 1 - 6 hours
            distance_range: Some((30_000.0, 200_000.0)),
            elevation_range: Some((200.0, 3000.0)),
            heart_rate_range: (135, 170),
            names: &[
                "Gravel Adventure",
                "Mixed Surface",
                "Backroads Explorer",
                "Gravel Century",
            ],
        },
        SportConfig {
            sport_type: "virtual_ride",
            display_name: "Virtual Ride",
            weight: 6,
            duration_range: (1800, 5400), // 30 min - 1.5 hours
            distance_range: Some((15000.0, 60000.0)),
            elevation_range: Some((100.0, 1000.0)),
            heart_rate_range: (135, 175),
            names: &[
                "Zwift Session",
                "Indoor Training",
                "Trainer Workout",
                "Virtual Race",
            ],
        },
        // Winter sports
        SportConfig {
            sport_type: "nordic_ski",
            display_name: "Nordic Ski",
            weight: 6,
            duration_range: (2400, 10800), // 40 min - 3 hours
            distance_range: Some((5000.0, 50000.0)),
            elevation_range: Some((100.0, 800.0)),
            heart_rate_range: (140, 180),
            names: &[
                "Classic Ski",
                "Skate Ski",
                "Nordic Tour",
                "Ski Marathon Training",
                "Trail Ski",
            ],
        },
        SportConfig {
            sport_type: "backcountry_ski",
            display_name: "Backcountry Ski",
            weight: 3,
            duration_range: (3600, 18000), // 1 - 5 hours
            distance_range: Some((3000.0, 20000.0)),
            elevation_range: Some((500.0, 2500.0)), // Big climbing
            heart_rate_range: (130, 170),
            names: &[
                "Backcountry Tour",
                "Skin Up",
                "Powder Day",
                "Alpine Tour",
                "Summit Push",
            ],
        },
        SportConfig {
            sport_type: "alpine_ski",
            display_name: "Alpine Ski",
            weight: 4,
            duration_range: (3600, 21600), // 1 - 6 hours
            distance_range: Some((10000.0, 50000.0)),
            elevation_range: Some((1000.0, 5000.0)), // Vertical meters
            heart_rate_range: (100, 140),
            names: &[
                "Ski Day",
                "Resort Laps",
                "Powder Hunting",
                "Groomer Day",
                "All Mountain",
            ],
        },
        SportConfig {
            sport_type: "snowshoe",
            display_name: "Snowshoe",
            weight: 2,
            duration_range: (2400, 10800), // 40 min - 3 hours
            distance_range: Some((3000.0, 15000.0)),
            elevation_range: Some((100.0, 800.0)),
            heart_rate_range: (120, 155),
            names: &[
                "Snowshoe Hike",
                "Winter Trail",
                "Snow Trek",
                "Backcountry Snowshoe",
            ],
        },
        // Swimming
        SportConfig {
            sport_type: "swim",
            display_name: "Swim",
            weight: 6,
            duration_range: (1200, 5400), // 20 min - 1.5 hours
            distance_range: Some((500.0, 5000.0)),
            elevation_range: None,
            heart_rate_range: (120, 160),
            names: &[
                "Pool Swim",
                "Lap Session",
                "Endurance Swim",
                "Drill Work",
                "Speed Set",
            ],
        },
        SportConfig {
            sport_type: "open_water_swim",
            display_name: "Open Water Swim",
            weight: 2,
            duration_range: (1800, 7200), // 30 min - 2 hours
            distance_range: Some((1000.0, 10000.0)),
            elevation_range: None,
            heart_rate_range: (130, 165),
            names: &[
                "Lake Swim",
                "Ocean Swim",
                "River Crossing",
                "Triathlon Practice",
            ],
        },
        // Walking and hiking
        SportConfig {
            sport_type: "walk",
            display_name: "Walk",
            weight: 8,
            duration_range: (1200, 7200), // 20 min - 2 hours
            distance_range: Some((2000.0, 15000.0)),
            elevation_range: Some((10.0, 200.0)),
            heart_rate_range: (90, 120),
            names: &[
                "Morning Walk",
                "Lunch Walk",
                "Evening Stroll",
                "Active Recovery",
            ],
        },
        SportConfig {
            sport_type: "hike",
            display_name: "Hike",
            weight: 6,
            duration_range: (3600, 28800), // 1 - 8 hours
            distance_range: Some((5000.0, 30000.0)),
            elevation_range: Some((200.0, 2000.0)),
            heart_rate_range: (110, 150),
            names: &[
                "Day Hike",
                "Summit Hike",
                "Ridge Walk",
                "Canyon Hike",
                "Peak Bagging",
            ],
        },
        // Strength and indoor
        SportConfig {
            sport_type: "weight_training",
            display_name: "Weight Training",
            weight: 8,
            duration_range: (1800, 5400), // 30 min - 1.5 hours
            distance_range: None,
            elevation_range: None,
            heart_rate_range: (100, 145),
            names: &[
                "Strength Session",
                "Leg Day",
                "Upper Body",
                "Full Body",
                "Core Work",
            ],
        },
        SportConfig {
            sport_type: "yoga",
            display_name: "Yoga",
            weight: 4,
            duration_range: (1800, 5400), // 30 min - 1.5 hours
            distance_range: None,
            elevation_range: None,
            heart_rate_range: (70, 110),
            names: &[
                "Morning Yoga",
                "Vinyasa Flow",
                "Recovery Yoga",
                "Power Yoga",
                "Stretch Session",
            ],
        },
        SportConfig {
            sport_type: "workout",
            display_name: "Workout",
            weight: 5,
            duration_range: (1200, 3600), // 20 min - 1 hour
            distance_range: None,
            elevation_range: None,
            heart_rate_range: (130, 170),
            names: &[
                "HIIT Session",
                "CrossFit WOD",
                "Circuit Training",
                "Cardio Blast",
                "Functional Fitness",
            ],
        },
        // Water sports
        SportConfig {
            sport_type: "rowing",
            display_name: "Rowing",
            weight: 3,
            duration_range: (1200, 5400), // 20 min - 1.5 hours
            distance_range: Some((2000.0, 20000.0)),
            elevation_range: None,
            heart_rate_range: (140, 175),
            names: &[
                "Erg Session",
                "On-Water Row",
                "2K Test",
                "Steady State",
                "Intervals",
            ],
        },
        SportConfig {
            sport_type: "kayaking",
            display_name: "Kayaking",
            weight: 2,
            duration_range: (2400, 14400), // 40 min - 4 hours
            distance_range: Some((5000.0, 40000.0)),
            elevation_range: None,
            heart_rate_range: (110, 150),
            names: &[
                "Paddle Session",
                "River Run",
                "Lake Tour",
                "Sea Kayak",
                "Whitewater",
            ],
        },
        SportConfig {
            sport_type: "stand_up_paddling",
            display_name: "SUP",
            weight: 2,
            duration_range: (1800, 7200), // 30 min - 2 hours
            distance_range: Some((2000.0, 15000.0)),
            elevation_range: None,
            heart_rate_range: (100, 140),
            names: &[
                "SUP Session",
                "Paddle Tour",
                "SUP Yoga",
                "Downwind Run",
                "Flatwater Cruise",
            ],
        },
    ]
}

/// Build weighted selection vector from sport configs
fn build_weighted_sports(configs: &[SportConfig]) -> Vec<usize> {
    let mut weighted = Vec::new();
    for (index, config) in configs.iter().enumerate() {
        for _ in 0..config.weight {
            weighted.push(index);
        }
    }
    weighted
}

#[tokio::main]
async fn main() -> SeedResult<()> {
    let args = SeedArgs::parse();

    // Initialize logging
    let log_level = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt().with_env_filter(log_level).init();

    info!("🏃 Pierre Synthetic Activity Seeder");
    info!("   Email: {}", args.email);
    info!("   Count: {} activities", args.count);
    info!("   Days: {} days of history", args.days);

    // Connect to database
    let database_url = args
        .database_url
        .or_else(|| env::var("DATABASE_URL").ok())
        .unwrap_or_else(|| "sqlite:./data/users.db".to_owned());

    let pool = SqlitePool::connect(&database_url).await?;

    // Find user
    let user_row = sqlx::query("SELECT id, tenant_id FROM users WHERE email = ?")
        .bind(&args.email)
        .fetch_optional(&pool)
        .await?;

    let (user_id, tenant_id): (String, String) = match user_row {
        Some(row) => (row.get("id"), row.get("tenant_id")),
        None => {
            return Err(SeedError::Validation(format!(
                "User not found: {}. Run ./scripts/complete-user-workflow.sh first.",
                args.email
            )));
        }
    };

    info!("   User ID: {}", user_id);
    info!("   Tenant ID: {}", tenant_id);

    // Reset if requested
    if args.reset {
        info!("🗑️  Resetting synthetic activities...");
        sqlx::query("DELETE FROM synthetic_activities WHERE user_id = ?")
            .bind(&user_id)
            .execute(&pool)
            .await?;
    }

    // Initialize RNG
    let seed = args.seed.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(12345)
    });
    let mut rng = StdRng::seed_from_u64(seed);
    info!("   Random seed: {}", seed);

    // Get sport configurations
    let sport_configs = get_sport_configs();
    let weighted_sports = build_weighted_sports(&sport_configs);

    // Generate activities
    info!(
        "📊 Generating {} activities over {} days...",
        args.count, args.days
    );

    let now = Utc::now();
    let mut activities_by_type: HashMap<&str, u32> = HashMap::new();

    for i in 0..args.count {
        // Select random sport based on weights
        let sport_index = *weighted_sports.choose(&mut rng).unwrap_or(&0);
        let sport = &sport_configs[sport_index];

        // Random date within the range
        let days_ago = rng.gen_range(0..args.days);
        let hour = rng.gen_range(5..21); // 5 AM to 9 PM
        let minute = rng.gen_range(0..60);
        let start_date =
            now - Duration::days(i64::from(days_ago)) - Duration::hours(24 - i64::from(hour))
                + Duration::minutes(i64::from(minute));

        // Generate activity data
        let duration = rng.gen_range(sport.duration_range.0..=sport.duration_range.1);
        let distance = sport
            .distance_range
            .map(|(min, max)| rng.gen_range(min..=max));
        let elevation = sport
            .elevation_range
            .map(|(min, max)| rng.gen_range(min..=max));
        let avg_hr = rng.gen_range(sport.heart_rate_range.0..=sport.heart_rate_range.1);
        let max_hr = avg_hr + rng.gen_range(10..30);
        let calories = Some(rng.gen_range(200..1200));

        // Calculate speed from distance and duration
        let avg_speed = distance.map(|d| d / duration as f64);
        let max_speed = avg_speed.map(|s| s * rng.gen_range(1.15..1.4));

        // Pick activity name
        let name = format!(
            "{} #{}",
            sport.names.choose(&mut rng).unwrap_or(&sport.display_name),
            i + 1
        );

        let activity_id = Uuid::new_v4().to_string();

        // Convert types for database (casts are safe: bounded values)
        #[allow(clippy::cast_possible_wrap)]
        let duration_i64 = duration as i64; // max ~86400 seconds
        #[allow(clippy::cast_possible_wrap)]
        let avg_hr_i32 = avg_hr as i32; // heart rate 50-220 bpm
        #[allow(clippy::cast_possible_wrap)]
        let max_hr_i32 = max_hr as i32;

        // Insert activity
        sqlx::query(
            r"
            INSERT INTO synthetic_activities (
                id, user_id, tenant_id,
                name, sport_type, start_date, duration_seconds,
                distance_meters, elevation_gain,
                average_heart_rate, max_heart_rate,
                average_speed, max_speed, calories,
                city, region, country,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&activity_id)
        .bind(&user_id)
        .bind(&tenant_id)
        .bind(&name)
        .bind(sport.sport_type)
        .bind(start_date.to_rfc3339())
        .bind(duration_i64)
        .bind(distance)
        .bind(elevation)
        .bind(avg_hr_i32)
        .bind(max_hr_i32)
        .bind(avg_speed)
        .bind(max_speed)
        .bind(calories)
        .bind("Montreal")
        .bind("Quebec")
        .bind("Canada")
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(&pool)
        .await?;

        *activities_by_type.entry(sport.sport_type).or_insert(0) += 1;
    }

    // Print summary
    info!("✅ Created {} synthetic activities", args.count);
    info!("");
    info!("📈 Activity breakdown:");

    let mut sorted_types: Vec<_> = activities_by_type.iter().collect();
    sorted_types.sort_by(|a, b| b.1.cmp(a.1));

    for (sport_type, count) in sorted_types {
        info!("   {}: {}", sport_type, count);
    }

    info!("");
    info!("🎯 Login credentials:");
    info!("   Email: {}", args.email);
    info!("   Password: userpass123 (from .envrc)");

    Ok(())
}
