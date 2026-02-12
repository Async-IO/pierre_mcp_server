## Package Manager: bun ONLY

**CRITICAL: This project uses `bun` exclusively for project dependencies. NEVER use `npm`, `yarn`, or `pnpm` for project packages.**

Using npm/yarn for project dependencies will corrupt the project by creating conflicting lock files and inconsistent `node_modules/`.

**Exception**: `npm install -g` is allowed for installing global CLI tools (e.g., `npm install -g @github/copilot`) that are not project dependencies.

### Commands
| Task | Command |
|------|---------|
| Install dependencies | `bun install` |
| Add a package | `bun add <package>` |
| Add dev dependency | `bun add -d <package>` |
| Run scripts | `bun run <script>` |
| Run tests | `bun run test` |

**IMPORTANT: `bun test` vs `bun run test`**
- `bun test` = Bun's native test runner (searches for `.test.ts` files with bun-style tests)
- `bun run test` = Runs the `test` script from package.json (vitest for frontend, jest for mobile)
- **Always use `bun run test`** for frontend/ and frontend-mobile/ directories

### Enforcement
- All `package.json` files have a `preinstall` script that rejects npm/yarn
- `.gitignore` blocks `package-lock.json`, `yarn.lock`, and `pnpm-lock.yaml`
- CI workflows use `bun install --frozen-lockfile`

### If You See Corruption
If you accidentally ran npm and see both `bun.lock` and `package-lock.json`:
```bash
# Remove npm artifacts
rm -rf node_modules package-lock.json
rm -rf */node_modules */package-lock.json

# Reinstall with bun
bun install
```

## Git Workflow: NO Pull Requests

**CRITICAL: NEVER create Pull Requests. All merges happen locally via squash merge.**

### Rules
- **NEVER use `gh pr create`** or any PR creation command
- **NEVER suggest creating a PR**
- Feature branches are merged via **local squash merge**

### Workflow for Features
1. Create feature branch: `git checkout -b feature/my-feature`
2. Make commits, push to remote: `git push -u origin feature/my-feature`
3. When ready, squash merge locally (from main worktree):
   ```bash
   git checkout main
   git fetch origin
   git merge --squash origin/feature/my-feature
   git commit
   git push
   ```

### Bug Fixes
- Bug fixes go directly to `main` branch (no feature branch needed)
- Commit and push directly: `git push origin main`

## Development Quick Start

### Server Management Scripts
Use these shell scripts to manage the Pierre MCP Server:

```bash
# Start the server (loads .envrc, runs in background, shows health check)
./bin/start-server.sh

# Stop the server (graceful shutdown with fallback to force kill)
./bin/stop-server.sh

# Check server health
curl http://localhost:8081/health

# Reset development database (fixes migration checksum mismatches)
./bin/reset-dev-db.sh
```

### Database Reset (Development Only)
If you encounter migration checksum mismatch errors like:
```
migration 20250120000009 was previously applied but has been modified
```

Use the reset script to fix this:
```bash
./bin/reset-dev-db.sh
```

This script:
1. **Safety check**: Refuses to run against non-SQLite databases
2. **Backs up** the current database to `data/backups/`
3. **Deletes and recreates** the database with fresh migrations
4. **Runs all seeders** (admin user, coaches, demo data, social, mobility)

Default credentials after reset:
- Email: `admin@example.com`
- Password: `AdminPassword123`

### Admin User and Token Management
The `pierre-cli` binary manages admin users and API tokens:

```bash
# Create admin user for frontend login
RUST_LOG=info cargo run --bin pierre-cli -- user create --email admin@example.com --password SecurePassword123

# Generate API token for a service
RUST_LOG=info cargo run --bin pierre-cli -- token generate --service my_service --expires-days 30

# Generate super admin token (no expiry, all permissions)
RUST_LOG=info cargo run --bin pierre-cli -- token generate --service admin_console --super-admin

# List all admin tokens
RUST_LOG=warn cargo run --bin pierre-cli -- token list --detailed

# Revoke a token
cargo run --bin pierre-cli -- token revoke <token_id>
```

### OAuth Token Lifecycle
- Strava tokens expire after 6 hours
- The server automatically refreshes expired tokens using stored refresh_token
- Token refresh is transparent to tool execution
- If refresh fails, user must re-authenticate via OAuth flow

## API Keys and Credentials Lookup

When you need an API key or credential for any service:

1. **Check `.envrc`** — all API keys and tokens live here with comments explaining each one. This file is `.gitignore`d and holds the actual secret values.
2. **Check `.mcp.json`** to see which env vars are required — it references them as `${VAR_NAME}` placeholders (this file is committed to git, never contains actual secrets)
3. **If not found in either**, ask the project owner — never guess or fabricate credentials

