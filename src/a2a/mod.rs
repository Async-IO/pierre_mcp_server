// ABOUTME: A2A (Agent-to-Agent) protocol module exports and organization
// ABOUTME: Provides client registration, authentication, and protocol compliance for A2A communication
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! # A2A (Agent-to-Agent) Protocol Implementation
//!
//! This module implements the A2A protocol for Pierre, enabling agent-to-agent
//! communication and collaboration with other AI systems.

pub mod agent_card;
pub mod auth;
pub mod client;
pub mod protocol;
pub mod system_user;

pub use agent_card::AgentCard;
pub use client::A2AClientManager;
pub use protocol::{A2AError, A2AErrorResponse, A2ARequest, A2AResponse, A2AServer};

/// A2A Protocol Version
pub const A2A_VERSION: &str = "1.0.0";

impl warp::reject::Reject for A2AError {}

/// Helper function for mapping database errors to A2A errors
pub fn map_db_error(context: &str) -> impl Fn(anyhow::Error) -> A2AError + '_ {
    move |e| A2AError::InternalError(format!("{context}: {e}"))
}

/// Helper function for mapping database errors to A2A errors with string context
pub fn map_db_error_str(context: String) -> impl Fn(anyhow::Error) -> A2AError {
    move |e| A2AError::InternalError(format!("{context}: {e}"))
}
