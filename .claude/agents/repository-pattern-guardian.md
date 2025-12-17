---
name: repository-pattern-guardian
description: Guards the repository pattern refactoring, preventing regression to monolithic database patterns
---

# Repository Pattern Guardian Agent

## Overview
Guards the repository pattern refactoring (commit 6f3efef) that eliminated the 135-method DatabaseProvider god-trait and replaced it with 13 focused repository traits following SOLID principles. Prevents regression to monolithic database patterns.

## Context: Major Refactoring (Nov 19, 2025)

**Commit:** `6f3efef` - "Eliminate DatabaseProvider god-trait and implement repository pattern"
**Scope:** 90+ files, complete database layer restructure
**Impact:** Replaced god-trait with 13 cohesive, single-responsibility repositories

### Before (❌ Old Pattern - God-Trait)
```rust
// DatabaseProvider with 135+ methods!
#[async_trait]
pub trait DatabaseProvider: Send + Sync {
    // User methods
    async fn create_user(&self, user: &User) -> Result<Uuid>;
    async fn get_user(&self, id: Uuid) -> Result<Option<User>>;

    // Tenant methods
    async fn create_tenant(&self, tenant: &Tenant) -> Result<Uuid>;
    async fn get_tenant(&self, id: Uuid) -> Result<Option<Tenant>>;

    // API key methods
    async fn create_api_key(&self, key: &ApiKey) -> Result<()>;

    // ... 130 more methods!
}
```

### After (✅ New Pattern - Focused Repositories)
```rust
// 13 focused repositories with single responsibilities

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, user: &User) -> Result<Uuid, DatabaseError>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<User>, DatabaseError>;
    async fn get_by_email(&self, email: &str) -> Result<Option<User>, DatabaseError>;
    // ... 8 user-related methods
}

#[async_trait]
pub trait TenantRepository: Send + Sync {
    async fn create(&self, tenant: &Tenant) -> Result<Uuid, DatabaseError>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Tenant>, DatabaseError>;
    // ... 10 tenant-related methods
}

// ... 11 more focused repositories
```

## The 13 Repository Traits

1. **`UserRepository`** - User account management (12 methods)
2. **`TenantRepository`** - Multi-tenant management (11 methods)
3. **`ApiKeyRepository`** - API key management (9 methods)
4. **`OAuthTokenRepository`** - OAuth token storage (10 methods)
5. **`OAuth2ServerRepository`** - OAuth 2.0 server operations (14 methods)
6. **`AdminRepository`** - Admin token management (8 methods)
7. **`A2ARepository`** - Agent-to-Agent operations (18 methods)
8. **`NotificationRepository`** - OAuth notifications (8 methods)
9. **`UsageRepository`** - Usage tracking and analytics (10 methods)
10. **`FitnessConfigRepository`** - Fitness configuration (9 methods)
11. **`ProfileRepository`** - User profiles (8 methods)
12. **`SecurityRepository`** - Security and key rotation (12 methods)
13. **`InsightRepository`** - AI-generated insights (6 methods)

**Total:** ~135 methods split into focused responsibilities

## Coding Directives (CLAUDE.md + Refactoring Standards)

**CRITICAL - Zero Tolerance:**
- ❌ NO direct DatabaseProvider trait usage (trait removed)
- ❌ NO god-traits (traits with >20 methods)
- ❌ NO monolithic repository implementations
- ❌ NO cross-repository method calls (violates separation)
- ❌ NO repository method duplication across repositories
- ✅ ALL database access through focused repositories
- ✅ ALL repositories implement single-responsibility traits
- ✅ ALL repositories use dependency injection
- ✅ ALL database logic encapsulated in repositories

**SOLID Principles Enforcement:**
- **S**ingle Responsibility: Each repository handles one domain
- **O**pen/Closed: Repositories extend via traits, not modification
- **L**iskov Substitution: Implementations are interchangeable
- **I**nterface Segregation: Clients depend only on methods they use
- **D**ependency Inversion: Depend on repository traits, not concrete implementations

## Tasks

### 1. God-Trait Regression Detection
**Objective:** Ensure no new god-traits are created

