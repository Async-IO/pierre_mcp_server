<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 29: TypeScript SDK & CLI Usage

This appendix explains how to use the Pierre TypeScript SDK and command-line interface to connect MCP hosts to the Pierre server. You'll learn about the main SDK entry points, CLI flags, and environment-driven configuration.

## What You'll Learn

- SDK entrypoint and exports (`sdk/src/index.ts`)
- Bridge client configuration (`sdk/src/bridge.ts`, see Chapter 13)
- CLI wrapper and argument parsing (`sdk/src/cli.ts`)
- Environment variables for headless/CI usage
- Graceful shutdown and process lifecycle

## SDK Entrypoint

The SDK exposes the MCP bridge client and all generated tool types from a single module.

**Source**: sdk/src/index.ts:1-20
```ts
// ABOUTME: Main entry point for Pierre MCP Client TypeScript SDK
// ABOUTME: Re-exports MCP client and configuration for programmatic integration

/**
 * Pierre MCP Client SDK
 */

export { PierreMcpClient, BridgeConfig } from './bridge';

/**
 * Export all TypeScript type definitions for Pierre MCP tools
 *
 * These types are auto-generated from server tool schemas.
 * To regenerate: npm run generate-types
 */
export * from './types';
```

**Usage (programmatic)**:
```ts
import { PierreMcpClient, BridgeConfig } from 'pierre-mcp-client-sdk';

const config: BridgeConfig = {
  pierreServerUrl: 'https://api.pierre.ai',
  jwtToken: process.env.PIERRE_JWT_TOKEN,
};

const client = new PierreMcpClient(config);
await client.start();
// ... interact via MCP stdio protocol ...
```

Programmatic usage is mostly relevant if you are embedding Pierre into a larger Node-based MCP host; for most users, the CLI wrapper is the primary entrypoint.

## CLI Overview

The CLI wraps `PierreMcpClient` and exposes it as a standard MCP client binary.

**Source**: sdk/src/cli.ts:1-29
```ts
#!/usr/bin/env node

// ABOUTME: Command-line interface for Pierre MCP Client
// ABOUTME: Parses arguments, configures MCP client, and manages process lifecycle

/**
 * Pierre MCP Client CLI
 *
 * MCP-compliant client connecting MCP hosts to Pierre Fitness MCP Server (HTTP + OAuth 2.0)
 */

import { Command } from 'commander';
import { PierreMcpClient } from './bridge';

// DEBUG: Log environment at startup (stderr only - stdout is for MCP protocol)
console.error('[DEBUG] Bridge CLI starting...');
console.error('[DEBUG] CI environment variables:');
console.error(`  process.env.CI = ${process.env.CI}`);
console.error(`  process.env.GITHUB_ACTIONS = ${process.env.GITHUB_ACTIONS}`);
console.error(`  process.env.NODE_ENV = ${process.env.NODE_ENV}`);
console.error('[DEBUG] Auth environment variables:');
console.error(`  PIERRE_JWT_TOKEN = ${process.env.PIERRE_JWT_TOKEN ? '[SET]' : '[NOT SET]'}`);
console.error(`  PIERRE_SERVER_URL = ${process.env.PIERRE_SERVER_URL || '[NOT SET]'}`);

const program = new Command();
```

**Design details**:
- All debug logs go to **stderr** so stdout remains clean JSON-RPC for MCP.
- `commander` handles argument parsing, default values, and `--help` output.
- The CLI is intended to be invoked by an MCP host (e.g., Claude Desktop, VS Code, etc.).

## CLI Options & Environment Variables

The CLI exposes a set of options with sensible environment fallbacks.

**Source**: sdk/src/cli.ts:31-63
```ts
program
  .name('pierre-mcp-client')
  .description('MCP client connecting to Pierre Fitness MCP Server')
  .version('1.0.0')
  .option('-s, --server <url>', 'Pierre MCP server URL', process.env.PIERRE_SERVER_URL || 'http://localhost:8080')
  .option('-t, --token <jwt>', 'JWT authentication token', process.env.PIERRE_JWT_TOKEN)
  .option('--oauth-client-id <id>', 'OAuth 2.0 client ID', process.env.PIERRE_OAUTH_CLIENT_ID)
  .option('--oauth-client-secret <secret>', 'OAuth 2.0 client secret', process.env.PIERRE_OAUTH_CLIENT_SECRET)
  .option('--user-email <email>', 'User email for automated login', process.env.PIERRE_USER_EMAIL)
  .option('--user-password <password>', 'User password for automated login', process.env.PIERRE_USER_PASSWORD)
  .option('--callback-port <port>', 'OAuth callback server port', process.env.PIERRE_CALLBACK_PORT || '35535')
  .option('--no-browser', 'Disable automatic browser opening for OAuth (testing mode)')
  .option('--token-validation-timeout <ms>', 'Token validation timeout in milliseconds (default: 3000)', process.env.PIERRE_TOKEN_VALIDATION_TIMEOUT_MS || '3000')
  .option('--proactive-connection-timeout <ms>', 'Proactive connection timeout in milliseconds (default: 5000)', process.env.PIERRE_PROACTIVE_CONNECTION_TIMEOUT_MS || '5000')
  .option('--proactive-tools-list-timeout <ms>', 'Proactive tools list timeout in milliseconds (default: 3000)', process.env.PIERRE_PROACTIVE_TOOLS_LIST_TIMEOUT_MS || '3000')
  .option('--tool-call-connection-timeout <ms>', 'Tool-triggered connection timeout in milliseconds (default: 10000)', process.env.PIERRE_TOOL_CALL_CONNECTION_TIMEOUT_MS || '10000')
```

