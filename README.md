# Pierre MCP Server

A comprehensive Rust-based MCP (Model Context Protocol) server for Strava fitness data analysis. Provides secure access to Strava's rich fitness data through Claude and other AI assistants.

## Features

- **Strava Integration**: Full OAuth2 authentication with PKCE security
- **Comprehensive Data Access**: Activities, athlete profiles, and aggregated statistics
- **Enhanced Security**: PKCE (Proof Key for Code Exchange) implementation
- **Type-safe Architecture**: Built with Rust for reliability and performance
- **Extensible Design**: Easy to add new fitness providers in the future
- **MCP Protocol Compliance**: Works seamlessly with Claude and GitHub Copilot
- **Comprehensive Testing**: Unit tests, integration tests, and end-to-end tests
- **Full Documentation**: Complete rustdoc documentation and examples

## Installation

```bash
cargo build --release
```

## OAuth2 Setup

### Strava

1. Create a Strava application at https://www.strava.com/settings/api
2. Note your Client ID and Client Secret
3. Run the auth setup tool:

```bash
cargo run --bin auth-setup -- strava \
  --client-id YOUR_CLIENT_ID \
  --client-secret YOUR_CLIENT_SECRET
```

4. Follow the browser prompts to authorize the application
5. The tool will save your tokens to the config file

## Configuration

The server supports multiple configuration methods:

### Using direnv (.envrc):
```bash
# Copy the example file
cp .envrc.example .envrc

# Edit with your credentials
vim .envrc

# Allow direnv to load the file
direnv allow
```

### Using .env file:
```env
STRAVA_CLIENT_ID=your_client_id
STRAVA_CLIENT_SECRET=your_client_secret
STRAVA_ACCESS_TOKEN=your_access_token
STRAVA_REFRESH_TOKEN=your_refresh_token
```

### Using config.toml:
```toml
[providers.strava]
auth_type = "oauth2"
client_id = "your_client_id"
client_secret = "your_client_secret"
access_token = "your_access_token"
refresh_token = "your_refresh_token"
```

## Usage

```bash
# Run with default settings
cargo run

# Run with custom port
cargo run -- --port 9000

# Run with custom config file
cargo run -- --config /path/to/config.toml
```

## MCP Tools

The server exposes the following tools:

- `get_activities`: Fetch fitness activities from a provider (supports pagination with limit/offset)
- `get_athlete`: Get athlete profile information
- `get_stats`: Get aggregated statistics (uses Strava's athlete stats API with fallback)

### Example Usage

```bash
# Test the server with example queries
cargo run --bin find-2025-longest-run
cargo run --bin find-2024-longest-run
```

## Adding to Claude or GitHub Copilot

Add to your MCP configuration:

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "path/to/pierre-mcp-server",
      "args": ["--port", "8080"]
    }
  }
}
```

Or for development:

```json
{
  "mcpServers": {
    "pierre-fitness-dev": {
      "command": "cargo",
      "args": ["run", "--", "--port", "8080"],
      "cwd": "/path/to/pierre_mcp_server"
    }
  }
}
```

## Development Status

### 🎯 Recent Testing (June 2025)
- ✅ Successfully connected to live MCP server
- ✅ Retrieved 500+ activities with pagination
- ✅ Found 68 runs in 2025, identified longest: 12.59km trail run
- ✅ All 42 tests passing with clean compilation
- ✅ Strava OAuth2 integration fully operational

### ✅ Completed
- [x] Core MCP server implementation with JSON-RPC over TCP
- [x] Strava provider with full OAuth2 authentication and PKCE security
- [x] Configuration management (file-based and environment variables)
- [x] Comprehensive data models (Activity, Athlete, Stats, PersonalRecord)
- [x] Unit tests for all core modules (21 tests)
- [x] Integration tests for MCP server and providers (16 tests)
- [x] End-to-end workflow tests (5 tests)
- [x] Example client implementations (find-2024-longest-run, find-2025-longest-run)
- [x] Comprehensive test coverage (42+ tests passing)
- [x] Clean compilation with no warnings
- [x] Dual MIT/Apache 2.0 licensing
- [x] Complete rustdoc documentation
- [x] OAuth2 setup tooling with web callback
- [x] PKCE implementation for enhanced OAuth2 security

### 📋 TODO
- [ ] **Additional Providers** (Next Priority)
  - [ ] Fitbit integration with OAuth2 and PKCE support
  - [ ] Polar Flow integration with OAuth2 and PKCE support
  - [ ] Wahoo integration
  - [ ] TrainingPeaks integration
  - [ ] Garmin Connect integration (requires enterprise API access)

**Note**: Garmin Connect and RunKeeper providers were removed due to API accessibility issues. The infrastructure remains ready for future providers.

- [ ] **Enhanced Features**
  - [ ] Real-time webhook support for activity updates
  - [ ] Activity streaming data (GPS tracks, heart rate zones)
  - [ ] Training plans and workout data
  - [ ] Social features (segments, kudos, comments)
  - [ ] Advanced analytics and insights

- [ ] **Performance & Reliability**
  - [ ] Connection pooling for HTTP clients
  - [ ] Rate limiting and retry logic
  - [ ] Caching layer for frequently accessed data
  - [ ] Metrics and monitoring integration
  - [ ] Graceful error recovery

- [ ] **Developer Experience**
  - [ ] Docker containerization
  - [ ] CI/CD pipeline setup
  - [ ] Performance benchmarks
  - [ ] API documentation with examples
  - [ ] Provider development guide

- [ ] **Security Enhancements**
  - [ ] Token encryption at rest
  - [ ] Secure token rotation
  - [ ] Audit logging
  - [ ] Rate limiting per client

## Contributing

We welcome contributions! Please see our [contribution guidelines](CONTRIBUTING.md) for details.

### Quick Start for Contributors

1. **Fork and clone the repository**
   ```bash
   git clone https://github.com/jfarcand/pierre_mcp_server.git
   cd pierre_mcp_server
   ```

2. **Set up development environment**
   ```bash
   # Install Rust (if not already installed)
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Install development dependencies
   cargo install cargo-watch
   ```

3. **Run tests**
   ```bash
   # Run all tests
   cargo test
   
   # Run tests with output
   cargo test -- --nocapture
   
   # Run specific test module
   cargo test config::tests
   ```

4. **Development workflow**
   ```bash
   # Auto-rebuild on changes
   cargo watch -x check -x test
   
   # Format code
   cargo fmt
   
   # Lint code
   cargo clippy
   ```

### Adding a New Provider

1. Create a new file in `src/providers/your_provider.rs`
2. Implement the `FitnessProvider` trait
3. Add OAuth2 or API key authentication
4. Update the provider factory in `src/providers/mod.rs`
5. Add comprehensive tests in `tests/provider_integration.rs`
6. Update configuration examples in README

### Code Style

- Follow Rust standard formatting (`cargo fmt`)
- Use clippy for linting (`cargo clippy`)
- Write comprehensive tests for new features
- Document public APIs with rustdoc comments
- Follow the existing error handling patterns

### Commit Guidelines

- Use conventional commit format: `feat:`, `fix:`, `docs:`, etc.
- Write clear, descriptive commit messages
- Keep commits focused and atomic
- Reference issues in commit messages when applicable

## License

This project is dual-licensed under either of:

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
* MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.