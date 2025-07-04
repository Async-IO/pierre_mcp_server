// ABOUTME: A2A (Agent-to-Agent) protocol module exports and organization
// ABOUTME: Provides client registration, authentication, and protocol compliance for A2A communication
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # A2A (Agent-to-Agent) Protocol Implementation
//!
//! This module implements the A2A protocol for Pierre, enabling agent-to-agent
//! communication and collaboration with other AI systems.

pub mod agent_card;
pub mod auth;
pub mod client;
pub mod protocol;
pub mod system_user;

#[cfg(test)]
pub mod test_utils;

pub use agent_card::AgentCard;
pub use auth::A2AClient;
pub use auth::{A2AAuthenticator, A2AToken};
pub use client::A2AClientManager;
pub use protocol::{A2AMessage, A2ARequest, A2AResponse, A2AServer};
pub use system_user::A2ASystemUserService;

/// A2A protocol version supported by Pierre
pub const A2A_VERSION: &str = "1.0";

/// A2A content types
pub const A2A_CONTENT_TYPE: &str = "application/json";

/// A2A protocol errors
#[derive(Debug, thiserror::Error)]
pub enum A2AError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Invalid request format: {0}")]
    InvalidRequest(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Client not registered: {0}")]
    ClientNotRegistered(String),

    #[error("Rate limit exceeded for client: {0}")]
    RateLimitExceeded(String),

    #[error("Internal server error: {0}")]
    InternalError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}

impl warp::reject::Reject for A2AError {}

/// Helper function for mapping database errors to A2A errors
pub fn map_db_error(context: &str) -> impl Fn(anyhow::Error) -> A2AError + '_ {
    move |e| A2AError::InternalError(format!("{}: {}", context, e))
}

/// Helper function for mapping database errors to A2A errors with string context
pub fn map_db_error_str(context: String) -> impl Fn(anyhow::Error) -> A2AError {
    move |e| A2AError::InternalError(format!("{}: {}", context, e))
}
