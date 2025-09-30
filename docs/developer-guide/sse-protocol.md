# Server-Sent Events (SSE) Protocol

Pierre MCP Server implements Server-Sent Events for real-time notifications and MCP protocol streaming. SSE provides one-way server-to-client communication over HTTP.

## Architecture

Two types of SSE streams:

1. **OAuth Notification Stream** (src/sse/notifications.rs) - Real-time OAuth events for UI feedback
2. **MCP Protocol Stream** (src/sse/protocol.rs) - MCP JSON-RPC over SSE for streaming tool results

Both managed by unified `SseManager` (src/sse/manager.rs:31-54).

## Endpoints

### GET /notifications/sse

OAuth notification stream for real-time user notifications (src/sse/routes.rs:121-144).

**Query Parameters**:
- `user_id` - UUID of authenticated user (required)

**Response Headers**:
```
Content-Type: text/event-stream
Cache-Control: no-cache
Connection: keep-alive
```

**Example Request**:
```bash
curl -N "http://localhost:8081/notifications/sse?user_id=550e8400-e29b-41d4-a716-446655440000"
```

**Event Stream**:
```
event: connection
data: connected

event: notification
data: {"type":"oauth_completed","provider":"strava","success":true,"message":"Strava account connected successfully","user_id":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-15T10:30:00Z"}

event: notification
data: {"type":"token_refresh","provider":"strava","success":true,"message":"Access token refreshed","timestamp":"2024-01-15T11:00:00Z"}
```

### GET /mcp/sse

MCP protocol stream for JSON-RPC over SSE (src/sse/routes.rs:146-167).

**Headers**:
- `Authorization: Bearer JWT_TOKEN` - JWT authentication (optional)
- `Mcp-Session-Id: session_abc123` - Session identifier (optional, auto-generated if not provided)

**Query Parameters**:
- `session_id` - Session identifier (optional, can use header instead)

**Response Headers**:
```
Content-Type: text/event-stream
Cache-Control: no-cache
Connection: keep-alive
Mcp-Session-Id: session_abc123
```

**Example Request**:
```bash
curl -N -H "Authorization: Bearer $JWT_TOKEN" \
  "http://localhost:8081/mcp/sse?session_id=my-session-123"
```

**Event Stream**:
```
event: connected
data: MCP protocol stream ready

event: mcp
data: {"jsonrpc":"2.0","id":1,"result":{"tools":[{"name":"get_activities","description":"Retrieve user fitness activities"}]}}

event: mcp
data: {"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"{\"activities\":[...]}"}]}}
```

## Event Types

### OAuth Notification Events

**Connection Event** (initial):
```
event: connection
data: connected
```

