// ABOUTME: Security audit logging for OAuth operations and sensitive data access
// ABOUTME: Provides comprehensive audit trails for compliance and security investigation

//! # Security Audit Module
//!
//! Comprehensive audit logging for security-sensitive operations including:
//! - OAuth credential access and modifications
//! - Tenant operations and privilege escalations  
//! - API key usage and authentication events
//! - Encryption/decryption operations

use crate::database_plugins::DatabaseProvider;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Types of audit events tracked by the system
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    // Authentication Events
    UserLogin,
    UserLogout,
    AuthenticationFailed,
    ApiKeyUsed,

    // OAuth Events
    OAuthCredentialsAccessed,
    OAuthCredentialsModified,
    OAuthCredentialsCreated,
    OAuthCredentialsDeleted,
    TokenRefreshed,

    // Tenant Events
    TenantCreated,
    TenantModified,
    TenantDeleted,
    TenantUserAdded,
    TenantUserRemoved,
    TenantUserRoleChanged,

    // Encryption Events
    DataEncrypted,
    DataDecrypted,
    KeyRotated,
    EncryptionFailed,

    // Tool Execution Events
    ToolExecuted,
    ToolExecutionFailed,
    ProviderApiCalled,

    // Administrative Events
    ConfigurationChanged,
    SystemMaintenance,
    SecurityPolicyViolation,
}

