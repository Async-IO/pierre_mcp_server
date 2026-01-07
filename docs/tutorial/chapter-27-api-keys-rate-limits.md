<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 27: API Keys, Rate Limiting & Real-Time Dashboard

This appendix covers Pierre's B2B API key system, unified rate limiting engine, and real-time usage dashboard/WebSocket updates. You'll learn how API keys are modeled, how quotas and bursts are enforced, and how the dashboard surfaces this information to end users.

## What You'll Learn

- API key tiers, lifecycle, and storage (`src/api_keys.rs`)
- Unified rate limiting for API keys and JWTs (`src/rate_limiting.rs`)
- Usage tracking and monthly quotas
- Real-time usage updates over WebSocket (`src/websocket.rs`)
- Dashboard overview and analytics models (`src/dashboard_routes.rs`)
- How these pieces fit together with MCP tools

## API Key Model & Tiers

Pierre exposes a B2B API via API keys that carry their own tier and quota metadata.

**Source**: src/api_keys.rs:19-75
```rust
/// API Key tiers with rate limits
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyTier {
    /// Trial tier - 1,000 requests/month, auto-expires in 14 days
    Trial,
    /// Starter tier - 10,000 requests/month
    Starter,
    /// Professional tier - 100,000 requests/month
    Professional,
    /// Enterprise tier - Unlimited requests
    Enterprise,
}

impl ApiKeyTier {
    /// Returns the monthly API request limit for this tier
    #[must_use]
    pub const fn monthly_limit(&self) -> Option<u32> {
        match self {
            Self::Trial => Some(TRIAL_MONTHLY_LIMIT),
            Self::Starter => Some(STARTER_MONTHLY_LIMIT),
            Self::Professional => Some(PROFESSIONAL_MONTHLY_LIMIT),
            Self::Enterprise => None, // Unlimited
        }
    }

    /// Returns the rate limit window duration in seconds
    #[must_use]
    pub const fn rate_limit_window(&self) -> u32 {
        RATE_LIMIT_WINDOW_SECONDS // 30 days in seconds
    }

    /// Default expiration in days for trial keys
    #[must_use]
    pub const fn default_trial_days(&self) -> Option<i64> {
        match self {
            Self::Trial => Some(TRIAL_PERIOD_DAYS),
            _ => None,
        }
    }
}
```

**Tier semantics**:
- **Trial**: 1,000 requests/month, auto-expires after `TRIAL_PERIOD_DAYS`.
- **Starter**: 10,000 requests/month (`STARTER_MONTHLY_LIMIT`).
- **Professional**: 100,000 requests/month (`PROFESSIONAL_MONTHLY_LIMIT`).
- **Enterprise**: Unlimited (`monthly_limit() -> None`).

### API Key Structure

Each API key stores its hashed value, tier, and rate limiting parameters.

**Source**: src/api_keys.rs:77-121
```rust
/// API Key model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    /// Unique identifier for the API key
    pub id: String,
    /// ID of the user who owns this key
    pub user_id: Uuid,
    /// Human-readable name for the key
    pub name: String,
    /// Visible prefix of the key for identification
    pub key_prefix: String,
    /// SHA-256 hash of the full key for verification
    pub key_hash: String,
    /// Optional description of the key's purpose
    pub description: Option<String>,
    /// Tier level determining rate limits
    pub tier: ApiKeyTier,
    /// Maximum requests allowed in the rate limit window
    pub rate_limit_requests: u32,
    /// Rate limit window duration in seconds
    pub rate_limit_window_seconds: u32,
    /// Whether the key is currently active
    pub is_active: bool,
    /// When the key was last used
    pub last_used_at: Option<DateTime<Utc>>,
    /// When the key expires (if set)
    pub expires_at: Option<DateTime<Utc>>,
    /// When the key was created
    pub created_at: DateTime<Utc>,
}
```

**Design choices**:
- **Hashed storage**: Only the SHA-256 hash is stored (`key_hash`); the full key is returned once at creation.
- **Prefix**: `key_prefix` lets the dashboard identify which key made a request without revealing the whole key.
- **Tier + limit**: `tier` encodes semantic tier (trial/starter/...), while `rate_limit_requests` stores the actual numeric limit for flexibility.

