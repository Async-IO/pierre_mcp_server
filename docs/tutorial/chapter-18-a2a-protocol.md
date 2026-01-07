<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 18: A2A Protocol - Agent-to-Agent Communication

This chapter explores how Pierre implements the Agent-to-Agent (A2A) protocol for secure inter-agent communication. You'll learn about the A2A protocol architecture, Ed25519 signatures, agent capability discovery, and JSON-RPC-based messaging between AI agents.

## What You'll Learn

- A2A protocol architecture
- JSON-RPC 2.0 foundation for A2A
- Agent capability discovery with agent cards
- Ed25519 public key authentication
- A2A client registration and authentication
- Rate limiting for A2A clients
- Long-running tasks and notifications
- Message streaming and structured data
- A2A authentication with API keys

## A2A Protocol Overview

A2A (Agent-to-Agent) protocol enables AI agents to communicate and collaborate:

```
┌──────────────┐                  ┌──────────────┐                  ┌──────────────┐
│   Agent A    │                  │   Pierre     │                  │   Agent B    │
│  (Claude)    │                  │   A2A Server │                  │  (Other AI)  │
└──────────────┘                  └──────────────┘                  └──────────────┘
        │                                 │                                 │
        │  1. Get Agent Card              │                                 │
        ├────────────────────────────────►│                                 │
        │  (discover capabilities)        │                                 │
        │                                 │                                 │
        │  2. Register A2A Client         │                                 │
        │  (with Ed25519 public key)      │                                 │
        ├────────────────────────────────►│                                 │
        │                                 │                                 │
        │  3. Initialize session          │                                 │
        │  (negotiate protocol version)   │                                 │
        ├────────────────────────────────►│                                 │
        │                                 │                                 │
        │  4. Send message                │                                 │
        │  (with Ed25519 signature)       │                                 │
        ├────────────────────────────────►│                                 │
        │                                 │   5. Forward message            │
        │                                 ├────────────────────────────────►│
        │                                 │                                 │
        │  6. Stream response             │                                 │
        │◄────────────────────────────────┤                                 │
```

**A2A use cases**:
- **Multi-agent workflows**: Claude orchestrates Pierre for fitness analysis
- **Task delegation**: Long-running analytics tasks with progress updates
- **Capability discovery**: Agents learn what other agents can do
- **Secure messaging**: Ed25519 signatures prevent message tampering

## JSON-RPC 2.0 Foundation

A2A protocol uses JSON-RPC 2.0 for all communication:

**Source**: src/a2a/protocol.rs:23-28
```rust
// Phase 2: Type aliases pointing to unified JSON-RPC foundation

/// A2A protocol request (JSON-RPC 2.0 request)
pub type A2ARequest = crate::jsonrpc::JsonRpcRequest;
/// A2A protocol response (JSON-RPC 2.0 response)
pub type A2AResponse = crate::jsonrpc::JsonRpcResponse;
```

**Design choice**: A2A reuses the same JSON-RPC infrastructure as MCP (Chapter 9), ensuring consistency and reducing code duplication.

## A2A Error Types

A2A defines protocol-specific errors mapped to JSON-RPC error codes:

**Source**: src/a2a/protocol.rs:31-69
```rust
/// A2A Protocol Error types
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum A2AError {
    /// Invalid request parameters or format
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    /// Client not registered
    #[error("Client not registered: {0}")]
    ClientNotRegistered(String),
    /// Database operation failed
    #[error("Database error: {0}")]
    DatabaseError(String),
    /// Client has been deactivated
    #[error("Client deactivated: {0}")]
    ClientDeactivated(String),
    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
    /// Session expired or invalid
    #[error("Session expired: {0}")]
    SessionExpired(String),
    /// Insufficient permissions
    #[error("Insufficient permissions: {0}")]
    InsufficientPermissions(String),
    // ... more error types
}
```

**Error code mapping**:

