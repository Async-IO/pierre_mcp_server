// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: MCP bridge connecting MCP host (stdio) to Pierre Server (HTTP)
// ABOUTME: Manages MCP message translation, tool forwarding, and OAuth flow integration

import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";
import { OAuthClientInformationFull } from "@modelcontextprotocol/sdk/shared/auth.js";
import { z } from "zod";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  ListResourcesRequestSchema,
  ReadResourceRequestSchema,
  ListPromptsRequestSchema,
  GetPromptRequestSchema,
  CompleteRequestSchema,
  PingRequestSchema,
  SetLevelRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";
import {
  validateMcpToolResponse,
  configureValidator,
  type ResponseValidatorConfig,
} from "./response-validator.js";
import { PierreOAuthClientProvider, OAuthSessionConfig } from "./oauth-session-manager.js";
import { installBatchGuard, createBatchGuardMessageHandler } from "./batch-guard-transport.js";

// Define custom notification schema for Pierre's OAuth completion notifications
const OAuthCompletedNotificationSchema = z.object({
  method: z.literal("notifications/oauth_completed"),
  params: z
    .object({
      provider: z.string(),
      success: z.boolean(),
      message: z.string(),
      user_id: z.string().optional(),
    })
    .optional(),
});

// Define the notification type explicitly to avoid deep type instantiation issues
type OAuthCompletedNotification = z.infer<typeof OAuthCompletedNotificationSchema>;

export interface BridgeConfig {
  pierreServerUrl: string;
  jwtToken?: string;
  apiKey?: string;
  oauthClientId?: string;
  oauthClientSecret?: string;
  userEmail?: string;
  userPassword?: string;
  callbackPort?: number;
  disableBrowser?: boolean; // Disable browser auto-opening for OAuth (testing mode)
  tokenValidationTimeoutMs?: number; // Default: 3000ms
  proactiveConnectionTimeoutMs?: number; // Default: 5000ms
  proactiveToolsListTimeoutMs?: number; // Default: 3000ms
  toolCallConnectionTimeoutMs?: number; // Default: 10000ms (10s for tool-triggered connections)
  /** Response validation configuration - validates tool responses against Zod schemas */
  responseValidation?: Partial<ResponseValidatorConfig>;
}

export class PierreMcpClient {
  private config: BridgeConfig;
  private pierreClient: Client | null = null;
  private mcpServer: Server | null = null;
  private serverTransport: StdioServerTransport | null = null;
  private cachedTools: any = null;
  private proactiveConnectionPromise: Promise<void> | null = null;
  private oauthProvider: PierreOAuthClientProvider | null = null;
  private mcpUrl: string = "";

  constructor(config: BridgeConfig) {
    this.config = config;

    // Configure response validation if specified
    if (config.responseValidation) {
      configureValidator(config.responseValidation);
    }
  }

  private log(message: string, ...args: any[]) {
    const timestamp = new Date().toISOString();
    console.error(`[${timestamp}] [Pierre Bridge] ${message}`, ...args);
  }