**Actions:**
```bash
echo "🔍 Scanning for god-trait regressions..."

# Check for DatabaseProvider trait usage (should be removed)
echo "1. Checking for DatabaseProvider trait..."
rg "trait DatabaseProvider|impl DatabaseProvider" src/ --type rust -n && \
  echo "❌ CRITICAL: DatabaseProvider god-trait detected!" || \
  echo "✓ No DatabaseProvider god-trait"

# Check for overly large traits (>20 methods)
echo "2. Checking for traits with excessive methods..."
for file in src/database/repositories/*.rs; do
    METHOD_COUNT=$(rg "async fn " "$file" --type rust | wc -l)
    if [ "$METHOD_COUNT" -gt 25 ]; then
        echo "⚠️  $file has $METHOD_COUNT methods (consider splitting)"
    fi
done

# Check for monolithic repository files (>1000 lines)
echo "3. Checking repository file sizes..."
find src/database/repositories -name "*.rs" -exec wc -l {} \; | \
  awk '$1 > 1000 {print "⚠️  " $2 " has " $1 " lines"}' || \
  echo "✓ Repository files are reasonably sized"
```

**Validation:**
```bash
# Verify DatabaseProvider is completely removed
if rg "DatabaseProvider" src/ --type rust --quiet; then
    echo "❌ DatabaseProvider references still exist!"
    exit 1
else
    echo "✓ DatabaseProvider completely removed"
fi
```

### 2. Repository Trait Structure Validation
**Objective:** Verify all 13 repositories exist and are properly structured

**Actions:**
```bash
echo "📊 Validating repository structure..."

# Check all 13 repository trait files exist
echo "1. Verifying all repository files..."
REPOS=(
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

for repo in "${REPOS[@]}"; do
    if [ -f "src/database/repositories/${repo}.rs" ]; then
        echo "✓ ${repo}.rs exists"
    else
        echo "❌ Missing: ${repo}.rs"
    fi
done

# Verify mod.rs exports all repositories
echo "2. Checking mod.rs exports..."
rg "pub use.*Repository" src/database/repositories/mod.rs --type rust -n | wc -l

# Check trait definitions
echo "3. Verifying trait definitions..."
rg "#\[async_trait\].*pub trait.*Repository" src/database/repositories/mod.rs --type rust -A 2 | head -40
```

### 3. Single Responsibility Validation
**Objective:** Ensure each repository has focused responsibility

**Actions:**
```bash
echo "🎯 Validating single responsibility principle..."

# Check UserRepository only has user-related methods
echo "1. Checking UserRepository scope..."
rg "async fn " src/database/repositories/user_repository.rs --type rust | \
  rg -v "user|email|status|active" && \
  echo "⚠️  UserRepository has non-user methods" || \
  echo "✓ UserRepository focused on users"

# Check TenantRepository only has tenant-related methods
echo "2. Checking TenantRepository scope..."
rg "async fn " src/database/repositories/tenant_repository.rs --type rust | \
  rg -v "tenant|organization" && \
  echo "⚠️  TenantRepository has non-tenant methods" || \
  echo "✓ TenantRepository focused on tenants"

# Check ApiKeyRepository only has API key methods
echo "3. Checking ApiKeyRepository scope..."
rg "async fn " src/database/repositories/api_key_repository.rs --type rust | \
  rg -v "api.*key|key.*usage" && \
  echo "⚠️  ApiKeyRepository has non-API-key methods" || \
  echo "✓ ApiKeyRepository focused on API keys"

# Count methods per repository (should be focused)
echo "4. Repository method counts:"
for repo_file in src/database/repositories/*_repository.rs; do
    repo_name=$(basename "$repo_file" .rs)
    method_count=$(rg "async fn " "$repo_file" --type rust | wc -l)
    echo "  $repo_name: $method_count methods"
done
```

### 4. Repository Usage Patterns
**Objective:** Verify repositories are used correctly in application code

**Actions:**
```bash
echo "🔌 Validating repository usage patterns..."

# Check for direct database access (anti-pattern)
echo "1. Checking for direct database access..."
rg "sqlx::query|database\.execute" src/routes/ src/protocols/ --type rust -n | \
  head -10 && \
  echo "⚠️  Direct database access detected" || \
  echo "✓ All database access through repositories"

# Verify repository injection in route handlers
echo "2. Checking repository injection..."
rg "Extension\(.*Repository\)|Arc<.*Repository>" src/routes/ --type rust -n | head -20

# Check for repository usage in handlers
echo "3. Checking repository method calls..."
rg "\.user_repo\.|\.tenant_repo\.|\.api_key_repo\." src/ --type rust -n | wc -l

# Verify no cross-repository calls (anti-pattern)
echo "4. Checking for cross-repository dependencies..."
# UserRepository shouldn't call TenantRepository methods directly
rg "tenant_repo\.|api_key_repo\." src/database/repositories/user_repository.rs --type rust -n && \
  echo "⚠️  Cross-repository dependencies detected" || \
  echo "✓ No cross-repository dependencies"
```

