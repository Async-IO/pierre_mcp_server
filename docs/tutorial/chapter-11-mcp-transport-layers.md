<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 11: MCP Transport Layers

This chapter explores how the Pierre Fitness Platform supports multiple transport mechanisms for MCP communication. You'll learn about stdio, HTTP, WebSocket, and SSE transports, and how the platform coordinates them for flexible client integration.

## What You'll Learn

- Transport layer abstraction for MCP protocol
- stdio transport for command-line clients
- HTTP transport for web clients and APIs
- Server-Sent Events (SSE) for notifications
- WebSocket transport for bidirectional communication
- Transport coordination and multiplexing
- Notification broadcasting across transports
- Transport-agnostic request processing
- Error handling per transport type

## Transport Abstraction Overview

MCP is transport-agnostic - the same JSON-RPC messages work over any transport:

```
┌──────────────────────────────────────────────────────────┐
│                   MCP Protocol Layer                     │
│         (JSON-RPC requests/responses)                    │
└─────────────────┬────────────────────────────────────────┘
                  │
      ┌───────────┴───────────┬───────────┬────────────┐
      │                       │           │            │
      ▼                       ▼           ▼            ▼
┌──────────┐          ┌──────────┐  ┌────────┐  ┌────────┐
│  stdio   │          │   HTTP   │  │  SSE   │  │  WS    │
│  (CLI)   │          │  (Web)   │  │(Notify)│  │(Bidir) │
└──────────┘          └──────────┘  └────────┘  └────────┘
```

**Source**: src/mcp/transport_manager.rs:18-34
```rust
/// Manages multiple transport methods for MCP communication
pub struct TransportManager {
    resources: Arc<ServerResources>,
    notification_sender: broadcast::Sender<OAuthCompletedNotification>,
}

impl TransportManager {
    /// Create a new transport manager with shared resources
    #[must_use]
    pub fn new(resources: Arc<ServerResources>) -> Self {
        let (notification_sender, _) = broadcast::channel(100);
        Self {
            resources,
            notification_sender,
        }
    }
```

**Design**: Single `TransportManager` coordinates all transports using `broadcast::channel` for notifications.

## Stdio Transport

The stdio transport reads JSON-RPC from stdin and writes to stdout:

**Source**: src/mcp/transport_manager.rs:116-180
```rust
/// Handles stdio transport for MCP communication
pub struct StdioTransport {
    resources: Arc<ServerResources>,
}

impl StdioTransport {
    /// Creates a new stdio transport instance
    #[must_use]
    pub const fn new(resources: Arc<ServerResources>) -> Self {
        Self { resources }
    }

    /// Run stdio transport for MCP communication
    ///
    /// # Errors
    /// Returns an error if stdio processing fails
    pub async fn run(
        &self,
        notification_receiver: broadcast::Receiver<OAuthCompletedNotification>,
    ) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, BufReader};

        info!("MCP stdio transport ready - listening on stdin/stdout");

        let stdin = tokio::io::stdin();
        let mut lines = BufReader::new(stdin).lines();

        // Spawn notification handler for stdio transport
        let resources_for_notifications = self.resources.clone();
        let notification_handle = tokio::spawn(async move {
            Self::handle_stdio_notifications(notification_receiver, resources_for_notifications)
                .await
        });

        // Main stdio loop
        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }

            match Self::process_stdio_line(&line) {
                Ok(response) => {
                    if let Some(resp) = response {
                        println!("{resp}");
                    }
                }
                Err(e) => {
                    warn!("Error processing stdio input: {}", e);
                    let error_response = serde_json::json!({
                        "jsonrpc": "2.0",
                        "error": {
                            "code": -32603,
                            "message": "Internal error"
                        },
                        "id": null
                    });
                    println!("{error_response}");
                }
            }
        }

        // Clean up notification handler
        notification_handle.abort();
        Ok(())
    }
```

**stdio protocol**:
- **Input**: Read lines from stdin (`tokio::io::stdin()`)
- **Parse**: Deserialize JSON-RPC from line
- **Process**: Route through `McpRequestProcessor`
- **Output**: Write JSON response to stdout (`println!`)

**Use cases**:
- Claude Desktop MCP integration
- Command-line tools
- Process spawning (subprocess communication)
- Testing and debugging

**Rust Idiom**: `AsyncBufReadExt` for line-based I/O

The `BufReader::new(stdin).lines()` pattern provides async line iteration. Each `next_line()` call waits for a complete line (terminated by `\n`), which matches JSON-RPC line-delimited protocol.

