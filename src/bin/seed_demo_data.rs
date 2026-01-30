// ABOUTME: Demo data seeder for Pierre MCP Server dashboard testing
// ABOUTME: Generates realistic time-series data for users, API keys, and usage analytics
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Demo data seeder for Pierre MCP Server.
//!
//! This binary populates the database with realistic demo data for testing
//! the dashboard, analytics, and user management features.
//!
//! Usage:
//! ```bash
//! # Seed with default settings (assigns data to first admin user)
//! cargo run --bin seed-demo-data
//!
//! # Seed with specific admin email
//! cargo run --bin seed-demo-data -- --admin-email admin@example.com
//!
//! # Reset database before seeding
//! cargo run --bin seed-demo-data -- --reset
//!
//! # Verbose output
//! cargo run --bin seed-demo-data -- -v
//! ```

use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, Timelike, Utc, Weekday};
use clap::Parser;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use sqlx::{Row, SqlitePool};
use std::env;
use tracing::info;
use uuid::Uuid;

/// Default password for all demo users - allows login for testing.
/// Password: `DemoUser123!`
const DEMO_USER_PASSWORD: &str = "DemoUser123!";

#[derive(Parser)]
#[command(
    name = "seed-demo-data",
    about = "Pierre MCP Server Demo Data Seeder",
    long_about = "Populate the database with realistic demo data for dashboard testing"
)]
struct SeedArgs {
    /// Admin email to assign primary data to (uses first admin if not specified)
    #[arg(long)]
    admin_email: Option<String>,

    /// Database URL override
    #[arg(long)]
    database_url: Option<String>,

    /// Server URL for API calls (default: <http://localhost:8081>)
    #[arg(long)]
    server_url: Option<String>,

    /// Reset usage data before seeding (keeps users and API keys)
    #[arg(long)]
    reset: bool,

    /// Number of days of historical data to generate
    #[arg(long, default_value = "30")]
    days: u32,

    /// Enable verbose logging
    #[arg(long, short = 'v')]
    verbose: bool,
}

/// Demo user configuration
struct DemoUser {
    email: &'static str,
    display_name: &'static str,
    tier: &'static str,
    status: &'static str,
    /// Optional custom password (defaults to `DEMO_USER_PASSWORD` if None)
    password: Option<&'static str>,
}

/// Demo API key configuration
struct DemoApiKey {
    name: &'static str,
    description: &'static str,
    tier: &'static str,
    rate_limit: Option<i32>,
}

/// Demo A2A client configuration
struct DemoA2AClient {
    name: &'static str,
    description: &'static str,
    capabilities: &'static str,
}

/// Tool names for usage generation
const TOOLS: &[&str] = &[
    "get_activities",
    "analyze_workout",
    "get_profile",
    "sync_data",
    "generate_insights",
    "get_goals",
    "update_preferences",
    "get_recommendations",
    "get_heart_rate",
    "get_power_zones",
    "calculate_ftp",
    "predict_race",
    "get_training_load",
    "analyze_sleep",
    "get_nutrition_log",
    "sync_garmin",
    "sync_strava",
    "export_gpx",
    "import_tcx",
    "get_leaderboard",
];

/// A2A tool names
const A2A_TOOLS: &[&str] = &[
    "send_message",
    "analyze_activity",
    "get_recommendations",
    "sync_data",
    "export_report",
];

