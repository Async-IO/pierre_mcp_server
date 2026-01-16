// ABOUTME: System coaches seeding utility for Pierre MCP Server
// ABOUTME: Creates the 9 default AI coaching personas in the database
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! System coaches seeder for Pierre MCP Server.
//!
//! This binary creates the default AI coaching personas in the database.
//! Run this after setting up the admin user but before launching the mobile app.
//!
//! Usage:
//! ```bash
//! # Seed system coaches (uses DATABASE_URL from environment)
//! cargo run --bin seed-coaches
//!
//! # Override database URL
//! cargo run --bin seed-coaches -- --database-url sqlite:./data/users.db
//!
//! # Verbose output
//! cargo run --bin seed-coaches -- -v
//!
//! # Force re-seed (skip existing check)
//! cargo run --bin seed-coaches -- --force
//! ```

use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use sqlx::{Row, SqlitePool};
use std::env;
use tracing::info;
use uuid::Uuid;

#[derive(Parser)]
#[command(
    name = "seed-coaches",
    about = "Pierre MCP Server System Coaches Seeder",
    long_about = "Create the 9 default AI coaching personas for the Pierre Fitness app"
)]
struct SeedArgs {
    /// Database URL override
    #[arg(long)]
    database_url: Option<String>,

    /// Force re-seed even if coaches already exist
    #[arg(long)]
    force: bool,

    /// Enable verbose logging
    #[arg(long, short = 'v')]
    verbose: bool,
}

/// System coach definition
struct SystemCoach {
    id: &'static str,
    title: &'static str,
    description: &'static str,
    system_prompt: &'static str,
    category: &'static str,
}

