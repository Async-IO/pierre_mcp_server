# Tool Development Guide

Guide for creating custom fitness analysis tools in Pierre MCP Server.

## Architecture

Pierre uses a type-safe tool registry system (src/protocols/universal/tool_registry.rs) that eliminates string-based routing and provides compile-time safety.

### Tool Components

1. **Tool ID** - Enum variant in `ToolId` (src/protocols/universal/tool_registry.rs:12-45)
2. **Handler Function** - Implementation in `src/protocols/universal/handlers/`
3. **Schema Definition** - JSON Schema for input parameters
4. **Registration** - Entry in tool dispatch map

## Creating a New Tool

### Step 1: Add Tool ID

Edit `src/protocols/universal/tool_registry.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolId {
    // ... existing tools ...

    // Your new tool
    AnalyzeNutrition,
}
```

### Step 2: Implement Tool Name Mapping

Add to `from_name()` (src/protocols/universal/tool_registry.rs:48-80):

```rust
pub fn from_name(name: &str) -> Option<Self> {
    match name {
        // ... existing mappings ...
        "analyze_nutrition" => Some(Self::AnalyzeNutrition),
        _ => None,
    }
}
```

Add to `name()` (src/protocols/universal/tool_registry.rs:82-112):

```rust
pub const fn name(&self) -> &'static str {
    match self {
        // ... existing names ...
        Self::AnalyzeNutrition => "analyze_nutrition",
    }
}
```

Add to `description()` (src/protocols/universal/tool_registry.rs:114-160):

```rust
pub const fn description(&self) -> &'static str {
    match self {
        // ... existing descriptions ...
        Self::AnalyzeNutrition => "Analyze nutrition data from activities and provide dietary recommendations",
    }
}
```

### Step 3: Create Handler Function

Create `src/protocols/universal/handlers/nutrition.rs`:

```rust
// ABOUTME: Nutrition analysis tool handler for dietary recommendations
// ABOUTME: Analyzes calorie burn and provides nutrition guidance

use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use crate::protocols::ProtocolError;
use serde_json::Value;
use std::collections::HashMap;

/// Analyze nutrition data from activities
///
/// # Errors
/// Returns error if:
/// - activity_id parameter missing
/// - Activity not found
/// - User not authenticated
pub async fn analyze_nutrition(
    request: &UniversalRequest,
) -> Result<UniversalResponse, ProtocolError> {
    // Extract parameters
    let activity_id = request
        .arguments
        .get("activity_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ProtocolError::InvalidParameters {
            message: "activity_id parameter required".to_string(),
        })?;

    // Extract user context from request
    let user_id = request
        .user_id
        .ok_or_else(|| ProtocolError::Unauthorized {
            message: "Authentication required".to_string(),
        })?;

    // TODO: Fetch activity data from database
    // TODO: Calculate calories burned
    // TODO: Generate nutrition recommendations

    // Example response structure
    let result = serde_json::json!({
        "activity_id": activity_id,
        "calories_burned": 450,
        "macros_recommended": {
            "protein_grams": 30,
            "carbs_grams": 60,
            "fat_grams": 15
        },
        "recommendations": [
            "Consume protein within 30 minutes post-workout",
            "Hydrate with 500ml water",
            "Consider a banana for quick carbs"
        ],
        "meal_suggestions": [
            "Greek yogurt with berries",
            "Chicken breast with sweet potato",
            "Protein shake with banana"
        ]
    });

    Ok(UniversalResponse {
        success: true,
        result: Some(result),
        error: None,
        metadata: Some({
            let mut map = HashMap::new();
            map.insert(
                "tool_name".to_string(),
                Value::String("analyze_nutrition".to_string()),
            );
            map.insert(
                "analysis_version".to_string(),
                Value::String("1.0".to_string()),
            );
            map
        }),
    })
}
```

### Step 4: Register Handler

