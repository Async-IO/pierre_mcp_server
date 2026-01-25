// ABOUTME: Request and response types for admin routes
// ABOUTME: Defines DTOs for API key management, user administration, and coach review endpoints
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Admin request and response types
//!
//! This module contains all DTOs (Data Transfer Objects) used by the admin
//! routes for serialization and deserialization of API requests and responses.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// API key provisioning request
#[derive(Debug, Deserialize)]
pub struct ProvisionApiKeyRequest {
    /// Email of the user to provision the key for
    pub user_email: String,
    /// Tier level for the API key (starter/professional/enterprise)
    pub tier: String,
    /// Optional description of the API key's purpose
    pub description: Option<String>,
    /// Number of days until the key expires
    pub expires_in_days: Option<u32>,
    /// Maximum requests allowed
    pub rate_limit_requests: Option<u32>,
    /// Rate limit period (e.g., "hour", "day", "month")
    pub rate_limit_period: Option<String>,
}

/// API key revocation request
#[derive(Debug, Deserialize)]
pub struct RevokeKeyRequest {
    /// ID of the API key to revoke
    pub api_key_id: String,
    /// Optional reason for revoking the key
    pub reason: Option<String>,
}

/// Admin setup request
#[derive(Debug, Deserialize)]
pub struct AdminSetupRequest {
    /// Admin email address
    pub email: String,
    /// Admin password
    pub password: String,
    /// Optional display name for the admin
    pub display_name: Option<String>,
}

/// User approval request
#[derive(Debug, Deserialize)]
pub struct ApproveUserRequest {
    /// Optional reason for approval
    pub reason: Option<String>,
    /// Auto-create default tenant for single-user workflows
    pub create_default_tenant: Option<bool>,
    /// Custom tenant name (if `create_default_tenant` is true)
    pub tenant_name: Option<String>,
    /// Custom tenant slug (if `create_default_tenant` is true)
    pub tenant_slug: Option<String>,
}

/// User suspension request
#[derive(Debug, Deserialize)]
pub struct SuspendUserRequest {
    /// Optional reason for suspension
    pub reason: Option<String>,
}

/// User deletion request
#[derive(Debug, Deserialize)]
pub struct DeleteUserRequest {
    /// Optional reason for deletion (for audit trail)
    pub reason: Option<String>,
}

/// Coach rejection request
#[derive(Debug, Deserialize)]
pub struct RejectCoachRequest {
    /// Reason for rejection (required to help author improve)
    pub reason: String,
}

/// Query parameters for listing pending coaches
#[derive(Debug, Deserialize)]
pub struct ListPendingCoachesQuery {
    /// Tenant ID to filter by (required for tenant-scoped operations)
    pub tenant_id: String,
    /// Maximum number of results (default: 50, max: 100)
    pub limit: Option<u32>,
    /// Offset for pagination
    pub offset: Option<u32>,
}

/// Query parameters for coach review operations
#[derive(Debug, Deserialize)]
pub struct CoachReviewQuery {
    /// Tenant ID (required for tenant-scoped operations)
    pub tenant_id: String,
}

/// Query parameters for listing API keys
#[derive(Debug, Deserialize)]
pub struct ListApiKeysQuery {
    /// Filter by user email
    pub user_email: Option<String>,
    /// Show only active keys
    pub active_only: Option<bool>,
    /// Maximum number of results (use String to allow invalid values that will be ignored)
    pub limit: Option<String>,
    /// Offset for pagination (use String to allow invalid values that will be ignored)
    pub offset: Option<String>,
}

/// Query parameters for user activity endpoint
#[derive(Debug, Deserialize)]
pub struct UserActivityQuery {
    /// Number of days to look back (default: 30)
    pub days: Option<u32>,
}

/// Request to update auto-approval setting
#[derive(Debug, Deserialize)]
pub struct UpdateAutoApprovalRequest {
    /// Whether auto-approval should be enabled
    pub enabled: bool,
}

/// Response for auto-approval setting
#[derive(Debug, Serialize)]
pub struct AutoApprovalResponse {
    /// Whether auto-approval is currently enabled
    pub enabled: bool,
    /// Description of the setting
    pub description: String,
}

/// Query parameters for listing users
#[derive(Debug, Deserialize)]
pub struct ListUsersQuery {
    /// Filter by status
    pub status: Option<String>,
    /// Maximum number of results
    pub limit: Option<i32>,
    /// Offset for pagination
    pub offset: Option<i32>,
}

/// API Key provisioning response
#[derive(Debug, Clone, Serialize)]
pub struct ProvisionApiKeyResponse {
    /// Whether the operation succeeded
    pub success: bool,
    /// Unique identifier for the API key
    pub api_key_id: String,
    /// The actual API key (shown only once)
    pub api_key: String,
    /// ID of the user who owns this key
    pub user_id: String,
    /// Tier level of the key
    pub tier: String,
    /// When the key expires (ISO 8601 format)
    pub expires_at: Option<String>,
    /// Rate limit configuration
    pub rate_limit: Option<RateLimitInfo>,
}

/// Rate limit information
#[derive(Debug, Clone, Serialize)]
pub struct RateLimitInfo {
    /// Maximum number of requests allowed
    pub requests: u32,
    /// Time period for the rate limit
    pub period: String,
}

/// Generic admin response
#[derive(Debug, Clone, Serialize)]
pub struct AdminResponse {
    /// Whether the operation succeeded
    pub success: bool,
    /// Response message
    pub message: String,
    /// Optional additional data
    pub data: Option<Value>,
}

/// Admin setup response
#[derive(Debug, Clone, Serialize)]
pub struct AdminSetupResponse {
    /// ID of the created admin user
    pub user_id: String,
    /// JWT token for admin authentication
    pub admin_token: String,
    /// Success message
    pub message: String,
}

/// Information about created tenant
#[derive(Debug, Clone, Serialize)]
pub struct TenantCreatedInfo {
    /// Unique tenant identifier
    pub tenant_id: String,
    /// Tenant name
    pub name: String,
    /// Tenant URL slug
    pub slug: String,
    /// Subscription plan
    pub plan: String,
}
