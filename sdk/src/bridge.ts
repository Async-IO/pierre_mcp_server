/**
 * Pierre-Claude Bridge
 *
 * MCP-compliant bridge implementation connecting Claude Desktop (stdio) to Pierre MCP Server (Streamable HTTP + OAuth 2.0)
 */

import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { StreamableHTTPClientTransport } from '@modelcontextprotocol/sdk/client/streamableHttp.js';
import { OAuthClientProvider } from '@modelcontextprotocol/sdk/client/auth.js';
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  ListResourcesRequestSchema,
  ReadResourceRequestSchema,
  ListPromptsRequestSchema,
  GetPromptRequestSchema,
  CompleteRequestSchema,
  InitializeRequestSchema,
  PingRequestSchema,
  SetLevelRequestSchema,
  McpError
} from '@modelcontextprotocol/sdk/types.js';
import {
  OAuthClientMetadata,
  OAuthClientInformation,
  OAuthTokens,
  AuthorizationServerMetadata,
  OAuthClientInformationFull
} from '@modelcontextprotocol/sdk/shared/auth.js';
import { z } from 'zod';

// Define custom notification schema for Pierre's OAuth completion notifications
const OAuthCompletedNotificationSchema = z.object({
  method: z.literal('notifications/oauth_completed'),
  params: z.object({
    provider: z.string(),
    success: z.boolean(),
    message: z.string(),
    user_id: z.string().optional()
  }).optional()
});

interface StoredTokens {
  pierre?: OAuthTokens & { saved_at?: number };
  providers?: Record<string, {
    access_token: string;
    refresh_token?: string;
    expires_at?: number;
    token_type?: string;
    scope?: string;
  }>;
}

export interface BridgeConfig {
  pierreServerUrl: string;
  jwtToken?: string;
  apiKey?: string;
  oauthClientId?: string;
  oauthClientSecret?: string;
  userEmail?: string;
  userPassword?: string;
  callbackPort?: number;
  verbose: boolean;
}

interface OAuth2TokenResponse {
  access_token: string;
  token_type: string;
  expires_in?: number;
  refresh_token?: string;
  scope?: string;
}

interface OAuth2ClientRegistration {
  client_id: string;
  client_secret: string;
  redirect_uris: string[];
  grant_types: string[];
  response_types: string[];
  scope: string;
}

class PierreOAuthClientProvider implements OAuthClientProvider {
  private serverUrl: string;
  private config: BridgeConfig;
  private clientInfo: OAuthClientInformationFull | undefined = undefined;
  private savedTokens: OAuthTokens | undefined = undefined;
  private codeVerifierValue: string | undefined = undefined;
  private stateValue: string | undefined = undefined;
  private callbackServer: any = undefined;
  private authorizationPending: Promise<any> | undefined = undefined;
  private callbackPort: number = 0;

  // Client-side token storage
  private tokenStoragePath: string;
  private allStoredTokens: StoredTokens = {};

  constructor(serverUrl: string, config: BridgeConfig) {
    this.serverUrl = serverUrl;
    this.config = config;

    // Initialize client-side token storage
    const os = require('os');
    const path = require('path');
    this.tokenStoragePath = path.join(os.homedir(), '.pierre-claude-tokens.json');

    // Load existing tokens from storage
    this.loadStoredTokens();

    console.error(`[Pierre OAuth] OAuth client provider created for server: ${serverUrl}`);
    console.error(`[Pierre OAuth] Token storage path: ${this.tokenStoragePath}`);
  }

  // Client-side token storage methods
  private loadStoredTokens(): void {
    try {
      const fs = require('fs');
      console.error(`[Pierre OAuth] Checking for tokens at path: ${this.tokenStoragePath}`);
      console.error(`[Pierre OAuth] File exists: ${fs.existsSync(this.tokenStoragePath)}`);
      if (fs.existsSync(this.tokenStoragePath)) {
        const data = fs.readFileSync(this.tokenStoragePath, 'utf8');
        this.allStoredTokens = JSON.parse(data);

        // Load Pierre tokens into memory for MCP SDK compatibility
        if (this.allStoredTokens.pierre) {
          this.savedTokens = this.allStoredTokens.pierre;
          console.error(`[Pierre OAuth] Loaded Pierre tokens from storage`);
        }

        if (this.allStoredTokens.providers) {
          const providerCount = Object.keys(this.allStoredTokens.providers).length;
          console.error(`[Pierre OAuth] Loaded ${providerCount} provider token(s) from storage`);
        }
      } else {
        console.error(`[Pierre OAuth] No stored tokens found, starting fresh`);
      }
    } catch (error) {
      console.error(`[Pierre OAuth] Failed to load stored tokens: ${error}`);
      this.allStoredTokens = {};
    }
  }

  private async saveStoredTokens(): Promise<void> {
    try {
      const fs = require('fs');
      const data = JSON.stringify(this.allStoredTokens, null, 2);
      fs.writeFileSync(this.tokenStoragePath, data, 'utf8');
      console.error(`[Pierre OAuth] Saved tokens to storage`);
    } catch (error) {
      console.error(`[Pierre OAuth] Failed to save tokens to storage: ${error}`);
    }
  }

  async clearTokens(): Promise<void> {
    try {
      // Clear in-memory Pierre tokens
      this.savedTokens = undefined;
      delete this.allStoredTokens.pierre;

      // Save updated storage (without Pierre tokens)
      await this.saveStoredTokens();
      console.error(`[Pierre OAuth] Cleared Pierre tokens from storage`);
    } catch (error) {
      console.error(`[Pierre OAuth] Failed to clear tokens: ${error}`);
    }
  }

  async saveProviderToken(provider: string, tokenData: any): Promise<void> {
    if (!this.allStoredTokens.providers) {
      this.allStoredTokens.providers = {};
    }

    this.allStoredTokens.providers[provider] = {
      access_token: tokenData.access_token,
      refresh_token: tokenData.refresh_token,
      expires_at: tokenData.expires_in ? Date.now() + (tokenData.expires_in * 1000) : undefined,
      token_type: tokenData.token_type || 'Bearer',
      scope: tokenData.scope
    };

    await this.saveStoredTokens();
    console.error(`[Pierre OAuth] Saved ${provider} provider token to client storage`);
  }

  getProviderToken(provider: string): any | undefined {
    const token = this.allStoredTokens.providers?.[provider];
    if (!token) {
      return undefined;
    }

    // Check if token is expired
    if (token.expires_at && Date.now() > token.expires_at) {
      console.error(`[Pierre OAuth] ${provider} token expired, removing from storage`);
      delete this.allStoredTokens.providers![provider];
      this.saveStoredTokens();
      return undefined;
    }

    return token;
  }

  get redirectUrl(): string {
    // Ensure callback server is started before providing redirect URL
    if (this.callbackPort === 0 && !this.callbackServer) {
      // Start callback server synchronously if not already started
      this.startCallbackServerSync();
    }
    // Wait for callbackPort to be set by the server startup
    if (this.callbackPort === 0) {
      throw new Error('Callback server failed to start - no port available');
    }
    return `http://localhost:${this.callbackPort}/oauth/callback`;
  }

