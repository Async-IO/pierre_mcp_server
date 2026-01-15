// ABOUTME: Tool selection service for per-tenant MCP tool filtering
// ABOUTME: Computes effective tool list combining global disabling, plan restrictions, and tenant overrides
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::config::ToolSelectionConfig;
use crate::database_plugins::factory::Database;
use crate::database_plugins::DatabaseProvider;
use crate::errors::{AppError, AppResult};
use crate::models::{
    CategorySummary, EffectiveTool, TenantPlan, TenantToolOverride, ToolAvailabilitySummary,
    ToolCatalogEntry, ToolCategory, ToolEnablementSource,
};
use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

/// Cache size for tenant tool configurations (1000 tenants max)
const CACHE_SIZE: NonZeroUsize = NonZeroUsize::new(1000).unwrap(); // Safe: static data - compile-time const

/// Cache entry for effective tools per tenant
struct CacheEntry {
    tools: Vec<EffectiveTool>,
    cached_at: Instant,
}

/// Service for computing and caching effective tool lists per tenant
///
/// The service applies tool enablement in the following precedence order:
/// 1. **Global Disabled** (`PIERRE_DISABLED_TOOLS`) - Highest priority
/// 2. **Plan Restriction** - Tools require minimum plan level
/// 3. **Tenant Override** - Admin-configured per-tenant settings
/// 4. **Catalog Default** - Default enablement from `tool_catalog` table
pub struct ToolSelectionService {
    database: Arc<Database>,
    cache: Arc<RwLock<LruCache<Uuid, CacheEntry>>>,
    cache_ttl: Duration,
    /// Global tool selection configuration from environment
    config: ToolSelectionConfig,
}

impl ToolSelectionService {
    /// Create a new `ToolSelectionService` with the given database connection
    ///
    /// Loads tool selection configuration from environment variables.
    #[must_use]
    pub fn new(database: Arc<Database>) -> Self {
        let config = ToolSelectionConfig::from_env();
        Self::with_config(database, config)
    }

    /// Create a new `ToolSelectionService` with explicit configuration
    ///
    /// This constructor is useful for testing or when configuration
    /// should be managed externally.
    #[must_use]
    pub fn with_config(database: Arc<Database>, config: ToolSelectionConfig) -> Self {
        if config.has_disabled_tools() {
            info!(
                "Tool selection initialized with {} globally disabled tools: {:?}",
                config.disabled_count(),
                config.disabled_tools()
            );
        }

        Self {
            database,
            cache: Arc::new(RwLock::new(LruCache::new(CACHE_SIZE))),
            cache_ttl: Duration::from_secs(300),
            config,
        }
    }

    /// Create a new `ToolSelectionService` with custom cache TTL
    #[must_use]
    pub fn with_ttl(database: Arc<Database>, cache_ttl: Duration) -> Self {
        let config = ToolSelectionConfig::from_env();
        Self {
            database,
            cache: Arc::new(RwLock::new(LruCache::new(CACHE_SIZE))),
            cache_ttl,
            config,
        }
    }

    /// Get the list of globally disabled tool names
    ///
    /// Returns an empty vector if no tools are globally disabled.
    #[must_use]
    pub fn get_globally_disabled_tools(&self) -> Vec<String> {
        self.config.disabled_tools().iter().cloned().collect()
    }

    /// Check if any tools are globally disabled
    #[must_use]
    pub fn has_globally_disabled_tools(&self) -> bool {
        self.config.has_disabled_tools()
    }

