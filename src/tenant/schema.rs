// ABOUTME: Database schema definitions for multi-tenant architecture
// ABOUTME: Defines tenant tables, OAuth credentials storage, and tenant-user relationships

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
