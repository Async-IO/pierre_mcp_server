<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 23: Testing Framework - Comprehensive Testing Patterns

This chapter covers Pierre's testing infrastructure including database testing, integration patterns, synthetic data generation, async testing, error testing, and test organization best practices.

## What You'll Learn

- Database testing patterns (in-memory databases, isolation, fixtures)
- Integration test patterns for MCP protocol
- Async test patterns with tokio::test
- Synthetic data generation with seeded RNG
- Test helper utilities and common fixtures
- Error testing and edge case validation
- Mock/stub patterns for external APIs
- Test setup and teardown patterns
- Test organization (unit, integration, E2E)

## Database Testing Patterns

Pierre uses in-memory SQLite databases for fast, isolated tests without external dependencies.

### In-Memory Database Setup

**Source**: tests/database_memory_test.rs:18-71
```rust
#[tokio::test]
async fn test_memory_database_no_physical_files() -> Result<()> {
    let encryption_key = generate_encryption_key().to_vec();

    // Create in-memory database - NO physical files
    let database = Database::new("sqlite::memory:", encryption_key).await?;

    // Verify no physical files are created
    let current_dir = std::env::current_dir()?;
    let entries = fs::read_dir(&current_dir)?;

    for entry in entries {
        let entry = entry?;
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();

        assert!(
            !filename_str.starts_with(":memory:test_"),
            "Found physical file that should be in-memory: {filename_str}"
        );
    }

    // Test basic database functionality
    let user = User::new(
        "test@memory.test".to_owned(),
        "password_hash".to_owned(),
        Some("Memory Test User".to_owned()),
    );

    let user_id = database.create_user(&user).await?;
    let retrieved_user = database.get_user(user_id).await?.unwrap();

    assert_eq!(retrieved_user.email, "test@memory.test");
    assert_eq!(retrieved_user.display_name, Some("Memory Test User".to_owned()));

    Ok(())
}
```

**Benefits**:
- **Fast**: No disk I/O, tests run in milliseconds
- **Isolated**: Each test gets independent database
- **No cleanup**: Memory automatically freed after test
- **Deterministic**: No race conditions from shared state

### Database Isolation Testing

**Source**: tests/database_memory_test.rs:74-126
```rust
#[tokio::test]
async fn test_multiple_memory_databases_isolated() -> Result<()> {
    let encryption_key1 = generate_encryption_key().to_vec();
    let encryption_key2 = generate_encryption_key().to_vec();

    // Create two separate in-memory databases
    let database1 = Database::new("sqlite::memory:", encryption_key1).await?;
    let database2 = Database::new("sqlite::memory:", encryption_key2).await?;

    // Create users in each database
    let user1 = User::new(
        "user1@test.com".to_owned(),
        "hash1".to_owned(),
        Some("User 1".to_owned()),
    );

    let user2 = User::new(
        "user2@test.com".to_owned(),
        "hash2".to_owned(),
        Some("User 2".to_owned()),
    );

    let user1_id = database1.create_user(&user1).await?;
    let user2_id = database2.create_user(&user2).await?;

    // Verify isolation - each database only contains its own user
    assert!(database1.get_user(user1_id).await?.is_some());
    assert!(database2.get_user(user2_id).await?.is_some());

    // User1 should not exist in database2 and vice versa
    assert!(database2.get_user(user1_id).await?.is_none());
    assert!(database1.get_user(user2_id).await?.is_none());

    Ok(())
}
```

**Why isolation matters**: Tests can run in parallel without interfering. Each test gets clean database state.

### Test Fixture Helpers

**Common test fixtures** (tests/common.rs - conceptual):
```rust
/// Create test database with migrations applied
pub async fn create_test_database() -> Result<Arc<Database>> {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key).await?;
    database.migrate().await?;
    Ok(Arc::new(database))
}

/// Create test auth manager with default config
pub fn create_test_auth_manager() -> Arc<AuthManager> {
    Arc::new(AuthManager::new())
}

/// Create test cache
pub async fn create_test_cache() -> Result<Arc<Cache>> {
    Ok(Arc::new(Cache::new()))
}

/// Initialize server config from environment
pub fn init_server_config() {
    std::env::set_var("JWT_SECRET", "test_jwt_secret");
    std::env::set_var("ENCRYPTION_KEY", "test_encryption_key_32_bytes_long");
}
```