Add module declaration in `src/protocols/universal/handlers/mod.rs`:

```rust
pub mod strava_api;
pub mod goals;
pub mod configuration;
pub mod nutrition;  // Your new module
```

### Step 5: Add to Tool Dispatch

Edit `src/mcp/tool_handlers.rs`, add to `route_tool_call()` function:

```rust
use crate::protocols::universal::handlers::nutrition;

async fn route_tool_call(
    tool_name: &str,
    ctx: ToolRoutingContext<'_>,
    request: &UniversalRequest,
) -> Result<UniversalResponse, ProtocolError> {
    match tool_name {
        // ... existing tool routes ...

        "analyze_nutrition" => {
            nutrition::analyze_nutrition(request).await
        }

        _ => Err(ProtocolError::ToolNotFound {
            tool_name: tool_name.to_string(),
        }),
    }
}
```

### Step 6: Define Input Schema

Add JSON Schema definition for MCP protocol (in tool_registry.rs or separate schema file):

```rust
pub fn input_schema(&self) -> JsonValue {
    match self {
        Self::AnalyzeNutrition => json!({
            "type": "object",
            "properties": {
                "activity_id": {
                    "type": "string",
                    "description": "ID of the activity to analyze for nutrition",
                    "pattern": "^[0-9]+$"
                },
                "meal_timing": {
                    "type": "string",
                    "enum": ["pre-workout", "post-workout", "general"],
                    "description": "When the meal will be consumed",
                    "default": "post-workout"
                },
                "dietary_restrictions": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": ["vegetarian", "vegan", "gluten-free", "dairy-free", "keto"]
                    },
                    "description": "User dietary restrictions"
                }
            },
            "required": ["activity_id"]
        }),
        // ... other tools ...
    }
}
```

### Step 7: Write Tests

Create `tests/tools/test_nutrition.rs`:

```rust
use pierre_mcp_server::protocols::universal::handlers::nutrition::analyze_nutrition;
use pierre_mcp_server::protocols::universal::UniversalRequest;

#[tokio::test]
async fn test_analyze_nutrition_success() {
    let mut args = std::collections::HashMap::new();
    args.insert(
        "activity_id".to_string(),
        serde_json::json!("12345"),
    );

    let request = UniversalRequest {
        tool_name: "analyze_nutrition".to_string(),
        arguments: args,
        user_id: Some(uuid::Uuid::new_v4()),
        tenant_id: Some("test-tenant".to_string()),
        auth_token: None,
    };

    let response = analyze_nutrition(&request).await.unwrap();

    assert!(response.success);
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert!(result.get("calories_burned").is_some());
    assert!(result.get("recommendations").is_some());
}

#[tokio::test]
async fn test_analyze_nutrition_missing_activity_id() {
    let request = UniversalRequest {
        tool_name: "analyze_nutrition".to_string(),
        arguments: std::collections::HashMap::new(),
        user_id: Some(uuid::Uuid::new_v4()),
        tenant_id: Some("test-tenant".to_string()),
        auth_token: None,
    };

    let result = analyze_nutrition(&request).await;
    assert!(result.is_err());
}
```

Run tests:
```bash
cargo test test_nutrition
```

## Tool Handler Patterns

### Accessing User Context

```rust
pub async fn my_tool(request: &UniversalRequest) -> Result<UniversalResponse, ProtocolError> {
    // Get authenticated user ID
    let user_id = request.user_id.ok_or_else(|| {
        ProtocolError::Unauthorized {
            message: "Authentication required".to_string(),
        }
    })?;

    // Get tenant context
    let tenant_id = request.tenant_id.as_deref().unwrap_or("default");

    // User and tenant are now available for database queries
}
```

### Accessing Database

Tools have access to database via `ServerResources` passed through context:

```rust
use crate::mcp::resources::ServerResources;

pub async fn my_tool_with_db(
    request: &UniversalRequest,
    resources: &Arc<ServerResources>,
) -> Result<UniversalResponse, ProtocolError> {
    let user_id = request.user_id.ok_or_else(|| {
        ProtocolError::Unauthorized {
            message: "Authentication required".to_string(),
        }
    })?;

    // Access database
    let user = resources
        .data
        .database()
        .get_user(user_id)
        .await
        .map_err(|e| ProtocolError::InternalError {
            message: format!("Database error: {}", e),
        })?
        .ok_or_else(|| ProtocolError::NotFound {
            message: "User not found".to_string(),
        })?;

    // Use user data
    Ok(UniversalResponse {
        success: true,
        result: Some(serde_json::json!({
            "user_email": user.email,
        })),
        error: None,
        metadata: None,
    })
}
```

### Accessing OAuth Tokens

For tools that need OAuth provider access:

```rust
use crate::protocols::universal::handlers::strava_api::create_configured_strava_provider;

pub async fn my_strava_tool(
    request: &UniversalRequest,
    resources: &Arc<ServerResources>,
) -> Result<UniversalResponse, ProtocolError> {
    let user_id = request.user_id.ok_or_else(|| {
        ProtocolError::Unauthorized {
            message: "Authentication required".to_string(),
        }
    })?;

    let tenant_id = request.tenant_id.as_deref().unwrap_or("default");

    // Get OAuth token for Strava
    let token = resources
        .data
        .database()
        .get_user_oauth_token(user_id, tenant_id, "strava")
        .await
        .map_err(|e| ProtocolError::InternalError {
            message: format!("Failed to get OAuth token: {}", e),
        })?
        .ok_or_else(|| ProtocolError::ProviderNotConnected {
            provider: "strava".to_string(),
        })?;

    // Create configured provider
    let provider = create_configured_strava_provider(&token)
        .await
        .map_err(|e| ProtocolError::InternalError {
            message: format!("Failed to configure provider: {}", e),
        })?;

    // Use provider to fetch data
    let activities = provider
        .get_activities(10, None)
        .await
        .map_err(|e| ProtocolError::ProviderError {
            provider: "strava".to_string(),
            message: e.to_string(),
        })?;

    Ok(UniversalResponse {
        success: true,
        result: Some(serde_json::json!({ "activities": activities })),
        error: None,
        metadata: None,
    })
}
```

### Error Handling

Always use `ProtocolError` enum (src/protocols/mod.rs):

```rust
use crate::protocols::ProtocolError;

// Invalid parameters
return Err(ProtocolError::InvalidParameters {
    message: "activity_id must be a positive integer".to_string(),
});

// Authentication required
return Err(ProtocolError::Unauthorized {
    message: "JWT token required".to_string(),
});

// Provider not connected
return Err(ProtocolError::ProviderNotConnected {
    provider: "strava".to_string(),
});

// Tool execution failed
return Err(ProtocolError::ToolExecutionFailed {
    tool_name: "analyze_nutrition".to_string(),
    message: "Failed to calculate calories".to_string(),
});

// Internal error
return Err(ProtocolError::InternalError {
    message: format!("Database error: {}", e),
});
```

### Response Structure

Always return `UniversalResponse`:

```rust
use crate::protocols::universal::UniversalResponse;

Ok(UniversalResponse {
    success: true,  // false if tool executed but had business logic failure
    result: Some(serde_json::json!({
        // Your result data
        "data": "value",
    })),
    error: None,  // Set if success=false
    metadata: Some({
        let mut map = HashMap::new();
        map.insert("tool_name".to_string(), json!("my_tool"));
        map.insert("version".to_string(), json!("1.0"));
        map
    }),
})
```

## Parameter Validation

### Required Parameters

```rust
let activity_id = request
    .arguments
    .get("activity_id")
    .and_then(|v| v.as_str())
    .ok_or_else(|| ProtocolError::InvalidParameters {
        message: "activity_id parameter required".to_string(),
    })?;
```