**OAuth Completed Event** (src/mcp/schema.rs, sent via src/routes/auth.rs:500-522):
```json
{
  "type": "oauth_completed",
  "provider": "strava",
  "success": true,
  "message": "Strava account connected successfully! You can now use fitness tools.",
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

**Token Refresh Event**:
```json
{
  "type": "token_refresh",
  "provider": "strava",
  "success": true,
  "message": "Access token refreshed",
  "timestamp": "2024-01-15T11:00:00Z"
}
```

**Error Event**:
```json
{
  "type": "error",
  "provider": "strava",
  "success": false,
  "message": "OAuth authorization failed: access_denied",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

### MCP Protocol Events

**Connection Event** (initial):
```
event: connected
data: MCP protocol stream ready
```

**MCP Response Event**:
```
event: mcp
data: {"jsonrpc":"2.0","id":1,"result":{...}}
```

**MCP Error Event**:
```
event: mcp
data: {"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"Tool execution failed"}}
```

**Progress Event** (for long-running tools):
```
event: mcp
data: {"jsonrpc":"2.0","method":"notifications/progress","params":{"progressToken":"abc123","progress":50,"total":100}}
```

## Connection Management

### Stream Registration

**Notification Stream** (src/sse/manager.rs:57-81):
1. Client connects to `/notifications/sse?user_id=UUID`
2. Server creates `NotificationStream` for user
3. Returns `broadcast::Receiver` for receiving events
4. Registers connection metadata (created_at, last_activity)

**Protocol Stream** (src/sse/manager.rs:83-114):
1. Client connects to `/mcp/sse` with session_id
2. Server creates `McpProtocolStream` for session
3. Returns `broadcast::Receiver` for MCP messages
4. Registers connection metadata with session details

### Connection Lifecycle

**Connection** (src/sse/routes.rs:16-58):
```
Client              Server
  |                   |
  |--- GET /sse ---->|
  |                   | Create stream
  |<-- connected ----|
  |                   | Register receiver
  |<-- events --------|
  |                   |
```

**Disconnection** (src/sse/routes.rs:54, 109):
- Client closes connection
- Server detects closed channel
- Calls `unregister_*_stream()` to clean up
- Removes from active streams map

### Keep-Alive

SSE connections use keep-alive to prevent timeout (src/sse/routes.rs:57, 113):

```rust
warp::sse::keep_alive().stream(stream)
```

Sends periodic comments to keep connection alive:
```
: keep-alive

event: notification
data: {...}
```

## Client Implementation

### JavaScript/Browser

```javascript
// OAuth notification stream
const notificationSource = new EventSource(
  `http://localhost:8081/notifications/sse?user_id=${userId}`
);

notificationSource.addEventListener('connection', (event) => {
  console.log('Connected:', event.data);
});

notificationSource.addEventListener('notification', (event) => {
  const notification = JSON.parse(event.data);
  console.log('Notification:', notification);

  if (notification.type === 'oauth_completed' && notification.success) {
    // Update UI to show provider connected
    updateProviderStatus(notification.provider, 'connected');
  }
});

notificationSource.onerror = (error) => {
  console.error('SSE error:', error);
  notificationSource.close();
};

// MCP protocol stream
const mcpSource = new EventSource(
  'http://localhost:8081/mcp/sse?session_id=my-session',
  {
    headers: {
      'Authorization': `Bearer ${jwtToken}`
    }
  }
);

mcpSource.addEventListener('mcp', (event) => {
  const message = JSON.parse(event.data);
  console.log('MCP message:', message);

  // Handle JSON-RPC response
  if (message.id && message.result) {
    handleMcpResult(message.id, message.result);
  }
});
```

### Python

```python
import sseclient
import requests

# OAuth notifications
response = requests.get(
    f'http://localhost:8081/notifications/sse?user_id={user_id}',
    stream=True,
    headers={'Accept': 'text/event-stream'}
)

client = sseclient.SSEClient(response)
for event in client.events():
    if event.event == 'notification':
        notification = json.loads(event.data)
        print(f"Notification: {notification}")

        if notification['type'] == 'oauth_completed':
            print(f"OAuth success: {notification['provider']}")
```

### cURL (Testing)

```bash
# OAuth notifications (non-blocking, will run indefinitely)
curl -N "http://localhost:8081/notifications/sse?user_id=$USER_ID"

# MCP protocol
curl -N -H "Authorization: Bearer $JWT_TOKEN" \
  "http://localhost:8081/mcp/sse?session_id=test-session"
```

## OAuth Integration Flow

Complete OAuth flow with SSE notifications:

```bash
# Step 1: Open SSE notification stream (in background terminal)
curl -N "http://localhost:8081/notifications/sse?user_id=$USER_ID" &
SSE_PID=$!

# Step 2: Get OAuth authorization URL
AUTH_URL=$(curl -s "http://localhost:8081/api/oauth/auth/strava/$USER_ID" \
  -H "Authorization: Bearer $JWT_TOKEN" | jq -r '.authorization_url')

# Step 3: User visits AUTH_URL in browser and authorizes

