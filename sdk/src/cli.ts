#!/usr/bin/env node

// ABOUTME: Command-line interface for Pierre MCP Client
// ABOUTME: Parses arguments, configures MCP client, and manages process lifecycle
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

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
        disableBrowser: options.noBrowser || false,
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