## HTTP Transport

HTTP transport serves MCP over REST endpoints:

**Source**: src/mcp/transport_manager.rs:89-112
```rust
// Run unified HTTP server with all routes (OAuth2, MCP, etc.) - this should run indefinitely
loop {
    info!("Starting unified Axum HTTP server on port {}", port);

    // Clone shared resources for each iteration since run_http_server_with_resources takes ownership
    let server = super::multitenant::MultiTenantMcpServer::new(shared_resources.clone());

    let result = server
        .run_http_server_with_resources_axum(port, shared_resources.clone())
        .await;

    match result {
        Ok(()) => {
            error!("HTTP server unexpectedly completed - this should never happen");
            error!("HTTP server should run indefinitely. Restarting in 5 seconds...");
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
        Err(e) => {
            error!("HTTP server failed: {}", e);
            error!("Restarting HTTP server in 10 seconds...");
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    }
}
```

**Features**:
- Axum web framework for routing
- REST endpoints for MCP methods
- CORS support for web clients
- TLS/HTTPS support (production)
- Rate limiting per endpoint

**Typical endpoints**:
```
POST /mcp/initialize    - Initialize MCP session
POST /mcp/tools/list    - List available tools
POST /mcp/tools/call    - Execute tool
GET  /mcp/ping          - Health check
GET  /oauth/authorize   - OAuth flow start
POST /oauth/callback    - OAuth callback
```

## Sse Transport (Notifications)

Server-Sent Events provide server-to-client notifications:

**Source**: src/mcp/transport_manager.rs:80-87
```rust
// Start SSE notification forwarder task
let resources_for_sse = shared_resources.clone();
tokio::spawn(async move {
    let sse_forwarder = SseNotificationForwarder::new(resources_for_sse);
    if let Err(e) = sse_forwarder.run(sse_notification_receiver).await {
        error!("SSE notification forwarder failed: {}", e);
    }
});
```

**SSE characteristics**:
- **Unidirectional**: Server → Client only
- **Long-lived**: Connection stays open
- **Text-based**: Sends `data:` prefixed messages
- **Auto-reconnect**: Browsers reconnect on disconnect

**MCP notifications over SSE**:
- OAuth flow completion
- Tool execution progress
- Resource updates
- Prompt changes

**Example SSE event**:
```
data: {"jsonrpc":"2.0","method":"notifications/oauth_completed","params":{"provider":"strava","status":"success"}}

```

## Websocket Transport (Bidirectional)

WebSocket provides full-duplex bidirectional communication for real-time updates:

**Source**: src/websocket.rs:84-124
```rust
/// Manages WebSocket connections and message broadcasting
#[derive(Clone)]
pub struct WebSocketManager {
    database: Arc<Database>,
    auth_middleware: McpAuthMiddleware,
    clients: Arc<RwLock<HashMap<Uuid, ClientConnection>>>,
    broadcast_tx: broadcast::Sender<WebSocketMessage>,
}

impl WebSocketManager {
    /// Creates a new WebSocket manager instance
    #[must_use]
    pub fn new(
        database: Arc<Database>,
        auth_manager: &Arc<AuthManager>,
        jwks_manager: &Arc<crate::admin::jwks::JwksManager>,
        rate_limit_config: crate::config::environment::RateLimitConfig,
    ) -> Self {
        let (broadcast_tx, _) =
            broadcast::channel(crate::constants::rate_limits::WEBSOCKET_CHANNEL_CAPACITY);
        let auth_middleware = McpAuthMiddleware::new(
            (**auth_manager).clone(),
            database.clone(),
            jwks_manager.clone(),
            rate_limit_config,
        );

        Self {
            database,
            auth_middleware,
            clients: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
        }
    }
```

**WebSocket message types**:

**Source**: src/websocket.rs:32-82
```rust
/// WebSocket message types for real-time communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    /// Client authentication message
    #[serde(rename = "auth")]
    Authentication {
        token: String,
    },
    /// Subscribe to specific topics
    #[serde(rename = "subscribe")]
    Subscribe {
        topics: Vec<String>,
    },
    /// API key usage update notification
    #[serde(rename = "usage_update")]
    UsageUpdate {
        api_key_id: String,
        requests_today: u64,
        requests_this_month: u64,
        rate_limit_status: Value,
    },
    /// System-wide statistics update
    #[serde(rename = "system_stats")]
    SystemStats {
        total_requests_today: u64,
        total_requests_this_month: u64,
        active_connections: usize,
    },
    /// Error message to client
    #[serde(rename = "error")]
    Error {
        message: String,
    },
    /// Success confirmation message
    #[serde(rename = "success")]
    Success {
        message: String,
    },
}
```