### MCP-First Rule

When a service is configured in `.mcp.json`, **always use its MCP tools first** instead of CLI alternatives or web APIs. For example:
- **Linear** — use `mcp__linear-server__*` tools, not manual API calls
- **GitHub** — use `mcp__github__*` tools, not `gh` CLI (unless MCP lacks the needed operation)

### Key credentials in `.envrc`:
- `LINEAR_API_KEY` — Linear API key for session/issue tracking
- `GITHUB_PERSONAL_ACCESS_TOKEN` — GitHub PAT for MCP and API access
- `EXPO_TOKEN` — Expo MCP server token
- `STRAVA_CLIENT_ID` / `STRAVA_CLIENT_SECRET` — Strava OAuth credentials
- `PIERRE_JWT_TOKEN` — JWT token for Pierre MCP server auth
- `OPENAI_API_KEY` — OpenAI API key for LLM features

## Development Guides

| Guide | Description |
|-------|-------------|
| [Tool Development](book/src/tool-development.md) | How to create new MCP tools using the pluggable architecture |

## Port Allocation (CRITICAL)

**Port 8081 is RESERVED for the Pierre MCP Server. NEVER start other services on this port.**

| Service | Port | Notes |
|---------|------|-------|
| Pierre MCP Server | 8081 | Backend API, health checks, OAuth callbacks |
| Expo/Metro Bundler | 8082 | Mobile dev server (configured in metro.config.js) |
| Web Frontend | 3000 | Vite dev server |

### Mobile Development Warning
When working on `frontend-mobile/`:
- **NEVER run `expo start` without specifying port** - it defaults to 8081
- **ALWAYS use `bun start`** which is configured for port 8082
- The `metro.config.js` and `package.json` are configured to use port 8082

If you see "Port 8081 is already in use", the Pierre server is running correctly. Use port 8082 for Expo:
```bash
# Correct way to start mobile dev server
cd frontend-mobile && bun start

# If you must use expo directly, specify port
npx expo start --port 8082
```

### Mobile Testing with Cloudflare Tunnels

To test the mobile app on a physical device, use Cloudflare tunnels to expose the local Pierre server:

```bash
# From frontend-mobile directory:
bun run tunnel           # Start tunnel only (outputs URL)
bun run start:tunnel     # Start tunnel AND Expo together
bun run tunnel:stop      # Stop the tunnel
```

**How it works:**
1. The tunnel script starts a Cloudflare tunnel pointing to localhost:8081
2. It updates `BASE_URL` in `.envrc` with the tunnel URL
3. It updates `EXPO_PUBLIC_API_URL` in `frontend-mobile/.env`
4. OAuth callbacks use `BASE_URL` instead of hardcoded localhost

**After starting the tunnel:**
1. Run `direnv allow` in the backend directory
2. Restart the Pierre server: `./bin/stop-server.sh && ./bin/start-server.sh`
3. The mobile app will connect via the tunnel URL

**Environment Variable:**
- `BASE_URL` - When set, OAuth redirect URIs use this instead of `http://localhost:8081`

## Mobile Development (frontend-mobile/)

### Mobile Validation Commands
When working on `frontend-mobile/`, run these validations:

```bash
cd frontend-mobile

# Tier 0: TypeScript (fastest feedback)
bun run typecheck

# Tier 1: ESLint
bun run lint

# Tier 2: Unit tests (~3s, 135 tests)
bun run test

# All tiers at once (what pre-push runs)
../scripts/ci/pre-push-mobile-tests.sh

# E2E tests (requires iOS Simulator, run before PR)
bun run e2e:build && bun run e2e:test
```

### React Native Patterns
- **Styling**: Use NativeWind (Tailwind) classes via `className`, not inline styles
- **State**: React Query for server state, Context API for app state
- **Navigation**: Follow drawer/stack patterns in `src/navigation/`
- **Components**: Reusable UI in `src/components/ui/` (Button, Card, Input)

### TypeScript Requirements
- All files must pass `bun run typecheck` with zero errors
- Use explicit types for component props (no implicit `any`)
- Prefer `unknown` with type guards over `any`

## Web Frontend Development (frontend/)

### Frontend Validation Commands
When working on `frontend/`, run these validations:

```bash
cd frontend

# Tier 0: TypeScript (fastest feedback)
bun run type-check

# Tier 1: ESLint
bun run lint

# Tier 2: Unit tests (~4s)
bun run test -- --run

# All tiers at once (what pre-push runs)
../scripts/ci/pre-push-frontend-tests.sh

# E2E tests (requires browser, run before PR)
bun run test:e2e
```

### Frontend Patterns
- **Styling**: TailwindCSS classes
- **State**: React Query for server state, React Context for app state
- **Components**: Follow existing patterns in `src/components/`

## Git Hooks - MANDATORY for ALL AI Agents

**⚠️ MANDATORY - Run this at the START OF EVERY SESSION:**
```bash
git config core.hooksPath .githooks
```
This enables pre-commit, commit-msg, and pre-push hooks. Sessions get archived/revived, so this must run EVERY time you start working, not just once.

**NEVER use `--no-verify` when committing or pushing.** The hooks enforce:
- SPDX license headers on all source files
- Commit message format (max 2 lines, conventional commits)
- No AI-generated commit signatures (🤖, "Generated with", etc.)
- No claude_docs/ or unauthorized root markdown files
- Frontend/SDK lint and type-check

### Copilot Coding Agent
The `.github/workflows/copilot-setup-steps.yml` file configures the agent's environment before it starts working. It sets `core.hooksPath` and installs dependencies so git hooks run correctly. If hooks fail, fix the underlying issue — do not bypass them.

## Pre-Push Validation Workflow

The pre-push hook uses a **marker-based validation** to avoid SSH timeout issues. Tests run separately from the push.

### Workflow

1. **Make your changes and commit**
2. **Run validation before pushing:**
   ```bash
   ./scripts/ci/pre-push-validate.sh
   ```
   This runs:
   - Tier 0: Code formatting check
   - Tier 1: Architectural validation
   - Tier 2: Schema validation
   - Tier 3: Targeted tests (smart selection based on changed files)
   - Tier 4-6: Frontend/SDK/Mobile tests (if those files changed)

   On success, creates `.git/validation-passed` marker (valid for 15 minutes).

3. **Push:**
   ```bash
   git push
   ```
   The pre-push hook checks:
   - Marker exists
   - Marker is fresh (< 15 minutes)
   - Marker matches current commit (no changes since validation)

### Why This Design

- **Avoids SSH timeout**: Tests run in a separate terminal, not blocking the push
- **Enforces validation**: Can't push without running validation first
- **Prevents stale validation**: Marker expires, must re-validate after changes

### Important Notes

- **Clippy is NOT in `pre-push-validate.sh`** - Claude Code must follow CLAUDE.md and run clippy manually as part of the validation workflow
- If validation expires or commit changes, re-run `./scripts/ci/pre-push-validate.sh`
- To bypass (NOT RECOMMENDED): `git push --no-verify`

### NEVER

- Manually create `.git/validation-passed` marker - always run `./scripts/ci/pre-push-validate.sh`
- Skip validation by creating a fake marker - CI will catch issues and main will break
- Claim "rustfmt isn't installed" or similar excuses to bypass validation

### Before Pushing

1. Run `./scripts/ci/pre-push-validate.sh` to create the validation marker
2. Check CI: `gh run list --branch main` to avoid queueing redundant workflows
3. After push: `gh run watch` to monitor for CI failures

# Writing code

- CRITICAL: NEVER USE --no-verify WHEN COMMITTING CODE
- We prefer simple, clean, maintainable solutions over clever or complex ones, even if the latter are more concise or performant. Readability and maintainability are primary concerns.
- Make the smallest reasonable changes to get to the desired outcome. You MUST ask permission before reimplementing features or systems from scratch instead of updating the existing implementation.
- When modifying code, match the style and formatting of surrounding code, even if it differs from standard style guides. Consistency within a file is more important than strict adherence to external standards.
- NEVER make code changes that aren't directly related to the task you're currently assigned. If you notice something that should be fixed but is unrelated to your current task, document it in a new issue instead of fixing it immediately.
- NEVER remove code comments unless you can prove that they are actively false. Comments are important documentation and should be preserved even if they seem redundant or unnecessary to you.
- All code files should start with a brief 2 line comment explaining what the file does. Each line of the comment should start with the string "ABOUTME: " to make it easy to grep for.
- When writing comments, avoid referring to temporal context about refactors or recent changes. Comments should be evergreen and describe the code as it is, not how it evolved or was recently changed.
- When you are trying to fix a bug or compilation error or any other issue, YOU MUST NEVER throw away the old implementation and rewrite without explicit permission from the user. If you are going to do this, YOU MUST STOP and get explicit permission from the user.
- NEVER name things as 'improved' or 'new' or 'enhanced', etc. Code naming should be evergreen. What is new today will be "old" someday.
- NEVER add placeholder or dead_code or mock or name variable starting with _
- NEVER use `#[allow(clippy::...)]` attributes EXCEPT for type conversion casts (`cast_possible_truncation`, `cast_sign_loss`, `cast_precision_loss`) when properly validated - Fix the underlying issue instead of silencing warnings
- Be RUST idiomatic
- Do not hard code magic value
- Do not leave implementation with "In future versions" or "Implement the code" or "Fall back". Always implement the real thing.
- Commit without AI assistant-related commit messages. Do not reference AI assistance in git commits.
- Do not add AI-generated commit text in commit messages
- Always create a branch when adding new features. Bug fixes go directly to main branch.
- always run validation after making changes: cargo fmt, then ./scripts/ci/architectural-validation.sh, then clippy strict mode, then TARGETED tests (see "Tiered Validation Approach")
- avoid #[cfg(test)] in the src code. Only in tests

