// ABOUTME: Main library entry point for Pierre fitness API platform
// ABOUTME: Provides MCP, A2A, and REST API protocols for fitness data analysis
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

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
//! use pierre_mcp_server::providers::{FitnessProvider, AuthData};
//! use pierre_mcp_server::config::environment::ServerConfig;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Load configuration
//!     let config = ServerConfig::from_env()?;
//!     
//!     // Use TenantProviderFactory for provider creation
//!     // let factory = TenantProviderFactory::new(oauth_client);
//!     // let mut provider = factory.create_provider("strava", &tenant_context).await?;
//!     
//!     // Authenticate with OAuth2
//!     let auth_data = AuthData::OAuth2 {
//!         client_id: "your_client_id".to_string(),
//!         client_secret: "your_client_secret".to_string(),
//!         access_token: Some("access_token".to_string()),
//!         refresh_token: Some("refresh_token".to_string()),
//!     };
//!     provider.authenticate(auth_data).await?;
//!     
//!     // Get athlete data
//!     let athlete = provider.get_athlete().await?;
//!     println!("Athlete: {}", athlete.username);
//!     
//!     Ok(())
//! }
//! ```

/// Fitness provider implementations for various services
pub mod providers;

/// Common data models for fitness data
pub mod models;

/// Configuration management and persistence
pub mod config;

/// Application constants and configuration values
pub mod constants;

/// `OAuth2` client for secure `API` authentication
pub mod oauth2_client;

/// Model Context Protocol server implementation
pub mod mcp;

/// Athlete Intelligence for activity analysis and insights
pub mod intelligence;

/// Configuration management and runtime parameter system
pub mod configuration;

/// `HTTP` routes for configuration management
pub mod configuration_routes;

/// A2A (Agent-to-Agent) protocol implementation
pub mod a2a;

/// `HTTP` routes for A2A protocol endpoints
pub mod a2a_routes;

/// Multi-tenant database management (legacy)
pub mod database;

/// Database abstraction layer with plugin support  
pub mod database_plugins;

/// Authentication and session management
pub mod auth;

/// Cryptographic utilities and key management
pub mod crypto;

/// `HTTP` routes for user registration and `OAuth` flows
pub mod routes;

/// Multi-tenant management REST API routes
pub mod tenant_routes;

/// Production logging and structured output
pub mod logging;

/// Health checks and monitoring
pub mod health;

/// `API` key management for B2B authentication
pub mod api_keys;

/// `HTTP` routes for `API` key management
pub mod api_key_routes;

/// Dashboard routes for frontend consumption
pub mod dashboard_routes;

/// WebSocket support for real-time updates
pub mod websocket;

/// Security headers and protection middleware
pub mod security;

/// Admin token authentication and `API` key provisioning
pub mod admin;

/// Admin REST `API` routes for external services
pub mod admin_routes;

/// Universal protocol support for MCP and A2A
pub mod protocols;

/// Unified OAuth management for all fitness providers
pub mod oauth;

/// Unified rate limiting system for API keys and JWT tokens
pub mod rate_limiting;

/// Rate limiting middleware with HTTP headers and proper error responses
pub mod rate_limiting_middleware;

/// Unified error handling system with standard error codes and HTTP responses
pub mod errors;

/// Unified tool execution engine for fitness analysis and data processing
pub mod tools;

// Utility modules
pub mod tenant;
pub mod utils;
