// ABOUTME: Domain models for per-tenant MCP tool selection and configuration
// ABOUTME: Enables admins to customize which tools are exposed to MCP clients
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Tool categories for logical grouping in the catalog
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    /// Core fitness data access (`get_activities`, `get_athlete`, `get_stats`)
    Fitness,
    /// Analytics and intelligence (`analyze_activity`, `training_load`, `trends`)
    Analysis,
    /// Goal setting and progress tracking
    Goals,
    /// Nutrition calculation and USDA food database
    Nutrition,
    /// Recipe management and validation
    Recipes,
    /// Sleep analysis and recovery metrics
    Sleep,
    /// User and system configuration
    Configuration,
    /// OAuth provider connections
    Connections,
}

impl ToolCategory {
    /// Parse category from string representation
    #[must_use]
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "fitness" => Some(Self::Fitness),
            "analysis" => Some(Self::Analysis),
            "goals" => Some(Self::Goals),
            "nutrition" => Some(Self::Nutrition),
            "recipes" => Some(Self::Recipes),
            "sleep" => Some(Self::Sleep),
            "configuration" => Some(Self::Configuration),
            "connections" => Some(Self::Connections),
            _ => None,
        }
    }

    /// Convert enum to string
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Fitness => "fitness",
            Self::Analysis => "analysis",
            Self::Goals => "goals",
            Self::Nutrition => "nutrition",
            Self::Recipes => "recipes",
            Self::Sleep => "sleep",
            Self::Configuration => "configuration",
            Self::Connections => "connections",
        }
    }
}

/// Tenant subscription plan levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TenantPlan {
    /// Basic plan with essential tools
    Starter,
    /// Professional plan with advanced analytics
    Professional,
    /// Enterprise plan with all features
    Enterprise,
}

impl TenantPlan {
    /// Parse plan level from string representation
    #[must_use]
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "starter" => Some(Self::Starter),
            "professional" => Some(Self::Professional),
            "enterprise" => Some(Self::Enterprise),
            _ => None,
        }
    }

    /// Convert enum to string
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Starter => "starter",
            Self::Professional => "professional",
            Self::Enterprise => "enterprise",
        }
    }

    /// Check if this plan meets the minimum required plan
    #[must_use]
    pub const fn meets_minimum(&self, minimum: &Self) -> bool {
        // Ord is derived, so we can compare directly
        // Starter(0) < Professional(1) < Enterprise(2)
        matches!(
            (self, minimum),
            (Self::Enterprise, _)
                | (Self::Professional, Self::Starter | Self::Professional)
                | (Self::Starter, Self::Starter)
        )
    }
}

/// Tool catalog entry from the `tool_catalog` table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCatalogEntry {
    /// Unique identifier for this catalog entry
    pub id: Uuid,
    /// Tool name matching `ToolId::name()` (e.g., `get_activities`)
    pub tool_name: String,
    /// Human-friendly display name
    pub display_name: String,
    /// Tool description for documentation
    pub description: String,
    /// Category for logical grouping
    pub category: ToolCategory,
    /// Whether this tool is enabled by default for new tenants
    pub is_enabled_by_default: bool,
    /// Provider requirement (None if no specific provider required)
    pub requires_provider: Option<String>,
    /// Minimum subscription plan required for this tool
    pub min_plan: TenantPlan,
    /// When this catalog entry was created
    pub created_at: DateTime<Utc>,
    /// When this catalog entry was last updated
    pub updated_at: DateTime<Utc>,
}

/// Per-tenant tool override from the `tenant_tool_overrides` table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantToolOverride {
    /// Unique identifier for this override
    pub id: Uuid,
    /// Tenant this override applies to
    pub tenant_id: Uuid,
    /// Tool name being overridden
    pub tool_name: String,
    /// Whether the tool is enabled (overrides catalog default)
    pub is_enabled: bool,
    /// User who created this override (for audit)
    pub enabled_by_user_id: Option<Uuid>,
    /// Reason for the override (for audit)
    pub reason: Option<String>,
    /// When this override was created
    pub created_at: DateTime<Utc>,
    /// When this override was last updated
    pub updated_at: DateTime<Utc>,
}

/// Source of tool enablement state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolEnablementSource {
    /// Enabled state from `tool_catalog.is_enabled_by_default`
    Default,
    /// Enabled state from `tenant_tool_overrides`
    TenantOverride,
    /// Disabled because tenant plan doesn't meet `min_plan` requirement
    PlanRestriction,
    /// Disabled globally via `PIERRE_DISABLED_TOOLS` environment variable
    GlobalDisabled,
}

/// Effective tool state for a tenant (catalog + overrides + plan applied)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectiveTool {
    /// Tool name matching `ToolId::name()`
    pub tool_name: String,
    /// Human-friendly display name
    pub display_name: String,
    /// Tool description
    pub description: String,
    /// Category for grouping
    pub category: ToolCategory,
    /// Whether the tool is currently enabled for this tenant
    pub is_enabled: bool,
    /// Source of the enablement decision
    pub source: ToolEnablementSource,
    /// Minimum plan required (for display purposes)
    pub min_plan: TenantPlan,
}

/// Request to set a tool override for a tenant
#[derive(Debug, Clone, Deserialize)]
pub struct SetToolOverrideRequest {
    /// Whether to enable or disable the tool
    pub is_enabled: bool,
    /// Optional reason for the override
    pub reason: Option<String>,
}

/// Summary of tool availability for a tenant
#[derive(Debug, Clone, Serialize)]
pub struct ToolAvailabilitySummary {
    /// Total tools in catalog
    pub total_tools: usize,
    /// Tools enabled for this tenant
    pub enabled_tools: usize,
    /// Tools disabled by plan restrictions
    pub plan_restricted_tools: usize,
    /// Tools with tenant-specific overrides
    pub overridden_tools: usize,
    /// Breakdown by category
    pub by_category: Vec<CategorySummary>,
}

/// Tool summary for a specific category
#[derive(Debug, Clone, Serialize)]
pub struct CategorySummary {
    /// Category name
    pub category: ToolCategory,
    /// Total tools in this category
    pub total: usize,
    /// Enabled tools in this category
    pub enabled: usize,
}
