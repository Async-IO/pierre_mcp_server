---
name: check-repository-pattern
description: Validates database access follows repository pattern, detects god-trait regression, ensures focused repositories
---

# Check Repository Pattern Skill

## Purpose
Quick validation that database access follows the repository pattern (commit 6f3efef). Detects god-trait regression and ensures proper use of focused repositories.

## CLAUDE.md Compliance
- ✅ Enforces SOLID principles (Single Responsibility)
- ✅ Validates focused repository usage
- ✅ Prevents monolithic database patterns

## Usage
Run this skill:
- Before committing database changes
- Daily pre-commit validation
- After adding new repositories
- When reviewing database-related PRs

## Prerequisites
- ripgrep (`rg`)

## Commands

### Quick Check (Fast)
```bash
# Check for repository pattern compliance
echo "🔍 Checking repository pattern..."

# 1. Check for DatabaseProvider god-trait (FORBIDDEN)
if rg "trait DatabaseProvider|impl DatabaseProvider" src/ --type rust --quiet; then
    echo "❌ FAIL: DatabaseProvider god-trait detected!"
    rg "DatabaseProvider" src/ --type rust -n | head -10
    exit 1
else
    echo "✓ PASS: No DatabaseProvider god-trait"
fi

# 2. Verify repository directory exists
if [ -d "src/database/repositories" ]; then
    REPO_COUNT=$(ls -1 src/database/repositories/*_repository.rs 2>/dev/null | wc -l)
    echo "✓ PASS: $REPO_COUNT repository files found"
else
    echo "❌ FAIL: Repository directory missing!"
    exit 1
fi

# 3. Check for direct database access in routes (anti-pattern)
if rg "sqlx::query|\.execute\(|\.fetch" src/routes/ --type rust --quiet; then
    echo "⚠️  WARNING: Direct database access in routes detected"
    rg "sqlx::query" src/routes/ --type rust -n | head -5
else
    echo "✓ PASS: No direct database access in routes"
fi

# 4. Verify repository usage
REPO_USAGE=$(rg "Repository" src/routes/ src/protocols/ --type rust | wc -l)
echo "✓ Repository usage: $REPO_USAGE references"

echo ""
echo "✅ Repository pattern check PASSED"
```

