---
name: error-handling-guardian
description: Guards structured error handling, preventing regression to anyhow and ensuring AppResult usage
---

# Error Handling Guardian Agent

## Overview
Guards the unified error handling refactoring (commit b592b5e) that converted 111 files from `anyhow` to structured `AppResult` types. Prevents regression to unstructured error handling and ensures proper error context preservation.

## Context: Major Refactoring (Nov 19, 2025)

**Commit:** `b592b5e` - "Convert all error handling from anyhow to structured AppResult types"
**Scope:** 111 files across entire codebase
**Impact:** Standardized error handling with proper context preservation

### Before (❌ Old Pattern)
```rust
use anyhow::{anyhow, Context, Result};

fn fetch_user(id: Uuid) -> Result<User> {
    let user = db.get_user(id).context("Failed to fetch user")?;
    Ok(user)
}

// Error is unstructured string - no programmatic handling
return Err(anyhow!("Database connection failed"));
```

### After (✅ New Pattern)
```rust
use crate::errors::{AppError, AppResult, ErrorCode};

fn fetch_user(id: Uuid) -> AppResult<User> {
    let user = db.get_user(id)
        .map_err(|e| AppError::new(
            ErrorCode::DatabaseError,
            format!("Failed to fetch user {}: {}", id, e)
        ))?;
    Ok(user)
}

// Structured error with error code and context
return Err(AppError::new(
    ErrorCode::DatabaseError,
    "Database connection failed".to_string()
));
```

## Coding Directives (CLAUDE.md + Refactoring Standards)

**CRITICAL - Zero Tolerance:**
- ❌ NO `anyhow!()` macro anywhere in codebase
- ❌ NO `anyhow::Result` type (use `AppResult`)
- ❌ NO `anyhow::Error` type (use `AppError`)
- ❌ NO `.context()` method from anyhow (use `.map_err()` with `AppError`)
- ❌ NO bare `Result<T, anyhow::Error>` types
- ✅ ALL functions return `AppResult<T>` for fallible operations
- ✅ ALL errors use `ErrorCode` enum for categorization
- ✅ ALL error contexts preserved with detailed messages

**Error Code Categories (src/errors.rs):**
- Authentication & Authorization (`AuthRequired`, `AuthInvalid`, `AuthExpired`, `PermissionDenied`)
- Rate Limiting (`RateLimitExceeded`, `QuotaExceeded`)
- Validation (`InvalidInput`, `MissingRequiredField`, `ValueOutOfRange`)
- Resources (`ResourceNotFound`, `ResourceAlreadyExists`, `ResourceLocked`)
- External Services (`ExternalServiceError`, `ExternalServiceUnavailable`)
- Configuration (`ConfigError`, `ConfigMissing`, `ConfigInvalid`)
- Internal (`InternalError`, `DatabaseError`, `StorageError`, `SerializationError`)

## Tasks

### 1. Anyhow Regression Detection
**Objective:** Ensure no anyhow usage has crept back into codebase

**Actions:**
```bash
echo "🔍 Scanning for anyhow regressions..."

# Check for anyhow macro usage (FORBIDDEN)
echo "1. Checking for anyhow! macro..."
rg "anyhow!" src/ --type rust -n && \
  echo "❌ CRITICAL: anyhow! macro detected!" || \
  echo "✓ No anyhow! macro usage"

# Check for anyhow imports (FORBIDDEN)
echo "2. Checking for anyhow imports..."
rg "use anyhow::|use anyhow;" src/ --type rust -n && \
  echo "❌ CRITICAL: anyhow imports detected!" || \
  echo "✓ No anyhow imports"

# Check for anyhow::Result type (should be AppResult)
echo "3. Checking for anyhow::Result..."
rg "anyhow::Result|Result<.*anyhow::Error>" src/ --type rust -n && \
  echo "❌ CRITICAL: anyhow::Result detected!" || \
  echo "✓ No anyhow::Result usage"

# Check for .context() method (anyhow-specific)
echo "4. Checking for .context() method..."
rg "\.context\(\"" src/ --type rust -n | head -20 && \
  echo "⚠️  WARNING: .context() usage detected (should use .map_err())" || \
  echo "✓ No .context() usage"

# Check Cargo.toml dependencies
echo "5. Checking Cargo.toml..."
rg "^anyhow\s*=" Cargo.toml && \
  echo "⚠️  anyhow still in dependencies (ok if dev-dependency)" || \
  echo "✓ anyhow not in main dependencies"
```