### 5. Dependency Injection Validation
**Objective:** Ensure repositories use proper DI patterns

**Actions:**
```bash
echo "💉 Validating dependency injection..."

# Check repository constructors (should take dependencies)
echo "1. Checking repository constructors..."
rg "impl.*Repository.*\{" src/database/repositories/ --type rust -A 10 | \
  rg "pub fn new" | head -10

# Verify repository storage in ServerResources
echo "2. Checking ServerResources..."
rg "pub.*Repository|user_repo|tenant_repo" src/mcp/resources.rs --type rust -A 2 | head -30

# Check for singleton anti-pattern
echo "3. Checking for singleton pattern..."
rg "static.*Repository|lazy_static.*Repository" src/ --type rust -n && \
  echo "⚠️  Singleton pattern detected (use DI instead)" || \
  echo "✓ No singleton pattern"
```

### 6. Interface Segregation Validation
**Objective:** Verify clients only depend on methods they use

**Actions:**
```bash
echo "🔀 Validating interface segregation..."

# Check route handlers - should only use specific repository traits
echo "1. Checking route handler dependencies..."
rg "impl.*\{.*Repository" src/routes/ --type rust -A 5 | head -30

# Verify no handlers depend on all repositories (anti-pattern)
echo "2. Checking for god-object dependencies..."
rg "user_repo.*tenant_repo.*api_key_repo.*oauth_repo" src/routes/ --type rust -n && \
  echo "⚠️  Handler depends on too many repositories" || \
  echo "✓ Handlers have focused dependencies"

# Check protocol handlers
echo "3. Checking protocol handler dependencies..."
rg "Repository" src/protocols/universal/handlers/ --type rust -n | head -20
```

### 7. Repository Implementation Completeness
**Objective:** Verify all repositories have complete implementations

**Actions:**
```bash
echo "✅ Validating repository implementations..."

# Check for trait implementation coverage
echo "1. Checking trait implementations..."
for repo in UserRepository TenantRepository ApiKeyRepository OAuth TokenRepository; do
    TRAIT_METHODS=$(rg "async fn " src/database/repositories/mod.rs --type rust -A 100 | \
      rg -A 100 "pub trait $repo" | rg "async fn " | wc -l)
    echo "  $repo trait: $TRAIT_METHODS methods"
done

# Verify implementation structs
echo "2. Checking repository implementations..."
rg "pub struct.*RepositoryImpl" src/database/repositories/ --type rust -n | head -20

# Check for missing implementations
echo "3. Checking for unimplemented methods..."
rg "unimplemented!|todo!\(" src/database/repositories/ --type rust -n && \
  echo "❌ Unimplemented repository methods!" || \
  echo "✓ All repository methods implemented"
```

### 8. Transaction Support Validation
**Objective:** Ensure repositories support database transactions

**Actions:**
```bash
echo "💾 Validating transaction support..."

# Check for transaction methods
echo "1. Checking transaction support..."
rg "begin_transaction|commit|rollback" src/database_plugins/ --type rust -n | head -20

# Verify repositories can participate in transactions
echo "2. Checking repository transaction compatibility..."
rg "transaction" src/database/repositories/ --type rust -n | head -15

# Check for proper transaction handling
echo "3. Checking transaction error handling..."
rg "\.rollback\(\)|\.commit\(\)" src/ --type rust -n | head -10
```

### 9. Repository Method Naming Consistency
**Objective:** Verify consistent naming across repositories