**Pattern**: Centralized test helpers reduce duplication and ensure consistent test setup.

## Integration Testing Patterns

Pierre tests MCP protocol handlers using structured JSON-RPC requests.

### MCP Request Helpers

**Source**: tests/mcp_protocol_comprehensive_test.rs:27-47
```rust
/// Test helper to create MCP request
fn create_mcp_request(method: &str, params: Option<&Value>, id: Option<Value>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": id.unwrap_or_else(|| json!(1))
    })
}

/// Test helper to create authenticated MCP request
fn create_auth_mcp_request(
    method: &str,
    params: Option<&Value>,
    token: &str,
    id: Option<Value>,
) -> Value {
    let mut request = create_mcp_request(method, params, id);
    request["auth_token"] = json!(token);
    request
}
```

### MCP Protocol Integration Test

**Source**: tests/mcp_protocol_comprehensive_test.rs:49-77
```rust
#[tokio::test]
async fn test_mcp_initialize_request() -> Result<()> {
    common::init_server_config();
    let database = common::create_test_database().await?;
    let auth_manager = common::create_test_auth_manager();
    let config = Arc::new(ServerConfig::from_env()?);

    let cache = common::create_test_cache().await.unwrap();
    let resources = Arc::new(ServerResources::new(
        (*database).clone(),
        (*auth_manager).clone(),
        TEST_JWT_SECRET,
        config,
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution
        Some(common::get_shared_test_jwks()),
    ));
    let server = MultiTenantMcpServer::new(resources);

    // Test initialize request
    let _request = create_mcp_request("initialize", None, Some(json!("init-1")));

    // Validate server is properly initialized
    let _ = server.database();

    Ok(())
}
```

**Pattern**: Integration tests validate component interactions (server → database → auth) without mocking.

### Authentication Testing

**Source**: tests/mcp_protocol_comprehensive_test.rs:137-175
```rust
#[tokio::test]
async fn test_mcp_authenticate_request() -> Result<()> {
    common::init_server_config();
    let database = common::create_test_database().await?;
    let auth_manager = common::create_test_auth_manager();
    let config = Arc::new(ServerConfig::from_env()?);

    let cache = common::create_test_cache().await.unwrap();
    let resources = Arc::new(ServerResources::new(
        (*database).clone(),
        (*auth_manager).clone(),
        TEST_JWT_SECRET,
        config,
        cache,
        2048,
        Some(common::get_shared_test_jwks()),
    ));
    let _server = MultiTenantMcpServer::new(resources);

    // Create test user
    let user = User::new(
        "mcp_auth@example.com".to_owned(),
        "password123".to_owned(),
        Some("MCP Auth Test".to_owned()),
    );
    database.create_user(&user).await?;

    // Test authenticate request format
    let auth_params = json!({
        "email": "mcp_auth@example.com",
        "password": "password123"
    });
    let request = create_mcp_request("authenticate", Some(&auth_params), Some(json!("auth-1")));

    assert_eq!(request["method"], "authenticate");
    assert_eq!(request["params"]["email"], "mcp_auth@example.com");

    Ok(())
}
```

**Pattern**: Create test user → Construct auth request → Validate request structure.

## Async Testing Patterns

Pierre uses `#[tokio::test]` for async test execution.

### Async Test Basics

```rust
#[tokio::test]
async fn test_async_database_operation() -> Result<()> {
    let database = create_test_database().await?;

    // Async operations work naturally
    let user = User::new("test@example.com".to_owned(), "hash".to_owned(), None);
    let user_id = database.create_user(&user).await?;

    // Multiple awaits in sequence
    let retrieved = database.get_user(user_id).await?;
    assert!(retrieved.is_some());

    Ok(())
}
```

**tokio::test features**:
- **Multi-threaded runtime**: Tests run on tokio runtime
- **Async/await support**: Natural async syntax
- **Automatic cleanup**: Runtime shut down after test
- **Error propagation**: `Result<()>` with `?` operator

### Concurrent Async Operations

```rust
#[tokio::test]
async fn test_concurrent_database_writes() -> Result<()> {
    let database = create_test_database().await?;

    // Spawn multiple concurrent tasks
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let db = database.clone();
            tokio::spawn(async move {
                let user = User::new(
                    format!("user{}@test.com", i),
                    "hash".to_owned(),
                    None,
                );
                db.create_user(&user).await
            })
        })
        .collect();

    // Wait for all tasks to complete
    for handle in handles {
        handle.await??;
    }

    Ok(())
}
```

