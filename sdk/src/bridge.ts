// ABOUTME: Bidirectional MCP bridge connecting MCP host (stdio) to Pierre Server (HTTP)
// ABOUTME: Manages OAuth 2.0 flows, token persistence, and transparent MCP message translation
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

/**
 * Pierre MCP Client
 *
 * MCP-compliant client implementation connecting MCP hosts to Pierre MCP Server (HTTP + OAuth 2.0)
 */

import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { StreamableHTTPClientTransport } from '@modelcontextprotocol/sdk/client/streamableHttp.js';
import { OAuthClientProvider } from '@modelcontextprotocol/sdk/client/auth.js';
import { readFileSync } from 'fs';
import { join } from 'path';
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
import { createSecureStorage, SecureTokenStorage } from './secure-storage.js';

// Load OAuth HTML templates
// __dirname is available in CommonJS, TypeScript will compile this correctly
const OAUTH_SUCCESS_TEMPLATE = readFileSync(join(__dirname, '../templates/oauth_success.html'), 'utf-8');
const OAUTH_ERROR_TEMPLATE = readFileSync(join(__dirname, '../templates/oauth_error.html'), 'utf-8');

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
  tokenValidationTimeoutMs?: number;  // Default: 3000ms
  proactiveConnectionTimeoutMs?: number;  // Default: 5000ms
  proactiveToolsListTimeoutMs?: number;  // Default: 3000ms
  toolCallConnectionTimeoutMs?: number;  // Default: 10000ms (10s for tool-triggered connections)
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

interface ValidateRefreshResponse {
  status: 'valid' | 'refreshed' | 'invalid';
  expires_in?: number;
  access_token?: string;
  refresh_token?: string;
  token_type?: string;
  reason?: string;
  requires_full_reauth?: boolean;
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
  private callbackSessionToken: string | undefined = undefined;

  // Secure token storage using OS keychain
  private secureStorage: SecureTokenStorage | undefined = undefined;
  private allStoredTokens: StoredTokens = {};

  // Client-side client info storage (client info is not sensitive, can stay in file)
  private clientInfoPath: string;

  constructor(serverUrl: string, config: BridgeConfig) {
    this.serverUrl = serverUrl;
    this.config = config;

    // Initialize client info storage path
    const os = require('os');
    const path = require('path');
    this.clientInfoPath = path.join(os.homedir(), '.pierre-mcp-client-info.json');

    // NOTE: Secure storage initialization is async, so it's deferred to start()
    // to avoid race conditions with constructor completion
    // See initializePierreConnection() for the actual initialization

    // Load client info from storage (synchronous, non-sensitive)
    this.loadClientInfo();

    this.log(`OAuth client provider created for server: ${serverUrl}`);
    this.log(`Using OS keychain for secure token storage (will initialize on start)`);
    this.log(`Client info storage path: ${this.clientInfoPath}`);
  }

  public async initializeSecureStorage(): Promise<void> {
    try {
      this.secureStorage = await createSecureStorage(this.log.bind(this));
      // Load existing tokens from keychain
      await this.loadStoredTokens();
      this.log('Secure storage initialized with OS keychain');
    } catch (error) {
      this.log(`Failed to initialize secure storage: ${error}`);
      this.log('WARNING: Token storage will not be available');
    }
  }

  private log(message: string, ...args: any[]): void {
    const timestamp = new Date().toISOString();
    console.error(`[${timestamp}] [Pierre OAuth] ${message}`, ...args);
  }

  // Client-side token storage methods using secure keychain
  private async loadStoredTokens(): Promise<void> {
    try {
      if (!this.secureStorage) {
        this.log('Secure storage not initialized, skipping token load');
        return;
      }

      const tokens = await this.secureStorage.getTokens();
      if (tokens) {
        this.allStoredTokens = tokens;

        // Load Pierre tokens into memory for MCP SDK compatibility
        if (this.allStoredTokens.pierre) {
          this.savedTokens = this.allStoredTokens.pierre;
          this.log(`Loaded Pierre tokens from keychain`);
        }

        if (this.allStoredTokens.providers) {
          const providerCount = Object.keys(this.allStoredTokens.providers).length;
          this.log(`Loaded ${providerCount} provider token(s) from keychain`);
        }
      } else {
        this.log(`No stored tokens found in keychain, starting fresh`);
      }
    } catch (error) {
      this.log(`Failed to load stored tokens: ${error}`);
      this.allStoredTokens = {};
    }
  }

  private loadClientInfo(): void {
    try {
      const fs = require('fs');
      if (fs.existsSync(this.clientInfoPath)) {
        const data = fs.readFileSync(this.clientInfoPath, 'utf8');
        this.clientInfo = JSON.parse(data);
        this.log(`Loaded client info from storage: ${this.clientInfo?.client_id}`);
      } else {
        this.log(`No stored client info found, will perform dynamic registration on first OAuth`);
      }
    } catch (error) {
      this.log(`Failed to load client info: ${error}`);
      this.clientInfo = undefined;
    }
  }

  private saveClientInfoToFile(): void {
    if (!this.clientInfo) {
      return;
    }

    try {
      const fs = require('fs');
      fs.writeFileSync(this.clientInfoPath, JSON.stringify(this.clientInfo, null, 2), 'utf8');
      this.log(`Saved client info to disk: ${this.clientInfo.client_id}`);
    } catch (error) {
      this.log(`Failed to save client info: ${error}`);
    }
  }

  private async saveStoredTokens(): Promise<void> {
    try {
      if (!this.secureStorage) {
        this.log('Secure storage not initialized, cannot save tokens');
        return;
      }

      await this.secureStorage.saveTokens(this.allStoredTokens);
      this.log(`Saved tokens to OS keychain`);
    } catch (error) {
      this.log(`Failed to save tokens to keychain: ${error}`);
    }
  }