## Security Engineering Rules

### Authorization Boundaries
- Authentication (who you are) is NOT authorization (what you can do)
- Every admin/coach/write endpoint MUST check role/permission, not just valid session
- Super-admin token minting MUST require existing super-admin credentials
- API key operations (create/revoke/list) MUST verify ownership via tenant_id

### Multi-Tenant Isolation
- Every database query MUST include `tenant_id` in WHERE clause (no exceptions)
- OAuth tokens, API keys, and LLM credentials are per-tenant — NEVER use global/shared storage
- Cache keys MUST include tenant_id to prevent cross-tenant cache poisoning
- Config write/delete operations MUST verify tenant membership before executing
- Admin tools that modify coach/user data MUST verify target belongs to caller's tenant

### Input Domain Validation
- Any value used as a divisor MUST be checked for zero before division
- Pagination parameters MUST have min/max bounds (e.g., limit clamped to 1..=100)
- Numeric inputs from users MUST be validated against domain-specific ranges
- Use `.max(1)` or equivalent guard before any division operation

### OAuth & Protocol Compliance
- OAuth state parameter MUST be cryptographically random and validated on callback
- PKCE (code_challenge/code_verifier) MUST be enforced for public clients
- Grant types MUST be restricted per-client (reject unregistered grant types)
- Token endpoints MUST validate redirect_uri matches the one used in authorization
- Discovery endpoints (`.well-known/`) MUST return spec-compliant metadata

### Logging Hygiene
- NEVER log: access tokens, refresh tokens, API keys, passwords, client secrets
- Redact or hash sensitive fields before logging (use redaction middleware)
- PII (email, IP, user agent) in logs MUST be at DEBUG level or redacted at INFO+
- Log levels for security events: WARN for auth failures, ERROR for breaches

### Template & Query Safety
- NEVER use `format!()` to build SQL queries — always use parameterized queries (`$1`, `$2`)
- HTML rendered server-side MUST escape all user-supplied values (use `html_escape::encode_text`)
- URL parameters MUST be percent-encoded with `urlencoding::encode()`
- Error messages returned to users MUST NOT contain stack traces or internal details

## Command Permissions

I can run any command WITHOUT permission EXCEPT:
- Commands that delete or overwrite files (rm, mv with overwrite, etc.)
- Commands that modify system state (chmod, chown, sudo)
- Commands with --force flags
- Commands that write to files using > or >>
- In-place file modifications (sed -i, etc.)

Everything else, including all read-only operations and analysis tools, can be run freely.

### Write Permissions
- Writing markdown files is limited to the `claude_docs/` folder under the repo

## Required Pre-Commit Validation

### IMPORTANT: Test Suite Timing Context
- Full test suite: ~13 minutes (647 tests across 163 files)
- Full clippy with tests: ~2 minutes
- Clippy without tests: ~2.5 minutes
- **DO NOT run `cargo test` without targeting** - use targeted tests during development

### Tiered Validation Approach

#### Tier 1: Quick Iteration (during development)
Run after each code change to catch errors fast:
```bash
# 1. Format code
cargo fmt

# 2. Compile check only (fast - no linting)
cargo check --quiet

# 3. Run ONLY tests related to your changes (ALWAYS use --test to avoid compiling all files)
cargo test --test <test_file> <test_name_pattern> -- --nocapture
# Example: cargo test --test intelligence_test test_training_load -- --nocapture
# Example: cargo test --test store_routes_test test_browse_store -- --nocapture
```

