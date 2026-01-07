<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Database Architecture & Repository Pattern

This chapter explores Pierre's database architecture using the repository pattern with 13 focused repository traits following SOLID principles.

## What You'll Learn

- Repository pattern for database abstraction
- 13 focused repository traits
- Factory pattern for SQLite/PostgreSQL selection
- Feature flags for database backends
- AAD-based encryption for sensitive data
- Transaction retry patterns with exponential backoff
- Shared utilities (builders, mappers, validation)
- Migration system with SQLx
- Connection pooling strategies
- Multi-tenant data isolation at database level

## Repository Pattern Architecture

Pierre uses a repository pattern to provide focused, cohesive interfaces for database operations:

```
┌────────────────────────────────────────────────────────┐
│              Database (Core)                           │
│  Provides accessor methods for repositories            │
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
         │                               │
         └───────────────┬───────────────┘
                         │
         ┌───────────────┴───────────────┐
         │    13 Repository Traits        │
         ├────────────────────────────────┤
         │  • UserRepository              │
         │  • OAuthTokenRepository        │
         │  • ApiKeyRepository            │
         │  • UsageRepository             │
         │  • A2ARepository               │
         │  • ProfileRepository           │
         │  • InsightRepository           │
         │  • AdminRepository             │
         │  • TenantRepository            │
         │  • SecurityRepository          │
         │  • NotificationRepository      │
         │  • OAuth2ServerRepository      │
         │  • FitnessConfigRepository     │
         └────────────────────────────────┘
```

**Why repository pattern?**

The repository pattern follows SOLID principles:
- **Single Responsibility**: Each repository handles one domain (users, tokens, keys, etc.)
- **Interface Segregation**: Consumers depend only on the methods they need
- **Testability**: Mock individual repositories independently
- **Maintainability**: Changes isolated to specific repositories

Each of the 13 repository traits contains 5-20 cohesive methods for its domain.

## Repository Accessor Pattern

The `Database` struct provides accessor methods that return repository implementations:

**Source**: `src/database/mod.rs:139-230`
```rust
impl Database {
    /// Get UserRepository for user account management
    #[must_use]
    pub fn users(&self) -> repositories::UserRepositoryImpl {
        repositories::UserRepositoryImpl::new(
            crate::database_plugins::factory::Database::SQLite(self.clone())
        )
    }

    /// Get OAuthTokenRepository for OAuth token storage
    #[must_use]
    pub fn oauth_tokens(&self) -> repositories::OAuthTokenRepositoryImpl {
        repositories::OAuthTokenRepositoryImpl::new(
            crate::database_plugins::factory::Database::SQLite(self.clone())
        )
    }

    /// Get ApiKeyRepository for API key management
    #[must_use]
    pub fn api_keys(&self) -> repositories::ApiKeyRepositoryImpl {
        repositories::ApiKeyRepositoryImpl::new(
            crate::database_plugins::factory::Database::SQLite(self.clone())
        )
    }

    /// Get UsageRepository for usage tracking and analytics
    #[must_use]
    pub fn usage(&self) -> repositories::UsageRepositoryImpl {
        repositories::UsageRepositoryImpl::new(
            crate::database_plugins::factory::Database::SQLite(self.clone())
        )
    }

    /// Get A2ARepository for Agent-to-Agent management
    #[must_use]
    pub fn a2a(&self) -> repositories::A2ARepositoryImpl {
        repositories::A2ARepositoryImpl::new(
            crate::database_plugins::factory::Database::SQLite(self.clone())
        )
    }

    /// Get ProfileRepository for user profiles and goals
    #[must_use]
    pub fn profiles(&self) -> repositories::ProfileRepositoryImpl {
        repositories::ProfileRepositoryImpl::new(
            crate::database_plugins::factory::Database::SQLite(self.clone())
        )
    }

    /// Get InsightRepository for AI-generated insights
    #[must_use]
    pub fn insights(&self) -> repositories::InsightRepositoryImpl {
        repositories::InsightRepositoryImpl::new(
            crate::database_plugins::factory::Database::SQLite(self.clone())
        )
    }

    /// Get AdminRepository for admin token management
    #[must_use]
    pub fn admins(&self) -> repositories::AdminRepositoryImpl {
        repositories::AdminRepositoryImpl::new(
            crate::database_plugins::factory::Database::SQLite(self.clone())
        )
    }

    /// Get TenantRepository for multi-tenant management
    #[must_use]
    pub fn tenants(&self) -> repositories::TenantRepositoryImpl {
        repositories::TenantRepositoryImpl::new(
            crate::database_plugins::factory::Database::SQLite(self.clone())
        )
    }

    /// Get SecurityRepository for security and key rotation
    #[must_use]
    pub fn security(&self) -> repositories::SecurityRepositoryImpl {
        repositories::SecurityRepositoryImpl::new(
            crate::database_plugins::factory::Database::SQLite(self.clone())
        )
    }

    /// Get NotificationRepository for OAuth notifications
    #[must_use]
    pub fn notifications(&self) -> repositories::NotificationRepositoryImpl {
        repositories::NotificationRepositoryImpl::new(
            crate::database_plugins::factory::Database::SQLite(self.clone())
        )
    }

    /// Get OAuth2ServerRepository for OAuth 2.0 server
    #[must_use]
    pub fn oauth2_server(&self) -> repositories::OAuth2ServerRepositoryImpl {
        repositories::OAuth2ServerRepositoryImpl::new(
            crate::database_plugins::factory::Database::SQLite(self.clone())
        )
    }

    /// Get FitnessConfigRepository for fitness configuration
    #[must_use]
    pub fn fitness_configs(&self) -> repositories::FitnessConfigRepositoryImpl {
        repositories::FitnessConfigRepositoryImpl::new(
            crate::database_plugins::factory::Database::SQLite(self.clone())
        )
    }
}
```

**Usage pattern**:

```rust
// Repository pattern - access through typed accessors
let user = database.users().get_by_id(user_id).await?;
let token = database.oauth_tokens().get(user_id, tenant_id, provider).await?;
let keys = database.api_keys().list_by_user(user_id).await?;
```

**Benefits**:
- **Clarity**: `database.users().create(...)` is clearer than `database.create_user(...)`
- **Cohesion**: Related methods grouped together
- **Testability**: Can mock individual repositories
- **Interface Segregation**: Only depend on repositories you use

## The 13 Repository Traits

### 1. Userrepository - User Account Management

**Source**: `src/database/repositories/mod.rs:68-108`
```rust
/// User account management repository
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Create a new user account
    async fn create(&self, user: &User) -> Result<Uuid, DatabaseError>;

    /// Get user by ID
    async fn get_by_id(&self, id: Uuid) -> Result<Option<User>, DatabaseError>;

    /// Get user by email address
    async fn get_by_email(&self, email: &str) -> Result<Option<User>, DatabaseError>;

    /// Get user by email (required - fails if not found)
    async fn get_by_email_required(&self, email: &str) -> Result<User, DatabaseError>;

    /// Update user's last active timestamp
    async fn update_last_active(&self, id: Uuid) -> Result<(), DatabaseError>;

    /// Get total number of users
    async fn get_count(&self) -> Result<i64, DatabaseError>;

    /// Get users by status (pending, active, suspended)
    async fn list_by_status(&self, status: &str) -> Result<Vec<User>, DatabaseError>;

    /// Get users by status with cursor-based pagination
    async fn list_by_status_paginated(
        &self,
        status: &str,
        pagination: &PaginationParams,
    ) -> Result<CursorPage<User>, DatabaseError>;

    /// Update user status and approval information
    async fn update_status(
        &self,
        id: Uuid,
        new_status: UserStatus,
        admin_token_id: &str,
    ) -> Result<User, DatabaseError>;

    /// Update user's tenant_id to link them to a tenant
    async fn update_tenant_id(&self, id: Uuid, tenant_id: &str) -> Result<(), DatabaseError>;
}
```

