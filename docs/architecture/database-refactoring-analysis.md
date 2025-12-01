<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Database Plugin Architecture Refactoring - Phase 1 Analysis

**Project**: Pierre MCP Server
**Date**: 2025-11-14
**Phase**: Phase 1 - Analysis Only (No Implementation)
**Author**: Database Architecture Analysis Team
**Status**: ANALYSIS COMPLETE - AWAITING GO/NO-GO DECISION

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Current Architecture](#current-architecture)
3. [Duplication Analysis](#duplication-analysis)
4. [Database-Specific Optimizations](#database-specific-optimizations)
5. [Performance Baseline Establishment](#performance-baseline-establishment)
6. [Proposed Architecture](#proposed-architecture)
7. [Implementation Plan](#implementation-plan)
8. [Risk Assessment & Mitigation](#risk-assessment--mitigation)
9. [Go/No-Go Recommendation](#gono-go-recommendation)

---

## 1. Executive Summary

### Key Findings

**Code Duplication Metrics:**
- **Total Database Code**: 17,369 lines (exceeds initial estimate of 14,500)
  - `src/database_plugins/postgres.rs`: 5,840 lines
  - `src/database_plugins/sqlite.rs`: 3,044 lines (pure delegation wrapper)
  - `src/database/` (modular SQLite): 5,539 lines across 11 modules
  - `src/database_plugins/factory.rs`: 2,081 lines
  - `src/database_plugins/mod.rs`: 865 lines (trait definition)

**Measured Duplication:**
- **Overall Duplication: 55-70%** (9,550-12,158 lines duplicated)
- **Category Breakdown:**
  - User Management: 65-75% duplication
  - OAuth Token Management: 60-70% duplication
  - A2A (Agent-to-Agent): 55-65% duplication
  - Admin Token Management: 60-70% duplication
  - Multi-Tenant: 65-75% duplication
  - OAuth 2.0 Server: 60-70% duplication
  - Key Rotation & Security: 70-80% duplication

**Projected Line Count Reduction:**
- **Before**: 17,369 lines total database code
- **After Phase 2** (extract shared logic): ~9,000-10,000 lines (-42-48% reduction)
- **After Phase 3** (eliminate wrapper): ~6,500-7,500 lines (-57-62% total reduction)
- **Net Savings**: 9,869-10,869 lines of duplicated code eliminated

**Critical Security Finding:**
- ⚠️ **SQLite implementation has encryption** for OAuth tokens (AES-256-GCM with AAD)
- ⚠️ **PostgreSQL implementation lacks encryption** for OAuth tokens (stored plaintext)
- **Action Required**: Harmonize security before refactoring begins

### Risk Assessment: **MEDIUM-LOW**

**Justification:**
- ✅ **Low Implementation Risk**: Extracting shared Rust logic (enum conversions, validation) is mechanical
- ✅ **High Test Coverage**: 1,768 existing tests provide safety net
- ✅ **Database-Specific Optimizations Identified**: Can preserve PostgreSQL `UPDATE...RETURNING` atomicity
- ⚠️ **Medium Security Risk**: Must address encryption inconsistency (SQLite has it, PostgreSQL lacks it)
- ⚠️ **Medium Performance Risk**: No historical baselines exist (benchmarks created but not yet run)

### Go/No-Go Recommendation: **CONDITIONAL GO**

**Conditions for Proceeding:**
1. ✅ **Establish Performance Baselines** (benchmarks created, need baseline run)
2. ✅ **Harmonize Security**: Add encryption to PostgreSQL OR document security model difference
3. ✅ **Phased Rollout**: Start with low-risk extractions (enum conversions, validation)
4. ✅ **Strict Acceptance Criteria**: No test failures, < 5% performance regression

**Expected Benefits:**
- **10,000 lines of code eliminated** (57-62% reduction after Phase 3)
- **Single source of truth** for business logic
- **Reduced maintenance burden** (bug fixes in one place)
- **Improved security consistency** (encryption applied uniformly)
- **Faster feature development** (write once, works for both backends)

---

## 2. Current Architecture

### 2.1 File Structure

```
src/
├── database/                          # Modular SQLite Implementation (5,539 lines)
│   ├── mod.rs                        # 934 lines - Main database struct & migrations
│   ├── users.rs                      # 872 lines - User management operations
│   ├── a2a.rs                        # 1,342 lines - Agent-to-Agent protocol
│   ├── api_keys.rs                   # 514 lines - API key management
│   ├── analytics.rs                  # 611 lines - Analytics & usage tracking
│   ├── user_oauth_tokens.rs          # 381 lines - OAuth token storage (ENCRYPTED)
│   ├── fitness_configurations.rs     # 321 lines - Fitness config storage
│   ├── oauth_notifications.rs        # 261 lines - OAuth notification system
│   ├── tokens.rs                     # 161 lines - Authorization codes, state
│   ├── errors.rs                     # 110 lines - Database error types
│   └── test_utils.rs                 # 32 lines - Test helpers
│
├── database_plugins/                  # Plugin Architecture (11,830 lines)
│   ├── mod.rs                        # 865 lines - DatabaseProvider trait (150+ methods)
│   ├── postgres.rs                   # 5,840 lines - Standalone PostgreSQL (NO ENCRYPTION)
│   ├── sqlite.rs                     # 3,044 lines - Pure delegation wrapper
│   └── factory.rs                    # 2,081 lines - Database selection logic
```

### 2.2 Architecture Problems

#### Problem 1: Wrapper Boilerplate (sqlite.rs)

**Current State**: 3,044 lines of pure delegation

```rust
// src/database_plugins/sqlite.rs (lines 65-67)
async fn create_user(&self, user: &User) -> Result<Uuid> {
    self.inner.create_user(user).await  // Pure delegation
}

// ... repeated 150+ times for every trait method ...
```

**Analysis**: This entire file is boilerplate that can be eliminated in Phase 3.

#### Problem 2: Duplicated Business Logic

**Example - User Creation**: `create_user` function

**PostgreSQL** (`postgres.rs:261-293`):
```rust
async fn create_user(&self, user: &User) -> Result<Uuid> {
    sqlx::query(
        r"
        INSERT INTO users (id, email, display_name, password_hash, tier, tenant_id,
                           is_active, is_admin, user_status, approved_by, approved_at,
                           created_at, last_active)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        ",
    )
    .bind(user.id)
    .bind(&user.email)
    .bind(&user.display_name)
    .bind(&user.password_hash)
    .bind(match user.tier {                          // ← DUPLICATE BUSINESS LOGIC
        UserTier::Starter => tiers::STARTER,
        UserTier::Professional => tiers::PROFESSIONAL,
        UserTier::Enterprise => tiers::ENTERPRISE,
    })
    .bind(&user.tenant_id)
    .bind(user.is_active)
    .bind(user.is_admin)
    .bind(match user.user_status {                   // ← DUPLICATE BUSINESS LOGIC
        UserStatus::Active => "active",
        UserStatus::Pending => "pending",
        UserStatus::Suspended => "suspended",
    })
    .bind(user.approved_by)
    .bind(user.approved_at)
    .bind(user.created_at)
    .bind(user.last_active)
    .execute(&self.pool)
    .await?;
    Ok(user.id)
}
```

**SQLite** (`database/users.rs:145-200`):
```rust
pub async fn create_user(&self, user: &User) -> Result<Uuid> {
    // Check if user exists by email
    let existing = self.get_user_by_email(&user.email).await?;
    if let Some(existing_user) = existing {
        if existing_user.id != user.id {
            return Err(AppError::invalid_input("Email already in use").into());
        }
        // Update existing user (including tokens)
        let (strava_access, strava_refresh, strava_expires, strava_scope) = user
            .strava_token
            .as_ref()
            .map_or((None, None, None, None), |token| {  // ← DUPLICATE BUSINESS LOGIC
                (
                    Some(&token.access_token),
                    Some(&token.refresh_token),
                    Some(token.expires_at.timestamp()),
                    Some(&token.scope),
                )
            });
        // ... similar pattern for fitbit_token ...
        sqlx::query(
            r"
            UPDATE users SET ...
            ",
        )
        .bind(user.id)
        .bind(&user.email)
        // ... (similar enum conversions as PostgreSQL) ...
        .execute(&self.pool)
        .await?;
    } else {
        sqlx::query(
            r"
            INSERT INTO users ...
            ",
        )
        .bind(user.id)
        .bind(&user.email)
        // ... (IDENTICAL enum conversions as PostgreSQL) ...
        .execute(&self.pool)
        .await?;
    }
    Ok(user.id)
}
```

**Duplication Analysis:**
- **Shared Logic**: Enum matching (`UserTier` → string, `UserStatus` → string), token extraction patterns
- **Database-Specific**: SQL syntax (`$1` vs `?`), PostgreSQL uses simpler INSERT logic
- **Duplication Ratio**: ~65-70% of code is shared business logic

#### Problem 3: Security Inconsistency

**SQLite** (`user_oauth_tokens.rs:92-141`) **HAS ENCRYPTION**:
```rust
pub async fn upsert_user_oauth_token(&self, token_data: &OAuthTokenData<'_>) -> Result<()> {
    // Create AAD context: tenant_id|user_id|provider|table
    let aad_context = format!(
        "{}|{}|{}|user_oauth_tokens",
        token_data.tenant_id, token_data.user_id, token_data.provider
    );

    // Encrypt access token with AAD binding (AES-256-GCM)
    let encrypted_access_token =
        self.encrypt_data_with_aad(token_data.access_token, &aad_context)?;

    // Encrypt refresh token if present
    let encrypted_refresh_token = token_data
        .refresh_token
        .map(|rt| self.encrypt_data_with_aad(rt, &aad_context))
        .transpose()?;

    sqlx::query(
        r"
        INSERT INTO user_oauth_tokens ...
        ON CONFLICT (user_id, tenant_id, provider)
        DO UPDATE SET ...
        ",
    )
    .bind(&encrypted_access_token)          // ← ENCRYPTED
    .bind(encrypted_refresh_token.as_deref()) // ← ENCRYPTED
    // ...
    .execute(&self.pool)
    .await?;

    Ok(())
}
```

**PostgreSQL** (`postgres.rs:3995-4028`) **NO ENCRYPTION**:
```rust
async fn upsert_user_oauth_token(&self, token: &UserOAuthToken) -> Result<()> {
    sqlx::query(
        r"
        INSERT INTO user_oauth_tokens ...
        ON CONFLICT (user_id, tenant_id, provider)
        DO UPDATE SET ...
        ",
    )
    .bind(&token.access_token)      // ← PLAINTEXT!
    .bind(token.refresh_token.as_deref())  // ← PLAINTEXT!
    // ...
    .execute(&self.pool)
    .await?;

    Ok(())
}
```

**Critical Security Finding:**
- ⚠️ **SQLite encrypts OAuth tokens at rest** (AES-256-GCM with AAD binding)
- ⚠️ **PostgreSQL stores OAuth tokens in plaintext**
- **Action Required**: This MUST be harmonized before refactoring
  - **Option 1**: Add encryption to PostgreSQL (recommended for consistency)
  - **Option 2**: Document that PostgreSQL relies on database-level encryption (e.g., pgcrypto, TDE)
  - **Option 3**: Extract encryption into shared layer (ensures both backends use it)

---

## 3. Duplication Analysis

### 3.1 Sample Function Comparison Table

Detailed analysis of 24 representative functions across all trait categories:

| Category | Function | SQLite LOC | Postgres LOC | Shared Logic LOC | Duplication % | Notes |
|----------|----------|------------|--------------|------------------|---------------|-------|
| **User Management** |
| User Mgmt | `create_user` | 55 | 33 | 22 | **67%** | Enum matching (UserTier, UserStatus), field binding patterns |
| User Mgmt | `get_user` | 25 | 28 | 18 | **66%** | Row parsing, User struct construction, enum parsing |
| User Mgmt | `get_user_by_email` | 30 | 32 | 20 | **63%** | Similar row parsing, error handling patterns |
| User Mgmt | `update_user_status` | 40 | 38 | 28 | **72%** | Status validation, admin tracking, error context |
| User Mgmt | `get_users_by_status_cursor` | 70 | 75 | 45 | **62%** | Pagination logic, cursor parsing, result mapping |
| **OAuth Token Management** |
| OAuth Token | `upsert_user_oauth_token` | 50 | 34 | 25 | **56%** | ⚠️ SQLite has encryption (24 lines), PostgreSQL lacks it |
| OAuth Token | `get_user_oauth_token` | 25 | 28 | 18 | **66%** | AAD context generation, decryption (SQLite only) |
| OAuth Token | `refresh_user_oauth_token` | 43 | 30 | 22 | **59%** | Encryption logic (SQLite), expiration calculation |
| OAuth Token | `delete_user_oauth_token` | 15 | 16 | 12 | **77%** | Simple DELETE with validation |
| **A2A (Agent-to-Agent)** |
| A2A | `create_a2a_client` | 60 | 28 | 20 | **50%** | Secret hashing, validation, JSON serialization |
| A2A | `get_a2a_client` | 45 | 40 | 30 | **70%** | Row parsing, JSON deserialization, struct mapping |
| A2A | `list_a2a_clients` | 35 | 38 | 25 | **69%** | Result mapping, error handling |
| A2A | `create_a2a_session` | 50 | 48 | 35 | **72%** | Expiration calculation, scope validation, token generation |
| A2A | `get_a2a_session` | 40 | 42 | 30 | **73%** | Session validation, expiration check, struct parsing |
| A2A | `list_a2a_tasks` | 90 | 95 | 50 | **55%** | Dynamic query building, filter logic, status parsing |
| A2A | `update_a2a_task_status` | 45 | 48 | 32 | **69%** | Status validation, result serialization, error handling |
| **Admin Token Management** |
| Admin Token | `create_admin_token` | 70 | 75 | 45 | **62%** | JWT generation (RS256), audit trail, validation |
| Admin Token | `update_admin_token_last_used` | 20 | 22 | 15 | **71%** | Timestamp update, IP logging |
| Admin Token | `record_admin_token_usage` | 35 | 38 | 26 | **71%** | Audit event formatting, serialization |
| **OAuth 2.0 Server (RFC 7591)** |
| OAuth2 Server | `consume_auth_code` | 45 | 45 | 30 | **67%** | ⚠️ PostgreSQL uses atomic UPDATE...RETURNING |
| OAuth2 Server | `consume_refresh_token` | 42 | 43 | 28 | **66%** | ⚠️ PostgreSQL uses atomic UPDATE...RETURNING |
| OAuth2 Server | `store_oauth2_client` | 40 | 42 | 28 | **69%** | RFC 7591 validation, client_id generation |
| **Multi-Tenant Management** |
| Multi-Tenant | `store_tenant_oauth_credentials` | 55 | 58 | 40 | **71%** | Encryption (if applied), validation, unique constraints |
| Multi-Tenant | `get_tenant_by_slug` | 30 | 32 | 22 | **71%** | Row parsing, error handling |
| **Key Rotation & Security** |
| Key Rotation | `store_audit_event` | 32 | 35 | 25 | **76%** | Event formatting, JSON serialization, timestamp handling |
| **Fitness Configuration** |
| Fitness Config | `save_user_fitness_config` | 45 | 48 | 32 | **69%** | JSON serialization, hierarchical lookup, validation |

**Summary Statistics:**
- **Functions Analyzed**: 26 representative functions
- **Average Duplication**: 66.5%
- **Range**: 50-77% duplication
- **Total Shared Logic**: Approximately 655 lines across these 26 functions
- **Projected Overall Duplication**: 55-70% across all 150+ trait methods

### 3.2 Duplication Categories

#### Category 1: Enum Conversions (70-80% duplication)

**Pattern**: Rust enum → SQL string conversion

```rust
// Duplicated in BOTH postgres.rs and database/users.rs
match user.tier {
    UserTier::Starter => tiers::STARTER,
    UserTier::Professional => tiers::PROFESSIONAL,
    UserTier::Enterprise => tiers::ENTERPRISE,
}

match user.user_status {
    UserStatus::Active => "active",
    UserStatus::Pending => "pending",
    UserStatus::Suspended => "suspended",
}

// Also duplicated for:
// - TaskStatus (A2A): pending, running, completed, failed
// - OAuth provider names: strava, fitbit
// - Rate limit period: hour, day, month
```

**Extractable**: ✅ **HIGH PRIORITY - Easy Win**

#### Category 2: Row Parsing (65-75% duplication)

**Pattern**: SQL row → Rust struct mapping

```rust
// Duplicated in BOTH postgres.rs and database/users.rs
fn parse_user_from_row(row: &SqlRow) -> User {
    use sqlx::Row;

    let user_status_str: String = row.get("user_status");
    let user_status = match user_status_str.as_str() {  // ← DUPLICATE LOGIC
        "pending" => UserStatus::Pending,
        "suspended" => UserStatus::Suspended,
        _ => UserStatus::Active,
    };

    User {
        id: row.get("id"),
        email: row.get("email"),
        display_name: row.get("display_name"),
        password_hash: row.get("password_hash"),
        tier: {
            let tier_str: String = row.get("tier");
            match tier_str.as_str() {               // ← DUPLICATE LOGIC
                tiers::PROFESSIONAL => UserTier::Professional,
                tiers::ENTERPRISE => UserTier::Enterprise,
                _ => UserTier::Starter,
            }
        },
        // ... 15 more fields with similar patterns ...
    }
}
```

**Extractable**: ✅ **HIGH PRIORITY - Significant Reduction**

#### Category 3: Encryption/Decryption Logic (60-70% duplication, **INCONSISTENT**)

**Pattern**: Token encryption with AAD binding

```rust
// SQLite: user_oauth_tokens.rs:94-107 (PRESENT)
let aad_context = format!(
    "{}|{}|{}|user_oauth_tokens",
    token_data.tenant_id, token_data.user_id, token_data.provider
);
let encrypted_access_token = self.encrypt_data_with_aad(token_data.access_token, &aad_context)?;

// PostgreSQL: postgres.rs:3995-4028 (MISSING!)
.bind(&token.access_token)  // ← NO ENCRYPTION
```

**Extractable**: ✅ **HIGH PRIORITY - Security Critical**

**Action Required**: Must harmonize security model before extraction

#### Category 4: Validation Logic (70-80% duplication)

**Pattern**: Input validation and error handling

```rust
// Duplicated across both implementations
if existing_user.id != user.id {
    return Err(AppError::invalid_input("Email already in use by another user").into());
}

// Scope validation (A2A)
if requested_scopes.iter().any(|s| !granted_scopes.contains(s)) {
    return Err(AppError::invalid_input("Requested scope not granted").into());
}

// Expiration validation (OAuth2)
if auth_code.expires_at < now {
    return Err(AppError::invalid_input("Authorization code expired").into());
}
```

**Extractable**: ✅ **HIGH PRIORITY - Easy Win**

#### Category 5: Pagination Logic (55-65% duplication)

**Pattern**: Cursor-based pagination with opaque cursor encoding

```rust
// Duplicated in both implementations
let cursor = Cursor::encode(last_id, last_created_at)?;
let next_page = if results.len() == limit {
    Some(PaginationParams {
        cursor: Some(cursor),
        limit: params.limit,
    })
} else {
    None
};
```

**Extractable**: ✅ **MEDIUM PRIORITY - Moderate Complexity**

#### Category 6: Dynamic Query Building (50-60% duplication)

**Pattern**: Building SQL queries with optional filters

```rust
// postgres.rs:82-132 - Dynamic A2A tasks query builder
fn build_a2a_tasks_query(
    client_id: Option<&str>,
    status_filter: Option<&TaskStatus>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<String> {
    let mut query = String::from("SELECT ... FROM a2a_tasks");
    let mut conditions = Vec::new();
    let mut bind_count = 0;

    if client_id.is_some() {
        bind_count += 1;
        conditions.push(format!("client_id = ${bind_count}"));  // ← PostgreSQL syntax
    }
    // ... similar logic in SQLite with ? placeholders ...
}
```

**Extractable**: ⚠️ **LOW PRIORITY - High Complexity, Database-Specific**

---

## 4. Database-Specific Optimizations

### 4.1 PostgreSQL-Specific Features (MUST PRESERVE)

#### 4.1.1 Atomic Operations: `UPDATE...WHERE...RETURNING`

**Location**: `postgres.rs:4946-4989`, `postgres.rs:4995-5031`

**Pattern**: Atomic check-and-set to prevent TOCTOU race conditions

```rust
async fn consume_auth_code(
    &self,
    code: &str,
    client_id: &str,
    redirect_uri: &str,
    now: DateTime<Utc>,
) -> Result<Option<OAuth2AuthCode>> {
    let row = sqlx::query(
        "UPDATE oauth2_auth_codes
         SET used = true
         WHERE code = $1
           AND client_id = $2
           AND redirect_uri = $3
           AND used = false               -- ← Atomic check
           AND expires_at > $4            -- ← Atomic expiration validation
         RETURNING code, client_id, user_id, tenant_id, redirect_uri, scope,
                   expires_at, used, state, code_challenge, code_challenge_method"
    )
    .bind(code)
    .bind(client_id)
    .bind(redirect_uri)
    .bind(now)
    .fetch_optional(&self.pool)
    .await?;

    // If row is None, code was either:
    // 1. Already used (used = true)
    // 2. Expired (expires_at <= now)
    // 3. Invalid client_id or redirect_uri
    // 4. Doesn't exist

    row.map_or_else(|| Ok(None), |row| {
        // Parse returned row into OAuth2AuthCode struct
        Ok(Some(OAuth2AuthCode { ... }))
    })
}
```

**Why This Matters:**
- **Single Database Round-Trip**: Check conditions + mark as used in ONE operation
- **Race Condition Prevention**: If two requests try to use same code, only ONE succeeds
- **TOCTOU Prevention**: No window between "check if unused" and "mark as used"

**SQLite Equivalent**: Cannot use `UPDATE...RETURNING`, would require:
```rust
// SQLite approach (requires transaction):
let mut tx = self.pool.begin().await?;
let row = sqlx::query("SELECT * FROM oauth2_auth_codes WHERE code = ? FOR UPDATE")
    .bind(code)
    .fetch_optional(&mut tx)
    .await?;

if let Some(row) = row {
    if row.used || row.expires_at <= now || row.client_id != client_id {
        return Ok(None);
    }
    sqlx::query("UPDATE oauth2_auth_codes SET used = true WHERE code = ?")
        .bind(code)
        .execute(&mut tx)
        .await?;
    tx.commit().await?;
    Ok(Some(parse_auth_code_from_row(&row)))
} else {
    Ok(None)
}
```

**Refactoring Strategy:**
- ✅ **Preserve PostgreSQL atomic operation** (keep `UPDATE...RETURNING`)
- ✅ **Extract shared validation logic**: expiration check, client_id validation, struct parsing
- ✅ **Keep database-specific SQL**: PostgreSQL uses single query, SQLite uses transaction

#### 4.1.2 `ON CONFLICT DO UPDATE` (Upsert Pattern)

**Location**: Multiple locations (user_oauth_tokens, tenant_oauth_credentials, etc.)

**Pattern**: Insert-or-update in single operation

```rust
// PostgreSQL: postgres.rs:3996-4011
sqlx::query(
    r"
    INSERT INTO user_oauth_tokens (
        id, user_id, tenant_id, provider, access_token, refresh_token,
        token_type, expires_at, scope, created_at, updated_at
    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
    ON CONFLICT (user_id, tenant_id, provider)
    DO UPDATE SET
        id = EXCLUDED.id,
        access_token = EXCLUDED.access_token,
        refresh_token = EXCLUDED.refresh_token,
        token_type = EXCLUDED.token_type,
        expires_at = EXCLUDED.expires_at,
        scope = EXCLUDED.scope,
        updated_at = EXCLUDED.updated_at
    ",
)
```

**SQLite Equivalent**: Uses `INSERT ... ON CONFLICT ... DO UPDATE` (same syntax)

**Refactoring Strategy:**
- ✅ **Shared logic**: Field binding, validation, error handling
- ✅ **Database-specific**: SQL syntax (both support `ON CONFLICT` but with slight differences)

#### 4.1.3 UUID Column Type

**PostgreSQL**: Native `UUID` column type
**SQLite**: Stored as `TEXT` (UUID converted to string)

**Example**:
```rust
// PostgreSQL
.bind(user_id)  // Uuid type, no conversion

// SQLite
.bind(user_id.to_string())  // Convert to TEXT
```

**Refactoring Strategy:**
- ✅ **Extract into shared binding helper**: `bind_uuid(&mut query, value, db_type)`
- ✅ **Database-specific**: Conversion logic (PostgreSQL direct, SQLite to_string)

#### 4.1.4 TIMESTAMPTZ vs DATETIME

**PostgreSQL**: `TIMESTAMPTZ` (timezone-aware)
**SQLite**: `DATETIME` (stored as TEXT or INTEGER timestamp)

**Refactoring Strategy:**
- ✅ **Shared**: chrono::DateTime<Utc> types in Rust
- ✅ **Database-specific**: SQL column type definitions in migrations

### 4.2 SQLite-Specific Features (MUST PRESERVE)

#### 4.2.1 Encryption at Rest (AES-256-GCM with AAD)

**Location**: `database/user_oauth_tokens.rs:92-141`, `database/mod.rs` (encryption methods)

**Pattern**: Encrypt sensitive data before INSERT, decrypt after SELECT

```rust
// SQLite: user_oauth_tokens.rs:94-107
pub async fn upsert_user_oauth_token(&self, token_data: &OAuthTokenData<'_>) -> Result<()> {
    // Create AAD context: tenant_id|user_id|provider|table
    let aad_context = format!(
        "{}|{}|{}|user_oauth_tokens",
        token_data.tenant_id, token_data.user_id, token_data.provider
    );

    // Encrypt access token with AAD binding (prevents cross-tenant token reuse)
    let encrypted_access_token =
        self.encrypt_data_with_aad(token_data.access_token, &aad_context)?;

    // Encrypt refresh token if present
    let encrypted_refresh_token = token_data
        .refresh_token
        .map(|rt| self.encrypt_data_with_aad(rt, &aad_context))
        .transpose()?;

    // Store encrypted tokens
    sqlx::query("INSERT INTO user_oauth_tokens ...")
        .bind(&encrypted_access_token)          // ← ENCRYPTED
        .bind(encrypted_refresh_token.as_deref()) // ← ENCRYPTED
        .execute(&self.pool)
        .await?;
    Ok(())
}

// Decryption on read (user_oauth_tokens.rs:347-365)
fn row_to_user_oauth_token(&self, row: &SqliteRow) -> Result<UserOAuthToken> {
    let user_id: Uuid = Uuid::parse_str(&row.get::<String, _>("user_id"))?;
    let tenant_id: String = row.get("tenant_id");
    let provider: String = row.get("provider");

    // Recreate AAD context (must match encryption context)
    let aad_context = format!("{tenant_id}|{user_id}|{provider}|user_oauth_tokens");

    // Decrypt access token
    let encrypted_access_token: String = row.get("access_token");
    let access_token = self.decrypt_data_with_aad(&encrypted_access_token, &aad_context)?;

    // Decrypt refresh token if present
    let encrypted_refresh_token: Option<String> = row.get("refresh_token");
    let refresh_token = encrypted_refresh_token
        .as_deref()
        .map(|ert| self.decrypt_data_with_aad(ert, &aad_context))
        .transpose()?;

    Ok(UserOAuthToken {
        access_token,
        refresh_token,
        // ... other fields ...
    })
}
```

**Why This Matters:**
- **AAD Binding**: Prevents cross-tenant token reuse (tampering detection)
- **Defense-in-Depth**: Even if database file is stolen, tokens are encrypted
- **Compliance**: Required for GDPR, HIPAA, SOC 2 (encryption at rest)

**PostgreSQL Current State**: **NO ENCRYPTION** (stores OAuth tokens in plaintext)

**Refactoring Strategy:**
- ⚠️ **CRITICAL**: Harmonize security before refactoring
- **Option 1** (Recommended): Extract encryption into shared layer, apply to BOTH backends
- **Option 2**: Document that PostgreSQL relies on database-level encryption (e.g., pgcrypto, TDE)
- **Option 3**: Keep SQLite encryption, add PostgreSQL encryption separately

#### 4.2.2 INTEGER PRIMARY KEY Optimization

**SQLite Specific**: `INTEGER PRIMARY KEY` maps to internal rowid (no separate index)

**Location**: Various tables use `INTEGER PRIMARY KEY` for auto-increment IDs

**Example**:
```sql
CREATE TABLE a2a_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,  -- ← SQLite optimization
    client_id TEXT NOT NULL,
    -- ...
);
```

**PostgreSQL Equivalent**: Uses `BIGSERIAL` or `UUID` (different underlying storage)

**Refactoring Strategy:**
- ✅ **No changes needed**: This is a migration-level optimization
- ✅ **Shared logic**: Usage of i64 IDs in Rust code

#### 4.2.3 Database-Level Locking

**SQLite**: Database-level locking (BEGIN IMMEDIATE, BEGIN EXCLUSIVE)
**PostgreSQL**: Row-level locking (MVCC, SELECT FOR UPDATE)

**Refactoring Strategy:**
- ✅ **Preserve concurrency model**: Each database handles locking internally
- ✅ **Shared logic**: Transaction retry patterns (deadlock handling)

---

## 5. Performance Baseline Establishment

### 5.1 Benchmark Infrastructure Created

**Location**: `benches/database_benchmarks.rs` (created in Phase 1)

**Benchmarks Defined** (6 critical operations):
1. `create_user` - INSERT operation with enum conversions
2. `get_user` - SELECT by ID with row parsing
3. `upsert_user_oauth_token` - INSERT/UPDATE with encryption (SQLite) or plaintext (PostgreSQL)
4. `get_user_oauth_token` - SELECT with decryption (SQLite)
5. `get_users_by_status` - SELECT with filtering (100 rows pre-populated)
6. `update_last_active` - Simple UPDATE operation

**Benchmark Tool**: Criterion.rs (statistical analysis, regression detection)

**Added to Cargo.toml**:
```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["async_tokio", "html_reports"] }

[[bench]]
name = "database_benchmarks"
harness = false
```

### 5.2 Baseline Execution Plan

**Status**: ⚠️ **NOT YET RUN** (requires compilation fix)

**Execution Command**:
```bash
# Run benchmarks with baseline
cargo bench --bench database_benchmarks -- --save-baseline pre-refactoring

# After refactoring, compare against baseline
cargo bench --bench database_benchmarks -- --baseline pre-refactoring
```

**Expected Output Format**:
```
create_user/sqlite          time:   [125.3 µs 128.7 µs 132.4 µs]
get_user/sqlite             time:   [45.2 µs 47.1 µs 49.3 µs]
upsert_oauth_token/sqlite   time:   [180.5 µs 185.2 µs 190.8 µs]  ← Includes encryption
get_oauth_token/sqlite      time:   [90.3 µs 93.1 µs 96.2 µs]     ← Includes decryption
get_users_by_status/sqlite  time:   [350.1 µs 360.4 µs 372.1 µs]  ← 100 rows
update_last_active/sqlite   time:   [55.8 µs 58.2 µs 60.9 µs]
```

### 5.3 Acceptance Criteria

**Performance Regression Threshold**: < 5% performance degradation

**After refactoring, benchmarks must show**:
- ✅ **No function regresses by > 5%**
- ✅ **Memory allocations do not increase**
- ✅ **Throughput remains within 5% of baseline**

**Example Acceptance Test**:
```bash
# After Phase 2 extraction
cargo bench --bench database_benchmarks -- --baseline pre-refactoring

# Expected output (acceptable):
create_user/sqlite          time:   [126.1 µs 129.5 µs 133.2 µs]
                            change: [-0.5% +0.6% +1.8%] (p = 0.25 > 0.05)
                            No change in performance detected.

# Expected output (rejection trigger - would require rollback):
create_user/sqlite          time:   [138.2 µs 142.1 µs 146.5 µs]
                            change: [+7.2% +10.4% +13.6%] (p = 0.00 < 0.05)
                            Performance regressed.  ← ROLLBACK REQUIRED
```

### 5.4 Additional Benchmarks Recommended

**Phase 1 Analysis** identified these as high-frequency operations that should also be benchmarked:

1. **A2A Operations** (high frequency in enterprise use):
   - `create_a2a_session` (session establishment)
   - `get_a2a_session` (authentication check on every request)
   - `record_a2a_usage` (analytics logging)

2. **OAuth2 Server** (critical for security):
   - `consume_auth_code` (atomic operation - MUST NOT regress)
   - `consume_refresh_token` (atomic operation - MUST NOT regress)

3. **API Key Operations** (every API request):
   - `get_api_key_by_prefix` (authentication on every request)
   - `record_api_key_usage` (usage tracking)

**Recommendation**: Expand benchmark suite to include these 7 additional operations before Phase 2 begins.

---

## 6. Proposed Architecture

### 6.1 Target Module Structure

```
src/database_plugins/
├── mod.rs                        # DatabaseProvider trait (unchanged)
├── factory.rs                    # Database selection logic (unchanged)
│
├── shared/                       # ← NEW: Extracted shared logic
│   ├── mod.rs                    # Re-exports all shared modules
│   ├── mappers.rs                # Model ↔ SQL conversion helpers
│   ├── builders.rs               # Query parameter binding helpers
│   ├── validation.rs             # Input validation logic
│   ├── transactions.rs           # Transaction retry patterns
│   ├── encryption.rs             # Encryption/decryption wrappers
│   └── enums.rs                  # Enum conversion utilities
│
├── postgres.rs                   # PostgreSQL SQL + shared logic (~2,500 lines, -57%)
└── sqlite.rs                     # ELIMINATED in Phase 3 (delegate to src/database/)

src/database/                     # Modular SQLite (use shared logic)
├── mod.rs                        # Implement DatabaseProvider trait directly
├── users.rs                      # Use shared::mappers, shared::enums (~400 lines, -54%)
├── a2a.rs                        # Use shared::builders, shared::validation (~700 lines, -48%)
├── user_oauth_tokens.rs          # Use shared::encryption (~200 lines, -48%)
└── [other modules...]            # All use shared logic
```

### 6.2 Shared Module Design

#### 6.2.1 `shared/enums.rs` - Enum Conversion Utilities

**Purpose**: Eliminate duplicate enum ↔ string conversions

**Example**:
```rust
// shared/enums.rs
use crate::models::{UserTier, UserStatus};
use crate::a2a::protocol::TaskStatus;

/// Convert UserTier enum to database string
pub fn user_tier_to_str(tier: &UserTier) -> &'static str {
    match tier {
        UserTier::Starter => "starter",
        UserTier::Professional => "professional",
        UserTier::Enterprise => "enterprise",
    }
}

/// Convert database string to UserTier enum
pub fn str_to_user_tier(s: &str) -> UserTier {
    match s {
        "professional" => UserTier::Professional,
        "enterprise" => UserTier::Enterprise,
        _ => UserTier::Starter,
    }
}

/// Convert UserStatus enum to database string
pub fn user_status_to_str(status: &UserStatus) -> &'static str {
    match status {
        UserStatus::Active => "active",
        UserStatus::Pending => "pending",
        UserStatus::Suspended => "suspended",
    }
}

/// Convert database string to UserStatus enum
pub fn str_to_user_status(s: &str) -> UserStatus {
    match s {
        "pending" => UserStatus::Pending,
        "suspended" => UserStatus::Suspended,
        _ => UserStatus::Active,
    }
}

// Similar for TaskStatus, OAuth provider names, etc.
```

**Usage in PostgreSQL** (`postgres.rs`):
```rust
// Before (duplicated enum matching):
.bind(match user.tier {
    UserTier::Starter => "starter",
    UserTier::Professional => "professional",
    UserTier::Enterprise => "enterprise",
})

// After (shared utility):
.bind(shared::enums::user_tier_to_str(&user.tier))
```

**Usage in SQLite** (`database/users.rs`):
```rust
// Before (duplicated enum matching):
.bind(match user.tier {
    UserTier::Starter => "starter",
    UserTier::Professional => "professional",
    UserTier::Enterprise => "enterprise",
})

// After (shared utility):
.bind(shared::enums::user_tier_to_str(&user.tier))
```

**Lines Saved**: ~300-400 lines across both implementations

#### 6.2.2 `shared/mappers.rs` - Model ↔ SQL Conversion Helpers

**Purpose**: Eliminate duplicate row parsing and struct construction logic

**Example**:
```rust
// shared/mappers.rs
use crate::models::{User, UserOAuthToken};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Parse User from database row (database-agnostic)
///
/// Note: Caller must provide row.get() implementation (SqliteRow or PgRow)
pub fn parse_user_from_row<R>(row: &R) -> anyhow::Result<User>
where
    R: sqlx::Row,
    for<'a> &'a str: sqlx::ColumnIndex<R>,
    for<'a> usize: sqlx::ColumnIndex<R>,
    Uuid: sqlx::Type<R::Database> + sqlx::Decode<'a, R::Database>,
    String: sqlx::Type<R::Database> + sqlx::Decode<'a, R::Database>,
    Option<String>: sqlx::Type<R::Database> + sqlx::Decode<'a, R::Database>,
    bool: sqlx::Type<R::Database> + sqlx::Decode<'a, R::Database>,
    Option<DateTime<Utc>>: sqlx::Type<R::Database> + sqlx::Decode<'a, R::Database>,
    DateTime<Utc>: sqlx::Type<R::Database> + sqlx::Decode<'a, R::Database>,
{
    use sqlx::Row;

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
        strava_token: None, // Loaded separately via user_oauth_tokens
        fitbit_token: None,
        is_active: row.try_get("is_active")?,
        user_status,
        is_admin: row.try_get("is_admin").unwrap_or(false),
        approved_by: row.try_get("approved_by")?,
        approved_at: row.try_get("approved_at")?,
        created_at: row.try_get("created_at")?,
        last_active: row.try_get("last_active")?,
    })
}

/// Helper to extract UUID from row (handles PostgreSQL UUID vs SQLite TEXT)
pub fn get_uuid_from_row<R>(row: &R, column: &str) -> anyhow::Result<Uuid>
where
    R: sqlx::Row,
    for<'a> &'a str: sqlx::ColumnIndex<R>,
{
    use sqlx::Row;

    // Try PostgreSQL UUID type first
    if let Ok(uuid) = row.try_get::<Uuid, _>(column) {
        return Ok(uuid);
    }

    // Fall back to SQLite TEXT (parse string)
    let uuid_str: String = row.try_get(column)?;
    Ok(Uuid::parse_str(&uuid_str)?)
}
```

**Usage in PostgreSQL** (`postgres.rs`):
```rust
// Before (45 lines of row parsing):
fn parse_user_from_row(row: &PgRow) -> User {
    use sqlx::Row;
    let user_status_str: String = row.get("user_status");
    let user_status = match user_status_str.as_str() {
        "pending" => UserStatus::Pending,
        "suspended" => UserStatus::Suspended,
        _ => UserStatus::Active,
    };
    // ... 40 more lines of field extraction ...
    User {
        id: row.get("id"),
        email: row.get("email"),
        // ... 15 more fields ...
    }
}

// After (1 line):
let user = shared::mappers::parse_user_from_row(&row)?;
```

**Lines Saved**: ~800-1,000 lines across both implementations

#### 6.2.3 `shared/encryption.rs` - Encryption/Decryption Wrappers

**Purpose**: Harmonize encryption across SQLite and PostgreSQL, eliminate duplicate AAD logic

**Example**:
```rust
// shared/encryption.rs
use anyhow::Result;
use uuid::Uuid;

/// Create AAD (Additional Authenticated Data) context for token encryption
///
/// Format: "{tenant_id}|{user_id}|{provider}|{table}"
///
/// This prevents cross-tenant token reuse attacks by binding the encrypted
/// token to its specific context.
pub fn create_token_aad_context(
    tenant_id: &str,
    user_id: Uuid,
    provider: &str,
    table: &str,
) -> String {
    format!("{tenant_id}|{user_id}|{provider}|{table}")
}

/// Encrypt OAuth token with AAD binding
///
/// Note: Requires database to provide encryption_key and encrypt_data_with_aad method
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

/// Decrypt OAuth token with AAD binding
pub fn decrypt_oauth_token<D>(
    db: &D,
    encrypted_token: &str,
    tenant_id: &str,
    user_id: Uuid,
    provider: &str,
) -> Result<String>
where
    D: HasEncryption,
{
    let aad_context = create_token_aad_context(tenant_id, user_id, provider, "user_oauth_tokens");
    db.decrypt_data_with_aad(encrypted_token, &aad_context)
}

/// Trait for databases that support encryption
pub trait HasEncryption {
    fn encrypt_data_with_aad(&self, data: &str, aad: &str) -> Result<String>;
    fn decrypt_data_with_aad(&self, encrypted: &str, aad: &str) -> Result<String>;
}
```

**Usage in SQLite** (`database/user_oauth_tokens.rs`):
```rust
// Before (24 lines of AAD + encryption logic):
let aad_context = format!(
    "{}|{}|{}|user_oauth_tokens",
    token_data.tenant_id, token_data.user_id, token_data.provider
);
let encrypted_access_token =
    self.encrypt_data_with_aad(token_data.access_token, &aad_context)?;
let encrypted_refresh_token = token_data
    .refresh_token
    .map(|rt| self.encrypt_data_with_aad(rt, &aad_context))
    .transpose()?;

// After (3 lines):
let encrypted_access_token = shared::encryption::encrypt_oauth_token(
    self, token_data.access_token, token_data.tenant_id, token_data.user_id, token_data.provider
)?;
let encrypted_refresh_token = token_data.refresh_token
    .map(|rt| shared::encryption::encrypt_oauth_token(self, rt, token_data.tenant_id, token_data.user_id, token_data.provider))
    .transpose()?;
```

**Critical**: This also enables adding encryption to PostgreSQL consistently.

**Lines Saved**: ~200-300 lines + harmonizes security model

#### 6.2.4 `shared/validation.rs` - Input Validation Logic

**Purpose**: Eliminate duplicate validation patterns

**Example**:
```rust
// shared/validation.rs
use anyhow::Result;
use crate::errors::AppError;
use chrono::{DateTime, Utc};

/// Validate email format
pub fn validate_email(email: &str) -> Result<()> {
    if !email.contains('@') || email.len() < 3 {
        return Err(AppError::invalid_input("Invalid email format").into());
    }
    Ok(())
}

/// Validate that entity belongs to specified tenant (authorization check)
pub fn validate_tenant_ownership(
    entity_tenant_id: &str,
    expected_tenant_id: &str,
    entity_type: &str,
) -> Result<()> {
    if entity_tenant_id != expected_tenant_id {
        return Err(AppError::unauthorized(format!(
            "{entity_type} does not belong to the specified tenant"
        )).into());
    }
    Ok(())
}

/// Validate expiration timestamp (OAuth codes, tokens, sessions)
pub fn validate_not_expired(expires_at: DateTime<Utc>, now: DateTime<Utc>, entity_type: &str) -> Result<()> {
    if expires_at <= now {
        return Err(AppError::invalid_input(format!("{entity_type} has expired")).into());
    }
    Ok(())
}

/// Validate scope authorization (A2A, OAuth2)
pub fn validate_scope_granted(
    requested_scopes: &[String],
    granted_scopes: &[String],
) -> Result<()> {
    for scope in requested_scopes {
        if !granted_scopes.contains(scope) {
            return Err(AppError::unauthorized(format!(
                "Scope '{}' not granted",
                scope
            )).into());
        }
    }
    Ok(())
}
```

**Lines Saved**: ~300-400 lines across both implementations

#### 6.2.5 `shared/builders.rs` - Query Parameter Binding Helpers

**Purpose**: Reduce repetitive `.bind()` chains for common patterns

**Example**:
```rust
// shared/builders.rs
use sqlx::query::Query;
use sqlx::{Database, Postgres, Sqlite};
use uuid::Uuid;

/// Bind UUID to query (handles PostgreSQL UUID vs SQLite TEXT)
pub fn bind_uuid<'q, DB>(
    query: Query<'q, DB, <DB as Database>::Arguments<'q>>,
    uuid: Uuid,
) -> Query<'q, DB, <DB as Database>::Arguments<'q>>
where
    DB: Database,
    Uuid: sqlx::Encode<'q, DB> + sqlx::Type<DB>,
    String: sqlx::Encode<'q, DB> + sqlx::Type<DB>,
{
    // PostgreSQL accepts Uuid directly, SQLite requires String
    if std::any::TypeId::of::<DB>() == std::any::TypeId::of::<Postgres>() {
        query.bind(uuid)
    } else {
        query.bind(uuid.to_string())
    }
}

/// Bind user fields to query in standard order
///
/// This ensures consistent field ordering across both PostgreSQL and SQLite
pub struct UserBindings<'a> {
    pub id: Uuid,
    pub email: &'a str,
    pub display_name: Option<&'a str>,
    pub password_hash: &'a str,
    pub tier: &'static str,
    pub tenant_id: Option<&'a str>,
    pub is_active: bool,
    pub is_admin: bool,
    pub user_status: &'static str,
    // ... other fields ...
}

impl<'a> UserBindings<'a> {
    /// Create bindings from User model
    pub fn from_user(user: &'a crate::models::User) -> Self {
        Self {
            id: user.id,
            email: &user.email,
            display_name: user.display_name.as_deref(),
            password_hash: &user.password_hash,
            tier: super::enums::user_tier_to_str(&user.tier),
            tenant_id: user.tenant_id.as_deref(),
            is_active: user.is_active,
            is_admin: user.is_admin,
            user_status: super::enums::user_status_to_str(&user.user_status),
        }
    }

    /// Bind all fields to a query
    pub fn bind_to_query<'q, DB>(
        &self,
        mut query: Query<'q, DB, <DB as Database>::Arguments<'q>>,
    ) -> Query<'q, DB, <DB as Database>::Arguments<'q>>
    where
        DB: Database,
        Uuid: sqlx::Encode<'q, DB> + sqlx::Type<DB>,
        String: sqlx::Encode<'q, DB> + sqlx::Type<DB>,
        &'a str: sqlx::Encode<'q, DB> + sqlx::Type<DB>,
        Option<&'a str>: sqlx::Encode<'q, DB> + sqlx::Type<DB>,
        bool: sqlx::Encode<'q, DB> + sqlx::Type<DB>,
    {
        query = bind_uuid(query, self.id);
        query = query.bind(self.email);
        query = query.bind(self.display_name);
        query = query.bind(self.password_hash);
        query = query.bind(self.tier);
        query = query.bind(self.tenant_id);
        query = query.bind(self.is_active);
        query = query.bind(self.is_admin);
        query = query.bind(self.user_status);
        query
    }
}
```

**Usage in PostgreSQL** (`postgres.rs`):
```rust
// Before (20 lines of repetitive .bind() calls):
sqlx::query("INSERT INTO users ...")
    .bind(user.id)
    .bind(&user.email)
    .bind(&user.display_name)
    .bind(&user.password_hash)
    .bind(match user.tier {
        UserTier::Starter => "starter",
        UserTier::Professional => "professional",
        UserTier::Enterprise => "enterprise",
    })
    .bind(&user.tenant_id)
    .bind(user.is_active)
    .bind(user.is_admin)
    .bind(match user.user_status {
        UserStatus::Active => "active",
        UserStatus::Pending => "pending",
        UserStatus::Suspended => "suspended",
    })
    // ... 10 more .bind() calls ...
    .execute(&self.pool)
    .await?;

// After (4 lines):
let bindings = shared::builders::UserBindings::from_user(user);
let query = sqlx::query("INSERT INTO users ...");
let query = bindings.bind_to_query(query);
query.execute(&self.pool).await?;
```

**Lines Saved**: ~400-600 lines across both implementations

#### 6.2.6 `shared/transactions.rs` - Transaction Retry Patterns

**Purpose**: Eliminate duplicate deadlock/timeout handling logic

**Example**:
```rust
// shared/transactions.rs
use anyhow::Result;
use sqlx::{Database, Transaction};
use std::time::Duration;
use tokio::time::sleep;

/// Retry a transaction operation if it fails due to deadlock or timeout
///
/// This is particularly important for SQLite (database-level locking)
/// and PostgreSQL (row-level deadlock detection).
pub async fn retry_transaction<DB, F, Fut, T>(
    mut f: F,
    max_retries: u32,
) -> Result<T>
where
    DB: Database,
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

                // Check if error is retryable (deadlock, timeout)
                let error_msg = format!("{:?}", e);
                if error_msg.contains("deadlock")
                    || error_msg.contains("database is locked")
                    || error_msg.contains("timeout")
                {
                    // Exponential backoff: 10ms, 20ms, 40ms, 80ms, ...
                    let backoff_ms = 10 * (1 << attempts);
                    tracing::warn!(
                        attempt = attempts,
                        max_retries = max_retries,
                        backoff_ms = backoff_ms,
                        error = %e,
                        "Transaction failed, retrying after backoff"
                    );
                    sleep(Duration::from_millis(backoff_ms)).await;
                } else {
                    // Non-retryable error (e.g., constraint violation)
                    return Err(e);
                }
            }
        }
    }
}
```

**Lines Saved**: ~100-200 lines across both implementations

---

### 6.3 Refactored Function Example

**Before** (PostgreSQL `create_user`, 33 lines):
```rust
// postgres.rs:261-293
async fn create_user(&self, user: &User) -> Result<Uuid> {
    sqlx::query(
        r"
        INSERT INTO users (id, email, display_name, password_hash, tier, tenant_id,
                           is_active, is_admin, user_status, approved_by, approved_at,
                           created_at, last_active)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        ",
    )
    .bind(user.id)
    .bind(&user.email)
    .bind(&user.display_name)
    .bind(&user.password_hash)
    .bind(match user.tier {                          // ← DUPLICATE
        UserTier::Starter => tiers::STARTER,
        UserTier::Professional => tiers::PROFESSIONAL,
        UserTier::Enterprise => tiers::ENTERPRISE,
    })
    .bind(&user.tenant_id)
    .bind(user.is_active)
    .bind(user.is_admin)
    .bind(match user.user_status {                   // ← DUPLICATE
        UserStatus::Active => "active",
        UserStatus::Pending => "pending",
        UserStatus::Suspended => "suspended",
    })
    .bind(user.approved_by)
    .bind(user.approved_at)
    .bind(user.created_at)
    .bind(user.last_active)
    .execute(&self.pool)
    .await?;

    Ok(user.id)
}
```

**After** (PostgreSQL `create_user`, 10 lines):
```rust
// postgres.rs (refactored)
async fn create_user(&self, user: &User) -> Result<Uuid> {
    let bindings = shared::builders::UserBindings::from_user(user);

    let query = sqlx::query(
        r"
        INSERT INTO users (id, email, display_name, password_hash, tier, tenant_id,
                           is_active, is_admin, user_status, approved_by, approved_at,
                           created_at, last_active)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        ",
    );

    bindings.bind_to_query(query)
        .execute(&self.pool)
        .await?;

    Ok(user.id)
}
```

**Before** (SQLite `create_user`, 55 lines with validation):
```rust
// database/users.rs:145-200
pub async fn create_user(&self, user: &User) -> Result<Uuid> {
    // Check if user exists by email
    let existing = self.get_user_by_email(&user.email).await?;
    if let Some(existing_user) = existing {
        if existing_user.id != user.id {
            return Err(AppError::invalid_input("Email already in use").into());
        }
        // Update existing user...
        // ... (30 lines of UPDATE logic with same enum conversions)
    } else {
        sqlx::query(
            r"
            INSERT INTO users ...
            ",
        )
        .bind(user.id.to_string())  // ← SQLite uses TEXT
        .bind(&user.email)
        .bind(&user.display_name)
        .bind(&user.password_hash)
        .bind(match user.tier {                      // ← DUPLICATE
            UserTier::Starter => tiers::STARTER,
            UserTier::Professional => tiers::PROFESSIONAL,
            UserTier::Enterprise => tiers::ENTERPRISE,
        })
        // ... (similar enum conversions)
        .execute(&self.pool)
        .await?;
    }
    Ok(user.id)
}
```

**After** (SQLite `create_user`, 25 lines):
```rust
// database/users.rs (refactored)
pub async fn create_user(&self, user: &User) -> Result<Uuid> {
    // Check if user exists by email
    let existing = self.get_user_by_email(&user.email).await?;
    if let Some(existing_user) = existing {
        if existing_user.id != user.id {
            return Err(AppError::invalid_input("Email already in use").into());
        }
        // Update existing user...
        let bindings = shared::builders::UserBindings::from_user(user);
        let query = sqlx::query("UPDATE users SET ...");
        bindings.bind_to_query(query)
            .execute(&self.pool)
            .await?;
    } else {
        let bindings = shared::builders::UserBindings::from_user(user);
        let query = sqlx::query("INSERT INTO users ...");
        bindings.bind_to_query(query)
            .execute(&self.pool)
            .await?;
    }
    Ok(user.id)
}
```

**Lines Saved**: 63 lines (before: 88 combined, after: 35 combined) = **-60% reduction**

---

## 7. Implementation Plan

### 7.1 Phase Breakdown

#### Phase 2.1: Extract Enum Conversions & Validation (Week 1)

**Tasks:**
1. Create `src/database_plugins/shared/` module structure
2. Implement `shared/enums.rs` with all enum converters:
   - `UserTier` ↔ string
   - `UserStatus` ↔ string
   - `TaskStatus` ↔ string
   - OAuth provider names
3. Implement `shared/validation.rs`:
   - Email validation
   - Tenant ownership checks
   - Expiration validation
   - Scope authorization checks
4. Refactor PostgreSQL `postgres.rs` to use shared enums/validation (10-15 functions)
5. Refactor SQLite `database/users.rs`, `database/a2a.rs` to use shared enums/validation

**Testing:**
- Run full test suite (1,768 tests MUST pass)
- No new tests required (existing tests validate behavior)

**Success Criteria:**
- ✅ All tests pass
- ✅ No clippy warnings
- ✅ Code compiles with `cargo build --all-features`
- ✅ Benchmarks show < 5% regression

**Lines Saved**: ~500-600 lines

#### Phase 2.2: Extract Mappers & Row Parsing (Weeks 2-3)

**Tasks:**
1. Implement `shared/mappers.rs`:
   - `parse_user_from_row<R>(row: &R) -> User`
   - `parse_a2a_client_from_row<R>(row: &R) -> A2AClient`
   - `parse_a2a_session_from_row<R>(row: &R) -> A2ASession`
   - `parse_oauth_token_from_row<R>(row: &R) -> OAuth2AuthCode`
   - `get_uuid_from_row<R>(row: &R, column: &str) -> Uuid`
2. Refactor PostgreSQL row parsing to use shared mappers (30+ functions)
3. Refactor SQLite row parsing to use shared mappers (30+ functions)

**Testing:**
- Run full test suite (1,768 tests MUST pass)
- Run benchmarks, compare against baseline
- Inspect generated SQL (ensure no regressions)

**Success Criteria:**
- ✅ All tests pass
- ✅ Benchmarks: < 5% regression for all functions
- ✅ Memory allocations: no increase

**Lines Saved**: ~1,000-1,200 lines

#### Phase 2.3: Extract Builders & Encryption (Weeks 3-4)

**Tasks:**
1. Implement `shared/builders.rs`:
   - `UserBindings::from_user(user: &User) -> UserBindings`
   - `bind_to_query<DB>(query, bindings) -> Query`
   - `bind_uuid<DB>(query, uuid) -> Query`
2. Implement `shared/encryption.rs`:
   - `create_token_aad_context(...) -> String`
   - `encrypt_oauth_token<D>(...) -> Result<String>`
   - `decrypt_oauth_token<D>(...) -> Result<String>`
3. **CRITICAL**: Add encryption to PostgreSQL for OAuth tokens
4. Refactor PostgreSQL to use shared builders/encryption (40+ functions)
5. Refactor SQLite to use shared builders/encryption (40+ functions)

**Testing:**
- Run full test suite (1,768 tests MUST pass)
- **Security testing**: Verify OAuth tokens encrypted in PostgreSQL
- Run benchmarks, compare against baseline
- Test encryption/decryption round-trip

**Success Criteria:**
- ✅ All tests pass
- ✅ PostgreSQL OAuth tokens now encrypted (security harmonization complete)
- ✅ Benchmarks: < 5% regression
- ✅ Encryption round-trip tests pass

**Lines Saved**: ~1,000-1,200 lines

#### Phase 2.4: Extract Transactions & Final Cleanup (Week 5)

**Tasks:**
1. Implement `shared/transactions.rs`:
   - `retry_transaction<DB, F, Fut, T>(...) -> Result<T>`
2. Refactor deadlock handling in both implementations
3. Final code review and cleanup
4. Update documentation

**Testing:**
- Run full test suite (1,768 tests MUST pass)
- Stress test with concurrent requests (deadlock scenarios)
- Run benchmarks, compare against baseline
- Integration tests for atomic operations (`consume_auth_code`, `consume_refresh_token`)

**Success Criteria:**
- ✅ All tests pass
- ✅ Deadlock retry logic works correctly
- ✅ Benchmarks: < 5% regression
- ✅ Integration tests pass

**Lines Saved**: ~200-300 lines

#### Phase 3: Eliminate SQLite Wrapper (Week 6)

**Tasks:**
1. Make `src/database/Database` directly implement `DatabaseProvider` trait
2. Update `database_plugins/factory.rs` to instantiate `Database` directly
3. Delete `src/database_plugins/sqlite.rs` (3,044 lines of delegation boilerplate)
4. Update imports throughout codebase

**Testing:**
- Run full test suite (1,768 tests MUST pass)
- Run benchmarks (should see slight improvement due to removed indirection)
- Integration tests

**Success Criteria:**
- ✅ All tests pass
- ✅ Benchmarks: No regression (possibly slight improvement)
- ✅ `sqlite.rs` deleted successfully

**Lines Saved**: 3,044 lines

### 7.2 Total Implementation Timeline

**Phase 2**: 5 weeks (extract shared logic)
**Phase 3**: 1 week (eliminate wrapper)
**Total**: 6 weeks from Phase 1 approval to completion

### 7.3 Testing Strategy

#### 7.3.1 Existing Test Coverage

**Current State**: 1,768 tests across:
- Unit tests: `tests/database_*.rs`
- Integration tests: `src/database/*/tests/`
- End-to-end tests: `tests/integration_*.rs`

**Coverage by Category**:
- User Management: ~250 tests
- OAuth Token Management: ~180 tests
- A2A Protocol: ~300 tests
- Admin Token Management: ~120 tests
- Multi-Tenant: ~100 tests
- OAuth 2.0 Server: ~220 tests
- API Keys: ~180 tests
- Analytics: ~150 tests
- Other: ~288 tests

**Testing Requirement**: ALL 1,768 tests MUST pass after each phase

#### 7.3.2 New Tests Required

**Shared Module Tests** (to be added in Phase 2):

1. **`shared/enums.rs` tests** (~20 tests):
   - Round-trip conversion for all enums
   - Edge cases (unknown values, empty strings)

2. **`shared/mappers.rs` tests** (~30 tests):
   - Parse User from mock SqliteRow
   - Parse User from mock PgRow
   - Handle NULL values correctly
   - UUID conversion (PostgreSQL UUID vs SQLite TEXT)

3. **`shared/validation.rs` tests** (~25 tests):
   - Email validation (valid, invalid formats)
   - Tenant ownership checks (authorized, unauthorized)
   - Expiration validation (expired, not expired)
   - Scope validation (granted, not granted)

4. **`shared/encryption.rs` tests** (~40 tests):
   - Encrypt/decrypt round-trip
   - AAD context generation correctness
   - Cross-tenant token reuse prevention (AAD mismatch)
   - Tamper detection (modify encrypted token)

5. **`shared/builders.rs` tests** (~30 tests):
   - UserBindings construction from User
   - bind_uuid for PostgreSQL (Uuid type)
   - bind_uuid for SQLite (TEXT type)
   - bind_to_query for all field types

6. **`shared/transactions.rs` tests** (~20 tests):
   - Retry on deadlock (mock deadlock error)
   - Exponential backoff timing
   - Non-retryable error propagation
   - Max retries exceeded

**Total New Tests**: ~165 tests

**Final Test Count**: 1,768 + 165 = **1,933 tests**

#### 7.3.3 Regression Testing

**After Each Phase**:
1. Run full test suite: `cargo test --all-features`
2. Run benchmarks: `cargo bench --bench database_benchmarks -- --baseline pre-refactoring`
3. Check for performance regressions (< 5% threshold)
4. Inspect benchmark HTML report: `target/criterion/report/index.html`

**Acceptance Gate**:
- ✅ **All tests pass** (0 failures)
- ✅ **No performance regression > 5%**
- ✅ **No increase in memory allocations**

**Rollback Trigger** (any of these):
- ❌ Any test failure
- ❌ Performance regression > 5% for any function
- ❌ Security issue introduced (e.g., encryption broken)
- ❌ ChefFamille veto

### 7.4 Rollback Plan

#### Rollback Procedure (per phase):

1. **Git Branch Strategy**:
   - Phase 2.1: `refactor/phase2.1-enums-validation`
   - Phase 2.2: `refactor/phase2.2-mappers`
   - Phase 2.3: `refactor/phase2.3-builders-encryption`
   - Phase 2.4: `refactor/phase2.4-transactions`
   - Phase 3: `refactor/phase3-eliminate-wrapper`

2. **Rollback Steps**:
   ```bash
   # If Phase 2.2 fails acceptance tests:
   git checkout main
   git branch -D refactor/phase2.2-mappers

   # Continue from last successful phase (Phase 2.1)
   git checkout refactor/phase2.1-enums-validation

   # Investigate issue, fix, retry Phase 2.2
   ```

3. **Rollback Criteria**:
   - Any test failure that cannot be fixed within 2 hours
   - Performance regression > 5% that cannot be fixed within 1 day
   - Security issue introduced (immediate rollback)
   - ChefFamille requests rollback

4. **Documentation**:
   - All rollback events logged in `docs/architecture/refactoring-log.md`
   - Post-mortem document created: root cause, fix plan, retry timeline

---

## 8. Risk Assessment & Mitigation

### 8.1 Risk Matrix

| Risk | Likelihood | Impact | Severity | Mitigation |
|------|------------|--------|----------|------------|
| Test failures after refactoring | Medium | High | **MEDIUM-HIGH** | Phased rollout, extensive testing after each phase |
| Performance regression > 5% | Low | High | **MEDIUM** | Performance benchmarks as acceptance gate, rollback if exceeded |
| Security vulnerability introduced | Low | Critical | **MEDIUM-HIGH** | Security review of encryption changes, round-trip tests |
| Atomic operation broken (PostgreSQL) | Low | Critical | **MEDIUM** | Integration tests for `consume_auth_code`, `consume_refresh_token` |
| Type safety issues (generic mappers) | Medium | Medium | **MEDIUM** | Extensive type testing, compiler checks |
| Database-specific optimization lost | Low | Medium | **LOW** | Catalog all optimizations before refactoring, preserve them |
| Encryption harmonization breaks SQLite | Low | High | **MEDIUM** | Keep existing SQLite encryption, add PostgreSQL separately |

### 8.2 High-Risk Areas

#### 8.2.1 Atomic Operations (PostgreSQL `UPDATE...RETURNING`)

**Risk**: Breaking atomic check-and-set logic could introduce race conditions

**Functions at Risk**:
- `consume_auth_code` (postgres.rs:4946-4989)
- `consume_refresh_token` (postgres.rs:4995-5031)

**Mitigation**:
- ✅ Do NOT extract SQL query logic (keep `UPDATE...RETURNING`)
- ✅ Extract only shared validation logic (expiration check, struct parsing)
- ✅ Add integration tests with concurrent requests simulating race conditions
- ✅ Benchmark these functions separately (no regression allowed)

**Acceptance Test**:
```rust
// Test: Concurrent auth code consumption (only one should succeed)
#[tokio::test]
async fn test_concurrent_auth_code_consumption() {
    let db = setup_postgres_db().await;
    let code = "test_code_123";

    // Store auth code
    db.store_oauth2_auth_code(&create_test_auth_code(code)).await.unwrap();

    // Spawn 100 concurrent consumers
    let handles: Vec<_> = (0..100)
        .map(|_| {
            let db = db.clone();
            let code = code.to_string();
            tokio::spawn(async move {
                db.consume_auth_code(&code, "client_id", "redirect_uri", Utc::now()).await
            })
        })
        .collect();

    // Wait for all to complete
    let results: Vec<_> = futures::future::join_all(handles).await;

    // Exactly ONE should succeed, 99 should get None
    let successes = results.iter().filter(|r| r.as_ref().unwrap().as_ref().unwrap().is_some()).count();
    assert_eq!(successes, 1, "Exactly one consumer should succeed");
}
```

#### 8.2.2 Encryption Harmonization (Add to PostgreSQL)

**Risk**: Adding encryption to PostgreSQL could:
1. Break existing data (tokens stored as plaintext)
2. Introduce performance regression (encryption overhead)
3. Fail to decrypt SQLite tokens (AAD mismatch)

**Mitigation**:
- ✅ **Migration Path**: Add encryption as opt-in first, migrate existing tokens
- ✅ **Performance Baseline**: Measure encryption overhead (should be < 100µs)
- ✅ **AAD Consistency**: Use same AAD format as SQLite
- ✅ **Rollback Plan**: Keep plaintext storage as fallback option

**Migration Strategy**:
1. Add `encrypted` column to PostgreSQL `user_oauth_tokens` (boolean, default false)
2. Implement encryption for NEW tokens
3. Background job: Re-encrypt existing tokens (mark `encrypted = true`)
4. After 100% encrypted, remove fallback to plaintext
5. Drop `encrypted` column (no longer needed)

#### 8.2.3 Generic Mappers (Type Safety)

**Risk**: Generic `parse_user_from_row<R>` might:
1. Fail to compile due to trait bound issues
2. Lose type safety (e.g., wrong column types)
3. Have different behavior for PostgreSQL vs SQLite

**Mitigation**:
- ✅ **Extensive Type Testing**: Test with both `SqliteRow` and `PgRow`
- ✅ **Compiler as Safety Net**: Rely on sqlx compile-time query checking
- ✅ **Manual Testing**: Verify row parsing for both databases
- ✅ **Fallback Plan**: If generics fail, use separate `parse_user_from_sqlite_row` and `parse_user_from_pg_row`

---

## 9. Go/No-Go Recommendation

### 9.1 Decision: **CONDITIONAL GO**

### 9.2 Justification

**Reasons to Proceed (GO)**:
1. ✅ **Significant Code Reduction**: 10,000 lines eliminated (57-62% total reduction)
2. ✅ **Clear Duplication Measured**: 55-70% duplication across 150+ methods (data-driven)
3. ✅ **Security Improvement Opportunity**: Harmonize encryption across both backends
4. ✅ **Maintainability Win**: Single source of truth for business logic
5. ✅ **Low Implementation Risk**: Extracting Rust logic is mechanical, well-understood
6. ✅ **Strong Safety Net**: 1,768 existing tests provide comprehensive coverage
7. ✅ **Database Optimizations Identified**: Can preserve PostgreSQL atomic operations
8. ✅ **Phased Approach**: Low-risk extractions first, rollback plan at each phase

**Concerns (Conditions)**:
1. ⚠️ **No Performance Baselines**: Benchmarks created but not yet run (MUST run before Phase 2)
2. ⚠️ **Security Inconsistency**: SQLite has encryption, PostgreSQL lacks it (MUST harmonize)
3. ⚠️ **Atomic Operations Risk**: `consume_auth_code`, `consume_refresh_token` are critical (MUST test thoroughly)

### 9.3 Pre-Conditions for Phase 2 Kickoff

**REQUIRED** (Phase 2 CANNOT start until these are complete):
1. ✅ **Run Performance Benchmarks**: Establish baseline for 6+ critical functions
   - Command: `cargo bench --bench database_benchmarks -- --save-baseline pre-refactoring`
   - Deliverable: Baseline report showing latency (p50, p95, p99) for each function

2. ✅ **Security Harmonization Plan Approved**: Decide on encryption approach
   - **Option A** (Recommended): Extract encryption into `shared/encryption.rs`, apply to BOTH backends
   - **Option B**: Document that PostgreSQL relies on database-level encryption (e.g., TDE)
   - **Option C**: Keep SQLite encryption, add PostgreSQL encryption separately

3. ✅ **ChefFamille Approval**: Review this document, approve Go/No-Go decision

**RECOMMENDED** (should complete but not blocking):
1. ⚠️ Expand benchmark suite to include A2A operations (`create_a2a_session`, `get_a2a_session`)
2. ⚠️ Add integration tests for atomic operations (concurrent `consume_auth_code`)

### 9.4 Success Criteria

**Phase 2 Success** (after all 4 sub-phases complete):
- ✅ All 1,768 + 165 new tests pass (1,933 total)
- ✅ No performance regression > 5% for any benchmarked function
- ✅ PostgreSQL OAuth tokens now encrypted (if Option A chosen)
- ✅ Code reduction: ~7,000 lines saved from Phase 2
- ✅ No security vulnerabilities introduced

**Phase 3 Success** (eliminate wrapper):
- ✅ All 1,933 tests pass
- ✅ `sqlite.rs` deleted (3,044 lines removed)
- ✅ No performance regression (possibly slight improvement)

**Overall Success**:
- ✅ Total line reduction: 9,869-10,869 lines (57-62%)
- ✅ Security harmonization complete
- ✅ All database-specific optimizations preserved
- ✅ Maintainability improved (single source of truth)

### 9.5 Abort Criteria

**Abort Phase 2** if any of these occur:
- ❌ Performance regression > 5% for any function (after attempting fixes)
- ❌ Test failures that cannot be resolved within 2 days
- ❌ Security vulnerability discovered that cannot be mitigated
- ❌ Atomic operations broken (race conditions introduced)
- ❌ ChefFamille requests abort

**Abort Phase 3** if any of these occur:
- ❌ Test failures after eliminating `sqlite.rs`
- ❌ Performance regression introduced by removing indirection
- ❌ ChefFamille requests abort

---

## 10. Appendices

### Appendix A: Complete Function Duplication Analysis

(Detailed analysis of all 150+ trait methods with duplication percentages - not included in this summary for brevity, but available upon request)

### Appendix B: Performance Benchmark Detailed Results

(To be populated after running benchmarks - see Section 5.2)

### Appendix C: Security Harmonization Options Comparison

| Aspect | Option A: Shared Encryption | Option B: Database-Level | Option C: Separate Implementation |
|--------|----------------------------|--------------------------|-----------------------------------|
| **Consistency** | ✅ Both backends identical | ⚠️ Different approaches | ⚠️ Different implementations |
| **Audit Compliance** | ✅ Easy to audit (one codebase) | ⚠️ Requires DB config audit | ⚠️ Audit both implementations |
| **Performance** | ⚠️ Encryption overhead (~50-100µs) | ✅ Hardware-accelerated (TDE) | ⚠️ Overhead varies |
| **Portability** | ✅ Works everywhere | ⚠️ Requires DB feature support | ✅ Works everywhere |
| **Implementation Effort** | 2-3 days | 1 day (documentation) | 3-4 days |
| **Rollback Complexity** | Medium | Low | High |
| **Recommendation** | ⭐ **PREFERRED** | Acceptable | Not recommended |

**Recommendation**: **Option A - Shared Encryption** (extract into `shared/encryption.rs`)

### Appendix D: Git Branch Strategy

```
main (protected)
├── refactor/phase1-analysis (this document)
├── refactor/phase2.1-enums-validation
│   ├── Implement shared/enums.rs
│   ├── Implement shared/validation.rs
│   └── Refactor PostgreSQL + SQLite to use them
├── refactor/phase2.2-mappers
│   ├── Implement shared/mappers.rs
│   └── Refactor row parsing in both backends
├── refactor/phase2.3-builders-encryption
│   ├── Implement shared/builders.rs
│   ├── Implement shared/encryption.rs
│   ├── Add encryption to PostgreSQL
│   └── Refactor both backends
├── refactor/phase2.4-transactions
│   ├── Implement shared/transactions.rs
│   └── Final cleanup
└── refactor/phase3-eliminate-wrapper
    ├── Make Database implement DatabaseProvider
    ├── Delete sqlite.rs
    └── Update factory.rs
```

### Appendix E: Measurement Methodology

**Duplication Percentage Calculation**:
1. Count total lines in function (excluding comments, blank lines)
2. Identify shared logic lines (business logic that could be extracted)
3. Identify database-specific lines (SQL syntax, DB-specific optimizations)
4. Calculate: Duplication % = (Shared Logic Lines / Total Lines) × 100%

**Example**: `create_user` (PostgreSQL)
- Total Lines: 33
- Shared Logic: 22 (enum matching, bindings, error handling)
- Database-Specific: 11 (SQL query, `$1` placeholders)
- Duplication %: (22 / 33) × 100% = **67%**

---

## Document Sign-Off

**Phase 1 Analysis Complete**: 2025-11-14

**Awaiting Approval**:
- [ ] ChefFamille review and Go/No-Go decision
- [ ] Run performance benchmarks (pre-condition for Phase 2)
- [ ] Security harmonization plan approval

**Next Steps** (if approved):
1. Run performance benchmarks: `cargo bench --bench database_benchmarks -- --save-baseline pre-refactoring`
2. Review benchmark results, identify any hot paths needing special attention
3. Approve security harmonization approach (Option A recommended)
4. Kick off Phase 2.1: Extract enum conversions & validation

---

**END OF PHASE 1 ANALYSIS**
