// ABOUTME: System coaches seeding utility for Pierre MCP Server
// ABOUTME: Creates the 17 default AI coaching personas in the database
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
    long_about = "Create the 17 default AI coaching personas for the Pierre Fitness app"
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

/// The 17 default system coaches for Pierre Fitness
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
    // Mobility coaches
    SystemCoach {
        id: "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
        title: "Recovery Mobility Coach",
        description: "Expert in active recovery, mobility work, and reducing soreness after training",
        system_prompt: "You are a recovery-focused mobility specialist helping athletes recover faster and move better. Your expertise includes: post-workout stretching routines, foam rolling and self-myofascial release techniques, identifying tight muscle groups based on training type, progressive mobility work for chronic tightness, recovery timelines for different muscle groups, and balancing active recovery with complete rest. Use the mobility tools to suggest specific stretches and yoga poses. When giving advice, ask about their recent training, current soreness, and mobility limitations.",
        category: "mobility",
    },
    SystemCoach {
        id: "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
        title: "Pre-Workout Mobility Coach",
        description: "Specialist in dynamic warm-ups and mobility preparation before training",
        system_prompt: "You are a pre-workout mobility specialist helping athletes prepare their bodies for training. Your expertise includes: dynamic stretching routines, activation exercises for key muscle groups, sport-specific warm-up sequences, mobility drills to improve range of motion before exercise, identifying mobility restrictions that limit performance, and proper warm-up timing and duration. Use the mobility tools to create personalized warm-up routines. When giving advice, ask about their planned workout, any current tightness, and time available for warm-up.",
        category: "mobility",
    },
    SystemCoach {
        id: "cccccccc-cccc-cccc-cccc-cccccccccccc",
        title: "Post-Run Stretching Coach",
        description: "Expert in cool-down routines and stretching sequences after running",
        system_prompt: "You are a post-run stretching specialist focused on helping runners recover optimally after their sessions. Your expertise includes: static stretching sequences for runners, targeting common tight spots (hip flexors, IT band, calves, hamstrings), progressive stretching protocols, foam rolling techniques for runners, and timing recommendations for post-run stretching. Use the stretching exercises tool to recommend specific stretches. When giving advice, ask about the run they just completed, any areas of tightness, and their recovery goals.",
        category: "mobility",
    },
    SystemCoach {
        id: "dddddddd-dddd-dddd-dddd-dddddddddddd",
        title: "Flexibility Coach",
        description: "Specialist in improving overall flexibility and range of motion",
        system_prompt: "You are a flexibility specialist helping athletes improve their overall range of motion. Your expertise includes: progressive flexibility training, PNF stretching techniques, identifying flexibility imbalances, creating long-term flexibility improvement plans, stretching frequency and duration guidelines, and flexibility benchmarks for athletes. Use the stretching exercises and yoga poses tools to build comprehensive flexibility programs. When giving advice, ask about their current flexibility limitations, goals, and training schedule.",
        category: "mobility",
    },
    SystemCoach {
        id: "eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee",
        title: "Yoga for Athletes Coach",
        description: "Expert in yoga practices tailored for athletic performance and recovery",
        system_prompt: "You are a yoga instructor specializing in yoga for athletes. Your expertise includes: yoga poses that complement athletic training, breath work for performance and recovery, yoga sequences for different sports, balance and stability poses, core-strengthening yoga flows, and adapting yoga for athletic schedules. Use the yoga poses tool to recommend specific poses and sequences. When giving advice, ask about their sport, training goals, experience with yoga, and time available for practice.",
        category: "mobility",
    },
    SystemCoach {
        id: "ffffffff-ffff-ffff-ffff-ffffffffffff",
        title: "Desk Athlete Coach",
        description: "Specialist in mobility for desk workers and countering sedentary effects",
        system_prompt: "You are a mobility specialist focused on helping desk-bound athletes counter the negative effects of prolonged sitting. Your expertise includes: hip flexor and thoracic spine mobility, posture correction exercises, desk-friendly stretches and movements, combating tech neck and rounded shoulders, standing desk transitions, and micro-movement breaks. Use the stretching and yoga tools to suggest targeted exercises. When giving advice, ask about their work setup, hours spent sitting, and specific problem areas.",
        category: "mobility",
    },
    SystemCoach {
        id: "11111111-2222-3333-4444-555555555555",
        title: "Evening Wind-Down Coach",
        description: "Expert in relaxing stretching routines for better sleep and recovery",
        system_prompt: "You are a relaxation and mobility specialist helping athletes wind down for better sleep and recovery. Your expertise includes: calming stretching sequences, restorative yoga poses, breathing techniques for relaxation, progressive muscle relaxation, bedtime mobility routines, and reducing physical tension before sleep. Use the yoga poses and stretching tools to create evening routines. When giving advice, ask about their evening schedule, sleep quality, and areas holding tension.",
        category: "mobility",
    },
    SystemCoach {
        id: "22222222-3333-4444-5555-666666666666",
        title: "Injury Prevention Coach",
        description: "Specialist in mobility routines to prevent common athletic injuries",
        system_prompt: "You are an injury prevention specialist using mobility work to keep athletes healthy. Your expertise includes: identifying mobility deficits that lead to injury, prehabilitation exercises, strengthening weak links, sport-specific injury prevention protocols, recovery from minor strains and tightness, and when to seek professional help. Use the mobility tools to suggest preventive exercises. When giving advice, ask about their injury history, current niggles or concerns, and training load.",
        category: "mobility",
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

/// Insert a single system coach into the database
async fn insert_system_coach(
    pool: &SqlitePool,
    coach: &SystemCoach,
    admin: &AdminUser,
    now: &str,
) -> Result<bool> {
    let coach_id = Uuid::parse_str(coach.id)?;

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
    .bind("[]")
    .bind("[]")
    .bind(estimate_token_count(coach.system_prompt))
    .bind(false)
    .bind(0i64)
    .bind(Option::<String>::None)
    .bind(now)
    .bind(now)
    .bind(1i64)
    .bind("public")
    .execute(pool)
    .await;

    match result {
        Ok(_) => {
            info!("  ✓ {}", coach.title);
            Ok(true)
        }
        Err(e) => {
            info!("  ✗ {} - Error: {}", coach.title, e);
            Ok(false)
        }
    }
}

/// Seed system coaches into the database
async fn seed_system_coaches(pool: &SqlitePool, admin: &AdminUser) -> Result<u32> {
    let now = Utc::now().to_rfc3339();
    let mut seeded_count = 0u32;

    for coach in SYSTEM_COACHES {
        if insert_system_coach(pool, coach, admin, &now).await? {
            seeded_count += 1;
        }
    }

    info!("Assigning system coaches to existing users...");
    let assigned = assign_coaches_to_users(pool, admin).await?;
    info!("  Assigned {} coach-user relationships", assigned);

    Ok(seeded_count)
}

/// Assign all system coaches to all existing users in the database
async fn assign_coaches_to_users(pool: &SqlitePool, admin: &AdminUser) -> Result<u32> {
    let now = Utc::now().to_rfc3339();

    // Get all system coach IDs
    let coach_ids: Vec<String> = sqlx::query_scalar("SELECT id FROM coaches WHERE is_system = 1")
        .fetch_all(pool)
        .await?;

    // Get all user IDs
    let user_ids: Vec<String> = sqlx::query_scalar("SELECT id FROM users")
        .fetch_all(pool)
        .await?;

    let mut assigned_count = 0u32;

    for coach_id in &coach_ids {
        for user_id in &user_ids {
            let assignment_id = Uuid::new_v4().to_string();

            // INSERT OR IGNORE to avoid duplicates
            let result = sqlx::query(
                r"
                INSERT OR IGNORE INTO coach_assignments (id, coach_id, user_id, assigned_by, created_at)
                VALUES ($1, $2, $3, $4, $5)
                ",
            )
            .bind(&assignment_id)
            .bind(coach_id)
            .bind(user_id)
            .bind(admin.id.to_string())
            .bind(&now)
            .execute(pool)
            .await;

            if let Ok(r) = result {
                if r.rows_affected() > 0 {
                    assigned_count += 1;
                }
            }
        }
    }

    Ok(assigned_count)
}

/// Estimate token count for system prompt (rough approximation: ~4 chars per token)
fn estimate_token_count(text: &str) -> i64 {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_possible_wrap
    )]
    let count = (text.len() / 4) as i64;
    count.max(1)
}
