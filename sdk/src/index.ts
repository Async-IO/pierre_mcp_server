// ABOUTME: Main entry point for Pierre MCP Client TypeScript SDK
// ABOUTME: Re-exports MCP client and configuration for programmatic integration
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

/**
 * Pierre MCP Client SDK
 *
 * Export the main MCP client implementation for programmatic use
 */

export { PierreMcpClient, BridgeConfig } from './bridge';

/**
 * Export all TypeScript type definitions for Pierre MCP tools
 *
 * These types are auto-generated from server tool schemas.
 * To regenerate: npm run generate-types
 */
export * from './types';