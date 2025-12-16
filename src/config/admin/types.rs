// ABOUTME: Type definitions for admin configuration management
// ABOUTME: Defines parameter types, categories, validation, and API request/response structures
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Data type for configuration parameters
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigDataType {
    /// Floating point number
    Float,
    /// Integer number
    Integer,
    /// Boolean true/false
    Boolean,
    /// Text string
    String,
    /// Enumeration with specific allowed values
    Enum,
}

impl ConfigDataType {
    /// Convert to database representation
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Float => "float",
            Self::Integer => "integer",
            Self::Boolean => "boolean",
            Self::String => "string",
            Self::Enum => "enum",
        }
    }

    /// Parse from database representation
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "float" => Some(Self::Float),
            "integer" => Some(Self::Integer),
            "boolean" => Some(Self::Boolean),
            "string" => Some(Self::String),
            "enum" => Some(Self::Enum),
            _ => None,
        }
    }
}

/// Valid range constraint for numeric parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterRange {
    /// Minimum allowed value (inclusive)
    pub min: serde_json::Value,
    /// Maximum allowed value (inclusive)
    pub max: serde_json::Value,
    /// Step increment for UI sliders (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
}

/// Admin-configurable parameter with full metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfigParameter {
    /// Unique key identifier (e.g., `rate_limit.free_tier_burst`)
    pub key: String,
    /// Human-readable display name
    pub display_name: String,
    /// Detailed description of what this parameter controls
    pub description: String,
    /// Category this parameter belongs to
    pub category: String,
    /// Data type for validation
    pub data_type: ConfigDataType,
    /// Current effective value
    pub current_value: serde_json::Value,
    /// Default value from environment or code
    pub default_value: serde_json::Value,
    /// Whether the current value differs from default
    pub is_modified: bool,
    /// Valid range for numeric types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_range: Option<ParameterRange>,
    /// Allowed values for enum types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_options: Option<Vec<String>>,
    /// Unit of measurement (e.g., "% max HR", "km", "hours")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units: Option<String>,
    /// Scientific basis or reference for the default value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scientific_basis: Option<String>,
    /// Environment variable name if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_variable: Option<String>,
    /// Whether this can be changed at runtime without restart
    pub is_runtime_configurable: bool,
    /// Whether changing this requires server restart
    pub requires_restart: bool,
}

/// Configuration category for organizing parameters in the UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfigCategory {
    /// Unique identifier (e.g., `rate_limiting`)
    pub id: String,
    /// Internal name for API use
    pub name: String,
    /// Human-readable display name
    pub display_name: String,
    /// Description of the category
    pub description: String,
    /// Display order in UI (lower = first)
    pub display_order: i32,
    /// Icon identifier for UI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Whether this category is active
    pub is_active: bool,
    /// Parameters in this category (always serialized, even if empty)
    #[serde(default)]
    pub parameters: Vec<AdminConfigParameter>,
}

/// Stored configuration override in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigOverride {
    /// Database record ID
    pub id: String,
    /// Category of the parameter
    pub category: String,
    /// Parameter key
    pub config_key: String,
    /// JSON-encoded value
    pub config_value: serde_json::Value,
    /// Data type
    pub data_type: ConfigDataType,
    /// Tenant ID (None for system-wide)
    pub tenant_id: Option<String>,
    /// User who created/updated this override
    pub created_by: String,
    /// When created
    pub created_at: DateTime<Utc>,
    /// When last updated
    pub updated_at: DateTime<Utc>,
    /// Reason for the override
    pub reason: Option<String>,
}

/// Audit log entry for configuration changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigAuditEntry {
    /// Audit record ID
    pub id: String,
    /// When the change occurred
    pub timestamp: DateTime<Utc>,
    /// Admin user who made the change
    pub admin_user_id: String,
    /// Admin email for display
    pub admin_email: String,
    /// Category of the changed parameter
    pub category: String,
    /// Parameter key that was changed
    pub config_key: String,
    /// Previous value (None for new settings)
    pub old_value: Option<serde_json::Value>,
    /// New value
    pub new_value: serde_json::Value,
    /// Data type
    pub data_type: ConfigDataType,
    /// Reason for the change
    pub reason: Option<String>,
    /// Tenant ID if tenant-specific
    pub tenant_id: Option<String>,
    /// Client IP address
    pub ip_address: Option<String>,
    /// Client user agent
    pub user_agent: Option<String>,
}

// ============================================================================
// API Request/Response Types
// ============================================================================

/// Response containing the full configuration catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigCatalogResponse {
    /// All configuration categories with their parameters
    pub categories: Vec<AdminConfigCategory>,
    /// Total number of parameters
    pub total_parameters: usize,
    /// Number of runtime-configurable parameters
    pub runtime_configurable_count: usize,
    /// Number of static (restart-required) parameters
    pub static_count: usize,
    /// Schema version for client compatibility
    pub version: String,
}

/// Request to update configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfigRequest {
    /// Map of parameter keys to new values
    pub parameters: HashMap<String, serde_json::Value>,
    /// Optional reason for the changes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Validation error for a single parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigValidationError {
    /// Parameter key that failed validation
    pub parameter: String,
    /// Error message
    pub message: String,
    /// The invalid value that was provided
    pub provided_value: serde_json::Value,
    /// Valid range if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_range: Option<ParameterRange>,
}

/// Response after updating configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfigResponse {
    /// Whether all updates succeeded
    pub success: bool,
    /// Number of parameters updated
    pub updated_count: usize,
    /// List of validation errors if any
    pub validation_errors: Vec<ConfigValidationError>,
    /// Whether any changes require server restart
    pub requires_restart: bool,
    /// When the changes became effective
    pub effective_at: DateTime<Utc>,
}

/// Request to validate configuration before applying
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateConfigRequest {
    /// Map of parameter keys to proposed values
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Validation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateConfigResponse {
    /// Whether all values are valid
    pub is_valid: bool,
    /// List of validation errors
    pub errors: Vec<ConfigValidationError>,
    /// Warnings (valid but potentially problematic)
    pub warnings: Vec<String>,
}

/// Filter options for audit log queries
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConfigAuditFilter {
    /// Filter by category
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Filter by parameter key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_key: Option<String>,
    /// Filter by admin user ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin_user_id: Option<String>,
    /// Filter by tenant ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    /// Start timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_timestamp: Option<DateTime<Utc>>,
    /// End timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_timestamp: Option<DateTime<Utc>>,
}

/// Paginated audit log response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigAuditResponse {
    /// Audit entries
    pub entries: Vec<ConfigAuditEntry>,
    /// Total count for pagination
    pub total_count: usize,
    /// Current page offset
    pub offset: usize,
    /// Page size limit
    pub limit: usize,
}

/// Configuration export format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigExportData {
    /// Export timestamp
    pub exported_at: DateTime<Utc>,
    /// Schema version
    pub version: String,
    /// Tenant ID if tenant-specific export
    pub tenant_id: Option<String>,
    /// All configuration overrides
    pub overrides: Vec<ConfigOverride>,
    /// Categories for reference
    pub categories: Vec<AdminConfigCategory>,
}

/// Request to reset configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetConfigRequest {
    /// Specific category to reset (None = all)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Specific keys to reset (None = all in category)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keys: Option<Vec<String>>,
    /// Reason for reset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Response after reset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetConfigResponse {
    /// Whether reset succeeded
    pub success: bool,
    /// Number of parameters reset
    pub reset_count: usize,
    /// Keys that were reset
    pub reset_keys: Vec<String>,
}
