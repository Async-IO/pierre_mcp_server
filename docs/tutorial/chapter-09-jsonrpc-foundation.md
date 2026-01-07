<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 09: JSON-RPC 2.0 Foundation

This chapter explores the JSON-RPC 2.0 protocol that forms the foundation of the Model Context Protocol (MCP) in the Pierre Fitness Platform. You'll learn about request/response structures, error codes, notifications, and how Pierre extends JSON-RPC for authentication and multi-tenancy.

## What You'll Learn

- JSON-RPC 2.0 specification and structure
- Request, response, and error objects
- Standard error codes (-32700 to -32600)
- Request correlation with ID fields
- Notifications (fire-and-forget messages)
- Pierre's JSON-RPC extensions (auth, headers, metadata)
- Custom Debug implementation for security
- Protocol versioning with jsonrpc field
- Unified JSON-RPC for MCP and A2A protocols
- Error handling patterns and best practices

## JSON-RPC 2.0 Overview

JSON-RPC is a lightweight remote procedure call (RPC) protocol encoded in JSON. It's stateless, transport-agnostic, and simple to implement.

**Key characteristics**:
- **Stateless**: Each request is independent (no session state)
- **Transport-agnostic**: Works over HTTP, WebSocket, stdin/stdout, SSE
- **Bidirectional**: Both client and server can initiate requests
- **Simple**: Only 4 message types (request, response, error, notification)

### Protocol Structure

```
┌──────────────────────────────────────────────────────────┐
│                     JSON-RPC 2.0                         │
│                                                          │
│  Client                            Server                │
│    │                                  │                  │
│    │  ─────── Request ──────►         │                  │
│    │  {                               │                  │
│    │    "jsonrpc": "2.0",             │                  │
│    │    "method": "tools/call",       │                  │
│    │    "params": {...},              │                  │
│    │    "id": 1                       │                  │
│    │  }                               │                  │
│    │                                  │                  │
│    │  ◄──── Response ────────         │                  │
│    │  {                               │                  │
│    │    "jsonrpc": "2.0",             │                  │
│    │    "result": {...},              │                  │
│    │    "id": 1                       │                  │
│    │  }                               │                  │
│    │                                  │                  │
│    │  ─────── Notification ───►       │                  │
│    │  {                               │                  │
│    │    "jsonrpc": "2.0",             │                  │
│    │    "method": "progress",         │                  │
│    │    "params": {...}               │                  │
│    │    (no id field)                 │                  │
│    │  }                               │                  │
└──────────────────────────────────────────────────────────┘
```

**Source**: src/jsonrpc/mod.rs:1-44
```rust
// ABOUTME: Unified JSON-RPC 2.0 implementation for all protocols (MCP, A2A)
// ABOUTME: Provides shared request, response, and error types eliminating duplication

//! # JSON-RPC 2.0 Foundation
//!
//! This module provides a unified implementation of JSON-RPC 2.0 used by
//! all protocols in Pierre (MCP, A2A). This eliminates duplication and
//! ensures consistent behavior across protocols.
//!
//! ## Design Goals
//!
//! 1. **Single Source of Truth**: One JSON-RPC implementation
//! 2. **Protocol Agnostic**: Works for MCP, A2A, and future protocols
//! 3. **Type Safe**: Strong typing with serde support
//! 4. **Extensible**: Metadata field for protocol-specific extensions

/// JSON-RPC 2.0 version string
pub const JSONRPC_VERSION: &str = "2.0";
```

**Note**: Pierre uses a unified JSON-RPC implementation shared by MCP and A2A protocols. This ensures consistent behavior across all protocol handlers.

## Request Structure

A JSON-RPC request represents a method call from client to server or server to client:

