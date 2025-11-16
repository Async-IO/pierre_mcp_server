# database architecture & abstraction layer

This chapter explores Pierre's database architecture including the abstraction layer, factory pattern for multi-database support, shared utilities, encryption, transactions, and migration system.

## what you'll learn

- DatabaseProvider trait abstraction
- Factory pattern for SQLite/PostgreSQL selection
- Feature flags for database backends
- AAD-based encryption for sensitive data
- Transaction retry patterns with exponential backoff
- Shared utilities (builders, mappers, validation)
- Migration system with SQLx
- Connection pooling strategies
- Multi-tenant data isolation at database level

## database abstraction architecture

Pierre uses a trait-based abstraction to support multiple database backends:

```
┌────────────────────────────────────────────────────────┐
│           DatabaseProvider Trait                       │
│  (Unified interface for all database operations)       │
└────────────────────────────────────────────────────────┘
                         │
        ┌────────────────┴────────────────┐
        │                                 │
        ▼                                 ▼
┌──────────────┐                 ┌──────────────┐
│   SQLite     │                 │  PostgreSQL  │
│ Implementation│                 │Implementation│
│              │                 │              │
│ - Local dev  │                 │ - Production │
│ - Testing    │                 │ - Cloud      │
│ - Embedded   │                 │ - Scalable   │
└──────────────┘                 └──────────────┘
```

**Source**: src/database_plugins/mod.rs:38-43
```rust
/// Core database abstraction trait
///
/// All database implementations must implement this trait to provide
/// a consistent interface for the application layer.
#[async_trait]
pub trait DatabaseProvider: Send + Sync + Clone {
    /// Create a new database connection with encryption key
    async fn new(database_url: &str, encryption_key: Vec<u8>) -> Result<Self>
    where
        Self: Sized;

    /// Run database migrations to set up schema
    async fn migrate(&self) -> Result<()>;

    // User Management
    async fn create_user(&self, user: &User) -> Result<Uuid>;
    async fn get_user(&self, user_id: Uuid) -> Result<Option<User>>;
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>>;

    // OAuth Tokens (Multi-Tenant)
    async fn upsert_user_oauth_token(&self, token: &UserOAuthToken) -> Result<()>;
    async fn get_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> Result<Option<UserOAuthToken>>;

    // ... 100+ more methods
}
```

**Key design principles**:
- **Async trait**: All operations are async for non-blocking I/O
- **Send + Sync**: Thread-safe, can be shared across async tasks
- **Clone**: Cheap cloning via Arc wrappers internally
- **Unified interface**: Same API regardless of database backend

## factory pattern for database selection

Pierre automatically detects and instantiates the correct database backend:

**Source**: src/database_plugins/factory.rs:38-46
```rust
/// Database instance wrapper that delegates to the appropriate implementation
#[derive(Clone)]
pub enum Database {
    /// SQLite database instance
    SQLite(SqliteDatabase),
    /// PostgreSQL database instance (requires postgresql feature)
    #[cfg(feature = "postgresql")]
    PostgreSQL(PostgresDatabase),
}
```

**Automatic detection**:

**Source**: src/database_plugins/factory.rs:164-184
```rust
/// Automatically detect database type from connection string
pub fn detect_database_type(database_url: &str) -> Result<DatabaseType> {
    if database_url.starts_with("sqlite:") {
        Ok(DatabaseType::SQLite)
    } else if database_url.starts_with("postgresql://") || database_url.starts_with("postgres://") {
        #[cfg(feature = "postgresql")]
        return Ok(DatabaseType::PostgreSQL);

        #[cfg(not(feature = "postgresql"))]
        return Err(AppError::config(
            "PostgreSQL connection string detected, but PostgreSQL support is not enabled. \
             Enable the 'postgresql' feature flag in Cargo.toml",
        )
        .into());
    } else {
        Err(AppError::config(format!(
            "Unsupported database URL format: {database_url}. \
             Supported formats: sqlite:path/to/db.sqlite, postgresql://user:pass@host/db"
        ))
        .into())
    }
}
```

