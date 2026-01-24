// ABOUTME: Social data seeder for Pierre MCP Server social features testing
// ABOUTME: Generates friend connections, shared insights, reactions, and adapted insights
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Social data seeder for Pierre MCP Server.
//!
//! This binary populates the database with social demo data for testing
//! the Friends, Feed, and Adapt to My Training features.
//!
//! Usage:
//! ```bash
//! # Seed with default settings
//! cargo run --bin seed-social
//!
//! # Reset social data before seeding
//! cargo run --bin seed-social -- --reset
//!
//! # Verbose output
//! cargo run --bin seed-social -- -v
//! ```
//!
//! Prerequisites:
//! - Run `cargo run --bin seed-demo-data` first to create demo users

use anyhow::Result;
use chrono::{Duration, Utc};
use clap::Parser;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use sqlx::{Row, SqlitePool};
use std::env;
use tracing::info;
use uuid::Uuid;

#[derive(Parser)]
#[command(
    name = "seed-social",
    about = "Pierre MCP Server Social Data Seeder",
    long_about = "Populate the database with social demo data for Friends, Feed, and Adapt features"
)]
struct SeedArgs {
    /// Database URL override
    #[arg(long)]
    database_url: Option<String>,

    /// Reset social data before seeding
    #[arg(long)]
    reset: bool,

    /// Enable verbose logging
    #[arg(long, short = 'v')]
    verbose: bool,
}

/// Shared insight content definitions
struct InsightContent {
    insight_type: &'static str,
    sport_type: Option<&'static str>,
    title: &'static str,
    content: &'static str,
    training_phase: Option<&'static str>,
}

/// Sample insights for achievements and milestones
fn get_achievement_insights() -> Vec<InsightContent> {
    vec![
        InsightContent {
            insight_type: "achievement", sport_type: Some("run"), title: "New Personal Best!",
            content: "Crushed my tempo run today! Coach noted that my aerobic base has really improved over the past month. Feeling strong heading into race season.",
            training_phase: Some("build"),
        },
        InsightContent {
            insight_type: "milestone", sport_type: Some("ride"), title: "1000km Cycling Milestone",
            content: "Hit 1000km on the bike this month! Coach says my endurance foundation is solid and it's time to add some intensity work.",
            training_phase: Some("base"),
        },
        InsightContent {
            insight_type: "achievement", sport_type: Some("strength"), title: "Strength Gains",
            content: "Deadlift PR today! Coach has been emphasizing strength work to complement my endurance training. Feeling the difference on hills.",
            training_phase: Some("build"),
        },
        InsightContent {
            insight_type: "milestone", sport_type: Some("ride"), title: "First Century Complete",
            content: "Completed my first 100-mile ride! Pacing strategy coach suggested worked perfectly. Finished strong with energy to spare.",
            training_phase: Some("peak"),
        },
        InsightContent {
            insight_type: "achievement", sport_type: Some("run"), title: "Race Day Success",
            content: "Negative split my half marathon! Coach's pacing plan was spot on. Started conservative and had so much left for the final miles.",
            training_phase: Some("peak"),
        },
        InsightContent {
            insight_type: "milestone", sport_type: Some("run"), title: "Sub-4 Hour Marathon",
            content: "Broke 4 hours in the marathon! Months of preparation came together perfectly. Trust the process and trust your coach.",
            training_phase: Some("peak"),
        },
    ]
}

