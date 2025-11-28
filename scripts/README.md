# Scripts Directory

This directory contains shell scripts and utilities for development, testing, deployment, and validation of the Pierre MCP Server.

## Script Inventory

| Script | Category | Purpose |
|--------|----------|---------|
| **architectural-validation.sh** | Validation | Custom architectural validation that Cargo/Clippy cannot check. Enforces project-specific patterns using `validation-patterns.toml`. |
| **category-test-runner.sh** | Testing | Runs tests by category (mcp, admin, oauth, security, etc.) to prevent OOM and enable targeted testing. |
| **clean-test-databases.sh** | Cleanup | Removes accumulated test database files from `test_data/` directory while preserving directory structure. |
| **complete-user-workflow.sh** | Testing | Complete user registration and approval workflow test. Implements all 5 steps from HOW_TO_REGISTER_A_USER.md. |
| **deploy.sh** | Deployment | Production deployment script with Docker Compose management. Handles starting, stopping, and managing environments. |
| **dev-start.sh** | Development | Development server startup script. Builds project, creates admin and regular users, starts backend and frontend servers. |
| **ensure_mcp_compliance.sh** | Validation | MCP protocol compliance validation. Tests pierre-claude-bridge against Model Context Protocol specification. |
| **fast-tests.sh** | Testing | Fast test runner (< 5 minutes). Runs unit and quick component tests only, excludes slow E2E and comprehensive tests. |
| **fresh-start.sh** | Cleanup | Fresh start script for database cleanup. Removes all database files and Docker volumes for a clean state. |
| **generate-sdk-types.js** | SDK | Auto-generates TypeScript type definitions from Pierre server tool schemas. Fetches MCP tool schemas and converts to TypeScript interfaces. |
| **lint-and-test.sh** | CI/CD | Simplified validation orchestrator. Delegates to cargo fmt, cargo clippy, cargo deny, and custom architectural validation. Main CI script. |
| **parse-validation-patterns.py** | Validation | Parses validation patterns from TOML configuration file. Outputs shell-compatible variables for use in validation scripts. |
| **pre-push-tests.sh** | Git Hooks | Pre-push validation - Critical path tests (5-10 minutes). Runs essential tests to catch 80% of issues before pushing. |
| **run_bridge_tests.sh** | Testing | Complete bridge test suite runner. Validates bridge functionality from CLI parsing to full MCP Client simulation. |
| **safe-test-runner.sh** | Testing | Safe incremental test runner. Runs tests in small batches with memory cleanup pauses to prevent OOM. |
| **seed-demo-data.sh** | Development | Seeds the SQLite database with demo data for dashboard visualization. Creates users, API keys, A2A clients, usage records, and admin tokens. |
| **setup-git-hooks.sh** | Git Hooks | Installs git hooks for code quality enforcement. Sets up pre-commit, commit-msg, and pre-push hooks. |
| **smoke-test.sh** | Testing | Quick validation script for rapid development feedback (2-3 minutes). Format check, clippy, unit tests, one integration test. |
| **test_trial_keys.sh** | Testing | Tests business API key provisioning system. Full workflow: creates admin, registers user, provisions API keys, tests MCP access. |
| **test-claude-desktop.sh** | Testing | Automated Claude Desktop testing setup. Prepares server, tokens, and config for testing OAuth features. |
| **test-jwt-auth.sh** | Testing | Verifies JWT authentication after Claude Code restart. Checks config file JWT matches server's expected key ID. |
| **test-postgres.sh** | Testing | PostgreSQL database plugin integration test runner. Starts PostgreSQL via Docker and runs database operation tests. |
| **validate-no-secrets.sh** | Security | CI validation script to detect secret patterns. Prevents PII leakage, credential exposure, and GDPR/CCPA violations. |

## Configuration Files

| File | Purpose |
|------|---------|
| **validation-patterns.toml** | TOML configuration for architectural validation patterns. Defines critical, warning, and threshold patterns. |

## Usage by Category

### Essential Development Scripts
```bash
./scripts/dev-start.sh              # Start development environment
./scripts/fresh-start.sh            # Clean reset of database
./scripts/seed-demo-data.sh         # Populate database with demo data
./scripts/setup-git-hooks.sh        # Install git hooks (run once)
```

### Validation (Run Before Commit)
```bash
cargo fmt                           # Format code
./scripts/architectural-validation.sh  # Architectural patterns
cargo clippy --tests                # Linting
cargo test                          # Tests
```

### Testing Hierarchy (Fastest to Slowest)
```bash
./scripts/smoke-test.sh             # ~2-3 minutes - Quick feedback
./scripts/fast-tests.sh             # ~5 minutes - Unit + fast tests
./scripts/pre-push-tests.sh         # ~5-10 minutes - Critical path
./scripts/safe-test-runner.sh       # ~20-30 minutes - All tests (batched)
./scripts/lint-and-test.sh          # Full CI suite
```

### Specialized Testing
```bash
./scripts/category-test-runner.sh mcp       # MCP-specific tests
./scripts/category-test-runner.sh oauth     # OAuth tests
./scripts/test-postgres.sh                  # PostgreSQL integration
./scripts/run_bridge_tests.sh               # SDK/Bridge tests
./scripts/ensure_mcp_compliance.sh          # MCP protocol compliance
```

### Workflow Testing
```bash
./scripts/complete-user-workflow.sh         # Full user registration flow
./scripts/test-claude-desktop.sh            # Claude Desktop integration
```

### Cleanup
```bash
./scripts/fresh-start.sh            # Full database reset
./scripts/clean-test-databases.sh   # Remove test databases only
```

### Deployment
```bash
./scripts/deploy.sh development     # Start dev environment (Docker)
./scripts/deploy.sh production      # Start production (Docker)
./scripts/deploy.sh stop            # Stop all services
```

## Script Dependencies

- **architectural-validation.sh** depends on **validation-patterns.toml** and **parse-validation-patterns.py**
- **lint-and-test.sh** orchestrates multiple validation and test scripts
- **pre-push-tests.sh** is used by git pre-push hook (installed via **setup-git-hooks.sh**)
- **test-claude-desktop.sh** calls **fresh-start.sh** and **complete-user-workflow.sh**