**Pattern**: Test concurrent behavior with `tokio::spawn` to validate thread safety.

## Synthetic Data Generation

Pierre uses deterministic synthetic data for reproducible tests (covered in Chapter 14).

**Key benefits**:
- **No OAuth required**: Tests run without external API dependencies
- **Deterministic**: Seeded RNG ensures same data every run
- **Realistic**: Physiologically plausible activity data
- **Fast**: In-memory synthetic provider, no network calls

**Usage example** (tests/intelligence_synthetic_helpers_test.rs):
```rust
#[tokio::test]
async fn test_beginner_progression_algorithm() {
    let mut builder = SyntheticDataBuilder::new(42); // Deterministic seed
    let activities = builder.generate_pattern(TrainingPattern::BeginnerRunnerImproving);
    let provider = SyntheticProvider::with_activities(activities);

    // Test intelligence algorithms without OAuth
    let trends = analyze_performance_trends(&provider).await?;
    assert!(trends.pace_improvement > 0.30); // Expect 35% improvement
}
```

## Test Helpers and Scenario Builders

Pierre provides reusable test helpers for common testing patterns.

### Scenario-Based Testing

**Source**: tests/helpers/test_utils.rs:9-33
```rust
/// Test scenarios for intelligence testing
#[derive(Debug, Clone, Copy)]
pub enum TestScenario {
    /// Beginner runner showing 35% improvement over 6 weeks
    BeginnerRunnerImproving,
    /// Experienced cyclist with stable, consistent performance
    ExperiencedCyclistConsistent,
    /// Athlete showing signs of overtraining (TSB < -30)
    OvertrainingRisk,
    /// Return from injury with gradual progression
    InjuryRecovery,
}

impl TestScenario {
    /// Get the corresponding pattern from synthetic data builder
    #[must_use]
    pub const fn to_training_pattern(self) -> TrainingPattern {
        match self {
            Self::BeginnerRunnerImproving => TrainingPattern::BeginnerRunnerImproving,
            Self::ExperiencedCyclistConsistent => TrainingPattern::ExperiencedCyclistConsistent,
            Self::OvertrainingRisk => TrainingPattern::Overtraining,
            Self::InjuryRecovery => TrainingPattern::InjuryRecovery,
        }
    }
}
```

### Scenario Provider Creation

**Source**: tests/helpers/test_utils.rs:35-42
```rust
/// Create a synthetic provider with pre-configured scenario data
#[must_use]
pub fn create_synthetic_provider_with_scenario(scenario: TestScenario) -> SyntheticProvider {
    let mut builder = SyntheticDataBuilder::new(42); // Deterministic seed
    let activities = builder.generate_pattern(scenario.to_training_pattern());

    SyntheticProvider::with_activities(activities)
}
```

**Usage**:
```rust
#[tokio::test]
async fn test_overtraining_detection() -> Result<()> {
    let provider = create_synthetic_provider_with_scenario(TestScenario::OvertrainingRisk);

    let recovery = calculate_recovery_score(&provider).await?;
    assert!(recovery.tsb < -30.0); // Overtraining threshold

    Ok(())
}
```

**Benefits**:
- **Readable tests**: `TestScenario::BeginnerRunnerImproving` vs raw data construction
- **Reusable**: Same scenarios across multiple test files
- **Maintainable**: Change scenario in one place, all tests update

## Error Testing Patterns

Test error conditions explicitly to validate error handling.

### Testing Error Cases

```rust
#[tokio::test]
async fn test_duplicate_user_email_rejected() -> Result<()> {
    let database = create_test_database().await?;

    let user1 = User::new("duplicate@test.com".to_owned(), "hash1".to_owned(), None);
    let user2 = User::new("duplicate@test.com".to_owned(), "hash2".to_owned(), None);

    // First user succeeds
    database.create_user(&user1).await?;

    // Second user with same email fails
    let result = database.create_user(&user2).await;
    assert!(result.is_err());

    // Verify error type
    let err = result.unwrap_err();
    assert!(err.to_string().contains("UNIQUE constraint"));

    Ok(())
}
```

### Testing Validation Errors

