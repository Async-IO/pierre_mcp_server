// ABOUTME: Focused dependency injection contexts replacing the ServerResources service locator
// ABOUTME: Provides type-safe dependency injection with minimal coupling between components
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Focused dependency injection contexts
//!
//! This module replaces the `ServerResources` service locator anti-pattern with
//! focused contexts that provide only the dependencies needed for specific operations.
//!
//! # Architecture
//!
//! - `AuthContext`: Authentication and authorization dependencies
//! - `DataContext`: Database, cache, and data provider dependencies
//! - `ConfigContext`: Configuration and OAuth management dependencies
//! - `NotificationContext`: WebSocket and SSE notification dependencies
//! - `SecurityContext`: CSRF protection, PII redaction, and rate limiting
//! - `ExtensionContext`: Plugin execution and MCP protocol extensions

/// Authentication context with auth manager, middleware, and Firebase auth
pub mod auth;
/// Configuration context with OAuth, tenant settings, and admin config management
pub mod config;
/// Data context with database, cache, and provider access
pub mod data;
/// Extension context for plugins, sampling peer, and progress notifications
pub mod extension;
/// Notification context for WebSocket and SSE
pub mod notification;
/// Security context for CSRF, redaction, and rate limiting
pub mod security;
/// Server context combining all focused contexts
pub mod server;

/// Authentication context
pub use auth::AuthContext;
/// Configuration context
pub use config::ConfigContext;
/// Data access context
pub use data::DataContext;
/// Extension context
pub use extension::ExtensionContext;
/// Notification context
pub use notification::NotificationContext;
/// Security context
pub use security::SecurityContext;
/// Combined server context
pub use server::ServerContext;