  get clientMetadata(): OAuthClientMetadata {
    return {
      client_name: 'Pierre Claude Bridge',
      client_uri: 'https://claude.ai',
      redirect_uris: [this.redirectUrl],
      grant_types: ['authorization_code'],
      response_types: ['code'],
      scope: 'read:fitness write:fitness',
      token_endpoint_auth_method: 'client_secret_basic'
    };
  }

  async state(): Promise<string> {
    if (!this.stateValue) {
      this.stateValue = this.generateRandomString(32);
    }
    return this.stateValue;
  }

  async clientInformation(): Promise<OAuthClientInformation | undefined> {
    if (this.config.oauthClientId && this.config.oauthClientSecret) {
      return {
        client_id: this.config.oauthClientId,
        client_secret: this.config.oauthClientSecret
      };
    }
    return this.clientInfo ? {
      client_id: this.clientInfo.client_id,
      client_secret: this.clientInfo.client_secret
    } : undefined;
  }

  async saveClientInformation(clientInformation: OAuthClientInformationFull): Promise<void> {
    console.error(`[Pierre OAuth] Registering client with Pierre OAuth server: ${clientInformation.client_id}`);

    // Register this client with Pierre's OAuth server
    await this.registerClientWithPierre(clientInformation);

    this.clientInfo = clientInformation;
    console.error(`[Pierre OAuth] Saved client info: ${clientInformation.client_id}`);
  }

