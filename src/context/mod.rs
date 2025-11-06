// ABOUTME: Focused dependency injection contexts replacing the ServerResources service locator
// ABOUTME: Provides type-safe dependency injection with minimal coupling between components
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Focused dependency injection contexts
//!
//! This module replaces the `ServerResources` service locator anti-pattern with
//! focused contexts that provide only the dependencies needed for specific operations.
//!
//! # Architecture
//!
//! - `AuthContext`: Authentication and authorization dependencies
//! - `DataContext`: Database and data provider dependencies
//! - `ConfigContext`: Configuration and OAuth management dependencies
//! - `NotificationContext`: WebSocket and SSE notification dependencies

/// Authentication context with auth manager and middleware
pub mod auth;
/// Configuration context with OAuth and settings management
pub mod config;
/// Data context with database and provider access
pub mod data;
/// Notification context for WebSocket and SSE
pub mod notification;
/// Server context combining all focused contexts
pub mod server;

/// Authentication context
pub use auth::AuthContext;
/// Configuration context
pub use config::ConfigContext;
/// Data access context
pub use data::DataContext;
/// Notification context
pub use notification::NotificationContext;
/// Combined server context
pub use server::ServerContext;