/// Get demo user definitions (part 1) - extracted for function length
/// Includes visual testing users at the start for easy identification
fn get_demo_users_part1() -> Vec<DemoUser> {
    vec![
        // Visual Testing Users (created first for testing)
        DemoUser {
            email: "webtest@pierre.dev",
            display_name: "Web Test User",
            tier: "professional",
            status: "active",
            password: Some("WebTest123!"),
        },
        DemoUser {
            email: "mobiletest@pierre.dev",
            display_name: "Mobile Test User",
            tier: "professional",
            status: "active",
            password: Some("MobileTest123!"),
        },
        // Regular demo users
        DemoUser {
            email: "alice@acme.com",
            display_name: "Alice Johnson",
            tier: "professional",
            status: "active",
            password: None,
        },
        DemoUser {
            email: "bob@startup.io",
            display_name: "Bob Smith",
            tier: "starter",
            status: "active",
            password: None,
        },
        DemoUser {
            email: "charlie@enterprise.co",
            display_name: "Charlie Brown",
            tier: "enterprise",
            status: "active",
            password: None,
        },
        DemoUser {
            email: "diana@freelance.dev",
            display_name: "Diana Prince",
            tier: "starter",
            status: "active",
            password: None,
        },
        DemoUser {
            email: "eve@pending.com",
            display_name: "Eve Wilson",
            tier: "starter",
            status: "pending",
            password: None,
        },
        DemoUser {
            email: "frank@pending.org",
            display_name: "Frank Miller",
            tier: "starter",
            status: "pending",
            password: None,
        },
        DemoUser {
            email: "grace@suspended.net",
            display_name: "Grace Lee",
            tier: "professional",
            status: "suspended",
            password: None,
        },
        DemoUser {
            email: "henry@techcorp.io",
            display_name: "Henry Zhang",
            tier: "enterprise",
            status: "active",
            password: None,
        },
        DemoUser {
            email: "isabella@fitness.app",
            display_name: "Isabella Martinez",
            tier: "professional",
            status: "active",
            password: None,
        },
        DemoUser {
            email: "james@healthtrack.com",
            display_name: "James OBrien",
            tier: "starter",
            status: "active",
            password: None,
        },
    ]
}

/// Get demo user definitions (part 2) - extracted for function length
fn get_demo_users_part2() -> Vec<DemoUser> {
    vec![
        DemoUser {
            email: "kate@runclub.org",
            display_name: "Kate Williams",
            tier: "starter",
            status: "active",
            password: None,
        },
        DemoUser {
            email: "leo@gym.pro",
            display_name: "Leo Thompson",
            tier: "professional",
            status: "active",
            password: None,
        },
        DemoUser {
            email: "maria@cycling.team",
            display_name: "Maria Garcia",
            tier: "enterprise",
            status: "active",
            password: None,
        },
        DemoUser {
            email: "noah@swim.club",
            display_name: "Noah Davis",
            tier: "starter",
            status: "active",
            password: None,
        },
        DemoUser {
            email: "olivia@yoga.studio",
            display_name: "Olivia Taylor",
            tier: "professional",
            status: "active",
            password: None,
        },
        DemoUser {
            email: "peter@triathlon.org",
            display_name: "Peter Anderson",
            tier: "enterprise",
            status: "active",
            password: None,
        },
        DemoUser {
            email: "quinn@pending.io",
            display_name: "Quinn Roberts",
            tier: "starter",
            status: "pending",
            password: None,
        },
        DemoUser {
            email: "rachel@marathon.run",
            display_name: "Rachel Clark",
            tier: "professional",
            status: "active",
            password: None,
        },
        DemoUser {
            email: "sam@crossfit.box",
            display_name: "Sam Wilson",
            tier: "starter",
            status: "active",
            password: None,
        },
        DemoUser {
            email: "tina@pilates.center",
            display_name: "Tina Brown",
            tier: "professional",
            status: "active",
            password: None,
        },
    ]
}

/// Get combined demo users
fn get_demo_users() -> Vec<DemoUser> {
    let mut users = get_demo_users_part1();
    users.extend(get_demo_users_part2());
    users
}

/// Get demo API key definitions - extracted for function length (part 1)
fn get_demo_api_keys_part1() -> Vec<DemoApiKey> {
    vec![
        DemoApiKey {
            name: "Production API",
            description: "Main production workload",
            tier: "professional",
            rate_limit: Some(10000),
        },
        DemoApiKey {
            name: "Staging Environment",
            description: "Pre-production testing",
            tier: "starter",
            rate_limit: Some(1000),
        },
        DemoApiKey {
            name: "Mobile App Backend",
            description: "iOS and Android API",
            tier: "professional",
            rate_limit: Some(5000),
        },
        DemoApiKey {
            name: "Analytics Pipeline",
            description: "Data processing jobs",
            tier: "enterprise",
            rate_limit: None,
        },
        DemoApiKey {
            name: "Trial Key - Evaluation",
            description: "Testing the platform",
            tier: "trial",
            rate_limit: Some(100),
        },
        DemoApiKey {
            name: "Partner Integration",
            description: "Third-party integration",
            tier: "starter",
            rate_limit: Some(2000),
        },
        DemoApiKey {
            name: "Development",
            description: "Local dev testing",
            tier: "trial",
            rate_limit: Some(500),
        },
        DemoApiKey {
            name: "High Volume Batch",
            description: "Batch processing jobs",
            tier: "enterprise",
            rate_limit: None,
        },
        DemoApiKey {
            name: "Strava Sync",
            description: "Automated Strava activity sync",
            tier: "professional",
            rate_limit: Some(3000),
        },
        DemoApiKey {
            name: "Garmin Connect",
            description: "Garmin device integration",
            tier: "professional",
            rate_limit: Some(3000),
        },
    ]
}