  private async registerClientWithPierre(clientInfo: OAuthClientInformationFull): Promise<void> {
    const registrationEndpoint = `${this.serverUrl}/oauth2/register`;

    const registrationRequest = {
      client_id: clientInfo.client_id,
      client_secret: clientInfo.client_secret,
      redirect_uris: this.clientMetadata.redirect_uris,
      grant_types: this.clientMetadata.grant_types,
      response_types: this.clientMetadata.response_types,
      scope: this.clientMetadata.scope,
      client_name: this.clientMetadata.client_name,
      client_uri: this.clientMetadata.client_uri
    };

    console.error(`[Pierre OAuth] Registering client at ${registrationEndpoint}`);

    try {
      const fetch = (await import('node-fetch')).default;
      const response = await fetch(registrationEndpoint, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Accept': 'application/json'
        },
        body: JSON.stringify(registrationRequest)
      });

      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(`Client registration failed: ${response.status} ${response.statusText}: ${errorText}`);
      }

      const registrationResponse: any = await response.json();
      console.error(`[Pierre OAuth] Client registration successful: ${JSON.stringify(registrationResponse)}`);

      // Update client info with the response from Pierre's server
      if (registrationResponse.client_id && registrationResponse.client_secret) {
        console.error(`[Pierre OAuth] Updating client info to use Pierre's returned client ID: ${registrationResponse.client_id}`);
        clientInfo.client_id = registrationResponse.client_id;
        clientInfo.client_secret = registrationResponse.client_secret;
      }

    } catch (error) {
      console.error(`[Pierre OAuth] Client registration failed: ${error}`);
      throw error;
    }
  }

  async saveTokens(tokens: OAuthTokens): Promise<void> {
    this.savedTokens = tokens;

    // Also save to persistent client-side storage
    this.allStoredTokens.pierre = { ...tokens, saved_at: Math.floor(Date.now() / 1000) };
    await this.saveStoredTokens();

    console.error(`[Pierre OAuth] Saved Pierre tokens: expires_in=${tokens.expires_in}`);
  }

  async tokens(): Promise<OAuthTokens | undefined> {
    // If no in-memory tokens, try to load from persistent storage
    if (!this.savedTokens && this.allStoredTokens.pierre) {
      console.error(`[Pierre OAuth] No in-memory tokens, attempting to reload from persistent storage`);

      // Check if stored tokens are still valid
      const storedTokens = this.allStoredTokens.pierre;
      const now = Math.floor(Date.now() / 1000);
      const expiresAt = (storedTokens.saved_at || 0) + (storedTokens.expires_in || 0);
      if (storedTokens.expires_in && storedTokens.saved_at && now < expiresAt) {
        // Token hasn't expired, but validate it with the server
        const isValid = await this.validateToken(storedTokens.access_token);
        if (isValid) {
          this.savedTokens = {
            access_token: storedTokens.access_token,
            refresh_token: storedTokens.refresh_token,
            expires_in: storedTokens.expires_in,
            token_type: storedTokens.token_type || 'Bearer',
            scope: storedTokens.scope
          };
          console.error(`[Pierre OAuth] Reloaded valid tokens from storage`);
        } else {
          console.error(`[Pierre OAuth] Stored token failed server validation (user may no longer exist), clearing storage`);
          delete this.allStoredTokens.pierre;
          await this.saveStoredTokens();
        }
      } else {
        console.error(`[Pierre OAuth] Stored tokens are expired, clearing storage`);
        delete this.allStoredTokens.pierre;
        await this.saveStoredTokens();
      }
    }

    console.error(`[Pierre OAuth] tokens() called, returning tokens: ${this.savedTokens ? 'available' : 'none'}`);
    if (this.savedTokens) {
      console.error(`[Pierre OAuth] Returning access_token: ${this.savedTokens.access_token.substring(0, 20)}...`);
      console.error(`[Pierre OAuth] Token type: ${this.savedTokens.token_type}`);
    }
    return this.savedTokens;
  }

  private async validateToken(accessToken: string): Promise<boolean> {
    try {
      // AbortController with 5s timeout for token validation
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 5000);

      try {
        // Make a lightweight request to validate the token
        const response = await fetch(`${this.serverUrl}/oauth/status`, {
          method: 'GET',
          headers: {
            'Authorization': `Bearer ${accessToken}`,
            'Content-Type': 'application/json'
          },
          signal: controller.signal
        });

        clearTimeout(timeoutId);

        if (response.ok) {
          console.error(`[Pierre OAuth] Token validation successful`);
          return true;
        }

        console.error(`[Pierre OAuth] Token validation failed: ${response.status} ${response.statusText}`);
        return false;
      } catch (fetchError) {
        clearTimeout(timeoutId);
        if (fetchError instanceof Error && fetchError.name === 'AbortError') {
          console.error(`[Pierre OAuth] Token validation timed out after 5s`);
        } else {
          throw fetchError;
        }
        return false;
      }
    } catch (error) {
      console.error(`[Pierre OAuth] Token validation request failed: ${error}`);
      return false;
    }
  }

  async redirectToAuthorization(authorizationUrl: URL): Promise<void> {
    console.error(`[Pierre OAuth] Starting OAuth 2.0 authorization flow`);

    // Start callback server to receive authorization response
    await this.startCallbackServer();

    console.error(`[Pierre OAuth] Opening browser for authorization`);

    // Open the authorization URL in the user's default browser with focus
    await this.openUrlInBrowserWithFocus(authorizationUrl.toString());

    console.error(`[Pierre OAuth] If browser doesn't open automatically, visit:`);
    console.error(`[Pierre OAuth] ${authorizationUrl.toString()}`);
    console.error(`[Pierre OAuth] Waiting for authorization completion`);

    // Wait for authorization completion
    if (this.authorizationPending) {
      const authResult = await this.authorizationPending;
      console.error(`[Pierre OAuth] Authorization callback completed, exchanging code for tokens`);

      // Exchange authorization code for JWT token
      await this.exchangeCodeForTokens(authResult.code, authResult.state);
    }
  }

  private async exchangeCodeForTokens(authorizationCode: string, state: string): Promise<void> {
    if (!this.clientInfo) {
      throw new Error('Client information not available for token exchange');
    }

    if (!this.clientInfo.client_secret) {
      throw new Error('Client secret not available for token exchange');
    }

    if (!this.codeVerifierValue) {
      throw new Error('Code verifier not available for token exchange');
    }

    const tokenEndpoint = `${this.serverUrl}/oauth2/token`;
    const tokenRequestBody = new URLSearchParams({
      grant_type: 'authorization_code',
      code: authorizationCode,
      redirect_uri: this.redirectUrl,
      client_id: this.clientInfo.client_id,
      client_secret: this.clientInfo.client_secret,
      code_verifier: this.codeVerifierValue
    });

    console.error(`[Pierre OAuth] Requesting JWT token from ${tokenEndpoint}`);

    try {
      const fetch = (await import('node-fetch')).default;
      const response = await fetch(tokenEndpoint, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/x-www-form-urlencoded',
          'Accept': 'application/json'
        },
        body: tokenRequestBody.toString()
      });

      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(`Token exchange failed: ${response.status} ${response.statusText}: ${errorText}`);
      }

      const tokenResponse = await response.json() as OAuth2TokenResponse;
      console.error(`[Pierre OAuth] Successfully received JWT token, expires_in=${tokenResponse.expires_in}`);

      // Convert to MCP SDK OAuthTokens format and save
      const oauthTokens: OAuthTokens = {
        access_token: tokenResponse.access_token,
        token_type: tokenResponse.token_type,
        expires_in: tokenResponse.expires_in,
        refresh_token: tokenResponse.refresh_token,
        scope: tokenResponse.scope
      };

      await this.saveTokens(oauthTokens);

    } catch (error) {
      console.error(`[Pierre OAuth] Token exchange failed: ${error}`);
      throw error;
    }
  }

  async saveCodeVerifier(codeVerifier: string): Promise<void> {
    this.codeVerifierValue = codeVerifier;
  }

  async codeVerifier(): Promise<string> {
    if (!this.codeVerifierValue) {
      throw new Error('Code verifier not found - authorization flow not started');
    }
    return this.codeVerifierValue;
  }

  async invalidateCredentials(scope: 'all' | 'client' | 'tokens' | 'verifier'): Promise<void> {
    switch (scope) {
      case 'all':
        this.clientInfo = undefined;
        this.savedTokens = undefined;
        this.codeVerifierValue = undefined;
        this.stateValue = undefined;
        // Clear all stored tokens
        this.allStoredTokens = {};
        await this.saveStoredTokens();
        break;
      case 'client':
        this.clientInfo = undefined;
        break;
      case 'tokens':
        this.savedTokens = undefined;
        // Note: We intentionally do NOT clear persistent storage here
        // This allows token reload from storage if the tokens are still valid
        // Only clear persistent storage if tokens are truly expired
        break;
      case 'verifier':
        this.codeVerifierValue = undefined;
        break;
    }
    console.error(`[Pierre OAuth] Invalidated credentials: ${scope}`);
  }

  async clearProviderTokens(): Promise<void> {
    this.allStoredTokens.providers = {};
    await this.saveStoredTokens();
    console.error(`[Pierre OAuth] Cleared all provider tokens from client storage`);
  }

  getTokenStatus(): { pierre: boolean; providers: Record<string, boolean> } {
    const status = {
      pierre: false,
      providers: {} as Record<string, boolean>
    };

    // Check Pierre token
    status.pierre = !!this.savedTokens;

    // Check provider tokens from client-side storage
    if (this.allStoredTokens.providers) {
      for (const [provider, tokenData] of Object.entries(this.allStoredTokens.providers)) {
        // Check if token exists and is not expired
        const isValid = tokenData && (!tokenData.expires_at || Date.now() < tokenData.expires_at);
        status.providers[provider] = !!isValid;
      }
    }

    return status;
  }

  private generateRandomString(length: number): string {
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~';
    let result = '';
    for (let i = 0; i < length; i++) {
      result += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return result;
  }

  private async openUrlInBrowserWithFocus(url: string): Promise<void> {
    const { exec } = await import('child_process');
    const platform = process.platform;

    if (platform === 'darwin') {
      // macOS: Open URL then explicitly activate browser after a brief delay
      exec(`open "${url}"`, (error: Error | null) => {
        if (error) {
          console.error('[Pierre OAuth] Failed to open browser:', error.message);
          return;
        }

        // After opening, try to activate common browsers
        setTimeout(() => {
          exec(`osascript -e 'tell application "Google Chrome" to activate' 2>/dev/null || osascript -e 'tell application "Safari" to activate' 2>/dev/null || osascript -e 'tell application "Firefox" to activate' 2>/dev/null || osascript -e 'tell application "Brave Browser" to activate' 2>/dev/null`, (activateError) => {
            if (activateError) {
              console.error('[Pierre OAuth] Could not activate browser (non-fatal)');
            }
          });
        }, 500);
      });
    } else if (platform === 'win32') {
      // Windows: start command brings window to front by default
      exec(`start "" "${url}"`, (error: Error | null) => {
        if (error) {
          console.error('[Pierre OAuth] Failed to open browser:', error.message);
        }
      });
    } else {
      // Linux: xdg-open
      exec(`xdg-open "${url}"`, (error: Error | null) => {
        if (error) {
          console.error('[Pierre OAuth] Failed to open browser:', error.message);
        }
      });
    }
  }

  private startCallbackServerSync(): void {
    // This is a synchronous wrapper that starts the server immediately
    // Use configured port (default 35535) instead of dynamic port
    const http = require('http');

    if (this.callbackServer) {
      return; // Already started
    }

    // Use configured callback port or default to 35535
    const port = this.config.callbackPort || 35535;

    // Create server immediately with fixed port
    this.callbackServer = http.createServer();

    // Add error handler for port-in-use errors
    this.callbackServer.on('error', (error: any) => {
      if (error.code === 'EADDRINUSE') {
        console.error(`[Pierre OAuth] Port ${port} is already in use - likely from previous session`);
        console.error(`[Pierre OAuth] Attempting to use dynamic port assignment instead...`);

        // Clean up failed server
        this.callbackServer?.close();
        this.callbackServer = null;

        // Retry with dynamic port (OS will assign available port)
        this.callbackServer = http.createServer();
        this.callbackServer.listen(0, 'localhost', () => {
          this.callbackPort = this.callbackServer.address().port;
          console.error(`[Pierre OAuth] Callback server listening on http://localhost:${this.callbackPort}/oauth/callback`);
          this.setupCallbackHandler();
        });
      } else {
        console.error(`[Pierre OAuth] Failed to start callback server:`, error);
        throw error;
      }
    });

    this.callbackServer.listen(port, 'localhost', () => {
      this.callbackPort = this.callbackServer.address().port;
      console.error(`[Pierre OAuth] Callback server listening on http://localhost:${this.callbackPort}/oauth/callback`);
      // Set up the actual request handler
      this.setupCallbackHandler();
    });
  }

  private setupCallbackHandler(): void {
    if (!this.callbackServer) return;

    this.callbackServer.removeAllListeners('request');
    this.callbackServer.on('request', async (req: any, res: any) => {
      try {
        if (!req.url) {
          res.writeHead(400, { 'Content-Type': 'text/plain' });
          res.end('Bad Request: No URL provided');
          return;
        }

        const url = require('url');
        const parsedUrl = url.parse(req.url, true);

        if (parsedUrl.pathname === '/oauth/callback') {
          const query = parsedUrl.query;

          if (query.error) {
            console.error(`[Pierre OAuth] Authorization failed: ${query.error}`);
            res.writeHead(400, { 'Content-Type': 'text/html' });
            res.end(this.renderErrorTemplate(
              'Claude Desktop OAuth',
              `${query.error}`,
              `${query.error_description || 'Please try connecting again.'}`
            ));
          } else if (query.code && query.state) {
            console.error(`[Pierre OAuth] Authorization successful, received code`);
            res.writeHead(200, { 'Content-Type': 'text/html' });
            res.end(this.renderSuccessTemplate('Claude Desktop OAuth'));

            // Resolve the authorization promise if it exists
            const authResolve = (this.callbackServer as any)._authResolve;
            if (authResolve) {
              authResolve({ code: query.code, state: query.state });
            }

            // Authorization result is handled via promise resolution in redirectToAuthorization()
          } else {
            console.error(`[Pierre OAuth] Invalid callback parameters`);
            res.writeHead(400, { 'Content-Type': 'text/html' });
            res.end(this.renderErrorTemplate(
              'Claude Desktop OAuth',
              'Invalid Request',
              'Missing required authorization parameters. Please try connecting again.'
            ));
          }
        } else if (parsedUrl.pathname?.startsWith('/oauth/provider-callback/') && req.method === 'POST') {
          // Handle provider token callback for client-side storage
          const pathParts = parsedUrl.pathname.split('/');
          const provider = pathParts[3]; // /oauth/provider-callback/{provider}

          console.error(`[Pierre OAuth] Provider token callback for ${provider}`);

          let body = '';
          req.on('data', (chunk: any) => {
            body += chunk.toString();
          });

          req.on('end', async () => {
            try {
              const tokenData = JSON.parse(body);
              await this.saveProviderToken(provider, tokenData);

              res.writeHead(200, { 'Content-Type': 'application/json' });
              res.end(JSON.stringify({
                success: true,
                message: `${provider} token stored client-side`
              }));
            } catch (error) {
              console.error(`[Pierre OAuth] Failed to save ${provider} token: ${error}`);
              res.writeHead(400, { 'Content-Type': 'application/json' });
              res.end(JSON.stringify({
                success: false,
                message: 'Failed to save provider token'
              }));
            }
          });

          return; // Don't write response yet, wait for request body
        } else {
          res.writeHead(404, { 'Content-Type': 'text/plain' });
          res.end('Not Found');
        }
      } catch (error) {
        console.error(`[Pierre OAuth] Callback server error: ${error}`);
        res.writeHead(500, { 'Content-Type': 'text/plain' });
        res.end('Internal Server Error');
      }
    });
  }

  private async startCallbackServer(): Promise<void> {
    // If server already started by sync method, just set up the authorization promise
    if (this.callbackServer) {
      this.authorizationPending = new Promise((resolve, reject) => {
        // Store resolve/reject for the callback handler to use
        (this.callbackServer as any)._authResolve = resolve;
        (this.callbackServer as any)._authReject = reject;
      });
      return Promise.resolve();
    }

    // Otherwise start server normally
    this.startCallbackServerSync();
    this.authorizationPending = new Promise((resolve, reject) => {
      (this.callbackServer as any)._authResolve = resolve;
      (this.callbackServer as any)._authReject = reject;
    });
  }

  private renderSuccessTemplate(provider: string): string {
    return `
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>OAuth Success - ${provider}</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; background-color: #f5f5f5; }
        .container { max-width: 600px; margin: 0 auto; background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
        .success { color: #27ae60; font-size: 24px; margin-bottom: 20px; }
        .info { color: #2c3e50; margin: 10px 0; }
        .code { background: #ecf0f1; padding: 10px; border-radius: 4px; font-family: monospace; }
    </style>
</head>
<body>
    <div class="container">
        <h1 class="success">✓ OAuth Authorization Successful</h1>
        <div class="info"><strong>Provider:</strong> ${provider}</div>
        <div class="info"><strong>Status:</strong> Connected successfully</div>
        <div class="info"><strong>Status:</strong> <span class="code">Connected</span></div>
        <p><strong>✅ Authentication complete!</strong></p>
        <p>Please return to your MCP client (Claude Desktop, ChatGPT, etc.) to continue.</p>
        <p><small>This window will close automatically in 3 seconds...</small></p>
        <script>
            // Auto-close window after user has time to read the success message
            window.onload = function() {
                setTimeout(() => {
                    window.close();
                }, 3000);
            };
        </script>
    </div>
</body>
</html>
    `;
  }

  private renderErrorTemplate(provider: string, error: string, description: string): string {
    return `
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>OAuth Error - ${provider}</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; background-color: #f5f5f5; }
        .container { max-width: 600px; margin: 0 auto; background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
        .error { color: #e74c3c; font-size: 24px; margin-bottom: 20px; }
        .info { color: #2c3e50; margin: 10px 0; }
        .code { background: #ecf0f1; padding: 10px; border-radius: 4px; font-family: monospace; }
    </style>
</head>
<body>
    <div class="container">
        <h1 class="error">✗ OAuth Authorization Failed</h1>
        <div class="info"><strong>Provider:</strong> ${provider}</div>
        <div class="info"><strong>Error:</strong> <span class="code">${error}</span></div>
        <div class="info"><strong>Description:</strong> ${description}</div>
        <p>Please return to your MCP client and try connecting again.</p>
        <p><small>This window will close automatically in 5 seconds...</small></p>
        <script>
            // Auto-close window after user has time to read the error
            window.onload = function() {
                setTimeout(() => {
                    window.close();
                }, 5000);
            };
        </script>
    </div>
</body>
</html>
    `;
  }

}

