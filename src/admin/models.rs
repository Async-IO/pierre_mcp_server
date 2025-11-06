// ABOUTME: Data models and types for admin authentication and authorization system
// ABOUTME: Defines admin permissions, token structures, and validation types for admin operations
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
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
    /// Unique token identifier (UUID)
    pub id: String,
    /// Service or system name using this token
    pub service_name: String,
    /// Optional description of the service
    pub service_description: Option<String>,
    /// SHA-256 hash of the token for verification
    pub token_hash: String,
    /// First 8 characters of token for identification
    pub token_prefix: String,
    /// SHA-256 hash of JWT secret for this token
    pub jwt_secret_hash: String,
    /// Granted admin permissions
    pub permissions: AdminPermissions,
    /// Whether this is a super admin token
    pub is_super_admin: bool,
    /// Whether the token is active
    pub is_active: bool,
    /// When the token was created
    pub created_at: DateTime<Utc>,
    /// Optional token expiration time
    pub expires_at: Option<DateTime<Utc>>,
    /// When the token was last used
    pub last_used_at: Option<DateTime<Utc>>,
    /// IP address from last use
    pub last_used_ip: Option<String>,
    /// Total number of times token has been used
    pub usage_count: u64,
}

/// Admin permissions with strong typing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminPermissions {
    /// Set of granted permissions
    permissions: HashSet<AdminPermission>,
}

impl AdminPermissions {
    /// Create new permissions set
    #[must_use]
    pub fn new(permissions: Vec<AdminPermission>) -> Self {
        Self {
            permissions: permissions.into_iter().collect(),
        }
    }

    /// Create default permissions for regular admin
    #[must_use]
    pub fn default_admin() -> Self {
        Self::new(vec![
            AdminPermission::ProvisionKeys,
            AdminPermission::ListKeys,
            AdminPermission::RevokeKeys,
            AdminPermission::UpdateKeyLimits,
        ])
    }

    /// Create super admin permissions (all permissions)
    #[must_use]
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
    #[must_use]
    pub fn has_permission(&self, permission: &AdminPermission) -> bool {
        self.permissions.contains(permission)
    }

    /// Add permission
    pub fn add_permission(&mut self, permission: AdminPermission) -> bool {
        self.permissions.insert(permission)
    }

    /// Remove permission
    pub fn remove_permission(&mut self, permission: &AdminPermission) -> bool {
        self.permissions.remove(permission)
    }

    /// Get all permissions as vector
    #[must_use]
    pub fn to_vec(&self) -> Vec<AdminPermission> {
        self.permissions.iter().cloned().collect()
    }

    /// Convert to JSON string for database storage
    ///
    /// # Errors
    ///
    /// Returns `serde_json::Error` if serialization fails
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let permission_strings: Vec<String> =
            self.permissions.iter().map(ToString::to_string).collect();
        serde_json::to_string(&permission_strings)
    }

    /// Create from JSON string from database
    ///
    /// # Errors
    ///
    /// Returns `serde_json::Error` if deserialization fails
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
            Self::ProvisionKeys => write!(f, "provision_keys"),
            Self::ListKeys => write!(f, "list_keys"),
            Self::RevokeKeys => write!(f, "revoke_keys"),
            Self::UpdateKeyLimits => write!(f, "update_key_limits"),
            Self::ManageAdminTokens => write!(f, "manage_admin_tokens"),
            Self::ViewAuditLogs => write!(f, "view_audit_logs"),
            Self::ManageUsers => write!(f, "manage_users"),
        }
    }
}

impl std::str::FromStr for AdminPermission {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "provision_keys" => Ok(Self::ProvisionKeys),
            "list_keys" => Ok(Self::ListKeys),
            "revoke_keys" => Ok(Self::RevokeKeys),
            "update_key_limits" => Ok(Self::UpdateKeyLimits),
            "manage_admin_tokens" => Ok(Self::ManageAdminTokens),
            "view_audit_logs" => Ok(Self::ViewAuditLogs),
            "manage_users" => Ok(Self::ManageUsers),
            _ => Err(format!("Unknown permission: {s}")),
        }
    }
}

/// Admin token creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAdminTokenRequest {
    /// Name of the service requesting the token
    pub service_name: String,
    /// Optional description of the service
    pub service_description: Option<String>,
    /// List of permissions to grant (None = default permissions)
    pub permissions: Option<Vec<AdminPermission>>,
    /// Number of days until token expires (None = never expires)
    pub expires_in_days: Option<u64>,
    /// Whether this is a super admin token with all permissions
    pub is_super_admin: bool,
}

impl CreateAdminTokenRequest {
    /// Create request for regular admin token
    #[must_use]
    pub const fn new(service_name: String) -> Self {
        Self {
            service_name,
            service_description: None,
            permissions: None,          // Will use default
            expires_in_days: Some(365), // 1 year default
            is_super_admin: false,
        }
    }

    /// Create request for super admin token
    #[must_use]
    pub fn super_admin(service_name: String) -> Self {
        Self {
            service_name,
            service_description: Some("Super Admin Token".into()),
            permissions: None,     // Will use super admin permissions
            expires_in_days: None, // Never expires
            is_super_admin: true,
        }
    }
}

