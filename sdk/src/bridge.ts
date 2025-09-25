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

  constructor(serverUrl: string, config: BridgeConfig) {
    this.serverUrl = serverUrl;
    this.config = config;
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

  async tokens(): Promise<OAuthTokens | undefined> {
    return this.savedTokens;
  }

  async saveTokens(tokens: OAuthTokens): Promise<void> {
    this.savedTokens = tokens;
    console.error(`[Pierre OAuth] Saved tokens: expires_in=${tokens.expires_in}`);
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
        break;
      case 'client':
        this.clientInfo = undefined;
        break;
      case 'tokens':
        this.savedTokens = undefined;
        break;
      case 'verifier':
        this.codeVerifierValue = undefined;
        break;
    }
    console.error(`[Pierre OAuth] Invalidated credentials: ${scope}`);
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

            // Also handle the authorization result directly for the sync callback
            try {
              console.error(`[Pierre OAuth] Authorization callback completed, exchanging code for tokens`);
              await this.exchangeCodeForTokens(query.code, query.state);
            } catch (error) {
              console.error(`[Pierre OAuth] Token exchange failed: ${error}`);
            }
          } else {
            console.error(`[Pierre OAuth] Invalid callback parameters`);
            res.writeHead(400, { 'Content-Type': 'text/html' });
            res.end(this.renderErrorTemplate(
              'Claude Desktop OAuth',
              'Invalid Request',
              'Missing required authorization parameters. Please try connecting again.'
            ));
          }
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
        <script>setTimeout(() => window.close(), 3000);</script>
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
        <p>You can close this window and try connecting again from Claude Desktop.</p>
        <script>setTimeout(() => window.close(), 5000);</script>
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

  private async connectToPierre(): Promise<void> {
    this.log('🔌 Connecting to Pierre MCP Server...');

    // Create MCP client with Streamable HTTP transport
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

    const mcpUrl = `${this.config.pierreServerUrl}/mcp`;
    this.log(`📡 Connecting to: ${mcpUrl}`);

    // Create OAuth client provider for Pierre MCP Server
    const oauthProvider = new PierreOAuthClientProvider(this.config.pierreServerUrl, this.config);

    // Create Streamable HTTP transport with OAuth 2.0 authentication
    const baseUrl = new URL(mcpUrl);
    const transport = new StreamableHTTPClientTransport(baseUrl, {
      authProvider: oauthProvider,
      requestInit: {
        headers: {
          'Content-Type': 'application/json',
          'User-Agent': 'Pierre-Claude-Bridge/1.0.0'
        }
      }
    });

    this.log('🔐 Using OAuth 2.0 authentication with Pierre MCP Server');

    // Connect to Pierre MCP Server with OAuth 2.0 flow
    await this.pierreClient.connect(transport);

    this.log('✅ Connected to Pierre MCP Server with OAuth 2.0');
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
    if (!this.claudeServer || !this.pierreClient) {
      throw new Error('Server or client not initialized');
    }

    // Bridge tools/list requests
    this.claudeServer.setRequestHandler(ListToolsRequestSchema, async (request) => {
      this.log('📋 Bridging tools/list request');
      return await this.pierreClient!.request(request, ListToolsRequestSchema);
    });

    // Bridge tools/call requests
    this.claudeServer.setRequestHandler(CallToolRequestSchema, async (request) => {
      this.log('🔧 Bridging tool call:', request.params.name);
      return await this.pierreClient!.request(request, CallToolRequestSchema);
    });

    // Bridge resources/list requests
    this.claudeServer.setRequestHandler(ListResourcesRequestSchema, async (request) => {
      this.log('📚 Bridging resources/list request');
      return await this.pierreClient!.request(request, ListResourcesRequestSchema);
    });

    // Bridge resources/read requests
    this.claudeServer.setRequestHandler(ReadResourceRequestSchema, async (request) => {
      this.log('📖 Bridging resource read:', request.params.uri);
      return await this.pierreClient!.request(request, ReadResourceRequestSchema);
    });

    // Bridge prompts/list requests
    this.claudeServer.setRequestHandler(ListPromptsRequestSchema, async (request) => {
      this.log('💭 Bridging prompts/list request');
      return await this.pierreClient!.request(request, ListPromptsRequestSchema);
    });

    // Bridge prompts/get requests
    this.claudeServer.setRequestHandler(GetPromptRequestSchema, async (request) => {
      this.log('💬 Bridging prompt get:', request.params.name);
      return await this.pierreClient!.request(request, GetPromptRequestSchema);
    });

    // Bridge completion requests
    this.claudeServer.setRequestHandler(CompleteRequestSchema, async (request) => {
      this.log('✨ Bridging completion request');
      return await this.pierreClient!.request(request, CompleteRequestSchema);
    });

    this.log('🌉 Request handlers configured');
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