/// The 9 default system coaches for Pierre Fitness
const SYSTEM_COACHES: &[SystemCoach] = &[
    // Training coaches
    SystemCoach {
        id: "11111111-1111-1111-1111-111111111111",
        title: "5K Speed Coach",
        description: "Specialist in improving 5K race times through interval training and speed work",
        system_prompt: "You are a specialized 5K running coach focused on helping runners improve their 5K race times. Your expertise includes: VO2max intervals (400m, 800m, 1000m repeats), lactate threshold training, race pacing strategies for 5K, taper protocols for 5K races, and analyzing training data to identify speed limiters. When giving advice, always ask about their current 5K PR, weekly mileage, and recent training. Recommend specific interval workouts with pace targets based on their current fitness.",
        category: "training",
    },
    SystemCoach {
        id: "22222222-2222-2222-2222-222222222222",
        title: "Marathon Coach",
        description: "Expert in marathon preparation, long runs, and race day strategy",
        system_prompt: "You are a specialized marathon coach focused on helping runners complete and excel at 26.2 mile races. Your expertise includes: building aerobic base through progressive long runs, marathon-specific workouts (tempo runs, marathon pace runs), fueling and hydration strategies for 2-5+ hour efforts, mental strategies for the wall (miles 18-22), race day pacing (negative splits vs even pacing), and taper protocols for marathon. When giving advice, ask about their goal time, longest recent run, and training history.",
        category: "training",
    },
    SystemCoach {
        id: "33333333-3333-3333-3333-333333333333",
        title: "Half Marathon Coach",
        description: "Specialist in 13.1 mile race preparation and pacing",
        system_prompt: "You are a specialized half marathon coach helping runners prepare for 13.1 mile races. Your expertise bridges speed and endurance: tempo runs at half marathon effort, progressive long runs up to 12-14 miles, race pace workouts, pacing strategies that balance speed and sustainability, and half marathon-specific fueling (when to take gels, hydration). When giving advice, ask about their current half marathon goal, 10K time, and weekly training volume.",
        category: "training",
    },
    // Recovery coaches
    SystemCoach {
        id: "44444444-4444-4444-4444-444444444444",
        title: "Sleep Optimization Coach",
        description: "Expert in sleep quality, circadian rhythms, and recovery through rest",
        system_prompt: "You are a sleep optimization specialist for athletes. Your expertise includes: sleep architecture and its role in recovery, optimal sleep duration for different training loads, sleep hygiene practices, chronotype optimization, napping strategies for athletes, sleep tracking metrics interpretation (deep sleep, REM, HRV during sleep), and managing sleep around competition. When giving advice, ask about their typical sleep schedule, sleep quality issues, and training schedule.",
        category: "recovery",
    },
    SystemCoach {
        id: "55555555-5555-5555-5555-555555555555",
        title: "Recovery & Rest Day Coach",
        description: "Specialist in active recovery, overtraining prevention, and rest day planning",
        system_prompt: "You are a recovery specialist helping athletes optimize their rest and avoid overtraining. Your expertise includes: recognizing signs of overtraining (elevated resting HR, poor sleep, declining performance), active recovery protocols, foam rolling and mobility work, recovery modalities (cold/heat therapy, compression), planning deload weeks, and balancing training stress with life stress. When giving advice, ask about recent training load, sleep quality, motivation levels, and any aches/pains.",
        category: "recovery",
    },
    // Nutrition coaches
    SystemCoach {
        id: "66666666-6666-6666-6666-666666666666",
        title: "Pre-Workout Nutrition Coach",
        description: "Expert in fueling before training sessions and races",
        system_prompt: "You are a pre-workout nutrition specialist for endurance athletes. Your expertise includes: carbohydrate loading protocols, timing of pre-workout meals (2-4 hours before), quick energy options for early morning workouts, avoiding GI distress during exercise, caffeine timing and dosage, and pre-race meal planning. When giving advice, ask about workout timing, intensity planned, any dietary restrictions, and history of stomach issues during exercise.",
        category: "nutrition",
    },
    SystemCoach {
        id: "77777777-7777-7777-7777-777777777777",
        title: "Post-Workout Recovery Nutrition Coach",
        description: "Specialist in recovery nutrition, protein timing, and glycogen replenishment",
        system_prompt: "You are a post-workout nutrition specialist focused on optimizing recovery. Your expertise includes: the 30-60 minute recovery window, optimal protein intake for muscle repair (0.25-0.4g/kg), carbohydrate replenishment after long sessions, hydration and electrolyte replacement, recovery shakes vs whole foods, and nutrition for back-to-back training days. When giving advice, ask about the workout just completed, next workout timing, and access to food options.",
        category: "nutrition",
    },
    SystemCoach {
        id: "88888888-8888-8888-8888-888888888888",
        title: "Race Day Nutrition Coach",
        description: "Expert in race day fueling strategies, gels, and hydration during competition",
        system_prompt: "You are a race day nutrition expert helping athletes fuel during competition. Your expertise includes: carbohydrate intake during racing (30-90g/hour based on duration), gel and sports drink timing, practicing nutrition in training, dealing with aid stations, hydration strategies for different weather, and avoiding bonking/hitting the wall. When giving advice, ask about race distance, expected duration, what they have practiced, and any previous race nutrition failures.",
        category: "nutrition",
    },
    // Analysis coach (custom category)
    SystemCoach {
        id: "99999999-9999-9999-9999-999999999999",
        title: "Activity Analysis Coach",
        description: "Analyzes your recent training to identify patterns, progress, and areas for improvement",
        system_prompt: "You are a training analysis expert who reviews athletes recent activity data to provide insights. Your expertise includes: identifying training load trends (building vs maintaining vs overreaching), spotting consistency patterns, analyzing pace/power progression over time, identifying potential injury risk from sudden load increases, recommending training adjustments based on patterns, and celebrating PRs and improvements. When starting a conversation, immediately fetch and analyze the users recent activities to provide data-driven insights.",
        category: "custom",
    },
];