export class PierreClaudeBridge {
  private config: BridgeConfig;
  private pierreClient: Client | null = null;
  private claudeServer: Server | null = null;
  private serverTransport: StdioServerTransport | null = null;
  private cachedTools: any = null;

  constructor(config: BridgeConfig) {
    this.config = config;
  }

  private log(message: string, ...args: any[]) {
    if (this.config.verbose) {
      console.error(`[Pierre-Claude Bridge] ${message}`, ...args);
    }
  }

  async start(): Promise<void> {
    try {
      // Step 1: Create MCP server for Claude Desktop using stdio
      // This must happen FIRST so the bridge can respond to MCP validator
      await this.createClaudeServer();

      // Step 2: Start the bridge (stdio transport)
      await this.startBridge();

      // Step 3: Create MCP client connection to Pierre using Streamable HTTP
      // With proactive connection: if tokens exist, connect and cache tools
      await this.initializePierreConnection();

      this.log('✅ Bridge started successfully');
    } catch (error) {
      this.log('❌ Failed to start bridge:', error);
      throw error;
    }
  }

  private async initializePierreConnection(): Promise<void> {
    // Set up Pierre connection parameters
    this.mcpUrl = `${this.config.pierreServerUrl}/mcp`;
    this.oauthProvider = new PierreOAuthClientProvider(this.config.pierreServerUrl, this.config);
    this.log(`📡 Pierre MCP URL configured: ${this.mcpUrl}`);

    // ALWAYS connect proactively to cache tools for Claude Desktop
    // Server allows tools/list without authentication - only tool calls require auth
    // This ensures all tools are visible immediately in Claude Desktop (tools/list_changed doesn't work)
    try {
      this.log('🔌 Connecting to Pierre proactively to cache all tools for Claude Desktop');
      await this.connectToPierre();

      // Cache tools immediately so they're ready for tools/list
      if (this.pierreClient) {
        const client = this.pierreClient;
        const tools = await client.listTools();
        this.cachedTools = tools;
        this.log(`✅ Cached ${tools.tools.length} tools from Pierre: ${JSON.stringify(tools.tools.map((t: any) => t.name))}`);
      }
    } catch (error: any) {
      // If proactive connection fails, continue anyway
      // The bridge should still start - provide minimal toolset
      this.log(`⚠️ Proactive connection failed: ${error.message}`);
      this.log('📋 Bridge will start with connect_to_pierre tool only');
      // Don't propagate error - bridge should start successfully
    }
  }