/// Sample insights for training tips, recovery, and motivation
fn get_coaching_insights() -> Vec<InsightContent> {
    vec![
        InsightContent {
            insight_type: "training_tip", sport_type: Some("swim"), title: "Drill Focus Paying Off",
            content: "Been focusing on catch drills as coach suggested. Starting to feel more connected to the water. Efficiency improving!",
            training_phase: Some("base"),
        },
        InsightContent {
            insight_type: "recovery", sport_type: None, title: "Active Recovery Week",
            content: "Taking a planned recovery week. Coach reminded me that rest is when adaptation happens. Sleep quality has been great!",
            training_phase: Some("recovery"),
        },
        InsightContent {
            insight_type: "motivation", sport_type: Some("run"), title: "Consistency Is Key",
            content: "14 weeks of consistent training in the books! Coach pointed out that showing up every day matters more than any single workout.",
            training_phase: Some("build"),
        },
        InsightContent {
            insight_type: "training_tip", sport_type: Some("run"), title: "Heart Rate Zone Training",
            content: "Learning to stay in Zone 2 on easy runs was tough at first, but coach was right - my aerobic engine is so much stronger now.",
            training_phase: Some("base"),
        },
        InsightContent {
            insight_type: "motivation", sport_type: Some("swim"), title: "Open Water Confidence",
            content: "Did my first open water swim without anxiety! The visualization techniques coach taught really helped calm my nerves.",
            training_phase: Some("build"),
        },
        InsightContent {
            insight_type: "recovery", sport_type: None, title: "Sleep Quality Focus",
            content: "Been tracking sleep as coach suggested. Turns out my 5:30am workouts were hurting recovery. Shifted to evenings and feeling much better!",
            training_phase: Some("base"),
        },
        InsightContent {
            insight_type: "training_tip", sport_type: Some("ride"), title: "Cadence Work",
            content: "Finally comfortable at 90+ rpm on the bike. Those cadence drills coach programmed felt awkward at first but made a huge difference.",
            training_phase: Some("base"),
        },
        InsightContent {
            insight_type: "motivation", sport_type: None, title: "Community Support",
            content: "Love seeing everyone's progress on here! We're all on different journeys but pushing each other forward. Keep going!",
            training_phase: None,
        },
        InsightContent {
            insight_type: "recovery", sport_type: Some("run"), title: "Managing Minor Setback",
            content: "Dealing with some IT band tightness. Coach adjusted my plan with more mobility work and shorter runs. Smart training over tough training.",
            training_phase: Some("recovery"),
        },
    ]
}

/// Get all sample insights for seeding by combining achievement and coaching insights
fn get_sample_insights() -> Vec<InsightContent> {
    let mut insights = get_achievement_insights();
    insights.extend(get_coaching_insights());
    insights
}

/// Reaction types
const REACTION_TYPES: &[&str] = &["like", "celebrate", "inspire", "support"];

/// Adapted insight content templates
fn get_adaptation_templates() -> Vec<&'static str> {
    vec![
        "Interesting approach! For your current training phase, you might try something similar but with shorter intervals to match your fitness level.",
        "Love this! Given your focus on base building, you could adapt this by keeping the intensity lower but extending the duration.",
        "Great insight! Since you're training for a different distance, consider scaling the effort proportionally to your goal race.",
        "This resonates with my training too. For your recovery week, a lighter version of this approach could work well.",
        "Solid advice! With your higher weekly volume, you might need extra recovery time when incorporating this.",
    ]
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = SeedArgs::parse();

    // Initialize logging
    let log_level = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt().with_env_filter(log_level).init();

    info!("=== Pierre MCP Server Social Data Seeder ===");

    // Load database URL
    let database_url = args
        .database_url
        .or_else(|| env::var("DATABASE_URL").ok())
        .unwrap_or_else(|| "sqlite:./data/users.db".into());

    // Connect directly to SQLite for seeding
    info!("Connecting to database: {}", database_url);
    let connection_url = format!("{database_url}?mode=rwc");
    let pool = SqlitePool::connect(&connection_url).await?;

    // Verify demo users exist
    let user_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE is_admin = 0")
        .fetch_one(&pool)
        .await?;

    if user_count.0 < 5 {
        anyhow::bail!(
            "Not enough demo users found ({}). Run 'cargo run --bin seed-demo-data' first.",
            user_count.0
        );
    }

    // Reset if requested
    if args.reset {
        info!("Resetting social data...");
        reset_social_data(&pool).await?;
    }

    // Get demo user IDs (non-admin)
    let user_ids = get_demo_user_ids(&pool).await?;
    info!("Found {} demo users", user_ids.len());

    // Seed social settings
    info!("Step 1: Creating user social settings...");
    let settings_count = seed_social_settings(&pool, &user_ids).await?;
    info!("  Created {} social settings", settings_count);

    // Seed friend connections
    info!("Step 2: Creating friend connections...");
    let friend_count = seed_friend_connections(&pool, &user_ids).await?;
    info!("  Created {} friend connections", friend_count);

    // Seed shared insights
    info!("Step 3: Creating shared insights...");
    let insight_count = seed_shared_insights(&pool, &user_ids).await?;
    info!("  Created {} shared insights", insight_count);

    // Seed reactions
    info!("Step 4: Creating insight reactions...");
    let reaction_count = seed_reactions(&pool, &user_ids).await?;
    info!("  Created {} reactions", reaction_count);

    // Seed adapted insights
    info!("Step 5: Creating adapted insights...");
    let adapted_count = seed_adapted_insights(&pool, &user_ids).await?;
    info!("  Created {} adapted insights", adapted_count);

    // Print summary
    info!("");
    info!("=== Seeding Complete ===");
    print_summary(&pool).await?;

    Ok(())
}