### Optional Parameters with Defaults

```rust
let limit = request
    .arguments
    .get("limit")
    .and_then(|v| v.as_u64())
    .unwrap_or(10) as usize;
```

### Type Validation

```rust
let activity_id = request
    .arguments
    .get("activity_id")
    .and_then(|v| v.as_str())
    .and_then(|s| s.parse::<u64>().ok())
    .ok_or_else(|| ProtocolError::InvalidParameters {
        message: "activity_id must be a valid integer".to_string(),
    })?;
```

### Enum Validation

```rust
let provider = request
    .arguments
    .get("provider")
    .and_then(|v| v.as_str())
    .ok_or_else(|| ProtocolError::InvalidParameters {
        message: "provider parameter required".to_string(),
    })?;

if !["strava", "fitbit"].contains(&provider) {
    return Err(ProtocolError::InvalidParameters {
        message: format!("Invalid provider: {}. Must be 'strava' or 'fitbit'", provider),
    });
}
```

## Advanced Features

### Progress Notifications

For long-running tools, send progress updates via SSE:

```rust
use crate::mcp::schema::ProgressNotification;

pub async fn long_running_tool(
    request: &UniversalRequest,
    resources: &Arc<ServerResources>,
) -> Result<UniversalResponse, ProtocolError> {
    let total_steps = 100;

    for step in 0..total_steps {
        // Do work
        process_step(step).await?;

        // Send progress notification
        let progress = ProgressNotification {
            progress_token: format!("task_{}", request.request_id),
            progress: step as f64,
            total: total_steps as f64,
        };

        if let Some(sse_manager) = &resources.sse_manager {
            let _ = sse_manager
                .send_mcp_progress(&request.user_id.unwrap(), &progress)
                .await;
        }
    }

    Ok(UniversalResponse {
        success: true,
        result: Some(json!({"completed": true})),
        error: None,
        metadata: None,
    })
}
```

### Structured Content

Return structured data for rich UI rendering:

```rust
Ok(UniversalResponse {
    success: true,
    result: Some(json!({
        "structuredContent": {
            "type": "activity_analysis",
            "activity": {
                "id": "12345",
                "name": "Morning Run",
                "distance_km": 5.2,
                "duration_minutes": 28
            },
            "insights": [
                {
                    "type": "pace_improvement",
                    "title": "Pace Improved",
                    "description": "Your pace was 12% faster than average",
                    "confidence": 0.85
                }
            ],
            "charts": [
                {
                    "type": "heart_rate",
                    "data_points": [...],
                    "zones": [...]
                }
            ]
        }
    })),
    error: None,
    metadata: None,
})
```

### Caching Results

Cache expensive computations:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

pub struct AnalysisCache {
    cache: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl AnalysisCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_or_compute<F, Fut>(
        &self,
        key: &str,
        compute: F,
    ) -> Result<serde_json::Value, ProtocolError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<serde_json::Value, ProtocolError>>,
    {
        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(key) {
                return Ok(cached.clone());
            }
        }

        // Compute result
        let result = compute().await?;

        // Store in cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(key.to_string(), result.clone());
        }

        Ok(result)
    }
}
```

## Testing Tools

### Unit Tests

Test handler logic in isolation:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parameter_validation() {
        // Test missing required parameter
        let request = UniversalRequest {
            tool_name: "my_tool".to_string(),
            arguments: HashMap::new(),
            user_id: Some(Uuid::new_v4()),
            tenant_id: Some("test".to_string()),
            auth_token: None,
        };

        let result = my_tool(&request).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ProtocolError::InvalidParameters { .. }));
    }

    #[tokio::test]
    async fn test_success_case() {
        let mut args = HashMap::new();
        args.insert("activity_id".to_string(), json!("12345"));

        let request = UniversalRequest {
            tool_name: "my_tool".to_string(),
            arguments: args,
            user_id: Some(Uuid::new_v4()),
            tenant_id: Some("test".to_string()),
            auth_token: None,
        };

        let result = my_tool(&request).await.unwrap();
        assert!(result.success);
        assert!(result.result.is_some());
    }
}
```