  private async ensurePierreConnected(): Promise<void> {
    if (this.pierreClient) {
      return; // Already connected
    }

    this.log('🔌 Connecting to Pierre MCP Server...');
    await this.connectToPierre();
  }

  private oauthProvider: PierreOAuthClientProvider | null = null;
  private mcpUrl: string = '';

  private async connectToPierre(): Promise<void> {
    this.log('🔌 Connecting to Pierre MCP Server...');

    if (!this.oauthProvider) {
      throw new Error('OAuth provider not initialized - call initializePierreConnection() first');
    }

    this.log(`📡 Target URL: ${this.mcpUrl}`);

    // Always attempt connection to discover tools (initialize and tools/list don't require auth)
    // If tokens exist, the connection will be fully authenticated
    // If no tokens, we can still discover tools but tool calls will require authentication via connect_to_pierre
    const existingTokens = await this.oauthProvider.tokens();
    if (existingTokens) {
      this.log('🎫 Found existing tokens - connecting with authentication');
    } else {
      this.log('📋 No tokens found - connecting without authentication to discover tools');
    }

    await this.attemptConnection();
  }

  private async attemptConnection(): Promise<void> {
    if (!this.oauthProvider) {
      throw new Error('OAuth provider not initialized');
    }

    let connected = false;
    let retryCount = 0;
    const maxRetries = 3;

    while (!connected && retryCount < maxRetries) {
      try {
        // Create fresh MCP client for each attempt
        this.pierreClient = new Client(
          {
            name: 'pierre-claude-bridge',
            version: '1.0.0'
          },
          {
            capabilities: {
              tools: {},
              resources: {},
              prompts: {},
              logging: {}
            }
          }
        );

        // Create fresh transport for each attempt
        const baseUrl = new URL(this.mcpUrl);
        const transport = new StreamableHTTPClientTransport(baseUrl, {
          authProvider: this.oauthProvider
        });

        // Connect to Pierre MCP Server with OAuth 2.0 flow
        await this.pierreClient.connect(transport);
        connected = true;
        this.log('✅ Connected to Pierre MCP Server with OAuth 2.0');
        this.log(`✅ pierreClient is now set: ${!!this.pierreClient}`);
      } catch (error: any) {
        if (error.message === 'Unauthorized' && retryCount < maxRetries - 1) {
          retryCount++;
          this.log(`🔄 Token expired or invalid, retrying... (attempt ${retryCount}/${maxRetries})`);

          // Clear invalid tokens
          await this.oauthProvider.invalidateCredentials('tokens');

          await new Promise(resolve => setTimeout(resolve, 1000));
        } else {
          this.log(`❌ Failed to connect after ${retryCount + 1} attempts: ${error.message}`);
          throw error;
        }
      }
    }

    if (!connected) {
      throw new Error(`Failed to connect to Pierre MCP Server after ${maxRetries} attempts - authentication may be required`);
    }
  }

  async initiateConnection(): Promise<void> {
    if (!this.oauthProvider) {
      throw new Error('OAuth provider not initialized');
    }

    this.log('🚀 Initiating OAuth connection to Pierre MCP Server');

    // This will trigger the OAuth flow if no valid tokens exist
    await this.attemptConnection();

    this.log(`✅ After attemptConnection, pierreClient is: ${!!this.pierreClient}`);
  }

  getClientSideTokenStatus(): { pierre: boolean; providers: Record<string, boolean> } {
    if (!this.oauthProvider) {
      return { pierre: false, providers: {} };
    }

    return this.oauthProvider.getTokenStatus();
  }

  private async createClaudeServer(): Promise<void> {
    this.log('🖥️ Creating Claude Desktop server...');

    // Create MCP server for Claude Desktop
    this.claudeServer = new Server(
      {
        name: 'pierre-fitness',
        version: '1.0.0'
      },
      {
        capabilities: {
          tools: {},
          resources: {},
          prompts: {},
          logging: {}
        }
      }
    );

    // Set up request handlers - bridge all requests to Pierre
    this.setupRequestHandlers();

    // Create stdio transport for Claude Desktop
    this.serverTransport = new StdioServerTransport();

    this.log('✅ Claude Desktop server created');
  }

