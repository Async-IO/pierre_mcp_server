# chapter 12: MCP tool registry & type-safe routing

This final chapter of Part III explores how Pierre registers MCP tools, validates parameters with JSON Schema, and routes tool calls to handlers. You'll learn about the tool registry pattern, schema generation, and type-safe parameter validation.

## what you'll learn

- Tool registry pattern for MCP servers
- JSON Schema for parameter validation
- Tool schema generation from types
- Dynamic tool registration
- Parameter extraction and validation
- Tool routing to handler functions
- Input schema requirements
- Error handling for invalid parameters

## tool registry overview

Pierre registers all MCP tools at startup using a centralized registry:

**Source**: src/mcp/schema.rs:499-525
```rust
pub fn get_tools() -> Vec<ToolSchema> {
    create_fitness_tools()
}

/// Create all fitness provider tool schemas
fn create_fitness_tools() -> Vec<ToolSchema> {
    vec![
        // Connection tools
        create_connect_to_pierre_tool(),
        create_connect_provider_tool(),
        create_get_connection_status_tool(),
        create_disconnect_provider_tool(),
        // Original tools
        create_get_activities_tool(),
        create_get_athlete_tool(),
        create_get_stats_tool(),
        create_get_activity_intelligence_tool(),
        create_get_notifications_tool(),
        create_mark_notifications_read_tool(),
        create_announce_oauth_success_tool(),
        create_check_oauth_notifications_tool(),
        // Fitness configuration tools
        create_list_fitness_configs_tool(),
        create_get_fitness_config_tool(),
        create_set_fitness_config_tool(),
        create_delete_fitness_config_tool(),
        // Goal tracking tools
        create_set_goal_tool(),
        create_track_progress_tool(),
    ]
}
```

**Registry pattern**: Single `get_tools()` function returns all available tools. This ensures tools/list and tools/call use the same definitions.

## tool schema structure

Each tool has a name, description, and JSON Schema for parameters:

**Source**: src/mcp/schema.rs:54-64
```rust
/// MCP Tool Schema Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    /// Tool name identifier
    pub name: String,
    /// Human-readable tool description
    pub description: String,
    /// JSON Schema for tool input parameters
    #[serde(rename = "inputSchema")]
    pub input_schema: JsonSchema,
}
```

**Fields**:
- `name`: Unique identifier (e.g., "get_activities")
- `description`: Human-readable explanation for AI assistants
- `inputSchema`: JSON Schema defining required/optional parameters

## JSON Schema for validation

JSON Schema describes parameter structure:

**Source**: src/mcp/schema.rs:66-78
```rust
/// JSON Schema Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchema {
    /// Schema type (e.g., "object", "string")
    #[serde(rename = "type")]
    pub schema_type: String,
    /// Property definitions for object schemas
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, PropertySchema>>,
    /// List of required property names
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}
```

**Example tool schema** (conceptual):
```json
{
  "name": "get_activities",
  "description": "Fetch fitness activities from connected providers",
  "inputSchema": {
    "type": "object",
    "properties": {
      "provider": {
        "type": "string",
        "description": "Provider name (strava, garmin, etc.)"
      },
      "limit": {
        "type": "number",
        "description": "Maximum activities to return"
      }
    },
    "required": ["provider"]
  }
}
```

## parameter validation

MCP servers validate tool parameters against inputSchema before execution. Invalid parameters return error code -32602 (Invalid params).

**Validation rules**:
- Required parameters must be present
- Parameter types must match schema
- Unknown parameters may be ignored or rejected
- Nested objects validated recursively

## tool handler routing

Tool calls route to handler functions based on tool name. The full flow from Chapter 10 through 12:

```
tools/call request
      │
      ▼
Extract tool name and arguments
      │
      ▼
Look up tool in registry (Chapter 12)
      │
      ▼
Validate arguments against inputSchema (Chapter 12)
      │
      ▼
Route to handler function (Chapter 10)
      │
      ▼
Execute with authentication (Chapter 6)
      │
      ▼
Return ToolResponse
```

## key takeaways

1. **Centralized registry**: `get_tools()` returns all available tools for both tools/list and tools/call.

2. **JSON Schema validation**: inputSchema defines required/optional parameters with types.

3. **Type safety**: Rust types ensure schema correctness at compile time.

4. **Dynamic registration**: Adding new tools requires updating `create_fitness_tools()` array.

5. **Parameter extraction**: Tools parse `arguments` JSON using serde deserialization.

6. **Error codes**: Invalid parameters return -32602 per JSON-RPC spec.

7. **Tool discovery**: AI assistants call tools/list to learn available functionality.

8. **Schema-driven UX**: Good descriptions and schema help AI assistants use tools correctly.

---

**End of Part III: MCP Protocol**

You've completed the MCP protocol implementation section. You now understand:
- JSON-RPC 2.0 foundation (Chapter 9)
- MCP request flow and processing (Chapter 10)
- Transport layers (stdio, HTTP, SSE) (Chapter 11)
- Tool registry and JSON Schema validation (Chapter 12)

**Next Chapter**: [Chapter 13: SDK Bridge Architecture](./chapter-13-sdk-bridge-architecture.md) - Begin Part IV by learning how the TypeScript SDK communicates with the Rust MCP server via stdio transport.
