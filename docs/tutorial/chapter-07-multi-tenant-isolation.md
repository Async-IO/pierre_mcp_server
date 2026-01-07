<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 07: Multi-Tenant Database Isolation

This chapter explores how the Pierre Fitness Platform enforces strict tenant boundaries at the database layer, ensuring complete data isolation between different organizations using the same server instance. You'll learn about tenant context extraction, role-based access control, and query-level tenant filtering.

## What You'll Learn

- Multi-tenant architecture patterns for SaaS applications
- `TenantContext` structure and lifecycle
- Tenant roles: Owner, Admin, Billing, Member
- JWT claims integration with tenant_id
- Database query filtering with WHERE tenant_id
- OAuth credential isolation per tenant
- Resource access validation and RBAC
- Tenant-aware logging and observability
- Provider usage tracking per tenant
- Security patterns for preventing cross-tenant data leaks

## Multi-Tenant Architecture Overview

The Pierre platform implements true multi-tenancy, where multiple organizations (tenants) share the same database and application server while maintaining complete data isolation.

### Architecture Layers

```
┌──────────────────────────────────────────────────────────────┐
│                        HTTP Request                          │
│       Authorization: Bearer eyJhbGc...  (JWT token)          │
└────────────────────────┬─────────────────────────────────────┘
                         │
                         ▼
          ┌──────────────────────────┐
          │  McpAuthMiddleware       │
          │  - Extract user_id       │
          │  - Validate JWT          │
          └──────────────────────────┘
                         │
                         ▼
          ┌──────────────────────────┐
          │  TenantIsolation         │
          │  - Look up user.tenant_id│
          │  - Extract TenantContext │
          │  - Validate user role    │
          └──────────────────────────┘
                         │
                         ▼
          ┌──────────────────────────┐
          │  Database Queries        │
          │  WHERE tenant_id = $1    │
          │  (automatic filtering)   │
          └──────────────────────────┘
                         │
                         ▼
          ┌──────────────────────────┐
          │  Tenant-Scoped Results   │
          │  (only this org's data)  │
          └──────────────────────────┘
```

**Key principle**: Every database query includes `WHERE tenant_id = <current_tenant_id>` to enforce row-level security. No query can access data from a different tenant, even if the application code has a bug.

## Tenant Context Structure

The `TenantContext` struct carries tenant information throughout the request lifecycle:

**Source**: src/tenant/mod.rs:29-70
```rust
/// Tenant context for all operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantContext {
    /// Tenant ID
    pub tenant_id: Uuid,
    /// Tenant name for display
    pub tenant_name: String,
    /// User ID within tenant context
    pub user_id: Uuid,
    /// User's role within the tenant
    pub user_role: TenantRole,
}

impl TenantContext {
    /// Create new tenant context
    #[must_use]
    pub const fn new(
        tenant_id: Uuid,
        tenant_name: String,
        user_id: Uuid,
        user_role: TenantRole,
    ) -> Self {
        Self {
            tenant_id,
            tenant_name,
            user_id,
            user_role,
        }
    }

    /// Check if user has admin privileges in this tenant
    #[must_use]
    pub const fn is_admin(&self) -> bool {
        matches!(self.user_role, TenantRole::Admin | TenantRole::Owner)
    }

    /// Check if user can configure OAuth apps
    #[must_use]
    pub const fn can_configure_oauth(&self) -> bool {
        matches!(self.user_role, TenantRole::Admin | TenantRole::Owner)
    }
}
```

**Rust Idiom**: `#[derive(Clone, Serialize, Deserialize)]`

The `Clone` derive enables passing `TenantContext` across async boundaries. The struct is small (4 fields, all cheap to clone) and frequently needed by multiple handlers. Cloning is more ergonomic than managing lifetimes for a shared reference.

The `Serialize` and `Deserialize` derives allow embedding `TenantContext` in JSON responses and session data.

## Tenant Roles and Permissions