## Unified Rate Limiting Engine

The unified rate limiting engine applies the same logic to both API keys and JWT-authenticated users.

**Source**: src/rate_limiting.rs:22-60
```rust
/// Rate limit information for any authentication method
#[derive(Debug, Clone, Serialize)]
pub struct UnifiedRateLimitInfo {
    /// Whether the request is rate limited
    pub is_rate_limited: bool,
    /// Maximum requests allowed in the current period
    pub limit: Option<u32>,
    /// Remaining requests in the current period
    pub remaining: Option<u32>,
    /// When the current rate limit period resets
    pub reset_at: Option<DateTime<Utc>>,
    /// The tier associated with this rate limit
    pub tier: String,
    /// The authentication method used
    pub auth_method: String,
}
```

**Key idea**: whether a request is authenticated via API key or JWT, the **same** rate limiting structure is used, so downstream code can render consistent responses, dashboard metrics, and WebSocket updates.

### Tenant-Level Limit Tiers

Tenants have their own rate limit tiers layered on top of key-level limits.

**Source**: src/rate_limiting.rs:62-116
```rust
/// Tenant-specific rate limit tier configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantRateLimitTier {
    /// Base monthly request limit
    pub monthly_limit: u32,
    /// Requests per minute burst limit
    pub burst_limit: u32,
    /// Rate limit multiplier for this tenant (1.0 = normal, 2.0 = double)
    pub multiplier: f32,
    /// Whether tenant has unlimited requests
    pub unlimited: bool,
    /// Custom reset period in seconds (None = monthly)
    pub custom_reset_period: Option<u64>,
}

impl TenantRateLimitTier {
    /// Create tier configuration for starter tenants
    #[must_use]
    pub const fn starter() -> Self { /* ... */ }

    /// Create tier configuration for professional tenants
    #[must_use]
    pub const fn professional() -> Self { /* ... */ }

    /// Create tier configuration for enterprise tenants
    #[must_use]
    pub const fn enterprise() -> Self { /* ... */ }

    /// Apply multiplier to get effective monthly limit
    #[must_use]
    pub fn effective_monthly_limit(&self) -> u32 {
        if self.unlimited {
            u32::MAX
        } else {
            (self.monthly_limit as f32 * self.multiplier) as u32
        }
    }
}
```

**Patterns**:
- **Per-tenant limits**: SaaS plans map to `starter()`, `professional()`, `enterprise()`.
- **Multipliers**: `multiplier` allows custom boosts for specific tenants (e.g., 2× quota during migration).
- **Unlimited**: `unlimited = true` maps to `u32::MAX` effective limit.

## WebSocket Real-Time Updates

The WebSocket subsystem streams API usage and rate limit status in real time to dashboards.

**Source**: src/websocket.rs:23-74
```rust
/// WebSocket message types for real-time communication
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    /// Client authentication message
    #[serde(rename = "auth")]
    Authentication { token: String },

    /// Subscribe to specific topics
    #[serde(rename = "subscribe")]
    Subscribe { topics: Vec<String> },

    /// API key usage update notification
    #[serde(rename = "usage_update")]
    UsageUpdate {
        api_key_id: String,
        requests_today: u64,
        requests_this_month: u64,
        rate_limit_status: Value,
    },

    /// System-wide statistics update
    #[serde(rename = "system_stats")]
    SystemStats {
        total_requests_today: u64,
        total_requests_this_month: u64,
        active_connections: usize,
    },

    /// Error message to client
    #[serde(rename = "error")]
    Error { message: String },

    /// Success confirmation message
    #[serde(rename = "success")]
    Success { message: String },
}
```

**Topics**:
- `usage_update`: per-key usage and current `UnifiedRateLimitInfo` status.
- `system_stats`: aggregate metrics for all keys (e.g., for an admin dashboard).
- `auth` / `subscribe`: initial handshake; clients authenticate then opt into topics.

### WebSocket Manager

The `WebSocketManager` coordinates authentication, subscriptions, and broadcast.