### 2. Oauthtokenrepository - OAuth Token Storage (Tenant-scoped)

**Source**: `src/database/repositories/mod.rs:110-160`
```rust
/// OAuth token storage repository (tenant-scoped)
#[async_trait]
pub trait OAuthTokenRepository: Send + Sync {
    /// Store or update user OAuth token for a tenant-provider combination
    async fn upsert(&self, token: &UserOAuthToken) -> Result<(), DatabaseError>;

    /// Get user OAuth token for a specific tenant-provider combination
    async fn get(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> Result<Option<UserOAuthToken>, DatabaseError>;

    /// Get all OAuth tokens for a user across all tenants
    async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<UserOAuthToken>, DatabaseError>;

    /// Get all OAuth tokens for a tenant-provider combination
    async fn list_by_tenant_provider(
        &self,
        tenant_id: &str,
        provider: &str,
    ) -> Result<Vec<UserOAuthToken>, DatabaseError>;

    /// Delete user OAuth token for a tenant-provider combination
    async fn delete(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> Result<(), DatabaseError>;

    /// Delete all OAuth tokens for a user (when user is deleted)
    async fn delete_all_for_user(&self, user_id: Uuid) -> Result<(), DatabaseError>;

    /// Update OAuth token expiration and refresh info
    async fn refresh(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<(), DatabaseError>;
}
```

### 3. Apikeyrepository - API Key Management

**Source**: `src/database/repositories/mod.rs:162-200`
```rust
/// API key management repository
#[async_trait]
pub trait ApiKeyRepository: Send + Sync {
    /// Create a new API key
    async fn create(&self, key: &ApiKey) -> Result<(), DatabaseError>;

    /// Get API key by key hash
    async fn get_by_hash(&self, key_hash: &str) -> Result<Option<ApiKey>, DatabaseError>;

    /// Get API key by ID
    async fn get_by_id(&self, id: &str) -> Result<Option<ApiKey>, DatabaseError>;

    /// List all API keys for a user
    async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<ApiKey>, DatabaseError>;

    /// Revoke an API key
    async fn revoke(&self, id: &str) -> Result<(), DatabaseError>;

    /// Update API key last used timestamp
    async fn update_last_used(&self, id: &str) -> Result<(), DatabaseError>;

    /// Record API key usage
    async fn record_usage(&self, usage: &ApiKeyUsage) -> Result<(), DatabaseError>;

    /// Get usage statistics for an API key
    async fn get_usage_stats(&self, key_id: &str) -> Result<ApiKeyUsageStats, DatabaseError>;
}
```

### 4-13. Other Repository Traits

The remaining repositories follow the same focused pattern:

- **UsageRepository**: JWT usage tracking, API request analytics
- **A2ARepository**: Agent-to-Agent task management, client registration
- **ProfileRepository**: User profiles, fitness goals, activities
- **InsightRepository**: AI-generated insights and recommendations
- **AdminRepository**: Admin token management, authorization
- **TenantRepository**: Multi-tenant management, tenant creation
- **SecurityRepository**: Key rotation, encryption key management
- **NotificationRepository**: OAuth callback notifications
- **OAuth2ServerRepository**: OAuth 2.0 server (client registration, tokens)
- **FitnessConfigRepository**: User fitness configuration storage

**Complete trait definitions**: `src/database/repositories/mod.rs`

## Factory Pattern for Database Selection

Pierre automatically detects and instantiates the correct database backend:

**Source**: `src/database_plugins/factory.rs:38-46`
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

