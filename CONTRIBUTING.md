# Contributing to Pierre Fitness Platform

Thank you for your interest in contributing! This guide covers everything you need to make your first contribution.

## New Contributor Quick Start

**Prerequisites**: Complete the [Getting Started Guide](book/src/getting-started.md) first to set up your development environment.

### Step 1: Fork and Setup
```bash
# Fork on GitHub, then clone your fork
git clone https://github.com/YOUR_USERNAME/pierre_mcp_server.git
cd pierre_mcp_server

# Use the automated development setup from getting-started guide
./scripts/fresh-start.sh
source .envrc && cargo run --bin pierre-mcp-server &
./scripts/complete-user-workflow.sh
```

### Step 2: Validate Your Environment
```bash
# Run all tests and linting (this is what CI runs)
./scripts/ci/lint-and-test.sh
# Should end with: ✅ All checks passed!

# Test server is working
curl http://localhost:8081/api/health
# Should return: {"status":"healthy"}
```

### Step 3: Make Your First Change
```bash
# Create a branch
git checkout -b your-feature-name

# Make a small change (try adding a comment or fixing a typo)
# Then test it still works
cargo test

# Commit and push
git add .
git commit -m "Your change description"
git push origin your-feature-name
```

Ready to contribute - create a pull request from your branch.

## API Reference for Contributors

| Purpose | Port | Endpoint | Auth Needed | Use Case |
|---------|------|----------|-------------|----------|
| Health check | 8081 | `GET /api/health` | None | Verify server running |
| User registration | 8081 | `POST /api/auth/register` | None | New user signup |
| User login | 8081 | `POST /oauth/token` | None | Get JWT token (OAuth2 ROPC) |
| Admin actions | 8081 | `POST /admin/*` | Admin JWT | User approval, etc. |
| A2A protocol | 8081 | `POST /a2a/*` | Client credentials | Agent-to-agent |
| MCP protocol | 8080 | All MCP calls | User JWT | Claude Desktop, AI tools |

## Good First Contributions

## Ways to Contribute

### Bug Reports
- Use GitHub Issues with the "bug" label
- Include steps to reproduce, expected vs actual behavior
- Add system info (OS, Rust version, etc.) if relevant

### Feature Requests  
- Use GitHub Issues with the "enhancement" label
- Describe the use case and expected behavior
- Consider if it fits Pierre's scope (fitness data + AI protocols)

### Documentation
- Fix typos, improve clarity, add missing examples
- All `.md` files can be edited directly on GitHub
- Documentation is as important as code!

### New Fitness Providers
- Add support for Suunto, local files, etc.
- See `src/providers/strava.rs` as a reference implementation
- Provider needs: OAuth flow, activity fetching, data normalization
- Supported: Strava, Garmin, Fitbit, WHOOP, COROS, Terra (aggregator for 150+ devices)

### Client Libraries
- Build SDKs for Go, JavaScript, Ruby, PHP, etc.
- Follow the existing Python examples in `examples/python/`
- Include authentication, A2A protocol, and MCP tools

### Testing & Quality
- Add test cases for new functionality
- Improve CI/CD pipeline and tooling
- Performance testing and optimization

## Development Setup

### Easy
- **Fix documentation typos** - Look for typos in `README.md` or `book/src/`
- **Add API examples** - Add curl examples to `book/src/developer-guide/14-api-reference.md`
- **Improve error messages** - Make error messages more helpful in `src/errors.rs`

### Medium
- **Add new MCP tool** - Add fitness tool in `src/tools/` (see existing tools as examples)
- **Add test coverage** - Find untested code with `cargo tarpaulin`
- **Frontend improvements** - Add features to admin dashboard in `frontend/src/`

### Advanced
- **New fitness provider** - Add new provider support in `src/providers/`
- **Performance optimization** - Profile and optimize database queries
- **Security improvements** - Enhance authentication or encryption

## Development Environment

### Minimal Setup (Most Contributors)
**Prerequisites**: Only Rust 1.75+
```bash
# Everything you need
cargo build
cargo run --bin pierre-mcp-server
# Database auto-created, no external dependencies
```

### Full Development Setup (Advanced)
**Additional**: PostgreSQL, Redis, Strava API credentials
```bash
# See book/src/getting-started.md for complete setup
```

### Frontend Development (Optional)
```bash
cd frontend
bun install
bun run dev    # Development server on :5173
bun run test   # Component tests
```

## Code Standards