**Source**: src/a2a/protocol.rs:76-95
```rust
impl From<A2AError> for A2AErrorResponse {
    fn from(error: A2AError) -> Self {
        let (code, message) = match error {
            A2AError::InvalidRequest(msg) => (-32602, format!("Invalid params: {msg}")),
            A2AError::AuthenticationFailed(msg) => {
                (-32001, format!("Authentication failed: {msg}"))
            }
            A2AError::ClientNotRegistered(msg) => (-32003, format!("Client not registered: {msg}")),
            A2AError::RateLimitExceeded(msg) => (-32005, format!("Rate limit exceeded: {msg}")),
            A2AError::SessionExpired(msg) => (-32006, format!("Session expired: {msg}")),
            A2AError::InsufficientPermissions(msg) => {
                (-32008, format!("Insufficient permissions: {msg}"))
            }
            // ... more error mappings
        };

        Self {
            code,
            message,
            data: None,
        }
    }
}
```

**Error code ranges**:
- `-32600` to `-32699`: JSON-RPC reserved codes
- `-32000` to `-32099`: Server-defined errors
- `-32001` to `-32010`: A2A-specific error codes

## A2A Client Structure

A2A clients have identities, public keys, and capabilities:

**Source**: src/a2a/auth.rs:34-68
```rust
/// A2A Client registration information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AClient {
    /// Unique client identifier
    pub id: String,
    /// User ID for session tracking and consistency
    pub user_id: uuid::Uuid,
    /// Human-readable client name
    pub name: String,
    /// Description of the client application
    pub description: String,
    /// Public key for signature verification
    pub public_key: String,
    /// List of capabilities this client can access
    pub capabilities: Vec<String>,
    /// Allowed OAuth redirect URIs
    pub redirect_uris: Vec<String>,
    /// Whether this client is active
    pub is_active: bool,
    /// When this client was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// List of permissions granted to this client
    #[serde(default = "default_permissions")]
    pub permissions: Vec<String>,
    /// Maximum requests allowed per window
    #[serde(default = "default_rate_limit_requests")]
    pub rate_limit_requests: u32,
    /// Rate limit window duration in seconds
    #[serde(default = "default_rate_limit_window")]
    pub rate_limit_window_seconds: u32,
}
```

**Key fields**:
- `public_key`: Ed25519 public key for signature verification
- `permissions`: Granted access (e.g., `read_activities`, `write_goals`)
- `rate_limit_requests`: Max requests per time window
- `is_active`: Admin can deactivate misbehaving clients

## A2A Initialization Flow

Agents initialize sessions with protocol negotiation:

**Source**: src/a2a/protocol.rs:105-123
```rust
/// A2A Initialize Request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AInitializeRequest {
    /// A2A protocol version
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    /// Client information
    #[serde(rename = "clientInfo")]
    pub client_info: A2AClientInfo,
    /// Client capabilities
    pub capabilities: Vec<String>,
    /// Optional OAuth application credentials provided by the client
    #[serde(
        rename = "oauthCredentials",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub oauth_credentials: Option<HashMap<String, crate::mcp::schema::OAuthAppCredentials>>,
}
```

**Initialization request** (JSON):
```json
{
  "jsonrpc": "2.0",
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-11-15",
    "clientInfo": {
      "name": "Claude Agent",
      "version": "1.0.0"
    },
    "capabilities": [
      "message/send",
      "message/stream",
      "tasks/create"
    ]
  },
  "id": 1
}
```

**Initialization response**:

**Source**: src/a2a/protocol.rs:162-187
```rust
impl A2AInitializeResponse {
    /// Create a new A2A initialize response with server information
    #[must_use]
    pub fn new(protocol_version: String, server_name: String, server_version: String) -> Self {
        Self {
            protocol_version,
            server_info: A2AServerInfo {
                name: server_name,
                version: server_version,
                description: Some(
                    "AI-powered fitness data analysis and insights platform".to_owned(),
                ),
            },
            capabilities: vec![
                "message/send".to_owned(),
                "message/stream".to_owned(),
                "tasks/create".to_owned(),
                "tasks/get".to_owned(),
                "tasks/cancel".to_owned(),
                "tasks/pushNotificationConfig/set".to_owned(),
                "tools/list".to_owned(),
                "tools/call".to_owned(),
            ],
        }
    }
}
```