/// Get demo user IDs
async fn get_demo_user_ids(pool: &SqlitePool) -> Result<Vec<Uuid>> {
    let rows = sqlx::query("SELECT id FROM users WHERE is_admin = 0 ORDER BY created_at")
        .fetch_all(pool)
        .await?;

    let mut ids = Vec::with_capacity(rows.len());
    for row in rows {
        let id_str: String = row.get("id");
        ids.push(Uuid::parse_str(&id_str)?);
    }

    Ok(ids)
}

/// Reset social data tables
async fn reset_social_data(pool: &SqlitePool) -> Result<()> {
    // Order matters due to foreign keys
    sqlx::query("DELETE FROM adapted_insights")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM insight_reactions")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM shared_insights")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM friend_connections")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM user_social_settings")
        .execute(pool)
        .await?;
    Ok(())
}

/// Seed user social settings
async fn seed_social_settings(pool: &SqlitePool, user_ids: &[Uuid]) -> Result<u32> {
    let mut rng = StdRng::from_entropy();
    let mut count: u32 = 0;

    for user_id in user_ids {
        // Check if exists
        let existing: Option<(String,)> =
            sqlx::query_as("SELECT user_id FROM user_social_settings WHERE user_id = ?")
                .bind(user_id.to_string())
                .fetch_optional(pool)
                .await?;

        if existing.is_some() {
            continue;
        }

        let discoverable = i32::from(rng.gen_bool(0.9));
        let visibility = if rng.gen_bool(0.7) {
            "friends_only"
        } else {
            "public"
        };
        let share_types = r#"["run", "ride", "swim"]"#;
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO user_social_settings (user_id, discoverable, default_visibility, share_activity_types, notify_friend_requests, notify_insight_reactions, notify_adapted_insights, created_at, updated_at) \
             VALUES (?, ?, ?, ?, 1, 1, 1, ?, ?)"
        )
        .bind(user_id.to_string())
        .bind(discoverable)
        .bind(visibility)
        .bind(share_types)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        count += 1;
    }

    Ok(count)
}

