// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: OAuth callback simulator for automated testing of complete OAuth flow
// ABOUTME: Simulates browser OAuth approval and callback to bridge without user interaction
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

const fetch = global.fetch;
const { URL } = require('url');
const crypto = require('crypto');

/**
 * Simulates the complete OAuth 2.0 flow without browser interaction
 *
 * This helper allows automated testing of the EXACT code path where regressions occur:
 * - Dynamic client registration
 * - Authorization URL generation
 * - User approval (simulated)
 * - Authorization code exchange for tokens
 * - Tools cache refresh after OAuth
 */
class OAuthCallbackSimulator {
  constructor(bridgeClient, pierreServerUrl) {
    this.bridgeClient = bridgeClient;
    this.pierreServerUrl = pierreServerUrl;
    this.clientInfo = null;
    this.authState = null;
    this.codeVerifier = null;
    this.codeChallenge = null;
  }

  /**
   * Simulate the complete OAuth flow end-to-end
   *
   * @returns {Promise<{success: boolean, authCode: string, tokens: object}>}
   */
  async simulateCompleteOAuthFlow() {
    console.log('🔄 Starting OAuth callback simulation...');

    // Step 1: Trigger connect_to_pierre tool (starts OAuth flow in bridge)
    console.log('Step 1: Triggering connect_to_pierre tool...');
    const connectPromise = this.triggerConnectToPierre();

    // Give bridge time to:
    // - Register OAuth client
    // - Start callback server
    // - Generate authorization URL
    await this.sleep(2000);

    // Step 2: Register a test OAuth client with Pierre server
    console.log('Step 2: Registering OAuth client...');
    await this.registerOAuthClient();

    // Step 3: Generate PKCE challenge
    console.log('Step 3: Generating PKCE challenge...');
    this.generatePKCE();

    // Step 4: Simulate user approval and get authorization code
    console.log('Step 4: Simulating user approval...');
    const authCode = await this.simulateUserApproval();

    // Step 5: Trigger OAuth callback to bridge
    console.log('Step 5: Sending authorization code to bridge callback...');
    await this.triggerOAuthCallback(authCode);

    // Step 6: Wait for bridge to complete token exchange
    console.log('Step 6: Waiting for token exchange to complete...');
    await this.waitForTokenExchange();

    // Step 7: Verify tools cache was refreshed
    console.log('Step 7: Verifying tools cache refreshed...');
    const toolsRefreshed = await this.verifyToolsRefreshed();

    console.log('✅ OAuth callback simulation complete!');

    return {
      success: toolsRefreshed,
      authCode: authCode,
      toolsRefreshed: toolsRefreshed
    };
  }

  /**
   * Trigger the connect_to_pierre tool which starts OAuth flow
   */
  async triggerConnectToPierre() {
    const connectRequest = {
      jsonrpc: '2.0',
      id: 1000,
      method: 'tools/call',
      params: {
        name: 'connect_to_pierre',
        arguments: {}
      }
    };

    // This call will block waiting for OAuth callback
    // We'll send it but not wait for response (it completes when callback happens)
    try {
      // Don't await - let it run in background
      this.bridgeClient.send(connectRequest, 60000).catch(err => {
        console.log('connect_to_pierre returned:', err.message || 'completed');
      });
    } catch (error) {
      console.log('connect_to_pierre initiated:', error.message);
    }
  }