```rust
#[tokio::test]
async fn test_invalid_email_rejected() -> Result<()> {
    use pierre_mcp_server::database_plugins::shared::validation::validate_email;

    // Test various invalid email formats
    let invalid_emails = vec![
        "notanemail",
        "@test.com",
        "test@",
        "a@b",
        "",
    ];

    for email in invalid_emails {
        let result = validate_email(email);
        assert!(result.is_err(), "Email '{}' should be invalid", email);
    }

    // Valid email passes
    assert!(validate_email("valid@example.com").is_ok());

    Ok(())
}
```

**Pattern**: Test both success path AND failure paths to ensure error handling works.

## Test Organization

Pierre organizes tests by scope and type with 1,635 lines of test helper code.

**Test directory structure**:
```
tests/
├── helpers/                        # 1,635 lines of shared test utilities
│   ├── synthetic_data.rs           # Deterministic test data generation
│   ├── synthetic_provider.rs       # In-memory provider for testing
│   └── test_utils.rs               # Scenario builders and assertions
├── database_memory_test.rs         # Database isolation tests
├── mcp_protocol_comprehensive_test.rs  # MCP integration tests
├── admin_jwt_test.rs               # JWT authentication tests
├── oauth_e2e_test.rs               # OAuth flow E2E tests
├── intelligence_recovery_calculator_test.rs  # Algorithm tests
├── pagination_test.rs              # Pagination logic tests
├── configuration_profiles_test.rs  # Config validation tests
└── [40+ additional test files]
```

**Test categories**:
- **Database tests**: In-memory isolation, transaction handling, migration validation
- **Integration tests**: MCP protocol, OAuth flows, provider interactions
- **Algorithm tests**: Recovery calculations, nutrition calculations, performance analysis
- **E2E tests**: Full user workflows from authentication to data retrieval
- **Unit tests**: Validation functions, enum conversions, mappers

## Key Test Patterns

**Pattern 1: Builder for test data**
```rust
let activity = ActivityBuilder::new(SportType::Run)
    .distance_km(10.0)
    .duration_minutes(50)
    .average_hr(150)
    .build();
```

**Pattern 2: Seeded RNG for determinism**
```rust
let mut builder = SyntheticDataBuilder::new(42); // Same seed = same data
```

**Pattern 3: Synthetic provider for isolation**
```rust
let provider = SyntheticProvider::with_activities(vec![activity1, activity2]);
let result = service.analyze(&provider).await?;
```

## Key Takeaways

1. **In-memory databases**: `sqlite::memory:` provides fast, isolated tests without physical files or cleanup overhead.

2. **Database isolation**: Each test gets independent database instance, enabling safe parallel test execution.

3. **Test fixtures**: Centralized helpers like `create_test_database()` ensure consistent test setup across all tests.

4. **Integration testing**: MCP protocol tests validate component interactions (server → database → auth) without mocking.

5. **JSON-RPC helpers**: `create_mcp_request()` and `create_auth_mcp_request()` simplify MCP protocol testing.

6. **Async testing**: `#[tokio::test]` provides multi-threaded async runtime for natural async/await syntax in tests.

7. **Concurrent testing**: `tokio::spawn` validates thread safety by testing concurrent database writes and reads.

8. **Scenario-based testing**: `TestScenario` enum provides readable, reusable test scenarios (BeginnerRunnerImproving, OvertrainingRisk).

9. **Synthetic data**: Deterministic test data with seeded RNG (`SyntheticDataBuilder::new(42)`) ensures reproducible tests without OAuth.

10. **Error testing**: Explicitly test failure paths (duplicate emails, invalid data) to validate error handling works.

11. **Test organization**: 1,635 lines of helper code in `tests/helpers/` plus 40+ test files organized by category.

12. **Builder pattern**: Fluent API for constructing test activities and data structures.

13. **Validation testing**: Test shared validation functions (`validate_email`, `validate_tenant_ownership`) with multiple invalid inputs.

14. **No external dependencies**: Tests run offline using in-memory databases and synthetic providers.

15. **Fast execution**: In-memory databases + synthetic data = millisecond test times, enabling rapid development feedback.

---

**Next Chapter**: [Chapter 24: Design System](./chapter-24-design-system.md) - Learn about Pierre's design system, templates, frontend architecture, and user experience patterns.
