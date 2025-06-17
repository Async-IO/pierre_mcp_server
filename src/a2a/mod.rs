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

pub use agent_card::AgentCard;
pub use auth::A2AClient;
pub use auth::{A2AAuthenticator, A2AToken};
pub use client::A2AClientManager;
pub use protocol::{A2AMessage, A2ARequest, A2AResponse, A2AServer};

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