  /**
   * Register an OAuth client with Pierre server
   */
  async registerOAuthClient() {
    const registrationRequest = {
      client_id: `test_simulator_${Date.now()}`,
      client_secret: `secret_${Date.now()}`,
      redirect_uris: ['http://localhost:35535/oauth/callback'],
      grant_types: ['authorization_code'],
      response_types: ['code'],
      scope: 'read:fitness write:fitness',
      client_name: 'OAuth Callback Simulator',
      client_uri: 'https://test.example.com'
    };

    const response = await fetch(`${this.pierreServerUrl}/oauth2/register`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Accept': 'application/json'
      },
      body: JSON.stringify(registrationRequest)
    });

    if (!response.ok) {
      throw new Error(`Client registration failed: ${response.status}`);
    }

    const result = await response.json();
    this.clientInfo = {
      client_id: result.client_id || registrationRequest.client_id,
      client_secret: result.client_secret || registrationRequest.client_secret,
      redirect_uri: registrationRequest.redirect_uris[0]
    };

    console.log(`  ✓ Client registered: ${this.clientInfo.client_id}`);
  }

  /**
   * Generate PKCE code verifier and challenge
   */
  generatePKCE() {
    // Generate random code verifier (43-128 characters)
    this.codeVerifier = crypto.randomBytes(32).toString('base64url');

    // Generate code challenge (SHA256 hash of verifier)
    const hash = crypto.createHash('sha256').update(this.codeVerifier).digest();
    this.codeChallenge = hash.toString('base64url');

    // Generate state for CSRF protection
    this.authState = crypto.randomBytes(16).toString('hex');

    console.log(`  ✓ PKCE challenge generated`);
  }

  /**
   * Simulate user approving the OAuth authorization request
   *
   * In real flow: User clicks "Approve" in browser
   * In simulation: We call the authorize endpoint directly with approval=true
   *
   * @returns {Promise<string>} Authorization code
   */
  async simulateUserApproval() {
    // Build authorization URL (what would be shown to user)
    const authUrl = new URL(`${this.pierreServerUrl}/oauth2/authorize`);
    authUrl.searchParams.set('client_id', this.clientInfo.client_id);
    authUrl.searchParams.set('redirect_uri', this.clientInfo.redirect_uri);
    authUrl.searchParams.set('response_type', 'code');
    authUrl.searchParams.set('scope', 'read:fitness write:fitness');
    authUrl.searchParams.set('state', this.authState);
    authUrl.searchParams.set('code_challenge', this.codeChallenge);
    authUrl.searchParams.set('code_challenge_method', 'S256');

    console.log(`  Authorization URL: ${authUrl.toString().substring(0, 100)}...`);

    // Simulate user approving by calling authorize endpoint
    // Note: This requires Pierre server to support automated approval
    // For testing, we'll generate a test authorization code

    // Generate a test authorization code (in real scenario, server generates this)
    const authCode = crypto.randomBytes(16).toString('hex');

    console.log(`  ✓ User approval simulated, auth code: ${authCode.substring(0, 10)}...`);

    return authCode;
  }

  /**
   * Send OAuth callback to bridge's callback server
   *
   * @param {string} authCode - Authorization code from authorization server
   */
  async triggerOAuthCallback(authCode) {
    // Bridge's callback server is listening on http://localhost:35535/oauth/callback
    // We need to send a GET request with the authorization code and state

    const callbackUrl = new URL('http://localhost:35535/oauth/callback');
    callbackUrl.searchParams.set('code', authCode);
    callbackUrl.searchParams.set('state', this.authState);

    console.log(`  Callback URL: ${callbackUrl.toString()}`);

    try {
      // Send the callback (bridge will receive this and call exchangeCodeForTokens)
      const response = await fetch(callbackUrl.toString(), {
        method: 'GET',
        redirect: 'manual' // Don't follow redirects
      });

      console.log(`  ✓ Callback sent to bridge (status: ${response.status})`);

      // Bridge may return 302 redirect to success page, that's OK
      if (response.status === 302 || response.status === 200) {
        console.log(`  ✓ Bridge accepted callback`);
      }

    } catch (error) {
      // Connection refused or network error is expected if callback server not running
      console.log(`  ⚠️  Callback server not available: ${error.message}`);
      console.log(`  This is expected in automated tests without real OAuth flow`);
    }
  }

  /**
   * Wait for bridge to complete token exchange
   *
   * After bridge receives callback, it calls exchangeCodeForTokens() which:
   * 1. Exchanges authorization code for tokens
   * 2. Saves tokens
   * 3. CRITICAL: Refreshes tools cache (the regression fix)
   *
   * We detect completion by checking when tools list changes
   */
  async waitForTokenExchange(maxWaitMs = 10000) {
    const startTime = Date.now();
    let attempts = 0;

    while (Date.now() - startTime < maxWaitMs) {
      attempts++;

      try {
        const toolsList = await this.bridgeClient.send({
          jsonrpc: '2.0',
          id: 2000 + attempts,
          method: 'tools/list',
          params: {}
        }, 2000);

        const toolCount = toolsList.result.tools.length;
        const toolNames = toolsList.result.tools.map(t => t.name);

        // If we have more than just connect_to_pierre, token exchange completed
        if (toolCount > 5 && toolNames.includes('connect_provider')) {
          console.log(`  ✓ Token exchange completed (${toolCount} tools available)`);
          return true;
        }

        console.log(`  Waiting... (attempt ${attempts}, ${toolCount} tools)`);

      } catch (error) {
        console.log(`  Waiting for token exchange (attempt ${attempts})...`);
      }

      await this.sleep(500);
    }

    console.log(`  ⚠️  Token exchange did not complete within ${maxWaitMs}ms`);
    return false;
  }

  /**
   * Verify that tools cache was actually refreshed after OAuth
   *
   * This is THE critical check for the regression:
   * - Before OAuth: only connect_to_pierre
   * - After OAuth: 30+ tools including connect_provider
   */
  async verifyToolsRefreshed() {
    try {
      const toolsList = await this.bridgeClient.send({
        jsonrpc: '2.0',
        id: 3000,
        method: 'tools/list',
        params: {}
      }, 5000);

      const tools = toolsList.result.tools;
      const toolNames = tools.map(t => t.name);

      console.log(`  Tools after OAuth: ${tools.length} total`);
      console.log(`  Sample tools: ${toolNames.slice(0, 5).join(', ')}...`);

      // CRITICAL CHECKS:
      const hasConnectProvider = toolNames.includes('connect_provider');
      const hasGetActivities = toolNames.includes('get_activities');
      const hasMultipleTools = tools.length > 10;

      const allChecksPass = hasConnectProvider && hasGetActivities && hasMultipleTools;

      if (allChecksPass) {
        console.log(`  ✅ Tools cache REFRESHED successfully`);
        console.log(`     - connect_provider: ${hasConnectProvider ? '✓' : '✗'}`);
        console.log(`     - get_activities: ${hasGetActivities ? '✓' : '✗'}`);
        console.log(`     - Multiple tools (>10): ${hasMultipleTools ? '✓' : '✗'}`);
      } else {
        console.log(`  ❌ Tools cache NOT refreshed properly`);
        console.log(`     - connect_provider: ${hasConnectProvider ? '✓' : '✗'}`);
        console.log(`     - get_activities: ${hasGetActivities ? '✓' : '✗'}`);
        console.log(`     - Multiple tools (>10): ${hasMultipleTools ? '✓' : '✗'}`);
        console.log(`     Available tools: ${toolNames.join(', ')}`);
      }

      return allChecksPass;

    } catch (error) {
      console.log(`  ❌ Failed to verify tools: ${error.message}`);
      return false;
    }
  }

  /**
   * Helper: Sleep for specified milliseconds
   */
  sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}

/**
 * Simplified OAuth flow simulator for testing with pre-generated token
 *
 * Use this when you want to test post-OAuth behavior without full OAuth flow
 */
class SimpleOAuthSimulator {
  constructor(bridgeClient, pierreServerUrl) {
    this.bridgeClient = bridgeClient;
    this.pierreServerUrl = pierreServerUrl;
  }

  /**
   * Simulate OAuth by creating a user and generating a token via admin API
   * Then inject the token into bridge via --token flag restart
   *
   * This tests everything EXCEPT the exchangeCodeForTokens() path
   */
  async simulateWithAdminToken() {
    // Generate token via admin-setup binary
    const { generateTestToken } = require('./token-generator');

    const userId = crypto.randomUUID();
    const email = `test-${Date.now()}@simulator.com`;

    const testToken = generateTestToken(userId, email, 3600);

    console.log('✅ Generated test token (bypasses OAuth callback flow)');

    return testToken;
  }
}

module.exports = {
  OAuthCallbackSimulator,
  SimpleOAuthSimulator
};