### Integration Tests

Test end-to-end tool execution via MCP protocol:

```rust
#[tokio::test]
async fn test_tool_via_mcp() {
    // Set up test server
    let config = ServerConfig::from_env().unwrap();
    let database = create_test_database().await;
    let resources = create_test_resources(database).await;

    // Create MCP request
    let request = McpRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "my_tool",
            "arguments": {
                "activity_id": "12345"
            }
        })),
        id: Some(json!(1)),
        auth_token: Some(format!("Bearer {}", test_jwt_token)),
    };

    // Execute via MCP handler
    let response = ToolHandlers::handle_tools_call_with_resources(
        request,
        &resources,
    ).await;

    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.result.is_some());
}
```

## Best Practices

### Performance

1. **Minimize database queries** - Batch queries when possible
2. **Use connection pooling** - Database connections are pooled automatically
3. **Cache expensive computations** - Use in-memory cache for repeated calculations
4. **Async all the way** - Use async/await throughout, never block
5. **Stream large datasets** - Use pagination for large result sets

### Security

1. **Always validate user authentication** - Check `request.user_id`
2. **Validate all input parameters** - Never trust client input
3. **Check authorization** - Verify user has access to requested data
4. **Sanitize outputs** - Remove sensitive data from responses
5. **Rate limit expensive operations** - Prevent abuse

### Error Handling

1. **Use Result types** - Never panic in production code
2. **Provide helpful error messages** - Include context for debugging
3. **Log errors with tracing** - Use `tracing::error!()` for failures
4. **Categorize errors** - Use appropriate `ProtocolError` variants
5. **Handle partial failures gracefully** - Return partial results when possible

### Code Quality

1. **Follow Rust naming conventions** - `snake_case` for functions, `PascalCase` for types
2. **Add doc comments** - Document public functions with `///`
3. **Use type safety** - Leverage Rust's type system for correctness
4. **Keep functions small** - Aim for <50 lines per function
5. **Write tests** - Unit tests + integration tests

## Tool Registry Reference

Complete list of existing tools (src/protocols/universal/tool_registry.rs:12-45):

**Core API Tools**:
- `get_activities` - Fetch user activities
- `get_athlete` - Get athlete profile
- `get_stats` - Performance statistics
- `analyze_activity` - Activity analysis
- `get_activity_intelligence` - AI-powered insights
- `get_connection_status` - Provider connection status
- `disconnect_provider` - Disconnect OAuth provider

**Goal Tools**:
- `set_goal` - Create fitness goal
- `suggest_goals` - AI goal suggestions
- `analyze_goal_feasibility` - Goal feasibility analysis
- `track_progress` - Goal progress tracking

**Analysis Tools**:
- `calculate_metrics` - Custom metrics
- `analyze_performance_trends` - Trend analysis
- `compare_activities` - Activity comparison
- `detect_patterns` - Pattern detection
- `generate_recommendations` - Training recommendations
- `calculate_fitness_score` - Fitness scoring
- `predict_performance` - Performance prediction
- `analyze_training_load` - Training load analysis

**Configuration Tools**:
- `get_configuration_catalog` - Config catalog
- `get_configuration_profiles` - Config profiles
- `get_user_configuration` - User config
- `update_user_configuration` - Update config
- `calculate_personalized_zones` - Training zones
- `validate_configuration` - Config validation

## Related Documentation

- [MCP Protocol](04-mcp-protocol.md) - MCP JSON-RPC protocol
- [API Reference](14-api-reference.md) - REST API endpoints
- [Testing Strategy](16-testing-strategy.md) - Testing guidelines
- [Architecture](01-architecture.md) - System architecture
