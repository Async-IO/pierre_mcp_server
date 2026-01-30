<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Tool Development Guide

This guide explains how to create new MCP tools using the Pluggable Tools Architecture.

## Overview

The Pierre MCP Server uses a pluggable tool architecture where each tool:
- Implements the `McpTool` trait
- Declares capabilities via `ToolCapabilities` bitflags
- Is registered in the `ToolRegistry` at startup
- Can be conditionally compiled via feature flags

## Architecture

```
src/tools/
├── mod.rs              # Module exports and documentation
├── traits.rs           # McpTool trait and ToolCapabilities
├── registry.rs         # ToolRegistry for registration and lookup
├── context.rs          # ToolExecutionContext with resources
├── result.rs           # ToolResult and notifications
├── errors.rs           # ToolError types
├── decorators.rs       # AuditedTool wrapper
└── implementations/    # Tool implementations by category
    ├── mod.rs
    ├── data.rs         # get_activities, get_athlete, get_stats
    ├── analytics.rs    # training load, fitness score, patterns
    ├── coaches.rs      # coach CRUD operations
    ├── admin.rs        # admin-only coach management
    ├── goals.rs        # goal setting and tracking
    ├── configuration.rs # user configuration management
    ├── fitness_config.rs # fitness thresholds (FTP, zones)
    ├── nutrition.rs    # nutrition calculations
    ├── sleep.rs        # sleep analysis and recovery
    ├── recipes.rs      # recipe management
    └── connection.rs   # provider connection management
```

## Creating a New Tool

### Step 1: Implement the `McpTool` Trait

Create your tool in the appropriate category file under `src/tools/implementations/`:

```rust
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::errors::{AppError, AppResult};
use crate::mcp::schema::{JsonSchema, PropertySchema};
use crate::tools::{McpTool, ToolCapabilities, ToolExecutionContext, ToolResult};

/// Tool for calculating weekly training volume.
///
/// Analyzes activities from the past 7 days and returns
/// total duration, distance, and elevation gain.
pub struct CalculateWeeklyVolumeTool;

#[async_trait]
impl McpTool for CalculateWeeklyVolumeTool {
    fn name(&self) -> &'static str {
        "calculate_weekly_volume"
    }

    fn description(&self) -> &'static str {
        "Calculate total training volume (duration, distance, elevation) for the past 7 days"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();

        properties.insert(
            "sport_type".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("Filter by sport type (e.g., 'Run', 'Ride'). Optional.".to_owned()),
            },
        );

        properties.insert(
            "include_commutes".to_owned(),
            PropertySchema {
                property_type: "boolean".to_owned(),
                description: Some("Include commute activities. Defaults to false.".to_owned()),
            },
        );

        JsonSchema {
            schema_type: "object".to_owned(),
            properties,
            required: vec![], // No required parameters
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH
            | ToolCapabilities::REQUIRES_PROVIDER
            | ToolCapabilities::READS_DATA
            | ToolCapabilities::ANALYTICS
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        // Extract parameters with defaults
        let sport_type = args.get("sport_type").and_then(|v| v.as_str());
        let include_commutes = args.get("include_commutes")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Access database through context
        let pool = ctx.resources.database.sqlite_pool()
            .ok_or_else(|| AppError::internal("Database not available"))?;

        // Your business logic here...
        let total_duration_hours = 12.5;
        let total_distance_km = 85.0;
        let total_elevation_m = 1200;

        Ok(ToolResult::ok(json!({
            "period": "7 days",
            "sport_type": sport_type.unwrap_or("all"),
            "include_commutes": include_commutes,
            "totals": {
                "duration_hours": total_duration_hours,
                "distance_km": total_distance_km,
                "elevation_m": total_elevation_m
            }
        })))
    }
}
```

### Step 2: Create a Factory Function

At the bottom of your implementation file, create a factory function:

```rust
/// Create all weekly volume tools for registration
#[must_use]
pub fn create_weekly_volume_tools() -> Vec<Box<dyn McpTool>> {
    vec![
        Box::new(CalculateWeeklyVolumeTool),
        // Add more related tools here
    ]
}
```

