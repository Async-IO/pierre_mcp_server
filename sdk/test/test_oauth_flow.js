#!/usr/bin/env node

// ABOUTME: Integration test for OAuth 2.0 authentication flow verification
// ABOUTME: Tests token acquisition, storage, invalidation, and reload without Claude Desktop
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

/**
 * Test script to verify OAuth authentication flow without Claude Desktop
 * This simulates the exact sequence that Claude Desktop uses
 */

const { PierreClaudeBridge } = require('./dist/bridge.js');
const fs = require('fs');
const path = require('path');

async function testOAuthFlow() {
    console.log('🧪 Testing OAuth Flow...');

    // Test configuration matching Claude Desktop usage
    const config = {
        pierreServerUrl: 'http://localhost:8081',
        verbose: true
    };

    // 1. Create bridge (simulating bridge startup)
    console.log('\n1️⃣ Creating bridge...');
    const bridge = new PierreClaudeBridge(config);

    // Access the OAuth provider from the bridge
    console.log('\n2️⃣ Setting up Pierre connection...');
    await bridge.start();

    // Get the OAuth provider instance - we need to access it through bridge internals
    const oauthProvider = bridge.oauthProvider;

    // 3. Check initial token state (should match startup logs)
    console.log('\n3️⃣ Checking initial tokens...');
    const initialTokens = await oauthProvider.tokens();
    console.log(`Initial tokens: ${initialTokens ? 'available' : 'none'}`);

    // 4. Check token storage path and file existence
    console.log('\n4️⃣ Checking token storage...');
    const tokenPath = path.join(require('os').homedir(), '.pierre-claude-tokens.json');
    console.log(`Token storage path: ${tokenPath}`);
    console.log(`Token file exists: ${fs.existsSync(tokenPath)}`);

    if (fs.existsSync(tokenPath)) {
        const tokenData = JSON.parse(fs.readFileSync(tokenPath, 'utf8'));
        console.log(`Token file contents:`, JSON.stringify(tokenData, null, 2));

        // Check token expiration
        if (tokenData.pierre && tokenData.pierre.saved_at && tokenData.pierre.expires_in) {
            const now = Math.floor(Date.now() / 1000);
            const expiresAt = tokenData.pierre.saved_at + tokenData.pierre.expires_in;
            const timeRemaining = expiresAt - now;
            console.log(`Token expires at: ${expiresAt}, Current: ${now}, Remaining: ${timeRemaining} seconds`);
            console.log(`Token valid: ${timeRemaining > 0}`);
        }
    }

    // 5. Test token reload logic (simulating retry after invalidateCredentials)
    console.log('\n5️⃣ Testing token invalidation and reload...');

    // Simulate the invalidateCredentials('tokens') call that happens during retries
    await oauthProvider.invalidateCredentials('tokens');
    console.log('✅ Invalidated in-memory tokens');

    // Now test if tokens() can reload from persistent storage
    console.log('🔄 Testing token reload from persistent storage...');
    const reloadedTokens = await oauthProvider.tokens();
    console.log(`Reloaded tokens: ${reloadedTokens ? 'available' : 'none'}`);

    if (reloadedTokens) {
        console.log(`✅ SUCCESS: Token reload working!`);
        console.log(`Access token: ${reloadedTokens.access_token.substring(0, 20)}...`);
        console.log(`Token type: ${reloadedTokens.token_type}`);
    } else {
        console.log(`❌ FAILURE: Token reload not working`);
    }

    // 6. Test MCP SDK compatibility
    console.log('\n6️⃣ Testing MCP SDK compatibility...');

    // Simulate what StreamableHTTPClientTransport does - call authProvider.tokens()
    const sdkTokens = await oauthProvider.tokens();
    if (sdkTokens && sdkTokens.access_token) {
        console.log(`✅ MCP SDK can get tokens`);
        // Test if this would produce proper Authorization header
        const authHeader = `${sdkTokens.token_type || 'Bearer'} ${sdkTokens.access_token}`;
        console.log(`Authorization header would be: ${authHeader.substring(0, 30)}...`);
    } else {
        console.log(`❌ MCP SDK cannot get tokens`);
    }

    console.log('\n🏁 Test completed!');
}

// Handle async errors
testOAuthFlow().catch(error => {
    console.error('❌ Test failed:', error);
    process.exit(1);
});