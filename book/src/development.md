<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Development Guide

Development workflow, tools, and best practices for Pierre Fitness Platform.

## Table of Contents

- [Quick Start](#quick-start)
- [Port Allocation](#port-allocation)
- [Server Management](#server-management)
- [Claude Code Sessions](#claude-code-sessions)
- [Validation Workflow](#validation-workflow)
- [Testing](#testing)
- [Mobile Development](#mobile-development)
- [Frontend Development](#frontend-development)
- [Admin Tools](#admin-tools)
- [Database](#database)
- [Scripts Reference](#scripts-reference)

---

## Quick Start

```bash
# 1. Start the Pierre server
./bin/start-server.sh

# 2. Verify health
curl http://localhost:8081/health

# 3. Run setup workflow (creates admin, user, tenant)
./scripts/complete-user-workflow.sh

# 4. Load credentials
source .workflow_test_env
echo "JWT Token: ${JWT_TOKEN:0:50}..."
```

---

## Port Allocation

**CRITICAL: Port 8081 is RESERVED for the Pierre MCP Server.**

| Service | Port | Notes |
|---------|------|-------|
| Pierre MCP Server | 8081 | Backend API, health checks, OAuth callbacks |
| Expo/Metro Bundler | 8082 | Mobile dev server (configured in metro.config.js) |
| Web Frontend | 5173 | Vite dev server |

### Mobile Development Warning

When working on `frontend-mobile/`:
- **NEVER run `expo start` without specifying port** - it defaults to 8081
- **ALWAYS use `bun start`** which is configured for port 8082

---

## Server Management

### Startup Scripts

```bash
./bin/start-server.sh     # Start backend (loads .envrc, port 8081)
./bin/stop-server.sh      # Stop backend (graceful shutdown)
./bin/start-frontend.sh   # Start web dashboard (port 5173)
./bin/start-tunnel.sh     # Start Cloudflare tunnel for mobile testing
```

### Manual Startup

```bash
# Backend
cargo run --bin pierre-mcp-server

# Frontend (separate terminal)
cd frontend && bun run dev

# Mobile (separate terminal)
cd frontend-mobile && bun start
```

### Health Check

```bash
curl http://localhost:8081/health
```

---

## Claude Code Sessions

### Session Setup (MANDATORY)

**Run this at the START OF EVERY Claude Code session:**

```bash
./scripts/setup/setup-claude-code-mcp.sh
```

This script automatically:
1. Checks if Pierre server is running (starts it if not)
2. Validates current JWT token in `PIERRE_JWT_TOKEN`
3. Generates fresh 7-day token if expired
4. Updates `.envrc` with new token
5. Verifies MCP endpoint is responding

### Why Required

- JWT tokens expire (24 hours default, 7 days from script)
- `.mcp.json` uses `${PIERRE_JWT_TOKEN}` environment variable
- Expired tokens cause "JWT token signature is invalid" errors

### Manual Token Refresh

```bash
# Generate new 7-day token
cargo run --bin pierre-cli -- token generate --service claude_code --expires-days 7

# Update .envrc
export PIERRE_JWT_TOKEN="<paste_token_here>"

# Reload environment
direnv allow
```

### Linear Session Tracking

Sessions are tracked via Linear issues:

```bash
# Automatic - runs via SessionStart hook
./scripts/linear-session-init.sh

# Manual session commands (via /session skill)
/session              # Show current session status
/session update       # Add work log entry
/session decision     # Document a key decision
/session end          # Add end-of-session summary
```

---

## Validation Workflow

### Pre-Push Validation (Marker-Based)

The pre-push hook uses marker-based validation to avoid SSH timeouts:

```bash
# 1. Run validation (creates marker valid for 15 minutes)
./scripts/ci/pre-push-validate.sh

# 2. Push (hook checks marker)
git push
```

### Tiered Validation Approach

#### Tier 1: Quick Iteration (during development)

```bash
cargo fmt
cargo check --quiet
cargo test <test_name_pattern> -- --nocapture
```

#### Tier 2: Pre-Commit (before committing)

```bash
cargo fmt
./scripts/ci/architectural-validation.sh
cargo clippy --all-targets -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery -W clippy::cognitive_complexity
cargo test <module_pattern> -- --nocapture
```

**CRITICAL:** Always use `--all-targets` with clippy to catch errors in tests.

#### Tier 3: Full Validation (before PR/merge)

```bash
./scripts/ci/lint-and-test.sh
```

### Test Targeting Patterns

```bash
# By test name
cargo test test_training_load

# By test file
cargo test --test intelligence_test

# By module path
cargo test intelligence::

# With output
cargo test <pattern> -- --nocapture
```

---

## Testing

### Targeted Testing (During Development)

```bash
# Run specific test in a file (fastest - ~5-10s)
cargo test --test <test_file> <pattern> -- --nocapture

# Examples:
cargo test --test intelligence_test test_training_load -- --nocapture
cargo test --test store_routes_test test_browse -- --nocapture
```

### Pre-Push Validation

```bash
./scripts/ci/pre-push-validate.sh    # Tiered validation (~1-5 min)
```

### Full Test Suite

```bash
cargo test                        # All tests (~13 min, 647 tests)
./scripts/ci/lint-and-test.sh        # Full CI suite
```

### Finding Related Tests

```bash
# Find test files mentioning your module
rg "mod_name" tests/ --files-with-matches

# List tests in a specific test file
cargo test --test <test_file> -- --list
```

See [testing.md](testing.md) for comprehensive testing documentation.

---

## Mobile Development

### Setup

```bash
cd frontend-mobile
bun install
```

### Running

```bash
# Start Metro bundler on port 8082
bun start

# iOS Simulator
bun run ios

# Android Emulator
bun run android
```

### Validation

```bash
# TypeScript
bun run typecheck

# ESLint
bun run lint

# Unit tests
bun test

# All tiers
../scripts/ci/pre-push-mobile-tests.sh

# E2E tests (requires iOS Simulator)
bun run e2e:build && bun run e2e:test
```

### Testing on Physical Device (Cloudflare Tunnel)

```bash
# From frontend-mobile directory
bun run tunnel           # Start tunnel only
bun run start:tunnel     # Start tunnel AND Expo
bun run tunnel:stop      # Stop tunnel

# After starting tunnel:
# 1. Run `direnv allow` in backend directory
# 2. Restart Pierre server: ./bin/stop-server.sh && ./bin/start-server.sh
# 3. Mobile app connects via tunnel URL
```

---

## Frontend Development

### Setup

```bash
cd frontend
bun install
```

### Running

```bash
bun run dev              # Start Vite dev server (port 5173)
```

### Validation

```bash
# TypeScript
bun run type-check

# ESLint
bun run lint

# Unit tests
bun run test -- --run

# All tiers
../scripts/ci/pre-push-frontend-tests.sh

# E2E tests
bun run test:e2e
```

### Environment

```bash
# Add to .envrc for custom backend URL
export VITE_BACKEND_URL="http://localhost:8081"
```

---

## Admin Tools

### pierre-cli Binary

```bash
# Create admin user for frontend login
cargo run --bin pierre-cli -- user create \
  --email admin@example.com \
  --password SecurePassword123

# Generate API token for a service
cargo run --bin pierre-cli -- token generate \
  --service my_service \
  --expires-days 30

# Generate super admin token (no expiry, all permissions)
cargo run --bin pierre-cli -- token generate \
  --service admin_console \
  --super-admin

# List all admin tokens
cargo run --bin pierre-cli -- token list --detailed

# Revoke a token
cargo run --bin pierre-cli -- token revoke <token_id>
```

### Complete User Workflow

```bash
# Creates admin, user, tenant, and saves JWT token
./scripts/complete-user-workflow.sh

# Load saved credentials
source .workflow_test_env
```

---

## Database

### SQLite (Development)

```bash
# Location
./data/users.db

# Reset
./scripts/fresh-start.sh
```

### PostgreSQL (Production)

```bash
# Test PostgreSQL integration
./scripts/testing/test-postgres.sh
```

See [configuration.md](configuration.md) for database configuration.

---

## Scripts Reference

### Server Scripts (`bin/`)

| Script | Description |
|--------|-------------|
| `start-server.sh` | Start Pierre backend on port 8081 |
| `stop-server.sh` | Stop Pierre backend |
| `start-frontend.sh` | Start web dashboard |
| `start-tunnel.sh` | Start Cloudflare tunnel for mobile |
| `setup-and-start.sh` | Combined setup and start |

### Development Scripts (`scripts/`)

| Script | Description |
|--------|-------------|
| `setup/setup-claude-code-mcp.sh` | **MANDATORY** session setup |
| `fresh-start.sh` | Clean database and start fresh |
| `complete-user-workflow.sh` | Create admin, user, tenant |
| `dev-start.sh` | Development startup |
| `linear-session-init.sh` | Initialize Linear session tracking |

### Validation Scripts

| Script | Description |
|--------|-------------|
| `ci/pre-push-validate.sh` | Marker-based pre-push validation |
| `ci/architectural-validation.sh` | Check architectural patterns |
| `ci/lint-and-test.sh` | Full CI validation suite |
| `ci/pre-push-frontend-tests.sh` | Frontend-specific validation |
| `ci/pre-push-mobile-tests.sh` | Mobile-specific validation |

### Testing Scripts

| Script | Description |
|--------|-------------|
| `testing/test-postgres.sh` | PostgreSQL integration tests (Docker) |
| `testing/run-bridge-tests.sh` | SDK/Bridge test suite |
| `ci/ensure-mcp-compliance.sh` | MCP protocol compliance |

See `scripts/README.md` for complete documentation.

---

## Debugging

### Server Logs

```bash
# Real-time debug logs
RUST_LOG=debug cargo run --bin pierre-mcp-server

# Log to file
./bin/start-server.sh  # logs to server.log
```

### SDK Debugging

```bash
npx pierre-mcp-client@next --server http://localhost:8081 --verbose
```
