// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Structured error types for Pierre MCP Client SDK
// ABOUTME: Provides typed error codes and PierreError class for consistent error handling

/**
 * Error codes for categorizing Pierre SDK errors
 */
export enum PierreErrorCode {
  NETWORK_ERROR = 'NETWORK_ERROR',
  AUTH_ERROR = 'AUTH_ERROR',
  CONFIG_ERROR = 'CONFIG_ERROR',
  STORAGE_ERROR = 'STORAGE_ERROR',
  TIMEOUT_ERROR = 'TIMEOUT_ERROR',
  VALIDATION_ERROR = 'VALIDATION_ERROR',
  PROVIDER_ERROR = 'PROVIDER_ERROR',
}

/**
 * Structured error class for Pierre SDK operations
 * Extends Error with a typed error code and optional cause
 */
export class PierreError extends Error {
  constructor(
    public readonly code: PierreErrorCode,
    message: string,
    public readonly cause?: Error,
  ) {
    super(message);
    this.name = 'PierreError';
  }
}