### Rust Backend (Enforced by CI)
```bash
# These must pass before your PR is merged
cargo fmt --check          # Code formatting
cargo clippy -- -D warnings # Linting
cargo test                  # All tests pass
./scripts/ci/lint-and-test.sh  # Full validation
```

### Key Rules from [CLAUDE.md](CLAUDE.md)
- **No `unwrap()` or `panic!()`** - Use proper error handling with `Result<T, E>`
- **No placeholder code** - No TODOs, FIXMEs, or unimplemented features
- **Test everything** - New code needs comprehensive tests
- **Document public APIs** - Use `///` doc comments

### TypeScript Frontend
- **ESLint must pass**: `bun run lint`
- **Tests required**: `bun run test`
- **Type safety**: No `any` types

## Development Workflow

### Automated Git Hooks (Zero Manual Commands Required!)

Pierre uses automated git hooks to ensure code quality. **You don't need to run any commands manually!**

#### Initial Setup (One Time)
```bash
# Install git hooks (run once after cloning)
git config core.hooksPath .githooks
```

#### Your Daily Workflow
```bash
# 1. Write code as normal

# 2. Commit (hook runs automatically: 2-3 min)
git commit -m "feat: add new feature"
# ✅ Auto-runs: format check, clippy, unit tests

# 3. Push (hook runs automatically: 5-10 min)
git push origin your-branch
# ✅ Auto-runs: 20 critical path tests

# 4. CI validates everything (30-60 min, in background)
# ✅ Full test suite, security checks, cross-platform
```

**Total time YOU wait: 7-13 minutes** (vs 30-60 for full suite!)

#### What Hooks Do

**Pre-Commit Hook** (2-3 minutes):
- Format check (`cargo fmt --check`)
- Clippy on lib + bins
- Unit tests
- Blocks commit if fails

**Commit-Msg Hook** (instant):
- Enforces 1-2 line commit messages (no novels!)
- Blocks AI-generated commit signatures
- Validates first line length (max 100 chars)
- Encourages conventional commit format
- Blocks commit if message invalid

**Pre-Push Hook** (5-10 minutes):
- 20 critical path tests:
  - Infrastructure (health, database, encryption)
  - Security (auth, API keys, JWT, OAuth2)
  - MCP protocol compliance
  - Error handling (validates AppResult refactoring)
  - Multi-tenancy isolation
  - A2A protocol & algorithms
- Blocks push if fails (catches 80% of issues before CI!)

#### Skipping Hooks (When Needed)
```bash
# Skip pre-commit (use sparingly!)
git commit --no-verify -m "WIP: quick iteration"

# Skip pre-push (use sparingly!)
git push --no-verify
```

⚠️ **Warning:** CI will still run and catch issues, but you'll wait longer for feedback.

#### Manual Testing (Optional)
If you want to run tests manually:
```bash
# Targeted tests during development (fastest)
cargo test --test <test_file> <pattern> -- --nocapture

# Pre-push validation (tiered checks, creates marker)
./scripts/ci/pre-push-validate.sh

# Full test suite (comprehensive CI suite)
./scripts/ci/lint-and-test.sh
```

See [Testing Guide](book/src/testing.md) for complete testing documentation.

### Before Starting Work
1. **Check existing issues** - Avoid duplicate work
2. **Discuss big changes** - Comment on issue or create discussion
3. **Update dependencies** - `cargo update && npm update`
4. **Install git hooks** - `git config core.hooksPath .githooks` (one time)

### While Developing
```bash
# Continuous testing during development (optional)
cargo watch -x test           # Auto-run tests on changes
cargo watch -x clippy          # Auto-run linting
bun run dev                    # Frontend dev server with hot reload
```

### Before Submitting PR
```bash
# Git hooks handle this automatically!
# Just commit and push - hooks will validate

# Optional: Run full validation manually
./scripts/ci/lint-and-test.sh
```

## Pull Request Process

### Before Submitting
1. **Discuss large changes** in GitHub Issues/Discussions first
2. **Update documentation** if you change APIs or add features
3. **Add tests** for new functionality  
4. **Run linting**: `./scripts/ci/lint-and-test.sh` must pass
5. **Test manually** that your changes work as expected

### PR Description Template
```markdown
## Summary
Brief description of what this PR does.

## Type of Change
- [ ] Bug fix
- [ ] New feature  
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Refactoring

## Testing
- [ ] Added/updated tests
- [ ] Manual testing completed
- [ ] `./scripts/ci/lint-and-test.sh` passes

## Related Issues
Fixes #123, relates to #456
```