**Source**: src/jsonrpc/mod.rs:46-103
```rust
/// JSON-RPC 2.0 Request
///
/// This is the unified request structure used by all protocols.
/// Protocol-specific extensions (like MCP/A2A's `auth_token`) are included as optional fields.
#[derive(Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,

    /// Method name to invoke
    pub method: String,

    /// Optional parameters for the method
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,

    /// Request identifier (for correlation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,

    /// Authorization header value (Bearer token) - MCP/A2A extension
    #[serde(rename = "auth", skip_serializing_if = "Option::is_none", default)]
    pub auth_token: Option<String>,

    /// Optional HTTP headers for tenant context and other metadata - MCP extension
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub headers: Option<HashMap<String, Value>>,

    /// Protocol-specific metadata (additional extensions)
    /// Not part of JSON-RPC spec, but useful for future extensions
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub metadata: HashMap<String, String>,
}

// Custom Debug implementation that redacts sensitive auth tokens
impl fmt::Debug for JsonRpcRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JsonRpcRequest")
            .field("jsonrpc", &self.jsonrpc)
            .field("method", &self.method)
            .field("params", &self.params)
            .field("id", &self.id)
            .field(
                "auth_token",
                &self.auth_token.as_ref().map(|token| {
                    // Redact token: show first 10 and last 8 characters, or "[REDACTED]" if short
                    if token.len() > 20 {
                        format!("{}...{}", &token[..10], &token[token.len() - 8..])
                    } else {
                        "[REDACTED]".to_owned()
                    }
                }),
            )
            .field("headers", &self.headers)
            .field("metadata", &self.metadata)
            .finish()
    }
}
```

**Standard fields** (JSON-RPC 2.0 spec):
- `jsonrpc`: Protocol version ("2.0")
- `method`: Method name to invoke (e.g., "initialize", "tools/call")
- `params`: Optional parameters (JSON value)
- `id`: Request identifier for response correlation

**Pierre extensions** (not in JSON-RPC spec):
- `auth_token`: JWT token for authentication (renamed to "auth" in JSON)
- `headers`: HTTP headers for tenant context (x-tenant-id, etc.)
- `metadata`: Key-value pairs for protocol-specific extensions

**Rust Idiom**: `#[serde(skip_serializing_if = "Option::is_none")]`

This attribute omits `None` values from JSON serialization. A request with no parameters serializes as `{"jsonrpc": "2.0", "method": "ping"}` instead of `{"jsonrpc": "2.0", "method": "ping", "params": null}`. This reduces message size and improves readability.

### Custom Debug Implementation

The `JsonRpcRequest` provides a custom `Debug` impl that redacts auth tokens:

```rust
// Security: Custom Debug redacts sensitive tokens
impl fmt::Debug for JsonRpcRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JsonRpcRequest")
            .field("auth_token", &self.auth_token.as_ref().map(|token| {
                if token.len() > 20 {
                    format!("{}...{}", &token[..10], &token[token.len() - 8..])
                } else {
                    "[REDACTED]".to_owned()
                }
            }))
            // ... other fields
            .finish()
    }
}
```

**Security**: This prevents JWT tokens from appearing in debug logs. If a developer calls `dbg!(request)` or logs `{:?}`, the token shows as `"eyJhbGc...V6T6QMBv"` instead of the full token.

**Rust Idiom**: Manual Debug implementation

Deriving `Debug` would print the full auth token. By implementing `Debug` manually, we control exactly what gets logged. This is a common pattern for types containing secrets.

### Request Constructors

The platform provides builder methods for creating requests:

**Source**: src/jsonrpc/mod.rs:142-197
```rust
impl JsonRpcRequest {
    /// Create a new JSON-RPC request
    #[must_use]
    pub fn new(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            method: method.into(),
            params,
            id: Some(Value::Number(1.into())),
            auth_token: None,
            headers: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a new request with a specific ID
    #[must_use]
    pub fn with_id(method: impl Into<String>, params: Option<Value>, id: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            method: method.into(),
            params,
            id: Some(id),
            auth_token: None,
            headers: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a notification (no ID, no response expected)
    #[must_use]
    pub fn notification(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            method: method.into(),
            params,
            id: None,
            auth_token: None,
            headers: None,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the request
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get metadata value
    #[must_use]
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}
```

