# Pierre MCP Server SDK - Technical Documentation

## Overview

The Pierre MCP Server SDK provides a TypeScript bridge that enables Claude Desktop to communicate with the Pierre MCP Server using the Model Context Protocol (MCP). This bridge handles OAuth 2.0 authentication, token management, and protocol translation between MCP and the Pierre server's HTTP API.

## Architecture

### High-Level Flow

```
Claude Desktop
    “ (MCP Protocol via stdio)
Bridge (sdk/dist/cli.js)
    “ (HTTP/SSE)
Pierre MCP Server (port 8081)
    “ (OAuth)
External Providers (Strava, Fitbit, etc.)
```

### Components

1. **CLI Entry Point** (`src/cli.ts`)
   - Launches the bridge process
   - Parses command-line arguments
   - Configures server connection

2. **Bridge** (`src/bridge.ts`)
   - **PierreMcpBridge**: Main bridge class that translates between MCP and HTTP
   - **PierreOAuthClientProvider**: Handles OAuth 2.0 client flow and token management

## Token Storage

### Location

Tokens are stored in a JSON file on the local filesystem:
```
~/.pierre-claude-tokens.json
```

### Token File Structure

```json
{
  "pierre": {
    "access_token": "eyJ0eXA...",
    "token_type": "Bearer",
    "expires_in": 3600,
    "refresh_token": null,
    "scope": "read:fitness write:fitness",
    "saved_at": 1759339326
  },
  "providers": {
    "strava": {
      "access_token": "...",
      "refresh_token": "...",
      "expires_at": 1759425726000,
      "token_type": "Bearer",
      "scope": "activity:read_all"
    }
  }
}
```

### Token Types

1. **Pierre Token** (`pierre` key)
   - JWT token for authenticating with the Pierre MCP Server
   - Contains user ID in the `sub` claim
   - Stored with `saved_at` timestamp for expiry calculation

2. **Provider Tokens** (`providers.*` keys)
   - OAuth tokens for external services (Strava, Fitbit, etc.)
   - Stored with `expires_at` timestamp
   - Used by the server to make API calls on behalf of the user

## Token Validation Logic

### On Bridge Startup

When the bridge loads tokens from `~/.pierre-claude-tokens.json`:

1. **Expiry Check**
   ```typescript
   const now = Math.floor(Date.now() / 1000);
   const expiresAt = (storedTokens.saved_at || 0) + (storedTokens.expires_in || 0);
   if (now >= expiresAt) {
     // Token expired - clear from storage
   }
   ```

2. **Server Validation** (NEW - prevents stale user IDs)
   ```typescript
   const isValid = await validateToken(storedTokens.access_token);
   if (!isValid) {
     // Token validation failed - user may no longer exist
     // Clear cached token to force re-authentication
   }
   ```

The validation makes a request to `/oauth/status` to verify:
- Token signature is valid
- User still exists in the database
- User account is active

This prevents the bridge from using a cached JWT token containing a user ID that no longer exists (e.g., after database cleanup).

### Token Lifecycle

```
1. User opens Claude Desktop
2. Bridge checks ~/.pierre-claude-tokens.json
3. If token exists:
   a. Check expiry time
   b. Validate with server (/oauth/status)
   c. If valid: Use cached token
   d. If invalid: Delete cached token, force new OAuth flow
4. If no token or invalid:
   a. Initiate OAuth 2.0 authorization code flow
   b. Open browser for user authentication
   c. Receive authorization code via callback
   d. Exchange code for access token
   e. Save token to ~/.pierre-claude-tokens.json
```

## OAuth 2.0 Flow

### Authorization Code Flow with PKCE

1. **Client Registration**
   ```
   POST /oauth2/register
   ’ Returns: client_id, client_secret
   ```

2. **Authorization Request**
   ```
   GET /oauth2/authorize?
     client_id=...
     &redirect_uri=http://localhost:35536/oauth/callback
     &response_type=code
     &state=...
     &code_challenge=...
     &code_challenge_method=S256

   ’ User authenticates in browser
   ’ Redirects to: http://localhost:35536/oauth/callback?code=...&state=...
   ```

3. **Token Exchange**
   ```
   POST /oauth2/token
   {
     "grant_type": "authorization_code",
     "code": "...",
     "redirect_uri": "...",
     "client_id": "...",
     "client_secret": "...",
     "code_verifier": "..."
   }

   ’ Returns: access_token, refresh_token, expires_in
   ```

### Callback Server

The bridge runs a local HTTP server on a random port (default: 35536) to receive the OAuth callback:

```typescript
// Bridge starts local server
const callbackServer = express();
callbackServer.get('/oauth/callback', handleCallback);
callbackServer.listen(callbackPort);

// Browser redirects here after authentication
// http://localhost:35536/oauth/callback?code=abc123&state=xyz789
```

## Provider Authentication Flow

When a tool requires provider authentication (e.g., Strava):

1. **Tool Call** (e.g., `get_activities`)
   ```
   MCP tools/call ’ Bridge ’ POST /mcp/tools/call
   ```

2. **Server Responds** with authProvider if not authenticated:
   ```json
   {
     "result": {
       "content": [],
       "authProvider": {
         "url": "http://localhost:8081/api/oauth/auth/strava/{user_id}"
       }
     }
   }
   ```