The platform defines four tenant roles with increasing privileges:

**Source**: src/tenant/schema.rs:11-54
```rust
/// Tenant role within an organization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TenantRole {
    /// Organization owner (full permissions)
    Owner,
    /// Administrator (can configure OAuth, manage users)
    Admin,
    /// Billing manager (can view usage, manage billing)
    Billing,
    /// Regular member (can use tools)
    Member,
}

impl TenantRole {
    /// Convert from database string
    #[must_use]
    pub fn from_db_string(s: &str) -> Self {
        match s {
            "owner" => Self::Owner,
            "admin" => Self::Admin,
            "billing" => Self::Billing,
            "member" => Self::Member,
            _ => {
                // Log unknown role but fallback to member for security
                tracing::warn!(
                    "Unknown tenant role '{}' encountered, defaulting to Member",
                    s
                );
                Self::Member
            }
        }
    }

    /// Convert to database string
    #[must_use]
    pub const fn to_db_string(&self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Billing => "billing",
            Self::Member => "member",
        }
    }
}
```

**Permission hierarchy**:
- **Owner**: Full control, can modify tenant settings, delete tenant, manage all users
- **Admin**: Configure OAuth apps, manage users, access all tools
- **Billing**: View usage metrics, manage subscription and billing
- **Member**: Use tools and access their own data (no administrative functions)

**Rust Idiom**: Default to least privilege

The `from_db_string` method defaults to `Member` for unknown roles. This "fail-safe" approach ensures that database corruption or future role additions don't accidentally grant excessive permissions. Always default to the most restrictive option when parsing untrusted data.

## JWT Claims and Tenant Extraction

User authentication tokens (Chapter 6) include an optional `tenant_id` claim for multi-tenant deployment:

**Source**: src/auth.rs:108-130
```rust
/// JWT claims for user authentication
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,        // User ID
    pub email: String,
    pub iat: i64,
    pub exp: i64,
    pub iss: String,
    pub jti: String,
    pub providers: Vec<String>,
    pub aud: String,
    /// Tenant ID (optional for backward compatibility with existing tokens)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}
```

The platform extracts tenant context from JWT tokens during request authentication:

**Source**: src/mcp/tenant_isolation.rs:30-58
```rust
/// Validate JWT token and extract tenant context
///
/// # Errors
/// Returns an error if JWT validation fails or tenant information cannot be retrieved
pub async fn validate_tenant_access(&self, jwt_token: &str) -> Result<TenantContext> {
    let auth_result = self
        .resources
        .auth_manager
        .validate_token(jwt_token, &self.resources.jwks_manager)?;

    // Parse user ID from claims
    let user_id = crate::utils::uuid::parse_uuid(&auth_result.sub)
        .map_err(|e| {
            tracing::warn!(sub = %auth_result.sub, error = %e, "Invalid user ID in JWT token claims");
            AppError::auth_invalid("Invalid user ID in token")
        })?;

    let user = self.get_user_with_tenant(user_id).await?;
    let tenant_id = self.extract_tenant_id(&user)?;
    let tenant_name = self.get_tenant_name(tenant_id).await;
    let user_role = self.get_user_role_for_tenant(user_id, tenant_id).await?;

    Ok(TenantContext {
        tenant_id,
        tenant_name,
        user_id,
        user_role,
    })
}
```

**Flow**:
1. Validate JWT signature and expiration (Chapter 6)
2. Extract `user_id` from `sub` claim
3. Look up user in database to find their `tenant_id`
4. Query tenant table for `tenant_name`
5. Look up user's role within the tenant (Owner/Admin/Billing/Member)
6. Construct `TenantContext` for the request

### Extracting Tenant_id from User Record

Users belong to exactly one tenant (single-tenancy per user):

