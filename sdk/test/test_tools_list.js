#!/usr/bin/env node

/**
 * Test script to verify that the full tools list is properly exposed after OAuth connection
 * This specifically tests the MCP protocol fixes for resources/list and prompts/list errors
 */

const { PierreClaudeBridge } = require('./dist/bridge.js');
const { spawn } = require('child_process');
const fs = require('fs');

class ToolsListTest {
    constructor() {
        this.config = {
            pierreServerUrl: 'http://localhost:8081',
            verbose: true
        };
        this.bridge = null;
        this.mockClient = null;
        this.results = {
            passed: 0,
            failed: 0,
            errors: []
        };
    }

    log(message) {
        console.log(`[${new Date().toISOString()}] ${message}`);
    }

    assert(condition, message) {
        if (condition) {
            this.results.passed++;
            this.log(`✅ ${message}`);
            return true;
        } else {
            this.results.failed++;
            this.results.errors.push(message);
            this.log(`❌ ${message}`);
            return false;
        }
    }

    async simulateClaudeDesktopRequest(method, params = {}) {
        return new Promise((resolve, reject) => {
            try {
                const request = {
                    jsonrpc: '2.0',
                    id: Math.floor(Math.random() * 1000),
                    method: method,
                    params: params
                };

                // Create a mock request that the bridge can handle
                const mockRequest = {
                    method: method,
                    params: params
                };

                // Simulate how Claude Desktop calls the bridge
                if (method === 'tools/list') {
                    this.bridge.claudeServer.requestHandlers.get('tools/list')(mockRequest)
                        .then(resolve)
                        .catch(reject);
                } else if (method === 'prompts/list') {
                    this.bridge.claudeServer.requestHandlers.get('prompts/list')(mockRequest)
                        .then(resolve)
                        .catch(reject);
                } else if (method === 'resources/list') {
                    this.bridge.claudeServer.requestHandlers.get('resources/list')(mockRequest)
                        .then(resolve)
                        .catch(reject);
                } else {
                    reject(new Error(`Unknown method: ${method}`));
                }
            } catch (error) {
                reject(error);
            }
        });
    }

    async testBridgeSetup() {
        this.log('\n🧪 TEST: Bridge Setup and Connection');

        try {
            this.bridge = new PierreClaudeBridge(this.config);
            await this.bridge.start();

            this.assert(this.bridge !== null, 'Bridge created successfully');
            this.assert(this.bridge.oauthProvider !== null, 'OAuth provider initialized');

            // Check if we have tokens (authentication)
            const tokens = await this.bridge.oauthProvider.tokens();
            const isAuthenticated = tokens !== null && tokens !== undefined;
            this.assert(isAuthenticated, 'Bridge is authenticated with valid tokens');

            return isAuthenticated;
        } catch (error) {
            this.assert(false, `Bridge setup failed: ${error.message}`);
            return false;
        }
    }

    async testResourcesList() {
        this.log('\n🧪 TEST: Resources List (should return empty, no errors)');

        try {
            const result = await this.simulateClaudeDesktopRequest('resources/list');
            this.assert(Array.isArray(result.resources), 'Resources list returned as array');
            this.assert(result.resources.length === 0, 'Resources list is empty (expected for Pierre server)');
            this.log(`Resources count: ${result.resources.length}`);
        } catch (error) {
            this.assert(false, `Resources list failed: ${error.message}`);
        }
    }

    async testPromptsList() {
        this.log('\n🧪 TEST: Prompts List (should return empty, no errors)');

        try {
            const result = await this.simulateClaudeDesktopRequest('prompts/list');
            this.assert(Array.isArray(result.prompts), 'Prompts list returned as array');
            this.assert(result.prompts.length === 0, 'Prompts list is empty (expected for Pierre server)');
            this.log(`Prompts count: ${result.prompts.length}`);
        } catch (error) {
            this.assert(false, `Prompts list failed: ${error.message}`);
        }
    }

    async testToolsList() {
        this.log('\n🧪 TEST: Tools List (the critical test)');

        try {
            const result = await this.simulateClaudeDesktopRequest('tools/list');
            this.assert(Array.isArray(result.tools), 'Tools list returned as array');

            const toolCount = result.tools.length;
            this.log(`Tools found: ${toolCount}`);

            // Log all tool names for visibility
            if (toolCount > 0) {
                this.log('Tool names:');
                result.tools.forEach((tool, index) => {
                    this.log(`  ${index + 1}. ${tool.name}: ${tool.description.substring(0, 80)}...`);
                });
            }

            // If we're authenticated, we should get the full tools list (35 tools)
            // If not authenticated, we should get just connect_to_pierre (1 tool)
            const tokens = await this.bridge.oauthProvider.tokens();
            const isAuthenticated = tokens !== null && tokens !== undefined;

            if (isAuthenticated) {
                this.assert(toolCount > 1, 'Authenticated - should have multiple tools available');
                this.assert(toolCount >= 25, 'Authenticated - should have at least 25 fitness tools');

                // Check for specific tools that should exist
                const toolNames = result.tools.map(t => t.name);
                const hasConnectTool = toolNames.includes('connect_to_pierre');
                const hasFitnessTools = toolNames.some(name => name.includes('fitness') || name.includes('strava') || name.includes('activity'));

                // If authenticated and connected, we shouldn't see connect_to_pierre in the list
                this.assert(!hasConnectTool, 'Authenticated - connect_to_pierre should not appear in tools list');
                this.assert(hasFitnessTools, 'Authenticated - should have fitness-related tools');
            } else {
                this.assert(toolCount === 1, 'Not authenticated - should only have connect_to_pierre tool');
                this.assert(result.tools[0].name === 'connect_to_pierre', 'Not authenticated - only tool should be connect_to_pierre');
            }

        } catch (error) {
            this.assert(false, `Tools list failed: ${error.message}`);
        }
    }

    async runAllTests() {
        this.log('🚀 Starting Tools List Test Suite\n');

        const isAuthenticated = await this.testBridgeSetup();

        // Always test resources and prompts - these should work regardless of auth status
        await this.testResourcesList();
        await this.testPromptsList();

        // The main test - tools list
        await this.testToolsList();

        this.log('\n📊 TEST RESULTS:');
        this.log(`✅ Passed: ${this.results.passed}`);
        this.log(`❌ Failed: ${this.results.failed}`);

        if (this.results.passed + this.results.failed > 0) {
            this.log(`📈 Success Rate: ${Math.round((this.results.passed / (this.results.passed + this.results.failed)) * 100)}%`);
        }

        if (this.results.failed > 0) {
            this.log('\n❌ FAILURES:');
            this.results.errors.forEach(error => this.log(`  - ${error}`));
            process.exit(1);
        } else {
            this.log('\n🎉 ALL TESTS PASSED!');
            this.log('\n✨ The MCP protocol fixes are working correctly!');
            this.log('Claude Desktop should now see the full tools list without MCP errors.');
            process.exit(0);
        }
    }
}

// Run the tests
const tester = new ToolsListTest();
tester.runAllTests().catch(error => {
    console.error('💥 TEST SUITE CRASHED:', error);
    process.exit(1);
});