**Usage patterns**:
```rust
// Standard request with auto-generated ID
let request = JsonRpcRequest::new("initialize", Some(params));

// Request with specific ID for correlation
let request = JsonRpcRequest::with_id("tools/call", Some(params), Value::String("req-123".into()));

// Notification (fire-and-forget, no response expected)
let notification = JsonRpcRequest::notification("progress", Some(progress_data));

// Request with metadata
let request = JsonRpcRequest::new("initialize", Some(params))
    .with_metadata("tenant_id", tenant_id.to_string())
    .with_metadata("request_source", "web_ui");
```

**Rust Idiom**: Builder pattern with `#[must_use]`

The `with_metadata` method consumes `self` and returns the modified `Self`, enabling method chaining. The `#[must_use]` attribute warns if the returned value is ignored (preventing bugs where you call `request.with_metadata(...)` without assigning the result).

## Response Structure

A JSON-RPC response represents the result of a method call:

**Source**: src/jsonrpc/mod.rs:105-257
```rust
/// JSON-RPC 2.0 Response
///
/// Represents a successful response or an error.
/// Exactly one of `result` or `error` must be present.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,

    /// Result of the method call (mutually exclusive with error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,

    /// Error information (mutually exclusive with result)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,

    /// Request identifier for correlation
    pub id: Option<Value>,
}

impl JsonRpcResponse {
    /// Create a success response
    #[must_use]
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Create an error response
    #[must_use]
    pub fn error(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
            id,
        }
    }

    /// Create an error response with additional data
    #[must_use]
    pub fn error_with_data(
        id: Option<Value>,
        code: i32,
        message: impl Into<String>,
        data: Value,
    ) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: Some(data),
            }),
            id,
        }
    }

    /// Check if this is a success response
    #[must_use]
    pub const fn is_success(&self) -> bool {
        self.error.is_none() && self.result.is_some()
    }

    /// Check if this is an error response
    #[must_use]
    pub const fn is_error(&self) -> bool {
        self.error.is_some()
    }
}
```

**Invariant**: Exactly one of `result` or `error` must be `Some`. The JSON-RPC spec forbids responses with both fields set or both fields absent.

**Success response example**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "tools": [...]
  },
  "id": 1
}
```

**Error response example**:
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32601,
    "message": "Method not found"
  },
  "id": 1
}
```

## Error Structure

JSON-RPC errors contain a numeric code, human-readable message, and optional data:

**Source**: src/jsonrpc/mod.rs:126-140
```rust
/// JSON-RPC 2.0 Error Object
///
/// Standard error structure with code, message, and optional data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code (standard codes: -32700 to -32600)
    pub code: i32,

    /// Human-readable error message
    pub message: String,

    /// Additional error information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}
```

**Source**: src/jsonrpc/mod.rs:259-279
```rust
impl JsonRpcError {
    /// Create a new error
    #[must_use]
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Create an error with data
    #[must_use]
    pub fn with_data(code: i32, message: impl Into<String>, data: Value) -> Self {
        Self {
            code,
            message: message.into(),
            data: Some(data),
        }
    }
}
```

**Fields**:
- `code`: Integer error code (negative values reserved by spec)
- `message`: Human-readable description
- `data`: Optional structured error details (stack trace, validation errors, etc.)

### Standard Error Codes

JSON-RPC 2.0 defines standard error codes in the -32700 to -32600 range:

**Source**: src/jsonrpc/mod.rs:281-303
```rust
/// Standard JSON-RPC error codes
pub mod error_codes {
    /// Parse error - Invalid JSON
    pub const PARSE_ERROR: i32 = -32700;

    /// Invalid Request - Invalid JSON-RPC
    pub const INVALID_REQUEST: i32 = -32600;

    /// Method not found
    pub const METHOD_NOT_FOUND: i32 = -32601;

    /// Invalid params
    pub const INVALID_PARAMS: i32 = -32602;

    /// Internal error
    pub const INTERNAL_ERROR: i32 = -32603;

    /// Server error range start
    pub const SERVER_ERROR_START: i32 = -32000;

    /// Server error range end
    pub const SERVER_ERROR_END: i32 = -32099;
}
```

