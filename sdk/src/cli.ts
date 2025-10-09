#!/usr/bin/env node

// ABOUTME: Command-line interface for Pierre-Claude Bridge
// ABOUTME: Parses arguments, configures bridge, and manages process lifecycle
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

/**
 * Pierre-Claude Bridge CLI
 *
 * MCP-compliant bridge connecting Claude Desktop (stdio) to Pierre Fitness MCP Server (Streamable HTTP + OAuth 2.0)
 */

import { Command } from 'commander';
import { PierreClaudeBridge } from './bridge';

const program = new Command();

program
  .name('pierre-claude-bridge')
  .description('MCP bridge connecting Claude Desktop to Pierre Fitness MCP Server')
  .version('1.0.0')
  .option('-s, --server <url>', 'Pierre MCP server URL', 'http://localhost:8080')
  .option('-t, --token <jwt>', 'JWT authentication token')
  .option('--oauth-client-id <id>', 'OAuth 2.0 client ID')
  .option('--oauth-client-secret <secret>', 'OAuth 2.0 client secret')
  .option('--user-email <email>', 'User email for automated login')
  .option('--user-password <password>', 'User password for automated login')
  .option('--callback-port <port>', 'OAuth callback server port', '35535')
  .action(async (options) => {
    try {
      const bridge = new PierreClaudeBridge({
        pierreServerUrl: options.server,
        jwtToken: options.token,
        oauthClientId: options.oauthClientId,
        oauthClientSecret: options.oauthClientSecret,
        userEmail: options.userEmail,
        userPassword: options.userPassword,
        callbackPort: parseInt(options.callbackPort, 10)
      });

      await bridge.start();
    } catch (error) {
      console.error('Bridge failed to start:', error);
      process.exit(1);
    }
  });

// Handle graceful shutdown
process.on('SIGINT', () => {
  console.error('\n🛑 Bridge shutting down...');
  process.exit(0);
});

process.on('SIGTERM', () => {
  console.error('\n🛑 Bridge shutting down...');
  process.exit(0);
});

program.parse();