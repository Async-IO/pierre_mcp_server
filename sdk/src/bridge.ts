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
  McpError
} from '@modelcontextprotocol/sdk/types.js';
import {
  OAuthClientMetadata,
  OAuthClientInformation,
  OAuthTokens,
  AuthorizationServerMetadata,
  OAuthClientInformationFull
} from '@modelcontextprotocol/sdk/shared/auth.js';

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
    return `http://localhost:${this.callbackPort || 35536}/oauth/callback`;
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
        this.savedTokens = {
          access_token: storedTokens.access_token,
          refresh_token: storedTokens.refresh_token,
          expires_in: storedTokens.expires_in,
          token_type: storedTokens.token_type || 'Bearer',
          scope: storedTokens.scope
        };
        console.error(`[Pierre OAuth] Reloaded valid tokens from storage`);
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

  async redirectToAuthorization(authorizationUrl: URL): Promise<void> {
    console.error(`[Pierre OAuth] Starting OAuth 2.0 authorization flow`);

    // Start callback server to receive authorization response
    await this.startCallbackServer();

    console.error(`[Pierre OAuth] Opening browser for authorization`);

    // Open the authorization URL in the user's default browser
    const { spawn } = await import('child_process');
    const platform = process.platform;

    let openCommand: string;
    if (platform === 'darwin') {
      openCommand = 'open';
    } else if (platform === 'win32') {
      openCommand = 'start';
    } else {
      openCommand = 'xdg-open';
    }

    spawn(openCommand, [authorizationUrl.toString()], { detached: true, stdio: 'ignore' });

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

  private startCallbackServerSync(): void {
    // This is a synchronous wrapper that starts the server immediately
    // The actual server setup is done synchronously with a temporary port assignment
    const http = require('http');

    if (this.callbackServer) {
      return; // Already started
    }

    // Create server immediately to get port
    this.callbackServer = http.createServer();
    this.callbackServer.listen(0, 'localhost', () => {
      this.callbackPort = this.callbackServer.address().port;
      console.error(`[Pierre OAuth] Early callback server listening on http://localhost:${this.callbackPort}/oauth/callback`);
    });

    // Set up the actual request handler
    this.setupCallbackHandler();
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
        } else if (parsedUrl.pathname === '/oauth/focus-recovery' && req.method === 'POST') {
          // Handle focus recovery request from OAuth success page
          console.error(`[Pierre OAuth] Focus recovery requested - attempting to focus Claude Desktop`);

          try {
            await this.focusClaudeDesktop();
            res.writeHead(200, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ success: true, message: 'Focus recovery attempted' }));
          } catch (error) {
            console.error(`[Pierre OAuth] Focus recovery failed: ${error}`);
            res.writeHead(200, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ success: false, message: 'Focus recovery failed' }));
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
        <p>You can now close this window and return to Claude Desktop.</p>
        <p><small>Attempting to return focus to Claude Desktop automatically...</small></p>
        <script>
            // Attempt to focus Claude Desktop before closing
            async function focusClaudeDesktop() {
                try {
                    // Try to trigger focus recovery via bridge communication
                    await fetch('/oauth/focus-recovery', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ action: 'focus_claude_desktop' })
                    }).catch(() => {
                        // Ignore fetch errors - focus recovery is best-effort
                    });
                } catch (error) {
                    // Silently ignore errors
                }

                // Close the window after a short delay
                setTimeout(() => {
                    window.close();
                }, 1500);
            }

            // Start focus recovery immediately
            focusClaudeDesktop();
        </script>
    </div>
