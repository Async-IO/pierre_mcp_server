// ABOUTME: Security audit event types for compliance and investigation
// ABOUTME: AuditEventType, AuditSeverity, and AuditEvent DTOs with builder pattern
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::TenantId;

/// Types of audit events tracked by the system
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    // Authentication Events
    /// User successfully logged in
    UserLogin,
    /// User logged out
    UserLogout,
    /// Authentication attempt failed
    AuthenticationFailed,
    /// API key was used for authentication
    ApiKeyUsed,

    // OAuth Events
    /// OAuth credentials were accessed/read
    OAuthCredentialsAccessed,
    /// OAuth credentials were modified
    OAuthCredentialsModified,
    /// OAuth credentials were created
    OAuthCredentialsCreated,
    /// OAuth credentials were deleted
    OAuthCredentialsDeleted,
    /// OAuth token was refreshed
    TokenRefreshed,

    // Tenant Events
    /// New tenant was created
    TenantCreated,
    /// Tenant details were modified
    TenantModified,
    /// Tenant was deleted
    TenantDeleted,
    /// User was added to tenant
    TenantUserAdded,
    /// User was removed from tenant
    TenantUserRemoved,
    /// User's role in tenant was changed
    TenantUserRoleChanged,

    // Encryption Events
    /// Data was encrypted
    DataEncrypted,
    /// Data was decrypted
    DataDecrypted,
    /// Encryption key was rotated
    KeyRotated,
    /// Encryption operation failed
    EncryptionFailed,

    // Tool Execution Events
    /// Tool was executed successfully
    ToolExecuted,
    /// Tool execution failed
    ToolExecutionFailed,
    /// External provider API was called
    ProviderApiCalled,

    // Administrative Events
    /// System configuration was changed
    ConfigurationChanged,
    /// System maintenance was performed
    SystemMaintenance,
    /// Security policy was violated
    SecurityPolicyViolation,
}

/// Severity levels for audit events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditSeverity {
    /// Informational event (normal operation)
    Info,
    /// Warning event (potential issue)
    Warning,
    /// Error event (operation failed)
    Error,
    /// Critical event (security incident)
    Critical,
}

/// Security audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event identifier
    pub event_id: Uuid,
    /// Type of audit event
    pub event_type: AuditEventType,
    /// Severity level
    pub severity: AuditSeverity,
    /// Timestamp of the event
    pub timestamp: DateTime<Utc>,
    /// User ID who performed the action (if applicable)
    pub user_id: Option<Uuid>,
    /// Tenant ID associated with the event (if applicable)
    pub tenant_id: Option<TenantId>,
    /// Source IP address (if available)
    pub source_ip: Option<String>,
    /// User agent string (if available)
    pub user_agent: Option<String>,
    /// Session ID (if applicable)
    pub session_id: Option<String>,
    /// Event description
    pub description: String,
    /// Additional event metadata
    pub metadata: serde_json::Value,
    /// Resource affected by the event (e.g., "tenant:123", "`oauth_app:456`")
    pub resource: Option<String>,
    /// Action performed (e.g., "create", "update", "delete", "access")
    pub action: String,
    /// Result of the action (e.g., "success", "failure", "denied")
    pub result: String,
}

impl AuditEvent {
    /// Create a new audit event
    #[must_use]
    pub fn new(
        event_type: AuditEventType,
        severity: AuditSeverity,
        description: String,
        action: String,
        result: String,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            event_type,
            severity,
            timestamp: Utc::now(),
            user_id: None,
            tenant_id: None,
            source_ip: None,
            user_agent: None,
            session_id: None,
            description,
            metadata: serde_json::Value::Null,
            resource: None,
            action,
            result,
        }
    }

    /// Set user ID for the event
    #[must_use]
    pub const fn with_user_id(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Set tenant ID for the event
    #[must_use]
    pub const fn with_tenant_id(mut self, tenant_id: TenantId) -> Self {
        self.tenant_id = Some(tenant_id);
        self
    }

    /// Set source IP address
    #[must_use]
    pub fn with_source_ip(mut self, source_ip: String) -> Self {
        self.source_ip = Some(source_ip);
        self
    }

    /// Set user agent
    #[must_use]
    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    /// Set session ID
    #[must_use]
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Set resource affected
    #[must_use]
    pub fn with_resource(mut self, resource: String) -> Self {
        self.resource = Some(resource);
        self
    }

    /// Add metadata
    #[must_use]
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}