/// Severity levels for audit events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditSeverity {
    Info,
    Warning,
    Error,
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
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// User ID who performed the action (if applicable)
    pub user_id: Option<Uuid>,
    /// Tenant ID associated with the event (if applicable)
    pub tenant_id: Option<Uuid>,
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
            timestamp: chrono::Utc::now(),
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
    pub const fn with_tenant_id(mut self, tenant_id: Uuid) -> Self {
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

/// Audit logger for security events
pub struct SecurityAuditor {
    /// Database connection for storing audit events
    #[allow(dead_code)]
    database: Arc<crate::database_plugins::factory::Database>,
}

impl SecurityAuditor {
    /// Create new security auditor
    #[must_use]
    pub const fn new(database: Arc<crate::database_plugins::factory::Database>) -> Self {
        Self { database }
    }

    /// Log an audit event
    ///
    /// # Errors
    ///
    /// Returns an error if the audit event cannot be stored
    #[allow(clippy::unused_async)]
    pub async fn log_event(&self, event: AuditEvent) -> Result<()> {
        // Log to structured logger first (for immediate visibility)
        match event.severity {
            AuditSeverity::Info => {
                tracing::info!(
                    event_id = %event.event_id,
                    event_type = ?event.event_type,
                    user_id = ?event.user_id,
                    tenant_id = ?event.tenant_id,
                    resource = ?event.resource,
                    action = %event.action,
                    result = %event.result,
                    "Security audit event: {}",
                    event.description
                );
            }
            AuditSeverity::Warning => {
                tracing::warn!(
                    event_id = %event.event_id,
                    event_type = ?event.event_type,
                    user_id = ?event.user_id,
                    tenant_id = ?event.tenant_id,
                    resource = ?event.resource,
                    action = %event.action,
                    result = %event.result,
                    "Security audit warning: {}",
                    event.description
                );
            }
            AuditSeverity::Error => {
                tracing::error!(
                    event_id = %event.event_id,
                    event_type = ?event.event_type,
                    user_id = ?event.user_id,
                    tenant_id = ?event.tenant_id,
                    resource = ?event.resource,
                    action = %event.action,
                    result = %event.result,
                    "Security audit error: {}",
                    event.description
                );
            }
            AuditSeverity::Critical => {
                tracing::error!(
                    event_id = %event.event_id,
                    event_type = ?event.event_type,
                    user_id = ?event.user_id,
                    tenant_id = ?event.tenant_id,
                    resource = ?event.resource,
                    action = %event.action,
                    result = %event.result,
                    "CRITICAL security audit event: {}",
                    event.description
                );
            }
        }

        // Store in database for persistence and analysis
        self.store_audit_event(&event)?;

        // For critical events, also trigger alerts
        if matches!(event.severity, AuditSeverity::Critical) {
            self.trigger_security_alert(&event)?;
        }

        Ok(())
    }

    /// Store audit event in database
    fn store_audit_event(&self, event: &AuditEvent) -> Result<()> {
        // Use async runtime to call the database method
        let rt = tokio::runtime::Handle::current();
        rt.block_on(self.database.store_audit_event(event))?;

        tracing::debug!(
            "Stored audit event {} in database: {}",
            event.event_id,
            event.description
        );

        Ok(())
    }

    /// Trigger security alert for critical events
    #[allow(clippy::unused_self, clippy::unnecessary_wraps)]
    fn trigger_security_alert(&self, event: &AuditEvent) -> Result<()> {
        // Log critical security events with structured format for monitoring systems
        tracing::error!(
            target: "security_alert",
            event_id = %event.event_id,
            event_type = ?event.event_type,
            user_id = ?event.user_id,
            source_ip = ?event.source_ip,
            description = %event.description,
            "SECURITY ALERT: {}", event.description
        );

        // In production, this would integrate with:
        // - Email notification service (SendGrid, AWS SES)
        // - Slack/Teams webhooks for immediate alerts
        // - PagerDuty for critical incidents
        // - SIEM systems for security monitoring

        Ok(())
    }

    /// Log OAuth credential access
    ///
    /// # Errors
    ///
    /// Returns an error if the audit event cannot be logged
    pub async fn log_oauth_credential_access(
        &self,
        tenant_id: Uuid,
        provider: &str,
        user_id: Option<Uuid>,
        source_ip: Option<String>,
    ) -> Result<()> {
        let event = AuditEvent::new(
            AuditEventType::OAuthCredentialsAccessed,
            AuditSeverity::Info,
            format!("OAuth credentials accessed for provider {provider}"),
            "access".to_string(),
            "success".to_string(),
        )
        .with_tenant_id(tenant_id)
        .with_resource(format!("oauth_credentials:{tenant_id}:{provider}"))
        .with_metadata(serde_json::json!({
            "provider": provider,
        }));

        let event = if let Some(uid) = user_id {
            event.with_user_id(uid)
        } else {
            event
        };

        let event = if let Some(ip) = source_ip {
            event.with_source_ip(ip)
        } else {
            event
        };

        self.log_event(event).await
    }

    /// Log OAuth credential modification
    ///
    /// # Errors
    ///
    /// Returns an error if the audit event cannot be logged
    pub async fn log_oauth_credential_modification(
        &self,
        tenant_id: Uuid,
        provider: &str,
        user_id: Uuid,
        action: &str, // "created", "updated", "deleted"
        source_ip: Option<String>,
    ) -> Result<()> {
        let severity = match action {
            "deleted" => AuditSeverity::Warning,
            _ => AuditSeverity::Info,
        };

        let event = AuditEvent::new(
            AuditEventType::OAuthCredentialsModified,
            severity,
            format!("OAuth credentials {action} for provider {provider}"),
            action.to_string(),
            "success".to_string(),
        )
        .with_tenant_id(tenant_id)
        .with_user_id(user_id)
        .with_resource(format!("oauth_credentials:{tenant_id}:{provider}"))
        .with_metadata(serde_json::json!({
            "provider": provider,
            "modification_type": action,
        }));

        let event = if let Some(ip) = source_ip {
            event.with_source_ip(ip)
        } else {
            event
        };

        self.log_event(event).await
    }

    /// Log tool execution
    ///
    /// # Errors
    ///
    /// Returns an error if the audit event cannot be logged
    pub async fn log_tool_execution(
        &self,
        tool_name: &str,
        user_id: Uuid,
        tenant_id: Option<Uuid>,
        success: bool,
        duration_ms: u64,
        source_ip: Option<String>,
    ) -> Result<()> {
        let (severity, result) = if success {
            (AuditSeverity::Info, "success")
        } else {
            (AuditSeverity::Warning, "failure")
        };

        let mut event = AuditEvent::new(
            AuditEventType::ToolExecuted,
            severity,
            format!("Tool '{tool_name}' executed"),
            "execute".to_string(),
            result.to_string(),
        )
        .with_user_id(user_id)
        .with_resource(format!("tool:{tool_name}"))
        .with_metadata(serde_json::json!({
            "tool_name": tool_name,
            "duration_ms": duration_ms,
            "success": success,
        }));

        if let Some(tid) = tenant_id {
            event = event.with_tenant_id(tid);
        }

        if let Some(ip) = source_ip {
            event = event.with_source_ip(ip);
        }

        self.log_event(event).await
    }

    /// Log authentication event
    ///
    /// # Errors
    ///
    /// Returns an error if the audit event cannot be logged
    pub async fn log_authentication_event(
        &self,
        event_type: AuditEventType,
        user_id: Option<Uuid>,
        source_ip: Option<String>,
        user_agent: Option<String>,
        success: bool,
        details: Option<&str>,
    ) -> Result<()> {
        let severity = if success {
            AuditSeverity::Info
        } else {
            AuditSeverity::Warning
        };

        let description = match (&event_type, success) {
            (AuditEventType::UserLogin, true) => "User successfully logged in".to_string(),
            (AuditEventType::UserLogin, false) => "User login failed".to_string(),
            (AuditEventType::ApiKeyUsed, true) => "API key authentication successful".to_string(),
            (AuditEventType::ApiKeyUsed, false) => "API key authentication failed".to_string(),
            _ => format!("Authentication event: {event_type:?}"),
        };

        let mut event = AuditEvent::new(
            event_type,
            severity,
            description,
            "authenticate".to_string(),
            if success { "success" } else { "failure" }.to_string(),
        );

        if let Some(uid) = user_id {
            event = event.with_user_id(uid);
        }

        if let Some(ip) = source_ip {
            event = event.with_source_ip(ip);
        }

        if let Some(ua) = user_agent {
            event = event.with_user_agent(ua);
        }

        if let Some(details) = details {
            event = event.with_metadata(serde_json::json!({
                "details": details,
            }));
        }

        self.log_event(event).await
    }

    /// Log encryption/decryption event
    ///
    /// # Errors
    ///
    /// Returns an error if the audit event cannot be logged
    pub async fn log_encryption_event(
        &self,
        operation: &str, // "encrypt" or "decrypt"
        tenant_id: Option<Uuid>,
        success: bool,
        error_details: Option<&str>,
    ) -> Result<()> {
        let event_type = if operation == "encrypt" {
            AuditEventType::DataEncrypted
        } else {
            AuditEventType::DataDecrypted
        };

        let severity = if success {
            AuditSeverity::Info
        } else {
            AuditSeverity::Error
        };

        let description = if success {
            format!("Data {operation} successfully")
        } else {
            format!("Data {operation} failed")
        };

        let mut event = AuditEvent::new(
            event_type,
            severity,
            description,
            operation.to_string(),
            if success { "success" } else { "failure" }.to_string(),
        );

        if let Some(tid) = tenant_id {
            event = event.with_tenant_id(tid);
        }

        if let Some(error) = error_details {
            event = event.with_metadata(serde_json::json!({
                "error": error,
            }));
        }

        self.log_event(event).await
    }
}

/// Convenience macros for common audit operations
#[macro_export]
macro_rules! audit_oauth_access {
    ($auditor:expr, $tenant_id:expr, $provider:expr, $user_id:expr, $source_ip:expr) => {
        if let Err(e) = $auditor
            .log_oauth_credential_access($tenant_id, $provider, $user_id, $source_ip)
            .await
        {
            tracing::error!("Failed to log OAuth credential access audit: {}", e);
        }
    };
}

#[macro_export]
macro_rules! audit_tool_execution {
    ($auditor:expr, $tool_name:expr, $user_id:expr, $tenant_id:expr, $success:expr, $duration:expr, $source_ip:expr) => {
        if let Err(e) = $auditor
            .log_tool_execution(
                $tool_name, $user_id, $tenant_id, $success, $duration, $source_ip,
            )
            .await
        {
            tracing::error!("Failed to log tool execution audit: {}", e);
        }
    };
}
