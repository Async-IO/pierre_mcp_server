# AGENTS.md - Pierre MCP Server

> A guide for AI coding agents working with this codebase.

## Agent Persona

You are an expert Rust backend engineer working on a multi-tenant fitness intelligence platform. The codebase implements the Model Context Protocol (MCP) and Agent-to-Agent (A2A) protocols for AI agent integration with fitness data providers.

## Project Overview

**Pierre MCP Server** is a Rust-based fitness data aggregation and intelligence platform that:
- Integrates with fitness providers (Strava, Garmin, Fitbit, WHOOP, COROS, Terra)
- Provides sports science analytics (training load, performance metrics, recovery scoring)
- Implements MCP protocol for AI agent tool access
- Supports multi-tenant architecture with OAuth 2.0 authentication

## Tech Stack

| Component | Technology | Version |
|-----------|-----------|---------|
| Language | Rust | 2024 edition |
| Runtime | Tokio | async multi-threaded |
| Web Framework | Axum | latest |
| Database | SQLx | SQLite/PostgreSQL |
| Frontend | React + TypeScript | Vite, Tailwind CSS |
| SDK | TypeScript/Node.js | esbuild |

## Commands

### Build & Check

```bash
# Format code (always run first)
cargo fmt

# Quick compile check (no linting)
cargo check --quiet

# Full build
cargo build --release
```

### Linting (Strict Mode Required)

```bash
# REQUIRED before any commit - includes test files
cargo clippy --all-targets -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery -W clippy::cognitive_complexity

# Architectural validation (checks for banned patterns)
./scripts/architectural-validation.sh
```

### Testing

```bash
# Targeted tests (preferred during development)
cargo test <pattern> -- --nocapture
# Examples:
cargo test test_training_load -- --nocapture
cargo test --test oauth_test -- --nocapture
cargo test intelligence:: -- --nocapture

# Full test suite (only for PRs/merges - takes ~13 minutes)
cargo test

# Full validation script
./scripts/lint-and-test.sh
```

### Server Management

```bash
# Start server (loads .envrc, runs in background)
./bin/start-server.sh

# Stop server
./bin/stop-server.sh

# Health check
curl http://localhost:8081/health
```

### Admin Operations

```bash
# Create admin user
RUST_LOG=info cargo run --bin admin-setup -- create-admin-user --email admin@example.com --password SecurePassword123

# Generate API token
RUST_LOG=info cargo run --bin admin-setup -- generate-token --service my_service --expires-days 30
```

## Project Structure

```
pierre_mcp_server/
├── src/                      # Core Rust library
│   ├── bin/                  # Binaries (pierre-mcp-server, admin-setup)
│   ├── intelligence/         # Sports science algorithms (25 files)
│   ├── providers/            # Fitness provider integrations (20 files)
│   ├── mcp/                  # MCP protocol implementation
│   ├── a2a/                  # Agent-to-agent protocol
│   ├── database/             # Database abstraction (17 files)
│   ├── routes/               # HTTP route handlers (19 files)
│   ├── auth.rs               # JWT authentication
│   ├── oauth2_server/        # OAuth 2.0 server
│   └── errors.rs             # Structured error types
├── tests/                    # Integration tests (204 files)
├── frontend/                 # React dashboard
├── sdk/                      # TypeScript MCP client
├── scripts/                  # Development scripts (25+)
└── .github/workflows/        # CI/CD pipelines (12)
```

## Code Style

### File Headers

Every source file must start with a 2-line `ABOUTME:` comment:

```rust
// ABOUTME: Calculates training stress scores using multiple algorithms
// ABOUTME: Supports TSS, TRIMP, and custom Banister models
```

### Error Handling

Use structured error types. **Never use `anyhow!()`**:

```rust
// CORRECT: Structured error types
return Err(AppError::not_found(format!("User {user_id}")));
return Err(ProviderError::RateLimitExceeded {
    provider: "Strava".to_string(),
    retry_after_secs: 3600,
    limit_type: "Daily quota".to_string(),
});

// FORBIDDEN: anyhow! macro
return Err(anyhow!("User not found")); // DO NOT USE
```

### Ownership & Borrowing

```rust
// PREFER: Borrowing over cloning
fn process_data(data: &str) -> Result<(), AppError>

// PREFER: Iterator chains over loops
let results: Vec<_> = items.iter()
    .filter_map(|item| item.value())
    .collect();

// PREFER: Arc for async shared state
let db = Arc::new(database);
let db_clone = db.clone(); // Clone Arc, not contents
```