#### Tier 2: Pre-Commit (before committing)
Run before creating a commit:
```bash
# 1. Format code
cargo fmt

# 2. Architectural validation
./scripts/ci/architectural-validation.sh

# 3. Clippy — ONLY the crate(s) you actually changed
# Cargo.toml defines all lint levels - no CLI flags needed
#
# IMPORTANT: Run clippy ONLY on crates with actual changes.
# Do NOT run clippy on pierre_mcp_server if you only changed pierre-core.
# Each crate is independent — linting an unchanged crate wastes minutes.
#
# Pick ONE (or more if you changed multiple crates):
cargo clippy -p pierre-core              # models, errors, config
cargo clippy -p pierre-intelligence      # metrics, algorithms
cargo clippy -p pierre-providers         # Strava, Garmin, etc.
cargo clippy -p pierre_mcp_server        # main crate (routes, handlers, etc.)
#
# Add --all-targets ONLY if test files in that crate changed:
cargo clippy -p <changed-crate> --all-targets

# 4. Run TARGETED tests for changed modules (ALWAYS use --test)
cargo test --test <test_file> <test_pattern> -- --nocapture
```

**CRITICAL: Clippy scope must match change scope.** If you only changed `pierre-core`, run `cargo clippy -p pierre-core` — do NOT also run `cargo clippy -p pierre_mcp_server`. The main crate takes minutes to lint; leaf crates take seconds. Only lint what you changed. Use `--all-targets` when test files in that crate changed. Clippy is NOT run by pre-commit or pre-push hooks — it must be run manually.

#### Tier 3: Full Validation (before PR/merge only)
Run the full suite only when preparing a PR or merging:
```bash
./scripts/ci/lint-and-test.sh
# OR manually:
cargo fmt
./scripts/ci/architectural-validation.sh
cargo clippy --workspace --all-targets
cargo test
```

### Test Targeting Patterns

**CRITICAL: Always use `--test <file>` to avoid compiling all 163 test files!**

```bash
# ❌ SLOW - Compiles ALL 163 test files looking for a match
cargo test test_browse_store_with_cursor_pagination

# ✅ FAST - Only compiles the specific test file
cargo test --test store_routes_test test_browse_store_with_cursor_pagination
```

**Always specify the test file:**
```bash
# Format: cargo test --test <test_file_name> <test_name_pattern> -- --nocapture
cargo test --test intelligence_test test_training_load -- --nocapture
cargo test --test oauth_test test_oauth_flow -- --nocapture
cargo test --test store_routes_test test_browse -- --nocapture

# Run all tests in a specific file
cargo test --test intelligence_test -- --nocapture

# List tests in a specific test file (to find test names)
cargo test --test <test_file> -- --list
```

### Finding the Right Test File
When you need to run a test, first find which file contains it:
```bash
# Find test files mentioning your test or module
rg "test_name" tests/ --files-with-matches
rg "mod_name" tests/ --files-with-matches

# Example: find where test_browse_store lives
rg "test_browse_store" tests/ --files-with-matches
# Output: tests/store_routes_test.rs
# Then run: cargo test --test store_routes_test test_browse_store
```

### Test Output Verification - MANDATORY

**After running ANY test command, you MUST verify tests actually ran.**

The exit code alone is NOT sufficient - `cargo test` exits 0 even when 0 tests run.

**Red Flags - STOP and investigate if you see:**
- `running 0 tests` - Wrong target or flag used
- `0 passed; 0 failed` - No tests executed
- `filtered out` with 0 passed - Filter pattern too restrictive

**Verification checklist:**
1. Look for `running N tests` where N > 0
2. Confirm `N passed` in the summary where N > 0
3. If 0 tests ran, the validation FAILED - do not proceed

**Common mistakes that run 0 tests:**
```bash
# ❌ BAD: --lib only runs doc tests in src/, usually 0
cargo test --lib

# ❌ BAD: Typo in test name matches nothing
cargo test --test store_test test_brwose  # typo: brwose

# ✅ GOOD: Targeted test with correct name
cargo test --test store_routes_test test_browse
```

**Never claim "tests pass" if 0 tests ran - that is a failure, not a success.**

## Error Handling Requirements

### Acceptable Error Handling
- `?` operator for error propagation
- `Result<T, E>` for all fallible operations
- `Option<T>` for values that may not exist
- Custom error types implementing `std::error::Error`

### Prohibited Error Handling
- `unwrap()` except in:
  - Test code with clear failure expectations
  - Static data known to be valid at compile time
  - Binary main() functions where failure should crash the program
