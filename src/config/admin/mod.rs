// ABOUTME: Admin configuration management module
// ABOUTME: Provides types, database operations, and service layer for runtime configuration
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Admin Configuration Management
//!
//! This module provides a complete system for managing runtime configuration
//! parameters through an admin API. It supports:
//!
//! - **Type-safe parameters**: Float, Integer, Boolean, String, and Enum types
//! - **Validation**: Range checking, enum validation, type validation
//! - **Multi-tenant**: System-wide defaults with per-tenant overrides
//! - **Audit logging**: All changes are logged with user, timestamp, and reason
//! - **Hot reload**: Configuration changes take effect without server restart
//!
//! # Architecture
//!
//! ```text
//! AdminConfigService (service.rs)
//!        │
//!        ├── Parameter Definitions (in-memory)
//!        │   └── ParameterDefinition structs with metadata
//!        │
//!        ├── Cache (in-memory)
//!        │   └── Effective values for quick access
//!        │
//!        └── AdminConfigManager (manager.rs)
//!            └── Database operations (SQLite)
//!                ├── admin_config_overrides table
//!                ├── admin_config_audit table
//!                └── admin_config_categories table
//! ```
//!
//! # Example Usage
//!
//! ```text
//! // Create service
//! let service = AdminConfigService::new(pool).await?;
//!
//! // Get full catalog
//! let catalog = service.get_catalog(None).await?;
//!
//! // Update configuration
//! let request = UpdateConfigRequest {
//!     parameters: [("rate_limit.free_tier_burst".to_string(), json!(20))].into(),
//!     reason: Some("Increased for load testing".to_string()),
//! };
//! let response = service.update_config(&request, admin_id, admin_email, None, None, None).await?;
//!
//! // Get specific value
//! let value = service.get_value("rate_limit.free_tier_burst", None).await?;
//! ```

/// Type definitions for admin configuration
pub mod types;

/// Database operations for configuration management
pub mod manager;

/// Configuration service with caching and hot reload
pub mod service;

// Re-export main types for convenience
pub use manager::AdminConfigManager;
pub use service::{AdminConfigService, ParameterDefinition};
pub use types::{
    AdminConfigCategory, AdminConfigParameter, ConfigAuditEntry, ConfigAuditFilter,
    ConfigAuditResponse, ConfigCatalogResponse, ConfigDataType, ConfigExportData, ConfigOverride,
    ConfigValidationError, ParameterRange, ResetConfigRequest, ResetConfigResponse,
    UpdateConfigRequest, UpdateConfigResponse, ValidateConfigRequest, ValidateConfigResponse,
};