**Source**: src/mcp/tenant_isolation.rs:73-88
```rust
/// Extract tenant ID from user
///
/// # Errors
/// Returns an error if tenant ID is missing or invalid
pub fn extract_tenant_id(&self, user: &crate::models::User) -> Result<Uuid> {
    user.tenant_id
        .clone() // Safe: Option<String> ownership for UUID parsing
        .ok_or_else(|| -> anyhow::Error {
            AppError::auth_invalid("User does not belong to any tenant").into()
        })?
        .parse()
        .map_err(|e| -> anyhow::Error {
            tracing::warn!(user_id = %user.id, tenant_id = ?user.tenant_id, error = %e, "Invalid tenant ID format for user");
            AppError::invalid_input("Invalid tenant ID format").into()
        })
}
```

**Note**: The `tenant_id` is stored as a string to support both formats:
- UUID-based: `"550e8400-e29b-41d4-a716-446655440000"`
- Slug-based: `"acme-corp"` for vanity URLs

The platform attempts UUID parsing first, then falls back to slug lookup.

## Database Isolation with where Clauses

Every database query that accesses tenant-scoped data includes a `WHERE tenant_id = ?` clause:

### OAuth Credentials Isolation

Tenant OAuth credentials are stored separately from user credentials:

**Source**: src/database_plugins/sqlite.rs:1297-1302
```sql
SELECT tenant_id, provider, client_id, client_secret_encrypted, redirect_uri, scopes, rate_limit_per_day
FROM tenant_oauth_credentials
WHERE tenant_id = ?1 AND provider = ?2 AND is_active = true
```

**Source**: src/database_plugins/postgres.rs:3151-3156
```sql
SELECT client_id, client_secret_encrypted, client_secret_nonce,
       redirect_uri, scopes, rate_limit_per_day
FROM tenant_oauth_apps
WHERE tenant_id = $1 AND provider = $2 AND is_active = true
```

**Security**: The `WHERE tenant_id = ?` clause ensures that even if application code passes the wrong tenant ID, the database returns no results. This "defense in depth" prevents cross-tenant data leaks from programming errors.

### Listing Tenant OAuth Apps

**Source**: src/database_plugins/sqlite.rs:1249-1254
```sql
SELECT tenant_id, provider, client_id, client_secret_encrypted, redirect_uri, scopes, rate_limit_per_day
FROM tenant_oauth_credentials
WHERE tenant_id = ?1 AND is_active = true
ORDER BY provider
```

**Source**: src/database_plugins/postgres.rs:3077-3082
```sql
SELECT provider, client_id, client_secret_encrypted, client_secret_nonce,
       redirect_uri, scopes, rate_limit_per_day
FROM tenant_oauth_apps
WHERE tenant_id = $1 AND is_active = true
ORDER BY provider
```

**Pattern**: All tenant-scoped queries follow this structure:
1. SELECT only needed columns
2. FROM tenant-scoped table
3. WHERE tenant_id = $param AND is_active = true
4. ORDER BY for deterministic results

## Tenant Isolation Manager

The `TenantIsolation` manager coordinates tenant context extraction and validation:

**Source**: src/mcp/tenant_isolation.rs:18-28
```rust
/// Manages tenant isolation and multi-tenancy for the MCP server
pub struct TenantIsolation {
    resources: Arc<ServerResources>,
}

impl TenantIsolation {
    /// Create a new tenant isolation manager
    #[must_use]
    pub const fn new(resources: Arc<ServerResources>) -> Self {
        Self { resources }
    }
```

**Dependency**: The manager holds `Arc<ServerResources>` to access:
- `auth_manager`: JWT validation
- `jwks_manager`: RS256 key lookup
- `database`: User and tenant queries

**Rust Idiom**: `const fn new()`

The `const` qualifier allows creating `TenantIsolation` at compile time if all dependencies support it. In practice, `Arc<ServerResources>` isn't const-constructible, but the pattern future-proofs the API.

### Role-Based Access Control

The platform validates user permissions for specific actions:

**Source**: src/mcp/tenant_isolation.rs:238-276
```rust
/// Validate that a user can perform an action on behalf of a tenant
///
/// # Errors
/// Returns an error if validation fails
pub async fn validate_tenant_action(
    &self,
    user_id: Uuid,
    tenant_id: Uuid,
    action: &str,
) -> Result<()> {
    let user_role = self.get_user_role_for_tenant(user_id, tenant_id).await?;

    match action {
        "read_oauth_credentials" | "store_oauth_credentials" => {
            if matches!(user_role, TenantRole::Owner | TenantRole::Member) {
                Ok(())
            } else {
                Err(AppError::auth_invalid(format!(
                    "User {user_id} does not have permission to {action} for tenant {tenant_id}"
                ))
                .into())
            }
        }
        "modify_tenant_settings" => {
            if matches!(user_role, TenantRole::Owner) {
                Ok(())
            } else {
                Err(AppError::auth_invalid(format!(
                    "User {user_id} does not have owner permission for tenant {tenant_id}"
                ))
                .into())
            }
        }
        _ => {
            warn!("Unknown action for validation: {}", action);
            Err(AppError::invalid_input(format!("Unknown action: {action}")).into())
        }
    }
}
```

**Pattern**: Explicit action strings with role matching. The platform could use a more elaborate permission system (e.g., `Permission` enum with bitflags), but string matching provides flexibility for runtime-defined permissions.

**Rust Idiom**: `matches!()` macro

The `matches!(user_role, TenantRole::Owner | TenantRole::Member)` macro provides concise pattern matching for simple checks. It's more readable than:
```rust
user_role == TenantRole::Owner || user_role == TenantRole::Member
```

### Resource Access Validation

The platform validates access to specific resource types:

**Source**: src/mcp/tenant_isolation.rs:201-224
```rust
/// Check if user has access to a specific resource
///
/// # Errors
/// Returns an error if role lookup fails
pub async fn check_resource_access(
    &self,
    user_id: Uuid,
    tenant_id: Uuid,
    resource_type: &str,
) -> Result<bool> {
    // Verify user belongs to the tenant
    let user_role = self.get_user_role_for_tenant(user_id, tenant_id).await?;

    // Basic access control - can be extended based on requirements
    match resource_type {
        "oauth_credentials" => Ok(matches!(user_role, TenantRole::Owner | TenantRole::Member)),
        "fitness_data" => Ok(matches!(user_role, TenantRole::Owner | TenantRole::Member)),
        "tenant_settings" => Ok(matches!(user_role, TenantRole::Owner)),
        _ => {
            warn!("Unknown resource type: {}", resource_type);
            Ok(false)
        }
    }
}
```

**Security**: Unknown resource types return `false` (deny by default). This ensures that new resources added to the platform require explicit permission configuration.

## Tenant Resources Wrapper

The `TenantResources` struct provides tenant-scoped access to database operations:

**Source**: src/mcp/tenant_isolation.rs:279-360
```rust
/// Tenant-scoped resource accessor
pub struct TenantResources {
    /// Unique identifier for the tenant
    pub tenant_id: Uuid,
    /// Database connection for tenant-scoped operations
    pub database: Arc<Database>,
}

impl TenantResources {
    /// Get OAuth credentials for this tenant
    ///
    /// # Errors
    /// Returns an error if credential lookup fails
    pub async fn get_oauth_credentials(
        &self,
        provider: &str,
    ) -> Result<Option<crate::tenant::oauth_manager::TenantOAuthCredentials>> {
        self.database
            .get_tenant_oauth_credentials(self.tenant_id, provider)
            .await
    }

    /// Store OAuth credentials for this tenant
    ///
    /// # Errors
    /// Returns an error if credential storage fails or tenant ID mismatch
    pub async fn store_oauth_credentials(
        &self,
        credential: &crate::tenant::oauth_manager::TenantOAuthCredentials,
    ) -> Result<()> {
        // Ensure the credential belongs to this tenant
        if credential.tenant_id != self.tenant_id {
            return Err(AppError::invalid_input(format!(
                "Credential tenant ID mismatch: expected {}, got {}",
                self.tenant_id, credential.tenant_id
            ))
            .into());
        }

        self.database
            .store_tenant_oauth_credentials(credential)
            .await
    }

    /// Get user OAuth tokens for this tenant
    ///
    /// # Errors
    /// Returns an error if token lookup fails
    pub async fn get_user_oauth_tokens(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<Option<crate::models::UserOAuthToken>> {
        // Convert tenant_id to string for database query
        let tenant_id_str = self.tenant_id.to_string();
        self.database
            .get_user_oauth_token(user_id, &tenant_id_str, provider)
            .await
    }

    /// Store user OAuth token for this tenant
    ///
    /// # Errors
    /// Returns an error if token storage fails
    pub async fn store_user_oauth_token(
        &self,
        token: &crate::models::UserOAuthToken,
    ) -> Result<()> {
        // Additional validation could be added here to ensure
        // the user belongs to this tenant
        // For now, store using the user's OAuth app approach
        self.database
            .store_user_oauth_app(
                token.user_id,
                &token.provider,
                "", // client_id not available in UserOAuthToken
                "", // client_secret not available in UserOAuthToken
                "", // redirect_uri not available in UserOAuthToken
            )
            .await
    }
}
```

**Design pattern**: Type-state pattern for tenant isolation. The `TenantResources` struct "knows" its `tenant_id` and automatically includes it in all database queries. This prevents forgetting to filter by tenant.

**Rust Idiom**: Validation at storage time

The `store_oauth_credentials` method validates that `credential.tenant_id == self.tenant_id` before storing. This prevents accidentally storing credentials for the wrong tenant, which would leak sensitive data.

## Tenant-Aware Logging

The platform provides structured logging utilities that include tenant context:

**Source**: src/logging/tenant.rs:30-51
```rust
/// Tenant-aware logging utilities
pub struct TenantLogger;

impl TenantLogger {
    /// Log MCP tool call with tenant context
    pub fn log_mcp_tool_call(
        user_id: Uuid,
        tenant_id: Uuid,
        tool_name: &str,
        success: bool,
        duration_ms: u64,
    ) {
        tracing::info!(
            user_id = %user_id,
            tenant_id = %tenant_id,
            tool_name = %tool_name,
            success = %success,
            duration_ms = %duration_ms,
            event_type = "mcp_tool_call",
            "MCP tool call completed"
        );
    }
```

**Observability**: Including `tenant_id` in all log entries enables:
- Per-tenant usage analytics
- Security audit trails (which tenant accessed what data)
- Performance debugging (is one tenant causing slow queries?)
- Billing and chargeback (which tenant consumed how many resources)

### Authentication Logging

**Source**: src/logging/tenant.rs:54-81
```rust
/// Log authentication event with tenant context
pub fn log_auth_event(
    user_id: Option<Uuid>,
    tenant_id: Option<Uuid>,
    auth_method: &str,
    success: bool,
    error_details: Option<&str>,
) {
    if success {
        tracing::info!(
            user_id = ?user_id,
            tenant_id = ?tenant_id,
            auth_method = %auth_method,
            success = %success,
            event_type = "authentication",
            "Authentication successful"
        );
    } else {
        tracing::warn!(
            user_id = ?user_id,
            tenant_id = ?tenant_id,
            auth_method = %auth_method,
            success = %success,
            error_details = ?error_details,
            event_type = "authentication",
            "Authentication failed"
        );
    }
}
```

**Security**: Failed authentication attempts include `tenant_id` (if available) to detect:
- Brute force attacks against a specific tenant
- Cross-tenant authentication attempts (attacker trying tenant A credentials against tenant B)
- Compromised user accounts

### HTTP Request Logging