### Review Process
- Maintainers will review within a few days
- Address feedback promptly
- Keep PRs focused on a single change
- We may suggest architectural improvements

## Architecture Overview

Pierre is designed to be modular and extensible:

- **Core Server** (`src/`): MCP protocol, A2A protocol, authentication
- **Providers** (`src/providers/`): Pluggable fitness data sources
- **Intelligence** (`src/intelligence/`): Analysis and insights
- **Frontend** (`frontend/`): Admin dashboard and monitoring
- **Examples** (`examples/`): Client libraries and integration demos

### Key Design Principles
- **Protocol First**: MCP and A2A protocols are core abstractions
- **Provider Agnostic**: Easy to add new fitness data sources
- **Multi-Tenant**: Single deployment can serve multiple clients
- **AI Ready**: Built for LLM and AI agent integration

## Adding New Features

### New Fitness Provider

1. Implement `FitnessProvider` trait in `src/providers/`:
```rust
pub struct NewProvider {
    config: ProviderConfig,
    credentials: Option<OAuth2Credentials>,
}

#[async_trait]
impl FitnessProvider for NewProvider {
    fn name(&self) -> &'static str { "new_provider" }
    // ... implement other methods
}
```

2. Register in `src/providers/registry.rs`
3. Add OAuth configuration in `src/oauth/`
4. Add tests

### New MCP Tool

1. Define tool in `src/protocols/universal/tool_registry.rs`
2. Implement handler in `src/protocols/universal/handlers/`
3. Register in tool executor
4. Add unit + integration tests
5. Regenerate SDK types:
```bash
cd sdk && bun run generate-types
```

### New Database Backend

1. Implement repository traits in `src/database_plugins/`
2. Add to factory in `src/database_plugins/factory.rs`
3. Add migration support
4. Add comprehensive tests

See `src/database/repositories/mod.rs` for the 13 repository traits to implement.

## Recognition

Contributors are recognized through:
- **GitHub Contributors Graph**: Automatic recognition
- **Release Notes**: Major contributions highlighted  
- **Documentation Credits**: Contributor sections in docs
- **Community Showcases**: Featured contributions in discussions

## Getting Help

### When You're Stuck
1. **Check existing docs** - [book/src/getting-started.md](book/src/getting-started.md)
2. **Search closed issues** - Someone may have had the same problem
3. **Enable debug logging** - `RUST_LOG=debug cargo run --bin pierre-mcp-server`
4. **Ask in GitHub Discussions** - We're friendly and responsive!

### Common Issues & Solutions

**"cargo build fails"**
```bash
rustup update              # Update Rust
cargo clean && cargo build # Clean rebuild
```

**"Server won't start"**
```bash
./scripts/fresh-start.sh   # Clean database restart
lsof -i :8080 -i :8081     # Check if ports are in use
```

**"Tests failing"**
```bash
export RUST_LOG=debug      # More verbose test output
cargo test -- --nocapture  # See test output
```

## Good First Issues

Looking for where to start? Check issues labeled:
- `good first issue`: Perfect for newcomers
- `help wanted`: Community input desired
- `documentation`: Improve guides and examples
- `enhancement`: New features and improvements

## Code of Conduct

We are committed to providing a welcoming and inclusive experience for everyone. Please:

- **Be respectful** in all interactions
- **Be constructive** in feedback and discussions  
- **Be patient** with newcomers and questions
- **Focus on what is best** for the community and project

Unacceptable behavior will not be tolerated. Contact maintainers if you experience or witness any issues.

## License

By contributing to Pierre, you agree that your contributions will be licensed under the same dual license as the project (MIT OR Apache-2.0).

---

---

## TL;DR for Experienced Developers

```bash
# Clone, build, test, contribute
git clone YOUR_FORK
cd pierre_mcp_server
cargo build --release
./scripts/ci/lint-and-test.sh  # Must pass
# Make changes, test, submit PR
```

**Key files to know:**
- `src/main.rs` - Server entry point (doesn't exist, uses bin/)
- `src/bin/pierre-mcp-server.rs` - Main server binary  
- `src/routes.rs` - HTTP API routes
- `src/mcp/multitenant.rs` - MCP protocol implementation
- `src/providers/strava.rs` - Example fitness provider
- `CLAUDE.md` - Development standards (must read)

Thank you for contributing.