### Comprehensive Check
```bash
#!/bin/bash
set -e

echo "🔍 Comprehensive Repository Pattern Check"
echo "=========================================="

# 1. God-Trait Check
echo ""
echo "1. Checking for DatabaseProvider god-trait..."
if rg "trait DatabaseProvider" src/ --type rust --quiet; then
    echo "❌ DatabaseProvider god-trait detected:"
    rg "trait DatabaseProvider" src/ --type rust -n
    exit 1
else
    echo "✓ No DatabaseProvider god-trait"
fi

# 2. Repository Files
echo ""
echo "2. Verifying repository files..."
EXPECTED_REPOS=(
    "user_repository"
    "tenant_repository"
    "api_key_repository"
    "oauth_token_repository"
    "oauth2_server_repository"
    "admin_repository"
    "a2a_repository"
    "notification_repository"
    "usage_repository"
    "fitness_config_repository"
    "profile_repository"
    "security_repository"
    "insight_repository"
)

MISSING=0
for repo in "${EXPECTED_REPOS[@]}"; do
    if [ -f "src/database/repositories/${repo}.rs" ]; then
        echo "  ✓ ${repo}.rs"
    else
        echo "  ❌ MISSING: ${repo}.rs"
        MISSING=$((MISSING + 1))
    fi
done

if [ $MISSING -gt 0 ]; then
    echo "❌ $MISSING repository files missing!"
    exit 1
fi

# 3. Repository Trait Size
echo ""
echo "3. Checking repository sizes (should be focused)..."
for repo_file in src/database/repositories/*_repository.rs; do
    if [ -f "$repo_file" ]; then
        repo_name=$(basename "$repo_file" .rs)
        method_count=$(rg "async fn " "$repo_file" --type rust | wc -l)

        if [ "$method_count" -gt 25 ]; then
            echo "  ⚠️  $repo_name: $method_count methods (consider splitting)"
        else
            echo "  ✓ $repo_name: $method_count methods"
        fi
    fi
done

# 4. Direct Database Access
echo ""
echo "4. Checking for direct database access..."
DIRECT_ACCESS=$(rg "sqlx::query" src/routes/ src/protocols/ --type rust | wc -l)
if [ "$DIRECT_ACCESS" -gt 0 ]; then
    echo "⚠️  Warning: Found $DIRECT_ACCESS direct database accesses in routes/protocols"
    rg "sqlx::query" src/routes/ src/protocols/ --type rust -n | head -5
else
    echo "✓ No direct database access in routes/protocols"
fi

# 5. Repository Usage
echo ""
echo "5. Validating repository usage..."
REPO_USAGE=$(rg "Repository" src/routes/ src/protocols/ --type rust | wc -l)
echo "✓ Repository references in routes/protocols: $REPO_USAGE"

# Top repositories by usage
echo ""
echo "Most used repositories:"
rg "UserRepository|TenantRepository|ApiKeyRepository|OAuthTokenRepository" src/ --type rust -o | \
  sort | uniq -c | sort -rn | head -5

# 6. Repository Exports
echo ""
echo "6. Checking repository exports..."
EXPORTS=$(rg "pub use.*Repository" src/database/repositories/mod.rs --type rust | wc -l)
if [ "$EXPORTS" -ge 13 ]; then
    echo "✓ Repository exports: $EXPORTS"
else
    echo "⚠️  Warning: Only $EXPORTS repository exports (expected 13)"
fi

# 7. Trait Definitions
echo ""
echo "7. Verifying repository trait definitions..."
TRAITS=$(rg "pub trait.*Repository.*Send.*Sync" src/database/repositories/mod.rs --type rust | wc -l)
echo "✓ Repository traits: $TRAITS"

echo ""
echo "✅ Comprehensive repository pattern check PASSED"
```

## Success Criteria
- ✅ Zero DatabaseProvider trait references
- ✅ All 13 repository files exist
- ✅ Repository files have <25 methods each
- ✅ No direct database access in routes/protocols
- ✅ Repository usage > 50 references

## Expected Output (Success)
```
🔍 Checking repository pattern...
✓ PASS: No DatabaseProvider god-trait
✓ PASS: 13 repository files found
✓ PASS: No direct database access in routes
✓ Repository usage: 127 references

✅ Repository pattern check PASSED
```

## Failure Example
```
🔍 Checking repository pattern...
❌ FAIL: DatabaseProvider god-trait detected!
src/database/old_provider.rs:15:pub trait DatabaseProvider {
src/database/old_provider.rs:23:impl DatabaseProvider for PostgresProvider {
```

## Fixing Violations

### Remove god-trait usage
```rust
// ❌ Before
async fn handler(
    Extension(db): Extension<Arc<dyn DatabaseProvider>>,
) -> AppResult<Json<User>> {
    let user = db.get_user(id).await?;
    Ok(Json(user))
}

// ✅ After
async fn handler(
    Extension(user_repo): Extension<Arc<dyn UserRepository>>,
) -> AppResult<Json<User>> {
    let user = user_repo.get_by_id(id).await?
        .ok_or(AppError::new(
            ErrorCode::ResourceNotFound,
            format!("User {} not found", id)
        ))?;
    Ok(Json(user))
}
```

### Replace direct database access
```rust
// ❌ Before (direct sqlx usage in route)
async fn get_user(
    Extension(pool): Extension<Arc<Pool<Database>>>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<User>> {
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE id = $1"
    )
    .bind(id)
    .fetch_one(&*pool)
    .await?;
    Ok(Json(user))
}

// ✅ After (use repository)
async fn get_user(
    Extension(user_repo): Extension<Arc<dyn UserRepository>>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<User>> {
    let user = user_repo.get_by_id(id).await?
        .ok_or(AppError::new(
            ErrorCode::ResourceNotFound,
            format!("User {} not found", id)
        ))?;
    Ok(Json(user))
}
```