- `expect()` - Acceptable ONLY for documenting invariants that should never fail:
  - Static/compile-time data: `"127.0.0.1".parse().expect("valid IP literal")`
  - Environment setup in main(): `env::var("DATABASE_URL").expect("DATABASE_URL must be set")`
  - NEVER use expect() for runtime errors that could legitimately occur
- `panic!()` - Only in test assertions or unrecoverable binary errors
- **`anyhow!()` macro** - ABSOLUTELY FORBIDDEN in all production code (src/)
- **`anyhow::anyhow!()` macro** - ABSOLUTELY FORBIDDEN in all production code (src/)
- **ANY form of `anyhow!` macro** - ZERO TOLERANCE - CI will fail on detection

### Structured Error Type Requirements
**CRITICAL: All errors MUST use structured error types, NOT `anyhow::anyhow!()`**

When creating errors, you MUST:
1. **Use project-specific error enums** (e.g., `AppError`, `DatabaseError`, `ProviderError`)
2. **Use `.into()` or `?` for conversion** - let trait implementations handle the conversion
3. **Add context with `.context()`** when needed - but the base error MUST be a structured type
4. **Define new error variants** if no appropriate variant exists in the error enums

#### Correct Error Patterns
```rust
// GOOD: Using structured error types
return Err(AppError::not_found(format!("User {user_id}")));
return Err(DatabaseError::ConnectionFailed { source: e.to_string() }.into());
return Err(ProviderError::RateLimitExceeded {
    provider: "Strava".to_string(),
    retry_after_secs: 3600,
    limit_type: "Daily quota".to_string(),
});

// GOOD: Converting with context
database_operation().context("Failed to fetch user profile")?;
let user = get_user(id).await?; // Let ? operator handle conversion

// GOOD: Mapping external errors to structured types
external_lib_call().map_err(|e| AppError::internal(format!("External API failed: {e}")))?;
```

#### Prohibited Error Anti-Patterns
```rust
// FORBIDDEN: Using anyhow::anyhow!() - NEVER DO THIS
return Err(anyhow::anyhow!("User not found"));

// FORBIDDEN: Using anyhow! macro shorthand - NEVER DO THIS
return Err(anyhow!("Invalid input"));

// FORBIDDEN: In map_err closures - NEVER DO THIS
.map_err(|e| anyhow!("Failed to process: {e}"))?;

// FORBIDDEN: In ok_or_else - NEVER DO THIS
.ok_or_else(|| anyhow!("Value not found"))?;

// FORBIDDEN: Creating ad-hoc string errors - NEVER DO THIS
return Err(anyhow::Error::msg("Something failed"));
```

If no existing error variant fits your use case, add a new variant to the appropriate error enum (`AppError`, `DatabaseError`, `ProviderError`) with proper conversion traits.

## Mock Policy

### Real Implementation Preference
- PREFER real implementations over mocks in all production code
- NEVER implement mock modes for production features

### Acceptable Mock Usage (Test Code Only)
Mocks are permitted ONLY in test code for:
- Testing error conditions that are difficult to reproduce consistently
- Simulating network failures or timeout scenarios
- Testing against external APIs with rate limits during CI/CD
- Simulating hardware failures or edge cases

### Mock Requirements
- All mocks MUST be clearly documented with reasoning
- Mock usage MUST be isolated to test modules only
- Mock implementations MUST be realistic and representative of real behavior
- Tests using mocks MUST also have integration tests with real implementations

## Performance Standards

### Binary Size Constraints
- Target: <50MB for pierre_mcp_server
- Review large dependencies that significantly impact binary size
- Consider feature flags to minimize unused code inclusion
- Document any legitimate exceptions with business justification

### Clone Usage
- Document why each `clone()` is necessary
- Prefer `&T`, `Cow<T>`, or `Arc<T>` over `clone()`
- Justify each clone with ownership requirements analysis

### Arc Usage
- Only use when actual shared ownership required across threads
- Document the sharing requirement in comments
- Consider `Rc<T>` for single-threaded shared ownership
- Prefer `&T` references when data lifetime allows
- **Current count: ~107 Arc usages** - appropriate for multi-tenant async architecture

## Documentation Standards

### Code Documentation
- All public APIs MUST have comprehensive doc comments
- Use `/// ` for public API documentation
- Use `//` for inline implementation comments
- Document error conditions and panic scenarios
- Include usage examples for complex APIs

### Module Documentation
- Each module MUST have a module-level doc comment explaining its purpose
- Document the relationship between modules
- Explain design decisions and trade-offs
- Include architectural diagrams when helpful

### README Requirements
- Keep README.md current with actual functionality
- Include setup instructions that work from a clean environment
- Document all environment variables and configuration options
- Provide troubleshooting section for common issues