/// Get demo API key definitions - extracted for function length (part 2)
fn get_demo_api_keys_part2() -> Vec<DemoApiKey> {
    vec![
        DemoApiKey {
            name: "Terra Bridge",
            description: "Multi-provider workout imports via Terra",
            tier: "starter",
            rate_limit: Some(1500),
        },
        DemoApiKey {
            name: "Apple Health",
            description: "HealthKit data sync",
            tier: "professional",
            rate_limit: Some(5000),
        },
        DemoApiKey {
            name: "Workout Analyzer",
            description: "AI-powered workout analysis",
            tier: "enterprise",
            rate_limit: None,
        },
        DemoApiKey {
            name: "Recovery Tracker",
            description: "Sleep and recovery metrics",
            tier: "starter",
            rate_limit: Some(1000),
        },
        DemoApiKey {
            name: "Nutrition Logger",
            description: "Meal and calorie tracking",
            tier: "starter",
            rate_limit: Some(800),
        },
        DemoApiKey {
            name: "Training Plan Bot",
            description: "Automated plan generation",
            tier: "professional",
            rate_limit: Some(4000),
        },
        DemoApiKey {
            name: "Race Predictor",
            description: "Performance prediction engine",
            tier: "enterprise",
            rate_limit: None,
        },
        DemoApiKey {
            name: "Social Feed",
            description: "Activity sharing and comments",
            tier: "starter",
            rate_limit: Some(2000),
        },
        DemoApiKey {
            name: "Coaching Dashboard",
            description: "Personal trainer tools",
            tier: "professional",
            rate_limit: Some(6000),
        },
        DemoApiKey {
            name: "Challenge Manager",
            description: "Competition and challenge API",
            tier: "starter",
            rate_limit: Some(1500),
        },
    ]
}

/// Get combined demo API keys
fn get_demo_api_keys() -> Vec<DemoApiKey> {
    let mut keys = get_demo_api_keys_part1();
    keys.extend(get_demo_api_keys_part2());
    keys
}

/// Get demo A2A client definitions - extracted for function length
fn get_demo_a2a_clients() -> Vec<DemoA2AClient> {
    vec![
        DemoA2AClient {
            name: "Claude Desktop",
            description: "AI Assistant Integration",
            capabilities: r#"["chat", "analyze"]"#,
        },
        DemoA2AClient {
            name: "Fitness Bot",
            description: "Automated workout analysis",
            capabilities: r#"["sync", "analyze", "recommend"]"#,
        },
        DemoA2AClient {
            name: "Data Pipeline",
            description: "ETL processing agent",
            capabilities: r#"["sync", "export"]"#,
        },
        DemoA2AClient {
            name: "GPT-4 Fitness Coach",
            description: "OpenAI-powered coaching",
            capabilities: r#"["chat", "recommend", "plan"]"#,
        },
        DemoA2AClient {
            name: "Gemini Analyzer",
            description: "Google AI workout insights",
            capabilities: r#"["analyze", "summarize"]"#,
        },
        DemoA2AClient {
            name: "Slack Bot",
            description: "Team fitness notifications",
            capabilities: r#"["notify", "report"]"#,
        },
        DemoA2AClient {
            name: "Discord Bot",
            description: "Community challenges",
            capabilities: r#"["notify", "leaderboard"]"#,
        },
        DemoA2AClient {
            name: "Zapier Integration",
            description: "Workflow automation",
            capabilities: r#"["sync", "export", "webhook"]"#,
        },
        DemoA2AClient {
            name: "Training Peaks Sync",
            description: "TrainingPeaks data bridge",
            capabilities: r#"["sync", "import", "export"]"#,
        },
        DemoA2AClient {
            name: "Garmin Agent",
            description: "Garmin Connect automation",
            capabilities: r#"["sync", "analyze"]"#,
        },
    ]
}