### Step 3: Register in the ToolRegistry

In `src/tools/registry.rs`, add the registration:

```rust
// In register_builtin_tools():
#[cfg(feature = "tools-analytics")]
self.register_weekly_volume_tools();

// Add the registration method:
#[cfg(feature = "tools-analytics")]
fn register_weekly_volume_tools(&mut self) {
    use crate::tools::implementations::analytics::create_weekly_volume_tools;
    for tool in create_weekly_volume_tools() {
        self.register_with_category(Arc::from(tool), "analytics");
    }
}
```

### Step 4: Export from Module

In `src/tools/implementations/mod.rs`, export your factory:

```rust
pub use analytics::create_weekly_volume_tools;
```

## Tool Capabilities

Capabilities are bitflags that control tool visibility and behavior:

| Capability | Value | Description |
|------------|-------|-------------|
| `REQUIRES_AUTH` | `0x0001` | Tool requires authenticated user |
| `REQUIRES_TENANT` | `0x0002` | Tool requires tenant context |
| `REQUIRES_PROVIDER` | `0x0004` | Tool needs a connected fitness provider |
| `READS_DATA` | `0x0008` | Tool reads data (cacheable) |
| `WRITES_DATA` | `0x0010` | Tool modifies data (invalidates cache) |
| `ANALYTICS` | `0x0020` | Tool performs calculations/analysis |
| `GOALS` | `0x0040` | Tool manages goals |
| `CONFIGURATION` | `0x0080` | Tool manages configuration |
| `RECIPES` | `0x0100` | Tool manages recipes |
| `COACHES` | `0x0200` | Tool manages coaches |
| `ADMIN_ONLY` | `0x0400` | Tool requires admin privileges |
| `SLEEP_RECOVERY` | `0x0800` | Tool handles sleep/recovery data |

### Combining Capabilities

```rust
fn capabilities(&self) -> ToolCapabilities {
    // Read-only analytics tool
    ToolCapabilities::REQUIRES_AUTH
        | ToolCapabilities::READS_DATA
        | ToolCapabilities::ANALYTICS
}

fn capabilities(&self) -> ToolCapabilities {
    // Admin-only write tool
    ToolCapabilities::REQUIRES_AUTH
        | ToolCapabilities::ADMIN_ONLY
        | ToolCapabilities::WRITES_DATA
}
```

## ToolExecutionContext

The context provides access to resources and user information:

```rust
pub struct ToolExecutionContext {
    pub resources: Arc<ServerResources>,  // Database, config, providers
    pub user_id: Uuid,                    // Authenticated user
    pub tenant_id: Option<Uuid>,          // Multi-tenant context
    pub request_id: Option<String>,       // For tracing
    pub is_admin: bool,                   // Admin status (cached)
}
```

### Accessing Resources

```rust
async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
    // Database access
    let pool = ctx.resources.database.sqlite_pool()
        .ok_or_else(|| AppError::internal("Database not available"))?;

    // Check admin status
    if !ctx.is_admin() {
        return Err(AppError::forbidden("Admin access required"));
    }

    // Require tenant context
    let tenant_id = ctx.require_tenant()?;

    // Access configuration
    let config = &ctx.resources.config;

    // ...
}
```

## ToolResult

Return results using `ToolResult`:

```rust
// Success with JSON data
Ok(ToolResult::ok(json!({ "status": "success", "data": {...} })))

// Error result (still Ok, but indicates tool-level error)
Ok(ToolResult::error(json!({ "error": "Invalid date range" })))

// With notifications (for streaming updates)
let mut result = ToolResult::ok(json!({ "status": "complete" }));
result.add_notification(ToolNotification::progress(50, Some("Processing...")));
Ok(result)
```

## Error Handling

Use structured errors from `crate::errors`:

```rust
use crate::errors::{AppError, AppResult};

async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
    // Validation error
    let limit = args.get("limit")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| AppError::validation("limit must be a number"))?;

    // Not found
    let user = get_user(id).await
        .ok_or_else(|| AppError::not_found(format!("User {id}")))?;

    // Forbidden
    if !ctx.is_admin() {
        return Err(AppError::forbidden("Admin access required"));
    }

    // Internal error
    let result = external_api_call().await
        .map_err(|e| AppError::internal(format!("API call failed: {e}")))?;

    Ok(ToolResult::ok(json!({ "data": result })))
}
```

## Feature Flags

Tools can be conditionally compiled using feature flags in `Cargo.toml`:

```toml
[features]
default = ["tools-all"]

# Individual tool categories
tools-connection = []
tools-data = []
tools-analytics = []
tools-goals = []
tools-config = []
tools-nutrition = []
tools-sleep = []
tools-recipes = []
tools-coaches = []
tools-admin = []

# All tools
tools-all = [
    "tools-connection",
    "tools-data",
    "tools-analytics",
    "tools-goals",
    "tools-config",
    "tools-nutrition",
    "tools-sleep",
    "tools-recipes",
    "tools-coaches",
    "tools-admin",
]
```

### Using Feature Flags

```rust
// In registry.rs
#[cfg(feature = "tools-analytics")]
self.register_analytics_tools();

// In implementations
#[cfg(feature = "tools-analytics")]
pub mod analytics;
```

## Testing Tools

### Unit Tests

Test tool metadata and basic behavior:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_metadata() {
        let tool = CalculateWeeklyVolumeTool;

        assert_eq!(tool.name(), "calculate_weekly_volume");
        assert!(!tool.description().is_empty());
        assert!(tool.capabilities().contains(ToolCapabilities::REQUIRES_AUTH));
        assert!(tool.capabilities().contains(ToolCapabilities::READS_DATA));

        let schema = tool.input_schema();
        assert_eq!(schema.schema_type, "object");
    }
}
```

### Integration Tests

Test tool execution with real context:

```rust
#[tokio::test]
async fn test_tool_execution() {
    let resources = create_test_resources().await;
    let ctx = ToolExecutionContext::new(
        resources,
        Uuid::new_v4(),  // user_id
        Some(Uuid::new_v4()),  // tenant_id
        None,  // request_id
    );

    let tool = CalculateWeeklyVolumeTool;
    let args = json!({ "sport_type": "Run" });

    let result = tool.execute(args, &ctx).await.unwrap();
    assert!(result.is_success);
}
```

## Best Practices

1. **Use Direct Implementation**: Access business logic directly (e.g., `CoachesManager`) instead of wrapping HTTP handlers.

2. **Validate Early**: Check required parameters at the start of `execute()`.

3. **Use Structured Errors**: Never use `anyhow!()` - use `AppError` variants.

4. **Document Capabilities**: Choose capabilities carefully - they affect filtering and caching.

5. **Keep Tools Focused**: Each tool should do one thing well.

6. **Test Thoroughly**: Include unit tests for metadata and integration tests for execution.

## Example: Admin Tool

Admin tools have special handling - they're hidden from non-admin users:

```rust
pub struct AdminDeleteUserTool;

#[async_trait]
impl McpTool for AdminDeleteUserTool {
    fn name(&self) -> &'static str {
        "admin_delete_user"
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH
            | ToolCapabilities::ADMIN_ONLY  // Hidden from regular users
            | ToolCapabilities::WRITES_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        // Admin check is enforced by ToolRegistry, but double-check for safety
        if !ctx.is_admin() {
            return Err(AppError::forbidden("Admin access required"));
        }

        // ... admin logic
    }
}
```

## External Tool Registration

For tools defined outside the main crate:

```rust
use pierre_mcp_server::tools::{register_external_tool, McpTool};

// Register at startup
let my_tool: Arc<dyn McpTool> = Arc::new(MyExternalTool);
register_external_tool(my_tool);
```

## See Also

- `src/tools/traits.rs` - Full trait definitions
- `src/tools/registry.rs` - Registry implementation
- `tests/mcp_tools_unit_test.rs` - Tool test examples
- `tests/tool_registry_integration_test.rs` - Integration test examples
