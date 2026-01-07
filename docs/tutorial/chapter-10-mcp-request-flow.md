<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 10: MCP Protocol Deep Dive - Request Flow

This chapter explores how the Pierre Fitness Platform processes Model Context Protocol (MCP) requests from start to finish. You'll learn about request validation, method routing, authentication extraction, tool dispatch, and response serialization.

## What You'll Learn

- MCP request lifecycle from socket to response
- Request validation (jsonrpc version, method field)
- Method routing with pattern matching
- Authentication token extraction (HTTP header vs params)
- Tenant context extraction for multi-tenancy
- Tool handler dispatch and execution
- Response serialization and error handling
- Output formatters (JSON and TOON for LLM efficiency)
- Notification handling (no response)
- Structured logging with tracing spans
- Performance measurement and monitoring

## MCP Request Lifecycle

Every MCP request flows through multiple processing layers:

```
┌────────────────────────────────────────────────────────────┐
│                    MCP Client Request                      │
│  {"jsonrpc": "2.0", "method": "tools/call", ...}          │
└────────────────────────┬───────────────────────────────────┘
                         │
                         ▼
          ┌──────────────────────────┐
          │  Transport Layer         │  ← HTTP/WebSocket/stdio/SSE
          │  (receives JSON bytes)   │
          └──────────────────────────┘
                         │
                         ▼
          ┌──────────────────────────┐
          │  JSON Deserialization    │  ← Parse to McpRequest
          │  serde_json::from_str    │
          └──────────────────────────┘
                         │
                         ▼
          ┌──────────────────────────┐
          │  McpRequestProcessor     │  ← Validate and route
          │  handle_request()        │
          └──────────────────────────┘
                         │
        ┌────────────────┴────────────────┐
        │                                 │
        ▼                                 ▼
  ┌─────────────┐              ┌──────────────────┐
  │ Notification│              │  Method Routing  │
  │ (no resp)   │              │  initialize      │
  └─────────────┘              │  ping            │
                               │  tools/list      │
                               │  tools/call ───┐ │
                               └────────────────┼─┘
                                                │
                                                ▼
                               ┌──────────────────────────┐
                               │  Auth Middleware         │
                               │  - Extract token         │
                               │  - Validate JWT          │
                               │  - Extract user_id       │
                               └──────────────────────────┘
                                                │
                                                ▼
                               ┌──────────────────────────┐
                               │  Tenant Isolation        │
                               │  - Extract tenant_id     │
                               │  - Build TenantContext   │
                               └──────────────────────────┘
                                                │
                                                ▼
                               ┌──────────────────────────┐
                               │  Tool Handler Dispatch   │
                               │  - Route to specific tool│
                               │  - Execute with context  │
                               └──────────────────────────┘
                                                │
                                                ▼
                               ┌──────────────────────────┐
                               │  Response Serialization  │
                               │  McpResponse → JSON      │
                               └──────────────────────────┘
                                                │
                                                ▼
                               ┌──────────────────────────┐
                               │  Transport Layer         │
                               │  Send JSON bytes         │
                               └──────────────────────────┘
```

**Source**: src/mcp/mcp_request_processor.rs:25-76
```rust
/// Processes MCP protocol requests with validation, routing, and execution
pub struct McpRequestProcessor {
    resources: Arc<ServerResources>,
}

impl McpRequestProcessor {
    /// Create a new MCP request processor
    #[must_use]
    pub const fn new(resources: Arc<ServerResources>) -> Self {
        Self { resources }
    }

    /// Handle an MCP request and return a response
    pub async fn handle_request(&self, request: McpRequest) -> Option<McpResponse> {
        let start_time = std::time::Instant::now();

        // Log request with optional truncation
        Self::log_request(&request);

        // Handle notifications (no response needed)
        if request.method.starts_with("notifications/") {
            Self::handle_notification(&request);
            Self::log_completion("notification", start_time);
            return None;
        }

        // Process request and generate response
        let response = match self.process_request(request.clone()).await {
            Ok(response) => response,
            Err(e) => {
                error!(
                    "Failed to process MCP request: {} | Request: method={}, jsonrpc={}, id={:?}",
                    e, request.method, request.jsonrpc, request.id
                );
                error!("Request params: {:?}", request.params);
                error!("Full error details: {:#}", e);
                McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_owned(),
                    id: request.id.clone(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_INTERNAL_ERROR,
                        message: format!("Internal server error: {e}"),
                        data: None,
                    }),
                }
            }
        };

        Self::log_completion("request", start_time);
        Some(response)
    }
}
```

