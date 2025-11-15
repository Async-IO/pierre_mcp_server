// ABOUTME: Bridge between SDK tests and Rust test server for multi-tenant coordination
// ABOUTME: Provides utilities for SDK tests to interact with Rust-managed server state
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

const crypto = require('crypto');
const { generateTestToken } = require('./token-generator');

// Use native fetch (Node 18+) or dynamic import for node-fetch
const fetch = global.fetch || (async (...args) => {
    const nodeFetch = await import('node-fetch');
    return nodeFetch.default(...args);
});

/**
 * Call Rust test endpoint to create tenant
 * This coordinates with Rust-side test infrastructure
 *
 * @param {string} serverUrl - Base server URL (e.g., http://localhost:8081)
 * @param {string} email - Tenant email address
 * @param {object} options - Additional options (adminToken, password, etc.)
 * @returns {Promise<{userId, email, token, tokenData}>} Tenant credentials
 */
async function createTenantOnServer(serverUrl, email, options = {}) {
    const password = options.password || 'TestPassword123!';
    const name = options.name || `Test User ${email}`;

    try {
        // For tests, we'll use the admin endpoint if available
        // Otherwise, generate credentials directly (test mode)
        if (options.adminToken) {
            const response = await fetch(`${serverUrl}/admin/users`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'Authorization': `Bearer ${options.adminToken}`,
                },
                body: JSON.stringify({
                    email,
                    name,
                    password,
                    auto_approve: true,
                }),
            });

            if (!response.ok) {
                const errorText = await response.text();
                throw new Error(`Failed to create tenant: ${response.status} - ${errorText}`);
            }

            const user = await response.json();
            const tokenData = generateTestToken(user.id, user.email, 3600);

            return {
                userId: user.id,
                email: user.email,
                token: tokenData.access_token,
                tokenData,
                user,
            };
        } else {
            // Test mode: Generate credentials without server call
            const userId = crypto.randomUUID();
            const tokenData = generateTestToken(userId, email, 3600);

            return {
                userId,
                email,
                token: tokenData.access_token,
                tokenData,
            };
        }
    } catch (error) {
        throw new Error(`Failed to create tenant on server: ${error.message}`);
    }
}

/**
 * Cleanup all test tenants via Rust endpoint
 * This is useful for cleaning up after test suites
 *
 * @param {string} serverUrl - Base server URL
 * @param {string} adminToken - Admin authentication token
 * @returns {Promise<number>} Number of tenants cleaned up
 */
async function cleanupTestTenants(serverUrl, adminToken) {
    try {
        // Call admin endpoint to cleanup test tenants
        const response = await fetch(`${serverUrl}/admin/test-cleanup`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${adminToken}`,
            },
            body: JSON.stringify({
                cleanup_type: 'test_users',
            }),
        });

        if (!response.ok) {
            // Cleanup endpoint might not exist - that's OK for tests
            console.warn(`Cleanup endpoint returned ${response.status}, continuing...`);
            return 0;
        }

        const result = await response.json();
        return result.deleted_count || 0;
    } catch (error) {
        // Non-critical error - log and continue
        console.warn(`Warning: Failed to cleanup test tenants: ${error.message}`);
        return 0;
    }
}

/**
 * Verify server health before running tests
 * Ensures Rust server is ready to accept requests
 *
 * @param {string} serverUrl - Base server URL
 * @param {number} timeoutMs - Timeout in milliseconds (default: 30000)
 * @returns {Promise<boolean>} True if server is healthy
 */
async function waitForServerHealth(serverUrl, timeoutMs = 30000) {
    const startTime = Date.now();
    const healthUrl = `${serverUrl}/health`;

    while (Date.now() - startTime < timeoutMs) {
        try {
            const controller = new AbortController();
            const fetchTimeout = setTimeout(() => controller.abort(), 2000);

            const response = await fetch(healthUrl, { signal: controller.signal });
            clearTimeout(fetchTimeout);

            if (response.ok) {
                const data = await response.json();
                if (data.status === 'healthy') {
                    return true;
                }
            }
        } catch (error) {
            // Server not ready yet, continue waiting
        }

        // Wait 500ms before retrying
        await new Promise(resolve => setTimeout(resolve, 500));
    }

    throw new Error(`Server health check failed after ${timeoutMs}ms`);
}

/**
 * Send MCP protocol request via HTTP
 * Direct HTTP transport for testing (bypasses SDK stdio bridge)
 *
 * @param {string} serverUrl - Base server URL
 * @param {string} method - MCP method name (e.g., "tools/list")
 * @param {object} params - Method parameters
 * @param {string} token - JWT authentication token
 * @returns {Promise<object>} MCP response result
 */
async function sendMcpHttpRequest(serverUrl, method, params, token) {
    try {
        const response = await fetch(`${serverUrl}/mcp`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${token}`,
            },
            body: JSON.stringify({
                jsonrpc: '2.0',
                id: 1,
                method,
                params,
            }),
        });

        if (!response.ok) {
            throw new Error(`HTTP request failed with status: ${response.status}`);
        }

        const responseData = await response.json();

        // Check for JSON-RPC error
        if (responseData.error) {
            throw new Error(`MCP error: ${JSON.stringify(responseData.error)}`);
        }

        return responseData.result;
    } catch (error) {
        throw new Error(`Failed to send MCP HTTP request: ${error.message}`);
    }
}

/**
 * Compare HTTP and SDK transport responses
 * Validates that both transports return identical data
 *
 * @param {object} httpResponse - Response from HTTP transport
 * @param {object} sdkResponse - Response from SDK stdio transport
 * @param {object} options - Comparison options
 * @returns {boolean} True if responses match
 */
function compareTransportResponses(httpResponse, sdkResponse, options = {}) {
    const ignoreFields = options.ignoreFields || [];

    // Deep comparison, ignoring specified fields
    const normalize = (obj) => {
        if (obj === null || obj === undefined) return obj;
        if (typeof obj !== 'object') return obj;

        const normalized = Array.isArray(obj) ? [] : {};
        for (const [key, value] of Object.entries(obj)) {
            if (ignoreFields.includes(key)) continue;
            normalized[key] = normalize(value);
        }
        return normalized;
    };

    const normalizedHttp = normalize(httpResponse);
    const normalizedSdk = normalize(sdkResponse);

    return JSON.stringify(normalizedHttp) === JSON.stringify(normalizedSdk);
}

/**
 * Get server metrics for test validation
 * Useful for verifying rate limiting, concurrency, etc.
 *
 * @param {string} serverUrl - Base server URL
 * @param {string} adminToken - Admin authentication token
 * @returns {Promise<object>} Server metrics
 */
async function getServerMetrics(serverUrl, adminToken) {
    try {
        const response = await fetch(`${serverUrl}/admin/metrics`, {
            method: 'GET',
            headers: {
                'Authorization': `Bearer ${adminToken}`,
            },
        });

        if (!response.ok) {
            console.warn(`Metrics endpoint returned ${response.status}`);
            return {};
        }

        return await response.json();
    } catch (error) {
        console.warn(`Warning: Failed to get server metrics: ${error.message}`);
        return {};
    }
}

module.exports = {
    createTenantOnServer,
    cleanupTestTenants,
    waitForServerHealth,
    sendMcpHttpRequest,
    compareTransportResponses,
    getServerMetrics,
};
