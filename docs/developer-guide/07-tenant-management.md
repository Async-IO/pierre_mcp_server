# Tenant Management System

## Overview

Pierre MCP Server implements a comprehensive multi-tenant architecture that provides complete data isolation, per-tenant OAuth configurations, and enterprise-ready SaaS capabilities. Each tenant operates as an independent organization with its own users, data, configurations, and rate limits.

## Multi-Tenant Architecture

```mermaid
graph TB
    subgraph "Tenant A"
        UserA1[User A1]
        UserA2[User A2]
        DataA[(Tenant A Data)]
        ConfigA[OAuth Config A]
    end
    
    subgraph "Tenant B"
        UserB1[User B1]
        UserB2[User B2]
        DataB[(Tenant B Data)]
        ConfigB[OAuth Config B]
    end
    
    subgraph "Shared Infrastructure"
        API[Pierre API]
        Auth[Auth System]
        DB[(Shared Database)]
        Cache[Shared Cache]
    end
    
    subgraph "Providers"
        StravaA[Strava A]
        FitbitA[Fitbit A]
        StravaB[Strava B]
        FitbitB[Fitbit B]
    end
    
    UserA1 --> API
    UserA2 --> API
    UserB1 --> API
    UserB2 --> API
    
    API --> Auth
    Auth --> DB
    API --> Cache
    
    ConfigA --> StravaA
    ConfigA --> FitbitA
    ConfigB --> StravaB
    ConfigB --> FitbitB
    
    DataA --> DB
    DataB --> DB
```

## Tenant Schema Design

### Core Tenant Entity

```rust
// src/tenant/schema.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,              // Display name: "Acme Corporation"
    pub slug: String,              // URL-safe: "acme-corp"
    pub domain: Option<String>,    // Custom domain: "pierre.acme.com"
    pub subscription_tier: String,  // "starter", "professional", "enterprise"
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### Tenant Roles

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TenantRole {
    Owner,    // Full control, can delete tenant
    Admin,    // Can manage users, configure OAuth
    Billing,  // Can view usage, manage subscriptions
    Member,   // Can use fitness tools
}
```

### Tenant-User Relationships

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantUser {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub role: TenantRole,
    pub is_active: bool,
    pub invited_by: Option<Uuid>,
    pub joined_at: DateTime<Utc>,
}
```

## Tenant Context

Every request operates within a tenant context that provides data isolation:

```rust
// src/tenant/mod.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantContext {
    pub tenant_id: Uuid,
    pub tenant_name: String,
    pub user_id: Uuid,
    pub user_role: TenantRole,
}

impl TenantContext {
    pub fn is_admin(&self) -> bool {
        matches!(self.user_role, TenantRole::Admin | TenantRole::Owner)
    }
    
    pub fn can_configure_oauth(&self) -> bool {
        matches!(self.user_role, TenantRole::Admin | TenantRole::Owner)
    }
    
    pub fn can_manage_users(&self) -> bool {
        matches!(self.user_role, TenantRole::Admin | TenantRole::Owner)
    }
}
```

## Tenant Creation Flow

### 1. Organization Registration

```mermaid
sequenceDiagram
    participant User as New User
    participant API as Pierre API
    participant DB as Database
    participant Auth as Auth Manager
    
    User->>API: POST /api/tenants/register
    API->>API: Validate registration data
    API->>DB: Check tenant slug availability
    DB-->>API: Slug available
    
    API->>DB: Create tenant
    DB-->>API: Tenant created
    
    API->>DB: Create user
    DB-->>API: User created
    
    API->>DB: Create tenant-user relationship (Owner)
    DB-->>API: Relationship created
    
    API->>Auth: Generate JWT token
    Auth-->>API: Token
    
    API-->>User: Registration success + token
```

### Registration Request

```bash
curl -X POST https://pierre-api.example.com/api/tenants/register \
  -H "Content-Type: application/json" \
  -d '{
    "tenant": {
      "name": "Acme Corporation",
      "slug": "acme-corp"
    },
    "user": {
      "email": "admin@acme.com",
      "password": "SecurePassword123!",
      "firstname": "John",
      "lastname": "Doe"
    }
  }'
