# Contributing to Pierre MCP Server

Thank you for your interest in contributing! This guide will get you from zero to your first contribution in **30 minutes**.

## New Contributor Quick Start

### Step 1: Get the Code (2 minutes)
```bash
# Fork on GitHub, then clone your fork
git clone https://github.com/YOUR_USERNAME/pierre_mcp_server.git
cd pierre_mcp_server
```

### Step 2: Environment Setup (5 minutes)
**Prerequisites**: Only Rust 1.75+ required
```bash
# Install Rust if needed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Build the project
cargo build --release
```

### Step 3: Verify Setup (3 minutes)
```bash
# Start the server
cargo run --bin pierre-mcp-server
# Wait for: "Server ready on ports 8080 (MCP) and 8081 (HTTP)"

# In another terminal, test it works
curl http://localhost:8081/api/health
# Should return: {"status":"healthy"}
```

### Step 4: Run Full Test Suite (10 minutes)
```bash
# Run all tests and linting (this is what CI runs)
./scripts/lint-and-test.sh
# Should end with: ✅ All checks passed!
```

### Step 5: Make Your First Change (10 minutes)
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
| User login | 8081 | `POST /api/auth/login` | None | Get JWT token |
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
- Add support for Garmin, Polar, Suunto, local files, etc.
- See `src/providers/strava.rs` as a reference implementation
- Provider needs: OAuth flow, activity fetching, data normalization

### Client Libraries
- Build SDKs for Go, JavaScript, Ruby, PHP, etc.
- Follow the existing Python examples in `examples/python/`
- Include authentication, A2A protocol, and MCP tools

### Testing & Quality
- Add test cases for new functionality
- Improve CI/CD pipeline and tooling
- Performance testing and optimization

## Development Setup

### Easy (30 minutes)
- **Fix documentation typos** - Look for typos in `README.md` or `docs/`
- **Add API examples** - Add curl examples to `docs/developer-guide/14-api-reference.md`
- **Improve error messages** - Make error messages more helpful in `src/errors.rs`

### Medium (2-4 hours)
- **Add new MCP tool** - Add fitness tool in `src/tools/` (see existing tools as examples)
- **Add test coverage** - Find untested code with `cargo tarpaulin`
- **Frontend improvements** - Add features to admin dashboard in `frontend/src/`

### Advanced (1+ days)
- **New fitness provider** - Add Garmin/Polar support in `src/providers/`
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
# See docs/developer-guide/15-getting-started.md for complete setup
```

### Frontend Development (Optional)
```bash
cd frontend
npm install
npm run dev    # Development server on :5173
npm test       # Component tests
```

## Code Standards

### Rust Backend (Enforced by CI)
```bash
# These must pass before your PR is merged
cargo fmt --check          # Code formatting
cargo clippy -- -D warnings # Linting
cargo test                  # All tests pass
./scripts/lint-and-test.sh  # Full validation
```

### Key Rules from [CLAUDE.md](CLAUDE.md)
- **No `unwrap()` or `panic!()`** - Use proper error handling with `Result<T, E>`
- **No placeholder code** - No TODOs, FIXMEs, or unimplemented features
- **Test everything** - New code needs comprehensive tests
- **Document public APIs** - Use `///` doc comments

### TypeScript Frontend
- **ESLint must pass**: `npm run lint`
- **Tests required**: `npm test`
- **Type safety**: No `any` types

## Development Workflow

### Before Starting Work
1. **Check existing issues** - Avoid duplicate work
2. **Discuss big changes** - Comment on issue or create discussion
3. **Update dependencies** - `cargo update && npm update`

### While Developing
```bash
# Continuous testing during development
cargo watch -x test           # Auto-run tests on changes
cargo watch -x clippy          # Auto-run linting
npm run dev                    # Frontend dev server with hot reload
```

### Before Submitting PR
```bash
# This is what CI will run - make sure it passes
./scripts/lint-and-test.sh

# Check your changes don't break anything
cargo build --release
cargo run --bin pierre-mcp-server  # Should start without errors
```

## Pull Request Process

### Before Submitting
1. **Discuss large changes** in GitHub Issues/Discussions first
2. **Update documentation** if you change APIs or add features
3. **Add tests** for new functionality  
4. **Run linting**: `./scripts/lint-and-test.sh` must pass
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
- [ ] `./scripts/lint-and-test.sh` passes

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

## Recognition

Contributors are recognized through:
- **GitHub Contributors Graph**: Automatic recognition
- **Release Notes**: Major contributions highlighted  
- **Documentation Credits**: Contributor sections in docs
- **Community Showcases**: Featured contributions in discussions

## Getting Help

### When You're Stuck
1. **Check existing docs** - [docs/developer-guide/15-getting-started.md](docs/developer-guide/15-getting-started.md)
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
./scripts/lint-and-test.sh  # Must pass
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