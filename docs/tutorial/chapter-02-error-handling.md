<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 2: Error Handling & Type-Safe Errors

> **Learning Objectives**: Master structured error handling in Rust using `thiserror`, understand why Pierre eliminates `anyhow!()` from production code, and learn error propagation patterns.
>
> **Prerequisites**: Chapter 1, basic understanding of `Result<T, E>` and `Option<T>`
>
> **Estimated Time**: 3-4 hours

---

## Introduction

Error handling is one of Rust's greatest strengths. Unlike languages with exceptions, Rust uses the type system to enforce error handling at compile time. Pierre takes this further with a **zero-tolerance policy** on ad-hoc errors.

**CLAUDE.md Directive** (critical):
> **Never use `anyhow::anyhow!()` in production code**
>
> Use structured error types exclusively: `AppError`, `DatabaseError`, `ProviderError`

This chapter teaches you why this matters and how Pierre implements production-grade error handling.

---

## The Problem with Anyhow

The `anyhow` crate is popular for quick prototyping, but has serious issues in production code.

### Anyhow Example (Anti-pattern)

```rust
// DON'T DO THIS - Loses type information
use anyhow::anyhow;

fn fetch_user(id: &str) -> anyhow::Result<User> {
    if id.is_empty() {
        return Err(anyhow!("User ID cannot be empty"));  // ❌ Type-erased error
    }

    let user = database.get(id)
        .ok_or_else(|| anyhow!("User not found"))?;  // ❌ No structure

    Ok(user)
}
```

**Problems**:
1. **Type erasure**: All errors become `anyhow::Error` (opaque box)
2. **No pattern matching**: Can't handle different error types differently
3. **No programmatic access**: Error details are just strings
4. **Poor API**: Callers can't know what errors to expect
5. **No HTTP mapping**: How do you convert "User not found" to status code?

### Structured Error Example (Correct)

**Source**: `src/database/errors.rs:10-19`

```rust
// DO THIS - Type-safe, structured errors
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Entity not found: {entity_type} with id '{entity_id}'")]
    NotFound {
        entity_type: &'static str,
        entity_id: String,
    },
    // ... more variants
}

fn fetch_user(id: &str) -> Result<User, DatabaseError> {
    if id.is_empty() {
        return Err(DatabaseError::NotFound {
            entity_type: "user",
            entity_id: String::new(),
        });
    }

    // Callers can pattern match on this specific error
    database.get(id)
        .ok_or_else(|| DatabaseError::NotFound {
            entity_type: "user",
            entity_id: id.to_string(),
        })
}
```

**Benefits**:
1. ✅ **Type safety**: Errors are concrete types
2. ✅ **Pattern matching**: Can handle `NotFound` vs `ConnectionError` differently
3. ✅ **Programmatic access**: Extract `entity_id` from error
4. ✅ **Clear API**: Callers know what to expect
5. ✅ **HTTP mapping**: Easy to convert to status codes

---

## Pierre's Error Hierarchy

Pierre uses a three-tier error hierarchy:

```
AppError (src/errors.rs)              ← HTTP-level errors
    ↓ wraps
├── DatabaseError (src/database/errors.rs)     ← Database operations
├── ProviderError (src/providers/errors.rs)    ← External API calls
└── ProtocolError (src/protocols/...)          ← Protocol-specific errors
```

**Design principle**: Errors are defined close to their domain, then converted to `AppError` at API boundaries.

---

## Thiserror: Derive Macro for Errors

The `thiserror` crate provides a derive macro that auto-implements `std::error::Error` and `Display`.

### Basic Thiserror Usage