3. **Bridge Opens Browser** to the authProvider URL
   - Server redirects to Strava OAuth page
   - User authorizes
   - Strava redirects to server callback
   - Server saves provider token
   - Server shows success page

4. **Bridge Retries Tool Call**
   - Now that provider is authenticated, tool call succeeds

## MCP Protocol Mapping

### Tools Discovery

```
Claude Desktop: tools/list
    “
Bridge: GET /mcp/tools/list
    “
Server: Returns 35 tools (Strava, Fitbit, Configuration)
    “
Bridge: Converts to MCP format
    “
Claude Desktop: Displays available tools
```

### Tool Execution

```
Claude Desktop: tools/call { name: "get_activities", arguments: {...} }
    “
Bridge: POST /mcp/tools/call { tool_name: "get_activities", tool_input: {...} }
    “
Server: Executes tool
    “
Server: Returns { result: { content: [...], structuredContent: {...} } }
    “
Bridge: Converts to MCP ToolResponse
    “
Claude Desktop: Displays formatted result
```

## Response Format Translation

The bridge converts Pierre server responses to MCP ToolResponse format:

**Server Response:**
```json
{
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Retrieved 10 activities\n\n1. Morning Run..."
      }
    ],
    "structuredContent": {
      "activities": [...]
    }
  }
}
```

**MCP ToolResponse:**
```json
{
  "content": [
    {
      "type": "text",
      "text": "Retrieved 10 activities\n\n1. Morning Run..."
    }
  ],
  "isError": false
}
```

## Server-Sent Events (SSE)

For real-time OAuth notifications:

```
Bridge connects: GET /mcp/sse
    “
Server: Opens SSE stream
    “
User completes OAuth in browser
    “
Server: Sends SSE event { type: "oauth_success", provider: "strava" }
    “
Bridge: Receives notification
    “
Claude Desktop: Shows success message
```

## Configuration

### Bridge Config (`BridgeConfig`)

```typescript
interface BridgeConfig {
  pierreServerUrl: string;       // Default: http://localhost:8081
  verbose: boolean;              // Debug logging
  pollInterval: number;          // SSE reconnect interval (ms)
}
```

### Command-Line Usage

```bash
node sdk/dist/cli.js --server http://localhost:8081 --verbose
```

### Claude Desktop Config

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "node",
      "args": [
        "/path/to/sdk/dist/cli.js",
        "--server",
        "http://localhost:8081",
        "--verbose"
      ],
      "env": {}
    }
  }
}
```

## Security Considerations

### Token Storage

- **Local file storage**: `~/.pierre-claude-tokens.json` is stored with user-only read/write permissions
- **No encryption**: Tokens are stored in plaintext (acceptable for local development)
- **File permissions**: Should be `600` (rw-------) on Unix systems

### Token Validation

- **Server-side validation**: Tokens are validated against the server on each bridge startup
- **Signature verification**: Server validates JWT signature using its secret
- **User existence**: Server checks that the user ID in the token still exists
- **Expiry enforcement**: Both client and server enforce token expiry

### OAuth Flow Security

- **PKCE**: Uses Proof Key for Code Exchange to prevent authorization code interception
- **State parameter**: Validates OAuth callback to prevent CSRF attacks
- **Local callback server**: Runs only during OAuth flow, closed after token exchange

## Troubleshooting

### Token Cache Issues

**Problem**: Bridge using old user ID after database cleanup

**Solution**: Delete `~/.pierre-claude-tokens.json` to force fresh authentication

**Prevention**: Bridge now validates tokens with server on startup (implemented in this commit)

### Authentication Loops

**Problem**: Browser keeps opening for authentication

**Check**:
1. Server is running: `curl http://localhost:8081/health`
2. OAuth endpoints accessible: `curl http://localhost:8081/oauth/status`
3. Token file exists: `ls -la ~/.pierre-claude-tokens.json`
4. Token not expired: Check `saved_at + expires_in > now`

### SSE Connection Issues

**Problem**: OAuth notifications not received

**Check**:
1. SSE endpoint responding: `curl http://localhost:8081/mcp/sse`
2. Bridge connected: Look for `[SSE] Connected to server` in logs
3. Network issues: Check firewall/proxy settings

## Development

### Building

```bash
cd sdk
npm install
npm run build
```

### Testing

```bash
# Run bridge in verbose mode
node dist/cli.js --server http://localhost:8081 --verbose

# In another terminal, test with MCP client
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | node dist/cli.js
```

### Debugging

Enable verbose logging to see all HTTP requests and MCP protocol messages:

```bash
node dist/cli.js --server http://localhost:8081 --verbose 2>&1 | tee bridge.log
```

## Architecture Decisions

### Why Client-Side Token Storage?

- **Claude Desktop limitation**: MCP servers are stateless processes spawned per session
- **Session continuity**: Tokens must persist across bridge restarts
- **User experience**: Avoid re-authentication on every Claude Desktop restart

### Why Token Validation on Startup?

- **Database cleanup**: Prevents using cached tokens for deleted users
- **Account changes**: Detects suspended/deactivated accounts
- **Security**: Ensures tokens haven't been revoked server-side

### Why Local Callback Server?

- **OAuth redirect requirement**: OAuth providers need an HTTP callback URL
- **MCP limitation**: stdio-based MCP can't receive HTTP callbacks directly
- **Dynamic port**: Avoids conflicts if multiple bridges run simultaneously