```

**Response:**
```json
{
  "success": true,
  "tenant": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "Acme Corporation",
    "slug": "acme-corp",
    "subscription_tier": "starter",
    "is_active": true,
    "created_at": "2024-01-15T10:30:00Z"
  },
  "user": {
    "id": "660f9511-f3ac-52e5-b827-557766551111",
    "email": "admin@acme.com",
    "firstname": "John",
    "lastname": "Doe",
    "role": "owner"
  },
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "refresh_token": "rt_abc123def456...",
  "expires_at": "2024-01-16T10:30:00Z"
}
```

### 2. User Invitation Flow

```mermaid
sequenceDiagram
    participant Admin as Admin User
    participant API as Pierre API
    participant Email as Email Service
    participant NewUser as Invited User
    participant DB as Database
    
    Admin->>API: POST /api/tenants/{id}/invite
    API->>DB: Create invitation record
    DB-->>API: Invitation created
    
    API->>Email: Send invitation email
    Email-->>NewUser: Invitation email
    
    NewUser->>API: GET /api/invitations/{token}
    API->>DB: Validate invitation
    DB-->>API: Invitation valid
    
    NewUser->>API: POST /api/invitations/{token}/accept
    API->>DB: Create user + tenant relationship
    DB-->>API: User created
    
    API-->>NewUser: Account created + token
```

### Invite User Request

```bash
curl -X POST https://pierre-api.example.com/api/tenants/550e8400-e29b-41d4-a716-446655440000/invite \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -H "Content-Type: application/json" \
  -d '{
    "email": "member@acme.com",
    "role": "member",
    "message": "Welcome to our fitness tracking team!"
  }'
```

**Response:**
```json
{
  "success": true,
  "invitation": {
    "id": "770a0622-g4bd-63f6-c938-668877662222",
    "email": "member@acme.com",
    "role": "member",
    "invited_by": "660f9511-f3ac-52e5-b827-557766551111",
    "expires_at": "2024-01-22T10:30:00Z",
    "status": "pending"
  }
}
```

### Accept Invitation

```bash
curl -X POST https://pierre-api.example.com/api/invitations/770a0622-g4bd-63f6-c938-668877662222/accept \
  -H "Content-Type: application/json" \
  -d '{
    "password": "MemberPassword123!",
    "firstname": "Jane",
    "lastname": "Smith"
  }'
```

## OAuth Configuration Per Tenant

### Tenant-Specific OAuth Apps

Each tenant can configure their own OAuth applications for better rate limits and branding:

```rust
// src/tenant/oauth_manager.rs
#[derive(Debug, Clone)]
pub struct TenantOAuthCredentials {
    pub tenant_id: Uuid,
    pub provider: String,        // "strava", "fitbit"
    pub client_id: String,       // Public client ID
    pub client_secret: String,   // Encrypted client secret
    pub redirect_uri: String,    // Tenant-specific redirect
    pub scopes: Vec<String>,     // Requested scopes
    pub rate_limit_per_day: u32, // Provider rate limit
}
```

### Configure Tenant OAuth

```bash
curl -X POST https://pierre-api.example.com/api/tenants/550e8400-e29b-41d4-a716-446655440000/oauth/strava \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -H "Content-Type: application/json" \
  -d '{
    "client_id": "your_strava_client_id",
    "client_secret": "your_strava_client_secret",
    "redirect_uri": "https://pierre.acme.com/oauth/callback/strava",
    "scopes": ["read", "activity:read_all", "profile:read_all"]
  }'
```

**Response:**
```json
{
  "success": true,
  "provider": "strava",
  "client_id": "your_strava_client_id",
  "redirect_uri": "https://pierre.acme.com/oauth/callback/strava",
  "scopes": ["read", "activity:read_all", "profile:read_all"],
  "rate_limit_per_day": 15000,
  "configured_at": "2024-01-15T11:00:00Z"
}
```

### OAuth Flow with Tenant Context

```mermaid
sequenceDiagram
    participant User as Tenant User
    participant Pierre as Pierre API
    participant TenantOAuth as Tenant OAuth Config
    participant Strava as Strava
    participant DB as Database
    
    User->>Pierre: GET /oauth/strava/auth
    Pierre->>TenantOAuth: Get tenant OAuth config
    TenantOAuth-->>Pierre: Client ID, redirect URI
    
    Pierre->>Pierre: Generate OAuth state
    Pierre->>DB: Store state with tenant context
    
    Pierre-->>User: Redirect to Strava
    User->>Strava: Authorize app
    Strava-->>Pierre: Callback with code
    
    Pierre->>DB: Validate state + tenant
    Pierre->>TenantOAuth: Get client secret
    Pierre->>Strava: Exchange code for tokens
    Strava-->>Pierre: Access token
    
    Pierre->>DB: Store encrypted tokens (tenant-scoped)
    Pierre-->>User: Connection successful
