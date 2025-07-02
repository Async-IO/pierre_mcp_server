// ABOUTME: Data models and types for admin authentication and authorization system
// ABOUTME: Defines admin permissions, token structures, and validation types for admin operations
//! Admin Token Models
//!
//! Strong Rust types for the admin authentication system

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

/// Admin token with full details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminToken {
    pub id: String,
    pub service_name: String,
    pub service_description: Option<String>,
    pub token_hash: String,
    pub token_prefix: String,
    pub jwt_secret_hash: String,
    pub permissions: AdminPermissions,
    pub is_super_admin: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub last_used_ip: Option<String>,
    pub usage_count: u64,
}

/// Admin permissions with strong typing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminPermissions {
    permissions: HashSet<AdminPermission>,
}

impl AdminPermissions {
    /// Create new permissions set
    pub fn new(permissions: Vec<AdminPermission>) -> Self {
        Self {
            permissions: permissions.into_iter().collect(),
        }
    }

    /// Create default permissions for regular admin
    pub fn default_admin() -> Self {
        Self::new(vec![
            AdminPermission::ProvisionKeys,
            AdminPermission::ListKeys,
            AdminPermission::RevokeKeys,
            AdminPermission::UpdateKeyLimits,
        ])
    }

    /// Create super admin permissions (all permissions)
    pub fn super_admin() -> Self {
        Self::new(vec![
            AdminPermission::ProvisionKeys,
            AdminPermission::ListKeys,
            AdminPermission::RevokeKeys,
            AdminPermission::UpdateKeyLimits,
            AdminPermission::ManageAdminTokens,
            AdminPermission::ViewAuditLogs,
            AdminPermission::ManageUsers,
        ])
    }

    /// Check if permission is granted
    pub fn has_permission(&self, permission: &AdminPermission) -> bool {
        self.permissions.contains(permission)
    }

    /// Add permission
    pub fn add_permission(&mut self, permission: AdminPermission) {
        self.permissions.insert(permission);
    }

    /// Remove permission
    pub fn remove_permission(&mut self, permission: &AdminPermission) {
        self.permissions.remove(permission);
    }

    /// Get all permissions as vector
    pub fn to_vec(&self) -> Vec<AdminPermission> {
        self.permissions.iter().cloned().collect()
    }

    /// Convert to JSON string for database storage
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let permission_strings: Vec<String> =
            self.permissions.iter().map(|p| p.to_string()).collect();
        serde_json::to_string(&permission_strings)
    }

    /// Create from JSON string from database
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let permission_strings: Vec<String> = serde_json::from_str(json)?;
        let permissions = permission_strings
            .into_iter()
            .filter_map(|s| s.parse().ok())
            .collect();
        Ok(Self::new(permissions))
    }
}

/// Individual admin permissions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AdminPermission {
    /// Provision new API keys for users
    ProvisionKeys,
    /// List existing API keys
    ListKeys,
    /// Revoke/deactivate API keys
    RevokeKeys,
    /// Update API key rate limits
    UpdateKeyLimits,
    /// Manage admin tokens (super admin only)
    ManageAdminTokens,
    /// View audit logs (super admin only)
    ViewAuditLogs,
    /// Manage user accounts (super admin only)
    ManageUsers,
}

impl std::fmt::Display for AdminPermission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdminPermission::ProvisionKeys => write!(f, "provision_keys"),
            AdminPermission::ListKeys => write!(f, "list_keys"),
            AdminPermission::RevokeKeys => write!(f, "revoke_keys"),
            AdminPermission::UpdateKeyLimits => write!(f, "update_key_limits"),
            AdminPermission::ManageAdminTokens => write!(f, "manage_admin_tokens"),
            AdminPermission::ViewAuditLogs => write!(f, "view_audit_logs"),
            AdminPermission::ManageUsers => write!(f, "manage_users"),
        }
    }
}

impl std::str::FromStr for AdminPermission {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "provision_keys" => Ok(AdminPermission::ProvisionKeys),
            "list_keys" => Ok(AdminPermission::ListKeys),
            "revoke_keys" => Ok(AdminPermission::RevokeKeys),
            "update_key_limits" => Ok(AdminPermission::UpdateKeyLimits),
            "manage_admin_tokens" => Ok(AdminPermission::ManageAdminTokens),
            "view_audit_logs" => Ok(AdminPermission::ViewAuditLogs),
            "manage_users" => Ok(AdminPermission::ManageUsers),
            _ => Err(format!("Unknown permission: {}", s)),
        }
    }
}

/// Admin token creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAdminTokenRequest {
    pub service_name: String,
    pub service_description: Option<String>,
    pub permissions: Option<Vec<AdminPermission>>,
    pub expires_in_days: Option<u64>,
    pub is_super_admin: bool,
}

impl CreateAdminTokenRequest {
    /// Create request for regular admin token
    pub fn new(service_name: String) -> Self {
        Self {
            service_name,
            service_description: None,
            permissions: None,          // Will use default
            expires_in_days: Some(365), // 1 year default
            is_super_admin: false,
        }
    }

    /// Create request for super admin token
    pub fn super_admin(service_name: String) -> Self {
        Self {
            service_name,
            service_description: Some("Super Admin Token".to_string()),
            permissions: None,     // Will use super admin permissions
            expires_in_days: None, // Never expires
            is_super_admin: true,
        }
    }
}

