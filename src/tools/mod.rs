// ABOUTME: Unified tool execution engine providing fitness analysis and data processing tools
// ABOUTME: Central tool registry for MCP protocol tools, A2A tools, and fitness intelligence operations
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
//! Unified tool execution engine for Pierre MCP Server
//!
//! This module provides a shared tool execution engine that can be used
//! by both single-tenant and multi-tenant MCP implementations, eliminating
//! code duplication and providing a single source of truth for tool logic.

/// Tool execution engine core
pub mod engine;
/// Provider-specific tool implementations
pub mod providers;
/// Tool response formatting utilities
pub mod responses;
