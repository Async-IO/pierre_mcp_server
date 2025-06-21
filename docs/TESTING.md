# Testing and Development Guide

This document covers testing infrastructure, development workflows, and quality assurance for Pierre MCP Server.

## Testing Infrastructure

### Test Suites

Pierre includes comprehensive test coverage across multiple areas:

| Test Suite | Purpose | Location | Command |
|------------|---------|----------|---------|
| **Unit Tests** | Individual function testing | `src/` (inline) | `cargo test` |
| **Integration Tests** | End-to-end functionality | `tests/` | `cargo test --test` |
| **MCP Compliance** | Protocol compliance | `tests/mcp_compliance_test.rs` | `cargo test --test mcp_compliance_test` |
| **A2A Protocol** | Agent-to-agent protocol | `tests/a2a_compliance_test.rs` | `cargo test --test a2a_compliance_test` |
| **API Tests** | REST API validation | `tests/api_tests.rs` | `cargo test --test api_tests` |

### Automated Test Script

The project includes a comprehensive test script:

```bash
./scripts/lint-and-test.sh
```

**What it runs:**
1. **Code formatting**: `cargo fmt --all --check`
2. **Linting**: `cargo clippy --all-targets --all-features`
3. **Unit tests**: `cargo test --lib --quiet`
4. **Integration tests**: `cargo test --test api_tests --quiet`
5. **MCP compliance**: `cargo test --test mcp_compliance_test --quiet`
6. **A2A compliance**: `cargo test --test a2a_compliance_test --quiet`
7. **Python MCP demo**: Validates MCP client integration
8. **A2A demo**: Tests agent-to-agent protocol

### Running Tests

#### All Tests
```bash
# Run complete test suite
./scripts/lint-and-test.sh

# Run all tests manually
cargo test --all
```

#### Specific Test Suites
```bash
# Unit tests only
cargo test --lib

# Integration tests
cargo test --test api_tests
cargo test --test mcp_compliance_test
cargo test --test a2a_compliance_test

# Specific test function
cargo test test_jwt_token_validation
cargo test test_mcp_initialize_request
```

#### Test with Coverage
```bash
# Install coverage tool
cargo install cargo-tarpaulin

# Run tests with coverage
cargo tarpaulin --out Html --output-dir coverage/
```

### Test Configuration

#### Test Environment Variables
```bash
# Test database (in-memory SQLite)
TEST_DATABASE_URL=sqlite::memory:

# Mock providers for testing
MOCK_PROVIDERS=true

# Skip email verification in tests
SKIP_EMAIL_VERIFICATION=true

# Auto-approve API keys for testing
AUTO_APPROVE_KEYS=true

# Test user credentials
TEST_USER_EMAIL=test@example.com
TEST_USER_PASSWORD=test_password_123
```

#### Test Configuration File
```toml
# test_config.toml
[server]
database_url = "sqlite::memory:"
single_tenant = true

[development]
mock_providers = true
auto_approve_keys = true
skip_email_verification = true

[logging]
level = "debug"
format = "text"
```

## Development Workflow

### Setup Development Environment

1. **Clone and Build**
   ```bash
   git clone https://github.com/jfarcand/pierre_mcp_server.git
   cd pierre_mcp_server
   cargo build
   ```

2. **Install Development Tools**
   ```bash
   # Code formatting
   rustup component add rustfmt
   
   # Linting
   rustup component add clippy
   
   # Coverage
   cargo install cargo-tarpaulin
   
   # Watch for changes
   cargo install cargo-watch
   ```

3. **Setup Environment**
   ```bash
   # Copy example environment
   cp .env.example .envrc
   
   # Edit with your credentials
   vim .envrc
   
   # Load environment (if using direnv)
   direnv allow
   ```

### Development Commands

#### Code Quality
```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Lint code
cargo clippy

# Fix linting issues
cargo clippy --fix
```

#### Development Server
```bash
# Run in development mode with auto-reload
cargo watch -x 'run --bin pierre-mcp-server -- --single-tenant --log-level debug'

# Run specific binary
cargo run --bin auth-setup -- --help
cargo run --bin test-weather-integration
```

#### Database Management
```bash
# Reset test database
rm -f test.db

# Check database schema
sqlite3 ./data/users.db ".schema"

# View database contents
sqlite3 ./data/users.db "SELECT * FROM users;"
```

### Debugging

#### Enable Debug Logging
```bash
# Detailed logging
RUST_LOG=debug cargo run --bin pierre-mcp-server

# Module-specific logging
RUST_LOG=pierre_mcp_server::auth=debug cargo run

# Trace level for maximum detail
RUST_LOG=trace cargo run --bin pierre-mcp-server
```

#### Common Debugging Commands
```bash
# Test OAuth flow
cargo run --bin test-oauth-callback

# Validate weather integration
cargo run --bin diagnose-weather-api

# Test with real data
cargo run --bin test-with-data

# Check activity analysis
cargo run --bin test-intelligence-for-longest-run
```

## Protocol Testing

### MCP Protocol Compliance

The MCP compliance test validates:

```rust
#[tokio::test]
async fn test_mcp_protocol_version() {
    // Tests protocol version 2025-06-18
}

#[tokio::test]
async fn test_initialize_request_response() {
    // Tests initialize flow
}

#[tokio::test]
async fn test_tools_list_response() {
    // Tests tools/list method
}

#[tokio::test]
async fn test_tool_call_execution() {
    // Tests tools/call method
}
```

Run MCP compliance tests:
```bash
cargo test --test mcp_compliance_test
```

