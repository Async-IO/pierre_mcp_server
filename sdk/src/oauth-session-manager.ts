// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: OAuth 2.0 session management for Pierre MCP Client
// ABOUTME: Handles token storage, refresh, client registration, and callback server for OAuth flows

import { OAuthClientProvider } from "@modelcontextprotocol/sdk/client/auth.js";
import { readFileSync } from "fs";
import { join } from "path";
import {
  OAuthClientMetadata,
  OAuthClientInformation,
  OAuthTokens,
  OAuthClientInformationFull,
} from "@modelcontextprotocol/sdk/shared/auth.js";
import { createSecureStorage, SecureTokenStorage } from "./secure-storage.js";
import { PierreError, PierreErrorCode } from "./errors.js";

// Load OAuth HTML templates from dist/templates/ (copied during build)
// Templates are self-contained in the SDK bundle for portability
const OAUTH_SUCCESS_TEMPLATE = readFileSync(
  join(__dirname, "templates/oauth_success.html"),
  "utf-8",
);
const OAUTH_ERROR_TEMPLATE = readFileSync(
  join(__dirname, "templates/oauth_error.html"),
  "utf-8",
);

/** Stored tokens structure for keychain persistence */
export interface StoredTokens {
  pierre?: OAuthTokens & { saved_at?: number };
  providers?: Record<
    string,
    {
      access_token: string;
      refresh_token?: string;
      expires_at?: number;
      token_type?: string;
      scope?: string;
    }
  >;
  // OAuth client registration info (includes client_secret) - stored securely alongside tokens
  client_info?: OAuthClientInformationFull;
}

/** Base configuration shared by all authentication modes */
export interface OAuthSessionConfigBase {
  pierreServerUrl: string;
  callbackPort?: number;
  disableBrowser?: boolean;
  tokenValidationTimeoutMs?: number;
}

/** JWT token authentication mode */
export interface OAuthSessionConfigJwt extends OAuthSessionConfigBase {
  mode: 'jwt';
  jwtToken: string;
}

/** OAuth 2.0 client credentials authentication mode */
export interface OAuthSessionConfigOAuth extends OAuthSessionConfigBase {
  mode: 'oauth';
  oauthClientId: string;
  oauthClientSecret: string;
}

/** API key authentication mode */
export interface OAuthSessionConfigApiKey extends OAuthSessionConfigBase {
  mode: 'api-key';
  apiKey: string;
}

/** User email/password credentials authentication mode */
export interface OAuthSessionConfigCredentials extends OAuthSessionConfigBase {
  mode: 'credentials';
  userEmail: string;
  userPassword: string;
}

/** Discriminated union for authentication modes */
export type OAuthSessionConfig = OAuthSessionConfigJwt | OAuthSessionConfigOAuth | OAuthSessionConfigApiKey | OAuthSessionConfigCredentials;

interface OAuth2TokenResponse {
  access_token: string;
  token_type: string;
  expires_in?: number;
  refresh_token?: string;
  scope?: string;
}

interface ValidateRefreshResponse {
  status: "valid" | "refreshed" | "invalid";
  expires_in?: number;
  access_token?: string;
  refresh_token?: string;
  token_type?: string;
  reason?: string;
  requires_full_reauth?: boolean;
}

export class PierreOAuthClientProvider implements OAuthClientProvider {
  private serverUrl: string;
  private config: OAuthSessionConfig;
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

  // Callback for notifying when provider OAuth completes (called by PierreMcpClient)
  private onProviderOAuthComplete:
    | ((provider: string) => Promise<void>)
    | undefined;

  // Pending provider OAuth promises (keyed by provider name)
  private pendingProviderOAuth: Map<
    string,
    { resolve: (value: any) => void; reject: (error: any) => void }
  > = new Map();