**Error code ranges**:
- `-32700`: Parse error (malformed JSON)
- `-32600`: Invalid request (valid JSON, invalid JSON-RPC)
- `-32601`: Method not found (unknown method name)
- `-32602`: Invalid params (method exists, params are wrong)
- `-32603`: Internal error (server-side failure)
- `-32000` to `-32099`: Server-specific errors (application-defined)

**Usage example**:
```rust
use pierre_mcp_server::jsonrpc::{JsonRpcResponse, error_codes};

// Method not found
let response = JsonRpcResponse::error(
    Some(request_id),
    error_codes::METHOD_NOT_FOUND,
    "Method 'unknown_method' not found"
);

// Invalid params with error details
let response = JsonRpcResponse::error_with_data(
    Some(request_id),
    error_codes::INVALID_PARAMS,
    "Missing required parameter 'provider'",
    serde_json::json!({
        "required_params": ["provider"],
        "received_params": ["limit", "offset"]
    })
);
```

## Request Correlation with Ids

The `id` field correlates requests with responses in bidirectional communication:

```
Client ────────────────────────► Server
  Request: {
    "id": 1,
    "method": "initialize"
  }

Client ◄──────────────────────── Server
  Response: {
    "id": 1,
    "result": {...}
  }

Client ────────────────────────► Server
  Request: {
    "id": 2,
    "method": "tools/list"
  }

Client ◄──────────────────────── Server
  Response: {
    "id": 2,
    "result": {...}
  }
```

**Correlation rules**:
1. Response `id` must match request `id` exactly
2. `id` can be string, number, or null (but not missing)
3. Notifications have no `id` (no response expected)
4. Server can process requests out-of-order (async)

**Rust Idiom**: `Option<Value>` for flexible ID types

Using `serde_json::Value` allows IDs to be:
```rust
Some(Value::Number(1.into()))        // Numeric ID
Some(Value::String("req-123".into())) // String ID
Some(Value::Null)                     // Null ID (valid per spec)
None                                   // Notification (no response)
```

## Notifications (Fire-and-forget)

Notifications are requests without an `id` field. The server does not send a response:

**Use cases**:
- Progress updates (`notifications/progress`)
- Cancellation signals (`notifications/cancelled`)
- Log messages (`logging/logMessage`)
- Events that don't require acknowledgment

**Example**:
```json
{
  "jsonrpc": "2.0",
  "method": "notifications/progress",
  "params": {
    "progressToken": "tok-123",
    "progress": 50,
    "total": 100
  }
}
```

**Creating notifications**:
```rust
let notification = JsonRpcRequest::notification(
    "notifications/progress",
    Some(serde_json::json!({
        "progressToken": token,
        "progress": current,
        "total": total
    }))
);
```

**Rust Idiom**: Pattern matching on `id`

Handlers distinguish notifications from requests:
```rust
match request.id {
    None => {
        // Notification - process without sending response
        handle_notification(request);
    }
    Some(id) => {
        // Request - send response with matching ID
        let result = handle_request(request);
        JsonRpcResponse::success(Some(id), result)
    }
}
```

## MCP Extensions to JSON-RPC

Pierre extends JSON-RPC with additional fields for authentication and multi-tenancy:

### Auth_token Field

The `auth_token` field carries JWT authentication:

```rust
#[serde(rename = "auth", skip_serializing_if = "Option::is_none", default)]
pub auth_token: Option<String>,
```

**JSON representation** (note rename to "auth"):
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {...},
  "id": 1,
  "auth": "eyJhbGciOiJSUzI1NiIs..."
}
```

**Note**: The `auth_token` field (Rust name) serializes as `"auth"` (JSON name) via `#[serde(rename = "auth")]`. This keeps JSON messages concise while maintaining clear Rust naming.

### Headers Field

The `headers` field carries HTTP-like metadata:

```rust
#[serde(skip_serializing_if = "Option::is_none", default)]
pub headers: Option<HashMap<String, Value>>,
```

