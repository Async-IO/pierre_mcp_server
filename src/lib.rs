// ABOUTME: Main library entry point for Pierre fitness API platform
// ABOUTME: Provides MCP, A2A, and REST API protocols for fitness data analysis
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// Crate-level attributes:
// - recursion_limit: Increased from default 128 to 256 for complex derive macros
//   (serde, thiserror) on deeply nested types like protocol responses
// - deny(unsafe_code): Zero-tolerance unsafe policy. Any exception requires
//   approval via scripts/ci/architectural-validation.sh (e.g., src/health.rs Windows FFI)
#![recursion_limit = "256"]
#![deny(unsafe_code)]

//! # Pierre MCP Server
//!
//! A Model Context Protocol (MCP) server for fitness data aggregation and analysis.
//! This server provides a unified interface to access fitness data from various providers
//! like Strava and Fitbit through the MCP protocol.
//!
//! ## Features
//!
//! - **Multi-provider support**: Connect to Strava, Fitbit, and more
//! - **`OAuth2` authentication**: Secure authentication flow for fitness providers
//! - **MCP protocol**: Standard interface for Claude and other AI assistants
//! - **Real-time data**: Access to activities, athlete profiles, and statistics
//! - **Extensible architecture**: Easy to add new fitness providers
//!
//! ## Quick Start
//!
//! 1. Set up authentication credentials using the `auth-setup` binary
//! 2. Start the MCP server with `pierre-mcp-server`
//! 3. Connect from Claude or other MCP clients
//!
//! ## Architecture
//!
//! The server follows a modular architecture:
//! - **Providers**: Abstract fitness provider implementations
//! - **Models**: Common data structures for fitness data
//! - **MCP**: Model Context Protocol server implementation
//! - **`OAuth2`**: Authentication client for secure API access
//! - **Config**: Configuration management and persistence
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use pierre_mcp_server::config::environment::ServerConfig;
//! use pierre_mcp_server::errors::AppResult;
//!
//! #[tokio::main]
//! async fn main() -> AppResult<()> {
//!     // Load configuration
//!     let config = ServerConfig::from_env()?;
//!
//!     // Start Pierre MCP Server with loaded configuration
//!     println!("Pierre MCP Server configured with port: HTTP={}",
//!              config.http_port);
//!
//!     Ok(())
//! }
//! ```

// ── Public API ──────────────────────────────────────────────────────────
// These modules are used by binary crates (src/bin/) and integration tests (tests/).
// They must remain `pub` so external consumers can access them.

/// A2A (Agent-to-Agent) protocol implementation
pub mod a2a;

/// `HTTP` routes for A2A protocol endpoints
pub mod a2a_routes;

/// Admin token authentication and `API` key provisioning
pub mod admin;

/// `HTTP` routes for `API` key management
pub mod api_key_routes;

/// `API` key management for B2B authentication
pub mod api_keys;

/// Authentication and session management
pub mod auth;

/// Cache abstraction layer with pluggable backends
pub mod cache;

/// Coach definition parsing from markdown files
pub mod coaches;

/// Configuration management and persistence
pub mod config;

/// Application constants and configuration values
pub mod constants;

/// Focused dependency injection contexts
pub mod context;

/// Cryptographic utilities and key management
pub mod crypto;

/// Dashboard routes for frontend consumption
pub mod dashboard_routes;

/// Multi-tenant database management
pub mod database;

/// Database abstraction layer with plugin support
pub mod database_plugins;

/// Unified error handling system with standard error codes and HTTP responses
pub mod errors;

/// External API clients (USDA, weather services)
pub mod external;

/// Feature flag configuration and validation
pub mod features;

/// Output format abstraction (JSON, TOON) for efficient LLM serialization
pub mod formatters;

/// Health checks and monitoring
pub mod health;

/// Insight sample parsing from markdown files for validation testing
pub mod insight_samples;

/// Athlete Intelligence for activity analysis and insights
pub mod intelligence;

/// Unified JSON-RPC 2.0 foundation for all protocols
pub mod jsonrpc;

/// Two-tier key management system for secure database encryption
pub mod key_management;

/// LLM provider abstraction for AI chat integration
pub mod llm;

/// Production logging and structured output
pub mod logging;

/// Model Context Protocol server implementation
pub mod mcp;

/// HTTP middleware for request tracing and tenant context
pub mod middleware;

/// Common data models for fitness data
pub mod models;

/// OAuth 2.0 client (Pierre as client to fitness providers)
pub mod oauth2_client;

/// OAuth 2.0 authorization server (Pierre as provider for MCP clients)
pub mod oauth2_server;

/// Cursor-based pagination for efficient data traversal
pub mod pagination;

/// Role-based permission system with `super_admin`, `admin`, `user` hierarchy
pub mod permissions;

/// Compile-time plugin system for extensible tool architecture
pub mod plugins;

/// Universal protocol support for MCP and A2A
pub mod protocols;

/// Fitness provider implementations for various services
pub mod providers;

/// Unified rate limiting system for API keys and JWT tokens
pub mod rate_limiting;

/// `HTTP` routes for user registration and `OAuth` flows
pub mod routes;

/// Security headers and protection middleware
pub mod security;

/// Server-Sent Events (SSE) for real-time streaming
#[cfg(feature = "transport-sse")]
pub mod sse;

/// Multi-tenant data isolation and scoping
pub mod tenant;

/// Unified tool execution engine for fitness analysis and data processing
pub mod tools;

/// Common type definitions and shared types
pub mod types;

/// Utility functions and helpers
pub mod utils;

/// WebSocket support for real-time updates
#[cfg(feature = "transport-websocket")]
pub mod websocket;

/// Test utilities for creating consistent test data
#[cfg(any(test, feature = "testing"))]
pub mod test_utils;

// ── Internal modules ────────────────────────────────────────────────────
// These modules are implementation details not referenced by binaries or tests.
// They use `pub(crate)` to prevent external access.

/// Plugin lifecycle management for deterministic initialization
pub mod lifecycle;

/// Domain service layer for protocol-agnostic business logic
pub(crate) mod services;

/// Multi-tenant management REST API routes
pub mod tenant_routes;