```

## Data Isolation

### Database-Level Isolation

All data operations are scoped to tenant context:

```rust
// Example: Activity retrieval with tenant isolation
impl Database {
    pub async fn get_activities_for_tenant(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        limit: u32,
    ) -> Result<Vec<Activity>> {
        sqlx::query_as!(
            Activity,
            "SELECT * FROM activities 
             WHERE tenant_id = $1 AND user_id = $2 
             ORDER BY created_at DESC LIMIT $3",
            tenant_id,
            user_id,
            limit as i32
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }
}
```

### Provider Isolation

Each tenant's provider connections are isolated:

```rust
// src/providers/tenant_provider.rs
pub struct TenantProviderFactory {
    oauth_client: Arc<TenantOAuthClient>,
}

impl TenantProviderFactory {
    pub async fn create_provider(
        &self,
        provider_type: &str,
        tenant_context: &TenantContext,
    ) -> Result<Box<dyn FitnessProvider>> {
        // Load tenant-specific OAuth credentials
        let oauth_creds = self.oauth_client
            .get_credentials(tenant_context.tenant_id, provider_type)
            .await?;
        
        match provider_type {
            "strava" => {
                let provider = StravaTenantProvider::new(
                    oauth_creds,
                    tenant_context.clone()
                )?;
                Ok(Box::new(provider))
            },
            "fitbit" => {
                let provider = FitbitTenantProvider::new(
                    oauth_creds,
                    tenant_context.clone()
                )?;
                Ok(Box::new(provider))
            },
            _ => Err(anyhow!("Unknown provider: {}", provider_type)),
        }
    }
}
```

## Rate Limiting Per Tenant

### Tenant-Specific Rate Limits

```rust
// src/rate_limiting.rs
#[derive(Debug, Clone)]
pub struct TenantRateLimit {
    pub tenant_id: Uuid,
    pub subscription_tier: String,
    pub mcp_requests_per_minute: u32,
    pub api_requests_per_minute: u32,
    pub provider_requests_per_day: u32,
    pub burst_size: u32,
}

impl TenantRateLimit {
    pub fn for_subscription_tier(tier: &str) -> Self {
        match tier {
            "starter" => Self {
                tenant_id: Uuid::nil(),
                subscription_tier: tier.to_string(),
                mcp_requests_per_minute: 60,
                api_requests_per_minute: 120,
                provider_requests_per_day: 5000,
                burst_size: 10,
            },
            "professional" => Self {
                tenant_id: Uuid::nil(),
                subscription_tier: tier.to_string(),
                mcp_requests_per_minute: 300,
                api_requests_per_minute: 600,
                provider_requests_per_day: 20000,
                burst_size: 50,
            },
            "enterprise" => Self {
                tenant_id: Uuid::nil(),
                subscription_tier: tier.to_string(),
                mcp_requests_per_minute: 1000,
                api_requests_per_minute: 2000,
                provider_requests_per_day: 100000,
                burst_size: 200,
            },
            _ => Self::for_subscription_tier("starter"),
        }
    }
}
```

### Rate Limiting Middleware

```rust
impl RateLimitingMiddleware {
    pub async fn check_tenant_limits(
        &self,
        tenant_context: &TenantContext,
        request_type: RequestType,
    ) -> Result<()> {
        let limits = self.get_tenant_limits(&tenant_context.tenant_id).await?;
        let key = format!("{}:{}:{}", 
            tenant_context.tenant_id, 
            request_type.to_string(),
            self.current_window()
        );
        
        let current_usage = self.redis_client.get(&key).await.unwrap_or(0);
        let limit = match request_type {
            RequestType::MCP => limits.mcp_requests_per_minute,
            RequestType::API => limits.api_requests_per_minute,
            RequestType::Provider => limits.provider_requests_per_day,
        };
        
        if current_usage >= limit {
            return Err(RateLimitError::TenantQuotaExceeded {
                tenant_id: tenant_context.tenant_id,
                limit,
                current_usage,
            });
        }
        
        self.redis_client.incr(&key).await?;
        Ok(())
    }
}
```

## Tenant Administration

### Admin Dashboard API

```bash
# Get tenant overview
curl -X GET https://pierre-api.example.com/admin/tenants/550e8400-e29b-41d4-a716-446655440000 \
  -H "Authorization: Bearer admin_token..."
```

**Response:**
```json
{
  "tenant": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "Acme Corporation",
    "slug": "acme-corp",
    "subscription_tier": "professional",
    "is_active": true,
    "created_at": "2024-01-15T10:30:00Z"
  },
  "statistics": {
    "total_users": 15,
    "active_users": 12,
    "mcp_requests_this_month": 45678,
    "api_requests_this_month": 23456,
    "provider_connections": {
      "strava": 8,
      "fitbit": 4
    }
  },
  "usage": {
    "current_period": {
      "start": "2024-01-01T00:00:00Z",
      "end": "2024-01-31T23:59:59Z",
      "mcp_requests": 45678,
      "api_requests": 23456,
      "storage_mb": 245
    },
    "rate_limits": {
      "mcp_requests_per_minute": 300,
      "api_requests_per_minute": 600,
      "provider_requests_per_day": 20000
    }
  }
}
```

### User Management

```bash
# List tenant users
curl -X GET https://pierre-api.example.com/api/tenants/550e8400-e29b-41d4-a716-446655440000/users \
  -H "Authorization: Bearer owner_token..."
