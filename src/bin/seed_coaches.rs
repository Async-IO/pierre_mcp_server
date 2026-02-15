// ABOUTME: System coaches seeding utility for Pierre MCP Server
// ABOUTME: Loads coach definitions from markdown files in coaches/ directory
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Coach Markdown Seeder
//!
//! This binary loads coach definitions from markdown files and syncs them to the database.
//! Coaches are defined in `coaches/` directory with YAML frontmatter and structured sections.
//!
//! ## Usage
//!
//! ```bash
//! # Seed coaches from markdown files
//! cargo run --bin seed-coaches
//!
//! # Override database URL
//! cargo run --bin seed-coaches -- --database-url sqlite:./data/users.db
//!
//! # Verbose output
//! cargo run --bin seed-coaches -- -v
//!
//! # Dry run (show what would be done)
//! cargo run --bin seed-coaches -- --dry-run
//! ```

use std::cmp::Ordering;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

use chrono::Utc;
use clap::Parser;
use glob::glob;
use sqlx::{Row, SqlitePool};
use thiserror::Error;
use tracing::{debug, info, warn};
use uuid::Uuid;

use pierre_mcp_server::coaches::{parse_coach_file, CoachDefinition, RelatedCoach, RelationType};
use pierre_mcp_server::models::TenantId;

/// CLI-specific error type for the seed binary
#[derive(Error, Debug)]
enum SeedError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("UUID parse error: {0}")]
    Uuid(#[from] uuid::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Glob pattern error: {0}")]
    GlobPattern(#[from] glob::PatternError),

    #[error("Glob error: {0}")]
    Glob(#[from] glob::GlobError),

    #[error("{0}")]
    Validation(String),
}

type SeedResult<T> = Result<T, SeedError>;

#[derive(Parser)]
#[command(
    name = "seed-coaches",
    about = "Pierre MCP Server Coach Seeder",
    long_about = "Load coach definitions from markdown files and sync to database"
)]
struct SeedArgs {
    /// Database URL override
    #[arg(long)]
    database_url: Option<String>,

    /// Path to coaches directory
    #[arg(long, default_value = "coaches")]
    coaches_dir: PathBuf,

    /// Dry run - show what would be done without making changes
    #[arg(long)]
    dry_run: bool,

    /// Enable verbose logging
    #[arg(long, short = 'v')]
    verbose: bool,
}

/// Seeding result statistics
#[derive(Default)]
struct SeedStats {
    created: u32,
    updated: u32,
    unchanged: u32,
    relations_created: u32,
    errors: Vec<String>,
}

impl SeedStats {
    const fn total_processed(&self) -> u32 {
        self.created + self.updated + self.unchanged
    }
}

#[tokio::main]
async fn main() -> SeedResult<()> {
    let args = SeedArgs::parse();

    // Initialize logging
    let log_level = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt().with_env_filter(log_level).init();

    info!("=== Pierre MCP Server Coach Seeder ===");

    if args.dry_run {
        info!("DRY RUN - no changes will be made");
    }

    // Find and parse all coach markdown files
    let coaches = discover_coaches(&args.coaches_dir)?;
    info!("Found {} coach markdown files", coaches.len());

    if coaches.is_empty() {
        warn!("No coach files found in {:?}", args.coaches_dir);
        return Ok(());
    }

    // Load database URL
    let database_url = args
        .database_url
        .or_else(|| env::var("DATABASE_URL").ok())
        .unwrap_or_else(|| "sqlite:./data/users.db".into());

    // Connect to database
    info!("Connecting to database: {}", database_url);
    let connection_url = format!("{database_url}?mode=rwc");
    let pool = SqlitePool::connect(&connection_url).await?;

    // Find admin user
    let admin = find_admin_user(&pool).await?;
    info!(
        "Using admin user: {} (tenant: {})",
        admin.email, admin.tenant_id
    );

    // Pass 1: Upsert coaches
    let (mut stats, slug_to_id) = sync_coaches(&pool, &coaches, &admin, args.dry_run).await;

    // Pass 2: Create relations
    sync_relations(&pool, &coaches, &slug_to_id, &mut stats, args.dry_run).await;

    // Summary
    print_summary(&stats, args.dry_run);

    Ok(())
}