# Step 4: After OAuth callback completes, SSE stream receives:
# event: notification
# data: {"type":"oauth_completed","provider":"strava","success":true,...}

# Step 5: Close SSE stream
kill $SSE_PID
```

**UI Integration**:
```javascript
// Start SSE stream before OAuth flow
const sse = new EventSource(`/notifications/sse?user_id=${userId}`);

sse.addEventListener('notification', (event) => {
  const data = JSON.parse(event.data);

  if (data.type === 'oauth_completed') {
    // Close OAuth popup window
    oauthWindow.close();

    // Show success message
    showToast(`${data.provider} connected successfully!`);

    // Refresh provider list
    refreshProviders();

    // Close SSE connection
    sse.close();
  }
});

// Open OAuth popup
const oauthWindow = window.open(authUrl, 'oauth', 'width=600,height=700');
```

## MCP Protocol Streaming

MCP tools can use SSE for streaming results:

**Tool Execution with Progress**:
```javascript
// Connect to MCP SSE stream
const mcpSource = new EventSource('/mcp/sse');

// Send tool execution request via regular HTTP POST
fetch('/mcp', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json'
  },
  body: JSON.stringify({
    jsonrpc: '2.0',
    method: 'tools/call',
    params: {
      name: 'analyze_activity',
      arguments: { activity_id: '12345' }
    },
    id: 1
  })
});

// Receive progress updates via SSE
mcpSource.addEventListener('mcp', (event) => {
  const msg = JSON.parse(event.data);

  if (msg.method === 'notifications/progress') {
    // Update progress bar
    const percent = (msg.params.progress / msg.params.total) * 100;
    updateProgressBar(percent);
  }

  if (msg.id === 1 && msg.result) {
    // Final result
    displayAnalysis(msg.result);
    mcpSource.close();
  }
});
```

## Connection Metadata

Server tracks connection metadata (src/sse/manager.rs:23-29):

```rust
pub struct ConnectionMetadata {
    pub connection_type: ConnectionType,  // Notification or Protocol
    pub created_at: DateTime<Utc>,        // Connection timestamp
    pub last_activity: DateTime<Utc>,     // Last message sent
}
```

**Query Active Connections** (internal API):
```rust
// Get all active notification streams
let streams = sse_manager.notification_streams.read().await;
let active_users: Vec<Uuid> = streams.keys().cloned().collect();

// Get all active protocol sessions
let protocol_streams = sse_manager.protocol_streams.read().await;
let active_sessions: Vec<String> = protocol_streams.keys().cloned().collect();
```

## Error Handling

### Client-Side Reconnection

SSE connections may drop due to network issues. Implement reconnection:

```javascript
function connectSSE(userId) {
  const source = new EventSource(`/notifications/sse?user_id=${userId}`);

  source.onerror = (error) => {
    console.error('SSE error:', error);
    source.close();

    // Exponential backoff reconnection
    const delay = Math.min(30000, reconnectDelay * 2);
    console.log(`Reconnecting in ${delay}ms...`);

    setTimeout(() => {
      reconnectDelay = delay;
      connectSSE(userId);
    }, delay);
  };

  source.addEventListener('connection', () => {
    reconnectDelay = 1000; // Reset backoff on successful connection
  });

  return source;
}