  private async withTimeout<T>(
    promise: Promise<T>,
    timeoutMs: number,
    operation: string,
  ): Promise<T | null> {
    return Promise.race([
      promise,
      new Promise<null>((resolve) =>
        setTimeout(() => {
          this.log(`Operation '${operation}' timed out after ${timeoutMs}ms`);
          resolve(null);
        }, timeoutMs),
      ),
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
      // Store promise so tools/list can wait for completion
      this.proactiveConnectionPromise = this.initializePierreConnection()
        .catch((error) => {
          this.log(
            "Pierre connection initialization failed (will retry on first tool call):",
            error,
          );
        })
        .then(() => {
          // Mark promise as resolved
          this.log("Proactive connection promise resolved");
        });

      this.log(
        "Bridge started successfully (Pierre connection initializing in background)",
      );
    } catch (error) {
      this.log("Failed to start bridge:", error);
      throw error;
    }
  }

  private async initializePierreConnection(): Promise<void> {
    // Set up Pierre connection parameters
    this.mcpUrl = `${this.config.pierreServerUrl}/mcp`;

    // Create OAuth provider with callback to notify MCP host when provider OAuth completes
    const onProviderOAuthComplete = async (provider: string): Promise<void> => {
      if (this.mcpServer) {
        const capitalizedProvider =
          provider.charAt(0).toUpperCase() + provider.slice(1);
        await this.mcpServer.notification({
          method: "notifications/message",
          params: {
            level: "info",
            logger: "pierre-oauth",
            data: {
              provider: provider,
              event: "oauth_completed",
              message: `${capitalizedProvider} connected successfully! You can now access your fitness data.`,
            },
          },
        });
        this.log(`Sent ${provider} OAuth completion notification to MCP host`);
      }
    };

    // Convert BridgeConfig to OAuthSessionConfig
    const oauthConfig: OAuthSessionConfig = {
      pierreServerUrl: this.config.pierreServerUrl,
      jwtToken: this.config.jwtToken,
      apiKey: this.config.apiKey,
      oauthClientId: this.config.oauthClientId,
      oauthClientSecret: this.config.oauthClientSecret,
      userEmail: this.config.userEmail,
      userPassword: this.config.userPassword,
      callbackPort: this.config.callbackPort,
      disableBrowser: this.config.disableBrowser,
      tokenValidationTimeoutMs: this.config.tokenValidationTimeoutMs,
    };

    this.oauthProvider = new PierreOAuthClientProvider(
      this.config.pierreServerUrl,
      oauthConfig,
      onProviderOAuthComplete,
    );

    // Initialize secure storage before any operations that might need it
    await this.oauthProvider.initializeSecureStorage();
    this.log(`Pierre MCP URL configured: ${this.mcpUrl}`);

    // Validate cached tokens and client registration at startup
    // This prevents wasting user time with invalid credentials
    await this.oauthProvider.validateAndCleanupCachedCredentials();

    // ALWAYS connect proactively to cache tools for MCP host
    // Server allows tools/list without authentication - only tool calls require auth
    // This ensures all tools are visible immediately in MCP host (tools/list_changed doesn't work)
    const connectionTimeoutMs =
      this.config.proactiveConnectionTimeoutMs || 15000;
    const toolsListTimeoutMs = this.config.proactiveToolsListTimeoutMs || 10000;

    try {
      this.log(
        `Connecting to Pierre proactively to cache all tools for MCP host (timeout: ${connectionTimeoutMs}ms)`,
      );
      const connectionResult = await this.withTimeout(
        this.connectToPierre(),
        connectionTimeoutMs,
        "proactive Pierre connection",
      );

      if (connectionResult === null) {
        // Connection timed out - this is non-fatal for the bridge
        this.log(
          `Proactive connection timed out after ${connectionTimeoutMs}ms - will connect on first tool use`,
        );
        this.log("Bridge will start with connect_to_pierre tool only");
        return;
      }

      // Cache tools immediately so they're ready for tools/list
      if (this.pierreClient) {
        const client = this.pierreClient;
        const toolsResult = await this.withTimeout(
          client.listTools(),
          toolsListTimeoutMs,
          "proactive tools list",
        );

        if (toolsResult) {
          this.cachedTools = toolsResult;
          this.log(
            `Cached ${toolsResult.tools.length} tools from Pierre: ${JSON.stringify(toolsResult.tools.map((t: any) => t.name))}`,
          );
        } else {
          this.log(
            `Tools list timed out after ${toolsListTimeoutMs}ms - will fetch on first request`,
          );
        }
      }
    } catch (error: any) {
      // If proactive connection fails, continue anyway
      // The bridge should still start - provide minimal toolset
      this.log(`Proactive connection failed: ${error.message}`);
      this.log("Bridge will start with connect_to_pierre tool only");
      // Don't propagate error - bridge should start successfully
    }
  }

  private async ensurePierreConnected(): Promise<void> {
    if (this.pierreClient) {
      return; // Already connected
    }

    const connectionTimeoutMs =
      this.config.toolCallConnectionTimeoutMs || 10000;
    this.log(
      `Connecting to Pierre MCP Server (timeout: ${connectionTimeoutMs}ms)...`,
    );

    const connectionResult = await this.withTimeout(
      this.connectToPierre(),
      connectionTimeoutMs,
      "tool-triggered Pierre connection",
    );

    if (connectionResult === null) {
      throw new Error(
        `Failed to connect to Pierre within ${connectionTimeoutMs}ms. Please use the "Connect to Pierre" tool to establish a connection.`,
      );
    }
  }

  private async connectToPierre(): Promise<void> {
    this.log("Connecting to Pierre MCP Server...");

    if (!this.oauthProvider) {
      throw new Error(
        "OAuth provider not initialized - call initializePierreConnection() first",
      );
    }

    this.log(`Target URL: ${this.mcpUrl}`);

    // Always attempt connection to discover tools (initialize and tools/list don't require auth)
    // If tokens exist, the connection will be fully authenticated
    // If no tokens, we can still discover tools but tool calls will require authentication via connect_to_pierre
    const existingTokens = await this.oauthProvider.tokens();
    if (existingTokens) {
      this.log("Found existing tokens - connecting with authentication");
    } else {
      this.log(
        "No tokens found - connecting without authentication to discover tools",
      );
    }

    await this.attemptConnection();
  }

  private async attemptConnection(): Promise<void> {
    if (!this.oauthProvider) {
      throw new Error("OAuth provider not initialized");
    }

    let connected = false;
    let retryCount = 0;
    const maxRetries = 3;

    while (!connected && retryCount < maxRetries) {
      try {
        // Create fresh MCP client for each attempt
        this.pierreClient = new Client(
          {
            name: "pierre-mcp-client",
            version: "1.0.0",
          },
          {
            capabilities: {},
          },
        );

        // Check if we have authentication tokens BEFORE creating transport
        // This prevents the SDK from triggering interactive OAuth flow
        // IMPORTANT: Must await tokens() to ensure async token loading completes
        // Using synchronous savedTokens check causes race condition (tokens may not be loaded yet)
        const existingTokens = await this.oauthProvider.tokens();
        const hasTokens = !!existingTokens;

        if (hasTokens) {
          this.log("Creating authenticated MCP transport (tokens available)");
        } else {
          this.log(
            "Creating unauthenticated MCP transport (no tokens) - tools/list will work per MCP spec",
          );
        }

        // Create fresh transport for each attempt
        const baseUrl = new URL(this.mcpUrl);
        const transport = new StreamableHTTPClientTransport(baseUrl, {
          // CRITICAL: Only provide authProvider if we have tokens
          // If we don't have tokens, connect without auth (tools/list works unauthenticated)
          authProvider: hasTokens ? this.oauthProvider : undefined,
        });

        // Connect to Pierre MCP Server
        await this.pierreClient.connect(transport);

        // CRITICAL: Validate that the MCP handshake completed successfully
        // The server MUST respond to initialize with proper JSON-RPC, not custom SSE events
        // This catches servers that send "event:connected" or other non-MCP messages
        try {
          this.log("Validating MCP protocol handshake with ping...");
          const pingTimeout = 5000; // 5 second timeout for validation
          await Promise.race([
            this.pierreClient.ping(),
            new Promise((_, reject) =>
              setTimeout(
                () =>
                  reject(
                    new Error(
                      "MCP ping timeout - server may not be responding to JSON-RPC requests",
                    ),
                  ),
                pingTimeout,
              ),
            ),
          ]);
          this.log(
            "MCP protocol validation successful - server is responding to JSON-RPC requests",
          );
        } catch (validationError: any) {
          this.log(
            `MCP protocol validation FAILED: ${validationError.message}`,
          );
          this.log(
            'Server may be sending invalid SSE events (e.g., "event:connected") instead of JSON-RPC messages',
          );
          throw new Error(
            `MCP protocol validation failed: ${validationError.message}. Server must send only JSON-RPC messages over SSE, not custom events.`,
          );
        }

        connected = true;

        if (hasTokens) {
          this.log("Connected to Pierre MCP Server (authenticated)");
        } else {
          this.log(
            "Connected to Pierre MCP Server (unauthenticated - tool discovery only)",
          );
          this.log(
            'Use "Connect to Pierre" tool to authenticate and access your fitness data',
          );
        }
        this.log(`pierreClient is now set: ${!!this.pierreClient}`);
      } catch (error: any) {
        if (error.message === "Unauthorized" && retryCount < maxRetries - 1) {
          retryCount++;
          this.log(
            `Token expired or invalid, retrying... (attempt ${retryCount}/${maxRetries})`,
          );

          // Clear invalid tokens
          await this.oauthProvider.invalidateCredentials("tokens");

          await new Promise((resolve) => setTimeout(resolve, 1000));
        } else {
          this.log(
            `Failed to connect after ${retryCount + 1} attempts: ${error.message}`,
          );
          throw error;
        }
      }
    }

    if (!connected) {
      throw new Error(
        `Failed to connect to Pierre MCP Server after ${maxRetries} attempts - authentication may be required`,
      );
    }
  }

  async initiateConnection(): Promise<void> {
    if (!this.oauthProvider) {
      throw new Error("OAuth provider not initialized");
    }

    this.log("Initiating OAuth connection to Pierre MCP Server");

    // Check if we already have tokens
    const existingTokens = await this.oauthProvider.tokens();

    if (!existingTokens) {
      this.log("No tokens found - starting OAuth 2.0 authorization flow");

      // Manually trigger OAuth flow by building authorization URL and redirecting
      try {
        // Step 1: Ensure client is registered (dynamic client registration)
        let clientInfo = await this.oauthProvider.clientInformation();

        // Get client metadata for redirect URI (needed for both new and existing clients)
        const clientMetadata = this.oauthProvider["clientMetadata"];

        if (!clientInfo) {
          this.log(
            "No client info found - performing dynamic client registration",
          );

          // Generate new client credentials
          const crypto = require("crypto");
          const clientId = `pierre-bridge-${crypto.randomBytes(8).toString("hex")}`;
          const clientSecret = crypto.randomBytes(32).toString("hex");

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
            client_secret_expires_at: 0, // Never expires
          };

          // Save and register the client (this updates clientInfo with Pierre's assigned client_id)
          await this.oauthProvider.saveClientInformation(fullClientInfo);

          // Re-fetch client information to get the server-assigned client_id
          clientInfo = await this.oauthProvider.clientInformation();
          if (!clientInfo) {
            throw new Error(
              "Client registration failed - no client info after registration",
            );
          }

          this.log(
            `Dynamic client registration complete: ${clientInfo.client_id}`,
          );
        }

        // Step 2: Get redirect URI
        const redirectUri = clientMetadata.redirect_uris[0];

        // Step 3: Generate PKCE values
        const state = await this.oauthProvider.state();
        const codeVerifier = this.oauthProvider.generateRandomString(64);
        await this.oauthProvider.saveCodeVerifier(codeVerifier);

        const codeChallenge =
          await this.oauthProvider.generateCodeChallenge(codeVerifier);

        // Step 4: Build authorization URL
        const authUrl = new URL(
          `${this.config.pierreServerUrl}/oauth2/authorize`,
        );
        authUrl.searchParams.set("client_id", clientInfo.client_id);
        authUrl.searchParams.set("redirect_uri", redirectUri);
        authUrl.searchParams.set("response_type", "code");
        authUrl.searchParams.set("state", state);
        authUrl.searchParams.set("code_challenge", codeChallenge);
        authUrl.searchParams.set("code_challenge_method", "S256");
        authUrl.searchParams.set("scope", "read:fitness write:fitness");

        // Step 5: Redirect to authorization (opens browser)
        await this.oauthProvider.redirectToAuthorization(authUrl);

        // Step 6: Connect after OAuth completes
        await this.attemptConnection();

        // Step 7: Refresh cached tools with authenticated toolset
        // Before OAuth, we may have cached unauthenticated tools (just connect_to_pierre)
        // After OAuth, we need to fetch and cache the FULL authenticated toolset
        try {
          if (this.pierreClient) {
            this.log("Fetching authenticated tools after OAuth...");
            const toolsResult = await this.pierreClient.listTools();
            this.cachedTools = toolsResult;
            this.log(
              `Refreshed cache with ${toolsResult.tools.length} authenticated tools: ${JSON.stringify(toolsResult.tools.map((t: any) => t.name))}`,
            );

            // Notify MCP host that tools have changed (now authenticated)
            if (this.mcpServer) {
              try {
                await this.mcpServer.notification({
                  method: "notifications/tools/list_changed",
                  params: {},
                });
                this.log("Sent tools/list_changed notification after OAuth");
              } catch (notifError: any) {
                this.log(
                  "Failed to send tools/list_changed notification:",
                  notifError.message,
                );
              }
            }
          }
        } catch (toolsError: any) {
          this.log("Failed to refresh tools after OAuth:", toolsError.message);
          // Non-fatal - tools will be fetched on next request
        }
      } catch (error) {
        this.log(`Failed to start OAuth flow: ${error}`);
        throw error;
      }
    } else {
      this.log(
        "Tokens already exist - connecting with existing authentication",
      );
      await this.attemptConnection();
    }

    this.log(
      `After attemptConnection, pierreClient is: ${!!this.pierreClient}`,
    );
  }