**Source**: `src/database/errors.rs:10-46`

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    /// Entity not found in database
    #[error("Entity not found: {entity_type} with id '{entity_id}'")]
    NotFound {
        entity_type: &'static str,
        entity_id: String,
    },

    /// Cross-tenant access attempt detected
    #[error("Tenant isolation violation: attempted to access {entity_type} '{entity_id}' from tenant '{requested_tenant}' but it belongs to tenant '{actual_tenant}'")]
    TenantIsolationViolation {
        entity_type: &'static str,
        entity_id: String,
        requested_tenant: String,
        actual_tenant: String,
    },

    /// Encryption operation failed
    #[error("Encryption failed: {context}")]
    EncryptionFailed {
        context: String,
    },

    /// Decryption operation failed
    #[error("Decryption failed: {context}")]
    DecryptionFailed {
        context: String,
    },

    /// Database constraint violation
    #[error("Constraint violation: {constraint} - {details}")]
    ConstraintViolation {
        constraint: String,
        details: String,
    },
}
```

**Rust Idioms Explained**:

1. **`#[derive(Error, Debug)]`**
   - `Error`: thiserror's derive macro
   - `Debug`: Required by `std::error::Error` trait
   - Auto-implements `Display` using `#[error(...)]` attributes

2. **`#[error("...")]` format strings**
   - Defines the `Display` implementation
   - Use `{field_name}` to interpolate struct fields
   - Same syntax as `format!()` macro

3. **Enum variants with fields**
   - Struct-like variants: `NotFound { entity_type, entity_id }`
   - Tuple variants: `ConnectionError(String)`
   - Unit variants: `Timeout` (no fields)

4. **Documentation comments** `///`
   - Document each variant's purpose
   - Appears in IDE tooltips and `cargo doc`

**Generated code** (what thiserror creates):

```rust
// thiserror automatically generates this:
impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::NotFound { entity_type, entity_id } => {
                write!(f, "Entity not found: {} with id '{}'", entity_type, entity_id)
            }
            // ... other variants
        }
    }
}

impl std::error::Error for DatabaseError {}
```