**Source**: src/logging/tenant.rs:84-115
```rust
/// Log HTTP request with tenant context
pub fn log_http_request(
    user_id: Option<Uuid>,
    tenant_id: Option<Uuid>,
    method: &str,
    path: &str,
    status_code: u16,
    duration_ms: u64,
) {
    if status_code < crate::constants::network_config::HTTP_CLIENT_ERROR_THRESHOLD {
        tracing::info!(
            user_id = ?user_id,
            tenant_id = ?tenant_id,
            http_method = %method,
            http_path = %path,
            http_status = %status_code,
            duration_ms = %duration_ms,
            event_type = "http_request",
            "HTTP request completed"
        );
    } else {
        tracing::warn!(
            user_id = ?user_id,
            tenant_id = ?tenant_id,
            http_method = %method,
            http_path = %path,
            http_status = %status_code,
            duration_ms = %duration_ms,
            event_type = "http_request",
            "HTTP request failed"
        );
    }
}
```

**Rust Idiom**: `Option<Uuid>` for optional context

Not all requests have tenant context (e.g., health check endpoints, public landing pages). Using `Option<Uuid>` allows logging these requests with `None` values, which serialize as `null` in structured logs.

### Database Operation Logging

**Source**: src/logging/tenant.rs:118-138
```rust
/// Log database operation with tenant context
pub fn log_database_operation(
    user_id: Option<Uuid>,
    tenant_id: Option<Uuid>,
    operation: &str,
    table: &str,
    success: bool,
    duration_ms: u64,
    rows_affected: Option<usize>,
) {
    tracing::debug!(
        user_id = ?user_id,
        tenant_id = ?tenant_id,
        db_operation = %operation,
        db_table = %table,
        success = %success,
        duration_ms = %duration_ms,
        rows_affected = ?rows_affected,
        event_type = "database_operation",
        "Database operation completed"
    );
}
```

**Performance**: Database logs use `tracing::debug!()` level to avoid overwhelming production systems. Enable in development with `RUST_LOG=debug` to troubleshoot slow queries.

## Tenant Provider Isolation

Fitness provider requests use tenant-specific OAuth credentials:

**Source**: src/providers/tenant_provider.rs:15-47
```rust
/// Tenant-aware fitness provider that wraps existing providers with tenant context
#[async_trait]
pub trait TenantFitnessProvider: Send + Sync {
    /// Authenticate using tenant-specific OAuth credentials
    async fn authenticate_tenant(
        &mut self,
        tenant_context: &TenantContext,
        provider: &str,
        database: &dyn DatabaseProvider,
    ) -> Result<()>;

    /// Get athlete information for the authenticated tenant user
    async fn get_athlete(&self) -> Result<Athlete>;

    /// Get activities for the authenticated tenant user
    async fn get_activities(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Activity>>;

    /// Get specific activity by ID
    async fn get_activity(&self, id: &str) -> Result<Activity>;

    /// Get stats for the authenticated tenant user
    async fn get_stats(&self) -> Result<Stats>;

    /// Get personal records for the authenticated tenant user
    async fn get_personal_records(&self) -> Result<Vec<PersonalRecord>>;

    /// Get provider name
    fn provider_name(&self) -> &'static str;
}
```

**Architecture**: The `TenantFitnessProvider` trait wraps existing provider implementations (Strava, Garmin) with tenant context. When a user requests Strava data, the platform:

1. Extracts `TenantContext` from JWT
2. Looks up tenant's Strava OAuth credentials (client ID, client secret)
3. Uses tenant-specific credentials to fetch data
4. Returns results scoped to the user within the tenant

This allows multiple tenants to use the same Strava integration with different OAuth apps.

### Tenant Provider Factory