  private setupRequestHandlers(): void {
    if (!this.claudeServer) {
      throw new Error('Claude server not initialized');
    }

    // Bridge tools/list requests
    this.claudeServer.setRequestHandler(ListToolsRequestSchema, async (request) => {
      this.log('📋 Bridging tools/list request');
      this.log(`📋 pierreClient exists: ${!!this.pierreClient}, cached tools: ${!!this.cachedTools}`);

      try {
        // If we have cached tools, return them immediately (from proactive connection)
        if (this.cachedTools) {
          this.log(`📤 Sending cached tools/list response: ${JSON.stringify(this.cachedTools.tools.map((t: any) => t.name))}`);
          return this.cachedTools;
        }

        if (this.pierreClient) {
          // If connected, forward the request through the pierreClient
          // The client already has OAuth authentication built into its transport
          this.log('📋 Forwarding tools/list to Pierre via authenticated client');
          const client = this.pierreClient;
          const result = await client.listTools();
          this.log(`📋 Received ${result.tools.length} tools from Pierre`);
          // Cache the result for next time
          this.cachedTools = result;
          this.log(`📤 Sending tools/list response: ${JSON.stringify(result.tools.map((t: any) => t.name))}`);
          return result;
        } else {
          // If not connected, provide minimal tool list with connect_to_pierre
          return {
            tools: [{
              name: 'connect_to_pierre',
              description: 'Connect to Pierre - Authenticate with Pierre Fitness Server to access your fitness data. This will open a browser window for secure login. Use this when you\'re not connected or need to reconnect.',
              inputSchema: {
                type: 'object',
                properties: {},
                required: []
              }
            }]
          };
        }
      } catch (error: any) {
        this.log(`❌ Error getting tools list: ${error.message || error}`);
        this.log('❌ Providing connect tool only');

        return {
          tools: [{
            name: 'connect_to_pierre',
            description: 'Connect to Pierre - Authenticate with Pierre Fitness Server to access your fitness data. This will open a browser window for secure login. Use this when you\'re not connected or need to reconnect.',
            inputSchema: {
              type: 'object',
              properties: {},
              required: []
            }
          }]
        };
      }
    });

    // Bridge tools/call requests
    this.claudeServer.setRequestHandler(CallToolRequestSchema, async (request) => {
      this.log('🔧 Bridging tool call:', request.params.name);

      // Handle special authentication tools
      if (request.params.name === 'connect_to_pierre') {
        return await this.handleConnectToPierre(request);
      }

      if (request.params.name === 'connect_provider') {
        return await this.handleConnectProvider(request);
      }

      // Ensure we have a connection before forwarding other tools
      try {
        await this.ensurePierreConnected();
      } catch (error) {
        return {
          content: [{
            type: 'text',
            text: `Failed to connect to Pierre: ${error instanceof Error ? error.message : String(error)}. Please use the "Connect to Pierre" tool to authenticate.`
          }],
          isError: true
        };
      }

      try {
        this.log(`🔄 Forwarding tool call ${request.params.name} to Pierre server...`);
        // Use callTool() instead of request() - Client.request() is for raw JSON-RPC,
        // but we want the higher-level callTool() method which handles the protocol correctly
        const result = await this.pierreClient!.callTool({
          name: request.params.name,
          arguments: request.params.arguments || {}
        });
        this.log(`✅ Tool call ${request.params.name} result:`, JSON.stringify(result).substring(0, 200));
        return result;
      } catch (error) {
        this.log(`❌ Tool call ${request.params.name} failed:`, error);
        return {
          content: [{
            type: 'text',
            text: `Tool execution failed: ${error instanceof Error ? error.message : String(error)}`
          }],
          isError: true
        };
      }
    });

    // Bridge resources/list requests
    this.claudeServer.setRequestHandler(ListResourcesRequestSchema, async (request) => {
      this.log('📚 Bridging resources/list request');

      // Pierre server doesn't provide resources, so always return empty list
      return { resources: [] };
    });

    // Bridge resources/read requests
    this.claudeServer.setRequestHandler(ReadResourceRequestSchema, async (request) => {
      this.log('📖 Bridging resource read:', request.params.uri);

      if (!this.pierreClient) {
        return {
          contents: [{
            type: 'text',
            text: 'Not connected to Pierre. Please use the "Connect to Pierre" tool first to authenticate.'
          }]
        };
      }

      return await this.pierreClient.request(request, ReadResourceRequestSchema);
    });

    // Bridge prompts/list requests
    this.claudeServer.setRequestHandler(ListPromptsRequestSchema, async (request) => {
      this.log('💭 Bridging prompts/list request');

      // Pierre server doesn't provide prompts, so always return empty list
      return { prompts: [] };
    });

    // Bridge prompts/get requests
    this.claudeServer.setRequestHandler(GetPromptRequestSchema, async (request) => {
      this.log('💬 Bridging prompt get:', request.params.name);

      if (!this.pierreClient) {
        return {
          description: 'Not connected to Pierre',
          messages: [{
            role: 'user',
            content: {
              type: 'text',
              text: 'Not connected to Pierre. Please use the "Connect to Pierre" tool first to authenticate.'
            }
          }]
        };
      }

      return await this.pierreClient.request(request, GetPromptRequestSchema);
    });

    // Handle ping requests
    this.claudeServer.setRequestHandler(PingRequestSchema, async () => {
      this.log('🏓 Handling ping request');
      return {};
    });

    // Handle logging/setLevel requests
    this.claudeServer.setRequestHandler(SetLevelRequestSchema, async (request) => {
      this.log(`📊 Setting log level to: ${request.params.level}`);
      return {};
    });

    // Bridge completion requests
    this.claudeServer.setRequestHandler(CompleteRequestSchema, async (request) => {
      this.log('✨ Bridging completion request');

      if (!this.pierreClient) {
        return {
          completion: {
            values: [],
            total: 0,
            hasMore: false
          }
        };
      }

      return await this.pierreClient.request(request, CompleteRequestSchema);
    });

    this.log('🌉 Request handlers configured');
  }

  private async handleConnectToPierre(request: any): Promise<any> {
    try {
      this.log('🔐 Handling connect_to_pierre tool call - initiating OAuth flow');

      if (!this.oauthProvider) {
        return {
          content: [{
            type: 'text',
            text: 'OAuth provider not initialized. Please restart the bridge.'
          }],
          isError: true
        };
      }

      // Check if already authenticated (have valid tokens)
      const existingTokens = await this.oauthProvider.tokens();
      if (existingTokens && this.pierreClient) {
        return {
          content: [{
            type: 'text',
            text: 'Already connected to Pierre! You can now use all fitness tools to access your Strava and Fitbit data.'
          }],
          isError: false
        };
      }

      // Initiate the OAuth connection
      await this.initiateConnection();

      // Cache tools immediately after successful connection
      if (this.pierreClient) {
        try {
          const client = this.pierreClient as Client;
          const tools = await client.listTools();
          this.cachedTools = tools;
          this.log(`✅ Cached ${tools.tools.length} tools after connect_to_pierre: ${JSON.stringify(tools.tools.map((t: any) => t.name))}`);
        } catch (toolError: any) {
          this.log(`⚠️ Failed to cache tools: ${toolError.message}`);
        }
      }

      // Notify Claude Desktop that tools have changed (now authenticated)
      if (this.claudeServer) {
        try {
          await this.claudeServer.notification({
            method: 'notifications/tools/list_changed',
            params: {}
          });
          this.log('📢 Sent tools/list_changed notification to Claude Desktop');
        } catch (error: any) {
          this.log('⚠️ Failed to send tools/list_changed notification:', error.message);
        }
      }

      return {
        content: [{
          type: 'text',
          text: 'Successfully connected to Pierre! You can now access all your fitness data including Strava activities, athlete profile, and performance statistics.'
        }],
        isError: false
      };

    } catch (error: any) {
      this.log('❌ Failed to connect to Pierre:', error.message);

      return {
        content: [{
          type: 'text',
          text: `Failed to connect to Pierre: ${error.message}. Please check that the Pierre server is running and try again.`
        }],
        isError: true
      };
    }
  }

