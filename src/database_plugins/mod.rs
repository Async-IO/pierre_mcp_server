// ABOUTME: Database abstraction layer for Pierre MCP Server
// ABOUTME: Plugin architecture for database support with SQLite and PostgreSQL backends
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

// Re-export the A2A types from the main database module

/// A2A usage tracking record
pub use crate::database::A2AUsage;
/// A2A usage statistics
pub use crate::database::A2AUsageStats;

/// Database provider factory
pub mod factory;

/// PostgreSQL database implementation
#[cfg(feature = "postgresql")]
pub mod postgres;

/// Shared database logic (enum conversions, validation, mappers, encryption, etc.)
pub mod shared;