**Capability negotiation**: Server returns intersection of client-requested and server-supported capabilities.

## A2A Message Structure

Messages support text, structured data, and file attachments:

**Source**: src/a2a/protocol.rs:189-227
```rust
/// A2A Message structure for agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AMessage {
    /// Unique message identifier
    pub id: String,
    /// Message content parts (text, data, or files)
    pub parts: Vec<MessagePart>,
    /// Optional metadata key-value pairs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,
}

/// A2A Message Part types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessagePart {
    /// Plain text message content
    #[serde(rename = "text")]
    Text {
        /// Text content
        content: String,
    },
    /// Structured data content (JSON)
    #[serde(rename = "data")]
    Data {
        /// Data content as JSON value
        content: Value,
    },
    /// File attachment content
    #[serde(rename = "file")]
    File {
        /// File name
        name: String,
        /// MIME type of the file
        mime_type: String,
        /// File content (base64 encoded)
        content: String,
    },
}
```

**Example message** (JSON):
```json
{
  "id": "msg_abc123",
  "parts": [
    {
      "type": "text",
      "content": "Analyzing your recent running activities..."
    },
    {
      "type": "data",
      "content": {
        "activities_analyzed": 10,
        "average_pace": "5:30/km",
        "trend": "improving"
      }
    }
  ],
  "metadata": {
    "agent": "Pierre",
    "timestamp": "2025-11-15T10:00:00Z"
  }
}
```

## A2A Authentication

A2A supports API key authentication with rate limiting:

**Source**: src/a2a/auth.rs:95-113
```rust
/// Authenticate an A2A request using API key
///
/// # Errors
///
/// Returns an error if:
/// - The API key format is invalid
/// - Authentication fails
/// - Rate limits are exceeded
pub async fn authenticate_api_key(&self, api_key: &str) -> Result<AuthResult, anyhow::Error> {
    // Check if it's an A2A-specific API key (with a2a_ prefix)
    if api_key.starts_with("a2a_") {
        return self.authenticate_a2a_key(api_key).await;
    }

    // Use standard API key authentication through MCP middleware
    let middleware = &self.resources.auth_middleware;

    middleware.authenticate_request(Some(api_key)).await
}
```

**A2A-specific authentication**:

**Source**: src/a2a/auth.rs:116-181
```rust
/// Authenticate A2A-specific API key with rate limiting
async fn authenticate_a2a_key(&self, api_key: &str) -> Result<AuthResult, anyhow::Error> {
    // Extract key components (similar to API key validation)
    if !api_key.starts_with("a2a_") || api_key.len() < 16 {
        return Err(AppError::auth_invalid("Invalid A2A API key format").into());
    }

    let middleware = &self.resources.auth_middleware;

    // First authenticate using regular API key system
    let mut auth_result = middleware.authenticate_request(Some(api_key)).await?;

    // Add A2A-specific rate limiting
    if let AuthMethod::ApiKey { key_id, tier: _ } = &auth_result.auth_method {
        // Find A2A client associated with this API key
        if let Some(client) = self.get_a2a_client_by_api_key(key_id).await? {
            let client_manager = &*self.resources.a2a_client_manager;

            // Check A2A-specific rate limits
            let rate_limit_status = client_manager
                .get_client_rate_limit_status(&client.id)
                .await?;

            if rate_limit_status.is_rate_limited {
                return Err(ProviderError::RateLimitExceeded {
                    provider: "A2A Client Authentication".to_owned(),
                    retry_after_secs: /* calculate from reset_at */,
                    limit_type: format!(
                        "A2A client rate limit exceeded. Limit: {}, Reset at: {}",
                        rate_limit_status.limit.unwrap_or(0),
                        rate_limit_status.reset_at.map_or_else(|| "unknown".into(), |dt| dt.to_rfc3339())
                    ),
                }
                .into());
            }

            // Update auth method to indicate A2A authentication
            auth_result.auth_method = AuthMethod::ApiKey {
                key_id: key_id.clone(),
                tier: format!("A2A-{}", rate_limit_status.tier.display_name()),
            };
        }
    }

    Ok(auth_result)
}
```

