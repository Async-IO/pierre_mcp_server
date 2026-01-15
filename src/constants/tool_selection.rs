// ABOUTME: Tool selection constants for cache configuration and feature flags
// ABOUTME: Controls per-tenant tool filtering behavior and performance tuning
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

/// Default cache size for tenant tool configurations (number of tenants)
pub const DEFAULT_CACHE_SIZE: usize = 1_000;

/// Default cache TTL in seconds (5 minutes)
pub const DEFAULT_CACHE_TTL_SECS: u64 = 300;

/// Whether tool selection is enabled by default
pub const DEFAULT_ENABLED: bool = true;

/// Maximum number of tool overrides allowed per tenant
pub const MAX_OVERRIDES_PER_TENANT: usize = 100;

/// Minimum cache TTL in seconds (to prevent cache thrashing)
pub const MIN_CACHE_TTL_SECS: u64 = 10;

/// Maximum cache TTL in seconds (24 hours)
pub const MAX_CACHE_TTL_SECS: u64 = 86_400;