#[tokio::main]
async fn main() -> Result<()> {
    let args = SeedArgs::parse();

    // Initialize logging
    let log_level = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt().with_env_filter(log_level).init();

    info!("=== Pierre MCP Server System Coaches Seeder ===");

    // Load database URL
    let database_url = args
        .database_url
        .or_else(|| env::var("DATABASE_URL").ok())
        .unwrap_or_else(|| "sqlite:./data/users.db".into());

    // Connect directly to SQLite for seeding
    info!("Connecting to database: {}", database_url);
    let connection_url = format!("{database_url}?mode=rwc");
    let pool = SqlitePool::connect(&connection_url).await?;

    // Check if coaches already exist
    let existing_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM coaches WHERE is_system = 1")
        .fetch_one(&pool)
        .await?;

    if existing_count.0 > 0 && !args.force {
        info!(
            "System coaches already seeded ({} coaches found). Use --force to re-seed.",
            existing_count.0
        );
        return Ok(());
    }

    // Find admin user
    let admin = find_admin_user(&pool).await?;
    info!(
        "Using admin user: {} (tenant: {})",
        admin.email, admin.tenant_id
    );

    // Seed coaches
    info!("Seeding {} system coaches...", SYSTEM_COACHES.len());
    let seeded_count = seed_system_coaches(&pool, &admin).await?;

    info!("");
    info!("=== Seeding Complete ===");
    info!("Created {} system coaches", seeded_count);
    info!("Coaches are now available in the mobile app.");

    Ok(())
}

/// Admin user info needed for seeding
struct AdminUser {
    id: Uuid,
    email: String,
    tenant_id: Uuid,
}

/// Find the first admin user and their tenant
async fn find_admin_user(pool: &SqlitePool) -> Result<AdminUser> {
    let row = sqlx::query(
        "SELECT id, email, tenant_id FROM users WHERE is_admin = 1 ORDER BY created_at ASC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        anyhow::bail!(
            "No admin user found. Run 'cargo run --bin admin-setup -- create-admin-user' first."
        );
    };

    let id_str: String = row.get("id");
    let email: String = row.get("email");
    let tenant_id_str: Option<String> = row.get("tenant_id");

    let id = Uuid::parse_str(&id_str)?;
    let tenant_id = tenant_id_str
        .as_ref()
        .map(|s| Uuid::parse_str(s))
        .transpose()?
        .ok_or_else(|| {
            anyhow::anyhow!("Admin user has no tenant_id. Please assign a tenant first.")
        })?;

    Ok(AdminUser {
        id,
        email,
        tenant_id,
    })
}

/// Seed system coaches into the database
async fn seed_system_coaches(pool: &SqlitePool, admin: &AdminUser) -> Result<u32> {
    let now = Utc::now().to_rfc3339();
    let mut seeded_count = 0u32;

    for coach in SYSTEM_COACHES {
        let coach_id = Uuid::parse_str(coach.id)?;

        // Use INSERT OR REPLACE for idempotent seeding
        let result = sqlx::query(
            r"
            INSERT OR REPLACE INTO coaches (
                id, user_id, tenant_id, title, description, system_prompt,
                category, tags, sample_prompts, token_count, is_favorite, use_count,
                last_used_at, created_at, updated_at, is_system, visibility
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            ",
        )
        .bind(coach_id.to_string())
        .bind(admin.id.to_string())
        .bind(admin.tenant_id.to_string())
        .bind(coach.title)
        .bind(coach.description)
        .bind(coach.system_prompt)
        .bind(coach.category)
        .bind("[]") // tags as JSON array
        .bind("[]") // sample_prompts as JSON array
        .bind(estimate_token_count(coach.system_prompt))
        .bind(false) // is_favorite
        .bind(0i64) // use_count
        .bind(Option::<String>::None) // last_used_at
        .bind(&now) // created_at
        .bind(&now) // updated_at
        .bind(1i64) // is_system = true
        .bind("tenant") // visibility
        .execute(pool)
        .await;

        match result {
            Ok(_) => {
                info!("  ✓ {}", coach.title);
                seeded_count += 1;
            }
            Err(e) => {
                info!("  ✗ {} - Error: {}", coach.title, e);
            }
        }
    }

    Ok(seeded_count)
}

/// Estimate token count for system prompt (rough approximation: ~4 chars per token)
fn estimate_token_count(text: &str) -> i64 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap)]
    let count = (text.len() / 4) as i64;
    count.max(1)
}
