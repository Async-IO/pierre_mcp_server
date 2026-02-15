// ABOUTME: Security audit logging for OAuth operations and sensitive data access
// ABOUTME: Provides comprehensive audit trails for compliance and security investigation
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Security Audit Module
//!
//! Comprehensive audit logging for security-sensitive operations including:
//! - OAuth credential access and modifications
//! - Tenant operations and privilege escalations  
//! - API key usage and authentication events
//! - Encryption/decryption operations

use crate::database_plugins::factory::Database;
use crate::database_plugins::DatabaseProvider;
use crate::errors::AppResult;
use pierre_core::models::TenantId;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

// Re-export DTOs from pierre-core (canonical definitions)
pub use pierre_core::models::{AuditEvent, AuditEventType, AuditSeverity};

/// Audit logger for security events
pub struct SecurityAuditor {
    /// Database connection for storing audit events
    database: Arc<Database>,
}

impl SecurityAuditor {
    /// Create new security auditor
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }

    /// Log audit event to structured logger based on severity
    fn log_to_structured_logger(event: &AuditEvent) {
        match event.severity {
            AuditSeverity::Info => Self::log_info_event(event),
            AuditSeverity::Warning => Self::log_warning_event(event),
            AuditSeverity::Error => Self::log_error_event(event),
            AuditSeverity::Critical => Self::log_critical_event(event),
        }
    }

    fn log_info_event(event: &AuditEvent) {
        info!(
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

    fn log_warning_event(event: &AuditEvent) {
        warn!(
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

    fn log_error_event(event: &AuditEvent) {
        error!(
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

    fn log_critical_event(event: &AuditEvent) {
        error!(
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

    /// Log an audit event
    ///
    /// # Errors
    ///
    /// Returns an error if the audit event cannot be stored
    pub async fn log_event(&self, event: AuditEvent) -> AppResult<()> {
        // Log to structured logger first (for immediate visibility)
        Self::log_to_structured_logger(&event);

        // Store in database for persistence and analysis
        self.store_audit_event(&event).await?;

        // For critical events, also trigger alerts
        if matches!(event.severity, AuditSeverity::Critical) {
            Self::trigger_security_alert(&event);
        }

        Ok(())
    }

    /// Store audit event in database
    async fn store_audit_event(&self, event: &AuditEvent) -> AppResult<()> {
        // Store audit event in database
        self.database.store_audit_event(event).await?;

        debug!(
            "Stored audit event {} in database: {}",
            event.event_id, event.description
        );

        Ok(())
    }

    /// Trigger security alert for critical events
    fn trigger_security_alert(event: &AuditEvent) {
        // Log critical security events with structured format for monitoring systems
        error!(
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
    }

    /// Log OAuth credential access
    ///
    /// # Errors
    ///
    /// Returns an error if the audit event cannot be logged
    pub async fn log_oauth_credential_access(
        &self,
        tenant_id: TenantId,
        provider: &str,
        user_id: Option<Uuid>,
        source_ip: Option<String>,
    ) -> AppResult<()> {
        let event = AuditEvent::new(
            AuditEventType::OAuthCredentialsAccessed,
            AuditSeverity::Info,
            format!("OAuth credentials accessed for provider {provider}"),
            "access".to_owned(),
            "success".to_owned(),
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
        tenant_id: TenantId,
        provider: &str,
        user_id: Uuid,
        action: &str, // "created", "updated", "deleted"
        source_ip: Option<String>,
    ) -> AppResult<()> {
        let severity = match action {
            "deleted" => AuditSeverity::Warning,
            _ => AuditSeverity::Info,
        };

        let event = AuditEvent::new(
            AuditEventType::OAuthCredentialsModified,
            severity,
            format!("OAuth credentials {action} for provider {provider}"),
            action.to_owned(),
            "success".to_owned(),
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
        tenant_id: Option<TenantId>,
        success: bool,
        duration_ms: u64,
        source_ip: Option<String>,
    ) -> AppResult<()> {
        let (severity, result) = if success {
            (AuditSeverity::Info, "success")
        } else {
            (AuditSeverity::Warning, "failure")
        };

        let mut event = AuditEvent::new(
            AuditEventType::ToolExecuted,
            severity,
            format!("Tool '{tool_name}' executed"),
            "execute".to_owned(),
            result.to_owned(),
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
    ) -> AppResult<()> {
        let severity = if success {
            AuditSeverity::Info
        } else {
            AuditSeverity::Warning
        };

        let description = match (&event_type, success) {
            (AuditEventType::UserLogin, true) => "User successfully logged in".to_owned(),
            (AuditEventType::UserLogin, false) => "User login failed".to_owned(),
            (AuditEventType::ApiKeyUsed, true) => "API key authentication successful".to_owned(),
            (AuditEventType::ApiKeyUsed, false) => "API key authentication failed".to_owned(),
            _ => format!("Authentication event: {event_type:?}"),
        };

        let mut event = AuditEvent::new(
            event_type,
            severity,
            description,
            "authenticate".to_owned(),
            if success { "success" } else { "failure" }.to_owned(),
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
        tenant_id: Option<TenantId>,
        success: bool,
        error_details: Option<&str>,
    ) -> AppResult<()> {
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
            operation.to_owned(),
            if success { "success" } else { "failure" }.to_owned(),
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
            error!("Failed to log OAuth credential access audit: {}", e);
        }
    };
}

/// Macro to log a tool execution audit event with comprehensive details
#[macro_export]
macro_rules! audit_tool_execution {
    ($auditor:expr, $tool_name:expr, $user_id:expr, $tenant_id:expr, $success:expr, $duration:expr, $source_ip:expr) => {
        if let Err(e) = $auditor
            .log_tool_execution(
                $tool_name, $user_id, $tenant_id, $success, $duration, $source_ip,
            )
            .await
        {
            error!("Failed to log tool execution audit: {}", e);
        }
    };
}