/// Status codes with realistic distribution (mostly 200s)
fn random_status_code(rng: &mut impl Rng) -> i32 {
    let roll: u8 = rng.gen_range(0..100);
    match roll {
        0..=85 => 200,  // 86% success
        86..=90 => 201, // 5% created
        91..=93 => 400, // 3% bad request
        94..=95 => 401, // 2% unauthorized
        96 => 403,      // 1% forbidden
        97 => 429,      // 1% rate limited
        98 => 500,      // 1% server error
        _ => 502,       // 1% bad gateway
    }
}

/// Generate realistic response time in ms
fn random_response_time(rng: &mut impl Rng, tool: &str) -> i32 {
    // Different tools have different baseline performance
    let base = match tool {
        "get_profile" | "get_goals" => 30,
        "get_activities" | "get_heart_rate" => 80,
        "analyze_workout" | "generate_insights" => 200,
        "sync_data" | "sync_garmin" | "sync_strava" => 500,
        "predict_race" | "calculate_ftp" => 300,
        _ => 100,
    };

    // Add variance (50-150% of base)
    let variance: f64 = rng.gen_range(0.5..1.5);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let result = (f64::from(base) * variance) as i32;
    result.max(20) // Minimum 20ms
}

/// Check if a date is a weekend
fn is_weekend(dt: DateTime<Utc>) -> bool {
    matches!(dt.weekday(), Weekday::Sat | Weekday::Sun)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = SeedArgs::parse();

    // Initialize logging
    let log_level = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt().with_env_filter(log_level).init();

    info!("=== Pierre MCP Server Demo Data Seeder ===");

    // Load database URL
    let database_url = args
        .database_url
        .or_else(|| env::var("DATABASE_URL").ok())
        .unwrap_or_else(|| "sqlite:./data/users.db".into());

    // Connect directly to SQLite for seeding
    info!("Connecting to database: {}", database_url);
    let connection_url = format!("{database_url}?mode=rwc");
    let pool = SqlitePool::connect(&connection_url).await?;

    // Find admin user
    let admin_user = find_admin_user(&pool, args.admin_email.as_deref()).await?;
    info!("Using admin user: {} ({})", admin_user.1, admin_user.0);

    // Get server URL for API calls
    let server_url = args
        .server_url
        .or_else(|| {
            env::var("HTTP_PORT")
                .ok()
                .map(|p| format!("http://localhost:{p}"))
        })
        .unwrap_or_else(|| "http://localhost:8081".to_owned());
    info!("Using server URL: {}", server_url);

    // Reset if requested
    if args.reset {
        info!("Resetting usage data...");
        reset_usage_data(&pool).await?;
    }

    // Seed demo users via registration API (creates proper tenants)
    info!("Step 1: Creating demo users via registration API...");
    let user_ids = seed_demo_users(&pool, &server_url).await?;
    info!("  Created/found {} demo users", user_ids.len());

    // Seed API keys (assign to admin + demo users)
    info!("Step 2: Creating API keys...");
    let api_key_ids = seed_api_keys(&pool, &admin_user.0, &user_ids).await?;
    info!("  Created/found {} API keys", api_key_ids.len());

    // Seed A2A clients
    info!("Step 3: Creating A2A clients...");
    let a2a_client_ids = seed_a2a_clients(&pool, &admin_user.0, &user_ids).await?;
    info!("  Created/found {} A2A clients", a2a_client_ids.len());

    // Generate usage data
    info!("Step 4: Generating API usage data ({} days)...", args.days);
    let usage_count = seed_api_usage(&pool, &api_key_ids, args.days).await?;
    info!("  Generated {} usage records", usage_count);

    // Generate A2A usage data
    info!("Step 5: Generating A2A usage data...");
    let a2a_usage_count = seed_a2a_usage(&pool, &a2a_client_ids, args.days / 2).await?;
    info!("  Generated {} A2A usage records", a2a_usage_count);

    // Summary
    info!("");
    info!("=== Seeding Complete ===");
    print_summary(&pool).await?;

    Ok(())
}