/// Sync all coaches to the database (Pass 1)
async fn sync_coaches(
    pool: &SqlitePool,
    coaches: &[CoachDefinition],
    admin: &AdminUser,
    dry_run: bool,
) -> (SeedStats, HashMap<String, String>) {
    info!("");
    info!("=== Pass 1: Syncing Coaches ===");
    let mut stats = SeedStats::default();
    let mut slug_to_id: HashMap<String, String> = HashMap::new();

    for coach in coaches {
        match upsert_coach(pool, coach, admin, dry_run).await {
            Ok((coach_id, action)) => {
                slug_to_id.insert(coach.frontmatter.name.clone(), coach_id);
                log_upsert_result(&coach.frontmatter.title, &action, &mut stats);
            }
            Err(e) => {
                warn!("  ✗ {} - Error: {}", coach.frontmatter.title, e);
                stats
                    .errors
                    .push(format!("{}: {}", coach.frontmatter.name, e));
            }
        }
    }

    (stats, slug_to_id)
}

/// Log the result of an upsert operation and update stats
fn log_upsert_result(title: &str, action: &UpsertAction, stats: &mut SeedStats) {
    match action {
        UpsertAction::Created => {
            info!("  + {} (created)", title);
            stats.created += 1;
        }
        UpsertAction::Updated => {
            info!("  ~ {} (updated)", title);
            stats.updated += 1;
        }
        UpsertAction::Unchanged => {
            debug!("  = {} (unchanged)", title);
            stats.unchanged += 1;
        }
    }
}

/// Sync coach relations to the database (Pass 2)
async fn sync_relations(
    pool: &SqlitePool,
    coaches: &[CoachDefinition],
    slug_to_id: &HashMap<String, String>,
    stats: &mut SeedStats,
    dry_run: bool,
) {
    info!("");
    info!("=== Pass 2: Syncing Relations ===");

    for coach in coaches {
        process_coach_relations(pool, coach, slug_to_id, stats, dry_run).await;
    }

    log_relations_created(stats.relations_created);
}

/// Process all relations for a single coach
async fn process_coach_relations(
    pool: &SqlitePool,
    coach: &CoachDefinition,
    slug_to_id: &HashMap<String, String>,
    stats: &mut SeedStats,
    dry_run: bool,
) {
    let Some(coach_id) = slug_to_id.get(&coach.frontmatter.name) else {
        return;
    };

    for relation in &coach.sections.related_coaches {
        process_single_relation(
            pool,
            coach_id,
            &coach.frontmatter.name,
            relation,
            slug_to_id,
            stats,
            dry_run,
        )
        .await;
    }
}

/// Log how many relations were created
fn log_relations_created(count: u32) {
    if count > 0 {
        info!("  Created {} relations", count);
    }
}

/// Process a single coach relation
async fn process_single_relation(
    pool: &SqlitePool,
    coach_id: &str,
    coach_name: &str,
    relation: &RelatedCoach,
    slug_to_id: &HashMap<String, String>,
    stats: &mut SeedStats,
    dry_run: bool,
) {
    let Some(related_id) = slug_to_id.get(&relation.slug) else {
        debug!(
            "  Skipping relation {} -> {} (target not found)",
            coach_name, relation.slug
        );
        return;
    };

    if dry_run {
        log_dry_run_relation(coach_name, relation.relation_type, &relation.slug);
        return;
    }

    let relation_created = create_relation(pool, coach_id, related_id, relation.relation_type)
        .await
        .unwrap_or(false);
    if relation_created {
        stats.relations_created += 1;
    }
}

/// Log a relation that would be created in dry run mode
fn log_dry_run_relation(coach_name: &str, relation_type: RelationType, target_slug: &str) {
    info!(
        "  Would create: {} --[{}]--> {}",
        coach_name,
        format!("{relation_type:?}").to_lowercase(),
        target_slug
    );
}

/// Print final summary
fn print_summary(stats: &SeedStats, dry_run: bool) {
    info!("");
    info!("=== Seeding Complete ===");
    log_coach_counts(stats);
    print_errors(&stats.errors);
    log_dry_run_status(dry_run);
}

/// Log the coach processing counts
fn log_coach_counts(stats: &SeedStats) {
    info!(
        "Processed: {} coaches ({} created, {} updated, {} unchanged)",
        stats.total_processed(),
        stats.created,
        stats.updated,
        stats.unchanged
    );
}

/// Print error list if any errors occurred
fn print_errors(errors: &[String]) {
    if errors.is_empty() {
        return;
    }
    warn!("Errors: {}", errors.len());
    for error in errors {
        warn!("  - {}", error);
    }
}

/// Log dry run completion status
fn log_dry_run_status(dry_run: bool) {
    if dry_run {
        info!("DRY RUN complete - no changes were made");
    }
}

