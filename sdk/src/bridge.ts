// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Backward-compatible re-exports for Pierre MCP Client bridge components
// ABOUTME: Maintains existing import paths while code is organized into focused modules

/**
 * Pierre MCP Client Bridge - Re-export Module
 *
 * This file provides backward compatibility for existing imports.
 * The implementation has been decomposed into focused modules:
 * - oauth-session-manager.ts: OAuth 2.0 flows, token storage, callback server
 * - batch-guard-transport.ts: Batch request handling for MCP protocol
 * - mcp-bridge.ts: Main MCP bridge connecting stdio to HTTP
 */

// Re-export main client and config from mcp-bridge
export { PierreMcpClient, BridgeConfig } from './mcp-bridge.js';

// Re-export OAuth components for advanced usage
export {
  PierreOAuthClientProvider,
  OAuthSessionConfig,
  StoredTokens,
} from './oauth-session-manager.js';

// Re-export batch guard utilities for transport customization
export {
  installBatchGuard,
  createBatchGuardMessageHandler,
} from './batch-guard-transport.js';