/// Find admin user by email or get first admin
async fn find_admin_user(pool: &SqlitePool, email: Option<&str>) -> Result<(Uuid, String)> {
    let row = if let Some(email) = email {
        sqlx::query("SELECT id, email FROM users WHERE email = ? AND is_admin = 1")
            .bind(email)
            .fetch_optional(pool)
            .await?
    } else {
        sqlx::query("SELECT id, email FROM users WHERE is_admin = 1 ORDER BY created_at LIMIT 1")
            .fetch_optional(pool)
            .await?
    };

    let Some(row) = row else {
        anyhow::bail!(
            "No admin user found. Run 'cargo run --bin pierre-cli -- user create' first."
        );
    };

    let id_str: String = row.get("id");
    let email: String = row.get("email");
    let id = Uuid::parse_str(&id_str)?;

    Ok((id, email))
}

/// Reset usage data tables
async fn reset_usage_data(pool: &SqlitePool) -> Result<()> {
    sqlx::query("DELETE FROM api_key_usage")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM a2a_usage").execute(pool).await?;
    Ok(())
}

/// Seed demo users via the registration API (creates user + tenant properly)
async fn seed_demo_users(pool: &SqlitePool, server_url: &str) -> Result<Vec<Uuid>> {
    let demo_users = get_demo_users();
    let mut user_ids = Vec::new();
    let client = reqwest::Client::new();

    for user in &demo_users {
        // Check if user already exists
        let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE email = ?")
            .bind(user.email)
            .fetch_optional(pool)
            .await?;

        let user_id = if let Some((id_str,)) = existing {
            let id = Uuid::parse_str(&id_str)?;
            info!("  Found existing user: {}", user.email);
            id
        } else {
            // Use the registration API to create user with proper tenant
            let password = user.password.unwrap_or(DEMO_USER_PASSWORD);
            let register_request = serde_json::json!({
                "email": user.email,
                "password": password,
                "display_name": user.display_name
            });

            let response = client
                .post(format!("{server_url}/api/auth/register"))
                .json(&register_request)
                .send()
                .await?;

            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_default();
                anyhow::bail!("Failed to register user {}: {}", user.email, error_text);
            }

            let register_response: serde_json::Value = response.json().await?;
            let user_id_str = register_response["user_id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("No user_id in registration response"))?;
            let id = Uuid::parse_str(user_id_str)?;

            // Update tier and status if different from defaults (starter/active)
            if user.tier != "starter" || user.status != "active" {
                let is_active = i32::from(user.status != "suspended");
                sqlx::query(
                    "UPDATE users SET tier = ?, user_status = ?, is_active = ? WHERE id = ?",
                )
                .bind(user.tier)
                .bind(user.status)
                .bind(is_active)
                .bind(id.to_string())
                .execute(pool)
                .await?;
            }

            info!("  Created user: {} ({})", user.email, user.status);
            id
        };

        user_ids.push(user_id);
    }

    Ok(user_ids)
}

/// Seed API keys
async fn seed_api_keys(pool: &SqlitePool, admin_id: &Uuid, user_ids: &[Uuid]) -> Result<Vec<Uuid>> {
    let api_keys = get_demo_api_keys();
    let mut key_ids = Vec::new();
    let mut rng = StdRng::from_entropy();

    for (i, key) in api_keys.iter().enumerate() {
        // Check if exists
        let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM api_keys WHERE name = ?")
            .bind(key.name)
            .fetch_optional(pool)
            .await?;

        let key_id = if let Some((id_str,)) = existing {
            let id = Uuid::parse_str(&id_str)?;
            info!("  Found existing API key: {}", key.name);
            id
        } else {
            let id = Uuid::new_v4();

            // First 10 keys go to admin, rest distributed to demo users
            let user_id = if i < 10 {
                *admin_id
            } else {
                user_ids[(i - 10) % user_ids.len()]
            };

            let key_prefix = format!("pk_{:08x}", rng.gen::<u32>());
            let key_hash = format!("{:064x}", rng.gen::<u128>());
            let days_ago: i64 = rng.gen_range(5..30);
            let created_at = (Utc::now() - Duration::days(days_ago)).to_rfc3339();

            let expires_at = if key.tier == "trial" {
                Some((Utc::now() + Duration::days(14)).to_rfc3339())
            } else {
                None
            };

            sqlx::query(
                "INSERT INTO api_keys (id, user_id, name, description, key_hash, key_prefix, tier, rate_limit_requests, rate_limit_window_seconds, is_active, expires_at, created_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, 3600, 1, ?, ?)"
            )
            .bind(id.to_string())
            .bind(user_id.to_string())
            .bind(key.name)
            .bind(key.description)
            .bind(&key_hash)
            .bind(&key_prefix)
            .bind(key.tier)
            .bind(key.rate_limit)
            .bind(&expires_at)
            .bind(&created_at)
            .execute(pool)
            .await?;

            info!("  Created API key: {} ({})", key.name, key.tier);
            id
        };

        key_ids.push(key_id);
    }

    Ok(key_ids)
}

