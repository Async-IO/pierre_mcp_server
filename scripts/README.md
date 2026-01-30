# Scripts Directory

CI/Dev tools for validation, testing, and release of the Pierre MCP Server.

For runtime scripts (starting/stopping services), see [bin/README.md](../bin/README.md).

## Script Inventory

| Script | Category | Purpose |
|--------|----------|---------|
| **setup-claude-code-mcp.sh** | Development | Sets up Claude Code session with valid JWT token and running server. |
| **architectural-validation.sh** | Validation | Custom architectural validation that Cargo/Clippy cannot check. Enforces project-specific patterns using `validation-patterns.toml`. |
| **clean-test-databases.sh** | Cleanup | Removes accumulated test database files from `test_data/` directory while preserving directory structure. |
| **deploy.sh** | Deployment | Production deployment script with Docker Compose management. Handles starting, stopping, and managing environments. |
| **ensure_mcp_compliance.sh** | Validation | MCP protocol compliance validation. Tests pierre-claude-bridge against Model Context Protocol specification. |
| **generate-sdk-types.js** | SDK | Auto-generates TypeScript type definitions from Pierre server tool schemas. Fetches MCP tool schemas and converts to TypeScript interfaces. |
| **lint-and-test.sh** | CI/CD | Full CI validation suite. Runs format, clippy, deny, architectural validation, all tests, frontend, SDK, and bridge tests. |
| **parse-validation-patterns.py** | Validation | Parses validation patterns from TOML configuration file. Outputs shell-compatible variables for use in validation scripts. |
| **pre-push-validate.sh** | Git Hooks | Marker-based pre-push validation. Runs tiered checks and creates validation marker for git push. |
| **pre-push-frontend-tests.sh** | Git Hooks | Pre-push validation for web frontend (frontend/). Runs TypeScript check, ESLint, and unit tests (~5-10 seconds). |
| **pre-push-mobile-tests.sh** | Git Hooks | Pre-push validation for mobile (frontend-mobile/). Runs TypeScript check, ESLint, and unit tests (~5-10 seconds). |
| **run_bridge_tests.sh** | Testing | Complete bridge test suite runner. Validates bridge functionality from CLI parsing to full MCP Client simulation. |
| **setup-git-hooks.sh** | Git Hooks | Installs git hooks for code quality enforcement. Sets up pre-commit, commit-msg, and pre-push hooks. |
| **test_trial_keys.sh** | Testing | Tests business API key provisioning system. Full workflow: creates admin, registers user, provisions API keys, tests MCP access. |
| **test-claude-desktop.sh** | Testing | Automated Claude Desktop testing setup. Prepares server, tokens, and config for testing OAuth features. |
| **test-jwt-auth.sh** | Testing | Verifies JWT authentication after Claude Code restart. Checks config file JWT matches server's expected key ID. |
| **test-postgres.sh** | Testing | PostgreSQL database plugin integration test runner. Starts PostgreSQL via Docker and runs database operation tests. |
| **validate-no-secrets.sh** | Security | CI validation script to detect secret patterns. Prevents PII leakage, credential exposure, and GDPR/CCPA violations. |
| **validate-sdk-schemas.sh** | Validation | Validates SDK TypeScript schemas match server tool definitions. |
| **validate-release.sh** | Validation | Pre-release validation script for version consistency and build checks. |
| **prepare-release.sh** | Deployment | Prepares release artifacts and changelog. |

## Configuration Files

| File | Purpose |
|------|---------|
| **validation-patterns.toml** | TOML configuration for architectural validation patterns. Defines critical, warning, and threshold patterns. |

## Usage by Category

### Development Setup
```bash
# Full dev environment (DB + seeds + all servers) - see bin/
../bin/setup-db-with-seeds-and-oauth-and-start-servers.sh

# Claude Code session JWT setup
./scripts/setup-claude-code-mcp.sh

# Install git hooks (run once)
./scripts/setup-git-hooks.sh
```

### Validation (Run Before Commit)
```bash
cargo fmt                              # Format code
./scripts/architectural-validation.sh  # Architectural patterns
cargo clippy --all-targets             # Linting
cargo test --test <test_file> <pattern> -- --nocapture  # Targeted tests
```

### Testing Hierarchy

| Level | When | Command |
|-------|------|---------|
| **Targeted** | During development | `cargo test --test <test_file> <pattern>` |
| **Pre-push** | Before git push | `./scripts/pre-push-validate.sh` |
| **Full CI** | Before PR/merge | `./scripts/lint-and-test.sh` |

```bash
# Targeted tests (fastest - only compile one test file)
cargo test --test intelligence_test test_training_load -- --nocapture
cargo test --test store_routes_test test_browse -- --nocapture

# Pre-push validation (creates marker, runs tiered checks)
./scripts/pre-push-validate.sh

# Full CI suite (runs everything)
./scripts/lint-and-test.sh
```

### Frontend/Mobile Tests
```bash
./scripts/pre-push-frontend-tests.sh   # ~5-10 seconds - Web frontend
./scripts/pre-push-mobile-tests.sh     # ~5-10 seconds - Mobile
```

### Specialized Testing
```bash
./scripts/test-postgres.sh             # PostgreSQL integration (requires Docker)
./scripts/run_bridge_tests.sh          # SDK/Bridge tests
./scripts/ensure_mcp_compliance.sh     # MCP protocol compliance
./scripts/test-claude-desktop.sh       # Claude Desktop integration
./scripts/test_trial_keys.sh           # API key provisioning workflow
```

### Cleanup
```bash
./scripts/clean-test-databases.sh      # Remove test databases only
```

### Deployment
```bash
./scripts/deploy.sh development        # Start dev environment (Docker)
./scripts/deploy.sh production         # Start production (Docker)
./scripts/deploy.sh stop               # Stop all services
```

## Script Dependencies

- **architectural-validation.sh** depends on **validation-patterns.toml** and **parse-validation-patterns.py**
- **lint-and-test.sh** orchestrates multiple validation and test scripts including **run_bridge_tests.sh**
- **pre-push-validate.sh** is used by git pre-push hook (installed via **setup-git-hooks.sh**)
- **pre-push-validate.sh** calls **pre-push-frontend-tests.sh** and **pre-push-mobile-tests.sh** when those directories have changes