**Source**: `src/database_plugins/factory.rs:164-184`
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
    "sqlite:users.db",  // or "postgresql://localhost/pierre"
    encryption_key,
).await?;
```

## Feature Flags for Database Backends

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

**Source**: `src/database_plugins/factory.rs:44-45`
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

## AAD-Based Encryption for OAuth Tokens

Pierre encrypts OAuth tokens with Additional Authenticated Data (AAD) binding:

**Source**: `src/database_plugins/shared/encryption.rs:12-47`
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

**Source**: `src/database_plugins/shared/encryption.rs:84-96`
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

## Transaction Retry Patterns

Pierre handles database deadlocks and transient errors with exponential backoff:

**Source**: `src/database_plugins/shared/transactions.rs:59-105`
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

**Source**: `src/database_plugins/shared/transactions.rs:120-150`
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
        db.users().create(&user).await
    },
    3 // max retries
).await?;
```

## Shared Database Utilities

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

### Enum Conversions (Enums.rs)

**Source**: `src/database_plugins/shared/enums.rs:24-50`
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

### Generic Row Parsing (Mappers.rs)

**Database-agnostic User parsing**:

**Source**: `src/database_plugins/shared/mappers.rs:37-74`
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

**Source**: `src/database_plugins/shared/mappers.rs:177-192`
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

### Input Validation (Validation.rs)

**Email validation**:

**Source**: `src/database_plugins/shared/validation.rs:34-39`
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

**Source**: `src/database_plugins/shared/validation.rs:63-75`
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

**Source**: `src/database_plugins/shared/validation.rs:104-113`
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

**Source**: `src/database_plugins/shared/validation.rs:140-150`
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

### Hasencryption Trait

Both database backends implement this shared encryption interface:

**Source**: `src/database_plugins/shared/encryption.rs:129-137`
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

## Rust Idioms: Repository Pattern

**Pattern**: Focused, cohesive interfaces
```rust
// Database provides accessors
impl Database {
    pub fn users(&self) -> UserRepositoryImpl { ... }
    pub fn oauth_tokens(&self) -> OAuthTokenRepositoryImpl { ... }
}

// Usage
let user = db.users().get_by_id(user_id).await?;
let token = db.oauth_tokens().get(user_id, tenant_id, provider).await?;
```

**Why this works**:
- **Single Responsibility**: Each repository handles one domain
- **Interface Segregation**: Consumers only depend on what they need
- **Testability**: Can mock individual repositories
- **Clarity**: `db.users().create()` is clearer than `db.create_user()`

## Rust Idioms: Conditional Compilation

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

## Connection Pooling PostgreSQL

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

## Migration System

Pierre uses SQLx migrations for schema management. See [migrations/README.md](../../migrations/README.md) for comprehensive documentation.

**20 migration files covering 40+ tables**:

| Migration | Tables/Changes |
|-----------|----------------|
| `20250120000001_users_schema.sql` | users, user_profiles, user_oauth_app_credentials |
| `20250120000002_api_keys_schema.sql` | api_keys, api_key_usage |
| `20250120000003_analytics_schema.sql` | jwt_usage, goals, insights, request_logs |
| `20250120000004_a2a_schema.sql` | a2a_clients, a2a_sessions, a2a_tasks, a2a_usage |
| `20250120000005_admin_schema.sql` | admin_tokens, admin_token_usage, admin_provisioned_keys, system_secrets, rsa_keypairs |
| `20250120000006_oauth_tokens_schema.sql` | user_oauth_tokens |
| `20250120000007_oauth_notifications_schema.sql` | oauth_notifications |
| `20250120000008_oauth2_schema.sql` | oauth2_clients, oauth2_auth_codes, oauth2_refresh_tokens, oauth2_states |
| `20250120000009_tenant_management_schema.sql` | tenants, tenant_oauth_credentials, oauth_apps, key_versions, audit_events, tenant_users |
| `20250120000010_fitness_configurations_schema.sql` | fitness_configurations |
| `20250120000011_expand_oauth_provider_constraints.sql` | Adds garmin, whoop, terra to provider CHECK constraints |
| `20250120000012_user_roles_permissions.sql` | impersonation_sessions, permission_delegations, user_mcp_tokens; adds role column to users |
| `20250120000013_system_settings_schema.sql` | system_settings |
| `20250120000014_add_missing_foreign_keys.sql` | Adds FK constraints to a2a_clients.user_id, user_configurations.user_id |
| `20250120000015_remove_legacy_user_token_columns.sql` | Removes legacy OAuth columns from users; adds last_sync to user_oauth_tokens |
| `20250120000017_chat_schema.sql` | chat_conversations, chat_messages |
| `20250120000018_firebase_auth.sql` | Adds firebase_uid, auth_provider columns to users |
| `20250120000019_recipes_schema.sql` | recipes, recipe_ingredients |
| `20250120000020_admin_config_schema.sql` | admin_config_overrides, admin_config_audit, admin_config_categories |
| `20250120000021_add_config_categories.sql` | Adds provider, cache, MCP, monitoring categories |