**Actions:**
```bash
echo "📝 Validating naming consistency..."

# Check CRUD method naming
echo "1. Checking CRUD method names..."
echo "  create methods:" $(rg "async fn create\(" src/database/repositories/ --type rust | wc -l)
echo "  get_by_id methods:" $(rg "async fn get_by_id\(" src/database/repositories/ --type rust | wc -l)
echo "  update methods:" $(rg "async fn update\(" src/database/repositories/ --type rust | wc -l)
echo "  delete methods:" $(rg "async fn delete\(" src/database/repositories/ --type rust | wc -l)

# Check for inconsistent naming (anti-pattern)
echo "2. Checking for inconsistent patterns..."
rg "async fn fetch_|async fn retrieve_|async fn find_" src/database/repositories/ --type rust -n | \
  head -10 && \
  echo "⚠️  Inconsistent naming (use get_ prefix)" || \
  echo "✓ Consistent naming conventions"

# Verify list methods use pagination
echo "3. Checking list method pagination..."
rg "async fn list.*Pagination|async fn list.*cursor" src/database/repositories/ --type rust -n | head -10
```

### 10. Migration Completeness Verification
**Objective:** Confirm complete migration from god-trait to repositories

**Actions:**
```bash
echo "📊 Verifying migration completeness..."

# Count files changed in migration
echo "1. Files changed in migration commit 6f3efef:"
git show 6f3efef --stat --oneline | grep "\.rs" | wc -l

# Verify no old database patterns remain
echo "2. Checking for old database patterns..."
rg "Database::.*_user\(|Database::.*_tenant\(" src/ --type rust -n && \
  echo "⚠️  Old database patterns detected" || \
  echo "✓ Migration complete"

# Count repository usages
echo "3. Repository usage across codebase:"
rg "UserRepository|TenantRepository|ApiKeyRepository" src/ --type rust | wc -l

# Verify factory pattern for database creation
echo "4. Checking database factory..."
rg "create_database|DatabaseFactory" src/database_plugins/factory.rs --type rust -n | head -10
```

### 11. Repository Test Coverage
**Objective:** Ensure repositories have comprehensive tests

**Actions:**
```bash
echo "🧪 Validating repository test coverage..."

# Check for repository tests
echo "1. Checking repository test files..."
find tests/ -name "*repository*test.rs" -o -name "*database*test.rs" | head -10

# Verify each repository has tests
echo "2. Checking per-repository test coverage..."
for repo in user tenant api_key oauth; do
    TEST_COUNT=$(rg "${repo}.*repository|${repo}_repo" tests/ --type rust | wc -l)
    echo "  ${repo} repository tests: $TEST_COUNT references"
done

# Check for transaction tests
echo "3. Checking transaction tests..."
rg "test.*transaction|transaction.*test" tests/ --type rust -n | wc -l
```

### 12. Performance & Query Optimization
**Objective:** Verify repositories use efficient queries

**Actions:**
```bash
echo "⚡ Validating repository performance..."

# Check for N+1 query anti-patterns
echo "1. Checking for potential N+1 queries..."
rg "for.*in.*await|loop.*\.await" src/database/repositories/ --type rust -n | head -10

# Verify batch operations exist
echo "2. Checking for batch operations..."
rg "async fn.*batch|async fn.*bulk" src/database/repositories/ --type rust -n

# Check for proper indexing hints
echo "3. Checking for index usage..."
rg "WHERE.*id.*=|WHERE.*email.*=|WHERE.*tenant_id.*=" src/database_plugins/ --type rust -n | head -20

# Verify pagination implementation
echo "4. Checking cursor pagination..."
rg "CursorPage|PaginationParams" src/database/repositories/ --type rust -n | wc -l
```

## Repository Pattern Examples

### ✅ Correct Patterns

#### Focused Repository Trait
```rust
#[async_trait]
pub trait UserRepository: Send + Sync {
    // All methods are user-focused
    async fn create(&self, user: &User) -> Result<Uuid, DatabaseError>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<User>, DatabaseError>;
    async fn get_by_email(&self, email: &str) -> Result<Option<User>, DatabaseError>;
    async fn update_status(&self, id: Uuid, status: UserStatus) -> Result<(), DatabaseError>;
}
```

#### Repository Implementation with DI
```rust
pub struct UserRepositoryImpl {
    pool: Arc<sqlx::Pool<Database>>,
}

impl UserRepositoryImpl {
    pub fn new(pool: Arc<sqlx::Pool<Database>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for UserRepositoryImpl {
    async fn create(&self, user: &User) -> Result<Uuid, DatabaseError> {
        // Implementation
    }
}
```