**Validation:**
```bash
# Count anyhow occurrences (should be 0 in src/)
ANYHOW_COUNT=$(rg "use anyhow" src/ --type rust | wc -l)
if [ "$ANYHOW_COUNT" -gt 0 ]; then
    echo "❌ Found $ANYHOW_COUNT anyhow imports in src/"
    exit 1
else
    echo "✓ Zero anyhow imports in production code"
fi
```

### 2. AppResult Usage Validation
**Objective:** Verify proper use of `AppResult<T>` type

**Actions:**
```bash
echo "✅ Validating AppResult usage..."

# Verify AppResult imports
echo "1. Checking AppResult imports..."
rg "use crate::errors::\{.*AppResult" src/ --type rust -n | wc -l

# Check for proper Result type usage
echo "2. Checking Result<T, AppError> patterns..."
rg "Result<.*AppError>" src/ --type rust -n | head -20

# Verify functions return AppResult
echo "3. Checking public function signatures..."
rg "pub (async )?fn.*->.*AppResult" src/ --type rust -n | wc -l

# Check for bare Result without error type (anti-pattern)
echo "4. Checking for unqualified Result usage..."
rg "-> Result<[^,]+>(?!;)" src/ --type rust -n | head -10 && \
  echo "⚠️  Found unqualified Result usage" || \
  echo "✓ All Results properly typed"
```

### 3. ErrorCode Usage Validation
**Objective:** Ensure all errors use appropriate ErrorCode enum

**Actions:**
```bash
echo "🏷️ Validating ErrorCode usage..."

# Verify ErrorCode enum completeness
echo "1. Checking ErrorCode enum..."
rg "pub enum ErrorCode" src/errors.rs --type rust -A 50

# Check for ErrorCode::new usage
echo "2. Checking AppError::new() calls..."
rg "AppError::new\(ErrorCode::" src/ --type rust -n | wc -l

# Identify most common error codes
echo "3. Most common ErrorCodes..."
rg "ErrorCode::" src/ --type rust -o | sort | uniq -c | sort -rn | head -20

# Check for hardcoded error strings (anti-pattern)
echo "4. Checking for hardcoded error strings..."
rg "Err\(\"" src/ --type rust -n | head -10 && \
  echo "⚠️  Found hardcoded error strings" || \
  echo "✓ No hardcoded error strings"
```

### 4. Error Context Preservation
**Objective:** Verify error context is properly preserved during conversion

**Actions:**
```bash
echo "📝 Validating error context preservation..."

# Check for .map_err() usage (proper pattern)
echo "1. Checking .map_err() usage..."
rg "\.map_err\(\|.*\|.*AppError::new" src/ --type rust -n | wc -l

# Verify error messages include context
echo "2. Checking error message formats..."
rg "AppError::new\(.*format!\(" src/ --type rust -n | head -10

# Check for lost context (anti-pattern: generic error messages)
echo "3. Checking for generic error messages..."
rg "AppError::new\(.*\"Error\"\)" src/ --type rust -n && \
  echo "⚠️  Generic error messages found" || \
  echo "✓ Error messages are specific"

# Verify errors include identifiers (UUIDs, IDs)
echo "4. Checking error messages include identifiers..."
rg "AppError::new\(.*\{\}.*id\|uuid\|tenant" src/ --type rust -n | head -10
```

### 5. HTTP Status Code Mapping
**Objective:** Validate ErrorCode → HTTP status mapping

**Actions:**
```bash
echo "🌐 Validating HTTP status code mapping..."

# Check ErrorCode::http_status() implementation
echo "1. Checking http_status() method..."
rg "pub const fn http_status" src/errors.rs --type rust -A 100 | head -50

# Verify status code constants
echo "2. Checking HTTP status constants..."
rg "http_status::" src/errors.rs --type rust -n | sort | uniq

# Check for correct status code usage in routes
echo "3. Checking route error handling..."
rg "\.http_status\(\)" src/routes/ --type rust -n | head -10
```

### 6. External Crate Error Conversion
**Objective:** Ensure proper conversion of external errors to AppError