**JSON representation**:
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {...},
  "id": 1,
  "headers": {
    "x-tenant-id": "550e8400-e29b-41d4-a716-446655440000",
    "x-tenant-name": "Acme Corp"
  }
}
```

**Use cases**:
- Tenant identification (`x-tenant-id`, `x-tenant-name`)
- Request tracing (`x-request-id`)
- Feature flags (`x-enable-experimental`)

### Metadata Field

The `metadata` field provides protocol-specific extensions:

```rust
#[serde(skip_serializing_if = "HashMap::is_empty", default)]
pub metadata: HashMap<String, String>,
```

**JSON representation**:
```json
{
  "jsonrpc": "2.0",
  "method": "initialize",
  "params": {...},
  "id": 1,
  "metadata": {
    "client_version": "1.2.3",
    "platform": "macos",
    "locale": "en-US"
  }
}
```

**Rust Idiom**: `#[serde(skip_serializing_if = "HashMap::is_empty")]`

Empty hashmaps are omitted from JSON. A request with no metadata serializes without the `"metadata"` key, reducing message size.

## MCP Protocol Implementation

The Pierre platform uses these JSON-RPC foundations to implement MCP:

**Source**: src/mcp/protocol.rs:33-52
```rust
/// MCP protocol handlers
pub struct ProtocolHandler;

// Re-export types from multitenant module to avoid duplication
pub use super::multitenant::{McpError, McpRequest, McpResponse};

/// Default ID for notifications and error responses that don't have a request ID
fn default_request_id() -> Value {
    serde_json::Value::Number(serde_json::Number::from(0))
}

impl ProtocolHandler {
    /// Supported MCP protocol versions (in preference order)
    const SUPPORTED_VERSIONS: &'static [&'static str] = &["2025-06-18", "2024-11-05"];
```

**Type aliases**:
```rust
pub use super::multitenant::{McpError, McpRequest, McpResponse};
```

The `McpRequest` and `McpResponse` types are aliases for `JsonRpcRequest` and `JsonRpcResponse`. This shows how Pierre's unified JSON-RPC implementation supports multiple protocols (MCP, A2A) without duplication.

### Initialize Handler

The initialize method validates protocol versions:

**Source**: src/mcp/protocol.rs:103-173
```rust
/// Internal initialize handler
fn handle_initialize_internal(
    request: McpRequest,
    resources: Option<&Arc<ServerResources>>,
) -> McpResponse {
    let request_id = request.id.unwrap_or_else(default_request_id);

    // Parse initialize request parameters
    let Some(init_request) = request
        .params
        .as_ref()
        .and_then(|params| serde_json::from_value::<InitializeRequest>(params.clone()).ok())
    else {
        return McpResponse::error(
            Some(request_id),
            ERROR_INVALID_PARAMS,
            "Invalid initialize request parameters".to_owned(),
        );
    };

    // Validate client protocol version
    let client_version = &init_request.protocol_version;
    let negotiated_version = if Self::SUPPORTED_VERSIONS.contains(&client_version.as_str()) {
        // Use client version if supported
        client_version.clone()
    } else {
        // Return error for unsupported versions
        let supported_versions = Self::SUPPORTED_VERSIONS.join(", ");
        return McpResponse::error(
            Some(request_id),
            ERROR_VERSION_MISMATCH,
            format!("{MSG_VERSION_MISMATCH}. Client version: {client_version}, Supported versions: {supported_versions}")
        );
    };

    info!(
        "MCP version negotiated: {} (client: {}, server supports: {:?})",
        negotiated_version,
        client_version,
        Self::SUPPORTED_VERSIONS
    );

    // Create successful initialize response with negotiated version
    let init_response = if let Some(resources) = resources {
        // Use dynamic HTTP port from server configuration
        InitializeResponse::new_with_ports(
            negotiated_version,
            crate::constants::protocol::server_name_multitenant(),
            SERVER_VERSION.to_owned(),
            resources.config.http_port,
        )
    } else {
        // Fallback to default (hardcoded port)
        InitializeResponse::new(
            negotiated_version,
            crate::constants::protocol::server_name_multitenant(),
            SERVER_VERSION.to_owned(),
        )
    };

    match serde_json::to_value(&init_response) {
        Ok(result) => McpResponse::success(Some(request_id), result),
        Err(e) => {
            error!("Failed to serialize initialize response: {}", e);
            McpResponse::error(
                Some(request_id),
                ERROR_SERIALIZATION,
                format!("{MSG_SERIALIZATION}: {e}"),
            )
        }
    }
}
```