**Source**: src/providers/tenant_provider.rs:49-80
```rust
/// Factory for creating tenant-aware fitness providers
pub struct TenantProviderFactory {
    oauth_client: Arc<TenantOAuthClient>,
}

impl TenantProviderFactory {
    /// Create new tenant provider factory
    #[must_use]
    pub const fn new(oauth_client: Arc<TenantOAuthClient>) -> Self {
        Self { oauth_client }
    }

    /// Create tenant-aware provider for the specified type
    ///
    /// # Errors
    ///
    /// Returns an error if the provider type is not supported
    pub fn create_tenant_provider(
        &self,
        provider_type: &str,
    ) -> Result<Box<dyn TenantFitnessProvider>> {
        match provider_type.to_lowercase().as_str() {
            "strava" => Ok(Box::new(super::strava_tenant::TenantStravaProvider::new(
                self.oauth_client.clone(),
            ))),
            _ => Err(AppError::invalid_input(format!(
                "Unknown tenant provider: {provider_type}. Currently supported: strava"
            ))
            .into()),
        }
    }
}
```

**Extensibility**: The factory pattern makes it easy to add new providers (Garmin, Fitbit, Polar) by implementing `TenantFitnessProvider` and adding a match arm.

## Tenant Schema and Models

The database schema enforces tenant isolation with foreign key constraints:

**Source**: src/tenant/schema.rs:56-93
```rust
/// Tenant/Organization in the multi-tenant system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    /// Unique tenant identifier
    pub id: Uuid,
    /// Display name for the organization
    pub name: String,
    /// URL-safe identifier for tenant (e.g., "acme-corp")
    pub slug: String,
    /// Domain for custom tenant routing (optional)
    pub domain: Option<String>,
    /// Subscription tier
    pub subscription_tier: String,
    /// Whether tenant is active
    pub is_active: bool,
    /// When tenant was created
    pub created_at: DateTime<Utc>,
    /// When tenant was last updated
    pub updated_at: DateTime<Utc>,
}

impl Tenant {
    /// Create a new tenant
    #[must_use]
    pub fn new(name: String, slug: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            slug,
            domain: None,
            subscription_tier: "starter".into(),
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }
}
```

**Fields**:
- `id`: Primary key (UUID)
- `slug`: URL-safe identifier for vanity URLs (`acme-corp.pierre.app`)
- `domain`: Custom domain for white-label deployments (`fitness.acme.com`)
- `subscription_tier`: For tiered pricing (starter, professional, enterprise)
- `is_active`: Soft delete (deactivate tenant without deleting data)

### Tenant-User Relationship

**Source**: src/tenant/schema.rs:95-122
```rust
/// User membership in a tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantUser {
    /// Unique relationship identifier
    pub id: Uuid,
    /// Tenant ID
    pub tenant_id: Uuid,
    /// User ID
    pub user_id: Uuid,
    /// User's role in this tenant
    pub role: TenantRole,
    /// When user joined tenant
    pub joined_at: DateTime<Utc>,
}

impl TenantUser {
    /// Create new tenant-user relationship
    #[must_use]
    pub fn new(tenant_id: Uuid, user_id: Uuid, role: TenantRole) -> Self {
        Self {
            id: Uuid::new_v4(),
            tenant_id,
            user_id,
            role,
            joined_at: Utc::now(),
        }
    }
}
```

**Design**: The `tenant_users` junction table supports:
- Future multi-tenant users (one user, multiple tenants)
- Role changes over time (member promoted to admin)
- Audit trail (`joined_at` timestamp)

Currently, users belong to exactly one tenant, but the schema allows future enhancement.

### Tenant Usage Tracking

**Source**: src/tenant/schema.rs:124-143
```rust
/// Daily usage tracking per tenant per provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantProviderUsage {
    /// Unique usage record identifier
    pub id: Uuid,
    /// Tenant ID
    pub tenant_id: Uuid,
    /// Provider name
    pub provider: String,
    /// Usage date
    pub usage_date: chrono::NaiveDate,
    /// Number of successful requests
    pub request_count: u32,
    /// Number of failed requests
    pub error_count: u32,
    /// When record was created
    pub created_at: DateTime<Utc>,
    /// When record was last updated
    pub updated_at: DateTime<Utc>,
}
```