**Connection handling**:

**Source**: src/websocket.rs:203-266
```rust
/// Handle incoming WebSocket connection
pub async fn handle_connection(&self, ws: axum::extract::ws::WebSocket) {
    let (mut ws_tx, mut ws_rx) = ws.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    let connection_id = Uuid::new_v4();
    let mut authenticated_user: Option<Uuid> = None;
    let mut subscriptions: Vec<String> = Vec::new();

    // Spawn task to forward messages to WebSocket
    let ws_send_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if ws_tx.send(message).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages
    while let Some(msg) = ws_rx.next().await {
        match msg {
            Ok(Message::Text(text)) => match serde_json::from_str::<WebSocketMessage>(&text) {
                Ok(WebSocketMessage::Authentication { token }) => {
                    authenticated_user = self.handle_auth_message(&token, &tx).await;
                }
                Ok(WebSocketMessage::Subscribe { topics }) => {
                    subscriptions = Self::handle_subscribe_message(topics, authenticated_user, &tx);
                }
                // ... error handling
                _ => {}
            },
            Ok(Message::Close(_)) | Err(_) => break,
            _ => {}
        }
    }

    // Store authenticated connection
    if let Some(user_id) = authenticated_user {
        let client = ClientConnection {
            user_id,
            subscriptions,
            tx: tx.clone(),
        };
        self.clients.write().await.insert(connection_id, client);
    }

    // Clean up on disconnect
    ws_send_task.abort();
    self.clients.write().await.remove(&connection_id);
}
```

**WebSocket authentication flow**:
1. Client connects to `/ws` endpoint
2. Client sends `{"type":"auth","token":"Bearer ..."}` message
3. Server validates JWT using `McpAuthMiddleware`
4. Server responds with `{"type":"success"}` or `{"type":"error"}`
5. Authenticated client can subscribe to topics

**Topic subscription**:
```json
{
  "type": "subscribe",
  "topics": ["usage", "system"]
}
```

**Broadcasting updates**:

**Source**: src/websocket.rs:281-299
```rust
/// Broadcast usage update to subscribed clients
pub async fn broadcast_usage_update(
    &self,
    api_key_id: &str,
    user_id: &Uuid,
    requests_today: u64,
    requests_this_month: u64,
    rate_limit_status: Value,
) {
    let message = WebSocketMessage::UsageUpdate {
        api_key_id: api_key_id.to_owned(),
        requests_today,
        requests_this_month,
        rate_limit_status,
    };

    self.send_to_user_subscribers(user_id, &message, "usage").await;
}
```

**Periodic system stats**:

**Source**: src/websocket.rs:384-399
```rust
/// Start background task for periodic updates
pub fn start_periodic_updates(&self) {
    let manager = self.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(30)); // Update every 30 seconds

        loop {
            interval.tick().await;

            // Broadcast system stats
            if let Err(e) = manager.broadcast_system_stats().await {
                tracing::warn!("Failed to broadcast system stats: {}", e);
            }
        }
    });
}
```

**WebSocket characteristics**:
- **Bidirectional**: Full-duplex client ↔ server communication
- **JWT authentication**: Required before subscribing
- **Topic-based subscriptions**: Clients choose what to receive
- **Broadcast channels**: `tokio::sync::broadcast` for efficient distribution
- **Connection tracking**: `HashMap<Uuid, ClientConnection>` with `RwLock`
- **Automatic cleanup**: Connections removed on disconnect
- **Periodic updates**: System stats every 30 seconds

**Use cases**:
- Real-time API usage monitoring
- Rate limit status updates
- System health dashboards
- Live fitness data streaming
- OAuth flow status updates

**Rust Idiom**: WebSocket connection splitting

The `ws.split()` pattern separates the WebSocket into independent read and write halves. This allows concurrent sending/receiving without conflicts. The `mpsc::unbounded_channel` bridges the write half to the message handler, decoupling message generation from socket I/O.

## Transport Coordination

The `TransportManager` starts all transports concurrently:

**Source**: src/mcp/transport_manager.rs:39-78
```rust
/// Start all transport methods (stdio, HTTP, SSE) in coordinated fashion
///
/// # Errors
/// Returns an error if transport setup or server startup fails
pub async fn start_all_transports(&self, port: u16) -> Result<()> {
    info!(
        "Transport manager coordinating all transports on port {}",
        port
    );

    // Delegate to the unified server implementation
    self.start_legacy_unified_server(port).await
}

/// Unified server startup using existing transport coordination
async fn start_legacy_unified_server(&self, port: u16) -> Result<()> {
    info!("Starting MCP server with stdio and HTTP transports (Axum framework)");

    // Use the notification sender from the struct instance
    let notification_receiver = self.notification_sender.subscribe();
    let sse_notification_receiver = self.notification_sender.subscribe();

    // Set up notification sender in resources for OAuth callbacks
    let mut resources_clone = (*self.resources).clone(); // Safe: ServerResources clone for notification setup
    resources_clone.set_oauth_notification_sender(self.notification_sender.clone()); // Safe: Sender clone for notification
    let shared_resources = Arc::new(resources_clone);

    // Start stdio transport in background
    let resources_for_stdio = shared_resources.clone();
    let stdio_handle = tokio::spawn(async move {
        let stdio_transport = StdioTransport::new(resources_for_stdio);
        match stdio_transport.run(notification_receiver).await {
            Ok(()) => info!("stdio transport completed successfully"),
            Err(e) => warn!("stdio transport failed: {}", e),
        }
    });

    // Monitor stdio transport in background
    tokio::spawn(async move {
        match stdio_handle.await {
            Ok(()) => info!("stdio transport task completed"),
            Err(e) => warn!("stdio transport task failed: {}", e),
        }
    });
```

**Concurrency**: All transports run in separate `tokio::spawn` tasks, allowing simultaneous HTTP and stdio clients.

## Notification Broadcasting

The `broadcast::channel` distributes notifications to all transports:

```rust
let (notification_sender, _) = broadcast::channel(100);

// Subscribe for stdio transport
let notification_receiver = self.notification_sender.subscribe();

// Subscribe for SSE transport
let sse_notification_receiver = self.notification_sender.subscribe();

// Send notification (from OAuth callback)
notification_sender.send(OAuthCompletedNotification {
    provider: "strava",
    status: "success",
    user_id
})?;
```

**Rust Idiom**: `broadcast::channel` for pub-sub

The `broadcast::channel` allows multiple subscribers. When a notification is sent, all active subscribers receive it. This is perfect for distributing OAuth completion events to stdio and SSE transports simultaneously.

## Key Takeaways

1. **Transport abstraction**: MCP protocol is transport-agnostic. Same JSON-RPC messages work over stdio, HTTP, SSE, and WebSocket.

2. **stdio transport**: Line-delimited JSON-RPC over stdin/stdout for CLI tools and Claude Desktop integration.

3. **HTTP transport**: REST endpoints with Axum framework for web clients, with CORS and rate limiting support.

4. **SSE for notifications**: Server-Sent Events provide unidirectional server→client notifications for OAuth completion and progress updates.

5. **WebSocket transport**: Full-duplex bidirectional communication with JWT authentication, topic-based subscriptions, and real-time updates. Supports usage monitoring, system stats broadcasting every 30 seconds, and live data streaming.

6. **WebSocket message types**: Tagged enum with Authentication, Subscribe, UsageUpdate, SystemStats, Error, and Success variants for type-safe messaging.

7. **Connection management**: `WebSocketManager` tracks authenticated clients in `HashMap<Uuid, ClientConnection>` with `RwLock` for concurrent access.

8. **Broadcast notifications**: `tokio::sync::broadcast` distributes notifications to all active transports simultaneously.

9. **Concurrent transports**: All transports run in separate `tokio::spawn` tasks, allowing simultaneous stdio, HTTP, and WebSocket clients.

10. **Shared resources**: `Arc<ServerResources>` provides thread-safe access to database, auth manager, and other services across transports.

11. **Error isolation**: Each transport handles errors independently. stdio failure doesn't affect HTTP or WebSocket transports.

12. **Auto-recovery**: HTTP transport restarts on failure with exponential backoff (5s, 10s).

13. **Transport-agnostic processing**: `McpRequestProcessor` handles requests identically regardless of transport source.

14. **WebSocket splitting**: `ws.split()` pattern separates read/write halves for concurrent bidirectional communication without conflicts.

---

**Next Chapter**: [Chapter 12: MCP Tool Registry & Type-Safe Routing](./chapter-12-mcp-tool-registry.md) - Learn how the Pierre platform registers MCP tools, validates parameters with JSON Schema, and routes tool calls to handlers.