**Usage**:
```rust
// Automatically selects SQLite or PostgreSQL based on URL
let database = Database::new(
    "sqlite:pierre.db",  // or "postgresql://localhost/pierre"
    encryption_key,
    &pool_config,
).await?;
```

## feature flags for database backends

Pierre uses Cargo feature flags for conditional compilation:

**Source**: Cargo.toml (conceptual)
```toml
[features]
default = ["sqlite"]
sqlite = []
postgresql = ["sqlx/postgres"]

[dependencies]
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "macros", "migrate"] }
```

**Conditional compilation**:

**Source**: src/database_plugins/factory.rs:44-45
```rust
/// PostgreSQL database instance (requires postgresql feature)
#[cfg(feature = "postgresql")]
PostgreSQL(PostgresDatabase),
```

**Build commands**:
```bash
# SQLite (default)
cargo build

# PostgreSQL
cargo build --features postgresql

# Both (for testing)
cargo build --all-features
```

## trait delegation pattern

The factory enum delegates all trait methods to the underlying implementation:

**Source**: src/database_plugins/factory.rs:219-243
```rust
#[async_trait]
impl DatabaseProvider for Database {
    async fn migrate(&self) -> Result<()> {
        match self {
            Self::SQLite(db) => db.migrate().await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.migrate().await,
        }
    }

    #[tracing::instrument(skip(self, user), fields(db_operation = "create_user", email = %user.email))]
    async fn create_user(&self, user: &crate::models::User) -> Result<uuid::Uuid> {
        match self {
            Self::SQLite(db) => db.create_user(user).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.create_user(user).await,
        }
    }

    // ... delegate all 100+ methods
}
```

**Tracing instrumentation**: `#[tracing::instrument]` macro adds database operation context to all logs.

## AAD-based encryption for OAuth tokens

Pierre encrypts OAuth tokens with Additional Authenticated Data (AAD) binding:

**Source**: src/database_plugins/shared/encryption.rs:12-47
```rust
/// Create AAD (Additional Authenticated Data) context for token encryption
///
/// Format: "{tenant_id}|{user_id}|{provider}|{table}"
///
/// This prevents cross-tenant token reuse attacks by binding the encrypted
/// token to its specific context. If an attacker copies an encrypted token
/// to a different tenant/user/provider context, decryption will fail due to
/// AAD mismatch.
#[must_use]
pub fn create_token_aad_context(
    tenant_id: &str,
    user_id: Uuid,
    provider: &str,
    table: &str,
) -> String {
    format!("{tenant_id}|{user_id}|{provider}|{table}")
}
```

**Encryption with AAD**:

**Source**: src/database_plugins/shared/encryption.rs:84-96
```rust
pub fn encrypt_oauth_token<D>(
    db: &D,
    token: &str,
    tenant_id: &str,
    user_id: Uuid,
    provider: &str,
) -> Result<String>
where
    D: HasEncryption,
{
    let aad_context = create_token_aad_context(tenant_id, user_id, provider, "user_oauth_tokens");
    db.encrypt_data_with_aad(token, &aad_context)
}
```

**Security benefits**:
- **AES-256-GCM**: AEAD cipher with authentication
- **AAD binding**: Token bound to tenant/user/provider context
- **Cross-tenant protection**: Can't copy encrypted token to different tenant
- **Tampering detection**: AAD verification fails if data modified
- **Compliance**: GDPR, HIPAA, SOC 2 encryption-at-rest requirements

**AAD format example**:
```
tenant-123|550e8400-e29b-41d4-a716-446655440000|strava|user_oauth_tokens
```

## transaction retry patterns

Pierre handles database deadlocks and transient errors with exponential backoff:

**Source**: src/database_plugins/shared/transactions.rs:59-105
```rust
/// Retry a transaction operation if it fails due to deadlock or timeout
///
/// Exponential Backoff:
/// - Attempt 1: 10ms
/// - Attempt 2: 20ms
/// - Attempt 3: 40ms
/// - Attempt 4: 80ms
/// - Attempt 5: 160ms
pub async fn retry_transaction<F, Fut, T>(mut f: F, max_retries: u32) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut attempts = 0;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempts += 1;
                if attempts >= max_retries {
                    return Err(e);
                }

                let error_msg = format!("{e:?}");
                if is_retryable_error(&error_msg) {
                    let backoff_ms = 10 * (1 << attempts);
                    sleep(Duration::from_millis(backoff_ms)).await;
                } else {
                    // Non-retryable error
                    return Err(e);
                }
            }
        }
    }
}
```

**Retryable errors**:

**Source**: src/database_plugins/shared/transactions.rs:120-150
```rust
fn is_retryable_error(error_msg: &str) -> bool {
    let error_lower = error_msg.to_lowercase();

    // Retryable: Deadlock and locking errors
    if error_lower.contains("deadlock")
        || error_lower.contains("database is locked")
        || error_lower.contains("locked")
        || error_lower.contains("busy")
    {
        return true;
    }

    // Retryable: Timeout errors
    if error_lower.contains("timeout") || error_lower.contains("timed out") {
        return true;
    }

    // Retryable: Serialization failures (PostgreSQL)
    if error_lower.contains("serialization failure") {
        return true;
    }

    // Non-retryable: Constraint violations
    if error_lower.contains("unique constraint")
        || error_lower.contains("foreign key constraint")
        || error_lower.contains("check constraint")
    {
        return false;
    }

    false
}
```

**Usage example**:
```rust
use crate::database_plugins::shared::transactions::retry_transaction;

retry_transaction(
    || async {
        db.create_user(&user).await
    },
    3 // max retries
).await?;
```

## shared database utilities

The `shared` module provides reusable components across backends (880 lines total), eliminating massive code duplication. The refactoring deleted the 3,058-line `sqlite.rs` wrapper file entirely.

**Structure**:
```
src/database_plugins/shared/
├── mod.rs              # Module exports (23 lines)
├── encryption.rs       # AAD-based encryption utilities (201 lines)
├── transactions.rs     # Retry patterns with backoff (162 lines)
├── enums.rs            # Shared enum conversions (143 lines)
├── mappers.rs          # Row -> struct conversion (192 lines)
├── validation.rs       # Input validation (150 lines)
└── builders.rs         # Query builder helpers (9 lines, deferred)
```

**Benefits**:
1. **DRY principle**: No duplicate encryption/retry logic
2. **Consistency**: Same behavior across SQLite and PostgreSQL
3. **Testability**: Shared utilities tested once, work everywhere
4. **Maintainability**: Bug fixes apply to all backends
5. **Code reduction**: Eliminated 3,058 lines of wrapper boilerplate

### enum conversions (enums.rs)

**Source**: src/database_plugins/shared/enums.rs:24-50
```rust
/// Convert UserTier enum to database string representation
#[must_use]
#[inline]
pub const fn user_tier_to_str(tier: &UserTier) -> &'static str {
    match tier {
        UserTier::Starter => tiers::STARTER,
        UserTier::Professional => tiers::PROFESSIONAL,
        UserTier::Enterprise => tiers::ENTERPRISE,
    }
}

/// Convert database string to UserTier enum
/// Unknown values default to Starter tier for safety
#[must_use]
pub fn str_to_user_tier(s: &str) -> UserTier {
    match s {
        tiers::PROFESSIONAL | "pro" => UserTier::Professional,
        tiers::ENTERPRISE => UserTier::Enterprise,
        _ => UserTier::Starter,
    }
}
```

**Also includes**:
- `user_status_to_str()` / `str_to_user_status()` - Active/Pending/Suspended
- `task_status_to_str()` / `str_to_task_status()` - Pending/Running/Completed/Failed/Cancelled

**Why this matters**: Both SQLite and PostgreSQL store enums as TEXT, requiring identical conversion logic. Sharing this eliminates duplicate code and ensures consistent enum handling.