### Split oversized repository
```rust
// ❌ Before: Repository with 30+ methods
pub trait UserRepository: Send + Sync {
    // User account methods
    async fn create(&self, user: &User) -> Result<Uuid>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<User>>;

    // User preferences
    async fn get_preferences(&self, id: Uuid) -> Result<Preferences>;

    // User profile
    async fn get_profile(&self, id: Uuid) -> Result<Profile>;

    // ... 25 more methods
}

// ✅ After: Split into focused repositories
pub trait UserRepository: Send + Sync {
    // Only core user account methods (10 methods)
    async fn create(&self, user: &User) -> Result<Uuid>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<User>>;
}

pub trait ProfileRepository: Send + Sync {
    // Profile-specific methods (8 methods)
    async fn get_profile(&self, user_id: Uuid) -> Result<Profile>;
    async fn update_profile(&self, profile: &Profile) -> Result<()>;
}
```

## Integration with Git Hooks

### Pre-Commit Hook
Add to `.git/hooks/pre-commit`:
```bash
#!/bin/bash

# Check staged database files for god-trait
if git diff --cached --name-only | grep -q "src/database/"; then
    if git diff --cached | grep -q "trait DatabaseProvider"; then
        echo "❌ ERROR: DatabaseProvider god-trait detected!"
        echo "Use focused repository traits instead."
        echo ""
        echo "Run: .claude/skills/check-repository-pattern.md"
        exit 1
    fi
fi
```

## CI/CD Integration

### GitHub Actions
```yaml
# .github/workflows/repository-pattern.yml
name: Repository Pattern Check

on: [push, pull_request]

jobs:
  check-repository-pattern:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install ripgrep
        run: sudo apt-get install -y ripgrep

      - name: Check for god-trait
        run: |
          if rg "trait DatabaseProvider" src/ --type rust --quiet; then
            echo "❌ DatabaseProvider god-trait detected"
            exit 1
          fi

      - name: Verify repositories exist
        run: |
          if [ ! -d "src/database/repositories" ]; then
            echo "❌ Repository directory missing"
            exit 1
          fi

          count=$(ls -1 src/database/repositories/*_repository.rs 2>/dev/null | wc -l)
          if [ "$count" -lt 13 ]; then
            echo "❌ Only $count repositories found (expected 13)"
            exit 1
          fi

          echo "✓ Repository pattern check passed"
```

## Related Files
- `src/database/repositories/mod.rs` - Repository traits
- `src/database/repositories/*_repository.rs` - 13 implementations
- Commit 6f3efef - Repository pattern migration

## Related Agents
- `repository-pattern-guardian.md` - Comprehensive repository validation

## Troubleshooting

**Issue:** Repository file count incorrect
```bash
# List all repository files
ls -1 src/database/repositories/*_repository.rs

# Check for naming inconsistencies
find src/database/repositories -name "*.rs" -type f
```

**Issue:** False positive for direct database access
```bash
# Check if it's in a repository implementation (OK)
rg "sqlx::query" src/database/repositories/ --type rust -n
# vs routes/protocols (NOT OK)
rg "sqlx::query" src/routes/ src/protocols/ --type rust -n
```

**Issue:** Repository has too many methods
```bash
# Count methods per repository
for file in src/database/repositories/*_repository.rs; do
    echo "$(basename $file): $(rg 'async fn ' $file --type rust | wc -l) methods"
done
```

## Quick Reference

```bash
# One-line check
rg "trait DatabaseProvider" src/ --type rust && echo "FAIL" || echo "PASS"

# Check repository count
ls -1 src/database/repositories/*_repository.rs | wc -l

# Check for direct database access
rg "sqlx::query" src/routes/ --type rust && echo "WARNING" || echo "PASS"

# Repository usage count
rg "Repository" src/routes/ src/protocols/ --type rust | wc -l
```