**Rate limiting flow**:
1. **Validate API key format**: Must start with `a2a_` and have minimum length
2. **Standard authentication**: Use existing API key middleware
3. **Lookup A2A client**: Find client associated with API key
4. **Check rate limits**: Enforce A2A-specific rate limits
5. **Return auth result**: Include rate limit status in response

## Agent Capability Discovery

Agents advertise capabilities through agent cards:

**Source**: src/a2a/agent_card.rs:16-34
```rust
/// A2A Agent Card for Pierre
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCard {
    /// Agent name ("Pierre Fitness AI")
    pub name: String,
    /// Human-readable description of the agent's capabilities
    pub description: String,
    /// Agent version number
    pub version: String,
    /// List of high-level capabilities (e.g., "fitness-data-analysis")
    pub capabilities: Vec<String>,
    /// Authentication methods supported
    pub authentication: AuthenticationInfo,
    /// Available tools/endpoints with schemas
    pub tools: Vec<ToolDefinition>,
    /// Optional additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,
}
```

**Agent card example** (Pierre):

**Source**: src/a2a/agent_card.rs:98-135
```rust
impl AgentCard {
    /// Create a new Agent Card for Pierre
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "Pierre Fitness AI".into(),
            description: "AI-powered fitness data analysis and insights platform providing comprehensive activity analysis, performance tracking, and intelligent recommendations for athletes and fitness enthusiasts.".into(),
            version: "1.0.0".into(),
            capabilities: vec![
                "fitness-data-analysis".into(),
                "activity-intelligence".into(),
                "goal-management".into(),
                "performance-prediction".into(),
                "training-analytics".into(),
                "provider-integration".into(),
            ],
            authentication: AuthenticationInfo {
                schemes: vec!["api-key".into(), "oauth2".into()],
                oauth2: Some(OAuth2Info {
                    authorization_url: "https://pierre.ai/oauth/authorize".into(),
                    token_url: "https://pierre.ai/oauth/token".into(),
                    scopes: vec![
                        "fitness:read".into(),
                        "analytics:read".into(),
                        "goals:read".into(),
                        "goals:write".into(),
                    ],
                }),
                api_key: Some(ApiKeyInfo {
                    header_name: "Authorization".into(),
                    prefix: Some("Bearer".into()),
                    registration_url: "https://pierre.ai/api/keys/request".into(),
                }),
            },
            tools: Self::create_tool_definitions(),
            metadata: Some(Self::create_metadata()),
        }
    }
}
```

**Tool definition in agent card**:

**Source**: src/a2a/agent_card.rs:140-200
```rust
ToolDefinition {
    name: "get_activities".into(),
    description: "Retrieve user fitness activities from connected providers".to_owned(),
    input_schema: serde_json::json!({
        "type": "object",
        "properties": {
            "limit": {
                "type": "number",
                "description": "Number of activities to retrieve (max 100)",
                "minimum": 1,
                "maximum": 100,
                "default": 10
            },
            "before": {
                "type": "string",
                "format": "date-time",
                "description": "ISO 8601 date to get activities before"
            },
            "provider": {
                "type": "string",
                "enum": ["strava", "fitbit"],
                "description": "Specific provider to query (optional)"
            }
        }
    }),
    output_schema: serde_json::json!({
        "type": "object",
        "properties": {
            "activities": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "name": {"type": "string"},
                        "sport_type": {"type": "string"},
                        "start_date": {"type": "string", "format": "date-time"},
                        "duration_seconds": {"type": "number"},
                        "distance_meters": {"type": "number"},
                        "elevation_gain": {"type": "number"}
                    }
                }
            },
            "total_count": {"type": "number"}
        }
    }),
    examples: Some(vec![ToolExample {
        description: "Get recent activities".into(),
        input: serde_json::json!({"limit": 5}),
        output: serde_json::json!({/* example output */}),
    }]),
}
```

