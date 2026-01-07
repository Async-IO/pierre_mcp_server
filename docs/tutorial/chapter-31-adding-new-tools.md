<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 31: Adding New MCP Tools - Complete Checklist

This appendix provides a comprehensive checklist for adding new MCP tools to Pierre. Following this checklist ensures tools are properly integrated across all layers and tested.

## Quick Reference Checklist

Use this checklist when adding new tools:

```
□ 1. Constants     - src/constants/tools/identifiers.rs
□ 2. Schema        - src/mcp/schema.rs (import + create_*_tool fn + register)
□ 3. ToolId Enum   - src/protocols/universal/tool_registry.rs (enum + from_name + name)
□ 4. Handler       - src/protocols/universal/handlers/*.rs
□ 5. Executor      - src/protocols/universal/executor.rs (import + register)
□ 6. Tests         - tests/mcp_tools_unit.rs (presence + schema validation)
□ 7. Tests         - tests/schema_completeness_test.rs (critical tools list)
□ 8. SDK Tests     - sdk/test/integration/tool-call-validation.test.js
□ 9. Docs          - docs/tools-reference.md
□ 10. Tutorial     - docs/tutorial/chapter-19-tools-guide.md (update counts)
□ 11. Clippy       - cargo clippy --all-targets (strict mode)
□ 12. Run Tests    - cargo test (targeted tests for new tools)
```

## Step-by-Step Guide

### Step 1: Add Tool Identifier Constant

**File**: `src/constants/tools/identifiers.rs`

Add a constant for your tool name:

```rust
/// Recipe management tools (Combat des Chefs)
pub const GET_RECIPE_CONSTRAINTS: &str = "get_recipe_constraints";
pub const LIST_RECIPES: &str = "list_recipes";
pub const GET_RECIPE: &str = "get_recipe";
// ... add your tool constant here
```

**Why**: Eliminates hardcoded strings, enables compile-time checking.

### Step 2: Create Tool Schema

**File**: `src/mcp/schema.rs`

#### 2a. Add import for your constant:

```rust
use crate::constants::tools::{
    // ... existing imports ...
    YOUR_NEW_TOOL,  // Add your constant
};
```

#### 2b. Create schema function:

```rust
/// Create the `your_new_tool` tool schema
fn create_your_new_tool_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    // Add required parameters
    properties.insert(
        "param_name".to_owned(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Description of parameter".into()),
        },
    );

    // Add optional parameters
    properties.insert(
        "limit".to_owned(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Maximum results (default: 10)".into()),
        },
    );

    ToolSchema {
        name: YOUR_NEW_TOOL.to_owned(),  // Use constant!
        description: "Clear description of what the tool does".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["param_name".to_owned()]),  // Required params
        },
    }
}
```

#### 2c. Register in `create_fitness_tools()`:

```rust
fn create_fitness_tools() -> Vec<ToolSchema> {
    vec![
        // ... existing tools ...
        create_your_new_tool_tool(),  // Add here
    ]
}
```

### Step 3: Add to ToolId Enum

**File**: `src/protocols/universal/tool_registry.rs`

#### 3a. Add import:

```rust
use crate::constants::tools::{
    // ... existing imports ...
    YOUR_NEW_TOOL,
};
```

#### 3b. Add enum variant:

```rust
pub enum ToolId {
    // ... existing variants ...
    /// Your tool description
    YourNewTool,
}
```

#### 3c. Add to `from_name()`:

```rust
pub fn from_name(name: &str) -> Option<Self> {
    match name {
        // ... existing matches ...
        YOUR_NEW_TOOL => Some(Self::YourNewTool),
        _ => None,
    }
}
```

#### 3d. Add to `name()`:

```rust
pub const fn name(&self) -> &'static str {
    match self {
        // ... existing matches ...
        Self::YourNewTool => YOUR_NEW_TOOL,
    }
}
```

#### 3e. Add to `description()`:

```rust
pub const fn description(&self) -> &'static str {
    match self {
        // ... existing matches ...
        Self::YourNewTool => "Your tool description",
    }
}
```

### Step 4: Create Handler Function

**File**: `src/protocols/universal/handlers/your_module.rs` (or existing file)

```rust
/// Handle `your_new_tool` - description of what it does
///
/// # Arguments
/// * `executor` - Universal executor with database and auth context
/// * `request` - MCP request containing tool parameters
///
/// # Returns
/// JSON response with tool results or error
pub async fn handle_your_new_tool(
    executor: Arc<UniversalExecutor>,
    request: UniversalRequest,
) -> UniversalResponse {
    // Extract parameters
    let params = match request.params.as_ref() {
        Some(p) => p,
        None => return error_response(-32602, "Missing parameters"),
    };

    // Parse required parameters
    let param_name = match params.get("param_name").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return error_response(-32602, "Missing required parameter: param_name"),
    };

    // Get user context
    let user_id = match executor.user_id() {
        Some(id) => id,
        None => return error_response(-32603, "Authentication required"),
    };

    // Execute business logic
    match do_something(user_id, param_name).await {
        Ok(result) => success_response(result),
        Err(e) => error_response(-32603, &e.to_string()),
    }
}
```