/// Discover and parse all coach markdown files
fn discover_coaches(coaches_dir: &Path) -> SeedResult<Vec<CoachDefinition>> {
    let pattern = coaches_dir.join("**/*.md");
    let pattern_str = pattern.to_string_lossy();

    let mut coaches = Vec::new();

    for entry in glob(&pattern_str)? {
        let path = entry?;

        // Skip README files
        if path.file_name().is_some_and(|n| n == "README.md") {
            continue;
        }

        match parse_coach_file(&path) {
            Ok(coach) => {
                debug!("Parsed: {} ({})", coach.frontmatter.name, path.display());
                coaches.push(coach);
            }
            Err(e) => {
                warn!("Failed to parse {}: {}", path.display(), e);
            }
        }
    }

    // Sort by category then name for consistent ordering
    coaches.sort_by(|a, b| {
        let cat_cmp = a
            .frontmatter
            .category
            .as_str()
            .cmp(b.frontmatter.category.as_str());
        if cat_cmp == Ordering::Equal {
            a.frontmatter.name.cmp(&b.frontmatter.name)
        } else {
            cat_cmp
        }
    });

    Ok(coaches)
}

/// Admin user info needed for seeding
struct AdminUser {
    id: Uuid,
    email: String,
    tenant_id: TenantId,
}

/// Find the first admin user and their tenant
async fn find_admin_user(pool: &SqlitePool) -> SeedResult<AdminUser> {
    let row = sqlx::query(
        "SELECT id, email, tenant_id FROM users WHERE is_admin = 1 ORDER BY created_at ASC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Err(SeedError::Validation(
            "No admin user found. Run 'cargo run --bin pierre-cli -- user create' first."
                .to_owned(),
        ));
    };

    let id_str: String = row.get("id");
    let email: String = row.get("email");
    let tenant_id_str: Option<String> = row.get("tenant_id");

    let id = Uuid::parse_str(&id_str)?;
    let tenant_id = tenant_id_str
        .as_ref()
        .map(|s| Uuid::parse_str(s).map(TenantId::from_uuid))
        .transpose()?
        .ok_or_else(|| {
            SeedError::Validation(
                "Admin user has no tenant_id. Please assign a tenant first.".to_owned(),
            )
        })?;

    Ok(AdminUser {
        id,
        email,
        tenant_id,
    })
}

/// Result of upsert operation
enum UpsertAction {
    Created,
    Updated,
    Unchanged,
}

/// Upsert a coach into the database
async fn upsert_coach(
    pool: &SqlitePool,
    coach: &CoachDefinition,
    admin: &AdminUser,
    dry_run: bool,
) -> SeedResult<(String, UpsertAction)> {
    let now = Utc::now().to_rfc3339();
    let slug = &coach.frontmatter.name;

    // Check if coach exists by slug
    let existing: Option<(String, Option<String>)> =
        sqlx::query_as("SELECT id, content_hash FROM coaches WHERE slug = $1 AND tenant_id = $2")
            .bind(slug)
            .bind(admin.tenant_id.to_string())
            .fetch_optional(pool)
            .await?;

    let action = if let Some((existing_id, existing_hash)) = existing {
        // Coach exists - check if content changed
        if existing_hash.as_deref() == Some(&coach.content_hash) {
            return Ok((existing_id, UpsertAction::Unchanged));
        }

        if !dry_run {
            update_coach(pool, &existing_id, coach, &now).await?;
        }
        (existing_id, UpsertAction::Updated)
    } else {
        // New coach
        let new_id = Uuid::new_v4().to_string();

        if !dry_run {
            insert_coach(pool, &new_id, coach, admin, &now).await?;
        }
        (new_id, UpsertAction::Created)
    };

    Ok(action)
}

/// Convert example inputs bullet list to JSON array
fn parse_sample_prompts(example_inputs: Option<&String>) -> String {
    example_inputs.map_or_else(
        || "[]".to_owned(),
        |inputs| {
            let prompts: Vec<&str> = inputs
                .lines()
                .filter_map(|line| {
                    line.trim()
                        .strip_prefix('-')
                        .map(|rest| rest.trim().trim_matches('"'))
                })
                .collect();
            serde_json::to_string(&prompts).unwrap_or_else(|_| "[]".to_owned())
        },
    )
}