**Agent card benefits**:
- **Discoverability**: Agents learn what Pierre can do without documentation
- **JSON Schema**: Input/output schemas enable automatic validation
- **Examples**: Sample usage helps agents understand tool behavior
- **Authentication**: Agents know how to authenticate (OAuth2, API keys)

## Ed25519 Signatures

A2A uses Ed25519 for message authentication:

**Ed25519 key generation** (conceptual from src/a2a/client.rs:226):
```rust
// Generate Ed25519 keypair for the client
let signing_key = ed25519_dalek::SigningKey::generate(&mut OsRng);
let public_key = signing_key.verifying_key();

// Store public key in A2A client record
A2AClient {
    public_key: base64::encode(public_key.to_bytes()),
    key_type: "ed25519".into(),
    // ... other fields
}
```

**Why Ed25519**:
- **Fast**: Much faster than RSA for both signing and verification
- **Small keys**: 32-byte public keys (vs 256+ bytes for RSA)
- **Secure**: 128-bit security level, resistant to timing attacks
- **Deterministic**: Same message always produces same signature (unlike ECDSA)

**Signature verification** (conceptual):
```rust
fn verify_signature(
    message: &[u8],
    signature: &[u8],
    public_key_base64: &str,
) -> Result<(), A2AError> {
    let public_key_bytes = base64::decode(public_key_base64)?;
    let public_key = VerifyingKey::from_bytes(&public_key_bytes)?;
    let signature = Signature::from_bytes(signature.try_into()?);

    public_key
        .verify(message, &signature)
        .map_err(|_| A2AError::AuthenticationFailed("Invalid signature".into()))
}
```

## A2A Tasks

A2A supports long-running tasks with progress tracking:

**Source**: src/a2a/protocol.rs:229-250 (conceptual)
```rust
/// A2A Task structure for long-running operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ATask {
    /// Unique task identifier
    pub id: String,
    /// Current status of the task
    pub status: TaskStatus,
    /// When the task was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the task completed (if finished)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Task result data (if completed successfully)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Client ID that created this task
    pub client_id: String,
    /// Type of task being performed
    pub task_type: String,
}
```

**Task lifecycle**:
```
Created → Running → Completed
                  ↘ Failed
                  ↘ Cancelled
```

**Task notifications**: Server pushes progress updates via Server-Sent Events (SSE).

## Key Takeaways

1. **JSON-RPC foundation**: A2A reuses the same JSON-RPC infrastructure as MCP.

2. **Agent cards**: Self-describing capabilities enable dynamic discovery without documentation.

3. **Ed25519 signatures**: Fast, secure public key authentication for agent messages.

4. **Structured messages**: Support text, JSON data, and base64-encoded file attachments.

5. **Rate limiting**: A2A clients have separate rate limits from regular API keys.

6. **API key prefix**: A2A API keys use `a2a_` prefix to distinguish from standard API keys.

7. **Protocol negotiation**: Clients and servers negotiate supported capabilities during initialization.

8. **Long-running tasks**: Async operations return task IDs with progress tracking.

9. **Error codes**: A2A-specific error codes in -32001 to -32010 range.

10. **Tool schemas**: JSON Schema for input/output enables automatic validation and client generation.

11. **Multi-part messages**: Single message can contain multiple content parts (text + data + files).

12. **Permission model**: A2A clients have granular permissions (read_activities, write_goals, etc.).

---

**End of Part V: OAuth, A2A & Providers**

You've completed the OAuth and provider integration section. You now understand:
- OAuth 2.0 server implementation (Chapter 15)
- OAuth 2.0 client for fitness providers (Chapter 16)
- Provider data models and rate limiting (Chapter 17)
- A2A protocol for agent communication (Chapter 18)

**Next Chapter**: [Chapter 19: Comprehensive Tools Guide](./chapter-19-tools-guide.md) - Begin Part VI by learning about all 45+ MCP tools Pierre provides for fitness data analysis, how to use them with natural language prompts, and tool categorization.