  getClientSideTokenStatus(): {
    pierre: boolean;
    providers: Record<string, boolean>;
  } {
    if (!this.oauthProvider) {
      return { pierre: false, providers: {} };
    }

    return this.oauthProvider.getTokenStatus();
  }

  private async createMcpServer(): Promise<void> {
    this.log("Creating MCP host server...");

    // Create MCP server for MCP host
    this.mcpServer = new Server(
      {
        name: "pierre-fitness",
        version: "1.0.0",
      },
      {
        capabilities: {
          tools: {},
          resources: {},
          prompts: {},
          logging: {},
          completions: {},
        },
      },
    );

    // Set up request handlers - bridge all requests to Pierre
    this.setupRequestHandlers();

    // Create stdio transport for MCP host
    this.serverTransport = new StdioServerTransport();

    this.log("MCP host server created");
  }

  private setupRequestHandlers(): void {
    if (!this.mcpServer) {
      throw new Error("MCP server not initialized");
    }

    // Bridge tools/list requests
    this.mcpServer.setRequestHandler(
      ListToolsRequestSchema,
      async (_request) => {
        this.log("Bridging tools/list request");

        try {
          // Wait for proactive connection to complete if it's still running
          // This ensures we have the full toolset cached before responding
          // Use a shorter timeout (1 second) to avoid blocking tools/list too long
          if (this.proactiveConnectionPromise) {
            this.log("Waiting for proactive connection to complete...");
            const waitResult = await this.withTimeout(
              this.proactiveConnectionPromise,
              1000,
              "tools/list waiting for proactive connection",
            );

            // Clear the promise reference so subsequent calls don't wait
            this.proactiveConnectionPromise = null;

            if (waitResult === null) {
              this.log(
                "Proactive connection still running after 1s, proceeding with current cache",
              );
            } else {
              this.log("Proactive connection completed, checking cache");
            }
          }

          // If we have cached tools, return them immediately (from proactive connection)
          if (this.cachedTools) {
            this.log(
              `Using cached tools from proactive connection (${this.cachedTools.tools.length} tools)`,
            );
            return this.cachedTools;
          }

          // Per MCP spec: tools/list MUST return ALL tools regardless of connection/auth status
          // If not connected yet, establish connection now (without auth is OK - server allows this)
          if (!this.pierreClient) {
            this.log(
              "Not connected - establishing connection to fetch tools (per MCP spec)",
            );
            try {
              await this.initializePierreConnection();
            } catch (error: any) {
              this.log(`Failed to connect to fetch tools: ${error.message}`);
              // Even if connection fails, we must return something
              // Return connect_to_pierre tool as fallback
              return {
                tools: [
                  {
                    name: "connect_to_pierre",
                    description:
                      "Connect to Pierre - Authenticate with Pierre Fitness Server to access your fitness data. This will open a browser window for secure login. Use this when you're not connected or need to reconnect.",
                    inputSchema: {
                      type: "object",
                      properties: {},
                      required: [],
                    },
                  },
                ],
              };
            }
          }

          // Now we should have a connection - fetch tools from server
          if (this.pierreClient) {
            this.log("Fetching tools from Pierre server");
            const client = this.pierreClient;
            const result = await client.listTools();
            this.log(`Received ${result.tools.length} tools from Pierre`);
            // Cache the result for next time
            this.cachedTools = result;
            return result;
          }

          // Should not reach here, but safety fallback
          this.log("Unexpected: no Pierre client after connection attempt");
          return {
            tools: [
              {
                name: "connect_to_pierre",
                description:
                  "Connect to Pierre - Authenticate with Pierre Fitness Server to access your fitness data. This will open a browser window for secure login. Use this when you're not connected or need to reconnect.",
                inputSchema: {
                  type: "object",
                  properties: {},
                  required: [],
                },
              },
            ],
          };
        } catch (error: any) {
          this.log(`Error getting tools list: ${error.message || error}`);
          this.log("Providing connect tool only");

          return {
            tools: [
              {
                name: "connect_to_pierre",
                description:
                  "Connect to Pierre - Authenticate with Pierre Fitness Server to access your fitness data. This will open a browser window for secure login. Use this when you're not connected or need to reconnect.",
                inputSchema: {
                  type: "object",
                  properties: {},
                  required: [],
                },
              },
            ],
          };
        }
      },
    );

    // Bridge tools/call requests
    this.mcpServer.setRequestHandler(CallToolRequestSchema, async (request) => {
      this.log("Bridging tool call:", request.params.name);

      // Handle special authentication tools
      if (request.params.name === "connect_to_pierre") {
        return await this.handleConnectToPierre(request);
      }

      if (request.params.name === "connect_provider") {
        return await this.handleConnectProvider(request);
      }

      // CRITICAL: Check for authentication tokens BEFORE attempting tool call
      // If no tokens, automatically trigger OAuth flow (not just return error)
      if (this.oauthProvider) {
        // IMPORTANT: Must await tokens() to ensure async token loading completes
        // Using synchronous savedTokens check causes race condition (tokens may not be loaded yet)
        const existingTokens = await this.oauthProvider.tokens();
        if (!existingTokens) {
          this.log(
            `No authentication tokens available - triggering OAuth flow for ${request.params.name}`,
          );

          // Automatically trigger OAuth instead of returning error
          // This provides seamless UX - user doesn't need to know about "connect_to_pierre"
          try {
            const connectResult = await this.handleConnectToPierre(request);
            if (connectResult.isError) {
              return connectResult;
            }
            // After successful OAuth, retry the original tool call
            this.log(`OAuth completed, retrying ${request.params.name}`);
          } catch (oauthError) {
            this.log(`OAuth flow failed: ${oauthError}`);
            return {
              content: [
                {
                  type: "text",
                  text: `Authentication required but OAuth flow failed: ${oauthError instanceof Error ? oauthError.message : String(oauthError)}. Please try again.`,
                },
              ],
              isError: true,
            };
          }
        }
      }

      // Ensure we have a connection before forwarding other tools
      try {
        await this.ensurePierreConnected();
      } catch (error) {
        return {
          content: [
            {
              type: "text",
              text: `Failed to connect to Pierre: ${error instanceof Error ? error.message : String(error)}. Please use the "Connect to Pierre" tool to authenticate.`,
            },
          ],
          isError: true,
        };
      }

      try {
        this.log(
          `Forwarding tool call ${request.params.name} to Pierre server...`,
        );
        // Use callTool() instead of request() - Client.request() is for raw JSON-RPC,
        // but we want the higher-level callTool() method which handles the protocol correctly
        const result = await this.pierreClient!.callTool({
          name: request.params.name,
          arguments: request.params.arguments || {},
        });
        this.log(
          `Tool call ${request.params.name} result:`,
          JSON.stringify(result).substring(0, 200),
        );

        // Validate response against Zod schema (logs warnings on mismatch, doesn't block)
        validateMcpToolResponse(request.params.name, result);

        return result;
      } catch (error) {
        this.log(`Tool call ${request.params.name} failed:`, error);

        // Check if this is an authentication error using multiple detection methods
        const errorAny = error as any;

        // Method 1: Check structured MCP error data (server sets authentication_failed: true)
        const authFailedFlag = errorAny?.data?.authentication_failed === true;

        // Method 2: Check MCP JSON-RPC error codes for auth errors
        const errorCode = errorAny?.code;
        const hasAuthErrorCode =
          errorCode && (errorCode === -32603 || errorCode === -32602);

        // Method 3: Check HTTP status (transport layer errors)
        const errorMessage =
          error instanceof Error ? error.message : String(error);
        const errorLower = errorMessage.toLowerCase();
        const hasHttpAuthStatus =
          errorLower.includes("http 401") || errorLower.includes("http 400"); // Fallback for misconfigured servers

        // Method 4: Check error message content
        const messageIndicatesAuth =
          errorLower.includes("unauthorized") ||
          errorLower.includes("authentication failed") ||
          errorLower.includes("jwt token") ||
          errorLower.includes("authentication") ||
          errorLower.includes("re-authenticate");

        const isAuthError =
          authFailedFlag ||
          hasAuthErrorCode ||
          hasHttpAuthStatus ||
          messageIndicatesAuth;

        if (isAuthError && this.oauthProvider) {
          this.log(
            `Authentication error detected - attempting automatic recovery`,
          );

          // Try to validate and refresh the token
          const tokens = await this.oauthProvider.tokens();
          if (tokens?.access_token && tokens?.refresh_token) {
            const validationResult = await this.oauthProvider.validateAndRefreshToken(
              tokens.access_token,
              tokens.refresh_token,
            );

            if (validationResult?.status === "refreshed") {
              this.log(`Session automatically renewed - retrying your request`);

              // Retry the tool call with new tokens
              try {
                const retryResult = await this.pierreClient!.callTool({
                  name: request.params.name,
                  arguments: request.params.arguments || {},
                });
                this.log(`Request succeeded after automatic session renewal`);

                // Validate response against Zod schema
                validateMcpToolResponse(request.params.name, retryResult);

                return retryResult;
              } catch (retryError) {
                this.log(`Request failed even after session renewal`);
                return {
                  content: [
                    {
                      type: "text",
                      text: `Tool execution failed after token refresh: ${retryError instanceof Error ? retryError.message : String(retryError)}`,
                    },
                  ],
                  isError: true,
                };
              }
            } else if (validationResult?.status === "invalid") {
              this.log(`Automatic recovery failed - session cannot be renewed`);
              this.log(`Full re-authentication required`);

              // Clear the invalid connection
              await this.oauthProvider.invalidateCredentials("all");
              this.pierreClient = null;

              return {
                content: [
                  {
                    type: "text",
                    text: `Your session has expired and could not be refreshed. Please use the "Connect to Pierre" tool to re-authenticate.`,
                  },
                ],
                isError: true,
              };
            }
          }
        }

        // Return the original error if not an auth error or recovery failed
        return {
          content: [
            {
              type: "text",
              text: `Tool execution failed: ${errorMessage}`,
            },
          ],
          isError: true,
        };
      }
    });

    // Bridge resources/list requests
    this.mcpServer.setRequestHandler(
      ListResourcesRequestSchema,
      async (_request) => {
        this.log("Bridging resources/list request");

        // Pierre server doesn't provide resources, so always return empty list
        return { resources: [] };
      },
    );

    // Bridge resources/read requests
    this.mcpServer.setRequestHandler(
      ReadResourceRequestSchema,
      async (request) => {
        this.log("Bridging resource read:", request.params.uri);

        if (!this.pierreClient) {
          return {
            contents: [
              {
                type: "text",
                text: 'Not connected to Pierre. Please use the "Connect to Pierre" tool first to authenticate.',
              },
            ],
          };
        }

        return await this.pierreClient.request(
          request,
          ReadResourceRequestSchema,
        );
      },
    );

    // Bridge prompts/list requests
    this.mcpServer.setRequestHandler(
      ListPromptsRequestSchema,
      async (_request) => {
        this.log("Bridging prompts/list request");

        // Pierre server doesn't provide prompts, so always return empty list
        return { prompts: [] };
      },
    );

    // Bridge prompts/get requests
    this.mcpServer.setRequestHandler(
      GetPromptRequestSchema,
      async (request) => {
        this.log("Bridging prompt get:", request.params.name);

        if (!this.pierreClient) {
          return {
            description: "Not connected to Pierre",
            messages: [
              {
                role: "user",
                content: {
                  type: "text",
                  text: 'Not connected to Pierre. Please use the "Connect to Pierre" tool first to authenticate.',
                },
              },
            ],
          };
        }

        return await this.pierreClient.request(request, GetPromptRequestSchema);
      },
    );

    // Handle ping requests
    this.mcpServer.setRequestHandler(PingRequestSchema, async () => {
      this.log("Handling ping request");
      return {};
    });

    // Handle logging/setLevel requests
    this.mcpServer.setRequestHandler(SetLevelRequestSchema, async (request) => {
      this.log(`Setting log level to: ${request.params.level}`);
      return {};
    });

    // Bridge completion requests
    this.mcpServer.setRequestHandler(CompleteRequestSchema, async (request) => {
      this.log("Bridging completion request");

      if (!this.pierreClient) {
        return {
          completion: {
            values: [],
            total: 0,
            hasMore: false,
          },
        };
      }

      return await this.pierreClient.request(request, CompleteRequestSchema);
    });

    this.log("Request handlers configured");
  }