/// Insert a new coach
async fn insert_coach(
    pool: &SqlitePool,
    id: &str,
    coach: &CoachDefinition,
    admin: &AdminUser,
    now: &str,
) -> SeedResult<()> {
    let prerequisites_json = serde_json::to_string(&coach.frontmatter.prerequisites)?;
    let tags_json = serde_json::to_string(&coach.frontmatter.tags)?;
    let sample_prompts_json = parse_sample_prompts(coach.sections.example_inputs.as_ref());

    sqlx::query(
        r"
        INSERT INTO coaches (
            id, user_id, tenant_id, title, description, system_prompt,
            category, tags, sample_prompts, token_count, is_favorite, use_count,
            last_used_at, created_at, updated_at, is_system, visibility, is_active,
            slug, purpose, when_to_use, instructions, example_inputs, example_outputs,
            success_criteria, prerequisites, source_file, content_hash, startup_query
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18,
            $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29
        )
        ",
    )
    .bind(id)
    .bind(admin.id.to_string())
    .bind(admin.tenant_id.to_string())
    .bind(&coach.frontmatter.title)
    .bind(&coach.sections.purpose)
    .bind(&coach.sections.instructions) // system_prompt = instructions for compatibility
    .bind(coach.frontmatter.category.as_str())
    .bind(&tags_json)
    .bind(&sample_prompts_json)
    .bind(i64::from(coach.token_count))
    .bind(false)
    .bind(0i64)
    .bind(Option::<String>::None)
    .bind(now)
    .bind(now)
    .bind(1i64) // is_system = true for markdown-defined coaches
    .bind(coach.frontmatter.visibility.as_str())
    .bind(false)
    // Markdown section columns
    .bind(&coach.frontmatter.name)
    .bind(&coach.sections.purpose)
    .bind(&coach.sections.when_to_use)
    .bind(&coach.sections.instructions)
    .bind(&coach.sections.example_inputs)
    .bind(&coach.sections.example_outputs)
    .bind(&coach.sections.success_criteria)
    .bind(&prerequisites_json)
    .bind(&coach.source_file)
    .bind(&coach.content_hash)
    .bind(coach.frontmatter.startup.query.as_deref()) // startup_query
    .execute(pool)
    .await?;

    Ok(())
}

/// Update an existing coach
async fn update_coach(
    pool: &SqlitePool,
    id: &str,
    coach: &CoachDefinition,
    now: &str,
) -> SeedResult<()> {
    let prerequisites_json = serde_json::to_string(&coach.frontmatter.prerequisites)?;
    let tags_json = serde_json::to_string(&coach.frontmatter.tags)?;
    let sample_prompts_json = parse_sample_prompts(coach.sections.example_inputs.as_ref());

    sqlx::query(
        r"
        UPDATE coaches SET
            title = $1, description = $2, system_prompt = $3, category = $4,
            tags = $5, sample_prompts = $6, token_count = $7, updated_at = $8,
            visibility = $9, purpose = $10, when_to_use = $11, instructions = $12,
            example_inputs = $13, example_outputs = $14, success_criteria = $15,
            prerequisites = $16, source_file = $17, content_hash = $18, startup_query = $19
        WHERE id = $20
        ",
    )
    .bind(&coach.frontmatter.title)
    .bind(&coach.sections.purpose)
    .bind(&coach.sections.instructions)
    .bind(coach.frontmatter.category.as_str())
    .bind(&tags_json)
    .bind(&sample_prompts_json)
    .bind(i64::from(coach.token_count))
    .bind(now)
    .bind(coach.frontmatter.visibility.as_str())
    .bind(&coach.sections.purpose)
    .bind(&coach.sections.when_to_use)
    .bind(&coach.sections.instructions)
    .bind(&coach.sections.example_inputs)
    .bind(&coach.sections.example_outputs)
    .bind(&coach.sections.success_criteria)
    .bind(&prerequisites_json)
    .bind(&coach.source_file)
    .bind(&coach.content_hash)
    .bind(coach.frontmatter.startup.query.as_deref()) // startup_query
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Create a relation between two coaches
async fn create_relation(
    pool: &SqlitePool,
    coach_id: &str,
    related_id: &str,
    relation_type: RelationType,
) -> SeedResult<bool> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let relation_str = match relation_type {
        RelationType::Related => "related",
        RelationType::Alternative => "alternative",
        RelationType::Prerequisite => "prerequisite",
        RelationType::Sequel => "sequel",
    };

    let result = sqlx::query(
        r"
        INSERT OR IGNORE INTO coach_relations (id, coach_id, related_coach_id, relation_type, created_at)
        VALUES ($1, $2, $3, $4, $5)
        ",
    )
    .bind(&id)
    .bind(coach_id)
    .bind(related_id)
    .bind(relation_str)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}