  constructor(
    serverUrl: string,
    config: OAuthSessionConfig,
    onProviderOAuthComplete?: (provider: string) => Promise<void>,
  ) {
    this.onProviderOAuthComplete = onProviderOAuthComplete;
    this.serverUrl = serverUrl;
    this.config = config;

    // NOTE: Secure storage initialization is async, so it's deferred to start()
    // to avoid race conditions with constructor completion
    // See initializePierreConnection() for the actual initialization
    // Client info (including client_secret) is loaded from secure storage alongside tokens

    this.log(`OAuth client provider created for server: ${serverUrl}`);
    this.log(
      `Using OS keychain for secure token storage (will initialize on start)`,
    );
  }

  // Wait for provider OAuth to complete (called from PierreMcpClient.handleConnectProvider)
  public waitForProviderOAuth(
    provider: string,
    timeoutMs: number = 120000,
  ): Promise<void> {
    return new Promise((resolve, reject) => {
      this.log(`Waiting for ${provider} OAuth completion (timeout: ${timeoutMs}ms)`);

      // Store the promise resolvers
      this.pendingProviderOAuth.set(provider, { resolve, reject });

      // Set timeout
      const timeoutId = setTimeout(() => {
        this.pendingProviderOAuth.delete(provider);
        reject(new Error(`${provider} OAuth timed out after ${timeoutMs}ms`));
      }, timeoutMs);

      // Wrap resolve to clear timeout
      const originalResolve = resolve;
      this.pendingProviderOAuth.set(provider, {
        resolve: (value: any) => {
          clearTimeout(timeoutId);
          originalResolve(value);
        },
        reject: (error: any) => {
          clearTimeout(timeoutId);
          reject(error);
        },
      });
    });
  }

  // Resolve pending provider OAuth (called from callback handler)
  private resolveProviderOAuth(provider: string): void {
    const pending = this.pendingProviderOAuth.get(provider);
    if (pending) {
      this.log(`Resolving ${provider} OAuth promise`);
      pending.resolve({ provider });
      this.pendingProviderOAuth.delete(provider);
    }
  }

  public async initializeSecureStorage(): Promise<void> {
    try {
      this.secureStorage = await createSecureStorage(this.log.bind(this));
      // Load existing tokens and client info from keychain
      await this.loadStoredTokens();

      // Load client info from secure storage (stored alongside tokens)
      this.loadClientInfoFromSecureStorage();

      // Migrate legacy plaintext client info file to secure storage
      await this.migratePlaintextClientInfo();

      // If JWT token was provided via --token parameter, use it for authentication
      // This is used in testing scenarios where tokens are passed directly
      // CRITICAL: CLI token ALWAYS takes precedence over keychain tokens (for testing)
      if (this.config.mode === 'jwt') {
        this.log(
          "Using JWT token from --token parameter for authentication (overrides keychain)",
        );
        this.savedTokens = {
          access_token: this.config.jwtToken,
          token_type: "Bearer",
          expires_in: 3600, // Default 1 hour, actual expiry is in the JWT itself
          scope: "read:fitness write:fitness",
          // Note: No refresh_token when using direct JWT
        };
        this.log("JWT token loaded from configuration");
      }

      this.log("Secure storage initialized with OS keychain");
    } catch (error) {
      this.log(`Failed to initialize secure storage: ${error}`);
      this.log("WARNING: Token storage will not be available");
    }
  }