    /// Get effective tools for a tenant (uses cache if available)
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail
    pub async fn get_effective_tools(&self, tenant_id: Uuid) -> AppResult<Vec<EffectiveTool>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.peek(&tenant_id) {
                if entry.cached_at.elapsed() < self.cache_ttl {
                    debug!("Tool selection cache hit for tenant {tenant_id}");
                    return Ok(entry.tools.clone());
                }
            }
        }

        // Cache miss - compute effective tools
        debug!("Tool selection cache miss for tenant {tenant_id}");
        let tools = self.compute_effective_tools(tenant_id).await?;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.put(
                tenant_id,
                CacheEntry {
                    tools: tools.clone(),
                    cached_at: Instant::now(),
                },
            );
        }

        Ok(tools)
    }

    /// Get only enabled tools for a tenant (for `tools/list` endpoint)
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail
    pub async fn get_enabled_tools(&self, tenant_id: Uuid) -> AppResult<Vec<EffectiveTool>> {
        let all_tools = self.get_effective_tools(tenant_id).await?;
        Ok(all_tools.into_iter().filter(|t| t.is_enabled).collect())
    }

    /// Check if a specific tool is enabled for a tenant
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The tool doesn't exist in the catalog
    /// - Database operations fail
    pub async fn is_tool_enabled(&self, tenant_id: Uuid, tool_name: &str) -> AppResult<bool> {
        // Check global disabled first (highest precedence)
        if self.config.is_globally_disabled(tool_name) {
            return Ok(false);
        }

        // Optimized path: check if we have cached data
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.peek(&tenant_id) {
                if entry.cached_at.elapsed() < self.cache_ttl {
                    return Ok(entry
                        .tools
                        .iter()
                        .any(|t| t.tool_name == tool_name && t.is_enabled));
                }
            }
        }

        // Cache miss - fetch just what we need for this tool
        let catalog_entry = self
            .database
            .get_tool_catalog_entry(tool_name)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Tool '{tool_name}'")))?;

        let tenant = self.database.get_tenant_by_id(tenant_id).await?;
        let tenant_plan = TenantPlan::parse_str(&tenant.plan)
            .ok_or_else(|| AppError::internal(format!("Invalid tenant plan: {}", tenant.plan)))?;

        // Check plan restriction
        if !tenant_plan.meets_minimum(&catalog_entry.min_plan) {
            return Ok(false);
        }

        // Check tenant override
        if let Some(override_entry) = self
            .database
            .get_tenant_tool_override(tenant_id, tool_name)
            .await?
        {
            return Ok(override_entry.is_enabled);
        }

        // Fall back to catalog default
        Ok(catalog_entry.is_enabled_by_default)
    }

    /// Set a tool override for a tenant (admin operation)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The tool doesn't exist in the catalog
    /// - Database operations fail
    pub async fn set_tool_override(
        &self,
        tenant_id: Uuid,
        tool_name: &str,
        is_enabled: bool,
        admin_user_id: Uuid,
        reason: Option<String>,
    ) -> AppResult<TenantToolOverride> {
        // Validate tool exists
        self.database
            .get_tool_catalog_entry(tool_name)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Tool '{tool_name}'")))?;

        // Upsert override
        let override_entry = self
            .database
            .upsert_tenant_tool_override(
                tenant_id,
                tool_name,
                is_enabled,
                Some(admin_user_id),
                reason,
            )
            .await?;

        // Invalidate cache
        self.invalidate_tenant(tenant_id).await;

        Ok(override_entry)
    }

    /// Remove a tool override (revert to catalog default)
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail
    pub async fn remove_tool_override(&self, tenant_id: Uuid, tool_name: &str) -> AppResult<bool> {
        let deleted = self
            .database
            .delete_tenant_tool_override(tenant_id, tool_name)
            .await?;

        // Invalidate cache
        self.invalidate_tenant(tenant_id).await;

        Ok(deleted)
    }

    /// Get tool availability summary for a tenant
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail
    pub async fn get_availability_summary(
        &self,
        tenant_id: Uuid,
    ) -> AppResult<ToolAvailabilitySummary> {
        let tools = self.get_effective_tools(tenant_id).await?;

        let total_tools = tools.len();
        let enabled_tools = tools.iter().filter(|t| t.is_enabled).count();
        let plan_restricted_tools = tools
            .iter()
            .filter(|t| t.source == ToolEnablementSource::PlanRestriction)
            .count();
        let overridden_tools = tools
            .iter()
            .filter(|t| t.source == ToolEnablementSource::TenantOverride)
            .count();

        // Group by category
        let mut category_map: HashMap<ToolCategory, (usize, usize)> = HashMap::new();
        for tool in &tools {
            let entry = category_map.entry(tool.category).or_insert((0, 0));
            entry.0 += 1;
            if tool.is_enabled {
                entry.1 += 1;
            }
        }

        let by_category = category_map
            .into_iter()
            .map(|(category, (total, enabled))| CategorySummary {
                category,
                total,
                enabled,
            })
            .collect();

        Ok(ToolAvailabilitySummary {
            total_tools,
            enabled_tools,
            plan_restricted_tools,
            overridden_tools,
            by_category,
        })
    }

    /// Invalidate cache for a specific tenant
    pub async fn invalidate_tenant(&self, tenant_id: Uuid) {
        self.cache.write().await.pop(&tenant_id);
        debug!("Invalidated tool selection cache for tenant {tenant_id}");
    }

    /// Invalidate entire cache (for admin operations affecting all tenants)
    pub async fn invalidate_all(&self) {
        self.cache.write().await.clear();
        debug!("Invalidated all tool selection cache entries");
    }

    /// Get the tool catalog (all tools regardless of tenant)
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail
    pub async fn get_catalog(&self) -> AppResult<Vec<ToolCatalogEntry>> {
        self.database.get_tool_catalog().await
    }

    /// Compute effective tools for a tenant (no caching)
    async fn compute_effective_tools(&self, tenant_id: Uuid) -> AppResult<Vec<EffectiveTool>> {
        // Get tenant to determine plan
        let tenant = self.database.get_tenant_by_id(tenant_id).await?;
        let tenant_plan = TenantPlan::parse_str(&tenant.plan)
            .ok_or_else(|| AppError::internal(format!("Invalid tenant plan: {}", tenant.plan)))?;

        // Load full catalog
        let catalog: Vec<ToolCatalogEntry> = self.database.get_tool_catalog().await?;

        // Load tenant overrides
        let overrides: Vec<TenantToolOverride> =
            self.database.get_tenant_tool_overrides(tenant_id).await?;
        let override_map: HashMap<String, bool> = overrides
            .into_iter()
            .map(|o| (o.tool_name, o.is_enabled))
            .collect();

        // Compute effective tools
        let effective_tools = catalog
            .into_iter()
            .map(|entry| {
                let is_globally_disabled = self.config.is_globally_disabled(&entry.tool_name);
                let (is_enabled, source) = Self::compute_enablement(
                    &entry,
                    tenant_plan,
                    override_map.get(&entry.tool_name),
                    is_globally_disabled,
                );

                EffectiveTool {
                    tool_name: entry.tool_name,
                    display_name: entry.display_name,
                    description: entry.description,
                    category: entry.category,
                    is_enabled,
                    source,
                    min_plan: entry.min_plan,
                }
            })
            .collect();

        Ok(effective_tools)
    }

    /// Compute enablement state for a single tool
    ///
    /// Precedence order (highest to lowest):
    /// 1. Global disabled (`PIERRE_DISABLED_TOOLS`)
    /// 2. Plan restriction
    /// 3. Tenant override
    /// 4. Catalog default
    const fn compute_enablement(
        entry: &ToolCatalogEntry,
        tenant_plan: TenantPlan,
        override_value: Option<&bool>,
        is_globally_disabled: bool,
    ) -> (bool, ToolEnablementSource) {
        // Global disabled takes highest precedence
        if is_globally_disabled {
            return (false, ToolEnablementSource::GlobalDisabled);
        }

        // Plan restriction next
        if !tenant_plan.meets_minimum(&entry.min_plan) {
            return (false, ToolEnablementSource::PlanRestriction);
        }

        // Tenant override next
        if let Some(&is_enabled) = override_value {
            return (is_enabled, ToolEnablementSource::TenantOverride);
        }

        // Fall back to catalog default
        (entry.is_enabled_by_default, ToolEnablementSource::Default)
    }
}