**Protocol negotiation**:
1. Client sends `{"protocol_version": "2025-06-18"}` in initialize request
2. Server checks if version is in `SUPPORTED_VERSIONS`
3. If supported, use client's version (allows newer clients)
4. If unsupported, return `ERROR_VERSION_MISMATCH` with supported versions list

This forward-compatibility pattern allows adding new protocol versions without breaking old clients.

### Ping Handler

The simplest MCP method returns an empty result:

**Source**: src/mcp/protocol.rs:175-179
```rust
/// Handle ping request
pub fn handle_ping(request: McpRequest) -> McpResponse {
    let request_id = request.id.unwrap_or_else(default_request_id);
    McpResponse::success(Some(request_id), serde_json::json!({}))
}
```

**Usage**:
```
Request:  {"jsonrpc": "2.0", "method": "ping", "id": 1}
Response: {"jsonrpc": "2.0", "result": {}, "id": 1}
```

Clients use `ping` to test connectivity and measure latency.

### tools/list Handler

The tools/list method returns available MCP tools:

**Source**: src/mcp/protocol.rs:181-187
```rust
/// Handle tools list request
pub fn handle_tools_list(request: McpRequest) -> McpResponse {
    let tools = get_tools();

    let request_id = request.id.unwrap_or_else(default_request_id);
    McpResponse::success(Some(request_id), serde_json::json!({ "tools": tools }))
}
```

**Response structure**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "tools": [
      {
        "name": "get_activities",
        "description": "Fetch fitness activities from connected providers",
        "inputSchema": {...}
      },
      ...
    ]
  },
  "id": 1
}
```

## Error Handling Patterns

The platform uses consistent error handling across JSON-RPC methods:

### Method Not Found

**Source**: src/mcp/protocol.rs:336-343
```rust
/// Handle unknown method request
pub fn handle_unknown_method(request: McpRequest) -> McpResponse {
    let request_id = request.id.unwrap_or_else(default_request_id);
    McpResponse::error(
        Some(request_id),
        ERROR_METHOD_NOT_FOUND,
        format!("Unknown method: {}", request.method),
    )
}
```

### Invalid Params

```rust
return McpResponse::error(
    Some(request_id),
    ERROR_INVALID_PARAMS,
    "Invalid initialize request parameters".to_owned(),
);
```

### Authentication Errors

```rust
return McpResponse::error(
    Some(request_id),
    ERROR_AUTHENTICATION,
    "Authentication token required".to_owned(),
);
```

**Pattern**: All error responses follow the same structure:
1. Extract request ID (or use default for notifications)
2. Call `McpResponse::error()` with appropriate code
3. Include actionable error message
4. Optionally add `data` field with details

## MCP Version Compatibility

Pierre implements version negotiation during the `initialize` handshake to ensure compatibility with different MCP client versions.

### Version Negotiation Flow

```
Client                                 Server
  │                                      │
  │  ──── initialize ──────────────►     │
  │  {                                   │
  │    "method": "initialize",           │
  │    "params": {                       │
  │      "protocolVersion": "2024-11-05",│
  │      "clientInfo": {...}             │
  │    }                                 │
  │  }                                   │
  │                                      │
  │  ◄──── initialized ────────────      │
  │  {                                   │
  │    "result": {                       │
  │      "protocolVersion": "2024-11-05",│  (echo or negotiate down)
  │      "serverInfo": {...},            │
  │      "capabilities": {...}           │
  │    }                                 │
  │  }                                   │
  └──────────────────────────────────────┘