### generic row parsing (mappers.rs)

**Database-agnostic User parsing**:

**Source**: src/database_plugins/shared/mappers.rs:37-74
```rust
/// Parse User from database row (works with PostgreSQL and SQLite)
pub fn parse_user_from_row<R>(row: &R) -> Result<User>
where
    R: sqlx::Row,
    for<'a> &'a str: sqlx::ColumnIndex<R>,
    for<'a> usize: sqlx::ColumnIndex<R>,
    Uuid: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    String: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    Option<String>: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    // ... extensive trait bounds
{
    // Parse enum fields using shared converters
    let user_status_str: String = row.try_get("user_status")?;
    let user_status = super::enums::str_to_user_status(&user_status_str);

    let tier_str: String = row.try_get("tier")?;
    let tier = super::enums::str_to_user_tier(&tier_str);

    Ok(User {
        id: row.try_get("id")?,
        email: row.try_get("email")?,
        display_name: row.try_get("display_name")?,
        password_hash: row.try_get("password_hash")?,
        tier,
        tenant_id: row.try_get("tenant_id")?,
        is_active: row.try_get("is_active")?,
        user_status,
        is_admin: row.try_get("is_admin").unwrap_or(false),
        approved_by: row.try_get("approved_by")?,
        approved_at: row.try_get("approved_at")?,
        created_at: row.try_get("created_at")?,
        last_active: row.try_get("last_active")?,
        // OAuth tokens loaded separately
        strava_token: None,
        fitbit_token: None,
    })
}
```

**UUID handling across databases**:

**Source**: src/database_plugins/shared/mappers.rs:177-192
```rust
/// Extract UUID from row (handles PostgreSQL UUID vs SQLite TEXT)
pub fn get_uuid_from_row<R>(row: &R, column: &str) -> Result<Uuid>
where
    R: sqlx::Row,
    for<'a> &'a str: sqlx::ColumnIndex<R>,
    Uuid: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    String: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
{
    // Try PostgreSQL UUID type first
    if let Ok(uuid) = row.try_get::<Uuid, _>(column) {
        return Ok(uuid);
    }

    // Fall back to SQLite TEXT (parse string)
    let uuid_str: String = row.try_get(column)?;
    Ok(Uuid::parse_str(&uuid_str)?)
}
```

**Why this matters**: PostgreSQL has native UUID support, SQLite stores UUIDs as TEXT. This helper abstracts the difference.

**Also includes**: `parse_a2a_task_from_row<R>()` for A2A task parsing with JSON deserialization.

### input validation (validation.rs)

**Email validation**:

**Source**: src/database_plugins/shared/validation.rs:34-39
```rust
/// Validate email format
pub fn validate_email(email: &str) -> Result<()> {
    if !email.contains('@') || email.len() < 3 {
        return Err(AppError::invalid_input("Invalid email format").into());
    }
    Ok(())
}
```

**Tenant ownership (authorization)**:

**Source**: src/database_plugins/shared/validation.rs:63-75
```rust
/// Validate that entity belongs to specified tenant
pub fn validate_tenant_ownership(
    entity_tenant_id: &str,
    expected_tenant_id: &str,
    entity_type: &str,
) -> Result<()> {
    if entity_tenant_id != expected_tenant_id {
        return Err(AppError::auth_invalid(format!(
            "{entity_type} does not belong to the specified tenant"
        ))
        .into());
    }
    Ok(())
}
```

**Expiration checks (OAuth tokens, sessions)**:

**Source**: src/database_plugins/shared/validation.rs:104-113
```rust
/// Validate expiration timestamp
pub fn validate_not_expired(
    expires_at: DateTime<Utc>,
    now: DateTime<Utc>,
    entity_type: &str,
) -> Result<()> {
    if expires_at <= now {
        return Err(AppError::invalid_input(format!("{entity_type} has expired")).into());
    }
    Ok(())
}
```

**Scope authorization (OAuth2, A2A)**:

**Source**: src/database_plugins/shared/validation.rs:140-150
```rust
/// Validate scope authorization
pub fn validate_scope_granted(
    requested_scopes: &[String],
    granted_scopes: &[String],
) -> Result<()> {
    for scope in requested_scopes {
        if !granted_scopes.contains(scope) {
            return Err(AppError::auth_invalid(format!("Scope '{scope}' not granted")).into());
        }
    }
    Ok(())
}
```

**Why this matters**: Multi-tenant authorization and OAuth validation logic is identical across backends. Centralizing prevents divergence.

### query builders (builders.rs) - deferred

**Source**: src/database_plugins/shared/builders.rs:1-9
```rust
// This module is currently minimal as the query parameter binding helpers
// were deferred to a later phase. The existing database implementations
// use direct .bind() chains which, while verbose, are explicit and type-safe.
```

**Status**: Query builder helpers were identified in the refactoring analysis but deferred to a later phase. Current implementations use direct `.bind()` chains, which are more verbose but maintain type safety and explicitness.

### HasEncryption trait

Both database backends implement this shared encryption interface:

**Source**: src/database_plugins/shared/encryption.rs:129-137
```rust
/// Trait for database encryption operations
pub trait HasEncryption {
    /// Encrypt data with Additional Authenticated Data (AAD) context
    fn encrypt_data_with_aad(&self, data: &str, aad: &str) -> Result<String>;

    /// Decrypt data with AAD context verification
    fn decrypt_data_with_aad(&self, encrypted: &str, aad: &str) -> Result<String>;
}
```

**Why this matters**: Allows shared encryption utilities to work with both SQLite and PostgreSQL implementations through trait bounds.

## rust idioms: delegation pattern

**Pattern**: Enum with match for delegation
```rust
enum Database {
    SQLite(SqliteDatabase),
    PostgreSQL(PostgresDatabase),
}

impl DatabaseProvider for Database {
    async fn operation(&self) -> Result<T> {
        match self {
            Self::SQLite(db) => db.operation().await,
            Self::PostgreSQL(db) => db.operation().await,
        }
    }
}
```

**Why this works**:
- **Type safety**: Compiler ensures all enum variants handled
- **Zero cost**: Enum dispatch optimizes to static dispatch
- **Extensibility**: Easy to add new database backends

## rust idioms: conditional compilation

**Pattern**: Feature-gated code with clear error messages
```rust
#[cfg(feature = "postgresql")]
PostgreSQL(PostgresDatabase),

#[cfg(not(feature = "postgresql"))]
DatabaseType::PostgreSQL => {
    Err(AppError::config(
        "PostgreSQL support not enabled. Enable the 'postgresql' feature flag."
    ).into())
}
```

**Benefits**:
- **Binary size**: SQLite-only builds exclude PostgreSQL dependencies
- **Compilation speed**: Only compile enabled backends
- **Clear errors**: Helpful messages when feature missing

## connection pooling (PostgreSQL)

PostgreSQL implementation uses connection pooling for performance:

**Configuration** (src/config/environment.rs - conceptual):
```rust
pub struct PostgresPoolConfig {
    pub max_connections: u32,          // Default: 10
    pub min_connections: u32,          // Default: 2
    pub acquire_timeout_secs: u64,     // Default: 30
    pub idle_timeout_secs: Option<u64>, // Default: 600 (10 min)
    pub max_lifetime_secs: Option<u64>, // Default: 1800 (30 min)
}
```

**Pool creation**:
```rust
let pool = PgPoolOptions::new()
    .max_connections(config.max_connections)
    .min_connections(config.min_connections)
    .acquire_timeout(Duration::from_secs(config.acquire_timeout_secs))
    .idle_timeout(config.idle_timeout_secs.map(Duration::from_secs))
    .max_lifetime(config.max_lifetime_secs.map(Duration::from_secs))
    .connect(&database_url)
    .await?;
```

