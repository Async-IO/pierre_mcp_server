// ABOUTME: Domain service layer for business logic extracted from route handlers
// ABOUTME: Provides protocol-agnostic services reusable across REST, MCP, and A2A
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Domain service layer
//!
//! This module contains protocol-agnostic business logic extracted from route handlers.
//! Services are designed to be reusable across REST, MCP, and A2A protocols, ensuring
//! consistent business rules regardless of the entry point.

/// Coach lifecycle operations: prerequisites, assignments, and generation
pub mod coaches;

/// OAuth flow orchestration: state validation, redirect URL parsing
pub mod oauth_flow;

/// Recipe import/export and markdown conversion
pub mod recipes;

/// Tenant administration: slug validation, tenant creation, user provisioning
pub mod tenant_admin;