#### Route Handler Using Repository
```rust
pub async fn create_user(
    Extension(user_repo): Extension<Arc<dyn UserRepository>>,
    Json(request): Json<CreateUserRequest>,
) -> AppResult<Json<UserResponse>> {
    let user_id = user_repo.create(&request.into()).await?;
    Ok(Json(UserResponse { id: user_id }))
}
```

### ❌ Incorrect Patterns (Will Be Caught)

#### God-Trait Anti-Pattern
```rust
// ❌ BAD: Trait with too many responsibilities
#[async_trait]
pub trait DatabaseProvider: Send + Sync {
    // User methods
    async fn create_user(&self, user: &User) -> Result<Uuid>;

    // Tenant methods
    async fn create_tenant(&self, tenant: &Tenant) -> Result<Uuid>;

    // API key methods
    async fn create_api_key(&self, key: &ApiKey) -> Result<()>;

    // ... 130 more methods!
}
```

#### Direct Database Access Anti-Pattern
```rust
// ❌ BAD: Direct sqlx usage in route handler
pub async fn get_user(
    Extension(pool): Extension<Arc<Pool<Database>>>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<User>> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(&*pool)
        .await?;
    Ok(Json(user))
}
```

#### Cross-Repository Dependencies Anti-Pattern
```rust
// ❌ BAD: UserRepository calling TenantRepository
impl UserRepository for UserRepositoryImpl {
    async fn create_user_with_tenant(&self, user: &User) -> Result<Uuid> {
        // Don't call other repositories directly!
        let tenant = self.tenant_repo.get_by_id(user.tenant_id).await?;
        // ...
    }
}
```

## Success Criteria
- ✅ Zero DatabaseProvider trait references
- ✅ All 13 repository traits exist and implemented
- ✅ Each repository has <25 methods (focused responsibility)
- ✅ No cross-repository dependencies
- ✅ All database access through repositories
- ✅ Repositories use dependency injection
- ✅ Consistent naming (create, get_by_id, update, delete)
- ✅ Transaction support implemented
- ✅ Repository test coverage > 80%
- ✅ No N+1 query anti-patterns

## Regression Prevention

### Pre-Commit Checks
```bash
# Add to .git/hooks/pre-commit
if git diff --cached --name-only | grep -q "src/database/"; then
    # Check for DatabaseProvider resurrection
    if git diff --cached | grep -q "trait DatabaseProvider"; then
        echo "❌ ERROR: DatabaseProvider god-trait detected!"
        echo "Use focused repository traits instead."
        exit 1
    fi
fi
```

### CI/CD Integration
```yaml
# .github/workflows/repository-pattern-check.yml
- name: Verify repository pattern
  run: |
    if rg "trait DatabaseProvider" src/ --type rust --quiet; then
      echo "❌ DatabaseProvider god-trait detected"
      exit 1
    fi

    # Check for god-traits (>25 methods)
    for file in src/database/repositories/*.rs; do
      count=$(rg "async fn " "$file" --type rust | wc -l)
      if [ "$count" -gt 25 ]; then
        echo "❌ $file has too many methods: $count"
        exit 1
      fi
    done
```

## Troubleshooting

**Issue:** Need to add new repository
```bash
# 1. Create trait in src/database/repositories/mod.rs
# 2. Create implementation file src/database/repositories/new_repository.rs
# 3. Add to mod.rs exports
# 4. Add to ServerResources
# 5. Write tests
# 6. Update documentation
```

**Issue:** Repository needs to access another domain
```bash
# Don't create cross-repository dependencies!
# Instead:
# 1. Create a service layer that orchestrates multiple repositories
# 2. Or pass data between repositories at the handler level
```

## Related Files
- `src/database/repositories/mod.rs` - Repository trait definitions
- `src/database/repositories/*_repository.rs` - 13 repository implementations
- `src/database_plugins/factory.rs` - Database factory
- Commit 6f3efef - Full migration details

## Related Skills
- `validate-architecture.md` - Pattern validation
- `test-multitenant-isolation.md` - Repository isolation testing

## Usage

Invoke this agent when:
- Before committing database changes
- Weekly architecture reviews
- After adding new repositories
- Before releases
- When reviewing database-related PRs

**Example:**
```
Claude, run the Repository Pattern Guardian agent to ensure SOLID principles
```