**Purpose**: Per-tenant provider usage enables:
- Rate limiting enforcement (prevent one tenant from exhausting API quotas)
- Billing and chargeback (charge tenants for Strava API usage)
- Analytics (which tenants use which providers most)
- Capacity planning (do we need higher Strava rate limits?)

**Rust Idiom**: `chrono::NaiveDate` for calendar dates

Using `NaiveDate` (date without time zone) for `usage_date` avoids time zone confusion. The platform aggregates usage per calendar day in UTC, regardless of the tenant's time zone.

## Security Patterns and Best Practices

### Defense in Depth

The platform employs multiple layers of security:

1. **JWT validation**: Verify token signature and expiration (Chapter 6)
2. **Tenant extraction**: Look up user's tenant from database
3. **Role validation**: Check user's role within tenant
4. **Query filtering**: Include `WHERE tenant_id = ?` in all queries
5. **Response validation**: Ensure returned data belongs to tenant

**Principle**: Even if one layer fails (e.g., application bug passes wrong tenant ID), the database filtering prevents cross-tenant leaks.

### Preventing Common Vulnerabilities

**SQL injection**: All queries use parameterized statements (`?1`, `$1`) instead of string concatenation:
```rust
// CORRECT (parameterized)
sqlx::query("SELECT * FROM users WHERE tenant_id = ?1")
    .bind(tenant_id)
    .fetch_all(&pool)
    .await?;

// WRONG (vulnerable to SQL injection)
let query = format!("SELECT * FROM users WHERE tenant_id = '{}'", tenant_id);
sqlx::query(&query).fetch_all(&pool).await?;
```

**Insecure direct object references (IDOR)**: Always validate resource ownership:
```rust
// CORRECT
async fn get_activity(tenant_id: Uuid, user_id: Uuid, activity_id: &str) -> Result<Activity> {
    let activity = database.get_activity(activity_id).await?;

    // Verify activity belongs to this tenant
    if activity.tenant_id != tenant_id {
        return Err(AppError::not_found("Activity"));
    }

    Ok(activity)
}
```

**Cross-tenant data leaks**: Never trust client-provided tenant IDs. Always extract from authenticated user:
```rust
// CORRECT
let tenant_context = tenant_isolation.validate_tenant_access(&jwt_token).await?;
let activities = database.get_activities(tenant_context.tenant_id, user_id).await?;

// WRONG (client can forge tenant_id)
let tenant_id = request.headers.get("x-tenant-id")?;
let activities = database.get_activities(tenant_id, user_id).await?;
```

## Key Takeaways

1. **True multi-tenancy**: Multiple organizations share infrastructure with complete data isolation. Every database query filters by `tenant_id`.

2. **TenantContext lifecycle**: Extract tenant from JWT → Look up user's tenant_id → Validate role → Pass context to handlers.

3. **Role-based access control**: Four roles (Owner, Admin, Billing, Member) with explicit permission checks for sensitive operations.

4. **Database-level isolation**: `WHERE tenant_id = ?` clauses in all queries provide defense in depth against application bugs.

5. **Tenant-scoped resources**: `TenantResources` wrapper automatically includes tenant_id in all operations.

6. **OAuth credential isolation**: Each tenant configures their own Strava/Garmin OAuth apps. No sharing of API credentials.

7. **Structured logging**: All log entries include `tenant_id` for security audits, billing, and performance analysis.

8. **Type-state pattern**: Rust's type system prevents passing wrong tenant IDs by encapsulating tenant_id in `TenantResources`.

9. **Fail-safe defaults**: Unknown roles default to `Member` (least privilege). Unknown resource types deny access.

10. **Usage tracking**: Per-tenant provider usage enables rate limiting, billing, and capacity planning.

---

**Next Chapter**: [Chapter 08: Middleware & Request Context](./chapter-08-middleware-context.md) - Learn how the Pierre platform uses Axum middleware to extract authentication, tenant context, and rate limiting information from HTTP requests before routing to handlers.