  private async handleConnectToPierre(_request: any): Promise<any> {
    try {
      this.log("Handling connect_to_pierre tool call - initiating OAuth flow");

      if (!this.oauthProvider) {
        return {
          content: [
            {
              type: "text",
              text: "OAuth provider not initialized. Please restart the bridge.",
            },
          ],
          isError: true,
        };
      }

      // Check if already authenticated
      // Credentials were validated at startup, so if they exist they're valid
      // IMPORTANT: Must await tokens() to ensure async token loading completes
      // Using synchronous savedTokens check causes race condition (tokens may not be loaded yet)
      const existingTokens = await this.oauthProvider.tokens();
      if (existingTokens && this.pierreClient) {
        return {
          content: [
            {
              type: "text",
              text: "Already connected to Pierre! You can now use all fitness tools to access your Strava and Fitbit data.",
            },
          ],
          isError: false,
        };
      }

      // CRITICAL: Prevent interactive OAuth flow ONLY in CI/CD environments
      // Block OAuth if:
      // 1. No TTY (not a terminal session)
      // 2. Running in CI/CD (CI=true or GITHUB_ACTIONS=true)
      // This prevents hanging in automated tests while allowing OAuth in MCP hosts like Claude Code Desktop
      const isCI =
        process.env.CI === "true" || process.env.GITHUB_ACTIONS === "true";
      const hasTTY = process.stdin.isTTY;

      if (!existingTokens && !hasTTY && isCI) {
        this.log(
          "Refusing to start interactive OAuth flow in CI/CD environment (would hang automated tests)",
        );
        this.log(
          "Hint: In CI/CD, pre-authenticate using credentials or skip OAuth-requiring tests",
        );
        return {
          content: [
            {
              type: "text",
              text: "Authentication required but cannot start interactive OAuth flow in CI/CD environment. Please use credentials-based auth or skip OAuth tests.",
            },
          ],
          isError: true,
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
          this.log(
            `Cached ${tools.tools.length} tools after connect_to_pierre: ${JSON.stringify(tools.tools.map((t: any) => t.name))}`,
          );
        } catch (toolError: any) {
          this.log(`Failed to cache tools: ${toolError.message}`);
        }
      }

      // Notify MCP host that tools have changed (now authenticated)
      if (this.mcpServer) {
        try {
          await this.mcpServer.notification({
            method: "notifications/tools/list_changed",
            params: {},
          });
          this.log("Sent tools/list_changed notification to MCP host");
        } catch (error: any) {
          this.log(
            "Failed to send tools/list_changed notification:",
            error.message,
          );
        }
      }

      return {
        content: [
          {
            type: "text",
            text:
              "Successfully connected to Pierre Fitness Server!\n\n" +
              "**Next step:** Connect to a fitness provider to access your activity data.\n\n" +
              "Available providers:\n" +
              "- **Strava** - Connect your Strava account to access activities, stats, and athlete profile\n" +
              "- **Fitbit** - Connect your Fitbit account (if you use Fitbit)\n\n" +
              'To connect to Strava, say: "Connect to Strava"',
          },
        ],
        isError: false,
      };
    } catch (error: any) {
      this.log("Failed to connect to Pierre:", error.message);

      return {
        content: [
          {
            type: "text",
            text: `Failed to connect to Pierre: ${error.message}. Please check that the Pierre server is running and try again.`,
          },
        ],
        isError: true,
      };
    }
  }