### Async Patterns

```rust
// PREFER: async fn over impl Future
async fn fetch_activities(user_id: Uuid) -> Result<Vec<Activity>, AppError>

// PREFER: Structured concurrency
let (activities, profile) = tokio::join!(
    fetch_activities(user_id),
    fetch_profile(user_id)
);
```

## Testing Practices

### Test Coverage Requirements

- Unit tests: Required for all business logic
- Integration tests: Required for API endpoints and database operations
- End-to-end tests: Required for complete workflows

### Test Targeting

```bash
# Find tests for a module
rg "mod_name" tests/ --files-with-matches

# List tests in a file
cargo test --test <test_file> -- --list

# Run with output
cargo test <pattern> -- --nocapture
```

### Test File Organization

Tests are in `/tests/` organized by feature:
- `auth_*.rs` - Authentication tests
- `provider_*.rs` - Provider integration tests
- `intelligence_*.rs` - Analytics algorithm tests
- `mcp_*.rs` - MCP protocol tests

## Git Workflow

### Branch Strategy

- **Feature branches**: Create new branch for features
- **Bug fixes**: Commit directly to main branch
- **Branch naming**: Use descriptive names (e.g., `feat/add-whoop-provider`)

### Commit Guidelines

```bash
# Format before commit
cargo fmt

# Validate before commit
./scripts/architectural-validation.sh
cargo clippy --all-targets -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery

# Commit message style: imperative mood, focus on "why"
git commit -m "Add rate limiting to Strava provider to prevent API quota exhaustion"
```

### Pre-Push Validation

The pre-push hook runs strict clippy validation automatically. Never bypass with `--no-verify`.

## Boundaries

### Always Do

- Run `cargo fmt` before any commit
- Run clippy with `--all-targets` flag
- Use structured error types (`AppError`, `ProviderError`, `DatabaseError`)
- Document `Arc` usage with justification
- Include `ABOUTME:` headers in new files
- Handle all `Result` and `Option` types explicitly

### Ask First

- Reimplementing features from scratch instead of modifying
- Adding new dependencies to `Cargo.toml`
- Modifying database schema
- Changes to authentication/authorization logic
- Removing existing code comments

### Never Do

- Use `anyhow!()` macro in production code
- Use `unwrap()` or `expect()` for runtime errors
- Use `panic!()` outside of tests
- Use `#[allow(clippy::...)]` (except for cast validations)
- Use variable names starting with `_` (except unused parameters)
- Add `--no-verify` to git commands
- Leave placeholder/TODO implementations
- Hard-code magic values
- Name things "new", "improved", "enhanced" (use evergreen names)
- Commit secrets or credentials

## Validation Tiers

### Tier 1: Quick Iteration (during development)

```bash
cargo fmt
cargo check --quiet
cargo test <specific_test> -- --nocapture
```

### Tier 2: Pre-Commit

```bash
cargo fmt
./scripts/architectural-validation.sh
cargo clippy --all-targets -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery
cargo test <module_pattern> -- --nocapture
```

### Tier 3: Pre-PR (full validation)

```bash
./scripts/lint-and-test.sh
```

## Key Files Reference

| Purpose | File |
|---------|------|
| Error types | `src/errors.rs` |
| Configuration | `src/config/environment.rs` |
| Database trait | `src/database/mod.rs` |
| Provider trait | `src/providers/mod.rs` |
| MCP tools | `src/mcp/tools.rs` |
| Intelligence engine | `src/intelligence/mod.rs` |
| HTTP routes | `src/routes/mod.rs` |

## Performance Constraints

- Binary size target: <50MB
- Full test suite: ~13 minutes
- Clippy validation: ~2-3 minutes
- Minimize clone() usage; prefer borrowing
- Use `Vec::with_capacity()` when size known

## Environment Variables

Key variables (see `.envrc.example` for full list):

```bash
DATABASE_URL          # SQLite or PostgreSQL connection
STRAVA_CLIENT_ID      # Strava OAuth credentials
STRAVA_CLIENT_SECRET
JWT_SECRET            # Token signing key
REDIS_URL             # Optional: Redis cache
```

---

*This file follows the [AGENTS.md](https://agents.md/) specification for AI coding agent guidance.*