```

**Response:**
```json
{
  "users": [
    {
      "id": "660f9511-f3ac-52e5-b827-557766551111",
      "email": "admin@acme.com",
      "firstname": "John",
      "lastname": "Doe",
      "role": "owner",
      "is_active": true,
      "joined_at": "2024-01-15T10:30:00Z",
      "last_active": "2024-01-20T14:22:00Z"
    },
    {
      "id": "770a0622-g4bd-63f6-c938-668877662222",
      "email": "member@acme.com",
      "firstname": "Jane",
      "lastname": "Smith",
      "role": "member",
      "is_active": true,
      "joined_at": "2024-01-16T09:15:00Z",
      "last_active": "2024-01-20T16:45:00Z"
    }
  ],
  "total_users": 15,
  "active_users": 12,
  "pending_invitations": 2
}
```

### Update User Role

```bash
curl -X PUT https://pierre-api.example.com/api/tenants/550e8400-e29b-41d4-a716-446655440000/users/770a0622-g4bd-63f6-c938-668877662222/role \
  -H "Authorization: Bearer owner_token..." \
  -H "Content-Type: application/json" \
  -d '{
    "role": "admin"
  }'
```

## Subscription Management

### Subscription Tiers

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionTier {
    pub name: String,
    pub monthly_price: u32,
    pub max_users: Option<u32>,
    pub mcp_requests_per_month: u32,
    pub api_requests_per_month: u32,
    pub storage_gb: u32,
    pub features: Vec<String>,
}

impl SubscriptionTier {
    pub fn all_tiers() -> Vec<Self> {
        vec![
            Self {
                name: "starter".to_string(),
                monthly_price: 0,
                max_users: Some(5),
                mcp_requests_per_month: 10000,
                api_requests_per_month: 50000,
                storage_gb: 1,
                features: vec!["basic-analytics".to_string()],
            },
            Self {
                name: "professional".to_string(),
                monthly_price: 29,
                max_users: Some(25),
                mcp_requests_per_month: 100000,
                api_requests_per_month: 500000,
                storage_gb: 10,
                features: vec![
                    "advanced-analytics".to_string(),
                    "custom-oauth".to_string(),
                    "priority-support".to_string(),
                ],
            },
            Self {
                name: "enterprise".to_string(),
                monthly_price: 99,
                max_users: None,
                mcp_requests_per_month: 1000000,
                api_requests_per_month: 5000000,
                storage_gb: 100,
                features: vec![
                    "advanced-analytics".to_string(),
                    "custom-oauth".to_string(),
                    "priority-support".to_string(),
                    "dedicated-support".to_string(),
                    "custom-integrations".to_string(),
                    "sso".to_string(),
                ],
            },
        ]
    }
}
```