  private async handleConnectProvider(request: any): Promise<any> {
    try {
      this.log("Handling unified connect_provider tool call");

      if (!this.oauthProvider) {
        return {
          content: [
            {
              type: "text",
              text: "OAuth provider not initialized. Please restart the bridge.",
            },
          ],
          isError: true,
        };
      }

      // Extract provider from request parameters
      const provider = request.params.arguments?.provider || "strava";
      this.log(`Unified flow for provider: ${provider}`);

      // Step 1: Ensure Pierre authentication is complete
      if (!this.pierreClient) {
        this.log(
          "Pierre not connected - initiating Pierre authentication first",
        );
        try {
          await this.initiateConnection();
          this.log("Pierre authentication completed");
        } catch (error: any) {
          this.log(`Pierre authentication failed: ${error.message}`);
          return {
            content: [
              {
                type: "text",
                text: `Failed to authenticate with Pierre: ${error.message}. Please try again.`,
              },
            ],
            isError: true,
          };
        }
      } else {
        this.log("Pierre already authenticated");
      }

      // Step 2: Check if provider is already connected
      this.log(`Checking if ${provider} is already connected`);
      try {
        if (this.pierreClient) {
          const connectionStatus = await this.pierreClient.callTool({
            name: "get_connection_status",
            arguments: { provider: provider },
          });

          // Check if the provider is already connected
          // The server returns structuredContent with providers array containing connection status
          if (connectionStatus) {
            this.log(
              `Full connection status response: ${JSON.stringify(connectionStatus).substring(0, 500)}...`,
            );

            // Access the structured content with provider connection status
            const structured = (connectionStatus as any).structuredContent;
            if (
              structured &&
              structured.providers &&
              Array.isArray(structured.providers)
            ) {
              const providerInfo = structured.providers.find(
                (p: any) =>
                  p.provider &&
                  p.provider.toLowerCase() === provider.toLowerCase(),
              );

              if (providerInfo && providerInfo.connected === true) {
                this.log(`${provider} is already connected - no OAuth needed`);
                return {
                  content: [
                    {
                      type: "text",
                      text: `Already connected to ${provider.toUpperCase()}! You can now access your ${provider} fitness data.`,
                    },
                  ],
                  isError: false,
                };
              } else {
                this.log(
                  `${provider} connected status: ${providerInfo ? providerInfo.connected : "not found"}`,
                );
              }
            }
          }
        }

        this.log(`${provider} not connected - proceeding with OAuth flow`);
      } catch (error: any) {
        this.log(
          `Could not check connection status: ${error.message} - proceeding with OAuth anyway`,
        );
      }

      // Step 3: Extract user_id from JWT token
      const tokens = await this.oauthProvider.tokens();
      if (!tokens?.access_token) {
        throw new Error("No access token available");
      }

      // Decode JWT to get user_id (JWT format: header.payload.signature)
      const payload = tokens.access_token.split(".")[1];
      const decoded = JSON.parse(Buffer.from(payload, "base64").toString());
      const userId = decoded.sub;

      if (!userId) {
        throw new Error("Could not extract user_id from JWT token");
      }

      this.log(`Initiating ${provider} OAuth flow for user: ${userId}`);

      // Ensure callback server is running to receive provider OAuth completion notification
      // The server will POST to this callback when provider OAuth completes
      if (this.oauthProvider) {
        const oauthProviderAny = this.oauthProvider as any;
        if (!oauthProviderAny.callbackServer) {
          this.log("Starting callback server for provider OAuth notification");
          // Accessing redirectUrl triggers startCallbackServerSync internally
          const callbackUrl = oauthProviderAny.redirectUrl;
          this.log(`Callback server ready at ${callbackUrl}`);
        }
      }

      try {
        // Correct OAuth URL format: /api/oauth/auth/{provider}/{user_id}
        const providerOAuthUrl = `${this.config.pierreServerUrl}/api/oauth/auth/${provider}/${userId}`;

        // Open provider OAuth in browser with focus
        await this.openUrlInBrowserWithFocus(providerOAuthUrl);

        this.log(`Opened ${provider} OAuth in browser: ${providerOAuthUrl}`);
        this.log(`Waiting for ${provider} OAuth to complete...`);

        // Wait for provider OAuth to complete (similar to Pierre OAuth flow)
        // Timeout after 2 minutes if user doesn't complete OAuth
        await this.oauthProvider.waitForProviderOAuth(provider, 120000);

        this.log(`${provider} OAuth completed successfully`);

        const capitalizedProvider =
          provider.charAt(0).toUpperCase() + provider.slice(1);

        return {
          content: [
            {
              type: "text",
              text: `${capitalizedProvider} connected successfully!\n\nYou now have full access to your ${capitalizedProvider} fitness data. Try asking me about your recent activities, stats, or training insights!`,
            },
          ],
          isError: false,
        };
      } catch (error: any) {
        // Check if it's a timeout
        if (error.message?.includes("timed out")) {
          this.log(`${provider} OAuth timed out`);
          return {
            content: [
              {
                type: "text",
                text: `${provider.toUpperCase()} authentication timed out. Please try again with connect_provider.`,
              },
            ],
            isError: true,
          };
        }
        this.log(`Failed to complete ${provider} OAuth: ${error.message}`);
        return {
          content: [
            {
              type: "text",
              text: `Pierre authentication successful, but failed to open ${provider.toUpperCase()} OAuth: ${error.message}. You can manually visit the OAuth page in Pierre's web interface.`,
            },
          ],
          isError: false, // Not a complete failure since Pierre auth worked
        };
      }
    } catch (error: any) {
      this.log("Unified connect_provider failed:", error.message);

      return {
        content: [
          {
            type: "text",
            text: `Unified authentication failed: ${error.message}. Please check that Pierre server is running and try again.`,
          },
        ],
        isError: true,
      };
    }
  }