let reconnectDelay = 1000;
const sse = connectSSE(userId);
```

### Server-Side Error Handling

**Invalid User ID** (src/sse/routes.rs:195-216):
```bash
curl "http://localhost:8081/notifications/sse?user_id=invalid"
# Response: 400 Bad Request
# Body: "Invalid or missing user_id parameter"
```

**Missing Authorization**:
```bash
curl "http://localhost:8081/mcp/sse"
# Response: Works (authorization optional), but may have limited access
```

**Channel Lag** (src/sse/routes.rs:43-45, 98-100):

If client can't keep up with events, broadcast channel lags:
```rust
Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
    tracing::warn!("SSE receiver lagged by {} messages", n);
    // Continue, messages lost
}
```

Client should implement backpressure or request missed data via HTTP API.

## Performance Considerations

### Connection Limits

- Maximum concurrent SSE connections: Limited by file descriptors (typically 1024 default)
- Use `ulimit -n` to increase: `ulimit -n 65536`
- Consider load balancing for >1000 concurrent connections

### Memory Usage

Each SSE connection consumes:
- ~8KB for TCP buffers
- ~4KB for broadcast channel
- ~1KB for connection metadata

Estimate: 1000 connections ≈ 13MB

### Broadcast Channel Configuration

Channel capacity (src/sse/notifications.rs, src/sse/protocol.rs):
```rust
broadcast::channel(1000)  // Buffer 1000 messages
```

If messages arrive faster than clients consume, channel lags and drops old messages.

### Keep-Alive Interval

Default keep-alive: 15 seconds (warp SSE default)

Adjust for slow networks:
```rust
warp::sse::keep_alive()
    .interval(Duration::from_secs(30))
    .stream(stream)
```

## Security

### Authentication

**OAuth Notification Stream**:
- Requires valid `user_id` UUID
- No authentication header needed (user_id acts as token)
- Validate user_id exists in database

**MCP Protocol Stream**:
- Accepts `Authorization: Bearer JWT_TOKEN` header
- Falls back to public access if no auth
- JWT validated per standard authentication flow

### Authorization

Streams only send data for authenticated user:
- Notification stream: Only events for specified user_id
- Protocol stream: MCP tools respect JWT user context

### Rate Limiting

Consider rate limiting SSE connections:

```nginx
# In nginx.conf
limit_conn_zone $binary_remote_addr zone=sse_limit:10m;

location /sse {
    limit_conn sse_limit 5;  # Max 5 SSE connections per IP
}
```

## Debugging

### Enable Debug Logging

```bash
RUST_LOG=pierre_mcp_server::sse=debug cargo run
```

**Log Output**:
```
DEBUG pierre_mcp_server::sse: New notification SSE connection for user: 550e8400-e29b-41d4-a716-446655440000
DEBUG pierre_mcp_server::sse: Registered notification stream for user: 550e8400-e29b-41d4-a716-446655440000
DEBUG pierre_mcp_server::sse: Sending notification to user: 550e8400-e29b-41d4-a716-446655440000
DEBUG pierre_mcp_server::sse: SSE channel closed for user: 550e8400-e29b-41d4-a716-446655440000
```

### Test with websocat

```bash
# Install websocat
brew install websocat

# Connect to SSE endpoint (websocat treats SSE as regular HTTP)
websocat -v --text "http://localhost:8081/notifications/sse?user_id=$USER_ID"
```

### Monitor Active Connections

```bash
# Count SSE connections
netstat -an | grep :8081 | grep ESTABLISHED | wc -l

# Show SSE connection details
lsof -i :8081 | grep ESTABLISHED
```

## Comparison with WebSocket

| Feature | SSE | WebSocket |
|---------|-----|-----------|
| Direction | Server → Client | Bidirectional |
| Protocol | HTTP | ws:// or wss:// |
| Reconnection | Automatic | Manual |
| Complexity | Simple | Complex |
| Firewall | Works everywhere | May be blocked |
| Use Case | Notifications, streaming | Real-time chat, gaming |

Pierre uses SSE because:
1. One-way server-to-client is sufficient for notifications
2. Automatic reconnection built into EventSource API
3. Works through corporate firewalls/proxies
4. Simpler implementation than WebSocket

## Related Documentation

- [MCP Protocol](04-mcp-protocol.md) - MCP JSON-RPC over HTTP and SSE
- [Authentication](06-authentication.md) - JWT authentication for SSE streams
- [API Reference](14-api-reference.md) - REST API endpoints
- [OAuth 2.0 Server](oauth2-authorization-server.md) - OAuth integration with SSE notifications
