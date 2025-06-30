//! Unified tool execution engine for Pierre MCP Server
//!
//! This module provides a shared tool execution engine that can be used
//! by both single-tenant and multi-tenant MCP implementations, eliminating
//! code duplication and providing a single source of truth for tool logic.

pub mod engine;
pub mod providers;
pub mod responses;