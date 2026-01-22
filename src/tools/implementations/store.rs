// ABOUTME: Coach Store MCP tools for browsing, searching, and installing coaches.
// ABOUTME: Implements browse_store, search_store, install_coach, uninstall_coach tools.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Coach Store MCP Tools
//!
//! This module provides MCP tools for interacting with the Coach Store:
//! - `BrowseStoreTool` - Browse published coaches with filters
//! - `SearchStoreTool` - Search coaches by query
//! - `InstallCoachTool` - Install a coach from the store
//! - `UninstallCoachTool` - Uninstall a previously installed coach

use crate::tools::traits::McpTool;

/// Create all store tools for registration
#[must_use]
pub fn create_store_tools() -> Vec<Box<dyn McpTool>> {
    Vec::new()
}