**Common environment variables**:
- `PIERRE_SERVER_URL`: Base URL for the Pierre server (`https://api.pierre.ai` in production).
- `PIERRE_JWT_TOKEN`: Pre-issued JWT for authenticating the bridge (see Chapter 6 / 15).
- `PIERRE_OAUTH_CLIENT_ID` / `PIERRE_OAUTH_CLIENT_SECRET`: OAuth client for the bridge itself.
- `PIERRE_USER_EMAIL` / `PIERRE_USER_PASSWORD`: For automated login flows (CI/testing).
- `PIERRE_CALLBACK_PORT`: Port for the local OAuth callback HTTP server.

**Example CLI invocation**:
```bash
# Minimal: rely on environment variables
export PIERRE_SERVER_URL="https://api.pierre.ai"
export PIERRE_JWT_TOKEN="<your-jwt>"

pierre-mcp-client
```

```bash
# Explicit flags (override env)
pierre-mcp-client \
  --server https://api.pierre.ai \
  --token "$PIERRE_JWT_TOKEN" \
  --oauth-client-id "$PIERRE_OAUTH_CLIENT_ID" \
  --oauth-client-secret "$PIERRE_OAUTH_CLIENT_SECRET" \
  --no-browser
```

## Bridge Configuration Wiring

The CLI simply maps parsed options into a `BridgeConfig` and starts the bridge.

**Source**: sdk/src/cli.ts:63-92
```ts
  .action(async (options) => {
    try {
      const bridge = new PierreMcpClient({
        pierreServerUrl: options.server,
        jwtToken: options.token,
        oauthClientId: options.oauthClientId,
        oauthClientSecret: options.oauthClientSecret,
        userEmail: options.userEmail,
        userPassword: options.userPassword,
        callbackPort: parseInt(options.callbackPort, 10),
        disableBrowser: !options.browser,
        tokenValidationTimeoutMs: parseInt(options.tokenValidationTimeout, 10),
        proactiveConnectionTimeoutMs: parseInt(options.proactiveConnectionTimeout, 10),
        proactiveToolsListTimeoutMs: parseInt(options.proactiveToolsListTimeout, 10),
        toolCallConnectionTimeoutMs: parseInt(options.toolCallConnectionTimeout, 10)
      });

      await bridge.start();

      // Store bridge instance for cleanup on shutdown
      (global as any).__bridge = bridge;
    } catch (error) {
      console.error('Bridge failed to start:', error);
      process.exit(1);
    }
  });
```

See **Chapter 13** for a deeper dive into `PierreMcpClient` and the `BridgeConfig` fields (OAuth flows, secure token storage, proactive connections, etc.). The CLI is a thin wrapper on that logic.

## Graceful Shutdown

The CLI handles termination signals and calls `bridge.stop()` to clean up resources.

**Source**: sdk/src/cli.ts:94-134
```ts
// Handle graceful shutdown
let shutdownInProgress = false;

const handleShutdown = (signal: string) => {
  if (shutdownInProgress) {
    console.error('\n⚠️  Forcing immediate exit...');
    process.exit(1);
  }

  shutdownInProgress = true;
  console.error(`\n🛑 Bridge shutting down (${signal})...`);

  const bridge = (global as any).__bridge;
  if (bridge) {
    bridge.stop()
      .then(() => {
        console.error('✅ Bridge stopped cleanly');
        process.exit(0);
      })
      .catch((error: any) => {
        console.error('Error during shutdown:', error);
        process.exit(1);
      });
  } else {
    process.exit(0);
  }
};

process.on('SIGINT', () => handleShutdown('SIGINT'));
process.on('SIGTERM', () => handleShutdown('SIGTERM'));

program.parse();
```

**Why this matters**:
- MCP hosts often manage client processes; clean shutdown avoids leaving stuck TCP connections or zombie OAuth callback servers.
- Double-pressing Ctrl+C forces immediate exit if shutdown is already in progress.

## Typical MCP Host Configuration

Most MCP hosts require a JSON manifest pointing to the CLI binary, for example:

```json
{
  "name": "pierre-fitness",
  "command": "pierre-mcp-client",
  "args": [],
  "env": {
    "PIERRE_SERVER_URL": "https://api.pierre.ai",
    "PIERRE_JWT_TOKEN": "${PIERRE_JWT_TOKEN}"
  }
}
```

The host spawns `pierre-mcp-client`, speaks JSON-RPC 2.0 over stdio, and the bridge translates MCP calls into HTTP/OAuth interactions with the Pierre server.

## Key Takeaways

1. **SDK entrypoint**: `sdk/src/index.ts` re-exports `PierreMcpClient`, `BridgeConfig`, and all tool types for programmatic use.
2. **CLI wrapper**: `pierre-mcp-client` is a thin layer over `PierreMcpClient` that wires CLI options into `BridgeConfig`.
3. **Env-driven config**: Most options have environment fallbacks, enabling headless and CI-friendly setups.
4. **Stderr vs stdout**: Debug logs go to stderr so stdout remains pure MCP JSON-RPC.
5. **Graceful shutdown**: Signal handlers call `bridge.stop()` to close connections and clean up resources.
6. **Host integration**: MCP hosts simply execute the CLI and communicate over stdio; no extra glue code is required.
