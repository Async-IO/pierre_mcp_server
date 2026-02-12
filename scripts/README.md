# Scripts Directory

CI/Dev tools for validation, testing, and release of the Pierre MCP Server.

For runtime scripts (starting/stopping services), see [bin/README.md](../bin/README.md).

## Directory Structure

```
scripts/
├── ci/                              # CI pipelines & pre-push validation
│   ├── architectural-validation.sh  # Custom architectural pattern validation
│   ├── validation-patterns.toml     # Pattern definitions for arch validation
│   ├── parse-validation-patterns.py # TOML pattern parser
│   ├── lint-and-test.sh             # Full CI validation suite
│   ├── validate-no-secrets.sh       # Secret/credential detection
│   ├── ensure-mcp-compliance.sh     # MCP protocol compliance
│   ├── security-review.sh           # Authorization & tenant isolation checks
│   ├── check-input-validation.sh    # Division-by-zero, pagination, range checks
│   ├── pre-push-validate.sh         # Marker-based pre-push orchestrator
│   ├── pre-push-frontend-tests.sh   # Web frontend pre-push validation
│   └── pre-push-mobile-tests.sh     # Mobile pre-push validation
├── testing/                         # Manual/integration test scripts
│   ├── test-postgres.sh             # PostgreSQL integration (requires Docker)
│   ├── run-bridge-tests.sh          # SDK bridge test suite
│   └── clean-test-databases.sh      # Test database cleanup
├── release/                         # Release management
│   ├── prepare-release.sh           # Version bump, validation, commit + tag
│   └── validate-release.sh          # Pre-release consistency checks
├── setup/                           # Dev environment setup
│   ├── setup-claude-code-mcp.sh     # Claude Code session JWT setup
│   ├── check-gh-cli.sh             # GitHub CLI availability check
│   └── add-license-headers.sh       # SPDX license header management
├── sdk/                             # SDK tooling
│   ├── generate-sdk-types.js        # TypeScript type generation from server schemas
│   └── validate-sdk-schemas.sh      # SDK schema drift detection
├── profiling/                       # Performance profiling
│   ├── benchmark-compare.sh         # Criterion benchmark comparison
│   ├── flamegraph.sh                # CPU flamegraph generation
│   └── memory-profile.sh            # Memory profiling with DHAT/Valgrind
└── README.md                        # This file
```

## Usage by Category

### Validation (Run Before Commit)
```bash
cargo fmt                                        # Format code
./scripts/ci/architectural-validation.sh         # Architectural patterns
cargo clippy -p <changed-crate>                  # Linting
cargo test --test <test_file> <pattern>          # Targeted tests
```

### Testing Hierarchy

| Level | When | Command |
|-------|------|---------|
| **Targeted** | During development | `cargo test --test <test_file> <pattern>` |
| **Pre-push** | Before git push | `./scripts/ci/pre-push-validate.sh` |
| **Full CI** | Before merge | `./scripts/ci/lint-and-test.sh` |

### Frontend/Mobile Tests
```bash
./scripts/ci/pre-push-frontend-tests.sh   # ~5-10 seconds
./scripts/ci/pre-push-mobile-tests.sh     # ~5-10 seconds
```

### Specialized Testing
```bash
./scripts/testing/test-postgres.sh         # PostgreSQL integration (Docker)
./scripts/testing/run-bridge-tests.sh      # SDK/Bridge tests
./scripts/ci/ensure-mcp-compliance.sh      # MCP protocol compliance
```

### Cleanup
```bash
./scripts/testing/clean-test-databases.sh  # Remove test databases
```

### Release
```bash
./scripts/release/prepare-release.sh 0.3.0           # Full release
./scripts/release/prepare-release.sh 0.3.0 --dry-run  # Preview changes
```

## Script Dependencies

- **ci/architectural-validation.sh** depends on **ci/validation-patterns.toml** and **ci/parse-validation-patterns.py**
- **ci/architectural-validation.sh** calls **ci/security-review.sh** and **ci/check-input-validation.sh** when `--apply-skills` is passed
- **ci/lint-and-test.sh** orchestrates ci/ scripts plus **testing/run-bridge-tests.sh** and **testing/clean-test-databases.sh**
- **ci/pre-push-validate.sh** calls **ci/architectural-validation.sh**, **ci/pre-push-frontend-tests.sh**, and **ci/pre-push-mobile-tests.sh**
- **release/prepare-release.sh** calls **release/validate-release.sh**