**Actions:**
```bash
echo "🔄 Validating external error conversions..."

# Check for impl From<T> for AppError
echo "1. Checking From<T> implementations..."
rg "impl From<.*> for AppError" src/errors.rs --type rust -A 10

# Verify sqlx errors are converted
echo "2. Checking sqlx error conversions..."
rg "sqlx::Error.*AppError" src/ --type rust -n | head -10

# Verify reqwest errors are converted
echo "3. Checking reqwest error conversions..."
rg "reqwest::Error.*AppError" src/ --type rust -n | head -10

# Check for unconverted external errors (anti-pattern)
echo "4. Checking for direct external error propagation..."
rg "-> Result<.*sqlx::Error>|-> Result<.*reqwest::Error>" src/ --type rust -n && \
  echo "⚠️  External errors not converted to AppError" || \
  echo "✓ All external errors properly converted"
```

### 7. Error Handling Test Coverage
**Objective:** Verify error paths are tested

**Actions:**
```bash
echo "🧪 Validating error handling test coverage..."

# Check for error case tests
echo "1. Checking error test cases..."
rg "#\[tokio::test\].*error|test.*error" tests/ --type rust -n | wc -l

# Verify AppError usage in tests
echo "2. Checking AppError in tests..."
rg "AppError::|ErrorCode::" tests/ --type rust -n | wc -l

# Check for .unwrap() in error handling code (anti-pattern)
echo "3. Checking for unwrap() in error paths..."
rg "\.unwrap\(\)" src/errors.rs --type rust -n && \
  echo "❌ unwrap() in error handling module!" || \
  echo "✓ No unwrap() in error handling"
```

### 8. Migration Completeness Verification
**Objective:** Confirm all 111 files properly migrated

**Actions:**
```bash
echo "📊 Verifying migration completeness..."

# Get list of files changed in migration commit
echo "1. Files changed in migration commit b592b5e:"
git show b592b5e --stat --oneline | grep "\.rs" | wc -l

# Verify no mixed error handling patterns
echo "2. Checking for mixed patterns (AppResult + anyhow)..."
for file in $(git show b592b5e --name-only --format="" | grep "^src/.*\.rs$"); do
    if git show main:"$file" 2>/dev/null | grep -q "use anyhow"; then
        echo "⚠️  $file still has anyhow import"
    fi
done || echo "✓ All migrated files clean"

# Count AppResult usage across codebase
echo "3. AppResult usage count:"
rg "AppResult<" src/ --type rust | wc -l

# Verify error module exports
echo "4. Checking error module exports..."
rg "pub use.*AppError|pub use.*AppResult|pub use.*ErrorCode" src/lib.rs --type rust -n
```

### 9. Error Serialization Validation
**Objective:** Ensure errors serialize correctly for API responses

**Actions:**
```bash
echo "📡 Validating error serialization..."

# Check Serialize/Deserialize on ErrorCode
echo "1. Checking ErrorCode serialization..."
rg "impl Serialize for ErrorCode|impl.*Deserialize.*for ErrorCode" src/errors.rs --type rust -A 10

# Verify error response format
echo "2. Checking error response structure..."
rg "struct.*ErrorResponse|error_response" src/ --type rust -A 10 | head -30

# Check JSON error formatting
echo "3. Checking JSON error responses..."
rg "Json\(.*error\)|json!.*error" src/routes/ --type rust -n | head -20
```

### 10. Documentation Validation
**Objective:** Verify error types are properly documented

**Actions:**
```bash
echo "📚 Validating error documentation..."

# Check doc comments on ErrorCode variants
echo "1. Checking ErrorCode documentation..."
rg "///.*Authentication|///.*Rate limit|///.*Validation" src/errors.rs --type rust -n | head -20

# Verify error code descriptions
echo "2. Checking error descriptions..."
rg "pub const fn description" src/errors.rs --type rust -A 50 | head -40

# Check for undocumented error codes
echo "3. Checking for missing documentation..."
rg "^\s+[A-Z][a-zA-Z]+,$" src/errors.rs --type rust | wc -l
```

## Error Pattern Examples

### ✅ Correct Patterns