  private async migratePlaintextClientInfo(): Promise<void> {
    const os = require("os");
    const path = require("path");
    const fs = require("fs");
    const legacyPath = path.join(os.homedir(), ".pierre-mcp-client-info.json");

    try {
      if (!fs.existsSync(legacyPath)) {
        return;
      }

      // Skip if we already have client info in secure storage
      if (this.clientInfo) {
        this.log("Client info already in secure storage, removing legacy plaintext file");
        fs.unlinkSync(legacyPath);
        return;
      }

      this.log(`Migrating plaintext client info from ${legacyPath} to secure storage`);
      const data = fs.readFileSync(legacyPath, "utf8");
      this.clientInfo = JSON.parse(data);

      // Save to secure storage
      await this.saveClientInfo();

      // Verify migration succeeded
      if (this.allStoredTokens.client_info) {
        fs.unlinkSync(legacyPath);
        this.log("Successfully migrated client info to secure storage and deleted plaintext file");
      } else {
        this.log("Migration verification failed - keeping plaintext file");
      }
    } catch (error) {
      this.log(`Failed to migrate plaintext client info: ${error}`);
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
        this.log("Secure storage not initialized, skipping token load");
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
          const providerCount = Object.keys(
            this.allStoredTokens.providers,
          ).length;
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

  private loadClientInfoFromSecureStorage(): void {
    // Load client info from the secure token storage (already loaded via loadStoredTokens)
    if (this.allStoredTokens.client_info) {
      this.clientInfo = this.allStoredTokens.client_info;
      this.log(
        `Loaded client info from secure storage: ${this.clientInfo?.client_id}`,
      );
    } else {
      this.log(
        `No stored client info found, will perform dynamic registration on first OAuth`,
      );
    }
  }

  private async saveClientInfo(): Promise<void> {
    if (!this.clientInfo) {
      return;
    }

    try {
      this.allStoredTokens.client_info = this.clientInfo;
      await this.saveStoredTokens();
      this.log(`Saved client info to secure storage: ${this.clientInfo.client_id}`);
    } catch (error) {
      this.log(`Failed to save client info: ${error}`);
    }
  }

  private async clearClientInfo(): Promise<void> {
    this.clientInfo = undefined;
    delete this.allStoredTokens.client_info;
    await this.saveStoredTokens();
    this.log("Cleared client info from secure storage");
  }

  private async saveStoredTokens(): Promise<void> {
    try {
      if (!this.secureStorage) {
        this.log("Secure storage not initialized, cannot save tokens");
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
      expires_at: tokenData.expires_in
        ? Date.now() + tokenData.expires_in * 1000
        : undefined,
      token_type: tokenData.token_type || "Bearer",
      scope: tokenData.scope,
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
      throw new PierreError(PierreErrorCode.CONFIG_ERROR, "Callback server failed to start - no port available");
    }
    return `http://localhost:${this.callbackPort}/oauth/callback`;
  }

  get clientMetadata(): OAuthClientMetadata {
    return {
      client_name: "Pierre MCP Client",
      client_uri: "https://claude.ai",
      redirect_uris: [this.redirectUrl],
      grant_types: ["authorization_code"],
      response_types: ["code"],
      scope: "read:fitness write:fitness",
      token_endpoint_auth_method: "client_secret_basic",
    };
  }

  async state(): Promise<string> {
    if (!this.stateValue) {
      this.stateValue = this.generateRandomString(32);
    }
    return this.stateValue;
  }

  async clientInformation(): Promise<OAuthClientInformation | undefined> {
    if (this.config.mode === 'oauth') {
      return {
        client_id: this.config.oauthClientId,
        client_secret: this.config.oauthClientSecret,
      };
    }
    return this.clientInfo
      ? {
          client_id: this.clientInfo.client_id,
          client_secret: this.clientInfo.client_secret,
        }
      : undefined;
  }

  async saveClientInformation(
    clientInformation: OAuthClientInformationFull,
  ): Promise<void> {
    this.log(
      `Registering client with Pierre OAuth server: ${clientInformation.client_id}`,
    );

    // Register this client with Pierre's OAuth server
    await this.registerClientWithPierre(clientInformation);

    this.clientInfo = clientInformation;
    await this.saveClientInfo();
    this.log(
      `Saved client info to secure storage: ${clientInformation.client_id}`,
    );
  }

  private async registerClientWithPierre(
    clientInfo: OAuthClientInformationFull,
  ): Promise<void> {
    const registrationEndpoint = `${this.serverUrl}/oauth2/register`;

    const registrationRequest = {
      client_id: clientInfo.client_id,
      client_secret: clientInfo.client_secret,
      redirect_uris: this.clientMetadata.redirect_uris,
      grant_types: this.clientMetadata.grant_types,
      response_types: this.clientMetadata.response_types,
      scope: this.clientMetadata.scope,
      client_name: this.clientMetadata.client_name,
      client_uri: this.clientMetadata.client_uri,
    };

    this.log(`Registering client at ${registrationEndpoint}`);

    try {
      // Use native fetch (Node 24+)
      const response = await fetch(registrationEndpoint, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Accept: "application/json",
        },
        body: JSON.stringify(registrationRequest),
      });

      if (!response.ok) {
        const errorText = await response.text();
        throw new PierreError(
          PierreErrorCode.AUTH_ERROR,
          `Client registration failed: ${response.status} ${response.statusText}: ${errorText}`,
        );
      }

      const registrationResponse: any = await response.json();

      // Security: Mask client_secret before logging
      const sanitizedResponse = { ...registrationResponse };
      if (sanitizedResponse.client_secret) {
        sanitizedResponse.client_secret = "***REDACTED***";
      }
      this.log(
        `Client registration successful: ${JSON.stringify(sanitizedResponse)}`,
      );

      // Update client info with the response from Pierre's server
      if (
        registrationResponse.client_id &&
        registrationResponse.client_secret
      ) {
        this.log(
          `Updating client info to use Pierre's returned client ID: ${registrationResponse.client_id}`,
        );
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
    this.allStoredTokens.pierre = {
      ...tokens,
      saved_at: Math.floor(Date.now() / 1000),
    };
    await this.saveStoredTokens();

    this.log(`Saved Pierre tokens: expires_in=${tokens.expires_in}`);
  }

  async tokens(): Promise<OAuthTokens | undefined> {
    // If no in-memory tokens, try to load from persistent storage
    if (!this.savedTokens && this.allStoredTokens.pierre) {
      this.log(
        `No in-memory tokens, attempting to reload from persistent storage`,
      );

      const storedTokens = this.allStoredTokens.pierre;

      // Always validate with server using validate-and-refresh endpoint
      const validationResult = await this.validateAndRefreshToken(
        storedTokens.access_token,
        storedTokens.refresh_token,
      );

      if (validationResult) {
        if (validationResult.status === "valid") {
          // Token is valid, reload into memory
          this.savedTokens = {
            access_token: storedTokens.access_token,
            refresh_token: storedTokens.refresh_token,
            expires_in: storedTokens.expires_in,
            token_type: storedTokens.token_type || "Bearer",
            scope: storedTokens.scope,
          };
          this.log(
            `Your session is valid (expires in ${validationResult.expires_in}s)`,
          );
        } else if (validationResult.status === "refreshed") {
          // Token was refreshed, save new tokens
          this.log(`Your session was automatically renewed with a fresh token`);

          this.savedTokens = {
            access_token: validationResult.access_token!,
            refresh_token: validationResult.refresh_token,
            expires_in: validationResult.expires_in,
            token_type: validationResult.token_type || "Bearer",
            scope: storedTokens.scope,
          };

          // Update persistent storage with new tokens
          this.allStoredTokens.pierre = {
            ...this.savedTokens,
            saved_at: Math.floor(Date.now() / 1000),
          };
          await this.saveStoredTokens();
          this.log(
            `Session renewed successfully - you can continue using Pierre tools`,
          );
        } else {
          // Token is invalid, clear storage and require full re-auth
          this.log(
            `Your session has expired and cannot be renewed automatically`,
          );
          this.log(
            `Reason: ${validationResult.reason || "Token validation failed"}`,
          );
          this.log(
            `Please use the "Connect to Pierre" tool to re-authenticate`,
          );
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

    this.log(
      `tokens() called, returning tokens: ${this.savedTokens ? "available" : "none"}`,
    );
    if (this.savedTokens) {
      this.log(`Token type: ${this.savedTokens.token_type}`);
    }
    return this.savedTokens;
  }

  public async validateAndRefreshToken(
    accessToken: string,
    refreshToken?: string,
  ): Promise<ValidateRefreshResponse | null> {
    try {
      this.log(`Validating token with server validate-and-refresh endpoint`);

      const timeoutMs = this.config.tokenValidationTimeoutMs || 3000;

      const fetchPromise = fetch(
        `${this.serverUrl}/oauth2/validate-and-refresh`,
        {
          method: "POST",
          headers: {
            Authorization: `Bearer ${accessToken}`,
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            refresh_token: refreshToken,
          }),
        },
      );

      const response = await Promise.race([
        fetchPromise,
        new Promise<Response>((_, reject) =>
          setTimeout(
            () =>
              reject(
                new Error(
                  `Token validate-and-refresh timeout after ${timeoutMs}ms`,
                ),
              ),
            timeoutMs,
          ),
        ),
      ]);

      if (!response.ok) {
        this.log(
          `Validate-and-refresh request failed: ${response.status} ${response.statusText}`,
        );
        return null;
      }

      const result = (await response.json()) as ValidateRefreshResponse;
      this.log(`Token validation status: ${result.status}`);

      if (result.status === "invalid" && result.reason) {
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

  private async exchangeCodeForTokens(
    authorizationCode: string,
    state: string,
  ): Promise<void> {
    // Validate OAuth state parameter to prevent CSRF attacks
    if (!this.stateValue || state !== this.stateValue) {
      this.log(
        `OAuth state mismatch: expected=${this.stateValue ? "[set]" : "[unset]"}, received=${state ? "[set]" : "[unset]"}`,
      );
      throw new PierreError(
        PierreErrorCode.AUTH_ERROR,
        "OAuth state parameter mismatch - possible CSRF attack. Please try connecting again.",
      );
    }

    if (!this.clientInfo) {
      throw new PierreError(PierreErrorCode.AUTH_ERROR, "Client information not available for token exchange");
    }

    if (!this.clientInfo.client_secret) {
      throw new PierreError(PierreErrorCode.AUTH_ERROR, "Client secret not available for token exchange");
    }

    if (!this.codeVerifierValue) {
      throw new PierreError(PierreErrorCode.AUTH_ERROR, "Code verifier not available for token exchange");
    }

    const tokenEndpoint = `${this.serverUrl}/oauth2/token`;
    const tokenRequestBody = new URLSearchParams({
      grant_type: "authorization_code",
      code: authorizationCode,
      redirect_uri: this.redirectUrl,
      client_id: this.clientInfo.client_id,
      client_secret: this.clientInfo.client_secret,
      code_verifier: this.codeVerifierValue,
    });

    this.log(`Requesting JWT token from ${tokenEndpoint}`);

    try {
      // Use native fetch (Node 24+)
      const response = await fetch(tokenEndpoint, {
        method: "POST",
        headers: {
          "Content-Type": "application/x-www-form-urlencoded",
          Accept: "application/json",
        },
        body: tokenRequestBody.toString(),
      });

      if (!response.ok) {
        const errorText = await response.text();
        throw new PierreError(
          PierreErrorCode.AUTH_ERROR,
          `Token exchange failed: ${response.status} ${response.statusText}: ${errorText}`,
        );
      }

      const tokenResponse = (await response.json()) as OAuth2TokenResponse;
      this.log(
        `Successfully received JWT token, expires_in=${tokenResponse.expires_in}`,
      );

      // Convert to MCP SDK OAuthTokens format and save
      const oauthTokens: OAuthTokens = {
        access_token: tokenResponse.access_token,
        token_type: tokenResponse.token_type,
        expires_in: tokenResponse.expires_in,
        refresh_token: tokenResponse.refresh_token,
        scope: tokenResponse.scope,
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
      throw new PierreError(
        PierreErrorCode.AUTH_ERROR,
        "Code verifier not found - authorization flow not started",
      );
    }
    return this.codeVerifierValue;
  }

  async invalidateCredentials(
    scope: "all" | "client" | "tokens" | "verifier",
  ): Promise<void> {
    switch (scope) {
      case "all":
        this.clientInfo = undefined;
        this.savedTokens = undefined;
        this.codeVerifierValue = undefined;
        this.stateValue = undefined;
        // Clear all stored tokens
        this.allStoredTokens = {};
        await this.saveStoredTokens();
        break;
      case "client":
        this.clientInfo = undefined;
        delete this.allStoredTokens.client_info;
        await this.saveStoredTokens();
        break;
      case "tokens":
        this.savedTokens = undefined;
        // Note: We intentionally do NOT clear persistent storage here
        // This allows token reload from storage if the tokens are still valid
        // Only clear persistent storage if tokens are truly expired
        break;
      case "verifier":
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
      providers: {} as Record<string, boolean>,
    };

    // Check Pierre token
    status.pierre = !!this.savedTokens;

    // Check provider tokens from client-side storage
    if (this.allStoredTokens.providers) {
      for (const [provider, tokenData] of Object.entries(
        this.allStoredTokens.providers,
      )) {
        // Check if token exists and is not expired
        const isValid =
          tokenData &&
          (!tokenData.expires_at || Date.now() < tokenData.expires_at);
        status.providers[provider] = !!isValid;
      }
    }

    return status;
  }

  public generateRandomString(length: number): string {
    const chars =
      "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    let result = "";
    for (let i = 0; i < length; i++) {
      result += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return result;
  }

  public async generateCodeChallenge(codeVerifier: string): Promise<string> {
    // Generate SHA-256 hash of the code verifier
    const crypto = require("crypto");
    const hash = crypto.createHash("sha256").update(codeVerifier).digest();

    // Base64 URL encode the hash
    return hash
      .toString("base64")
      .replace(/\+/g, "-")
      .replace(/\//g, "_")
      .replace(/=/g, "");
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

    const { exec } = await import("child_process");
    const platform = process.platform;

    if (platform === "darwin") {
      // macOS: Open URL then explicitly activate browser after a brief delay
      exec(`open "${url}"`, (error: Error | null) => {
        if (error) {
          this.log(`Failed to open browser: ${error.message}`);
          return;
        }

        // After opening, try to activate common browsers
        setTimeout(() => {
          exec(
            `osascript -e 'tell application "Google Chrome" to activate' 2>/dev/null || osascript -e 'tell application "Safari" to activate' 2>/dev/null || osascript -e 'tell application "Firefox" to activate' 2>/dev/null || osascript -e 'tell application "Brave Browser" to activate' 2>/dev/null`,
            (activateError) => {
              if (activateError) {
                this.log(`Could not activate browser (non-fatal)`);
              }
            },
          );
        }, 500);
      });
    } else if (platform === "win32") {
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
    const http = require("http");

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
    this.callbackServer.on("error", (error: any) => {
      if (error.code === "EADDRINUSE") {
        this.log(
          `Port ${port} is already in use - likely from previous session`,
        );
        this.log(`Attempting to use dynamic port assignment instead...`);

        // Clean up failed server
        this.callbackServer?.close();
        this.callbackServer = null;

        // Retry with dynamic port (OS will assign available port)
        this.callbackServer = http.createServer();
        this.callbackServer.listen(0, "localhost", () => {
          this.callbackPort = this.callbackServer.address().port;
          this.log(
            `Callback server listening on http://localhost:${this.callbackPort}/oauth/callback`,
          );
          this.setupCallbackHandler();
        });
      } else {
        this.log(`Failed to start callback server:`, error);
        throw error;
      }
    });

    this.callbackServer.listen(port, "localhost", () => {
      // Port already set above, just confirm it matches
      const actualPort = this.callbackServer.address().port;
      if (actualPort !== this.callbackPort) {
        this.log(
          `Warning: Server started on ${actualPort} but expected ${this.callbackPort}`,
        );
      }
      this.log(
        `Callback server listening on http://localhost:${this.callbackPort}/oauth/callback`,
      );
      // Set up the actual request handler
      this.setupCallbackHandler();
    });
  }

  private setupCallbackHandler(): void {
    if (!this.callbackServer) return;

    // Generate a random session token for callback authentication
    const crypto = require("crypto");
    this.callbackSessionToken = crypto.randomBytes(32).toString("hex");
    this.log(`Generated callback session token for authentication`);

    this.callbackServer.removeAllListeners("request");
    this.callbackServer.on("request", async (req: any, res: any) => {
      try {
        if (!req.url) {
          res.writeHead(400, { "Content-Type": "text/plain" });
          res.end("Bad Request: No URL provided");
          return;
        }

        const url = require("url");
        const parsedUrl = url.parse(req.url, true);

        if (parsedUrl.pathname === "/oauth/callback") {
          const query = parsedUrl.query;

          if (query.error) {
            this.log(`Authorization failed: ${query.error}`);
            res.writeHead(400, { "Content-Type": "text/html" });
            res.end(
              this.renderErrorTemplate(
                "MCP host OAuth",
                `${query.error}`,
                `${query.error_description || "Please try connecting again."}`,
              ),
            );
          } else if (query.code && query.state) {
            this.log(`Authorization successful, received code`);
            res.writeHead(200, { "Content-Type": "text/html" });
            res.end(this.renderSuccessTemplate("MCP host OAuth"));

            // Resolve the authorization promise if it exists
            const authResolve = (this.callbackServer as any)._authResolve;
            if (authResolve) {
              authResolve({ code: query.code, state: query.state });
            }

            // Authorization result is handled via promise resolution in redirectToAuthorization()
          } else {
            this.log(`Invalid callback parameters`);
            res.writeHead(400, { "Content-Type": "text/html" });
            res.end(
              this.renderErrorTemplate(
                "MCP host OAuth",
                "Invalid Request",
                "Missing required authorization parameters. Please try connecting again.",
              ),
            );
          }
        } else if (
          parsedUrl.pathname?.startsWith("/oauth/provider-callback/") &&
          req.method === "POST"
        ) {
          // Handle provider token callback for client-side storage
          const pathParts = parsedUrl.pathname.split("/");
          const provider = pathParts[3]; // /oauth/provider-callback/{provider}

          // Security: Validate both Host header and source IP (localhost only)
          // Provider OAuth callbacks come from Pierre server (not browser redirects)
          const host = req.headers.host;
          if (
            !host ||
            !(host.startsWith("localhost") || host.startsWith("127.0.0.1"))
          ) {
            this.log(
              `Rejected POST callback for ${provider}: Invalid host ${host}`,
            );
            res.writeHead(403, { "Content-Type": "application/json" });
            res.end(
              JSON.stringify({
                success: false,
                message: "Invalid host - only localhost allowed",
              }),
            );
            return;
          }

          // Validate source IP is actually localhost (Host header can be spoofed)
          const remoteAddress = req.socket?.remoteAddress;
          if (
            remoteAddress &&
            remoteAddress !== "127.0.0.1" &&
            remoteAddress !== "::1" &&
            remoteAddress !== "::ffff:127.0.0.1"
          ) {
            this.log(
              `Rejected POST callback for ${provider}: Non-localhost source IP ${remoteAddress}`,
            );
            res.writeHead(403, { "Content-Type": "application/json" });
            res.end(
              JSON.stringify({
                success: false,
                message: "Invalid source - only localhost allowed",
              }),
            );
            return;
          }

          // Validate callback session token if provided (defense in depth)
          const callbackToken =
            req.headers["x-callback-token"] ||
            parsedUrl.query?.callback_token;
          if (
            this.callbackSessionToken &&
            callbackToken &&
            callbackToken !== this.callbackSessionToken
          ) {
            this.log(
              `Rejected POST callback for ${provider}: Invalid callback session token`,
            );
            res.writeHead(403, { "Content-Type": "application/json" });
            res.end(
              JSON.stringify({
                success: false,
                message: "Invalid callback session token",
              }),
            );
            return;
          }

          this.log(`Provider token callback for ${provider}`);

          let body = "";
          req.on("data", (chunk: any) => {
            body += chunk.toString();
          });

          req.on("end", async () => {
            try {
              const tokenData = JSON.parse(body);
              await this.saveProviderToken(provider, tokenData);

              // Resolve pending provider OAuth promise (allows handleConnectProvider to continue)
              this.resolveProviderOAuth(provider);

              // Notify PierreMcpClient about provider OAuth completion (for MCP notification)
              if (this.onProviderOAuthComplete) {
                try {
                  await this.onProviderOAuthComplete(provider);
                  this.log(
                    `Notified MCP client about ${provider} OAuth completion`,
                  );
                } catch (notifyError) {
                  this.log(
                    `Failed to notify MCP client about ${provider} OAuth: ${notifyError}`,
                  );
                }
              }

              res.writeHead(200, { "Content-Type": "application/json" });
              res.end(
                JSON.stringify({
                  success: true,
                  message: `${provider} token stored client-side`,
                }),
              );
            } catch (error) {
              this.log(`Failed to save ${provider} token: ${error}`);
              res.writeHead(400, { "Content-Type": "application/json" });
              res.end(
                JSON.stringify({
                  success: false,
                  message: "Failed to save provider token",
                }),
              );
            }
          });

          return; // Don't write response yet, wait for request body
        } else {
          res.writeHead(404, { "Content-Type": "text/plain" });
          res.end("Not Found");
        }
      } catch (error) {
        this.log(`Callback server error: ${error}`);
        res.writeHead(500, { "Content-Type": "text/plain" });
        res.end("Internal Server Error");
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
    return OAUTH_SUCCESS_TEMPLATE.replace(/\{\{PROVIDER\}\}/g, this.escapeHtml(provider));
  }

  // Security: Escape HTML special characters to prevent XSS attacks
  // Query parameters like error and error_description come from external sources
  private escapeHtml(unsafe: string): string {
    return unsafe
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#039;");
  }

  private renderErrorTemplate(
    provider: string,
    error: string,
    description: string,
  ): string {
    // Security: Escape all user-provided values to prevent reflected XSS
    // The error and description values come from OAuth callback query parameters
    return OAUTH_ERROR_TEMPLATE.replace(/\{\{PROVIDER\}\}/g, this.escapeHtml(provider))
      .replace(/\{\{ERROR\}\}/g, this.escapeHtml(error))
      .replace(/\{\{DESCRIPTION\}\}/g, this.escapeHtml(description));
  }

  async validateAndCleanupCachedCredentials(): Promise<void> {
    const existingTokens = await this.tokens();
    const clientInfo = await this.clientInformation();

    if (!existingTokens && !clientInfo) {
      this.log("No cached credentials found - fresh start");
      return;
    }

    // Skip validation when using JWT token from --token parameter (testing mode)
    // These tokens are validated by the server on each request, not pre-validated
    if (this.config.mode === 'jwt') {
      this.log(
        "Skipping credential validation (using JWT token from --token parameter)",
      );
      return;
    }

    const timeoutMs = this.config.tokenValidationTimeoutMs || 3000;
    this.log(
      `Validating cached credentials with server (timeout: ${timeoutMs}ms)...`,
    );

    // Try to validate by calling the token validation endpoint with configurable timeout
    try {
      const fetchPromise = fetch(`${this.serverUrl}/oauth2/token-validate`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          ...(existingTokens
            ? { Authorization: `Bearer ${existingTokens.access_token}` }
            : {}),
        },
        body: JSON.stringify({
          client_id: clientInfo?.client_id,
        }),
      });

      const response = await Promise.race([
        fetchPromise,
        new Promise<Response>((_, reject) =>
          setTimeout(
            () =>
              reject(
                new Error(`Token validation timeout after ${timeoutMs}ms`),
              ),
            timeoutMs,
          ),
        ),
      ]);

      const result: any = await response.json();

      if (result.valid === false) {
        this.log(
          `Cached credentials are invalid: ${result.error || "unknown error"}`,
        );
        this.log("Cleaning up invalid cached credentials...");

        // Clear invalid tokens from keychain
        if (existingTokens && this.secureStorage) {
          await this.secureStorage.clearTokens();
          this.log("Cleared invalid tokens from keychain");
        }

        // Clear invalid client info from secure storage
        if (clientInfo) {
          await this.clearClientInfo();
          this.log("Cleared invalid client registration from secure storage");
        }

        // Reset in-memory state
        this.savedTokens = undefined;
        this.clientInfo = undefined;
        this.allStoredTokens = {};
      } else {
        this.log("Cached credentials are valid");
      }
    } catch (error: any) {
      this.log(`Failed to validate credentials: ${error.message}`);
      this.log(
        "Will proceed with cached credentials and handle errors during connection",
      );
    }
  }
}
