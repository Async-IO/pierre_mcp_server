// ABOUTME: A2A (Agent-to-Agent) protocol module exports and organization
// ABOUTME: Provides client registration, authentication, and protocol compliance for A2A communication
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # A2A (Agent-to-Agent) Protocol Implementation
//!
//! This module implements the A2A protocol for Pierre, enabling agent-to-agent
//! communication and collaboration with other AI systems.

/// Agent card metadata and capabilities
pub mod agent_card;
/// A2A authentication and authorization
pub mod auth;
/// A2A client management
pub mod client;
/// A2A protocol types and server implementation
pub mod protocol;
/// System user management for A2A agents
pub mod system_user;

use crate::errors::AppError;

pub use agent_card::AgentCard;
pub use client::A2AClientManager;
pub use protocol::{A2AError, A2AErrorResponse, A2ARequest, A2AResponse, A2AServer};

/// A2A Protocol Version
pub const A2A_VERSION: &str = "1.0.0";

/// Helper function for mapping database errors to A2A errors
pub fn map_db_error(context: &str) -> impl Fn(AppError) -> A2AError + '_ {
    move |e| A2AError::InternalError(format!("{context}: {e}"))
}

/// Helper function for mapping database errors to A2A errors with string context
pub fn map_db_error_str(context: String) -> impl Fn(AppError) -> A2AError {
    move |e| A2AError::InternalError(format!("{context}: {e}"))
}
