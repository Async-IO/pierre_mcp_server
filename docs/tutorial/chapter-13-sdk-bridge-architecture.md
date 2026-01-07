<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 13: SDK Bridge Architecture

This chapter explores how the TypeScript SDK bridges MCP hosts (like Claude Desktop) to the Pierre server, translating between stdio (MCP standard) and HTTP (Pierre's transport).

## What You'll Learn

- SDK bridge architecture pattern
- stdio to HTTP translation
- OAuth 2.0 client implementation
- Token persistence with OS keychain
- MCP host integration (Claude Desktop)
- Bidirectional message routing
- Automatic OAuth flow handling

## SDK Bridge Pattern

The SDK acts as a transparent bridge between MCP hosts and Pierre server:

```
┌──────────────┐         ┌──────────────┐         ┌──────────────┐
│ Claude       │ stdio   │   SDK        │  HTTP   │   Pierre     │
│ Desktop      │◄───────►│   Bridge     │◄───────►│   Server     │
│ (MCP Host)   │         │  (TypeScript)│         │   (Rust)     │
└──────────────┘         └──────────────┘         └──────────────┘
     │                         │                         │
     │  tools/list             │ GET /mcp/tools         │
     ├────────────────────────►├────────────────────────►│
     │                         │                         │
     │  tools (JSON-RPC)       │ HTTP 200                │
     │◄────────────────────────┼◄────────────────────────┤
```

**Source**: sdk/src/bridge.ts:70-84
```typescript
export interface BridgeConfig {
  pierreServerUrl: string;
  jwtToken?: string;
  apiKey?: string;
  oauthClientId?: string;
  oauthClientSecret?: string;
  userEmail?: string;
  userPassword?: string;
  callbackPort?: number;
  disableBrowser?: boolean;
  tokenValidationTimeoutMs?: number;
  proactiveConnectionTimeoutMs?: number;
  proactiveToolsListTimeoutMs?: number;
  toolCallConnectionTimeoutMs?: number;
}
```

**Configuration**:
- `pierreServerUrl`: Pierre HTTP endpoint (e.g., `http://localhost:8081`)
- `jwtToken`/`apiKey`: Pre-existing authentication
- `oauthClientId`/`oauthClientSecret`: OAuth app credentials
- `userEmail`/`userPassword`: Login credentials
- `callbackPort`: OAuth callback listener port

## OAuth Client Provider

The SDK implements OAuth 2.0 client for Pierre authentication:

**Source**: sdk/src/bridge.ts:113-150
```typescript
class PierreOAuthClientProvider implements OAuthClientProvider {
  private serverUrl: string;
  private config: BridgeConfig;
  private clientInfo: OAuthClientInformationFull | undefined = undefined;
  private savedTokens: OAuthTokens | undefined = undefined;
  private codeVerifierValue: string | undefined = undefined;
  private stateValue: string | undefined = undefined;
  private callbackServer: any = undefined;
  private authorizationPending: Promise<any> | undefined = undefined;
  private callbackPort: number = 0;
  private callbackSessionToken: string | undefined = undefined;

  // Secure token storage using OS keychain
  private secureStorage: SecureTokenStorage | undefined = undefined;
  private allStoredTokens: StoredTokens = {};

  // Client-side client info storage (client info is not sensitive, can stay in file)
  private clientInfoPath: string;

  constructor(serverUrl: string, config: BridgeConfig) {
    this.serverUrl = serverUrl;
    this.config = config;

    // Initialize client info storage path
    const os = require('os');
    const path = require('path');
    this.clientInfoPath = path.join(os.homedir(), '.pierre-mcp-client-info.json');

    // NOTE: Secure storage initialization is async, so it's deferred to start()
    // to avoid race conditions with constructor completion
    // See initializePierreConnection() for the actual initialization

    // Load client info from storage (synchronous, non-sensitive)
    this.loadClientInfo();

    this.log(`OAuth client provider created for server: ${serverUrl}`);
    this.log(`Using OS keychain for secure token storage (will initialize on start)`);
    this.log(`Client info storage path: ${this.clientInfoPath}`);
  }
```

**OAuth flow**:
1. **Discovery**: Fetch `/.well-known/oauth-authorization-server` for endpoints
2. **Registration**: Register OAuth client with Pierre (RFC 7591)
3. **Authorization**: Open browser to `/oauth/authorize`
4. **Callback**: Listen for OAuth callback on localhost
5. **Token exchange**: POST to `/oauth/token` with authorization code
6. **Token storage**: Save to OS keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service)