### Upgrade Subscription

```bash
curl -X PUT https://pierre-api.example.com/api/tenants/550e8400-e29b-41d4-a716-446655440000/subscription \
  -H "Authorization: Bearer owner_token..." \
  -H "Content-Type: application/json" \
  -d '{
    "tier": "professional",
    "billing_cycle": "monthly",
    "payment_method_id": "pm_1234567890"
  }'
```

## Tenant Deletion and Data Export

### Data Export

```bash
# Request tenant data export
curl -X POST https://pierre-api.example.com/api/tenants/550e8400-e29b-41d4-a716-446655440000/export \
  -H "Authorization: Bearer owner_token..."
```

**Response:**
```json
{
  "success": true,
  "export_id": "export_880b1733-h5ce-74g7-d049-779988773333",
  "status": "processing",
  "estimated_completion": "2024-01-20T15:30:00Z",
  "includes": [
    "user_data",
    "activities",
    "oauth_connections",
    "configurations",
    "analytics"
  ]
}
```

### Tenant Deletion

```bash
# Delete tenant (requires confirmation)
curl -X DELETE https://pierre-api.example.com/api/tenants/550e8400-e29b-41d4-a716-446655440000 \
  -H "Authorization: Bearer owner_token..." \
  -H "Content-Type: application/json" \
  -d '{
    "confirmation": "DELETE Acme Corporation",
    "reason": "Company closure"
  }'
```

## Security Considerations

### Tenant Isolation Validation

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_tenant_data_isolation() {
        let server = create_test_server().await;
        
        // Create two tenants
        let tenant_a = create_test_tenant("tenant-a").await;
        let tenant_b = create_test_tenant("tenant-b").await;
        
        // Create activities for each tenant
        let activity_a = create_test_activity(&tenant_a).await;
        let activity_b = create_test_activity(&tenant_b).await;
        
        // Verify tenant A can only see their data
        let tenant_a_activities = server
            .get_activities(&tenant_a.context)
            .await
            .unwrap();
        
        assert_eq!(tenant_a_activities.len(), 1);
        assert_eq!(tenant_a_activities[0].id, activity_a.id);
        
        // Verify tenant B can only see their data
        let tenant_b_activities = server
            .get_activities(&tenant_b.context)
            .await
            .unwrap();
        
        assert_eq!(tenant_b_activities.len(), 1);
        assert_eq!(tenant_b_activities[0].id, activity_b.id);
    }
}
```

### Cross-Tenant Access Prevention

```rust
impl Database {
    pub async fn get_activity_by_id(
        &self,
        activity_id: Uuid,
        tenant_context: &TenantContext,
    ) -> Result<Option<Activity>> {
        // Always include tenant_id in WHERE clause
        sqlx::query_as!(
            Activity,
            "SELECT * FROM activities 
             WHERE id = $1 AND tenant_id = $2",
            activity_id,
            tenant_context.tenant_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(Into::into)
    }
}
```

## Monitoring and Analytics

### Tenant Metrics

```rust
pub struct TenantMetrics {
    pub tenant_id: Uuid,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub mcp_requests: u64,
    pub api_requests: u64,
    pub unique_users: u32,
    pub provider_calls: HashMap<String, u64>,
    pub storage_mb: f64,
    pub errors: u64,
    pub response_time_p95: f64,
}
```

### Usage Billing

```rust
pub struct UsageBilling {
    pub tenant_id: Uuid,
    pub billing_period: String,
    pub base_cost: u32,
    pub overage_costs: HashMap<String, u32>,
    pub total_cost: u32,
    pub usage_details: UsageBreakdown,
}

pub struct UsageBreakdown {
    pub mcp_requests: UsageItem,
    pub api_requests: UsageItem,
    pub storage: UsageItem,
    pub support_requests: UsageItem,
}

pub struct UsageItem {
    pub included: u64,
    pub used: u64,
    pub overage: u64,
    pub unit_price: f64,
    pub cost: u32,
}
```

This comprehensive tenant management system ensures complete isolation between organizations while providing enterprise-ready features like custom OAuth configurations, role-based access control, and detailed usage tracking.