/// Generated admin token response
#[derive(Debug, Clone, Serialize)]
pub struct GeneratedAdminToken {
    pub token_id: String,
    pub service_name: String,
    pub jwt_token: String, // Only shown once!
    pub token_prefix: String,
    pub permissions: AdminPermissions,
    pub is_super_admin: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Admin token usage audit entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminTokenUsage {
    pub id: Option<i64>,
    pub admin_token_id: String,
    pub timestamp: DateTime<Utc>,
    pub action: AdminAction,
    pub target_resource: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub request_size_bytes: Option<u32>,
    pub success: bool,
    pub error_message: Option<String>,
    pub response_time_ms: Option<u32>,
}

/// Admin actions for audit logging
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AdminAction {
    ProvisionKey,
    RevokeKey,
    ListKeys,
    UpdateKeyLimits,
    ListAdminTokens,
    RevokeAdminToken,
    ViewAuditLogs,
    ManageUser,
}

impl std::fmt::Display for AdminAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdminAction::ProvisionKey => write!(f, "provision_key"),
            AdminAction::RevokeKey => write!(f, "revoke_key"),
            AdminAction::ListKeys => write!(f, "list_keys"),
            AdminAction::UpdateKeyLimits => write!(f, "update_key_limits"),
            AdminAction::ListAdminTokens => write!(f, "list_admin_tokens"),
            AdminAction::RevokeAdminToken => write!(f, "revoke_admin_token"),
            AdminAction::ViewAuditLogs => write!(f, "view_audit_logs"),
            AdminAction::ManageUser => write!(f, "manage_user"),
        }
    }
}

impl std::str::FromStr for AdminAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "provision_key" => Ok(AdminAction::ProvisionKey),
            "revoke_key" => Ok(AdminAction::RevokeKey),
            "list_keys" => Ok(AdminAction::ListKeys),
            "update_key_limits" => Ok(AdminAction::UpdateKeyLimits),
            "list_admin_tokens" => Ok(AdminAction::ListAdminTokens),
            "revoke_admin_token" => Ok(AdminAction::RevokeAdminToken),
            "view_audit_logs" => Ok(AdminAction::ViewAuditLogs),
            "manage_user" => Ok(AdminAction::ManageUser),
            _ => Err(format!("Unknown admin action: {}", s)),
        }
    }
}

/// API key provisioning request from admin service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyProvisionRequest {
    pub user_email: String,
    pub user_id: Option<Uuid>,
    pub tier: String, // "starter", "professional", "enterprise"
    pub rate_limit_requests: u32,
    pub rate_limit_period: RateLimitPeriod,
    pub expires_in_days: Option<u64>,
    pub metadata: Option<serde_json::Value>, // Company name, use case, etc.
}

/// Rate limit periods for API keys
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RateLimitPeriod {
    Hour,
    Day,
    Month,
}

impl std::fmt::Display for RateLimitPeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RateLimitPeriod::Hour => write!(f, "hour"),
            RateLimitPeriod::Day => write!(f, "day"),
            RateLimitPeriod::Month => write!(f, "month"),
        }
    }
}

impl RateLimitPeriod {
    /// Get the window duration in seconds
    pub fn window_seconds(&self) -> u64 {
        match self {
            RateLimitPeriod::Hour => 3600,
            RateLimitPeriod::Day => 86400,
            RateLimitPeriod::Month => 2_592_000, // 30 days
        }
    }
}

/// API key provisioning response
#[derive(Debug, Clone, Serialize)]
pub struct ProvisionedApiKey {
    pub api_key_id: String,
    pub api_key: String, // Only shown once!
    pub user_id: Uuid,
    pub user_email: String,
    pub tier: String,
    pub rate_limit_requests: u32,
    pub rate_limit_period: RateLimitPeriod,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Admin token validation result
#[derive(Debug, Clone)]
pub struct ValidatedAdminToken {
    pub token_id: String,
    pub service_name: String,
    pub permissions: AdminPermissions,
    pub is_super_admin: bool,
    pub user_info: Option<serde_json::Value>, // JWT claims
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_permissions_serialization() {
        let permissions = AdminPermissions::default_admin();
        let json = permissions.to_json().unwrap();
        let restored = AdminPermissions::from_json(&json).unwrap();
        assert_eq!(permissions, restored);
    }

    #[test]
    fn test_admin_permission_string_conversion() {
        let permission = AdminPermission::ProvisionKeys;
        let string = permission.to_string();
        let restored: AdminPermission = string.parse().unwrap();
        assert_eq!(permission, restored);
    }

    #[test]
    fn test_super_admin_permissions() {
        let super_admin = AdminPermissions::super_admin();
        assert!(super_admin.has_permission(&AdminPermission::ManageAdminTokens));
        assert!(super_admin.has_permission(&AdminPermission::ProvisionKeys));

        let regular_admin = AdminPermissions::default_admin();
        assert!(!regular_admin.has_permission(&AdminPermission::ManageAdminTokens));
        assert!(regular_admin.has_permission(&AdminPermission::ProvisionKeys));
    }

    #[test]
    fn test_rate_limit_period_conversion() {
        let period = RateLimitPeriod::Month;
        assert_eq!(period.window_seconds(), 2_592_000);
        assert_eq!(period.to_string(), "month");
    }
}