**Why pooling**:
- **Performance**: Reuse connections, avoid handshake overhead
- **Concurrency**: Handle multiple simultaneous requests
- **Resource limits**: Cap max connections to database
- **Health**: Recycle connections after max lifetime

## migration system

Pierre uses SQLx migrations for schema management:

**Migration files** (migrations/*.sql):
```sql
-- migrations/20240101000001_create_users.sql
CREATE TABLE users (
    id UUID PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    status TEXT NOT NULL DEFAULT 'pending'
);

-- migrations/20240101000002_create_oauth_tokens.sql
CREATE TABLE user_oauth_tokens (
    user_id UUID NOT NULL,
    tenant_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    access_token TEXT NOT NULL,  -- Encrypted
    refresh_token TEXT,           -- Encrypted
    expires_at TIMESTAMP,
    PRIMARY KEY (user_id, tenant_id, provider),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
```

**Migration execution**:
```rust
async fn migrate(&self) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(&self.pool)
        .await?;
    Ok(())
}
```

**Migration benefits**:
- **Version control**: Migrations tracked in git
- **Reproducibility**: Same schema on dev/staging/prod
- **Rollback**: Down migrations for reverting changes
- **Type safety**: SQLx compile-time query verification

## multi-tenant data isolation

Database schema enforces tenant isolation:

**Composite primary keys**:
```sql
CREATE TABLE user_oauth_tokens (
    user_id UUID NOT NULL,
    tenant_id TEXT NOT NULL,  -- Part of primary key
    provider TEXT NOT NULL,
    -- ...
    PRIMARY KEY (user_id, tenant_id, provider)
);
```

**Queries always include tenant_id**:
```rust
sqlx::query_as!(
    UserOAuthToken,
    r#"
    SELECT * FROM user_oauth_tokens
    WHERE user_id = $1
      AND tenant_id = $2
      AND provider = $3
    "#,
    user_id,
    tenant_id,  // Always filtered
    provider
)
.fetch_optional(&self.pool)
.await?
```

**AAD encryption binding**: Tenant ID in AAD prevents cross-tenant token copying at encryption layer.

## key takeaways

1. **DatabaseProvider trait**: Unified interface for SQLite and PostgreSQL (100+ async methods).

2. **Factory pattern**: Automatic database type detection from connection string.

3. **Feature flags**: Conditional compilation for database backends.

4. **Delegation enum**: Database enum delegates to underlying implementation.

5. **AAD encryption**: OAuth tokens encrypted with tenant/user/provider binding via `HasEncryption` trait.

6. **Transaction retry**: Exponential backoff for deadlock/timeout errors (10ms to 160ms).

7. **Shared utilities**: 880 lines across 6 modules eliminated 3,058 lines of wrapper boilerplate:
   - **enums.rs**: UserTier, UserStatus, TaskStatus conversions
   - **mappers.rs**: Generic row parsing with complex trait bounds
   - **validation.rs**: Email, tenant ownership, expiration, scope checks
   - **encryption.rs**: AAD-based encryption utilities
   - **transactions.rs**: Retry patterns with exponential backoff
   - **builders.rs**: Deferred to later phase (minimal implementation)

8. **Connection pooling**: PostgreSQL uses pooling for performance and concurrency.

9. **Migration system**: SQLx migrations for version-controlled schema changes.

10. **Multi-tenant isolation**: Composite keys and AAD binding enforce tenant boundaries.

11. **Instrumentation**: Tracing macros add database operation context to logs.

12. **Error handling**: Clear messages when feature flags missing or URLs invalid.

13. **Code reduction**: Refactoring deleted the entire 3,058-line `sqlite.rs` wrapper file.

---

**Placement Note**: This chapter should be inserted early in the tutorial (after Chapter 3: Configuration or as Chapter 4) since database architecture is foundational to understanding the rest of the system.

**Related Chapters**:
- Chapter 5: Cryptographic Keys (encryption key management)
- Chapter 7: Multi-Tenant Isolation (application-layer tenant context)
- Chapter 23: Testing Framework (database testing patterns)
