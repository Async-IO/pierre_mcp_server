<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 28: Tenant Admin APIs & Fitness Configuration

This appendix explains Pierre's tenant administration HTTP APIs and how tenant-scoped fitness configurations are managed. You'll see how tenants, OAuth apps, and fitness configs are modeled and exposed via REST routes.

## What You'll Learn

- Tenant creation and listing APIs (`src/tenant_routes.rs`)
- Tenant OAuth credential management
- OAuth app registration for MCP clients
- Tenant-scoped fitness configurations (`src/fitness_configuration_routes.rs`)
- How these APIs relate to earlier multi-tenant and OAuth chapters

## Tenant Management APIs

Tenants represent logical customers or organizations in the Pierre platform. The tenant routes provide CRUD-style operations for tenants and their OAuth settings.

### Creating Tenants

**Source**: src/tenant_routes.rs:27-57
```rust
/// Request body for creating a new tenant
#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    /// Display name for the tenant
    pub name: String,
    /// URL-safe slug identifier for the tenant
    pub slug: String,
    /// Optional custom domain for the tenant
    pub domain: Option<String>,
    /// Subscription plan (basic, pro, enterprise)
    pub plan: Option<String>,
}

/// Response containing created tenant details
#[derive(Debug, Serialize)]
pub struct CreateTenantResponse {
    pub tenant_id: String,
    pub name: String,
    pub slug: String,
    pub domain: Option<String>,
    pub created_at: String,
    /// API endpoint URL for this tenant
    pub api_endpoint: String,
}
```

**Usage**: an admin-facing HTTP route accepts `CreateTenantRequest`, persists the tenant, and returns `CreateTenantResponse` with a derived API endpoint URL (e.g., `https://api.pierre.ai/t/{slug}` or custom domain).

### Listing Tenants

**Source**: src/tenant_routes.rs:59-84
```rust
/// Response containing list of tenants with pagination
#[derive(Debug, Serialize)]
pub struct TenantListResponse {
    /// List of tenant summaries
    pub tenants: Vec<TenantSummary>,
    /// Total number of tenants
    pub total_count: usize,
}

/// Summary information about a tenant
#[derive(Debug, Serialize)]
pub struct TenantSummary {
    pub tenant_id: String,
    pub name: String,
    pub slug: String,
    pub domain: Option<String>,
    pub plan: String,
    pub created_at: String,
    /// List of configured OAuth providers
    pub oauth_providers: Vec<String>,
}
```

The list endpoint returns lightweight `TenantSummary` objects, including which OAuth providers are currently configured for each tenant.

## Tenant OAuth Credential Management

Per-tenant OAuth credentials allow each tenant to bring their own Strava/Fitbit apps instead of sharing a global client ID/secret.

**Source**: src/tenant_routes.rs:86-124
```rust
/// Request to configure OAuth provider credentials for a tenant
#[derive(Debug, Deserialize)]
pub struct ConfigureTenantOAuthRequest {
    /// OAuth provider name (e.g., "strava", "fitbit")
    pub provider: String,
    /// OAuth client ID from provider
    pub client_id: String,
    /// OAuth client secret from provider
    pub client_secret: String,
    /// Redirect URI for OAuth callbacks
    pub redirect_uri: String,
    /// OAuth scopes to request
    pub scopes: Vec<String>,
    /// Optional daily rate limit
    pub rate_limit_per_day: Option<u32>,
}

/// Response after configuring OAuth provider
#[derive(Debug, Serialize)]
pub struct ConfigureTenantOAuthResponse {
    pub provider: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub configured_at: String,
}
```

**Flow**:
1. Admin calls `POST /api/tenants/{tenant_id}/oauth` with `ConfigureTenantOAuthRequest`.
2. Server validates provider, encrypts `client_secret`, and stores `TenantOAuthCredentials`.
3. Response returns non-sensitive fields (client ID, redirect URI, scopes, timestamp).
4. Later, `TenantOAuthManager` (see Chapter 16) resolves tenant-specific credentials when performing provider OAuth flows.

### Listing Tenant OAuth Providers

**Source**: src/tenant_routes.rs:126-161
```rust
/// List of OAuth providers configured for a tenant
#[derive(Debug, Serialize)]
pub struct TenantOAuthListResponse {
    /// Configured OAuth providers
    pub providers: Vec<TenantOAuthProvider>,
}

/// OAuth provider configuration details
#[derive(Debug, Serialize)]
pub struct TenantOAuthProvider {
    pub provider: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub configured_at: String,
    pub enabled: bool,
}
```

This view powers an admin UI where operators can confirm which providers are active per tenant, rotate credentials, or temporarily disable a misconfigured provider.

## OAuth App Registration for MCP Clients

Beyond provider OAuth, Pierre exposes an OAuth server (Chapter 15) for MCP clients themselves. Tenant routes provide a convenience wrapper to register OAuth apps.

**Source**: src/tenant_routes.rs:163-205
```rust
/// Request to register a new OAuth application
#[derive(Debug, Deserialize)]
pub struct RegisterOAuthAppRequest {
    /// Application name
    pub name: String,
    /// Optional application description
    pub description: Option<String>,
    /// Allowed redirect URIs for OAuth callbacks
    pub redirect_uris: Vec<String>,
    /// Requested OAuth scopes (e.g., mcp:read, mcp:write, a2a:read)
    pub scopes: Vec<String>,
    /// Application type (desktop, web, mobile, server)
    pub app_type: String,
}

/// Response containing registered OAuth application credentials
#[derive(Debug, Serialize)]
pub struct RegisterOAuthAppResponse {
    pub client_id: String,
    pub client_secret: String,
    pub name: String,
    pub app_type: String,
    pub authorization_url: String,
    pub token_url: String,
    pub created_at: String,
}
```