/// Seed A2A clients
async fn seed_a2a_clients(
    pool: &SqlitePool,
    admin_id: &Uuid,
    user_ids: &[Uuid],
) -> Result<Vec<Uuid>> {
    let clients = get_demo_a2a_clients();
    let mut client_ids = Vec::new();
    let mut rng = StdRng::from_entropy();

    for (i, client) in clients.iter().enumerate() {
        let existing: Option<(String,)> =
            sqlx::query_as("SELECT id FROM a2a_clients WHERE name = ?")
                .bind(client.name)
                .fetch_optional(pool)
                .await?;

        let client_id = if let Some((id_str,)) = existing {
            let id = Uuid::parse_str(&id_str)?;
            info!("  Found existing A2A client: {}", client.name);
            id
        } else {
            let id = Uuid::new_v4();
            let user_id = if i < 5 {
                *admin_id
            } else {
                user_ids[i % user_ids.len()]
            };
            let public_key = format!("pk_a2a_{:016x}", rng.gen::<u64>());
            let client_secret = format!("{:064x}", rng.gen::<u128>());
            let permissions = r#"["read", "write"]"#;
            let days_ago: i64 = rng.gen_range(10..45);
            let created_at = (Utc::now() - Duration::days(days_ago)).to_rfc3339();
            let updated_at = Utc::now().to_rfc3339();

            sqlx::query(
                "INSERT INTO a2a_clients (id, user_id, name, description, public_key, client_secret, permissions, capabilities, rate_limit_requests, rate_limit_window_seconds, is_active, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1000, 3600, 1, ?, ?)"
            )
            .bind(id.to_string())
            .bind(user_id.to_string())
            .bind(client.name)
            .bind(client.description)
            .bind(&public_key)
            .bind(&client_secret)
            .bind(permissions)
            .bind(client.capabilities)
            .bind(&created_at)
            .bind(&updated_at)
            .execute(pool)
            .await?;

            info!("  Created A2A client: {}", client.name);
            id
        };

        client_ids.push(client_id);
    }

    Ok(client_ids)
}

/// Seed API usage data with realistic patterns
async fn seed_api_usage(pool: &SqlitePool, api_key_ids: &[Uuid], days: u32) -> Result<u64> {
    let mut rng = StdRng::from_entropy();
    let mut total_records: u64 = 0;

    for (idx, key_id) in api_key_ids.iter().enumerate() {
        // Determine base traffic based on key position (enterprise keys get more)
        let base_requests: u32 = if idx < 5 {
            200 // High volume keys
        } else if idx < 10 {
            100 // Medium volume
        } else {
            50 // Lower volume
        };

        for day_offset in 0..days {
            let day = Utc::now() - Duration::days(i64::from(day_offset));

            // Weekend adjustment (30% of normal)
            let weekend_factor: f64 = if is_weekend(day) { 0.3 } else { 1.0 };

            // Random daily variance
            let variance: f64 = rng.gen_range(0.7..1.3);

            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let daily_requests = (f64::from(base_requests) * weekend_factor * variance) as u32;

            for _ in 0..daily_requests {
                let id = Uuid::new_v4();
                let tool = TOOLS[rng.gen_range(0..TOOLS.len())];
                let status_code = random_status_code(&mut rng);
                let response_time = random_response_time(&mut rng, tool);

                // Generate timestamp with business hours bias
                let hour: u32 = if rng.gen_bool(0.7) {
                    rng.gen_range(8..20) // 70% during business hours
                } else {
                    rng.gen_range(0..24)
                };
                let minute: u32 = rng.gen_range(0..60);
                let second: u32 = rng.gen_range(0..60);

                let timestamp = day
                    .with_hour(hour)
                    .unwrap_or(day)
                    .with_minute(minute)
                    .unwrap_or(day)
                    .with_second(second)
                    .unwrap_or(day)
                    .to_rfc3339();

                // Ignore errors for duplicate inserts
                let result = sqlx::query(
                    "INSERT INTO api_key_usage (id, api_key_id, timestamp, tool_name, status_code, response_time_ms) \
                     VALUES (?, ?, ?, ?, ?, ?)"
                )
                .bind(id.to_string())
                .bind(key_id.to_string())
                .bind(&timestamp)
                .bind(tool)
                .bind(status_code)
                .bind(response_time)
                .execute(pool)
                .await;

                if result.is_ok() {
                    total_records += 1;
                }
            }
        }
        info!("  Generated usage for key: {}...", &key_id.to_string()[..8]);
    }

    Ok(total_records)
}