</body>
</html>
    `;
  }

  private async focusClaudeDesktop(): Promise<void> {
    console.error(`[Pierre OAuth] Attempting to focus Claude Desktop application`);

    const { spawn } = await import('child_process');
    const platform = process.platform;

    try {
      let focusCommand: string[];

      if (platform === 'darwin') {
        // macOS - Use AppleScript to activate Claude Desktop
        focusCommand = [
          'osascript',
          '-e',
          'tell application "Claude" to activate'
        ];
      } else if (platform === 'win32') {
        // Windows - Use PowerShell to bring Claude Desktop to foreground
        focusCommand = [
          'powershell',
          '-Command',
          'Add-Type -AssemblyName Microsoft.VisualBasic; [Microsoft.VisualBasic.Interaction]::AppActivate("Claude")'
        ];
      } else {
        // Linux - Use wmctrl if available, otherwise xdotool
        focusCommand = [
          'bash',
          '-c',
          'if command -v wmctrl >/dev/null 2>&1; then wmctrl -a "Claude"; elif command -v xdotool >/dev/null 2>&1; then xdotool search --name "Claude" windowactivate; fi'
        ];
      }

      console.error(`[Pierre OAuth] Executing focus command for ${platform}`);

      // Execute the focus command
      const focusProcess = spawn(focusCommand[0], focusCommand.slice(1), {
        detached: false,
        stdio: 'ignore',
        timeout: 5000 // 5 second timeout
      });

      // Wait for the process to complete (with timeout)
      await new Promise<void>((resolve) => {
        const timer = setTimeout(() => {
          focusProcess.kill('SIGTERM');
          resolve();
        }, 5000);

        focusProcess.on('close', () => {
          clearTimeout(timer);
          resolve();
        });

        focusProcess.on('error', (error) => {
          clearTimeout(timer);
          console.error(`[Pierre OAuth] Focus command error (ignored): ${error.message}`);
          resolve(); // Don't fail - focus recovery is best-effort
        });
      });

      console.error(`[Pierre OAuth] Focus recovery command completed`);

    } catch (error: any) {
      console.error(`[Pierre OAuth] Focus recovery failed (ignored): ${error.message}`);
      // Don't throw - focus recovery is best-effort and shouldn't break the OAuth flow
    }
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
        <p>You can close this window and try connecting again from Claude Desktop.</p>
        <p><small>Returning focus to Claude Desktop...</small></p>
        <script>
            // Attempt to focus Claude Desktop before closing (even on error)
            async function focusClaudeDesktop() {
                try {
                    // Try to trigger focus recovery via bridge communication
                    await fetch('/oauth/focus-recovery', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ action: 'focus_claude_desktop' })
                    }).catch(() => {
                        // Ignore fetch errors - focus recovery is best-effort
                    });
                } catch (error) {
                    // Silently ignore errors
                }

                // Close the window after a longer delay for error cases
                setTimeout(() => {
                    window.close();
                }, 3000);
            }

            // Start focus recovery immediately
            focusClaudeDesktop();
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
      // Step 1: Create MCP client connection to Pierre using Streamable HTTP
      await this.connectToPierre();

      // Step 2: Create MCP server for Claude Desktop using stdio
      await this.createClaudeServer();

      // Step 3: Start the bridge
      await this.startBridge();

      this.log('✅ Bridge started successfully');
    } catch (error) {
      this.log('❌ Failed to start bridge:', error);
      throw error;
    }
  }

  private oauthProvider: PierreOAuthClientProvider | null = null;
  private mcpUrl: string = '';

  private async connectToPierre(): Promise<void> {
    this.log('🔌 Setting up Pierre MCP Server connection...');

    this.mcpUrl = `${this.config.pierreServerUrl}/mcp`;
    this.log(`📡 Target URL: ${this.mcpUrl}`);

    // Create OAuth client provider for Pierre MCP Server (shared across attempts)
    this.oauthProvider = new PierreOAuthClientProvider(this.config.pierreServerUrl, this.config);

    this.log('🔐 OAuth provider initialized - connection deferred until authentication');

    // Check if we have existing valid tokens
    const existingTokens = await this.oauthProvider.tokens();
    if (existingTokens) {
      this.log('🎫 Found existing tokens - attempting connection');
      await this.attemptConnection();
    } else {
      this.log('⏳ No valid tokens found - connection will be established when connect_to_pierre is called');
    }
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
          authProvider: this.oauthProvider,
          requestInit: {
            headers: {
              'Content-Type': 'application/json',
              'User-Agent': 'Pierre-Claude-Bridge/1.0.0'
            }
          }
        });

        // Connect to Pierre MCP Server with OAuth 2.0 flow
        await this.pierreClient.connect(transport);
        connected = true;
        this.log('✅ Connected to Pierre MCP Server with OAuth 2.0');
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
      this.log(`📋 pierreClient exists: ${!!this.pierreClient}`);

      try {
        if (this.pierreClient) {
          // If connected, forward the request through the pierreClient
          // The client already has OAuth authentication built into its transport
          this.log('📋 Forwarding tools/list to Pierre via authenticated client');
          const result = await this.pierreClient.listTools();
          this.log(`📋 Received ${result.tools.length} tools from Pierre`);
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
      if (!this.pierreClient) {
        return {
          content: [{
            type: 'text',
            text: 'Not connected to Pierre. Please use the "Connect to Pierre" tool first to authenticate.'
          }],
          isError: true
        };
      }

      try {
        this.log(`🔄 Forwarding tool call ${request.params.name} to Pierre server...`);
        // Use callTool() instead of request() - Client.request() is for raw JSON-RPC,
        // but we want the higher-level callTool() method which handles the protocol correctly
        const result = await this.pierreClient.callTool({
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

      // Check if already connected
      if (this.pierreClient) {
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
            arguments: {}
          });

          // Check if the provider is already connected
          if (connectionStatus && connectionStatus.content && Array.isArray(connectionStatus.content)) {
            const statusContent = connectionStatus.content[0];
            if (statusContent && statusContent.type === 'text' && 'text' in statusContent) {
              const statusText = statusContent.text;
              // Look for provider connection status in the response
              const providerConnected = statusText.toLowerCase().includes(`${provider}:`) &&
                                       statusText.toLowerCase().includes('connected: true');

              if (providerConnected) {
                this.log(`✅ ${provider} is already connected - no OAuth needed`);
                return {
                  content: [{
                    type: 'text',
                    text: `Already connected to ${provider.toUpperCase()}! You can now access your ${provider} fitness data.`
                  }],
                  isError: false
                };
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

        // Open provider OAuth in browser
        const { spawn } = await import('child_process');
        const platform = process.platform;

        let openCommand: string;
        if (platform === 'darwin') {
          openCommand = 'open';
        } else if (platform === 'win32') {
          openCommand = 'start';
        } else {
          openCommand = 'xdg-open';
        }

        spawn(openCommand, [providerOAuthUrl], { detached: true, stdio: 'ignore' });

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

  private async startBridge(): Promise<void> {
    if (!this.claudeServer || !this.serverTransport) {
      throw new Error('Server or transport not initialized');
    }

    // Start the stdio server for Claude Desktop
    await this.claudeServer.connect(this.serverTransport);

    // Set up notification forwarding from Pierre to Claude
    this.setupNotificationForwarding();

    this.log('🚀 Bridge is running - Claude Desktop can now access Pierre Fitness tools');
  }

  private setupNotificationForwarding(): void {
    if (!this.pierreClient || !this.claudeServer) {
      return;
    }

    // Note: The official MCP SDK handles notifications and progress automatically
    // through the client/server connection. Manual forwarding may not be needed
    // for basic bridging scenarios, but we set up error handlers for visibility.

    this.pierreClient.onerror = (error) => {
      this.log('📢 Pierre client error:', error);
    };

    this.log('📡 Notification forwarding configured (automatic via MCP protocol)');
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