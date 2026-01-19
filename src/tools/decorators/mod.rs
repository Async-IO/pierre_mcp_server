// ABOUTME: Decorator module for MCP tools providing cross-cutting concerns.
// ABOUTME: Includes auditing and other transparent tool wrappers.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Tool Decorators
//!
//! Provides decorator implementations that wrap tools with additional
//! functionality without modifying the underlying tool logic:
//!
//! - `AuditedTool` - Logs tool executions for audit/security purposes
//!
//! These decorators follow the same pattern as `CachingFitnessProvider`
//! from `src/providers/caching_provider.rs`.
//!
//! ## Future Decorators
//!
//! - `MeteredTool` - Usage tracking for billing/quotas

mod audited;

pub use audited::AuditedTool;