### API Documentation
- Generate docs with `cargo doc --no-deps --open`
- Ensure all examples in doc comments compile and run
- Document thread safety guarantees
- Include performance characteristics where relevant

## Task Completion Protocol - MANDATORY

### Before Claiming ANY Task Complete:

1. **Run Tiered Validation (see "Required Pre-Commit Validation" above):**
   - For normal commits: Use Tier 2 (targeted tests)
   - For PRs/merges: Use Tier 3 (full suite via `./scripts/ci/lint-and-test.sh`)

   **Quick reference for targeted validation:**
   ```bash
   cargo fmt
   ./scripts/ci/architectural-validation.sh
   cargo clippy -p <changed-crate>  # Add --all-targets if test files changed
   cargo test --test <test_file> <test_pattern> -- --nocapture
   ```

2. **Manual Pattern Audit:**
   - Search for each banned pattern listed above
   - Justify or eliminate every occurrence
   - Document any exceptions with detailed reasoning

3. **Performance Verification:**
   - Binary size within acceptable limits
   - Memory usage profiling shows no leaks
   - Clone usage minimized and justified
   - Response times within specified limits
   - Benchmarks maintain expected performance

4. **Documentation Review:**
   - All public APIs documented
   - README updated if functionality changed
   - Module docs reflect current architecture
   - Examples compile and work correctly

5. **Architecture Review:**
   - Every Arc usage documented and justified
   - Error handling follows Result patterns throughout
   - No code paths that bypass real implementations

### Failure Criteria
If ANY of the above checks fail, the task is NOT complete regardless of test passing status.

### When Full Test Suite is Required
Run `cargo test` (all tests) ONLY when:
- Creating a PR for review
- Merging to main branch
- Major refactoring touching multiple modules
- CI has failed and you need to reproduce locally

# Getting help

- ALWAYS ask for clarification rather than making assumptions.
- If you're having trouble with something, it's ok to stop and ask for help. Especially if it's something your human might be better at.

# Testing

- Tests MUST cover the functionality being implemented.
- NEVER ignore the output of the system or the tests - Logs and messages often contain CRITICAL information.
- If the logs are supposed to contain errors, capture and test it.
- NO EXCEPTIONS POLICY: Under no circumstances should you mark any test type as "not applicable". Every project, regardless of size or complexity, MUST have unit tests, integration tests, AND end-to-end tests. If you believe a test type doesn't apply, you need the human to say exactly "I AUTHORIZE YOU TO SKIP WRITING TESTS THIS TIME"

## Test Integrity: No Skipping, No Ignoring

**CRITICAL: All tests must run and pass. No exceptions.**

### Forbidden Patterns
- **Rust**: NEVER use `#[ignore]` attribute on tests
- **JavaScript/TypeScript**: NEVER use `.skip()`, `xit()`, `xdescribe()`, or `test.skip()`
- **CI Workflows**: NEVER use `continue-on-error: true` on test jobs
- **Any language**: NEVER comment out tests to make CI pass

### If a Test Fails
1. **Fix the code** - not the test
2. **Fix the test** - only if the test itself is wrong
3. **Ask for help** - if you're stuck, don't skip

### Rationale
Skipped/ignored tests become forgotten tech debt. A red CI that gets ignored is worse than no CI at all.

# RUST IDIOMATIC CODE GENERATION

## Memory Management and Ownership
- PREFER borrowing `&T` over cloning when possible
- PREFER `&str` over `String` for function parameters (unless ownership needed)
- PREFER `&[T]` over `Vec<T>` for function parameters (unless ownership needed)
- PREFER `std::borrow::Cow<T>` for conditionally owned data
- PREFER `AsRef<T>` and `Into<T>` traits for flexible APIs
- NEVER clone Arc contents - clone the Arc itself: `arc.clone()` not `(*arc).clone()`
- Arc/Rc clones are self-documenting and don't need comments
- JUSTIFY non-obvious `.clone()` calls with comments when the reason isn't apparent from context

## Collection and Iterator Patterns
- PREFER iterator chains over manual loops
- USE turbofish `.collect::<Vec<_>>()` when element type is inferred; specify full type when not
- PREFER `filter_map()` over `filter().map()`
- PREFER `and_then()` over nested match statements for Options/Results
- USE `Iterator::fold()` for accumulation, but prefer explicit loops when fold reduces readability
- PREFER `Vec::with_capacity()` when size is known
- USE `HashMap::with_capacity()` when size is known