/// Seed friend connections between demo users
async fn seed_friend_connections(pool: &SqlitePool, user_ids: &[Uuid]) -> Result<u32> {
    let mut rng = StdRng::from_entropy();
    let mut count: u32 = 0;

    // Create connections between adjacent users and some random pairs
    for (i, initiator_id) in user_ids.iter().enumerate() {
        // Connect to next 3 users (with some randomness)
        for offset in 1..=3 {
            let receiver_idx = (i + offset) % user_ids.len();
            if receiver_idx == i {
                continue;
            }

            let receiver_id = &user_ids[receiver_idx];

            // Check if connection already exists in either direction
            let existing: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM friend_connections WHERE \
                 (initiator_id = ? AND receiver_id = ?) OR (initiator_id = ? AND receiver_id = ?)",
            )
            .bind(initiator_id.to_string())
            .bind(receiver_id.to_string())
            .bind(receiver_id.to_string())
            .bind(initiator_id.to_string())
            .fetch_optional(pool)
            .await?;

            if existing.is_some() {
                continue;
            }

            let id = Uuid::new_v4();
            let days_ago: i64 = rng.gen_range(1..30);
            let created_at = (Utc::now() - Duration::days(days_ago)).to_rfc3339();
            let updated_at = created_at.clone();

            // 80% accepted, 15% pending, 5% declined
            let status_roll: u8 = rng.gen_range(0..100);
            let (status, accepted_at) = match status_roll {
                0..=79 => {
                    let accept_time = (Utc::now() - Duration::days(days_ago - 1)).to_rfc3339();
                    ("accepted", Some(accept_time))
                }
                80..=94 => ("pending", None),
                _ => ("declined", None),
            };

            sqlx::query(
                "INSERT INTO friend_connections (id, initiator_id, receiver_id, status, created_at, updated_at, accepted_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(id.to_string())
            .bind(initiator_id.to_string())
            .bind(receiver_id.to_string())
            .bind(status)
            .bind(&created_at)
            .bind(&updated_at)
            .bind(&accepted_at)
            .execute(pool)
            .await?;

            count += 1;
        }
    }

    Ok(count)
}

/// Seed shared insights from demo users
async fn seed_shared_insights(pool: &SqlitePool, user_ids: &[Uuid]) -> Result<u32> {
    let mut rng = StdRng::from_entropy();
    let insights = get_sample_insights();
    let mut count: u32 = 0;

    // Each user shares 1-3 insights
    for user_id in user_ids {
        let num_insights: u32 = rng.gen_range(1..=3);

        for _ in 0..num_insights {
            let insight = &insights[rng.gen_range(0..insights.len())];
            let id = Uuid::new_v4();
            let days_ago: i64 = rng.gen_range(1..14);
            let created_at = (Utc::now() - Duration::days(days_ago)).to_rfc3339();
            let visibility = if rng.gen_bool(0.8) {
                "friends_only"
            } else {
                "public"
            };

            let result = sqlx::query(
                "INSERT INTO shared_insights (id, user_id, visibility, insight_type, sport_type, content, title, training_phase, reaction_count, adapt_count, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, 0, ?, ?)"
            )
            .bind(id.to_string())
            .bind(user_id.to_string())
            .bind(visibility)
            .bind(insight.insight_type)
            .bind(insight.sport_type)
            .bind(insight.content)
            .bind(insight.title)
            .bind(insight.training_phase)
            .bind(&created_at)
            .bind(&created_at)
            .execute(pool)
            .await;

            if result.is_ok() {
                count += 1;
            }
        }
    }

    Ok(count)
}

/// Seed reactions on shared insights
async fn seed_reactions(pool: &SqlitePool, user_ids: &[Uuid]) -> Result<u32> {
    let mut rng = StdRng::from_entropy();
    let mut count: u32 = 0;

    // Get all shared insights
    let insights: Vec<(String,)> = sqlx::query_as("SELECT id FROM shared_insights")
        .fetch_all(pool)
        .await?;

    for (insight_id,) in &insights {
        // 50-80% of users react to each insight
        let react_probability: f64 = rng.gen_range(0.3..0.6);

        for user_id in user_ids {
            if !rng.gen_bool(react_probability) {
                continue;
            }

            // Check if reaction already exists
            let existing: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM insight_reactions WHERE insight_id = ? AND user_id = ?",
            )
            .bind(insight_id)
            .bind(user_id.to_string())
            .fetch_optional(pool)
            .await?;

            if existing.is_some() {
                continue;
            }

            let id = Uuid::new_v4();
            let reaction_type = REACTION_TYPES[rng.gen_range(0..REACTION_TYPES.len())];
            let created_at = Utc::now().to_rfc3339();

            let result = sqlx::query(
                "INSERT INTO insight_reactions (id, insight_id, user_id, reaction_type, created_at) \
                 VALUES (?, ?, ?, ?, ?)"
            )
            .bind(id.to_string())
            .bind(insight_id)
            .bind(user_id.to_string())
            .bind(reaction_type)
            .bind(&created_at)
            .execute(pool)
            .await;

            if result.is_ok() {
                count += 1;
            }
        }
    }

    Ok(count)
}

