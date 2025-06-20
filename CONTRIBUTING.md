# Contributing to Pierre Fitness API

Thank you for your interest in contributing to Pierre! This project is built by the community, for the community. We welcome contributions from developers of all skill levels.

## 🚀 Quick Start

1. **Fork** the repository on GitHub
2. **Clone** your fork locally: `git clone https://github.com/YOUR_USERNAME/pierre_mcp_server.git`
3. **Create a branch**: `git checkout -b feature/your-feature-name`
4. **Make your changes** and test them
5. **Submit a pull request** with a clear description

## 🎯 Ways to Contribute

### 🐛 Bug Reports
- Use GitHub Issues with the "bug" label
- Include steps to reproduce, expected vs actual behavior
- Add system info (OS, Rust version, etc.) if relevant

### 💡 Feature Requests  
- Use GitHub Issues with the "enhancement" label
- Describe the use case and expected behavior
- Consider if it fits Pierre's scope (fitness data + AI protocols)

### 📚 Documentation
- Fix typos, improve clarity, add missing examples
- All `.md` files can be edited directly on GitHub
- Documentation is as important as code!

### 🔌 New Fitness Providers
- Add support for Garmin, Polar, Suunto, local files, etc.
- See `src/providers/strava.rs` as a reference implementation
- Provider needs: OAuth flow, activity fetching, data normalization

### 🐍 Client Libraries
- Build SDKs for Go, JavaScript, Ruby, PHP, etc.
- Follow the existing Python examples in `examples/python/`
- Include authentication, A2A protocol, and MCP tools

### 🧪 Testing & Quality
- Add test cases for new functionality
- Improve CI/CD pipeline and tooling
- Performance testing and optimization

## 🏗️ Development Setup

### Prerequisites
- **Rust** 1.70+ (install via [rustup](https://rustup.rs/))
- **Node.js** 18+ for frontend development
- **Python** 3.8+ for examples and tooling

### Local Development
```bash
# Clone and setup
git clone https://github.com/YOUR_USERNAME/pierre_mcp_server.git
cd pierre_mcp_server

# Backend development
cargo build          # Build the project
cargo test           # Run tests
cargo run --bin pierre-mcp-server -- --single-tenant --port 8080

# Frontend development (optional)
cd frontend
npm install
npm run dev          # Development server
npm test             # Run tests

# Verify everything works
./scripts/lint-and-test.sh
```

## 📋 Code Guidelines

### Rust Backend
- Follow Rust naming conventions and idioms
- Use `cargo fmt` for formatting
- Pass `cargo clippy` without warnings
- Add tests for new functionality
- Document public APIs with doc comments

### TypeScript Frontend
- Use ESLint configuration provided
- Follow React best practices
- Add tests for new components
- Maintain type safety

### Python Examples
- Follow PEP 8 style guidelines
- Include type hints where helpful  
- Add docstrings for classes and functions
- Test examples work in isolation

## 🔄 Pull Request Process

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

## 🏗️ Architecture Overview

Pierre is designed to be modular and extensible:

- **Core Server** (`src/`): MCP protocol, A2A protocol, authentication
- **Providers** (`src/providers/`): Pluggable fitness data sources
- **Intelligence** (`src/intelligence/`): AI-powered analysis and insights
- **Frontend** (`frontend/`): Admin dashboard and monitoring
- **Examples** (`examples/`): Client libraries and integration demos

### Key Design Principles
- **Protocol First**: MCP and A2A protocols are core abstractions
- **Provider Agnostic**: Easy to add new fitness data sources
- **Multi-Tenant**: Single deployment can serve multiple clients
- **AI Ready**: Built for LLM and AI agent integration

## 🌟 Recognition

Contributors are recognized through:
- **GitHub Contributors Graph**: Automatic recognition
- **Release Notes**: Major contributions highlighted  
- **Documentation Credits**: Contributor sections in docs
- **Community Showcases**: Featured contributions in discussions

## 📞 Getting Help

- 📖 **Documentation**: Start with [README.md](README.md) and [docs/SETUP.md](docs/SETUP.md)
- 💬 **GitHub Discussions**: Questions, ideas, and general discussion
- 🐛 **GitHub Issues**: Bug reports and feature requests
- 📧 **Maintainers**: Tag `@maintainers` for architectural questions

## 🎯 Good First Issues

Looking for where to start? Check issues labeled:
- `good first issue`: Perfect for newcomers
- `help wanted`: Community input desired
- `documentation`: Improve guides and examples
- `enhancement`: New features and improvements

## 📜 Code of Conduct

We are committed to providing a welcoming and inclusive experience for everyone. Please:

- **Be respectful** in all interactions
- **Be constructive** in feedback and discussions  
- **Be patient** with newcomers and questions
- **Focus on what is best** for the community and project

Unacceptable behavior will not be tolerated. Contact maintainers if you experience or witness any issues.

## 📄 License

By contributing to Pierre, you agree that your contributions will be licensed under the same dual license as the project (MIT OR Apache-2.0).

---

**Thank you for contributing to Pierre! Together we're building the future of intelligent fitness applications.** 🚀