/// Generated admin token response
#[derive(Debug, Clone, Serialize)]
pub struct GeneratedAdminToken {
    /// Unique identifier for the token
    pub token_id: String,
    /// Name of the service
    pub service_name: String,
    /// The actual JWT token (shown only once!)
    pub jwt_token: String,
    /// Visible prefix for identification
    pub token_prefix: String,
    /// Permissions granted to this token
    pub permissions: AdminPermissions,
    /// Whether this is a super admin token
    pub is_super_admin: bool,
    /// When the token expires (if set)
    pub expires_at: Option<DateTime<Utc>>,
    /// When the token was created
    pub created_at: DateTime<Utc>,
}

/// Admin token usage audit entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminTokenUsage {
    /// Unique identifier for this usage record
    pub id: Option<i64>,
    /// ID of the admin token that was used
    pub admin_token_id: String,
    /// When the action was performed
    pub timestamp: DateTime<Utc>,
    /// Type of admin action performed
    pub action: AdminAction,
    /// Resource that was targeted (if applicable)
    pub target_resource: Option<String>,
    /// Client IP address
    pub ip_address: Option<String>,
    /// Client user agent
    pub user_agent: Option<String>,
    /// Size of the request in bytes
    pub request_size_bytes: Option<u32>,
    /// Whether the action succeeded
    pub success: bool,
    /// Error message if the action failed
    pub error_message: Option<String>,
    /// Response time in milliseconds
    pub response_time_ms: Option<u32>,
}

/// Admin actions for audit logging
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AdminAction {
    /// Provision a new API key
    ProvisionKey,
    /// Revoke an existing API key
    RevokeKey,
    /// List all API keys
    ListKeys,
    /// Update rate limits for an API key
    UpdateKeyLimits,
    /// List all admin tokens
    ListAdminTokens,
    /// Revoke an admin token
    RevokeAdminToken,
    /// View audit logs
    ViewAuditLogs,
    /// Manage user accounts
    ManageUser,
}

impl std::fmt::Display for AdminAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProvisionKey => write!(f, "provision_key"),
            Self::RevokeKey => write!(f, "revoke_key"),
            Self::ListKeys => write!(f, "list_keys"),
            Self::UpdateKeyLimits => write!(f, "update_key_limits"),
            Self::ListAdminTokens => write!(f, "list_admin_tokens"),
            Self::RevokeAdminToken => write!(f, "revoke_admin_token"),
            Self::ViewAuditLogs => write!(f, "view_audit_logs"),
            Self::ManageUser => write!(f, "manage_user"),
        }
    }
}

impl std::str::FromStr for AdminAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "provision_key" => Ok(Self::ProvisionKey),
            "revoke_key" => Ok(Self::RevokeKey),
            "list_keys" => Ok(Self::ListKeys),
            "update_key_limits" => Ok(Self::UpdateKeyLimits),
            "list_admin_tokens" => Ok(Self::ListAdminTokens),
            "revoke_admin_token" => Ok(Self::RevokeAdminToken),
            "view_audit_logs" => Ok(Self::ViewAuditLogs),
            "manage_user" => Ok(Self::ManageUser),
            _ => Err(format!("Unknown admin action: {s}")),
        }
    }
}

/// API key provisioning request from admin service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyProvisionRequest {
    /// Email of the user to provision the key for
    pub user_email: String,
    /// Optional user ID (will be looked up if not provided)
    pub user_id: Option<Uuid>,
    /// Tier level ("starter", "professional", "enterprise")
    pub tier: String,
    /// Maximum requests allowed
    pub rate_limit_requests: u32,
    /// Rate limit period
    pub rate_limit_period: RateLimitPeriod,
    /// Number of days until the key expires
    pub expires_in_days: Option<u64>,
    /// Additional metadata (company name, use case, etc.)
    pub metadata: Option<serde_json::Value>,
}

/// Rate limit periods for API keys
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RateLimitPeriod {
    /// Rate limit per hour
    Hour,
    /// Rate limit per day
    Day,
    /// Rate limit per month
    Month,
}

impl std::fmt::Display for RateLimitPeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hour => write!(f, "hour"),
            Self::Day => write!(f, "day"),
            Self::Month => write!(f, "month"),
        }
    }
}

impl RateLimitPeriod {
    /// Get the window duration in seconds
    #[must_use]
    pub const fn window_seconds(&self) -> u64 {
        match self {
            Self::Hour => 3_600,
            Self::Day => 86_400,
            Self::Month => 2_592_000, // 30 days
        }
    }
}

/// API key provisioning response
#[derive(Debug, Clone, Serialize)]
pub struct ProvisionedApiKey {
    /// Unique identifier for the API key
    pub api_key_id: String,
    /// The actual API key (shown only once!)
    pub api_key: String,
    /// ID of the user who owns the key
    pub user_id: Uuid,
    /// Email of the user who owns the key
    pub user_email: String,
    /// Tier level of the key
    pub tier: String,
    /// Maximum requests allowed
    pub rate_limit_requests: u32,
    /// Rate limit period
    pub rate_limit_period: RateLimitPeriod,
    /// When the key expires (if set)
    pub expires_at: Option<DateTime<Utc>>,
    /// When the key was created
    pub created_at: DateTime<Utc>,
}

/// Admin token validation result
#[derive(Debug, Clone)]
pub struct ValidatedAdminToken {
    /// Unique identifier for the token
    pub token_id: String,
    /// Name of the service
    pub service_name: String,
    /// Permissions granted to this token
    pub permissions: AdminPermissions,
    /// Whether this is a super admin token
    pub is_super_admin: bool,
    /// Additional user info from JWT claims
    pub user_info: Option<serde_json::Value>,
}