```

### Supported Protocol Versions

| Version | Status | Notes |
|---------|--------|-------|
| `2024-11-05` | Current | Full feature support |
| `2024-10-07` | Supported | Backward compatible |
| `1.0` | Legacy | Basic tool support |

### Version Handling Logic

```rust
/// Handle protocol version negotiation
fn negotiate_version(client_version: &str) -> Result<String, ProtocolError> {
    match client_version {
        // Current version - full support
        "2024-11-05" => Ok("2024-11-05".to_owned()),

        // Previous version - backward compatible
        "2024-10-07" => Ok("2024-10-07".to_owned()),

        // Legacy version - limited features
        "1.0" | "1" => {
            tracing::warn!(
                client_version = client_version,
                "Client using legacy MCP version, some features unavailable"
            );
            Ok("1.0".to_owned())
        }

        // Unknown version - try to continue with current
        unknown => {
            tracing::warn!(
                client_version = unknown,
                server_version = "2024-11-05",
                "Unknown client version, attempting compatibility"
            );
            Ok("2024-11-05".to_owned())
        }
    }
}
```

### Capability Negotiation

Different versions expose different capabilities:

```rust
/// Get capabilities for protocol version
fn capabilities_for_version(version: &str) -> ServerCapabilities {
    match version {
        "2024-11-05" => ServerCapabilities {
            tools: true,
            resources: true,
            prompts: true,
            logging: true,
            experimental: Some(ExperimentalCapabilities {
                a2a: true,
                streaming: true,
            }),
        },
        "2024-10-07" => ServerCapabilities {
            tools: true,
            resources: true,
            prompts: false,  // Not available in older version
            logging: false,
            experimental: None,
        },
        _ => ServerCapabilities::minimal(), // Basic tool support only
    }
}
```

### Breaking Changes Policy

Pierre follows semantic versioning for API changes:

**Breaking changes** (require major version bump):
- Removing a tool from the registry
- Changing tool parameter types
- Changing response structure
- Removing capabilities

**Non-breaking changes** (minor version):
- Adding new tools
- Adding optional parameters
- Adding new capabilities
- Adding new response fields

**Deprecation process**:
1. Mark feature as deprecated in current version
2. Log warnings when deprecated features are used
3. Remove in next major version
4. Document migration path in release notes

### Client Version Detection

Pierre logs client information for compatibility tracking:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

// Example client identification
// Claude Desktop: {"name": "claude-desktop", "version": "0.7.0"}
// VSCode Copilot: {"name": "vscode-copilot", "version": "1.2.3"}
```

### Forward Compatibility

For unknown future versions, Pierre:
1. Logs a warning about unknown version
2. Responds with current server version
3. Excludes experimental features
4. Allows basic tool operations

This ensures new clients can still use Pierre even if server hasn't been updated.

## Key Takeaways

1. **JSON-RPC 2.0 foundation**: Lightweight RPC protocol with 4 message types (request, response, error, notification). Transport-agnostic and bidirectional.

2. **Unified implementation**: Pierre uses one JSON-RPC implementation for MCP and A2A protocols, eliminating duplication.

3. **Request structure**: Contains `jsonrpc`, `method`, `params`, and `id`. Pierre adds `auth_token`, `headers`, and `metadata` for authentication and multi-tenancy.

4. **Response structure**: Contains `jsonrpc`, `result` or `error` (mutually exclusive), and `id` matching the request.

5. **Error codes**: Standard codes (-32700 to -32600) for protocol errors. Server-specific codes (-32000 to -32099) for application errors.

6. **Request correlation**: The `id` field correlates requests with responses in async bidirectional communication. Notifications omit `id` (no response expected).

7. **Custom Debug implementation**: `JsonRpcRequest` redacts auth tokens in debug output to prevent token leakage in logs.

8. **Protocol versioning**: MCP initialize method negotiates protocol version with client, allowing forward compatibility.

9. **Extension fields**: Pierre extends JSON-RPC with optional fields while maintaining spec compliance (fields are skipped if absent).

10. **Type safety**: Rust's type system with serde ensures valid JSON-RPC messages. Invalid messages are caught at deserialization.

---

**Next Chapter**: [Chapter 10: MCP Protocol Deep Dive - Request Flow](./chapter-10-mcp-request-flow.md) - Learn how the Pierre platform routes MCP requests through authentication, tenant isolation, tool registry, and response serialization.