### A2A Protocol Compliance

The A2A compliance test validates:

```rust
#[tokio::test]
async fn test_a2a_client_registration() {
    // Tests client registration flow
}

#[tokio::test]
async fn test_a2a_tool_execution() {
    // Tests tool execution via A2A
}

#[tokio::test]
async fn test_a2a_session_management() {
    // Tests session lifecycle
}
```

Run A2A compliance tests:
```bash
cargo test --test a2a_compliance_test
```

### Manual Protocol Testing

#### MCP Client Testing
```bash
# Python MCP client demo
cd examples/python/mcp
python investor_demo.py
```

#### A2A Client Testing
```bash
# Direct A2A API calls
curl -X POST http://localhost:8081/a2a/clients \
  -H "Authorization: Bearer JWT_TOKEN" \
  -d '{"name": "TestClient", "description": "Test A2A client"}'
```

## Integration Testing

### API Integration Tests

Located in `tests/api_tests.rs`, covering:

- User registration and authentication
- API key creation and management
- OAuth flow validation
- Rate limiting enforcement
- Admin API functionality

Run API tests:
```bash
cargo test --test api_tests
```

### Real Data Integration Tests

Test with real fitness provider data:

```bash
# Test with real Strava data (requires OAuth setup)
cargo run --bin test-with-data

# Find real activities for testing
cargo run --bin find-2024-longest-run
cargo run --bin find-consecutive-10k-runs
```

### Weather Integration Tests

```bash
# Test weather API integration
cargo run --bin test-weather-integration

# Test with real OpenWeatherMap API
cargo run --bin test-real-weather

# Diagnose weather API issues
cargo run --bin diagnose-weather-api
```

## Performance Testing

### Load Testing

Basic load testing setup:

```bash
# Install load testing tools
cargo install drill  # Or use wrk, hey, etc.

# Create drill.yml
cat > drill.yml << EOF
base: 'http://localhost:8081'
iterations: 1000
rampup: 60

plan:
  - name: Health check
    request:
      url: /health
  
  - name: API key validation
    request:
      url: /api/keys
      headers:
        Authorization: 'Bearer YOUR_API_KEY'
EOF

# Run load test
drill --benchmark drill.yml
```

### Performance Benchmarks

```bash
# Benchmark critical paths
cargo bench

# Profile with flamegraph
cargo install flamegraph
sudo cargo flamegraph --bin pierre-mcp-server
```

## Continuous Integration

### GitHub Actions

The project includes CI/CD workflows:

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run tests
        run: ./scripts/lint-and-test.sh
```

### Local CI Simulation

```bash
# Run the same checks as CI
./scripts/lint-and-test.sh

# Check formatting (CI requirement)
cargo fmt --all -- --check

# Check clippy warnings (CI requirement)
cargo clippy --all-targets --all-features -- -D warnings
```

## Quality Assurance

### Code Quality Metrics

- **Test Coverage**: Aim for >80% code coverage
- **Clippy Warnings**: Zero warnings in CI
- **Documentation**: All public APIs documented
- **Performance**: No regressions in benchmarks

### Pre-commit Hooks

Setup pre-commit hooks:

```bash
# Install pre-commit
pip install pre-commit

# Setup hooks (if .pre-commit-config.yaml exists)
pre-commit install

# Manual checks before commit
cargo fmt
cargo clippy
./scripts/lint-and-test.sh
```

### Code Review Checklist

- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Code is formatted
- [ ] Documentation updated
- [ ] Breaking changes noted
- [ ] Performance impact considered
- [ ] Security implications reviewed

## Troubleshooting Tests

### Common Test Issues

**Database Connection Errors:**
```bash
# Reset test database
rm -f test.db

# Use in-memory database for tests
TEST_DATABASE_URL=sqlite::memory: cargo test
```

**OAuth Test Failures:**
```bash
# Use mock providers
MOCK_PROVIDERS=true cargo test

# Skip OAuth tests
cargo test -- --skip oauth
```

**MCP Compliance Failures:**
```bash
# Debug MCP protocol issues
RUST_LOG=debug cargo test --test mcp_compliance_test

# Check protocol version
cargo test test_mcp_protocol_version -- --nocapture
```

**Rate Limiting in Tests:**
```bash
# Disable rate limiting for tests
AUTO_APPROVE_KEYS=true cargo test

# Use high rate limits
TEST_RATE_LIMIT=999999 cargo test
```

### Test Data Management

```bash
# Generate test data
cargo run --bin create-test-data

# Clean up test artifacts
rm -rf ./test_data/
rm -f test.db coverage/
```

### Debug Test Failures

```bash
# Run single test with output
cargo test test_name -- --nocapture

# Run tests with backtraces
RUST_BACKTRACE=1 cargo test

# Run with debug logging
RUST_LOG=debug cargo test
```

## Contributing Tests

### Writing New Tests

1. **Unit Tests**: Add to relevant module
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       
       #[test]
       fn test_function_name() {
           // Test implementation
       }
   }
   ```

2. **Integration Tests**: Add to `tests/` directory
   ```rust
   // tests/new_feature_test.rs
   use pierre_mcp_server::*;
   
   #[tokio::test]
   async fn test_new_feature() {
       // Integration test
   }
   ```

3. **Update Test Script**: Add new test suites to `./scripts/lint-and-test.sh`

### Test Guidelines

- Tests should be fast and deterministic
- Use mock data when possible
- Test both success and error cases
- Include edge cases and boundary conditions
- Document complex test scenarios
- Clean up resources after tests