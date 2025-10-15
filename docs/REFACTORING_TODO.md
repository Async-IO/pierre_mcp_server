# Refactoring TODO - Code Review Improvements

This document tracks the remaining refactoring work from the 7/10 code review to bring the codebase to 9/10+.

## ✅ Completed (Score: 8.5/10)

### 1. OAuth2 Security Hardening
- ✅ Replace SHA-256 with Argon2id for client secret hashing
- ✅ Remove dangerous RNG fallback secret
- ✅ Enforce PKCE for authorization code flow
- ✅ Strengthen redirect URI validation (HTTPS, no wildcards, no fragments)
- ✅ Eliminate unsafe code (removed linkme, manual plugin registration)

### 2. Type Safety Improvements
- ✅ Add newtype wrappers (TenantId, UserId, ClientId, ProviderName) in `src/types.rs`
- ✅ Add TenantContext for tenant isolation tracking

### 3. Structured Error Handling
- ✅ Create DatabaseError with thiserror in `src/database/errors.rs`
- ✅ Create ProviderError with thiserror in `src/providers/errors.rs`

## 🚧 In Progress (Next Quarter - Score Target: 9+/10)

### Task 1: Tenant-Enforced Database Layer

**Goal**: Prevent cross-tenant data leakage by enforcing TenantContext at DAO layer.

**Pattern to Implement**:
```rust
// Current (unsafe - no tenant enforcement):
pub async fn get_user(&self, user_id: Uuid) -> Result<Option<User>> {
    sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
}

// Target (safe - tenant enforced):
use crate::types::{TenantContext, UserId};
use crate::database::errors::DatabaseResult;

pub async fn get_user(
    &self,
    ctx: &TenantContext,
    user_id: UserId,
) -> DatabaseResult<Option<User>> {
    let user = sqlx::query_as(
        "SELECT * FROM users WHERE id = ? AND tenant_id = ?"
    )
    .bind(user_id.as_uuid())
    .bind(ctx.tenant_id().as_uuid())
    .fetch_optional(&self.pool)
    .await?;

    // Additional validation
    if let Some(ref u) = user {
        if u.tenant_id != ctx.tenant_id().as_uuid() {
            return Err(DatabaseError::TenantIsolationViolation {
                entity_type: "User",
                entity_id: user_id.to_string(),
                requested_tenant: ctx.tenant_id().to_string(),
                actual_tenant: u.tenant_id.to_string(),
            });
        }
    }

    Ok(user)
}
```

**Files to Update**:
- [ ] `src/database/users.rs` - Add TenantContext to all user operations
- [ ] `src/database/user_oauth_tokens.rs` - Add TenantContext to token operations
- [ ] `src/database/api_keys.rs` - Add TenantContext to API key operations
- [ ] `src/database/analytics.rs` - Add TenantContext to analytics queries
- [ ] `src/database/a2a.rs` - Add TenantContext to A2A operations
- [ ] `src/database/fitness_configurations.rs` - Add TenantContext
- [ ] All callers of database methods - Thread TenantContext through

**Testing Strategy**:
- Add cross-tenant leakage tests
- Property tests with proptest for tenant isolation
- Fuzz testing for tenant_id manipulation attempts

### Task 2: Replace Global Singletons with Dependency Injection

**Goal**: Make singletons test-configurable and avoid hard-to-reset global state.

**Current Problematic Patterns**:
```rust
// src/providers/registry.rs (lines 240-250)
static GLOBAL_REGISTRY: OnceLock<Arc<RwLock<ProviderRegistry>>> = OnceLock::new();

pub fn global_registry() -> Arc<RwLock<ProviderRegistry>> {
    GLOBAL_REGISTRY
        .get_or_init(|| Arc::new(RwLock::new(ProviderRegistry::new())))
        .clone()
}
```

**Target Pattern**:
```rust
// Inject registry instead of global access
pub struct ServerResources {
    provider_registry: Arc<ProviderRegistry>,
    database: Arc<Database>,
    cache: Arc<dyn CacheBackend>,
}

impl ServerResources {
    pub fn new(
        provider_registry: Arc<ProviderRegistry>,
        database: Arc<Database>,
        cache: Arc<dyn CacheBackend>,
    ) -> Self {
        Self {
            provider_registry,
            database,
            cache,
        }
    }

    // For testing: inject custom implementations
    #[cfg(test)]
    pub fn test_default() -> Self {
        Self::new(
            Arc::new(ProviderRegistry::new()),
            Arc::new(Database::in_memory().unwrap()),
            Arc::new(InMemoryCache::new()),
        )
    }
}
```

