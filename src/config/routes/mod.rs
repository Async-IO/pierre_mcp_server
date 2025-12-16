// ABOUTME: HTTP route handlers for configuration management
// ABOUTME: Re-exports configuration and fitness configuration route modules
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! HTTP routes for configuration management
//!
//! This module provides REST API endpoints for managing:
//! - Runtime configuration parameters and profiles
//! - Fitness-specific configurations with tenant isolation

/// Admin configuration API route handlers
pub mod admin;
/// Configuration management route handlers and request/response types
pub mod configuration;
/// Fitness-specific configuration route handlers with tenant isolation
pub mod fitness;

// Re-export route handlers for convenience
pub use admin::{admin_config_router, AdminConfigState};
pub use configuration::ConfigurationRoutes;
pub use fitness::FitnessConfigurationRoutes;
