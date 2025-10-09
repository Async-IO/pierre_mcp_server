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

pub mod auth;
pub mod config;
pub mod data;
pub mod notification;
pub mod server;

pub use auth::AuthContext;
pub use config::ConfigContext;
pub use data::DataContext;
pub use notification::NotificationContext;
pub use server::ServerContext;