  private async handleConnectProvider(request: any): Promise<any> {
    try {
      this.log('🔄 Handling unified connect_provider tool call');

      if (!this.oauthProvider) {
        return {
          content: [{
            type: 'text',
            text: 'OAuth provider not initialized. Please restart the bridge.'
          }],
          isError: true
        };
      }

      // Extract provider from request parameters
      const provider = request.params.arguments?.provider || 'strava';
      this.log(`🔄 Unified flow for provider: ${provider}`);

      // Step 1: Ensure Pierre authentication is complete
      if (!this.pierreClient) {
        this.log('🔐 Pierre not connected - initiating Pierre authentication first');
        try {
          await this.initiateConnection();
          this.log('✅ Pierre authentication completed');
        } catch (error: any) {
          this.log(`❌ Pierre authentication failed: ${error.message}`);
          return {
            content: [{
              type: 'text',
              text: `Failed to authenticate with Pierre: ${error.message}. Please try again.`
            }],
            isError: true
          };
        }
      } else {
        this.log('✅ Pierre already authenticated');
      }

      // Step 2: Check if provider is already connected
      this.log(`🔍 Checking if ${provider} is already connected`);
      try {
        if (this.pierreClient) {
          const connectionStatus = await this.pierreClient.callTool({
            name: 'get_connection_status',
            arguments: { provider: provider }
          });

          // Check if the provider is already connected
          // The server returns structuredContent with providers array containing connection status
          if (connectionStatus) {
            this.log(`📊 Full connection status response: ${JSON.stringify(connectionStatus).substring(0, 500)}...`);

            // Access the structured content with provider connection status
            const structured = (connectionStatus as any).structuredContent;
            if (structured && structured.providers && Array.isArray(structured.providers)) {
              const providerInfo = structured.providers.find((p: any) =>
                p.provider && p.provider.toLowerCase() === provider.toLowerCase()
              );

              if (providerInfo && providerInfo.connected === true) {
                this.log(`✅ ${provider} is already connected - no OAuth needed`);
                return {
                  content: [{
                    type: 'text',
                    text: `Already connected to ${provider.toUpperCase()}! You can now access your ${provider} fitness data.`
                  }],
                  isError: false
                };
              } else {
                this.log(`🔄 ${provider} connected status: ${providerInfo ? providerInfo.connected : 'not found'}`);
              }
            }
          }
        }

        this.log(`🔄 ${provider} not connected - proceeding with OAuth flow`);
      } catch (error: any) {
        this.log(`⚠️ Could not check connection status: ${error.message} - proceeding with OAuth anyway`);
      }

      // Step 3: Extract user_id from JWT token
      const tokens = await this.oauthProvider.tokens();
      if (!tokens?.access_token) {
        throw new Error('No access token available');
      }

      // Decode JWT to get user_id (JWT format: header.payload.signature)
      const payload = tokens.access_token.split('.')[1];
      const decoded = JSON.parse(Buffer.from(payload, 'base64').toString());
      const userId = decoded.sub;

      if (!userId) {
        throw new Error('Could not extract user_id from JWT token');
      }

      this.log(`🔄 Initiating ${provider} OAuth flow for user: ${userId}`);

      try {
        // Correct OAuth URL format: /api/oauth/auth/{provider}/{user_id}
        const providerOAuthUrl = `${this.config.pierreServerUrl}/api/oauth/auth/${provider}/${userId}`;

        // Open provider OAuth in browser with focus
        await this.openUrlInBrowserWithFocus(providerOAuthUrl);

        this.log(`🌐 Opened ${provider} OAuth in browser: ${providerOAuthUrl}`);

        return {
          content: [{
            type: 'text',
            text: `🎉 Unified authentication flow completed!\n\n✅ Pierre: Connected\n🔄 ${provider.toUpperCase()}: Opening in browser...\n\nPlease complete the ${provider.toUpperCase()} authentication in your browser. Once done, you'll have full access to your fitness data!`
          }],
          isError: false
        };

      } catch (error: any) {
        this.log(`❌ Failed to open ${provider} OAuth: ${error.message}`);
        return {
          content: [{
            type: 'text',
            text: `Pierre authentication successful, but failed to open ${provider.toUpperCase()} OAuth: ${error.message}. You can manually visit the OAuth page in Pierre's web interface.`
          }],
          isError: false // Not a complete failure since Pierre auth worked
        };
      }

    } catch (error: any) {
      this.log('❌ Unified connect_provider failed:', error.message);

      return {
        content: [{
          type: 'text',
          text: `Unified authentication failed: ${error.message}. Please check that Pierre server is running and try again.`
        }],
        isError: true
      };
    }
  }

  private async openUrlInBrowserWithFocus(url: string): Promise<void> {
    const { exec } = await import('child_process');
    const platform = process.platform;

    if (platform === 'darwin') {
      // macOS: Open URL then explicitly activate browser after a brief delay
      exec(`open "${url}"`, (error: Error | null) => {
        if (error) {
          this.log(`⚠️ Failed to open browser: ${error.message}`);
          return;
        }

        // After opening, try to activate common browsers
        setTimeout(() => {
          exec(`osascript -e 'tell application "Google Chrome" to activate' 2>/dev/null || osascript -e 'tell application "Safari" to activate' 2>/dev/null || osascript -e 'tell application "Firefox" to activate' 2>/dev/null || osascript -e 'tell application "Brave Browser" to activate' 2>/dev/null`, (activateError) => {
            if (activateError) {
              this.log('⚠️ Could not activate browser (non-fatal)');
            }
          });
        }, 500);
      });
    } else if (platform === 'win32') {
      // Windows: start command brings window to front by default
      exec(`start "" "${url}"`, (error: Error | null) => {
        if (error) {
          this.log(`⚠️ Failed to open browser: ${error.message}`);
        }
      });
    } else {
      // Linux: xdg-open
      exec(`xdg-open "${url}"`, (error: Error | null) => {
        if (error) {
          this.log(`⚠️ Failed to open browser: ${error.message}`);
        }
      });
    }
  }

