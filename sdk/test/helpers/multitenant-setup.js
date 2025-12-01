// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Multi-tenant test setup utilities for SDK integration tests
// ABOUTME: Creates and manages multiple tenant clients for concurrent testing
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

const { generateTestToken } = require('./token-generator');
const crypto = require('crypto');

/**
 * Setup multiple MCP clients for multi-tenant testing
 * Creates isolated tenant users with separate JWT tokens
 *
 * @param {number} numTenants - Number of tenant clients to create (default: 2)
 * @param {object} serverConfig - Optional server configuration
 * @returns {Promise<Array>} Array of client objects with {user, token, client}
 */
async function setupMultiTenantClients(numTenants = 2, serverConfig = {}) {
    const clients = [];

    for (let i = 0; i < numTenants; i++) {
        const email = `tenant${i + 1}-${Date.now()}-${crypto.randomUUID()}@example.com`;
        const userId = crypto.randomUUID();

        // Generate JWT token for this tenant
        const tokenData = generateTestToken(userId, email, 3600);

        // Note: Actual MCP client instantiation happens in the test
        // This helper just prepares the tenant credentials
        clients.push({
            tenantId: i + 1,
            userId,
            email,
            token: tokenData.access_token,
            tokenData,
            serverUrl: serverConfig.serverUrl || `http://localhost:${serverConfig.port || 8081}`,
        });
    }

    return clients;
}

/**
 * Create tenant via Rust server HTTP endpoint
 * This simulates creating a tenant through the server's admin API
 *
 * @param {string} serverUrl - Base server URL (e.g., http://localhost:8081)
 * @param {string} email - Tenant email address
 * @param {string} adminToken - Admin authentication token
 * @returns {Promise<{user, token}>} User object and JWT token
 */
async function createTenantViaRustBridge(serverUrl, email, adminToken) {
    // Use native fetch (Node 18+) or dynamic import for node-fetch
    const fetch = global.fetch || (async (...args) => {
        const nodeFetch = await import('node-fetch');
        return nodeFetch.default(...args);
    });

    try {
        // Create user via admin endpoint
        const response = await fetch(`${serverUrl}/admin/users`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${adminToken}`,
            },
            body: JSON.stringify({
                email,
                name: `Test Tenant ${email}`,
                password: 'TestPassword123!',
                auto_approve: true,
            }),
        });

        if (!response.ok) {
            const errorText = await response.text();
            throw new Error(`Failed to create tenant: ${response.status} - ${errorText}`);
        }

        const user = await response.json();

        // Generate token for the new user
        const tokenData = generateTestToken(user.id, user.email, 3600);

        return {
            user,
            token: tokenData.access_token,
            tokenData,
        };
    } catch (error) {
        throw new Error(`Failed to create tenant via Rust bridge: ${error.message}`);
    }
}

/**
 * Cleanup tenant clients
 * Ensures all resources are properly released
 *
 * @param {Array} clients - Array of client objects from setupMultiTenantClients
 */
async function cleanupMultiTenantClients(clients) {
    for (const client of clients) {
        // Close any open connections
        if (client.connection && typeof client.connection.close === 'function') {
            try {
                await client.connection.close();
            } catch (error) {
                // Ignore cleanup errors
                console.warn(`Warning: Failed to close connection for ${client.email}:`, error.message);
            }
        }

        // Kill any spawned processes
        if (client.process && typeof client.process.kill === 'function') {
            try {
                client.process.kill('SIGTERM');
            } catch (error) {
                // Ignore cleanup errors
                console.warn(`Warning: Failed to kill process for ${client.email}:`, error.message);
            }
        }
    }
}

/**
 * Verify tenant isolation
 * Validates that tenant A cannot access tenant B's resources
 *
 * @param {object} tenantA - First tenant client
 * @param {object} tenantB - Second tenant client
 * @param {string} resourceId - Resource ID to test access control
 * @param {string} serverUrl - Server URL
 * @returns {Promise<boolean>} True if isolation is properly enforced
 */
async function verifyTenantIsolation(tenantA, tenantB, resourceId, serverUrl) {
    const fetch = global.fetch || (async (...args) => {
        const nodeFetch = await import('node-fetch');
        return nodeFetch.default(...args);
    });

    try {
        // Tenant B tries to access Tenant A's resource
        const response = await fetch(`${serverUrl}/api/resources/${resourceId}`, {
            method: 'GET',
            headers: {
                'Authorization': `Bearer ${tenantB.token}`,
            },
        });

        // Should receive 403 Forbidden or 404 Not Found
        if (response.status === 403 || response.status === 404) {
            return true; // Isolation enforced
        }

        if (response.ok) {
            // Tenant B was able to access Tenant A's resource - BAD!
            console.error('SECURITY VIOLATION: Tenant isolation failed!');
            return false;
        }

        // Other error - unclear
        console.warn(`Unexpected response status: ${response.status}`);
        return false;
    } catch (error) {
        throw new Error(`Failed to verify tenant isolation: ${error.message}`);
    }
}

module.exports = {
    setupMultiTenantClients,
    createTenantViaRustBridge,
    cleanupMultiTenantClients,
    verifyTenantIsolation,
};