/// Seed adapted insights
async fn seed_adapted_insights(pool: &SqlitePool, user_ids: &[Uuid]) -> Result<u32> {
    let mut rng = StdRng::from_entropy();
    let templates = get_adaptation_templates();
    let mut count: u32 = 0;

    // Get all shared insights
    let insights: Vec<(String, String)> = sqlx::query_as("SELECT id, user_id FROM shared_insights")
        .fetch_all(pool)
        .await?;

    for (insight_id, author_id) in &insights {
        // 20-40% of users adapt each insight (not including author)
        let adapt_probability: f64 = rng.gen_range(0.1..0.25);

        for user_id in user_ids {
            // Skip the author
            if user_id.to_string() == *author_id {
                continue;
            }

            if !rng.gen_bool(adapt_probability) {
                continue;
            }

            // Check if adaptation already exists
            let existing: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM adapted_insights WHERE source_insight_id = ? AND user_id = ?",
            )
            .bind(insight_id)
            .bind(user_id.to_string())
            .fetch_optional(pool)
            .await?;

            if existing.is_some() {
                continue;
            }

            let id = Uuid::new_v4();
            let adapted_content = templates[rng.gen_range(0..templates.len())];
            let context = r#"{"training_phase": "base", "fitness_level": "intermediate"}"#;
            let was_helpful = i32::from(rng.gen_bool(0.8));
            let created_at = Utc::now().to_rfc3339();

            let result = sqlx::query(
                "INSERT INTO adapted_insights (id, source_insight_id, user_id, adapted_content, adaptation_context, was_helpful, created_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(id.to_string())
            .bind(insight_id)
            .bind(user_id.to_string())
            .bind(adapted_content)
            .bind(context)
            .bind(was_helpful)
            .bind(&created_at)
            .execute(pool)
            .await;

            if result.is_ok() {
                count += 1;
            }
        }
    }

    Ok(count)
}

/// Print summary statistics
async fn print_summary(pool: &SqlitePool) -> Result<()> {
    let counts = [
        (
            "Friend Connections",
            "SELECT COUNT(*) FROM friend_connections",
        ),
        (
            "  - Accepted",
            "SELECT COUNT(*) FROM friend_connections WHERE status = 'accepted'",
        ),
        (
            "  - Pending",
            "SELECT COUNT(*) FROM friend_connections WHERE status = 'pending'",
        ),
        (
            "User Social Settings",
            "SELECT COUNT(*) FROM user_social_settings",
        ),
        ("Shared Insights", "SELECT COUNT(*) FROM shared_insights"),
        (
            "  - Achievements",
            "SELECT COUNT(*) FROM shared_insights WHERE insight_type = 'achievement'",
        ),
        (
            "  - Milestones",
            "SELECT COUNT(*) FROM shared_insights WHERE insight_type = 'milestone'",
        ),
        (
            "  - Training Tips",
            "SELECT COUNT(*) FROM shared_insights WHERE insight_type = 'training_tip'",
        ),
        (
            "Insight Reactions",
            "SELECT COUNT(*) FROM insight_reactions",
        ),
        ("Adapted Insights", "SELECT COUNT(*) FROM adapted_insights"),
    ];

    for (label, query) in counts {
        let row: (i64,) = sqlx::query_as(query).fetch_one(pool).await?;
        info!("{}: {}", label, row.0);
    }

    info!("");
    info!("Done! Social data is ready for testing.");

    Ok(())
}