  private async startBridge(): Promise<void> {
    if (!this.claudeServer || !this.serverTransport) {
      throw new Error('Server or transport not initialized');
    }

    // CRITICAL: Intercept batch requests at the ReadBuffer level BEFORE schema validation
    // The MCP SDK's JSONRPCMessageSchema does not support arrays, so batch requests
    // are rejected during deserialization. We need to intercept the raw buffer processing.
    const transport = this.serverTransport as any;  // Bypass TypeScript access control
    const originalProcessReadBuffer = transport.processReadBuffer.bind(this.serverTransport);
    transport.processReadBuffer = function (this: any) {
      const readBuffer = this._readBuffer;
      if (!readBuffer || !readBuffer._buffer) {
        return;
      }

      // Check for newline
      const index = readBuffer._buffer.indexOf('\n');
      if (index === -1) {
        return;
      }

      // Extract the line
      const line = readBuffer._buffer.toString('utf8', 0, index).replace(/\r$/, '');
      readBuffer._buffer = readBuffer._buffer.subarray(index + 1);

      try {
        const parsed = JSON.parse(line);

        // Handle batch requests specially (arrays)
        if (Array.isArray(parsed)) {
          // Trigger our onmessage handler directly with the array
          if (this.onmessage) {
            this.onmessage(parsed);
          }
          return;
        }

        // For non-batch messages, use the original processing
        // Put the line back in the buffer for normal processing
        readBuffer._buffer = Buffer.concat([
          Buffer.from(line + '\n'),
          readBuffer._buffer
        ]);
        originalProcessReadBuffer();
      } catch (error) {
        // JSON parse error - let original handler deal with it
        readBuffer._buffer = Buffer.concat([
          Buffer.from(line + '\n'),
          readBuffer._buffer
        ]);
        originalProcessReadBuffer();
      }
    };

    // Start the stdio server for Claude Desktop
    await this.claudeServer.connect(this.serverTransport);

    // IMPORTANT: Intercept messages AFTER connect() to ensure our handler isn't overwritten
    // The Server.connect() sets up its own onmessage handler, so we need to wrap it
    const mcpServerOnMessage = this.serverTransport.onmessage;
    this.serverTransport.onmessage = (message: any) => {
      // Debug log to understand message format
      if (this.config.verbose) {
        this.log(`📨 Received message type: ${typeof message}, isArray: ${Array.isArray(message)}`);
      }

      // Handle server/info requests
      if (message.method === 'server/info' && message.id !== undefined) {
        this.log(`📊 Handling server/info request with ID: ${message.id}`);
        const response = {
          jsonrpc: '2.0' as const,
          id: message.id,
          result: {
            name: 'pierre-claude-bridge',
            version: '1.0.0',
            protocolVersion: '2025-06-18',
            supportedVersions: ['2024-11-05', '2025-03-26', '2025-06-18'],
            capabilities: {
              tools: {},
              resources: {},
              prompts: {},
              logging: {}
            }
          }
        };
        this.serverTransport!.send(response).catch((err: any) => {
          this.log(`❌ Failed to send server/info response: ${err.message}`);
        });
        return;
      }

      // Handle JSON-RPC batch requests (should be rejected in 2025-06-18)
      // Batch requests come as arrays at the JSON level, not after parsing
      if (Array.isArray(message)) {
        this.log(`🚫 Rejecting JSON-RPC batch request (${message.length} requests, not supported in 2025-06-18)`);
        this.log(`📦 Batch request IDs: ${message.map((r: any) => r.id).join(', ')}`);

        // For batch requests, the validator expects a JSON array response on a SINGLE line
        // Each request in the batch gets an individual error response
        const responses = message.map((req: any) => ({
          jsonrpc: '2.0' as const,
          id: req.id,
          error: {
            code: -32600,
            message: 'Batch requests are not supported in protocol version 2025-06-18'
          }
        }));

        this.log(`📤 Sending batch response array with ${responses.length} responses`);
        this.log(`📤 Response structure: ${JSON.stringify(responses).substring(0, 200)}...`);

        // The MCP SDK's send() method serializes objects/arrays and adds newline
        // For batch responses, we need to send the array itself, not individual items
        // Cast to any to bypass TypeScript type checking for the array
        this.serverTransport!.send(responses as any).then(() => {
          this.log(`✅ Batch response sent successfully`);
        }).catch((err: any) => {
          this.log(`❌ Failed to send batch rejection response: ${err.message}`);
        });
        return;
      }

      // Handle client/log notifications gracefully
      if (message.method === 'client/log' && message.id === undefined) {
        this.log(`📝 Client log [${message.params?.level}]: ${message.params?.message}`);
        return;
      }

      // Protect against malformed messages that crash the server
      try {
        // Forward other messages to the MCP Server handler
        if (mcpServerOnMessage) {
          mcpServerOnMessage(message);
        }
      } catch (error: any) {
        this.log(`⚠️ Error handling message: ${error.message}`);
        // Send error response if message had an ID
        if (message.id !== undefined) {
          const errorResponse = {
            jsonrpc: '2.0' as const,
            id: message.id,
            error: {
              code: -32603,
              message: `Internal error: ${error.message}`
            }
          };
          this.serverTransport!.send(errorResponse);
        }
      }
    };

    // Set up notification forwarding from Pierre to Claude
    this.setupNotificationForwarding();

    this.log('🚀 Bridge is running - Claude Desktop can now access Pierre Fitness tools');
  }

  private setupNotificationForwarding(): void {
    if (!this.pierreClient || !this.claudeServer) {
      return;
    }

    // Set up error handler for visibility
    this.pierreClient.onerror = (error) => {
      this.log('📢 Pierre client error:', error);
    };

    // Set up OAuth completion notification handler
    // Listen for OAuth completion notifications from Pierre server
    // and forward them to Claude Desktop so users see the success message
    try {
      this.pierreClient.setNotificationHandler(
        OAuthCompletedNotificationSchema,
        async (notification) => {
          this.log('🔔 Received OAuth completion notification from Pierre:', JSON.stringify(notification));

          if (this.claudeServer) {
            try {
              // Forward the notification to Claude Desktop
              await this.claudeServer.notification({
                method: 'notifications/message',
                params: {
                  level: 'info',
                  message: notification.params?.message || 'OAuth authentication completed successfully!'
                }
              });
              this.log('✅ Forwarded OAuth notification to Claude Desktop');
            } catch (error: any) {
              this.log('❌ Failed to forward OAuth notification to Claude:', error.message);
            }
          }
        }
      );
      this.log('📡 OAuth notification handler registered');
    } catch (error: any) {
      this.log('⚠️ Failed to set up OAuth notification handler:', error.message);
    }

    this.log('📡 Notification forwarding configured');
  }

  async stop(): Promise<void> {
    this.log('🛑 Stopping bridge...');

    try {
      if (this.pierreClient) {
        await this.pierreClient.close();
        this.pierreClient = null;
      }

      if (this.claudeServer) {
        await this.claudeServer.close();
        this.claudeServer = null;
      }

      this.log('✅ Bridge stopped');
    } catch (error) {
      this.log('❌ Error stopping bridge:', error);
      throw error;
    }
  }
}