  private async openUrlInBrowserWithFocus(url: string): Promise<void> {
    // Check if browser opening is disabled (testing mode)
    if (this.config.disableBrowser) {
      this.log(
        "Browser opening disabled - OAuth URL available at callback server",
      );
      this.log(`OAuth URL: ${url}`);
      return;
    }

    // Security: Validate URL format before opening to prevent command injection
    // Only allow http/https URLs from trusted OAuth providers
    let parsedUrl: URL;
    try {
      parsedUrl = new URL(url);
      if (parsedUrl.protocol !== "http:" && parsedUrl.protocol !== "https:") {
        this.log(`Refusing to open non-HTTP URL: ${parsedUrl.protocol}`);
        return;
      }
    } catch {
      this.log("Invalid URL format, refusing to open");
      return;
    }

    // Use execFile instead of exec to prevent shell injection
    // execFile does not spawn a shell, so special characters in the URL cannot be interpreted
    const { execFile } = await import("child_process");
    const platform = process.platform;

    if (platform === "darwin") {
      // macOS: Open URL then explicitly activate browser after a brief delay
      execFile("open", [parsedUrl.href], (error: Error | null) => {
        if (error) {
          this.log(`Failed to open browser: ${error.message}`);
          return;
        }

        // After opening, try to activate common browsers using osascript
        // This is safe as we're not passing user input to the script
        setTimeout(() => {
          execFile(
            "osascript",
            [
              "-e",
              'tell application "Google Chrome" to activate',
            ],
            (chromeError) => {
              if (chromeError) {
                execFile(
                  "osascript",
                  ["-e", 'tell application "Safari" to activate'],
                  (safariError) => {
                    if (safariError) {
                      execFile(
                        "osascript",
                        ["-e", 'tell application "Firefox" to activate'],
                        () => {
                          // Ignore errors - browser activation is non-critical
                        },
                      );
                    }
                  },
                );
              }
            },
          );
        }, 500);
      });
    } else if (platform === "win32") {
      // Windows: Use cmd.exe with /c start to open URL
      // execFile with cmd.exe prevents shell injection while allowing URL opening
      execFile("cmd.exe", ["/c", "start", "", parsedUrl.href], (error: Error | null) => {
        if (error) {
          this.log(`Failed to open browser: ${error.message}`);
        }
      });
    } else {
      // Linux: xdg-open with validated URL
      execFile("xdg-open", [parsedUrl.href], (error: Error | null) => {
        if (error) {
          this.log(`Failed to open browser: ${error.message}`);
        }
      });
    }
  }