**Example schema** (migrations/20250120000006_oauth_tokens_schema.sql):
```sql
CREATE TABLE IF NOT EXISTS user_oauth_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    access_token TEXT NOT NULL,  -- Encrypted with AAD
    refresh_token TEXT,           -- Encrypted with AAD
    token_type TEXT NOT NULL DEFAULT 'bearer',
    expires_at TEXT,
    scope TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(user_id, tenant_id, provider)
);
```

**Cross-database compatibility**:
- All types use TEXT (portable across SQLite and PostgreSQL)
- Timestamps stored as ISO8601 strings (app-generated)
- UUIDs stored as TEXT (app-generated)
- Booleans stored as INTEGER (0/1)

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

## Multi-Tenant Data Isolation

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
db.oauth_tokens()
    .get(user_id, tenant_id, provider)
    .await?;
```

**AAD encryption binding**: Tenant ID in AAD prevents cross-tenant token copying at encryption layer.

## Key Takeaways

1. **Repository pattern**: 13 focused traits replaced 135-method god-trait (commit `6f3efef`).

2. **Accessor methods**: `db.users()`, `db.oauth_tokens()`, etc. provide clear, focused interfaces.

3. **SOLID principles**: Single Responsibility and Interface Segregation enforced.

4. **Factory pattern**: Automatic database type detection from connection string.

5. **Feature flags**: Conditional compilation for database backends.

6. **AAD encryption**: OAuth tokens encrypted with tenant/user/provider binding via `HasEncryption` trait.

7. **Transaction retry**: Exponential backoff for deadlock/timeout errors (10ms to 160ms).

8. **Shared utilities**: 880 lines across 6 modules eliminated 3,058 lines of wrapper boilerplate:
   - **enums.rs**: UserTier, UserStatus, TaskStatus conversions
   - **mappers.rs**: Generic row parsing with complex trait bounds
   - **validation.rs**: Email, tenant ownership, expiration, scope checks
   - **encryption.rs**: AAD-based encryption utilities
   - **transactions.rs**: Retry patterns with exponential backoff
   - **builders.rs**: Deferred to later phase (minimal implementation)

9. **Connection pooling**: PostgreSQL uses pooling for performance and concurrency.

10. **Migration system**: SQLx migrations for version-controlled schema changes.

11. **Multi-tenant isolation**: Composite keys and AAD binding enforce tenant boundaries.

12. **Instrumentation**: Tracing macros add database operation context to logs.

13. **Error handling**: Clear messages when feature flags missing or URLs invalid.

14. **Code reduction**: Refactoring deleted the entire 3,058-line `sqlite.rs` wrapper file.

---

**Related Chapters**:
- Chapter 2: Error Handling (DatabaseError types)
- Chapter 5: Cryptographic Keys (encryption key management)
- Chapter 7: Multi-Tenant Isolation (application-layer tenant context)
- Chapter 23: Testing Framework (database testing patterns)