#### Database Errors
```rust
async fn get_user(id: Uuid) -> AppResult<User> {
    let user = db.get_user(id)
        .await
        .map_err(|e| AppError::new(
            ErrorCode::DatabaseError,
            format!("Failed to fetch user {}: {}", id, e)
        ))?;
    Ok(user)
}
```

#### Validation Errors
```rust
fn validate_email(email: &str) -> AppResult<()> {
    if !email.contains('@') {
        return Err(AppError::new(
            ErrorCode::InvalidFormat,
            format!("Invalid email format: {}", email)
        ));
    }
    Ok(())
}
```

#### External Service Errors
```rust
async fn fetch_strava_data(token: &str) -> AppResult<StravaData> {
    let response = client
        .get("https://www.strava.com/api/v3/athlete")
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| AppError::new(
            ErrorCode::ExternalServiceError,
            format!("Strava API request failed: {}", e)
        ))?;

    // ... parse response
}
```

#### Resource Not Found
```rust
async fn get_activity(id: Uuid) -> AppResult<Activity> {
    db.get_activity(id)
        .await?
        .ok_or_else(|| AppError::new(
            ErrorCode::ResourceNotFound,
            format!("Activity {} not found", id)
        ))
}
```

### ❌ Incorrect Patterns (Will Be Caught)

#### Anyhow Usage (FORBIDDEN)
```rust
// ❌ BAD: Using anyhow
use anyhow::{anyhow, Context, Result};

fn process() -> Result<()> {
    let data = fetch_data().context("Failed to fetch")?;
    Ok(())
}
```

#### Generic Error Messages (Anti-Pattern)
```rust
// ❌ BAD: Generic error without context
return Err(AppError::new(
    ErrorCode::InternalError,
    "Error occurred".to_string()  // No context!
));
```

#### Hardcoded Error Strings (Anti-Pattern)
```rust
// ❌ BAD: Hardcoded string instead of AppError
return Err("Database connection failed");  // Wrong type!
```

## Success Criteria
- ✅ Zero anyhow imports in src/
- ✅ Zero anyhow! macro usage
- ✅ All functions return `AppResult<T>`
- ✅ All errors use `ErrorCode` enum
- ✅ Error messages include context (IDs, operations)
- ✅ HTTP status mapping correct for all ErrorCode variants
- ✅ External errors properly converted to AppError
- ✅ Error handling test coverage > 80%
- ✅ All error codes documented
- ✅ Proper error serialization for API responses

## Regression Prevention

### Pre-Commit Checks
```bash
# Add to .git/hooks/pre-commit
if git diff --cached --name-only | grep -q "\.rs$"; then
    # Check for anyhow in staged files
    if git diff --cached | grep -q "use anyhow"; then
        echo "❌ ERROR: anyhow import detected in staged files!"
        echo "Use AppResult and AppError instead."
        exit 1
    fi
fi
```

### CI/CD Integration
```yaml
# .github/workflows/error-handling-check.yml
- name: Verify no anyhow regression
  run: |
    if rg "use anyhow::|anyhow!" src/ --type rust --quiet; then
      echo "❌ anyhow usage detected - use AppResult instead"
      exit 1
    fi
```

## Troubleshooting

**Issue:** Need to add new error type
```bash
# 1. Add to ErrorCode enum in src/errors.rs
# 2. Add HTTP status mapping in http_status() method
# 3. Add description in description() method
# 4. Add serialization case if custom needed
# 5. Run full test suite
```

**Issue:** External library error needs conversion
```bash
# Implement From<ExternalError> for AppError
impl From<ExternalError> for AppError {
    fn from(err: ExternalError) -> Self {
        AppError::new(
            ErrorCode::ExternalServiceError,
            format!("External error: {}", err)
        )
    }
}
```

## Related Files
- `src/errors.rs` - Error type definitions (commit b592b5e)
- `src/lib.rs` - AppResult/AppError exports
- All 111 files in migration (see git show b592b5e --stat)

## Related Skills
- `validate-architecture.md` - Pattern validation
- `strict-clippy-check.md` - Code quality enforcement

## Usage

Invoke this agent when:
- Before committing error handling changes
- Weekly regression checks
- After dependency updates
- Before releases
- When reviewing pull requests with error handling

**Example:**
```
Claude, run the Error Handling Guardian agent to check for anyhow regressions
```