/// Seed A2A usage data
async fn seed_a2a_usage(pool: &SqlitePool, client_ids: &[Uuid], days: u32) -> Result<u64> {
    let mut rng = StdRng::from_entropy();
    let mut total_records: u64 = 0;

    for client_id in client_ids {
        let base_requests: u32 = rng.gen_range(20..50);

        for day_offset in 0..days {
            let day = Utc::now() - Duration::days(i64::from(day_offset));
            let daily_requests: u32 = rng.gen_range(base_requests / 2..base_requests * 2);

            for _ in 0..daily_requests {
                let id = Uuid::new_v4();
                let tool = A2A_TOOLS[rng.gen_range(0..A2A_TOOLS.len())];
                let status_code = random_status_code(&mut rng);
                let response_time: i32 = rng.gen_range(100..600);

                let hour: u32 = rng.gen_range(0..24);
                let minute: u32 = rng.gen_range(0..60);

                let timestamp = day
                    .with_hour(hour)
                    .unwrap_or(day)
                    .with_minute(minute)
                    .unwrap_or(day)
                    .to_rfc3339();

                let result = sqlx::query(
                    "INSERT INTO a2a_usage (id, client_id, timestamp, tool_name, status_code, response_time_ms, protocol_version) \
                     VALUES (?, ?, ?, ?, ?, ?, '1.0')"
                )
                .bind(id.to_string())
                .bind(client_id.to_string())
                .bind(&timestamp)
                .bind(tool)
                .bind(status_code)
                .bind(response_time)
                .execute(pool)
                .await;

                if result.is_ok() {
                    total_records += 1;
                }
            }
        }
    }

    Ok(total_records)
}

/// Print visual testing credentials
fn print_test_credentials() {
    info!(
        "\n\
         === Visual Testing Credentials ===\n\
         Web Test User:    webtest@pierre.dev / WebTest123!\n\
         Mobile Test User: mobiletest@pierre.dev / MobileTest123!\n\
         Demo Users:       DemoUser123! (for alice@acme.com, bob@startup.io, etc.)\n\
         \n\
         Done! Restart the server to see the demo data in the dashboard."
    );
}

/// Print summary statistics
async fn print_summary(pool: &SqlitePool) -> Result<()> {
    print_count(pool, "Users", "SELECT COUNT(*) FROM users").await?;
    print_count(pool, "API Keys", "SELECT COUNT(*) FROM api_keys").await?;
    print_count(
        pool,
        "API Usage Records",
        "SELECT COUNT(*) FROM api_key_usage",
    )
    .await?;
    print_count(pool, "A2A Clients", "SELECT COUNT(*) FROM a2a_clients").await?;
    print_count(pool, "A2A Usage Records", "SELECT COUNT(*) FROM a2a_usage").await?;
    print_count(
        pool,
        "Pending Users",
        "SELECT COUNT(*) FROM users WHERE user_status = 'pending'",
    )
    .await?;

    print_test_credentials();
    Ok(())
}

/// Helper to print a single count query result
async fn print_count(pool: &SqlitePool, label: &str, query: &str) -> Result<()> {
    let row: (i64,) = sqlx::query_as(query).fetch_one(pool).await?;
    info!("{}: {}", label, row.0);
    Ok(())
}