## Secure Token Storage

The SDK stores OAuth tokens in OS-native secure storage:

**Source**: sdk/src/bridge.ts:59-68
```typescript
interface StoredTokens {
  pierre?: OAuthTokens & { saved_at?: number };
  providers?: Record<string, {
    access_token: string;
    refresh_token?: string;
    expires_at?: number;
    token_type?: string;
    scope?: string;
  }>;
}
```

**Storage locations**:
- **macOS**: Keychain (`security` command-line tool)
- **Windows**: Credential Manager (Windows Credential Store API)
- **Linux**: Secret Service API (libsecret)

**Security**: Tokens never stored in plaintext files. OS-native encryption protects credentials.

## MCP Host Integration

The SDK integrates with MCP hosts via stdio transport:

**Source**: sdk/src/bridge.ts:13-16
```typescript
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { StreamableHTTPClientTransport } from '@modelcontextprotocol/sdk/client/streamableHttp.js';
```

**Components**:
- `Server`: MCP server exposed to host via stdio
- `StdioServerTransport`: stdio transport for MCP host communication
- `Client`: MCP client connecting to Pierre
- `StreamableHTTPClientTransport`: HTTP transport for Pierre connection

## Message Routing

The SDK routes messages bidirectionally:

```
Claude Desktop → Server (stdio) → Client (HTTP) → Pierre
Claude Desktop ← Server (stdio) ← Client (HTTP) ← Pierre
```

**Request flow**:
1. MCP host sends JSON-RPC to SDK's stdio (e.g., `tools/call`)
2. SDK's Server receives via `StdioServerTransport`
3. SDK's Client forwards to Pierre via `StreamableHTTPClientTransport`
4. Pierre processes and returns JSON-RPC response
5. SDK's Client receives HTTP response
6. SDK's Server sends JSON-RPC to MCP host via stdio

## Automatic OAuth Handling

The SDK handles OAuth flows transparently:

**Source**: sdk/src/bridge.ts:48-57
```typescript
// Define custom notification schema for Pierre's OAuth completion notifications
const OAuthCompletedNotificationSchema = z.object({
  method: z.literal('notifications/oauth_completed'),
  params: z.object({
    provider: z.string(),
    success: z.boolean(),
    message: z.string(),
    user_id: z.string().optional()
  }).optional()
});
```

**OAuth notifications**:
- Pierre sends `notifications/oauth_completed` via SSE
- SDK receives notification and updates stored tokens
- Future requests use refreshed tokens automatically

## Key Takeaways

1. **Bridge pattern**: SDK translates stdio (MCP standard) ↔ HTTP (Pierre transport).

2. **OAuth client**: Full OAuth 2.0 implementation with discovery, registration, and token exchange.

3. **Secure storage**: OS-native keychain for token storage (never plaintext files).

4. **Transparent integration**: MCP hosts (Claude Desktop) connect via stdio without knowing about HTTP backend.

5. **Bidirectional routing**: Messages flow both directions through SDK bridge.

6. **Automatic token refresh**: SDK handles token expiration and refresh transparently.

7. **MCP SDK**: Built on official `@modelcontextprotocol/sdk` for standard compliance.

---

**Next Chapter**: [Chapter 14: Type Generation & Tools-to-Types](./chapter-14-type-generation.md) - Learn how Pierre generates TypeScript types from Rust tool definitions for type-safe SDK development.