  private async startBridge(): Promise<void> {
    if (!this.mcpServer || !this.serverTransport) {
      throw new Error("Server or transport not initialized");
    }

    // Install batch request guard on transport
    installBatchGuard(this.serverTransport, this.log.bind(this));

    // Start the stdio server for MCP host
    await this.mcpServer.connect(this.serverTransport);

    // IMPORTANT: Intercept messages AFTER connect() to ensure our handler isn't overwritten
    // The Server.connect() sets up its own onmessage handler, so we need to wrap it
    const mcpServerOnMessage = this.serverTransport.onmessage;
    this.serverTransport.onmessage = createBatchGuardMessageHandler(
      this.serverTransport,
      mcpServerOnMessage,
      this.log.bind(this),
    );

    // Set up notification forwarding from Pierre to Claude
    this.setupNotificationForwarding();

    this.log(
      "Bridge is running - MCP host can now access Pierre Fitness tools",
    );
  }

  private setupNotificationForwarding(): void {
    if (!this.pierreClient || !this.mcpServer) {
      return;
    }

    // Set up error handler for visibility
    this.pierreClient.onerror = (error) => {
      this.log("Pierre client error:", error);
    };

    // Set up OAuth completion notification handler
    // Listen for OAuth completion notifications from Pierre server
    // and forward them to MCP host so users see the success message
    try {
      // Use explicit handler function to avoid deep type instantiation
      const oauthNotificationHandler = async (
        notification: OAuthCompletedNotification,
      ) => {
        this.log(
          "Received OAuth completion notification from Pierre:",
          JSON.stringify(notification),
        );

        if (this.mcpServer) {
          try {
            // Forward the notification to MCP host
            await this.mcpServer.notification({
              method: "notifications/message",
              params: {
                level: "info",
                message:
                  notification.params?.message ||
                  "OAuth authentication completed successfully!",
              },
            });
            this.log("Forwarded OAuth notification to MCP host");
          } catch (error: any) {
            this.log(
              "Failed to forward OAuth notification to MCP host:",
              error.message,
            );
          }
        }
      };
      this.pierreClient.setNotificationHandler(
        OAuthCompletedNotificationSchema as any,
        oauthNotificationHandler as any,
      );
      this.log("OAuth notification handler registered");
    } catch (error: any) {
      this.log("Failed to set up OAuth notification handler:", error.message);
    }

    this.log("Notification forwarding configured");
  }

  async stop(): Promise<void> {
    this.log("Stopping bridge...");

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
            this.log("OAuth callback server closed");
            resolve();
          });
        });
      }

      this.log("Bridge stopped");
    } catch (error) {
      this.log("Error stopping bridge:", error);
      throw error;
    }
  }
}
