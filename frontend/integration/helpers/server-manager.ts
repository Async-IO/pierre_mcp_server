// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Server management utilities for integration tests.
// ABOUTME: Provides health check polling and server readiness verification.

const BACKEND_URL = process.env.BACKEND_URL || 'http://localhost:8081';
const FRONTEND_URL = process.env.FRONTEND_URL || 'http://localhost:5173';

export interface HealthCheckResult {
  healthy: boolean;
  status?: string;
  version?: string;
  error?: string;
}

/**
 * Wait for the backend server to be healthy and ready to accept requests.
 * Polls the /health endpoint until it returns a successful response.
 */
export async function waitForBackendHealth(
  url: string = `${BACKEND_URL}/health`,
  maxAttempts: number = 60,
  intervalMs: number = 1000
): Promise<HealthCheckResult> {
  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      const response = await fetch(url, {
        method: 'GET',
        headers: { 'Accept': 'application/json' },
      });

      if (response.ok) {
        const data = await response.json();
        return {
          healthy: true,
          status: data.status,
          version: data.version,
        };
      }
    } catch (error) {
      if (attempt === maxAttempts) {
        return {
          healthy: false,
          error: `Server health check failed after ${maxAttempts} attempts: ${error}`,
        };
      }
    }

    await sleep(intervalMs);
  }

  return {
    healthy: false,
    error: `Server health check timed out after ${maxAttempts} attempts`,
  };
}

/**
 * Wait for the frontend dev server to be ready.
 */
export async function waitForFrontendReady(
  url: string = FRONTEND_URL,
  maxAttempts: number = 30,
  intervalMs: number = 1000
): Promise<boolean> {
  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      const response = await fetch(url, { method: 'GET' });
      if (response.ok) {
        return true;
      }
    } catch {
      if (attempt === maxAttempts) {
        return false;
      }
    }

    await sleep(intervalMs);
  }

  return false;
}

/**
 * Check if both servers are ready for integration tests.
 */
export async function waitForServersReady(): Promise<{ backend: boolean; frontend: boolean }> {
  const [backendResult, frontendReady] = await Promise.all([
    waitForBackendHealth(),
    waitForFrontendReady(),
  ]);

  return {
    backend: backendResult.healthy,
    frontend: frontendReady,
  };
}

/**
 * Get the backend API base URL.
 */
export function getBackendUrl(): string {
  return BACKEND_URL;
}

/**
 * Get the frontend base URL.
 */
export function getFrontendUrl(): string {
  return FRONTEND_URL;
}

function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}
