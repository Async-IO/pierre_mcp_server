// ABOUTME: Tool selection configuration for per-tenant MCP tool filtering
// ABOUTME: Configures cache behavior and feature enablement via environment variables
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::constants::tool_selection;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;

/// Configuration for the tool selection service
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ToolSelectionConfig {
    /// Whether tool selection filtering is enabled
    /// When disabled, all tools are available to all tenants
    pub enabled: bool,
    /// Cache size (number of tenant configurations to cache)
    pub cache_size: usize,
    /// Cache TTL in seconds
    pub cache_ttl_secs: u64,
    /// Maximum tool overrides allowed per tenant
    pub max_overrides_per_tenant: usize,
}

impl Default for ToolSelectionConfig {
    fn default() -> Self {
        Self {
            enabled: tool_selection::DEFAULT_ENABLED,
            cache_size: tool_selection::DEFAULT_CACHE_SIZE,
            cache_ttl_secs: tool_selection::DEFAULT_CACHE_TTL_SECS,
            max_overrides_per_tenant: tool_selection::MAX_OVERRIDES_PER_TENANT,
        }
    }
}

impl ToolSelectionConfig {
    /// Load tool selection configuration from environment variables
    ///
    /// Environment variables:
    /// - `TOOL_SELECTION_ENABLED`: Enable/disable tool selection (default: true)
    /// - `TOOL_SELECTION_CACHE_SIZE`: Cache size in entries (default: 1000)
    /// - `TOOL_SELECTION_CACHE_TTL_SECS`: Cache TTL in seconds (default: 300)
    /// - `TOOL_SELECTION_MAX_OVERRIDES`: Max overrides per tenant (default: 100)
    #[must_use]
    pub fn from_env() -> Self {
        let cache_ttl_secs = env::var("TOOL_SELECTION_CACHE_TTL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .map_or(tool_selection::DEFAULT_CACHE_TTL_SECS, Self::clamp_ttl);

        Self {
            enabled: env::var("TOOL_SELECTION_ENABLED")
                .ok()
                .map_or(tool_selection::DEFAULT_ENABLED, |s| {
                    s.to_lowercase() == "true" || s == "1"
                }),
            cache_size: env::var("TOOL_SELECTION_CACHE_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(tool_selection::DEFAULT_CACHE_SIZE),
            cache_ttl_secs,
            max_overrides_per_tenant: env::var("TOOL_SELECTION_MAX_OVERRIDES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(tool_selection::MAX_OVERRIDES_PER_TENANT),
        }
    }

    /// Clamp TTL to valid range to prevent cache thrashing or excessive staleness
    const fn clamp_ttl(ttl: u64) -> u64 {
        if ttl < tool_selection::MIN_CACHE_TTL_SECS {
            tool_selection::MIN_CACHE_TTL_SECS
        } else if ttl > tool_selection::MAX_CACHE_TTL_SECS {
            tool_selection::MAX_CACHE_TTL_SECS
        } else {
            ttl
        }
    }

    /// Get cache TTL as a Duration
    #[must_use]
    pub const fn cache_ttl(&self) -> Duration {
        Duration::from_secs(self.cache_ttl_secs)
    }
}