### Step 5: Register in Executor

**File**: `src/protocols/universal/executor.rs`

#### 5a. Add import:

```rust
use crate::protocols::universal::handlers::your_module::handle_your_new_tool;
```

#### 5b. Register handler:

```rust
impl UniversalExecutor {
    fn register_tools(&mut self) {
        // ... existing registrations ...

        self.register_handler(
            ToolId::YourNewTool,
            |executor, request| Box::pin(handle_your_new_tool(executor, request)),
        );
    }
}
```

### Step 6: Add Unit Tests

**File**: `tests/mcp_tools_unit.rs`

#### 6a. Add to presence test:

```rust
#[test]
fn test_mcp_tool_schemas() {
    let tools = get_tools();
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

    // ... existing assertions ...

    // Your new tools
    assert!(tool_names.contains(&"your_new_tool"));
}
```

#### 6b. Add schema validation test:

```rust
#[test]
fn test_your_new_tool_schema() {
    let tools = get_tools();

    let tool = tools
        .iter()
        .find(|t| t.name == "your_new_tool")
        .expect("your_new_tool tool should exist");

    assert!(tool.description.contains("expected keyword"));

    if let Some(required) = &tool.input_schema.required {
        assert!(required.contains(&"param_name".to_owned()));
    } else {
        panic!("your_new_tool should have required parameters");
    }
}
```

### Step 7: Add to Critical Tools List

**File**: `tests/schema_completeness_test.rs`

```rust
#[test]
fn test_critical_tools_are_present() {
    let critical_tools = vec![
        // ... existing tools ...
        "your_new_tool",
    ];
    // ...
}
```

### Step 8: Add SDK Tests

**File**: `sdk/test/integration/tool-call-validation.test.js`

```javascript
const toolCallTests = [
    // ... existing tests ...
    {
        name: 'your_new_tool',
        description: 'Your tool description',
        arguments: { param_name: 'test-value' },
        expectedError: null  // or /expected error pattern/
    },
];
```

### Step 9: Update Documentation

**File**: `docs/tools-reference.md`

Add tool to the appropriate category section.

**File**: `docs/tutorial/chapter-19-tools-guide.md`

Update tool counts in the overview section.

### Step 10: Run Validation

```bash
# Format code
cargo fmt

# Run clippy strict mode
cargo clippy --all-targets --quiet -- \
    -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery

# Run targeted tests
cargo test your_new_tool -- --nocapture
cargo test test_mcp_tool_schemas -- --nocapture
cargo test test_recipe_tool_schemas -- --nocapture  # if recipe tool

# Run SDK tests
cd sdk && npm test
```

## Common Mistakes to Avoid

### 1. Forgetting to use constants
```rust
// WRONG - hardcoded string
name: "your_new_tool".to_owned(),

// CORRECT - use constant
name: YOUR_NEW_TOOL.to_owned(),
```

### 2. Missing from ToolId enum
If you see "Unknown tool" errors, check that your tool is in:
- `ToolId` enum variant
- `from_name()` match arm
- `name()` match arm

### 3. Not registering handler in executor
Handler must be registered in `executor.rs` or tools will fail with internal errors.

### 4. Forgetting to update test counts
Update tool counts in:
- `tests/mcp_tools_unit.rs`
- `tests/configuration_mcp_integration_test.rs`
- `tests/mcp_multitenant_complete_test.rs`

### 5. Not adding clippy allow in test files
Test files need:
```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
```

## File Reference Summary

| File | Purpose |
|------|---------|
| `src/constants/tools/identifiers.rs` | Tool name constants |
| `src/mcp/schema.rs` | Tool schemas for MCP discovery |
| `src/protocols/universal/tool_registry.rs` | Type-safe ToolId enum |
| `src/protocols/universal/handlers/*.rs` | Handler implementations |
| `src/protocols/universal/executor.rs` | Handler registration |
| `tests/mcp_tools_unit.rs` | Schema validation tests |
| `tests/schema_completeness_test.rs` | Registry completeness tests |
| `sdk/test/integration/tool-call-validation.test.js` | SDK integration tests |
| `docs/tools-reference.md` | Tool documentation |
| `docs/tutorial/chapter-19-tools-guide.md` | Tool usage guide |

## Example: Complete Tool Addition

See the recipe tools implementation for a complete example:
- Constants: `src/constants/tools/identifiers.rs` (lines 77-90)
- Schemas: `src/mcp/schema.rs` (search for `create_list_recipes_tool`)
- ToolId: `src/protocols/universal/tool_registry.rs` (search for `ListRecipes`)
- Handlers: `src/protocols/universal/handlers/recipes.rs`
- Executor: `src/protocols/universal/executor.rs` (search for `handle_list_recipes`)
- Tests: `tests/mcp_tools_unit.rs` (search for `test_recipe_tool_schemas`)