  async clearTokens(): Promise<void> {
    try {
      // Clear in-memory Pierre tokens
      this.savedTokens = undefined;
      delete this.allStoredTokens.pierre;

      // Clear from keychain if secure storage is available
      if (this.secureStorage) {
        await this.secureStorage.clearTokens();
        this.log(`Cleared all tokens from keychain`);
      }

      // Reset in-memory storage
      this.allStoredTokens = {};
    } catch (error) {
      this.log(`Failed to clear tokens: ${error}`);
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
    this.log(`Saved ${provider} provider token to client storage`);
  }

  getProviderToken(provider: string): any | undefined {
    const token = this.allStoredTokens.providers?.[provider];
    if (!token) {
      return undefined;
    }

    // Check if token is expired
    if (token.expires_at && Date.now() > token.expires_at) {
      this.log(`${provider} token expired, removing from storage`);
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
      client_name: 'Pierre MCP Client',
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
    this.log(`Registering client with Pierre OAuth server: ${clientInformation.client_id}`);

    // Register this client with Pierre's OAuth server
    await this.registerClientWithPierre(clientInformation);

    this.clientInfo = clientInformation;
    this.saveClientInfoToFile(); // Persist to disk
    this.log(`Saved client info to memory and disk: ${clientInformation.client_id}`);
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

    this.log(`Registering client at ${registrationEndpoint}`);

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
      this.log(`Client registration successful: ${JSON.stringify(registrationResponse)}`);

      // Update client info with the response from Pierre's server
      if (registrationResponse.client_id && registrationResponse.client_secret) {
        this.log(`Updating client info to use Pierre's returned client ID: ${registrationResponse.client_id}`);
        clientInfo.client_id = registrationResponse.client_id;
        clientInfo.client_secret = registrationResponse.client_secret;
      }

    } catch (error) {
      this.log(`Client registration failed: ${error}`);
      throw error;
    }
  }

  async saveTokens(tokens: OAuthTokens): Promise<void> {
    this.savedTokens = tokens;

    // Also save to persistent client-side storage
    this.allStoredTokens.pierre = { ...tokens, saved_at: Math.floor(Date.now() / 1000) };
    await this.saveStoredTokens();

    this.log(`Saved Pierre tokens: expires_in=${tokens.expires_in}`);
  }

  async tokens(): Promise<OAuthTokens | undefined> {
    // If no in-memory tokens, try to load from persistent storage
    if (!this.savedTokens && this.allStoredTokens.pierre) {
      this.log(`No in-memory tokens, attempting to reload from persistent storage`);

      const storedTokens = this.allStoredTokens.pierre;

      // Always validate with server using validate-and-refresh endpoint
      const validationResult = await this.validateAndRefreshToken(
        storedTokens.access_token,
        storedTokens.refresh_token
      );

      if (validationResult) {
        if (validationResult.status === 'valid') {
          // Token is valid, reload into memory
          this.savedTokens = {
            access_token: storedTokens.access_token,
            refresh_token: storedTokens.refresh_token,
            expires_in: storedTokens.expires_in,
            token_type: storedTokens.token_type || 'Bearer',
            scope: storedTokens.scope
          };
          this.log(`Your session is valid (expires in ${validationResult.expires_in}s)`);
        } else if (validationResult.status === 'refreshed') {
          // Token was refreshed, save new tokens
          this.log(`Your session was automatically renewed with a fresh token`);

          this.savedTokens = {
            access_token: validationResult.access_token!,
            refresh_token: validationResult.refresh_token,
            expires_in: validationResult.expires_in,
            token_type: validationResult.token_type || 'Bearer',
            scope: storedTokens.scope
          };

          // Update persistent storage with new tokens
          this.allStoredTokens.pierre = { ...this.savedTokens, saved_at: Math.floor(Date.now() / 1000) };
          await this.saveStoredTokens();
          this.log(`Session renewed successfully - you can continue using Pierre tools`);
        } else {
          // Token is invalid, clear storage and require full re-auth
          this.log(`Your session has expired and cannot be renewed automatically`);
          this.log(`Reason: ${validationResult.reason || 'Token validation failed'}`);
          this.log(`Please use the "Connect to Pierre" tool to re-authenticate`);
          delete this.allStoredTokens.pierre;
          await this.saveStoredTokens();
          this.savedTokens = undefined;
        }
      } else {
        // Validation request failed, clear storage to be safe
        this.log(`Unable to validate your session with Pierre server`);
        this.log(`Please use the "Connect to Pierre" tool to re-authenticate`);
        delete this.allStoredTokens.pierre;
        await this.saveStoredTokens();
        this.savedTokens = undefined;
      }
    }

    this.log(`tokens() called, returning tokens: ${this.savedTokens ? 'available' : 'none'}`);
    if (this.savedTokens) {
      this.log(`Token type: ${this.savedTokens.token_type}`);
    }
    return this.savedTokens;
  }

  private async validateAndRefreshToken(accessToken: string, refreshToken?: string): Promise<ValidateRefreshResponse | null> {
    try {
      this.log(`Validating token with server validate-and-refresh endpoint`);

      const timeoutMs = this.config.tokenValidationTimeoutMs || 3000;

      const fetchPromise = fetch(`${this.serverUrl}/oauth2/validate-and-refresh`, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${accessToken}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          refresh_token: refreshToken
        })
      });

      const response = await Promise.race([
        fetchPromise,
        new Promise<Response>((_, reject) =>
          setTimeout(() => reject(new Error(`Token validate-and-refresh timeout after ${timeoutMs}ms`)), timeoutMs)
        )
      ]);

      if (!response.ok) {
        this.log(`Validate-and-refresh request failed: ${response.status} ${response.statusText}`);
        return null;
      }

      const result = await response.json() as ValidateRefreshResponse;
      this.log(`Token validation status: ${result.status}`);

      if (result.status === 'invalid' && result.reason) {
        this.log(`Token invalid reason: ${result.reason}`);
      }