**Files to Update**:
- [ ] `src/providers/registry.rs` - Remove GLOBAL_REGISTRY, use dependency injection
- [ ] `src/plugins/registry.rs` - Remove global_registry(), inject instead
- [ ] `src/cache/factory.rs` - Refactor cache singleton
- [ ] All route handlers - Accept injected dependencies
- [ ] Integration tests - Use injected test resources

**Benefits**:
- Testable: Each test can have its own isolated registry
- Configurable: Different configurations per tenant/environment
- Explicit dependencies: Clear what each component needs

### Task 3: Adopt thiserror Throughout Core Modules

**Goal**: Replace anyhow with domain-specific errors in all core modules.

**Pattern**:
```rust
// Keep anyhow at boundaries (HTTP handlers, CLI)
pub async fn handle_request(req: Request) -> anyhow::Result<Response> {
    let result = database_operation()
        .await
        .map_err(|e| anyhow::anyhow!("Database operation failed: {}", e))?;

    Ok(result)
}

// Use thiserror in core domain logic
pub async fn database_operation() -> DatabaseResult<Data> {
    let data = fetch_data()
        .await
        .map_err(|e| DatabaseError::QueryError {
            context: format!("Failed to fetch: {}", e)
        })?;

    Ok(data)
}
```

**Error Types to Create**:
- [ ] `src/oauth2/errors.rs` - OAuth2Error (already using custom type, migrate to thiserror)
- [ ] `src/auth/errors.rs` - AuthError
- [ ] `src/cache/errors.rs` - CacheError
- [ ] `src/intelligence/errors.rs` - IntelligenceError
- [ ] `src/protocols/errors.rs` - ProtocolError (already exists, verify thiserror usage)

**Migration Strategy**:
1. Create new error type with thiserror
2. Add `From<anyhow::Error>` impl for compatibility
3. Update module internals to use new error type
4. Update public API to return new error type
5. Update callers to handle specific error variants

## 📊 Estimated Impact

### Before (Current: 8.5/10)
- ✅ OAuth2 hardening complete
- ✅ Zero unsafe code
- ⚠️ Tenant isolation at app layer only
- ⚠️ Global singletons hard to test
- ⚠️ anyhow everywhere loses error context

### After (Target: 9.5/10)
- ✅ OAuth2 hardening complete
- ✅ Zero unsafe code
- ✅ Tenant isolation enforced at database layer
- ✅ Dependency injection throughout
- ✅ Rich error types with context

## 🎯 Prioritization

### High Priority (Production Blockers)
1. **Tenant-enforced DB layer** - Prevents data leakage
   - Estimated effort: 2-3 weeks
   - Risk: High (multi-tenant security)

### Medium Priority (Quality Improvements)
2. **Replace global singletons** - Improves testability
   - Estimated effort: 1 week
   - Risk: Medium (refactor existing code)

3. **thiserror adoption** - Better error handling
   - Estimated effort: 1 week
   - Risk: Low (backwards compatible with anyhow)

## 📝 Implementation Notes

### Tenant Context Threading Pattern
```rust
// Extract TenantContext from JWT in middleware
pub async fn extract_tenant_context(
    req: &Request,
    auth_manager: &AuthManager,
) -> Result<TenantContext, AuthError> {
    let token = extract_bearer_token(req)?;
    let claims = auth_manager.validate_token(token)?;

    Ok(TenantContext::new(
        TenantId::parse_str(&claims.tenant_id)?,
        UserId::parse_str(&claims.sub)?,
    ))
}

// Thread through request handlers
pub async fn get_user_handler(
    ctx: TenantContext,
    db: Arc<Database>,
    user_id: UserId,
) -> Result<Response> {
    let user = db.get_user(&ctx, user_id).await?;
    Ok(json_response(user))
}
```

### Testing Strategy
- Add `#[cfg(test)]` helper to create TenantContext
- Property tests with multiple tenants
- Integration tests with cross-tenant access attempts
- Benchmark tenant filtering overhead

## 🔗 Related Documentation
- [Code Review Summary](./docs/CODE_REVIEW_SUMMARY.md)
- [Security Compliance](./docs/mcp-security-compliance.md)
- [Architecture Overview](./docs/developer-guide/01_architecture_overview.md)