**Pattern**: tenants can programmatically register OAuth clients to integrate their own MCP tooling with Pierre, receiving a `client_id`/`client_secret` and the relevant auth/token endpoints.

## Fitness Configuration APIs

The fitness configuration routes expose tenant- and user-scoped configuration blobs used by the intelligence layer (e.g., thresholds, algorithm choices, personalized presets).

### Models

**Source**: src/fitness_configuration_routes.rs:15-64
```rust
/// Request to save fitness configuration
#[derive(Debug, Deserialize)]
pub struct SaveFitnessConfigRequest {
    /// Configuration name (defaults to "default")
    pub configuration_name: Option<String>,
    /// Fitness configuration data
    pub configuration: FitnessConfig,
}

/// Response containing fitness configuration details
#[derive(Debug, Serialize)]
pub struct FitnessConfigurationResponse {
    pub id: String,
    pub tenant_id: String,
    pub user_id: Option<String>,
    pub configuration_name: String,
    pub configuration: FitnessConfig,
    pub created_at: String,
    pub updated_at: String,
    pub metadata: ResponseMetadata,
}

/// Response containing list of available fitness configurations
#[derive(Debug, Serialize)]
pub struct FitnessConfigurationListResponse {
    pub configurations: Vec<String>,
    pub total_count: usize,
    pub metadata: ResponseMetadata,
}
```

`FitnessConfig` (from `crate::config::fitness_config`) holds the actual structured configuration (zones, algorithm selection enums, etc.), while the routes add multi-tenant context and standard response metadata.

### Listing Configurations

**Source**: src/fitness_configuration_routes.rs:90-141
```rust
/// Fitness configuration routes handler
#[derive(Clone)]
pub struct FitnessConfigurationRoutes {
    resources: Arc<crate::mcp::resources::ServerResources>,
}

impl FitnessConfigurationRoutes {
    /// GET /api/fitness-configurations - List all configuration names for user
    pub async fn list_configurations(
        &self,
        auth: &AuthResult,
    ) -> AppResult<FitnessConfigurationListResponse> {
        let processing_start = std::time::Instant::now();
        let user_id = auth.user_id;
        let tenant_id = self.get_user_tenant(user_id).await?;

        let tenant_id_str = tenant_id.to_string();
        let user_id_str = user_id.to_string();

        // Get both user-specific and tenant-level configurations
        let mut configurations = self
            .resources
            .database
            .list_user_fitness_configurations(&tenant_id_str, &user_id_str)
            .await?;

        let tenant_configs = self
            .resources
            .database
            .list_tenant_fitness_configurations(&tenant_id_str)
            .await?;

        configurations.extend(tenant_configs);
        configurations.sort();
        configurations.dedup();

        Ok(FitnessConfigurationListResponse {
            total_count: configurations.len(),
            configurations,
            metadata: Self::create_metadata(processing_start),
        })
    }
}
```

**Key detail**: the list endpoint merges user-specific and tenant-level configs, deduplicates them, and returns a simple list of names. This mirrors how the MCP tools can resolve configuration precedence (user overrides tenant defaults).

### Resolving Tenant Context

`get_user_tenant` extracts the tenant ID from the authenticated user.

**Source**: src/fitness_configuration_routes.rs:66-88
```rust
async fn get_user_tenant(&self, user_id: Uuid) -> AppResult<Uuid> {
    let user = self
        .resources
        .database
        .get_user(user_id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("User {user_id}")))?;

    let tenant_id = user
        .tenant_id
        .as_ref()
        .and_then(|id| Uuid::parse_str(id).ok())
        .ok_or_else(||
            AppError::invalid_input(format!("User has no valid tenant: {user_id}"))
        )?;

    Ok(tenant_id)
}
```

This helper is reused across fitness configuration handlers to ensure every configuration is bound to the correct tenant.

## Relationship to Earlier Chapters

- **Chapter 7 (multi-tenant isolation)**: Covered database-level tenant separation; here you see the **HTTP admin surface** for managing tenants.
- **Chapters 15–16 (OAuth server & client)**: Explained OAuth protocols; tenant routes add **per-tenant OAuth credentials and app registration**.
- **Chapter 19 (tools guide)**: Configuration tools like `get_fitness_config` and `set_fitness_config` ultimately call into these REST routes under the hood (directly or via internal services).

## Key Takeaways

1. **Tenants**: Represent customers, each with their own slug, domain, plan, and OAuth configuration.
2. **Tenant OAuth**: `ConfigureTenantOAuthRequest` binds provider credentials to a tenant, enabling "bring your own app" flows.
3. **OAuth apps**: Tenants can register OAuth clients for integrating external MCP tooling with Pierre.
4. **Fitness configs**: Tenant- and user-scoped fitness configurations are stored via dedicated REST routes and used by intelligence algorithms.
5. **Precedence**: User configs override tenant defaults, but both are visible via `list_configurations`.
6. **Admin APIs**: These HTTP routes are the operational surface for SaaS administrators and automation tools.
