// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Pierre MCP Server
//!
//! A Model Context Protocol (MCP) server for fitness data aggregation and analysis.
//! This server provides a unified interface to access fitness data from various providers
//! like Strava, Garmin Connect, and RunKeeper through the MCP protocol.
//!
//! ## Features
//!
//! - **Multi-provider support**: Connect to Strava, Garmin, RunKeeper, and more
//! - **OAuth2 authentication**: Secure authentication flow for fitness providers
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
//! - **OAuth2**: Authentication client for secure API access
//! - **Config**: Configuration management and persistence
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use pierre_mcp_server::providers::{FitnessProvider, create_provider, AuthData};
//! use pierre_mcp_server::config::Config;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Load configuration
//!     let config = Config::load(None)?;
//!     
//!     // Create provider
//!     let mut provider = create_provider("strava")?;
//!     
//!     // Authenticate
//!     if let Some(strava_config) = config.providers.get("strava") {
//!         let auth_data = AuthData::OAuth2 {
//!             client_id: strava_config.client_id.clone().unwrap(),
//!             client_secret: strava_config.client_secret.clone().unwrap(),
//!             access_token: strava_config.access_token.clone(),
//!             refresh_token: strava_config.refresh_token.clone(),
//!         };
//!         provider.authenticate(auth_data).await?;
//!     }
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

/// OAuth2 client for secure API authentication
pub mod oauth2_client;

/// Model Context Protocol server implementation
pub mod mcp;

/// Athlete Intelligence for activity analysis and insights
pub mod intelligence;

/// Multi-tenant database management
pub mod database;

/// Authentication and session management
pub mod auth;

/// HTTP routes for user registration and OAuth flows
pub mod routes;

/// Production logging and structured output
pub mod logging;

/// Health checks and monitoring
pub mod health;