**Reference**: [thiserror documentation](https://docs.rs/thiserror/)

---

## Error Variant Design Patterns

Pierre uses several patterns for error variants.

### Pattern 1: Struct-Like Variants with Context

**Source**: `src/providers/errors.rs:13-23`

```rust
#[derive(Error, Debug)]
pub enum ProviderError {
    /// Provider API is unavailable or returning errors
    #[error("Provider {provider} API error: {status_code} - {message}")]
    ApiError {
        provider: String,
        status_code: u16,
        message: String,
        retryable: bool,  // ← Extra context for retry logic
    },
}
```

**Use when**: You need multiple pieces of context (who, what, why)

**Pattern matching**:

```rust
match error {
    ProviderError::ApiError { status_code: 429, provider, retry_after_secs, .. } => {
        println!("Rate limited by {}, retry in {} seconds", provider, retry_after_secs);
    }
    ProviderError::ApiError { status_code, .. } if status_code >= 500 => {
        println!("Server error, retry with backoff");
    }
    _ => println!("Non-retryable error"),
}
```

### Pattern 2: Tuple Variants for Simple Errors

**Source**: `src/database/errors.rs:57-59`

```rust
#[derive(Error, Debug)]
pub enum DatabaseError {
    /// Database connection error
    #[error("Database connection error: {0}")]
    ConnectionError(String),

    // More examples:

    /// Database query error
    #[error("Query execution error: {context}")]
    QueryError { context: String },
}
```

**Use when**: Single piece of context is sufficient

**Creating**:

```rust
return Err(DatabaseError::ConnectionError(
    "Failed to connect to postgres://localhost:5432".to_string()
));
```

### Pattern 3: Unit Variants for Simple Cases

```rust
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file not found")]
    NotFound,

    #[error("Permission denied accessing configuration")]
    PermissionDenied,
}
```

**Use when**: Error needs no additional context

### Pattern 4: Wrapping External Errors

**Source**: `src/database/errors.rs:86-96`

```rust
#[derive(Error, Debug)]
pub enum DatabaseError {
    /// Underlying SQLx error
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),  // ← Automatic conversion

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// UUID parsing error
    #[error("Invalid UUID: {0}")]
    InvalidUuid(#[from] uuid::Error),

    // Note: No blanket anyhow::Error conversion - all errors are structured!
}
```

**Rust Idioms Explained**:

1. **`#[from]` attribute**
   - Auto-generates `From<ExternalError> for MyError`
   - Enables `?` operator to auto-convert errors

2. **Generated `From` implementation**:

```rust
// thiserror generates this:
impl From<sqlx::Error> for DatabaseError {
    fn from(err: sqlx::Error) -> Self {
        Self::Sqlx(err)
    }
}
```

3. **Usage with `?` operator**:

```rust
fn get_user(id: &str) -> Result<User, DatabaseError> {
    // sqlx::Error automatically converts to DatabaseError::Sqlx
    let row = sqlx::query!("SELECT * FROM users WHERE id = ?", id)
        .fetch_one(&pool)
        .await?;  // ← Auto-conversion happens here

    Ok(user_from_row(row))
}
```

**Reference**: [Rust Book - The ? Operator](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html#a-shortcut-for-propagating-errors-the--operator)

---

## Error Code System

Pierre maps domain errors to HTTP status codes and error codes.

**Source**: `src/errors.rs:17-85`

```rust
/// Standard error codes used throughout the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // Authentication & Authorization
    AuthRequired,        // 401
    AuthInvalid,         // 401
    AuthExpired,         // 403
    AuthMalformed,       // 403
    PermissionDenied,    // 403

    // Rate Limiting
    RateLimitExceeded,   // 429
    QuotaExceeded,       // 429

    // Validation
    InvalidInput,        // 400
    MissingRequiredField,// 400
    InvalidFormat,       // 400
    ValueOutOfRange,     // 400

    // Resource Management
    ResourceNotFound,    // 404
    ResourceAlreadyExists, // 409
    ResourceLocked,      // 409
    ResourceUnavailable, // 503

    // External Services
    ExternalServiceError,       // 502
    ExternalServiceUnavailable, // 502
    ExternalAuthFailed,         // 503
    ExternalRateLimited,        // 503

    // Internal Errors
    InternalError,       // 500
    DatabaseError,       // 500
    StorageError,        // 500
    SerializationError,  // 500
}
```

### HTTP Status Code Mapping

**Source**: `src/errors.rs:87-138`

```rust
impl ErrorCode {
    /// Get the HTTP status code for this error
    #[must_use]
    pub const fn http_status(self) -> u16 {
        match self {
            // 400 Bad Request
            Self::InvalidInput
            | Self::MissingRequiredField
            | Self::InvalidFormat
            | Self::ValueOutOfRange => crate::constants::http_status::BAD_REQUEST,

            // 401 Unauthorized - Authentication issues
            Self::AuthRequired | Self::AuthInvalid =>
                crate::constants::http_status::UNAUTHORIZED,

            // 403 Forbidden - Authorization issues
            Self::AuthExpired | Self::AuthMalformed | Self::PermissionDenied =>
                crate::constants::http_status::FORBIDDEN,

            // 404 Not Found
            Self::ResourceNotFound => crate::constants::http_status::NOT_FOUND,

            // 409 Conflict
            Self::ResourceAlreadyExists | Self::ResourceLocked =>
                crate::constants::http_status::CONFLICT,

            // 429 Too Many Requests
            Self::RateLimitExceeded | Self::QuotaExceeded =>
                crate::constants::http_status::TOO_MANY_REQUESTS,

            // 500 Internal Server Error
            Self::InternalError
            | Self::DatabaseError
            | Self::StorageError
            | Self::SerializationError =>
                crate::constants::http_status::INTERNAL_SERVER_ERROR,
        }
    }
}
```

**Rust Idioms Explained**:

1. **`#[must_use]` attribute**
   - Compiler warning if return value is ignored
   - Prevents silent errors: `error.http_status();` (unused) is a warning

2. **`pub const fn`** - Const function
   - Can be evaluated at compile time
   - No heap allocations allowed
   - Perfect for simple mappings like this

3. **Pattern matching with `|` (OR patterns)**
   - `Self::InvalidInput | Self::MissingRequiredField` = match either variant
   - Cleaner than nested `if` statements

**Reference**: [Rust Reference - Const Functions](https://doc.rust-lang.org/reference/const_eval.html#const-functions)

### User-Friendly Descriptions

**Source**: `src/errors.rs:140-172`

```rust
impl ErrorCode {
    /// Get a user-friendly description of this error
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::AuthRequired =>
                "Authentication is required to access this resource",
            Self::AuthInvalid =>
                "The provided authentication credentials are invalid",
            Self::RateLimitExceeded =>
                "Rate limit exceeded. Please slow down your requests",
            Self::ResourceNotFound =>
                "The requested resource was not found",
            // ... more descriptions
        }
    }
}
```

**Return type**: `&'static str` - String slice with `'static` lifetime
- Lives for entire program duration
- No heap allocation
- Stored in binary's read-only data section

---

## Error Conversion with From/Into

Rust's `?` operator relies on `From` trait implementations for automatic error conversion.

### Automatic from with #[from]

**Source**: `src/database/errors.rs:86-96`

```rust
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),  // ← Generates From impl automatically
}
```

### Error Propagation Chain

```rust
// Example: Error propagates through multiple layers

// Layer 1: Database operation
async fn get_user_from_db(id: &str) -> Result<User, DatabaseError> {
    let row = sqlx::query!("SELECT * FROM users WHERE id = ?", id)
        .fetch_one(&pool)
        .await?;  // sqlx::Error → DatabaseError::Sqlx
    Ok(user_from_row(row))
}

// Layer 2: Service operation
async fn fetch_user(id: &str) -> Result<User, AppError> {
    let user = get_user_from_db(id)
        .await?;  // DatabaseError → AppError::Database
    Ok(user)
}

// Layer 3: HTTP handler
async fn user_endpoint(id: String) -> impl IntoResponse {
    match fetch_user(&id).await {
        Ok(user) => (StatusCode::OK, Json(user)),
        Err(app_error) => {
            let status = app_error.http_status();
            let body = app_error.to_json();
            (status, Json(body))
        }
    }
}
```

**Rust Idioms Explained**:

1. **`?` operator propagation**
   - Converts error types automatically via `From` implementations
   - Early return on `Err` variant
   - Equivalent to manual `match`:

```rust
// These are equivalent:
let user = get_user_from_db(id).await?;

// Desugared version:
let user = match get_user_from_db(id).await {
    Ok(val) => val,
    Err(e) => return Err(e.into()),  // ← Calls From::from
};
```

2. **Error wrapping hierarchy**
   - Low-level errors (sqlx::Error) → Domain errors (DatabaseError)
   - Domain errors → Application errors (AppError)
   - Application errors → HTTP responses

**Reference**: [Rust Book - Error Propagation](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html)

---

## Provider Error with Retry Logic

Provider errors include retry information for transient failures.

**Source**: `src/providers/errors.rs:10-101`

```rust
#[derive(Error, Debug)]
pub enum ProviderError {
    /// Provider API is unavailable or returning errors
    #[error("Provider {provider} API error: {status_code} - {message}")]
    ApiError {
        provider: String,
        status_code: u16,
        message: String,
        retryable: bool,
    },

    /// Rate limit exceeded with retry information
    #[error("Rate limit exceeded for {provider}: retry after {retry_after_secs} seconds")]
    RateLimitExceeded {
        provider: String,
        retry_after_secs: u64,
        limit_type: String,
    },

    // ... more variants
}

impl ProviderError {
    /// Check if error is retryable
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        match self {
            Self::ApiError { retryable, .. } => *retryable,
            Self::RateLimitExceeded { .. } | Self::NetworkError(_) => true,
            Self::AuthenticationFailed { .. }
            | Self::NotFound { .. }
            | Self::InvalidData { .. } => false,
        }
    }

    /// Get retry delay in seconds if applicable
    #[must_use]
    pub const fn retry_after_secs(&self) -> Option<u64> {
        match self {
            Self::RateLimitExceeded { retry_after_secs, .. } =>
                Some(*retry_after_secs),
            _ => None,
        }
    }
}
```

**Usage in retry logic**:

```rust
async fn fetch_with_retry(url: &str) -> Result<Response, ProviderError> {
    let mut attempts = 0;
    loop {
        match fetch(url).await {
            Ok(response) => return Ok(response),
            Err(e) if e.is_retryable() && attempts < 3 => {
                attempts += 1;
                if let Some(delay) = e.retry_after_secs() {
                    tokio::time::sleep(Duration::from_secs(delay)).await;
                } else {
                    // Exponential backoff: 2^attempts seconds
                    let delay = 2_u64.pow(attempts);
                    tokio::time::sleep(Duration::from_secs(delay)).await;
                }
            }
            Err(e) => return Err(e),  // Non-retryable or max attempts
        }
    }
}
```

**Rust Idioms Explained**:

1. **Match guards** `if e.is_retryable()`
   - Add conditions to match arms
   - `Err(e) if e.is_retryable()` only matches retryable errors

2. **`const fn` methods**
   - Methods callable in const contexts
   - No allocations, pure logic only

3. **Exponential backoff calculation**
   - `2_u64.pow(attempts)` calculates 2^n
   - Underscores in numbers (`2_u64`) are for readability

---

## Result Type Aliases

Pierre defines type aliases for cleaner signatures.

**Source**: `src/database/errors.rs:109-110`

```rust
/// Result type for database operations
pub type DatabaseResult<T> = Result<T, DatabaseError>;
```

**Source**: `src/providers/errors.rs:132-133`

```rust
/// Result type for provider operations
pub type ProviderResult<T> = Result<T, ProviderError>;
```

**Usage**:

```rust
// Without alias
async fn get_user(id: &str) -> Result<User, DatabaseError> { ... }

// With alias (cleaner)
async fn get_user(id: &str) -> DatabaseResult<User> { ... }
```

**Rust Idiom**: Type aliases reduce boilerplate for commonly-used Result types.

**Reference**: [Rust Book - Type Aliases](https://doc.rust-lang.org/book/ch19-04-advanced-types.html#creating-type-synonyms-with-type-aliases)

---

## Error Handling Patterns

### Pattern 1: Map_err for Context

```rust
use crate::database::DatabaseError;

async fn load_config(path: &str) -> DatabaseResult<Config> {
    let contents = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| DatabaseError::InvalidData {
            field: "config_file".to_string(),
            reason: format!("Failed to read config from {}: {}", path, e),
        })?;

    let config: Config = serde_json::from_str(&contents)
        .map_err(|e| DatabaseError::SerializationError(e))?;

    Ok(config)
}
```

**Rust Idiom**: `.map_err(|e| ...)` transforms one error type to another, adding context.

### Pattern 2: Ok_or for Option → Result

```rust
fn find_user_by_email(email: &str) -> DatabaseResult<User> {
    users_cache.get(email)
        .ok_or_else(|| DatabaseError::NotFound {
            entity_type: "user",
            entity_id: email.to_string(),
        })
}
```

**Rust Idiom**: Convert `Option<T>` to `Result<T, E>` with custom error.

### Pattern 3: And_then for Chaining

```rust
async fn get_user_and_validate(id: &str) -> DatabaseResult<User> {
    get_user_from_db(id)
        .await
        .and_then(|user| {
            if user.is_active {
                Ok(user)
            } else {
                Err(DatabaseError::InvalidData {
                    field: "is_active".to_string(),
                    reason: "User account is inactive".to_string(),
                })
            }
        })
}
```

**Rust Idiom**: `.and_then()` chains operations that can fail, flattening nested Results.

**Reference**: [Rust Book - Result Methods](https://doc.rust-lang.org/std/result/enum.Result.html)

---

## Diagram: Error Flow

```
┌─────────────────────────────────────────────────────────────┐
│                      HTTP Request                           │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
         ┌─────────────────────────────┐
         │    HTTP Handler (Axum)      │
         │  Returns: Result<T, AppError>│
         └─────────────┬───────────────┘
                       │ ?
                       ▼
         ┌─────────────────────────────┐
         │   Service Layer             │
         │  Returns: Result<T, AppError>│
         └─────────────┬───────────────┘
                       │ ?
         ┌─────────────┼────────────────┐
         │             │                │
         ▼             ▼                ▼
┌────────────────┐ ┌──────────────┐ ┌──────────────┐
│ Database Layer │ │Provider Layer│ │ Other Layers │
│DatabaseError   │ │ProviderError │ │ProtocolError │
└────────┬───────┘ └──────┬───────┘ └──────┬───────┘
         │                │                │
         │ From impl      │ From impl      │ From impl
         └────────────────┼────────────────┘
                          │
                          ▼
            ┌─────────────────────────────┐
            │         AppError            │
            │   (unified application error)│
            └─────────────┬───────────────┘
                          │
                          ▼
            ┌─────────────────────────────┐
            │   HTTP Response             │
            │  Status Code + JSON Body    │
            └─────────────────────────────┘
```

**Flow explanation**:
1. Request enters HTTP handler
2. Handler calls service layer (propagates with `?`)
3. Service calls database/provider/protocol layers (propagates with `?`)
4. Domain errors automatically convert to `AppError` via `From` implementations
5. `AppError` converts to HTTP response (status code + JSON body)

---

## Practical Exercises

### Exercise 1: Define a Custom Error Type

Create a configuration error type with `thiserror`:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    // TODO: Add variants for:
    // - File not found (with path)
    // - Invalid format (with details)
    // - Missing required field (with field name)
    // - Invalid value (with field and value)
}
```

**Solution**:
```rust
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file not found: {path}")]
    FileNotFound { path: String },

    #[error("Invalid configuration format: {details}")]
    InvalidFormat { details: String },

    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Invalid value for {field}: {value}")]
    InvalidValue { field: String, value: String },
}
```

### Exercise 2: Implement Error Conversion

Add `From<std::io::Error>` for `ConfigError`:

```rust
impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        // TODO: Convert io::Error to appropriate ConfigError variant
    }
}
```

**Solution**:
```rust
impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => Self::FileNotFound {
                path: "unknown".to_string(),  // Could enhance with path tracking
            },
            _ => Self::InvalidFormat {
                details: err.to_string(),
            },
        }
    }
}
```

### Exercise 3: Error Propagation Chain

Write a function that reads and parses a JSON config file:

```rust
async fn load_json_config(path: &str) -> Result<Config, ConfigError> {
    // TODO:
    // 1. Read file with tokio::fs::read_to_string (returns io::Error)
    // 2. Parse JSON with serde_json::from_str (returns serde_json::Error)
    // 3. Use ? operator for both operations
    // 4. Implement necessary From conversions
}
```

**Solution**:
```rust
async fn load_json_config(path: &str) -> Result<Config, ConfigError> {
    let contents = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => ConfigError::FileNotFound {
                path: path.to_string(),
            },
            _ => ConfigError::InvalidFormat {
                details: e.to_string(),
            },
        })?;

    let config: Config = serde_json::from_str(&contents)
        .map_err(|e| ConfigError::InvalidFormat {
            details: format!("JSON parse error: {}", e),
        })?;

    Ok(config)
}
```

---

## Rust Idioms Summary

| Idiom | Purpose | Example Location |
|-------|---------|-----------------|
| **`thiserror::Error` derive** | Auto-implement Error trait | `src/database/errors.rs:10` |
| **`#[error("...")]` attribute** | Define Display format | `src/database/errors.rs:13` |
| **`#[from]` attribute** | Auto-generate From impl | `src/database/errors.rs:88` |
| **Enum variants with fields** | Structured error context | `src/errors.rs:19-85` |
| **`#[must_use]` attribute** | Warn on unused return | `src/errors.rs:89` |
| **`pub const fn`** | Compile-time functions | `src/errors.rs:90` |
| **Type aliases** | Cleaner Result signatures | `src/database/errors.rs:110` |
| **`.map_err()`** | Error transformation | Throughout codebase |
| **`?` operator** | Error propagation | Throughout codebase |

**References**:
- [Rust Book - Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [thiserror documentation](https://docs.rs/thiserror/)
- [std::error::Error trait](https://doc.rust-lang.org/std/error/trait.Error.html)

---

## Key Takeaways

1. **Never use `anyhow::anyhow!()` in production** - Use structured error types
2. **thiserror is the standard** - Derive macro for custom errors
3. **Error hierarchies match domains** - DatabaseError, ProviderError, AppError
4. **`#[from]` enables `?` operator** - Automatic error conversion
5. **Add context to errors** - Struct variants with meaningful fields
6. **HTTP mapping at boundaries** - ErrorCode → status codes
7. **Retry logic in error types** - ProviderError includes retry information

---

## Next Chapter

[Chapter 3: Configuration Management & Environment Variables](./chapter-03-configuration.md) - Learn how Pierre uses type-safe configuration with `dotenvy`, `clap`, and the algorithm selection system.