      return result;
    } catch (error) {
      this.log(`Validate-and-refresh request failed: ${error}`);
      return null;
    }
  }

  async redirectToAuthorization(authorizationUrl: URL): Promise<void> {
    this.log(`Starting OAuth 2.0 authorization flow`);

    // Start callback server to receive authorization response
    await this.startCallbackServer();

    this.log(`Opening browser for authorization`);

    // Open the authorization URL in the user's default browser with focus
    await this.openUrlInBrowserWithFocus(authorizationUrl.toString());

    this.log(`If browser doesn't open automatically, visit:`);
    this.log(`${authorizationUrl.toString()}`);
    this.log(`Waiting for authorization completion`);

    // Wait for authorization completion
    if (this.authorizationPending) {
      const authResult = await this.authorizationPending;
      this.log(`Authorization callback completed, exchanging code for tokens`);

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

    this.log(`Requesting JWT token from ${tokenEndpoint}`);

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
      this.log(`Successfully received JWT token, expires_in=${tokenResponse.expires_in}`);

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
      this.log(`Token exchange failed: ${error}`);
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
    this.log(`Invalidated credentials: ${scope}`);
  }

  async clearProviderTokens(): Promise<void> {
    this.allStoredTokens.providers = {};
    await this.saveStoredTokens();
    this.log(`Cleared all provider tokens from client storage`);
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

  private async generateCodeChallenge(codeVerifier: string): Promise<string> {
    // Generate SHA-256 hash of the code verifier
    const crypto = require('crypto');
    const hash = crypto.createHash('sha256').update(codeVerifier).digest();

    // Base64 URL encode the hash
    return hash.toString('base64')
      .replace(/\+/g, '-')
      .replace(/\//g, '_')
      .replace(/=/g, '');
  }

  private async openUrlInBrowserWithFocus(url: string): Promise<void> {
    const { exec } = await import('child_process');
    const platform = process.platform;

    if (platform === 'darwin') {
      // macOS: Open URL then explicitly activate browser after a brief delay
      exec(`open "${url}"`, (error: Error | null) => {
        if (error) {
          this.log(`Failed to open browser: ${error.message}`);
          return;
        }

        // After opening, try to activate common browsers
        setTimeout(() => {
          exec(`osascript -e 'tell application "Google Chrome" to activate' 2>/dev/null || osascript -e 'tell application "Safari" to activate' 2>/dev/null || osascript -e 'tell application "Firefox" to activate' 2>/dev/null || osascript -e 'tell application "Brave Browser" to activate' 2>/dev/null`, (activateError) => {
            if (activateError) {
              this.log(`Could not activate browser (non-fatal)`);
            }
          });
        }, 500);
      });
    } else if (platform === 'win32') {
      // Windows: start command brings window to front by default
      exec(`start "" "${url}"`, (error: Error | null) => {
        if (error) {
          this.log(`Failed to open browser: ${error.message}`);
        }
      });
    } else {
      // Linux: xdg-open
      exec(`xdg-open "${url}"`, (error: Error | null) => {
        if (error) {
          this.log(`Failed to open browser: ${error.message}`);
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

    // Set the port immediately so redirectUrl can use it
    // (server.listen is async but we need the port value synchronously)
    this.callbackPort = port;

    // Create server immediately with fixed port
    this.callbackServer = http.createServer();

    // Add error handler for port-in-use errors
    this.callbackServer.on('error', (error: any) => {
      if (error.code === 'EADDRINUSE') {
        this.log(`Port ${port} is already in use - likely from previous session`);
        this.log(`Attempting to use dynamic port assignment instead...`);

        // Clean up failed server
        this.callbackServer?.close();
        this.callbackServer = null;

        // Retry with dynamic port (OS will assign available port)
        this.callbackServer = http.createServer();
        this.callbackServer.listen(0, 'localhost', () => {
          this.callbackPort = this.callbackServer.address().port;
          this.log(`Callback server listening on http://localhost:${this.callbackPort}/oauth/callback`);
          this.setupCallbackHandler();
        });
      } else {
        this.log(`Failed to start callback server:`, error);
        throw error;
      }
    });

    this.callbackServer.listen(port, 'localhost', () => {
      // Port already set above, just confirm it matches
      const actualPort = this.callbackServer.address().port;
      if (actualPort !== this.callbackPort) {
        this.log(`Warning: Server started on ${actualPort} but expected ${this.callbackPort}`);
      }
      this.log(`Callback server listening on http://localhost:${this.callbackPort}/oauth/callback`);
      // Set up the actual request handler
      this.setupCallbackHandler();
    });
  }

  private setupCallbackHandler(): void {
    if (!this.callbackServer) return;

    // Generate a random session token for callback authentication
    const crypto = require('crypto');
    this.callbackSessionToken = crypto.randomBytes(32).toString('hex');
    this.log(`Generated callback session token for authentication`);

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
            this.log(`Authorization failed: ${query.error}`);
            res.writeHead(400, { 'Content-Type': 'text/html' });
            res.end(this.renderErrorTemplate(
              'MCP host OAuth',
              `${query.error}`,
              `${query.error_description || 'Please try connecting again.'}`
            ));
          } else if (query.code && query.state) {
            this.log(`Authorization successful, received code`);
            res.writeHead(200, { 'Content-Type': 'text/html' });
            res.end(this.renderSuccessTemplate('MCP host OAuth'));

            // Resolve the authorization promise if it exists
            const authResolve = (this.callbackServer as any)._authResolve;
            if (authResolve) {
              authResolve({ code: query.code, state: query.state });
            }

            // Authorization result is handled via promise resolution in redirectToAuthorization()
          } else {
            this.log(`Invalid callback parameters`);
            res.writeHead(400, { 'Content-Type': 'text/html' });
            res.end(this.renderErrorTemplate(
              'MCP host OAuth',
              'Invalid Request',
              'Missing required authorization parameters. Please try connecting again.'
            ));
          }
        } else if (parsedUrl.pathname?.startsWith('/oauth/provider-callback/') && req.method === 'POST') {
          // Handle provider token callback for client-side storage
          const pathParts = parsedUrl.pathname.split('/');
          const provider = pathParts[3]; // /oauth/provider-callback/{provider}

          // Security: Validate session token to prevent local CSRF attacks
          const sessionToken = parsedUrl.query.session_token || req.headers['x-session-token'];
          if (sessionToken !== this.callbackSessionToken) {
            this.log(`Rejected POST callback for ${provider}: Invalid session token`);
            res.writeHead(403, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({
              success: false,
              message: 'Invalid session token - authentication required'
            }));
            return;
          }

          // Security: Validate Host header (localhost only)
          const host = req.headers.host;
          if (!host || !(host.startsWith('localhost') || host.startsWith('127.0.0.1'))) {
            this.log(`Rejected POST callback for ${provider}: Invalid host ${host}`);
            res.writeHead(403, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({
              success: false,
              message: 'Invalid host - only localhost allowed'
            }));
            return;
          }

          this.log(`Provider token callback for ${provider}`);

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
              this.log(`Failed to save ${provider} token: ${error}`);
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
        this.log(`Callback server error: ${error}`);
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
    return OAUTH_SUCCESS_TEMPLATE.replace(/\{\{PROVIDER\}\}/g, provider);
  }

  private renderErrorTemplate(provider: string, error: string, description: string): string {
    return OAUTH_ERROR_TEMPLATE
      .replace(/\{\{PROVIDER\}\}/g, provider)
      .replace(/\{\{ERROR\}\}/g, error)
      .replace(/\{\{DESCRIPTION\}\}/g, description);
  }

  async validateAndCleanupCachedCredentials(): Promise<void> {
    const existingTokens = await this.tokens();
    const clientInfo = await this.clientInformation();

    if (!existingTokens && !clientInfo) {
      this.log('No cached credentials found - fresh start');
      return;
    }

    const timeoutMs = this.config.tokenValidationTimeoutMs || 3000;
    this.log(`Validating cached credentials with server (timeout: ${timeoutMs}ms)...`);

    // Try to validate by calling the token validation endpoint with configurable timeout
    try {
      const fetchPromise = fetch(`${this.serverUrl}/oauth2/token-validate`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          ...(existingTokens ? { 'Authorization': `Bearer ${existingTokens.access_token}` } : {})
        },
        body: JSON.stringify({
          client_id: clientInfo?.client_id
        })
      });

      const response = await Promise.race([
        fetchPromise,
        new Promise<Response>((_, reject) =>
          setTimeout(() => reject(new Error(`Token validation timeout after ${timeoutMs}ms`)), timeoutMs)
        )
      ]);

      const result: any = await response.json();

      if (result.valid === false) {
        this.log(`Cached credentials are invalid: ${result.error || 'unknown error'}`);
        this.log('Cleaning up invalid cached credentials...');

        // Clear invalid tokens from keychain
        if (existingTokens && this.secureStorage) {
          await this.secureStorage.clearTokens();
          this.log('Cleared invalid tokens from keychain');
        }

        // Clear invalid client info
        if (clientInfo) {
          const fs = require('fs');
          if (fs.existsSync(this.clientInfoPath)) {
            fs.unlinkSync(this.clientInfoPath);
            this.log('Cleared invalid client registration');
          }
        }

        // Reset in-memory state
        this.savedTokens = undefined;
        this.clientInfo = undefined;
        this.allStoredTokens = {};
      } else {
        this.log('Cached credentials are valid');
      }
    } catch (error: any) {
      this.log(`Failed to validate credentials: ${error.message}`);
      this.log('Will proceed with cached credentials and handle errors during connection');
    }
  }

}

export class PierreMcpClient {
  private config: BridgeConfig;
  private pierreClient: Client | null = null;
  private mcpServer: Server | null = null;
  private serverTransport: StdioServerTransport | null = null;
  private cachedTools: any = null;

  constructor(config: BridgeConfig) {
    this.config = config;
  }

  private log(message: string, ...args: any[]) {
    const timestamp = new Date().toISOString();
    console.error(`[${timestamp}] [Pierre Bridge] ${message}`, ...args);
  }

  private async withTimeout<T>(promise: Promise<T>, timeoutMs: number, operation: string): Promise<T | null> {
    return Promise.race([
      promise,
      new Promise<null>((resolve) =>
        setTimeout(() => {
          this.log(`Operation '${operation}' timed out after ${timeoutMs}ms`);
          resolve(null);
        }, timeoutMs)
      )
    ]);
  }

  async start(): Promise<void> {
    try {
      // Step 1: Create MCP server for MCP host using stdio
      // This must happen FIRST so the bridge can respond to MCP validator
      await this.createMcpServer();

      // Step 2: Start the bridge (stdio transport)
      await this.startBridge();

      // Step 3: Create MCP client connection to Pierre using Streamable HTTP
      // Initialize in background so MCP server can respond immediately (critical for CI/validators)
      // Connection will complete asynchronously; tools will be available once connected
      this.initializePierreConnection().catch(error => {
        this.log('Pierre connection initialization failed (will retry on first tool call):', error);
      });

      this.log('Bridge started successfully (Pierre connection initializing in background)');
    } catch (error) {
      this.log('Failed to start bridge:', error);
      throw error;
    }
  }

  private async initializePierreConnection(): Promise<void> {
    // Set up Pierre connection parameters
    this.mcpUrl = `${this.config.pierreServerUrl}/mcp`;
    this.oauthProvider = new PierreOAuthClientProvider(this.config.pierreServerUrl, this.config);

    // Initialize secure storage before any operations that might need it
    await this.oauthProvider.initializeSecureStorage();
    this.log(`Pierre MCP URL configured: ${this.mcpUrl}`);

    // Validate cached tokens and client registration at startup
    // This prevents wasting user time with invalid credentials
    await this.oauthProvider.validateAndCleanupCachedCredentials();

    // ALWAYS connect proactively to cache tools for MCP host
    // Server allows tools/list without authentication - only tool calls require auth
    // This ensures all tools are visible immediately in MCP host (tools/list_changed doesn't work)
    const connectionTimeoutMs = this.config.proactiveConnectionTimeoutMs || 5000;
    const toolsListTimeoutMs = this.config.proactiveToolsListTimeoutMs || 3000;

    try {
      this.log(`Connecting to Pierre proactively to cache all tools for MCP host (timeout: ${connectionTimeoutMs}ms)`);
      const connectionResult = await this.withTimeout(
        this.connectToPierre(),
        connectionTimeoutMs,
        'proactive Pierre connection'
      );

      if (connectionResult === null) {
        // Connection timed out - this is non-fatal for the bridge
        this.log(`Proactive connection timed out after ${connectionTimeoutMs}ms - will connect on first tool use`);
        this.log('Bridge will start with connect_to_pierre tool only');
        return;
      }

      // Cache tools immediately so they're ready for tools/list
      if (this.pierreClient) {
        const client = this.pierreClient;
        const toolsResult = await this.withTimeout(
          client.listTools(),
          toolsListTimeoutMs,
          'proactive tools list'
        );

        if (toolsResult) {
          this.cachedTools = toolsResult;
          this.log(`Cached ${toolsResult.tools.length} tools from Pierre: ${JSON.stringify(toolsResult.tools.map((t: any) => t.name))}`);
        } else {
          this.log(`Tools list timed out after ${toolsListTimeoutMs}ms - will fetch on first request`);
        }
      }
    } catch (error: any) {
      // If proactive connection fails, continue anyway
      // The bridge should still start - provide minimal toolset
      this.log(`Proactive connection failed: ${error.message}`);
      this.log('Bridge will start with connect_to_pierre tool only');
      // Don't propagate error - bridge should start successfully
    }
  }

  private async ensurePierreConnected(): Promise<void> {
    if (this.pierreClient) {
      return; // Already connected
    }

    const connectionTimeoutMs = this.config.toolCallConnectionTimeoutMs || 10000;
    this.log(`Connecting to Pierre MCP Server (timeout: ${connectionTimeoutMs}ms)...`);

    const connectionResult = await this.withTimeout(
      this.connectToPierre(),
      connectionTimeoutMs,
      'tool-triggered Pierre connection'
    );

    if (connectionResult === null) {
      throw new Error(`Failed to connect to Pierre within ${connectionTimeoutMs}ms. Please use the "Connect to Pierre" tool to establish a connection.`);
    }
  }

  private oauthProvider: PierreOAuthClientProvider | null = null;
  private mcpUrl: string = '';

  private async connectToPierre(): Promise<void> {
    this.log('Connecting to Pierre MCP Server...');

    if (!this.oauthProvider) {
      throw new Error('OAuth provider not initialized - call initializePierreConnection() first');
    }

    this.log(`Target URL: ${this.mcpUrl}`);

    // Always attempt connection to discover tools (initialize and tools/list don't require auth)
    // If tokens exist, the connection will be fully authenticated
    // If no tokens, we can still discover tools but tool calls will require authentication via connect_to_pierre
    const existingTokens = await this.oauthProvider.tokens();
    if (existingTokens) {
      this.log('Found existing tokens - connecting with authentication');
    } else {
      this.log('No tokens found - connecting without authentication to discover tools');
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
            name: 'pierre-mcp-client',
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

        // Check if we have authentication tokens BEFORE creating transport
        // This prevents the SDK from triggering interactive OAuth flow
        const hasTokens = !!(this.oauthProvider as any).savedTokens;

        // Create fresh transport for each attempt
        const baseUrl = new URL(this.mcpUrl);
        const transport = new StreamableHTTPClientTransport(baseUrl, {
          // CRITICAL: Only provide authProvider if we have tokens
          // If we don't have tokens, connect without auth (tools/list works unauthenticated)
          authProvider: hasTokens ? this.oauthProvider : undefined
        });

        // Connect to Pierre MCP Server
        await this.pierreClient.connect(transport);

        // CRITICAL: Validate that the MCP handshake completed successfully
        // The server MUST respond to initialize with proper JSON-RPC, not custom SSE events
        // This catches servers that send "event:connected" or other non-MCP messages
        try {
          this.log('Validating MCP protocol handshake with ping...');
          const pingTimeout = 5000; // 5 second timeout for validation
          const pingResult = await Promise.race([
            this.pierreClient.ping(),
            new Promise((_, reject) =>
              setTimeout(() => reject(new Error('MCP ping timeout - server may not be responding to JSON-RPC requests')), pingTimeout)
            )
          ]);
          this.log('MCP protocol validation successful - server is responding to JSON-RPC requests');
        } catch (validationError: any) {
          this.log(`MCP protocol validation FAILED: ${validationError.message}`);
          this.log('Server may be sending invalid SSE events (e.g., "event:connected") instead of JSON-RPC messages');
          throw new Error(`MCP protocol validation failed: ${validationError.message}. Server must send only JSON-RPC messages over SSE, not custom events.`);
        }

        connected = true;

        if (hasTokens) {
          this.log('Connected to Pierre MCP Server (authenticated)');
        } else {
          this.log('Connected to Pierre MCP Server (unauthenticated - tool discovery only)');
          this.log('Use "Connect to Pierre" tool to authenticate and access your fitness data');
        }
        this.log(`pierreClient is now set: ${!!this.pierreClient}`);
      } catch (error: any) {
        if (error.message === 'Unauthorized' && retryCount < maxRetries - 1) {
          retryCount++;
          this.log(`Token expired or invalid, retrying... (attempt ${retryCount}/${maxRetries})`);

          // Clear invalid tokens
          await this.oauthProvider.invalidateCredentials('tokens');

          await new Promise(resolve => setTimeout(resolve, 1000));
        } else {
          this.log(`Failed to connect after ${retryCount + 1} attempts: ${error.message}`);
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

    this.log('Initiating OAuth connection to Pierre MCP Server');

    // Check if we already have tokens
    const existingTokens = await this.oauthProvider.tokens();

    if (!existingTokens) {
      this.log('No tokens found - starting OAuth 2.0 authorization flow');

      // Manually trigger OAuth flow by building authorization URL and redirecting
      try {
        // Step 1: Ensure client is registered (dynamic client registration)
        let clientInfo = await this.oauthProvider.clientInformation();

        // Get client metadata for redirect URI (needed for both new and existing clients)
        const clientMetadata = this.oauthProvider['clientMetadata'];

        if (!clientInfo) {
          this.log('No client info found - performing dynamic client registration');

          // Generate new client credentials
          const crypto = require('crypto');
          const clientId = `pierre-bridge-${crypto.randomBytes(8).toString('hex')}`;
          const clientSecret = crypto.randomBytes(32).toString('hex');

          const fullClientInfo: OAuthClientInformationFull = {
            client_id: clientId,
            client_secret: clientSecret,
            redirect_uris: clientMetadata.redirect_uris,
            grant_types: clientMetadata.grant_types,
            response_types: clientMetadata.response_types,
            scope: clientMetadata.scope,
            client_name: clientMetadata.client_name,
            client_uri: clientMetadata.client_uri,
            client_id_issued_at: Math.floor(Date.now() / 1000),
            client_secret_expires_at: 0  // Never expires
          };

          // Save and register the client (this updates clientInfo with Pierre's assigned client_id)
          await this.oauthProvider.saveClientInformation(fullClientInfo);

          // Re-fetch client information to get the server-assigned client_id
          clientInfo = await this.oauthProvider.clientInformation();
          if (!clientInfo) {
            throw new Error('Client registration failed - no client info after registration');
          }

          this.log(`Dynamic client registration complete: ${clientInfo.client_id}`);
        }

        // Step 2: Get redirect URI
        const redirectUri = clientMetadata.redirect_uris[0];

        // Step 3: Generate PKCE values
        const state = await this.oauthProvider.state();
        const codeVerifier = this.oauthProvider['generateRandomString'](64);
        await this.oauthProvider.saveCodeVerifier(codeVerifier);

        const codeChallenge = await this.oauthProvider['generateCodeChallenge'](codeVerifier);

        // Step 4: Build authorization URL
        const authUrl = new URL(`${this.config.pierreServerUrl}/oauth2/authorize`);
        authUrl.searchParams.set('client_id', clientInfo.client_id);
        authUrl.searchParams.set('redirect_uri', redirectUri);
        authUrl.searchParams.set('response_type', 'code');
        authUrl.searchParams.set('state', state);
        authUrl.searchParams.set('code_challenge', codeChallenge);
        authUrl.searchParams.set('code_challenge_method', 'S256');
        authUrl.searchParams.set('scope', 'read:fitness write:fitness');

        // Step 5: Redirect to authorization (opens browser)
        await this.oauthProvider.redirectToAuthorization(authUrl);

        // Step 6: Connect after OAuth completes
        await this.attemptConnection();
      } catch (error) {
        this.log(`Failed to start OAuth flow: ${error}`);
        throw error;
      }
    } else {
      this.log('Tokens already exist - connecting with existing authentication');
      await this.attemptConnection();
    }

    this.log(`After attemptConnection, pierreClient is: ${!!this.pierreClient}`);
  }

  getClientSideTokenStatus(): { pierre: boolean; providers: Record<string, boolean> } {
    if (!this.oauthProvider) {
      return { pierre: false, providers: {} };
    }

    return this.oauthProvider.getTokenStatus();
  }

  private async createMcpServer(): Promise<void> {
    this.log('Creating MCP host server...');

    // Create MCP server for MCP host
    this.mcpServer = new Server(
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

    // Create stdio transport for MCP host
    this.serverTransport = new StdioServerTransport();

    this.log('MCP host server created');
  }

  private setupRequestHandlers(): void {
    if (!this.mcpServer) {
      throw new Error('MCP server not initialized');
    }

    // Bridge tools/list requests
    this.mcpServer.setRequestHandler(ListToolsRequestSchema, async (request) => {
      this.log('Bridging tools/list request');

      try {
        // If we have cached tools, return them immediately (from proactive connection)
        if (this.cachedTools) {
          this.log(`Using cached tools from proactive connection (${this.cachedTools.tools.length} tools)`);
          return this.cachedTools;
        }

        if (this.pierreClient) {
          // If connected, forward the request through the pierreClient
          // The client already has OAuth authentication built into its transport
          this.log('Fetching tools from Pierre server via authenticated client');
          const client = this.pierreClient;
          const result = await client.listTools();
          this.log(`Received ${result.tools.length} tools from Pierre`);
          // Cache the result for next time
          this.cachedTools = result;
          return result;
        } else {
          // If not connected, provide minimal tool list with connect_to_pierre
          this.log('Not connected to Pierre - offering connect_to_pierre tool only');
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
        this.log(`Error getting tools list: ${error.message || error}`);
        this.log('Providing connect tool only');

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
    this.mcpServer.setRequestHandler(CallToolRequestSchema, async (request) => {
      this.log('Bridging tool call:', request.params.name);

      // Handle special authentication tools
      if (request.params.name === 'connect_to_pierre') {
        return await this.handleConnectToPierre(request);
      }

      if (request.params.name === 'connect_provider') {
        return await this.handleConnectProvider(request);
      }

      // CRITICAL: Check for authentication tokens BEFORE attempting to connect
      // This prevents the SDK from triggering interactive OAuth flow during connection attempts
      if (this.oauthProvider) {
        // Check saved tokens directly without triggering validation
        const hasTokens = !!(this.oauthProvider as any).savedTokens;
        if (!hasTokens) {
          this.log(`No authentication tokens available - rejecting tool call ${request.params.name}`);
          return {
            content: [{
              type: 'text',
              text: `Authentication required. Please use the "Connect to Pierre" tool to authenticate before accessing fitness data.`
            }],
            isError: true
          };
        }
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
        this.log(`Forwarding tool call ${request.params.name} to Pierre server...`);
        // Use callTool() instead of request() - Client.request() is for raw JSON-RPC,
        // but we want the higher-level callTool() method which handles the protocol correctly
        const result = await this.pierreClient!.callTool({
          name: request.params.name,
          arguments: request.params.arguments || {}
        });
        this.log(`Tool call ${request.params.name} result:`, JSON.stringify(result).substring(0, 200));
        return result;
      } catch (error) {
        this.log(`Tool call ${request.params.name} failed:`, error);

        // Check if this is an authentication error using multiple detection methods
        const errorAny = error as any;

        // Method 1: Check structured MCP error data (server sets authentication_failed: true)
        const authFailedFlag = errorAny?.data?.authentication_failed === true;

        // Method 2: Check MCP JSON-RPC error codes for auth errors
        const errorCode = errorAny?.code;
        const hasAuthErrorCode = errorCode && (errorCode === -32603 || errorCode === -32602);

        // Method 3: Check HTTP status (transport layer errors)
        const errorMessage = error instanceof Error ? error.message : String(error);
        const errorLower = errorMessage.toLowerCase();
        const hasHttpAuthStatus = errorLower.includes('http 401') ||
                                  errorLower.includes('http 400'); // Fallback for misconfigured servers

        // Method 4: Check error message content
        const messageIndicatesAuth = errorLower.includes('unauthorized') ||
                                     errorLower.includes('authentication failed') ||
                                     errorLower.includes('jwt token') ||
                                     errorLower.includes('authentication') ||
                                     errorLower.includes('re-authenticate');

        const isAuthError = authFailedFlag || hasAuthErrorCode || hasHttpAuthStatus || messageIndicatesAuth;

        if (isAuthError && this.oauthProvider) {
          this.log(`Authentication error detected - attempting automatic recovery`);

          // Try to validate and refresh the token
          const tokens = await this.oauthProvider.tokens();
          if (tokens?.access_token && tokens?.refresh_token) {
            const validationResult = await this.oauthProvider['validateAndRefreshToken'](
              tokens.access_token,
              tokens.refresh_token
            );

            if (validationResult?.status === 'refreshed') {
              this.log(`Session automatically renewed - retrying your request`);

              // Retry the tool call with new tokens
              try {
                const retryResult = await this.pierreClient!.callTool({
                  name: request.params.name,
                  arguments: request.params.arguments || {}
                });
                this.log(`Request succeeded after automatic session renewal`);
                return retryResult;
              } catch (retryError) {
                this.log(`Request failed even after session renewal`);
                return {
                  content: [{
                    type: 'text',
                    text: `Tool execution failed after token refresh: ${retryError instanceof Error ? retryError.message : String(retryError)}`
                  }],
                  isError: true
                };
              }
            } else if (validationResult?.status === 'invalid') {
              this.log(`Automatic recovery failed - session cannot be renewed`);
              this.log(`Full re-authentication required`);

              // Clear the invalid connection
              await this.oauthProvider.invalidateCredentials('all');
              this.pierreClient = null;

              return {
                content: [{
                  type: 'text',
                  text: `Your session has expired and could not be refreshed. Please use the "Connect to Pierre" tool to re-authenticate.`
                }],
                isError: true
              };
            }
          }
        }

        // Return the original error if not an auth error or recovery failed
        return {
          content: [{
            type: 'text',
            text: `Tool execution failed: ${errorMessage}`
          }],
          isError: true
        };
      }
    });

    // Bridge resources/list requests
    this.mcpServer.setRequestHandler(ListResourcesRequestSchema, async (request) => {
      this.log('Bridging resources/list request');

      // Pierre server doesn't provide resources, so always return empty list
      return { resources: [] };
    });

    // Bridge resources/read requests
    this.mcpServer.setRequestHandler(ReadResourceRequestSchema, async (request) => {
      this.log('Bridging resource read:', request.params.uri);

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
    this.mcpServer.setRequestHandler(ListPromptsRequestSchema, async (request) => {
      this.log('Bridging prompts/list request');

      // Pierre server doesn't provide prompts, so always return empty list
      return { prompts: [] };
    });

    // Bridge prompts/get requests
    this.mcpServer.setRequestHandler(GetPromptRequestSchema, async (request) => {
      this.log('Bridging prompt get:', request.params.name);

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
    this.mcpServer.setRequestHandler(PingRequestSchema, async () => {
      this.log('Handling ping request');
      return {};
    });

    // Handle logging/setLevel requests
    this.mcpServer.setRequestHandler(SetLevelRequestSchema, async (request) => {
      this.log(`Setting log level to: ${request.params.level}`);
      return {};
    });

    // Bridge completion requests
    this.mcpServer.setRequestHandler(CompleteRequestSchema, async (request) => {
      this.log('Bridging completion request');

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

    this.log('Request handlers configured');
  }

  private async handleConnectToPierre(request: any): Promise<any> {
    try {
      this.log('Handling connect_to_pierre tool call - initiating OAuth flow');

      if (!this.oauthProvider) {
        return {
          content: [{
            type: 'text',
            text: 'OAuth provider not initialized. Please restart the bridge.'
          }],
          isError: true
        };
      }

      // Check if already authenticated
      // Credentials were validated at startup, so if they exist they're valid
      const hasTokens = !!(this.oauthProvider as any).savedTokens;
      if (hasTokens && this.pierreClient) {
        return {
          content: [{
            type: 'text',
            text: 'Already connected to Pierre! You can now use all fitness tools to access your Strava and Fitbit data.'
          }],
          isError: false
        };
      }

      // CRITICAL: Prevent interactive OAuth flow in automated test environments
      // Refuse interactive OAuth if:
      // 1. No TTY (not a terminal session)
      // 2. PIERRE_ALLOW_INTERACTIVE_OAUTH is not explicitly set to 'true'
      // This prevents OAuth browser flows in CI/CD and automated tests
      const allowInteractive = process.env.PIERRE_ALLOW_INTERACTIVE_OAUTH === 'true';
      const hasTTY = process.stdin.isTTY;

      if (!hasTokens && !hasTTY && !allowInteractive) {
        this.log('Refusing to start interactive OAuth flow in non-interactive environment (likely automated test or CI/CD)');
        this.log('Hint: Set PIERRE_ALLOW_INTERACTIVE_OAUTH=true to enable browser-based OAuth in MCP hosts');
        return {
          content: [{
            type: 'text',
            text: 'Authentication required but cannot start interactive OAuth flow in non-interactive environment. Please authenticate manually or provide credentials via environment variables.'
          }],
          isError: true
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
          this.log(`Cached ${tools.tools.length} tools after connect_to_pierre: ${JSON.stringify(tools.tools.map((t: any) => t.name))}`);
        } catch (toolError: any) {
          this.log(`Failed to cache tools: ${toolError.message}`);
        }
      }

      // Notify MCP host that tools have changed (now authenticated)
      if (this.mcpServer) {
        try {
          await this.mcpServer.notification({
            method: 'notifications/tools/list_changed',
            params: {}
          });
          this.log('Sent tools/list_changed notification to MCP host');
        } catch (error: any) {
          this.log('Failed to send tools/list_changed notification:', error.message);
        }
      }

      return {
        content: [{
          type: 'text',
          text: 'Successfully connected to Pierre Fitness Server!\n\n' +
                '**Next step:** Connect to a fitness provider to access your activity data.\n\n' +
                'Available providers:\n' +
                '- **Strava** - Connect your Strava account to access activities, stats, and athlete profile\n' +
                '- **Fitbit** - Connect your Fitbit account (if you use Fitbit)\n\n' +
                'To connect to Strava, say: "Connect to Strava"'
        }],
        isError: false
      };

    } catch (error: any) {
      this.log('Failed to connect to Pierre:', error.message);

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
      this.log('Handling unified connect_provider tool call');

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
      this.log(`Unified flow for provider: ${provider}`);

      // Step 1: Ensure Pierre authentication is complete
      if (!this.pierreClient) {
        this.log('Pierre not connected - initiating Pierre authentication first');
        try {
          await this.initiateConnection();
          this.log('Pierre authentication completed');
        } catch (error: any) {
          this.log(`Pierre authentication failed: ${error.message}`);
          return {
            content: [{
              type: 'text',
              text: `Failed to authenticate with Pierre: ${error.message}. Please try again.`
            }],
            isError: true
          };
        }
      } else {
        this.log('Pierre already authenticated');
      }

      // Step 2: Check if provider is already connected
      this.log(`Checking if ${provider} is already connected`);
      try {
        if (this.pierreClient) {
          const connectionStatus = await this.pierreClient.callTool({
            name: 'get_connection_status',
            arguments: { provider: provider }
          });

          // Check if the provider is already connected
          // The server returns structuredContent with providers array containing connection status
          if (connectionStatus) {
            this.log(`Full connection status response: ${JSON.stringify(connectionStatus).substring(0, 500)}...`);

            // Access the structured content with provider connection status
            const structured = (connectionStatus as any).structuredContent;
            if (structured && structured.providers && Array.isArray(structured.providers)) {
              const providerInfo = structured.providers.find((p: any) =>
                p.provider && p.provider.toLowerCase() === provider.toLowerCase()
              );

              if (providerInfo && providerInfo.connected === true) {
                this.log(`${provider} is already connected - no OAuth needed`);
                return {
                  content: [{
                    type: 'text',
                    text: `Already connected to ${provider.toUpperCase()}! You can now access your ${provider} fitness data.`
                  }],
                  isError: false
                };
              } else {
                this.log(`${provider} connected status: ${providerInfo ? providerInfo.connected : 'not found'}`);
              }
            }
          }
        }

        this.log(`${provider} not connected - proceeding with OAuth flow`);
      } catch (error: any) {
        this.log(`Could not check connection status: ${error.message} - proceeding with OAuth anyway`);
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

      this.log(`Initiating ${provider} OAuth flow for user: ${userId}`);

      try {
        // Correct OAuth URL format: /api/oauth/auth/{provider}/{user_id}
        const providerOAuthUrl = `${this.config.pierreServerUrl}/api/oauth/auth/${provider}/${userId}`;

        // Open provider OAuth in browser with focus
        await this.openUrlInBrowserWithFocus(providerOAuthUrl);

        this.log(`Opened ${provider} OAuth in browser: ${providerOAuthUrl}`);

        return {
          content: [{
            type: 'text',
            text: `Unified authentication flow completed!\n\nPierre: Connected\n${provider.toUpperCase()}: Opening in browser...\n\nPlease complete the ${provider.toUpperCase()} authentication in your browser. Once done, you'll have full access to your fitness data!`
          }],
          isError: false
        };

      } catch (error: any) {
        this.log(`Failed to open ${provider} OAuth: ${error.message}`);
        return {
          content: [{
            type: 'text',
            text: `Pierre authentication successful, but failed to open ${provider.toUpperCase()} OAuth: ${error.message}. You can manually visit the OAuth page in Pierre's web interface.`
          }],
          isError: false // Not a complete failure since Pierre auth worked
        };
      }

    } catch (error: any) {
      this.log('Unified connect_provider failed:', error.message);

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
          this.log(`Failed to open browser: ${error.message}`);
          return;
        }

        // After opening, try to activate common browsers
        setTimeout(() => {
          exec(`osascript -e 'tell application "Google Chrome" to activate' 2>/dev/null || osascript -e 'tell application "Safari" to activate' 2>/dev/null || osascript -e 'tell application "Firefox" to activate' 2>/dev/null || osascript -e 'tell application "Brave Browser" to activate' 2>/dev/null`, (activateError) => {
            if (activateError) {
              this.log('Could not activate browser (non-fatal)');
            }
          });
        }, 500);
      });
    } else if (platform === 'win32') {
      // Windows: start command brings window to front by default
      exec(`start "" "${url}"`, (error: Error | null) => {
        if (error) {
          this.log(`Failed to open browser: ${error.message}`);
        }
      });
    } else {
      // Linux: xdg-open
      exec(`xdg-open "${url}"`, (error: Error | null) => {
        if (error) {
          this.log(`Failed to open browser: ${error.message}`);
        }
      });
    }
  }

  private async startBridge(): Promise<void> {
    if (!this.mcpServer || !this.serverTransport) {
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

    // Start the stdio server for MCP host
    await this.mcpServer.connect(this.serverTransport);

    // IMPORTANT: Intercept messages AFTER connect() to ensure our handler isn't overwritten
    // The Server.connect() sets up its own onmessage handler, so we need to wrap it
    const mcpServerOnMessage = this.serverTransport.onmessage;
    this.serverTransport.onmessage = (message: any) => {
      // Log message details for debugging
      const messageMethod = message?.method || 'unknown';
      const messageId = message?.id !== undefined ? `id: ${message.id}` : 'notification';
      const messagePreview = Array.isArray(message)
        ? `batch[${message.length}]`
        : messageMethod;
      this.log(`Received MCP message: ${messagePreview} (${messageId})`);

      // Handle server/info requests
      if (message.method === 'server/info' && message.id !== undefined) {
        this.log(`Handling server/info request with ID: ${message.id}`);
        const response = {
          jsonrpc: '2.0' as const,
          id: message.id,
          result: {
            name: 'pierre-mcp-client',
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
          this.log(`Failed to send server/info response: ${err.message}`);
        });
        return;
      }

      // Handle JSON-RPC batch requests (should be rejected in 2025-06-18)
      // Batch requests come as arrays at the JSON level, not after parsing
      if (Array.isArray(message)) {
        this.log(`Rejecting JSON-RPC batch request (${message.length} requests, not supported in 2025-06-18)`);
        this.log(`Batch request IDs: ${message.map((r: any) => r.id).join(', ')}`);

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

        this.log(`Sending batch response array with ${responses.length} responses`);
        this.log(`Response structure: ${JSON.stringify(responses).substring(0, 200)}...`);

        // The MCP SDK's send() method serializes objects/arrays and adds newline
        // For batch responses, we need to send the array itself, not individual items
        // Cast to any to bypass TypeScript type checking for the array
        this.serverTransport!.send(responses as any).then(() => {
          this.log(`Batch response sent successfully`);
        }).catch((err: any) => {
          this.log(`Failed to send batch rejection response: ${err.message}`);
        });
        return;
      }

      // Handle client/log notifications gracefully
      if (message.method === 'client/log' && message.id === undefined) {
        this.log(`Client log [${message.params?.level}]: ${message.params?.message}`);
        return;
      }

      // Protect against malformed messages that crash the server
      try {
        // Forward other messages to the MCP Server handler
        if (mcpServerOnMessage) {
          mcpServerOnMessage(message);
        }
      } catch (error: any) {
        this.log(`Error handling message: ${error.message}`);
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

    this.log('Bridge is running - MCP host can now access Pierre Fitness tools');
  }

  private setupNotificationForwarding(): void {
    if (!this.pierreClient || !this.mcpServer) {
      return;
    }

    // Set up error handler for visibility
    this.pierreClient.onerror = (error) => {
      this.log('Pierre client error:', error);
    };

    // Set up OAuth completion notification handler
    // Listen for OAuth completion notifications from Pierre server
    // and forward them to MCP host so users see the success message
    try {
      this.pierreClient.setNotificationHandler(
        OAuthCompletedNotificationSchema,
        async (notification) => {
          this.log('Received OAuth completion notification from Pierre:', JSON.stringify(notification));

          if (this.mcpServer) {
            try {
              // Forward the notification to MCP host
              await this.mcpServer.notification({
                method: 'notifications/message',
                params: {
                  level: 'info',
                  message: notification.params?.message || 'OAuth authentication completed successfully!'
                }
              });
              this.log('Forwarded OAuth notification to MCP host');
            } catch (error: any) {
              this.log('Failed to forward OAuth notification to MCP host:', error.message);
            }
          }
        }
      );
      this.log('OAuth notification handler registered');
    } catch (error: any) {
      this.log('Failed to set up OAuth notification handler:', error.message);
    }

    this.log('Notification forwarding configured');
  }

  async stop(): Promise<void> {
    this.log('Stopping bridge...');

    try {
      // Close Pierre client connection
      if (this.pierreClient) {
        await this.pierreClient.close();
        this.pierreClient = null;
      }

      // Close MCP server
      if (this.mcpServer) {
        await this.mcpServer.close();
        this.mcpServer = null;
      }

      // Close OAuth callback server
      if (this.oauthProvider && (this.oauthProvider as any).callbackServer) {
        const callbackServer = (this.oauthProvider as any).callbackServer;
        return new Promise<void>((resolve) => {
          callbackServer.close(() => {
            this.log('OAuth callback server closed');
            resolve();
          });
        });
      }

      this.log('Bridge stopped');
    } catch (error) {
      this.log('Error stopping bridge:', error);
      throw error;
    }
  }
}