## String Handling
- PREFER format arguments `format!("{name}")` over concatenation
- PREFER `&'static str` for string constants
- USE `format_args!()` for performance-critical formatting
- PREFER `String::push_str()` over repeated concatenation
- USE `format!()` macro for complex string building

## Async/Await Patterns
- PREFER `async fn` over `impl Future` (clearer, more maintainable)
- USE `tokio::spawn()` for concurrent background tasks; use `.await` for sequential execution
- USE `#[tokio::main]` for async main functions
- PREFER structured concurrency with `tokio::join!()` and `tokio::select!()`
- ALWAYS handle `JoinHandle` results properly (don't ignore panics)

## Function Design
- PREFER small, focused functions (max 50 lines)
- PREFER composition over inheritance
- USE builder pattern for complex construction
- USE `impl Trait` for return types when the concrete type is an implementation detail
- PREFER concrete return types when callers need to name the type or use it in bounds
- USE associated types over generic parameters when the relationship is 1:1 (not multiple implementations)

## Pattern Matching
- USE exhaustive matching when all variants need distinct handling
- USE catch-all `_` when only specific variants need special handling (more maintainable for evolving enums)
- USE `if let` for simple single-pattern matches
- USE `match` for complex logic or multiple patterns
- PREFER early returns with `?` over nested matches

## Type System Usage
- PREFER newtype patterns for domain modeling (e.g., `struct UserId(i64)`)
- USE `#[derive]` macros for common traits (Debug, Clone, PartialEq, etc.)
- PREFER `enum` over boolean flags for state (more expressive, harder to misuse)
- USE associated constants for type-level values; use `const fn` for computed constants

## Advanced Performance Optimization

### Memory Patterns
- AVOID unnecessary allocations in hot paths
- PREFER stack allocation over heap when possible
- USE `Box<T>` only when dynamic sizing required
- PREFER `Rc<T>` over `Arc<T>` for single-threaded contexts (note: async Tokio typically requires Arc)
- USE `std::sync::LazyLock` for lazy statics (Rust 1.80+, replaces lazy_static! crate)
- USE `std::sync::OnceLock` for one-time initialization with runtime values

### Concurrent Programming
- PREFER `Arc<RwLock<T>>` over `Arc<Mutex<T>>` for read-heavy workloads
- USE channels (`mpsc`, `crossbeam`) over shared mutable state
- PREFER atomic types (`AtomicU64`, etc.) for simple shared counters
- DOCUMENT every `Arc<T>` usage with justification for shared ownership
- AVOID `Arc<Mutex<T>>` for simple data - consider message passing

### Compilation Optimization
- AVOID premature `#[inline]` - LLVM handles inlining well
- USE `#[inline]` only for cross-crate generics or profiler-identified hot paths
- USE `#[cold]` for error handling paths to hint branch prediction
- PREFER `const fn` for compile-time evaluation when possible
- USE `#[repr(C)]` only when needed for FFI
- AVOID recursive types without `Box<T>` indirection

## Code Organization

### Module Structure
- PREFER flat module hierarchies over deep nesting
- GROUP related functionality in modules
- For library crates:
  - USE `pub(crate)` for internal APIs not exposed to consumers
  - PREFER re-exports at crate root for public APIs
- For binary crates (like this project):
  - USE explicit module paths for clarity (no external consumers)
  - `pub(crate)` documents intent but has no visibility effect

### Import Style (Enforced by clippy::absolute_paths)
- USE `use` imports at the top of the file for items used in the module
- AVOID inline qualified paths like `crate::models::User` or `std::collections::HashMap`
- Qualified paths are acceptable ONLY for:
  - Name collisions (two types with the same name from different modules)
  - Single-use items where the qualified path adds clarity
- This is enforced by `clippy::absolute_paths = "deny"` in Cargo.toml
- Example:
  ```rust
  // GOOD: Import at top of file
  use crate::models::User;
  use std::collections::HashMap;

  fn example() {
      let user = User::new();
      let map = HashMap::new();
  }

  // BAD: Inline qualified paths
  fn example() {
      let user = crate::models::User::new();
      let map = std::collections::HashMap::new();
  }
  ```

### Dependency Management
- PREFER minimal dependencies
- AVOID `unwrap()` on external library calls - handle errors properly
- USE specific feature flags to minimize dependencies
- PREFER `std` library over external crates when sufficient

### API Design
- PREFER `impl Trait` in argument position for flexibility; use concrete types in return position for clarity
- USE explicit lifetimes only when the compiler cannot infer them
- DESIGN APIs to be hard to misuse (parse, don't validate)
- PROVIDE builder patterns for structs with many optional fields