**Source**: src/websocket.rs:76-115
```rust
/// Manages WebSocket connections and message broadcasting
#[derive(Clone)]
pub struct WebSocketManager {
    database: Arc<Database>,
    auth_middleware: McpAuthMiddleware,
    clients: Arc<RwLock<HashMap<Uuid, ClientConnection>>>,
    broadcast_tx: broadcast::Sender<WebSocketMessage>,
}

impl WebSocketManager {
    /// Creates a new WebSocket manager instance
    #[must_use]
    pub fn new(
        database: Arc<Database>,
        auth_manager: &Arc<AuthManager>,
        jwks_manager: &Arc<crate::admin::jwks::JwksManager>,
        rate_limit_config: crate::config::environment::RateLimitConfig,
    ) -> Self {
        let (broadcast_tx, _) =
            broadcast::channel(crate::constants::rate_limits::WEBSOCKET_CHANNEL_CAPACITY);
        let auth_middleware = McpAuthMiddleware::new(
            (**auth_manager).clone(),
            database.clone(),
            jwks_manager.clone(),
            rate_limit_config,
        );

        Self {
            database,
            auth_middleware,
            clients: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
        }
    }
}
```

**Flow**:
1. Client connects to WebSocket endpoint and sends `Authentication { token }`.
2. `WebSocketManager` verifies token via `McpAuthMiddleware`.
3. Client sends `Subscribe { topics }` (e.g., `"usage_update"`, `"system_stats"`).
4. Server periodically pushes `UsageUpdate` and `SystemStats` messages.

## Dashboard Overview & Analytics

The dashboard HTTP routes expose human-friendly analytics built on top of usage and rate limiting data.

**Source**: src/dashboard_routes.rs:16-73
```rust
/// Dashboard overview with key metrics and recent activity
#[derive(Debug, Serialize)]
pub struct DashboardOverview {
    pub total_api_keys: u32,
    pub active_api_keys: u32,
    pub total_requests_today: u64,
    pub total_requests_this_month: u64,
    pub current_month_usage_by_tier: Vec<TierUsage>,
    pub recent_activity: Vec<RecentActivity>,
}

/// Usage statistics for a specific tier
#[derive(Debug, Serialize)]
pub struct TierUsage {
    pub tier: String,
    pub key_count: u32,
    pub total_requests: u64,
    pub average_requests_per_key: f64,
}

/// Recent API activity entry
#[derive(Debug, Serialize)]
pub struct RecentActivity {
    pub timestamp: chrono::DateTime<Utc>,
    pub api_key_name: String,
    pub tool_name: String,
    pub status_code: i32,
    pub response_time_ms: Option<i32>,
}
```

Additional structs like `UsageAnalytics`, `UsageDataPoint`, `ToolUsage`, `RateLimitOverview`, and `RequestLog` provide time series and per-tool breakdowns used by the frontend dashboard to render charts and tables.

## How This Ties Into MCP Tools

From the MCP side, API key and rate limit status surfaces via:

- **WebSocket**: Real-time updates for dashboards and observability tools.
- **HTTP analytics routes**: JSON endpoints consumed by the dashboard frontend.
- **A2A / tools**: Internal tools can introspect rate limit status when generating explanations (e.g., "you hit your trial quota").

Typical workflow for a B2B integrator:

1. **Create API key** using admin UI or REST route.
2. **Use key** as `Authorization: Bearer <api_key>` when calling Pierre MCP HTTP endpoints.
3. **Monitor usage** via dashboard or WebSocket feed.
4. **Upgrade tier** (trial → starter → professional) to unlock higher quotas.

## Key Takeaways

1. **API keys**: Tiered API keys with hashed storage and explicit monthly limits enable safe B2B access.
2. **Unified rate limiting**: `UnifiedRateLimitInfo` abstracts over API key vs JWT, ensuring consistent quota behavior.
3. **Tenant tiers**: `TenantRateLimitTier` augments per-key limits with SaaS plan semantics.
4. **Real-time updates**: WebSockets stream `UsageUpdate` and `SystemStats` messages for live dashboards.
5. **Dashboard models**: `DashboardOverview`, `UsageAnalytics`, and related structs power the analytics UI.
6. **Observability**: Combined HTTP + WebSocket surfaces make it easy to monitor usage, spot abuse, and tune quotas.