**Flow**:
1. **Start timer**: Capture request start time for performance monitoring
2. **Log request**: Record method, ID, and params (truncated for security)
3. **Check notifications**: If method starts with "notifications/", handle without response
4. **Process request**: Validate, route, and execute
5. **Error handling**: Convert `Result<McpResponse>` to `McpResponse` with error
6. **Log completion**: Record duration in logs
7. **Return**: `Some(response)` for requests, `None` for notifications

**Rust Idiom**: `Option<McpResponse>` return type

The `handle_request` method returns `Option<McpResponse>` instead of always returning a response. This explicitly represents "notifications don't get responses" in the type system. The transport layer can then handle `None` by not sending anything.

## Request Validation

The platform validates all requests before processing:

**Source**: src/mcp/mcp_request_processor.rs:96-111
```rust
/// Validate MCP request format and required fields
fn validate_request(request: &McpRequest) -> Result<()> {
    if request.jsonrpc != JSONRPC_VERSION {
        return Err(AppError::invalid_input(format!(
            "Invalid JSON-RPC version: got '{}', expected '{}'",
            request.jsonrpc, JSONRPC_VERSION
        ))
        .into());
    }

    if request.method.is_empty() {
        return Err(AppError::invalid_input("Missing method").into());
    }

    Ok(())
}
```

**Validation rules**:
- `jsonrpc` must be exactly `"2.0"`
- `method` must not be empty string
- Other fields are optional (validated by method handlers)

**Security**: Validating `jsonrpc` version prevents processing malformed or legacy JSON-RPC 1.0 requests.

## Method Routing

The processor routes requests to handlers based on the `method` field:

**Source**: src/mcp/mcp_request_processor.rs:78-94
```rust
/// Process an MCP request and generate response
async fn process_request(&self, request: McpRequest) -> Result<McpResponse> {
    // Validate request format
    Self::validate_request(&request)?;

    // Route to appropriate handler based on method
    match request.method.as_str() {
        "initialize" => Ok(Self::handle_initialize(&request)),
        "ping" => Ok(Self::handle_ping(&request)),
        "tools/list" => Ok(Self::handle_tools_list(&request)),
        "tools/call" => self.handle_tools_call(&request).await,
        "authenticate" => Ok(Self::handle_authenticate(&request)),
        method if method.starts_with("resources/") => Ok(Self::handle_resources(&request)),
        method if method.starts_with("prompts/") => Ok(Self::handle_prompts(&request)),
        _ => Ok(Self::handle_unknown_method(&request)),
    }
}
```

**Routing patterns**:
- **Exact match**: `"initialize"`, `"ping"`, `"tools/list"`
- **Async methods**: `tools/call` returns `Future` (awaited)
- **Prefix match**: `method.starts_with("resources/")` for resource operations
- **Fallback**: `_` pattern returns "method not found" error

**Rust Idiom**: Guard clauses in match arms

The `method if method.starts_with("resources/")` pattern uses a guard clause to match all methods with a specific prefix. This is more flexible than enumerating every resource method.

## MCP Protocol Handlers

### Initialize Handler

The initialize method establishes the protocol connection:

**Source**: src/mcp/mcp_request_processor.rs:113-143
```rust
/// Handle MCP initialize request
fn handle_initialize(request: &McpRequest) -> McpResponse {
    debug!("Handling initialize request");

    let server_info = serde_json::json!({
        "protocolVersion": crate::constants::protocol::mcp_protocol_version(),
        "capabilities": {
            "tools": {
                "listChanged": true
            },
            "resources": {
                "subscribe": true,
                "listChanged": true
            },
            "prompts": {
                "listChanged": true
            }
        },
        "serverInfo": {
            "name": "pierre-mcp-server",
            "version": env!("CARGO_PKG_VERSION")
        }
    });

    McpResponse {
        jsonrpc: JSONRPC_VERSION.to_owned(),
        id: request.id.clone(),
        result: Some(server_info),
        error: None,
    }
}
```

**Response structure**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "protocolVersion": "2025-06-18",
    "capabilities": {
      "tools": {"listChanged": true},
      "resources": {"subscribe": true, "listChanged": true},
      "prompts": {"listChanged": true}
    },
    "serverInfo": {
      "name": "pierre-mcp-server",
      "version": "0.1.0"
    }
  },
  "id": 1
}
```

**Capabilities**:
- `tools.listChanged`: Server notifies when tool list changes
- `resources.subscribe`: Clients can subscribe to resource updates
- `resources.listChanged`: Server notifies when resource list changes
- `prompts.listChanged`: Server notifies when prompt list changes

**Rust Idiom**: `env!("CARGO_PKG_VERSION")`

The `env!()` macro reads Cargo.toml version at compile time. This ensures the server version in responses always matches the actual build version.

### Ping Handler

The ping method tests connectivity:

**Source**: src/mcp/mcp_request_processor.rs:145-155
```rust
/// Handle MCP ping request
fn handle_ping(request: &McpRequest) -> McpResponse {
    debug!("Handling ping request");

    McpResponse {
        jsonrpc: JSONRPC_VERSION.to_owned(),
        id: request.id.clone(),
        result: Some(serde_json::json!({})),
        error: None,
    }
}
```

**Usage**: Clients use `ping` to measure latency and verify the server is responsive.

### tools/list Handler

The tools/list method returns available tools:

**Source**: src/mcp/mcp_request_processor.rs:174-193
```rust
/// Handle tools/list request
///
/// Per MCP specification, tools/list does NOT require authentication.
/// All tools are returned regardless of authentication status.
/// Individual tool calls will check authentication and trigger OAuth if needed.
fn handle_tools_list(request: &McpRequest) -> McpResponse {
    debug!("Handling tools/list request");

    // Get all available tools from schema
    // MCP spec: tools/list must work without authentication
    // Authentication is checked at tools/call time, not discovery time
    let tools = crate::mcp::schema::get_tools();

    McpResponse {
        jsonrpc: JSONRPC_VERSION.to_owned(),
        id: request.id.clone(),
        result: Some(serde_json::json!({ "tools": tools })),
        error: None,
    }
}
```

**Note**: Per MCP spec, `tools/list` does not require authentication. This allows AI assistants to discover available tools before users authenticate. Authentication is enforced at `tools/call` time.

### tools/call Handler

The tools/call method executes a specific tool:

**Source**: src/mcp/mcp_request_processor.rs:195-217
```rust
/// Handle tools/call request
async fn handle_tools_call(&self, request: &McpRequest) -> Result<McpResponse> {
    debug!("Handling tools/call request");

    request
        .params
        .as_ref()
        .ok_or_else(|| AppError::invalid_input("Missing parameters for tools/call"))?;

    // Execute tool using static method - delegate to ToolHandlers
    let handler_request = McpRequest {
        jsonrpc: request.jsonrpc.clone(),
        method: request.method.clone(),
        params: request.params.clone(),
        id: request.id.clone(),
        auth_token: request.auth_token.clone(),
        headers: request.headers.clone(),
        metadata: HashMap::new(),
    };
    let response =
        ToolHandlers::handle_tools_call_with_resources(handler_request, &self.resources).await;
    Ok(response)
}
```

**Delegation**: The `tools/call` handler delegates to `ToolHandlers::handle_tools_call_with_resources` which performs authentication and tool dispatch.

## Authentication Extraction

The tool handler extracts authentication tokens from multiple sources:

**Source**: src/mcp/tool_handlers.rs:63-101
```rust
#[tracing::instrument(
    skip(request, resources),
    fields(
        method = %request.method,
        request_id = ?request.id,
        tool_name = tracing::field::Empty,
        user_id = tracing::field::Empty,
        tenant_id = tracing::field::Empty,
        success = tracing::field::Empty,
        duration_ms = tracing::field::Empty,
    )
)]
pub async fn handle_tools_call_with_resources(
    request: McpRequest,
    resources: &Arc<ServerResources>,
) -> McpResponse {
    // Extract auth token from either HTTP Authorization header or MCP params
    let auth_token_string = request
        .params
        .as_ref()
        .and_then(|params| params.get("token"))
        .and_then(|token| token.as_str())
        .map(|mcp_token| format!("Bearer {mcp_token}"));

    let auth_token = request
        .auth_token
        .as_deref()
        .or(auth_token_string.as_deref());

    debug!(
        "MCP tool call authentication attempt for method: {} (token source: {})",
        request.method,
        if request.auth_token.is_some() {
            "HTTP header"
        } else {
            "MCP params"
        }
    );
```

**Token sources** (in priority order):
1. **HTTP header**: `request.auth_token` (from `Authorization: Bearer <token>`)
2. **MCP params**: `request.params.token` (passed in JSON-RPC params)

**Design pattern**: Checking multiple sources allows flexibility:
- WebSocket/HTTP clients use HTTP Authorization header
- stdio clients pass token in MCP params (no HTTP headers available)

**Rust Idiom**: `or()` for fallback

The expression `request.auth_token.as_deref().or(auth_token_string.as_deref())` tries the first source, then falls back to the second if `None`. This is more concise than `if let` chains.

## Authentication and Tenant Extraction

After extracting the token, the handler authenticates and extracts tenant context:

**Source**: src/mcp/tool_handlers.rs:103-160
```rust
match resources
    .auth_middleware
    .authenticate_request(auth_token)
    .await
{
    Ok(auth_result) => {
        // Record authentication success in span
        tracing::Span::current()
            .record("user_id", auth_result.user_id.to_string())
            .record("tenant_id", auth_result.user_id.to_string()); // Use user_id as tenant_id for now

        info!(
            "MCP tool call authentication successful for user: {} (method: {})",
            auth_result.user_id,
            auth_result.auth_method.display_name()
        );

        // Update user's last active timestamp
        if let Err(e) = resources
            .database
            .update_last_active(auth_result.user_id)
            .await
        {
            tracing::warn!(
                user_id = %auth_result.user_id,
                error = %e,
                "Failed to update user last active timestamp (activity tracking impacted)"
            );
        }

        // Extract tenant context from request and auth result
        let tenant_context = crate::mcp::tenant_isolation::extract_tenant_context_internal(
            &resources.database,
            Some(auth_result.user_id),
            None,
            None, // MCP transport headers not applicable here
        )
        .await
        .inspect_err(|e| {
            tracing::warn!(
                user_id = %auth_result.user_id,
                error = %e,
                "Failed to extract tenant context - tool will execute without tenant isolation"
            );
        })
        .ok()
        .flatten();

        // Use the provided ServerResources directly
        Self::handle_tool_execution_direct(request, auth_result, tenant_context, resources)
            .await
    }
    Err(e) => {
        tracing::Span::current().record("success", false);
        Self::handle_authentication_error(request, &e)
    }
}
```

**Flow**:
1. **Authenticate**: Validate JWT token with `auth_middleware.authenticate_request`
2. **Record span**: Add `user_id` and `tenant_id` to tracing span
3. **Update last active**: Record user activity timestamp
4. **Extract tenant**: Look up tenant context for multi-tenancy
5. **Execute tool**: Dispatch to specific tool handler
6. **Handle errors**: Return authentication error response

**Rust Idiom**: `inspect_err` for side effects

The `inspect_err(|e| { tracing::warn!(...) })` method logs errors without affecting the `Result` chain. This is cleaner than:
```rust
match tenant_context {
    Err(e) => {
        tracing::warn!(...);
        Err(e)
    }
    ok => ok
}
```

## Tool Handler Dispatch

The tool execution handler routes to specific tool implementations:

**Source**: src/mcp/tool_handlers.rs:173-200
```rust
async fn handle_tool_execution_direct(
    request: McpRequest,
    auth_result: AuthResult,
    tenant_context: Option<TenantContext>,
    resources: &Arc<ServerResources>,
) -> McpResponse {
    let Some(params) = request.params else {
        error!("Missing request parameters in tools/call");
        return McpResponse {
            jsonrpc: "2.0".to_owned(),
            id: request.id,
            result: None,
            error: Some(McpError {
                code: ERROR_INVALID_PARAMS,
                message: "Invalid params: Missing request parameters".to_owned(),
                data: None,
            }),
        };
    };
    let tool_name = params["name"].as_str().unwrap_or("");
    let args = &params["arguments"];
    let user_id = auth_result.user_id;

    // Record tool name in span
    tracing::Span::current().record("tool_name", tool_name);

    let start_time = std::time::Instant::now();
```

**Parameter extraction**:
- `params["name"]`: Tool name (e.g., "get_activities")
- `params["arguments"]`: Tool arguments as JSON
- `auth_result.user_id`: Authenticated user

The handler then dispatches to tool-specific functions based on `tool_name`.

## Notification Handling

Notifications are requests without responses:

**Source**: Inferred from src/mcp/mcp_request_processor.rs:44-49
```rust
// Handle notifications (no response needed)
if request.method.starts_with("notifications/") {
    Self::handle_notification(&request);
    Self::log_completion("notification", start_time);
    return None;
}
```

**MCP notification methods**:
- `notifications/progress`: Progress updates for long-running operations
- `notifications/cancelled`: Cancellation signals
- `notifications/message`: Log messages

**Return value**: `None` indicates no response should be sent. The transport layer handles this by not writing to the connection.

## Structured Logging

The platform uses tracing spans for structured logging:

**Source**: src/mcp/tool_handlers.rs:64-75
```rust
#[tracing::instrument(
    skip(request, resources),
    fields(
        method = %request.method,
        request_id = ?request.id,
        tool_name = tracing::field::Empty,
        user_id = tracing::field::Empty,
        tenant_id = tracing::field::Empty,
        success = tracing::field::Empty,
        duration_ms = tracing::field::Empty,
    )
)]
```

**Span fields**:
- `method`: MCP method name (always present)
- `request_id`: Request correlation ID (always present)
- `tool_name`: Filled in after extracting from params
- `user_id`: Filled in after authentication
- `tenant_id`: Filled in after tenant extraction
- `success`: Filled in after tool execution
- `duration_ms`: Filled in before returning

**Recording values**:
```rust
tracing::Span::current().record("tool_name", tool_name);
tracing::Span::current().record("user_id", auth_result.user_id.to_string());
tracing::Span::current().record("success", true);
```

## Error Handling Patterns

The platform converts `Result<McpResponse>` to `McpResponse` with errors:

**Source**: src/mcp/mcp_request_processor.rs:52-72
```rust
let response = match self.process_request(request.clone()).await {
    Ok(response) => response,
    Err(e) => {
        error!(
            "Failed to process MCP request: {} | Request: method={}, jsonrpc={}, id={:?}",
            e, request.method, request.jsonrpc, request.id
        );
        error!("Request params: {:?}", request.params);
        error!("Full error details: {:#}", e);
        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            id: request.id.clone(),
            result: None,
            error: Some(McpError {
                code: ERROR_INTERNAL_ERROR,
                message: format!("Internal server error: {e}"),
                data: None,
            }),
        }
    }
};
```

**Error logging**:
- Error message with context
- Request details (method, jsonrpc, id)
- Request params (may contain sensitive data, use `debug` level)
- Full error chain with `{:#}` formatter

**Response structure**: All errors return `ERROR_INTERNAL_ERROR` (-32603) code. More specific codes (METHOD_NOT_FOUND, INVALID_PARAMS) are returned by individual handlers.

## Output Formatters (TOON Support)

Pierre supports multiple output formats for tool responses, optimized for different consumers.

**Source**: src/formatters/mod.rs

```rust
/// Output serialization format selector
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// JSON format (default) - universal compatibility
    #[default]
    Json,
    /// TOON format - Token-Oriented Object Notation for LLM efficiency
    /// Achieves ~40% token reduction compared to JSON
    Toon,
}

impl OutputFormat {
    /// Parse format from string parameter (case-insensitive)
    #[must_use]
    pub fn from_str_param(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "toon" => Self::Toon,
            _ => Self::Json,
        }
    }

    /// Get the MIME content type for this format
    #[must_use]
    pub const fn content_type(&self) -> &'static str {
        match self {
            Self::Json => "application/json",
            Self::Toon => "application/vnd.toon",
        }
    }
}
```

**Why TOON?**

When LLMs process large datasets (e.g., a year of fitness activities), token count directly impacts:
- API costs (tokens × price per token)
- Context window usage (limited tokens available)
- Response latency (more tokens = slower processing)

TOON achieves ~40% token reduction by:
- Eliminating redundant JSON syntax (quotes, colons, commas)
- Using whitespace-based structure
- Preserving semantic meaning for LLM comprehension

**Usage in tools**:

```rust
use crate::formatters::{format_output, OutputFormat};

// Tool receives format preference from client
let format = params.output_format
    .map(|s| OutputFormat::from_str_param(&s))
    .unwrap_or_default();

// Serialize response in requested format
let output = format_output(&activities, format)?;
// output.data contains the serialized string
// output.content_type contains the MIME type
```

**Format comparison**:

```json
// JSON (default): 847 tokens for 100 activities
{"activities":[{"id":"act_001","type":"Run","distance":5000,...},...]}

// TOON (~40% fewer tokens): 508 tokens for same data
activities
  act_001
    type Run
    distance 5000
    ...
```

## Key Takeaways

1. **Request lifecycle**: MCP requests flow through transport → deserialization → validation → routing → authentication → tenant extraction → tool execution → serialization → transport.

2. **Validation first**: All requests are validated for `jsonrpc` version and `method` field before routing.

3. **Method routing**: Pattern matching on `method` string with exact match, async methods, prefix match, and fallback.

4. **Authentication sources**: Tokens extracted from HTTP Authorization header (WebSocket/HTTP) or MCP params (stdio).

5. **Notification handling**: Requests with `method.starts_with("notifications/")` return `None` (no response sent).

6. **Structured logging**: `#[tracing::instrument]` with empty fields filled during processing provides comprehensive observability.

7. **Tenant extraction**: After authentication, platform looks up user's tenant for multi-tenant isolation.

8. **Error conversion**: `Result<McpResponse>` converted to `McpResponse` with error field for all failures.

9. **Tool dispatch**: `tools/call` delegates to `ToolHandlers` which routes to specific tool implementations.

10. **Performance monitoring**: Request duration measured from start to completion, recorded in logs.

---

**Next Chapter**: [Chapter 11: MCP Transport Layers](./chapter-11-mcp-transport-layers.md) - Learn how the Pierre platform supports multiple transport mechanisms (HTTP, stdio, WebSocket, SSE) for MCP communication.
