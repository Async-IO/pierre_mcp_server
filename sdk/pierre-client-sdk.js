// ABOUTME: JavaScript SDK for Pierre MCP Server client interactions
// ABOUTME: Provides clean abstractions for user onboarding, OAuth setup, and API management

/**
 * Pierre MCP Server Client SDK
 * 
 * A comprehensive JavaScript SDK for interacting with the Pierre MCP Server
 * from Claude Desktop and other MCP clients. Handles user registration,
 * admin approval, OAuth credential storage, and API key management.
 * 
 * @author Pierre Team
 * @version 1.0.0
 */

class PierreClientSDK {
    /**
     * Initialize the Pierre Client SDK
     * @param {string} serverUrl - The base URL of the Pierre MCP Server (e.g., 'http://localhost:3030')
     * @param {Object} options - Configuration options
     * @param {number} options.timeout - Request timeout in milliseconds (default: 30000)
     * @param {boolean} options.debug - Enable debug logging (default: false)
     */
    constructor(serverUrl, options = {}) {
        this.serverUrl = serverUrl.replace(/\/$/, ''); // Remove trailing slash
        this.timeout = options.timeout || 30000;
        this.debug = options.debug || false;
        this.userSession = null;
        
        this.log('SDK initialized for server:', this.serverUrl);
    }

    /**
     * Internal logging method
     * @private
     */
    log(...args) {
        if (this.debug) {
            console.log('[Pierre SDK]', ...args);
        }
    }

    /**
     * Make HTTP request with error handling
     * @private
     */
    async makeRequest(method, endpoint, data = null, headers = {}) {
        const url = `${this.serverUrl}${endpoint}`;
        const config = {
            method,
            headers: {
                'Content-Type': 'application/json',
                ...headers
            }
        };

        if (data && method !== 'GET') {
            config.body = JSON.stringify(data);
        }

        this.log(`${method} ${url}`, data ? data : '');

        try {
            const response = await fetch(url, config);
            const responseText = await response.text();
            
            this.log(`Response ${response.status}:`, responseText);

            let responseData;
            try {
                responseData = JSON.parse(responseText);
            } catch (e) {
                responseData = { message: responseText };
            }

            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${responseData.message || responseData.error || responseText}`);
            }

            return responseData;
        } catch (error) {
            this.log('Request failed:', error.message);
            throw error;
        }
    }

    /**
     * Step 1: Register a new user account
     * Note: User will be created with 'pending' status and needs admin approval
     * 
     * @param {string} email - User's email address
     * @param {string} password - User's password (min 8 characters)
     * @param {string} displayName - Optional display name
     * @returns {Promise<Object>} Registration response with user_id and message
     */
    async registerUser(email, password, displayName = null) {
        this.log('Registering user:', email);
        
        const response = await this.makeRequest('POST', '/register', {
            email,
            password,
            display_name: displayName
        });

        this.log('User registered successfully. Status: pending approval');
        return response;
    }

    /**
     * Step 2: Check if user account has been approved by admin
     * 
     * @param {string} email - User's email address
     * @param {string} password - User's password
     * @returns {Promise<Object>} Login response or error if still pending
     */
    async checkApprovalStatus(email, password) {
        this.log('Checking approval status for:', email);
        
        try {
            const response = await this.makeRequest('POST', '/login', {
                email,
                password
            });
            
            this.userSession = {
                jwt_token: response.jwt_token,
                user: response.user,
                expires_at: response.expires_at
            };
            
            this.log('User approved and logged in successfully');
            return {
                approved: true,
                session: this.userSession
            };
        } catch (error) {
            if (error.message.includes('pending admin approval')) {
                this.log('User still pending approval');
                return {
                    approved: false,
                    message: 'Your account is still pending admin approval. Please wait for approval before proceeding.'
                };
            }
            throw error;
        }
    }

    /**
     * Step 3: Store OAuth app credentials for a fitness provider
     * Requires approved user session
     * 
     * @param {string} provider - Provider name ('strava', 'fitbit', etc.)
     * @param {string} clientId - OAuth app client ID
     * @param {string} clientSecret - OAuth app client secret
     * @param {string} redirectUri - OAuth app redirect URI
     * @returns {Promise<Object>} Storage confirmation
     */
    async storeOAuthCredentials(provider, clientId, clientSecret, redirectUri) {
        if (!this.userSession) {
            throw new Error('User must be logged in to store OAuth credentials. Call checkApprovalStatus() first.');
        }

        this.log(`Storing OAuth credentials for provider: ${provider}`);
        
        const response = await this.makeRequest('POST', '/oauth-apps', {
            provider,
            client_id: clientId,
            client_secret: clientSecret,
            redirect_uri: redirectUri
        }, {
            'Authorization': `Bearer ${this.userSession.jwt_token}`
        });

        this.log('OAuth credentials stored successfully');
        return response;
    }

    /**
     * Step 4: Create an API key for MCP access
     * Requires approved user session
     * 
     * @param {string} description - Description for the API key
     * @param {Object} options - API key options
     * @param {string} options.tier - API key tier ('trial', 'starter', 'professional', 'enterprise')
     * @param {number} options.expiresInDays - Optional expiration in days
     * @returns {Promise<Object>} API key creation response
     */
    async createApiKey(description = 'MCP Client API Key', options = {}) {
        if (!this.userSession) {
            throw new Error('User must be logged in to create API key. Call checkApprovalStatus() first.');
        }

        this.log('Creating API key:', description);
        
        const response = await this.makeRequest('POST', '/api-keys', {
            name: description,
            description: description,
            tier: options.tier || 'starter',
            expires_in_days: options.expiresInDays
        }, {
            'Authorization': `Bearer ${this.userSession.jwt_token}`
        });

        this.log('API key created successfully');
        return response;
    }

    /**
     * Step 5: Get MCP server configuration for Claude Desktop
     * Requires API key from step 4
     * 
     * @param {string} apiKey - The API key created in step 4
     * @returns {Object} MCP server configuration for Claude Desktop
     */
    getMcpServerConfig(apiKey) {
        if (!apiKey) {
            throw new Error('API key is required for MCP server configuration');
        }

        const config = {
            "mcpServers": {
                "pierre-fitness": {
                    "command": "node",
                    "args": ["-e", `
                        const http = require('http');
                        const WebSocket = require('ws');
                        
                        // MCP over HTTP transport for Pierre
                        const server = http.createServer();
                        const wss = new WebSocket.Server({ server });
                        
                        wss.on('connection', (ws) => {
                            ws.on('message', async (message) => {
                                try {
                                    const request = JSON.parse(message);
                                    
                                    const response = await fetch('${this.serverUrl}/mcp', {
                                        method: 'POST',
                                        headers: {
                                            'Content-Type': 'application/json',
                                            'Authorization': 'Bearer ${apiKey}'
                                        },
                                        body: JSON.stringify(request)
                                    });
                                    
                                    const result = await response.json();
                                    ws.send(JSON.stringify(result));
                                } catch (error) {
                                    ws.send(JSON.stringify({
                                        error: {
                                            code: -32603,
                                            message: error.message
                                        }
                                    }));
                                }
                            });
                        });
                        
                        server.listen(0, () => {
                            console.log('Pierre MCP Bridge started on port', server.address().port);
                        });
                    `]
                }
            }
        };

        this.log('Generated MCP server configuration');
        return config;
    }

    /**
     * Complete onboarding flow - guides user through all steps
     * This is the main method for new users
     * 
     * @param {Object} userData - User registration data
     * @param {string} userData.email - User's email
     * @param {string} userData.password - User's password
     * @param {string} userData.displayName - Optional display name
     * @param {Object} oauthCredentials - OAuth app credentials
     * @param {string} oauthCredentials.provider - Provider name
     * @param {string} oauthCredentials.clientId - OAuth client ID
     * @param {string} oauthCredentials.clientSecret - OAuth client secret
     * @param {string} oauthCredentials.redirectUri - OAuth redirect URI
     * @returns {Promise<Object>} Complete onboarding result
     */
    async completeOnboarding(userData, oauthCredentials) {
        this.log('Starting complete onboarding flow');
        
        const result = {
            steps: {},
            mcpConfig: null,
            apiKey: null
        };

        try {
            // Step 1: Register user
            this.log('Step 1: User registration');
            result.steps.registration = await this.registerUser(
                userData.email,
                userData.password,
                userData.displayName
            );

            // Step 2: Wait for approval (user must check manually)
            this.log('Step 2: Checking approval status');
            const approvalStatus = await this.checkApprovalStatus(userData.email, userData.password);
            result.steps.approval = approvalStatus;

            if (!approvalStatus.approved) {
                result.nextStep = 'approval_pending';
                result.message = 'Registration complete. Please wait for admin approval before proceeding.';
                return result;
            }

            // Step 3: Store OAuth credentials
            this.log('Step 3: Storing OAuth credentials');
            result.steps.oauthStorage = await this.storeOAuthCredentials(
                oauthCredentials.provider,
                oauthCredentials.clientId,
                oauthCredentials.clientSecret,
                oauthCredentials.redirectUri
            );

            // Step 4: Create API key
            this.log('Step 4: Creating API key');
            const apiKeyResponse = await this.createApiKey('MCP Client API Key');
            result.steps.apiKeyCreation = apiKeyResponse;
            result.apiKey = apiKeyResponse.api_key;

            // Step 5: Generate MCP config
            this.log('Step 5: Generating MCP configuration');
            result.mcpConfig = this.getMcpServerConfig(result.apiKey);

            result.nextStep = 'complete';
            result.message = 'Onboarding complete! Use the MCP configuration in Claude Desktop.';

            this.log('Onboarding completed successfully');
            return result;

        } catch (error) {
            this.log('Onboarding failed:', error.message);
            result.error = error.message;
            result.nextStep = 'failed';
            throw error;
        }
    }

    /**
     * Resume onboarding for users who were pending approval
     * 
     * @param {string} email - User's email
     * @param {string} password - User's password  
     * @param {Object} oauthCredentials - OAuth app credentials
     * @returns {Promise<Object>} Resumed onboarding result
     */
    async resumeOnboarding(email, password, oauthCredentials) {
        this.log('Resuming onboarding for approved user');
        
        // Check approval status and login
        const approvalStatus = await this.checkApprovalStatus(email, password);
        if (!approvalStatus.approved) {
            throw new Error('User still pending approval');
        }

        // Continue with steps 3-5
        const result = {
            steps: {
                approval: approvalStatus
            }
        };

        // Store OAuth credentials
        result.steps.oauthStorage = await this.storeOAuthCredentials(
            oauthCredentials.provider,
            oauthCredentials.clientId,
            oauthCredentials.clientSecret,
            oauthCredentials.redirectUri
        );

        // Create API key
        const apiKeyResponse = await this.createApiKey('MCP Client API Key');
        result.steps.apiKeyCreation = apiKeyResponse;
        result.apiKey = apiKeyResponse.api_key;

        // Generate MCP config
        result.mcpConfig = this.getMcpServerConfig(result.apiKey);
        result.nextStep = 'complete';
        result.message = 'Onboarding completed successfully!';

        return result;
    }

    /**
     * List fitness tools available from Pierre MCP server
     * Requires active session and API key
     * 
     * @param {string} apiKey - Valid API key
     * @returns {Promise<Object>} List of available tools
     */
    async listFitnessTools(apiKey) {
        this.log('Listing available fitness tools');
        
        const response = await this.makeRequest('POST', '/mcp', {
            jsonrpc: "2.0",
            id: 1,
            method: "tools/list",
            params: {}
        }, {
            'Authorization': `Bearer ${apiKey}`
        });

        this.log('Retrieved fitness tools list');
        return response;
    }

    /**
     * Test MCP connection with a simple request
     * 
     * @param {string} apiKey - Valid API key
     * @returns {Promise<Object>} Connection test result
     */
    async testMcpConnection(apiKey) {
        this.log('Testing MCP connection');
        
        try {
            const response = await this.makeRequest('POST', '/mcp', {
                jsonrpc: "2.0",
                id: 1,
                method: "initialize",
                params: {
                    protocolVersion: "2025-06-18",
                    capabilities: {
                        tools: {}
                    },
                    clientInfo: {
                        name: "pierre-sdk-test",
                        version: "1.0.0"
                    }
                }
            }, {
                'Authorization': `Bearer ${apiKey}`
            });

            this.log('MCP connection test successful');
            return {
                success: true,
                serverInfo: response.result
            };
        } catch (error) {
            this.log('MCP connection test failed:', error.message);
            return {
                success: false,
                error: error.message
            };
        }
    }
}

// Export for different environments
if (typeof module !== 'undefined' && module.exports) {
    // Node.js environment
    module.exports = PierreClientSDK;
} else if (typeof window !== 'undefined') {
    // Browser environment
    window.PierreClientSDK = PierreClientSDK;
}

// Example usage documentation
const USAGE_EXAMPLES = {
    // Complete new user onboarding
    newUserFlow: `
// Initialize SDK
const pierre = new PierreClientSDK('http://localhost:3030', { debug: true });

// Complete onboarding for new user
const result = await pierre.completeOnboarding(
    {
        email: 'user@example.com',
        password: 'SecurePassword123',
        displayName: 'John Doe'
    },
    {
        provider: 'strava',
        clientId: 'your_strava_client_id',
        clientSecret: 'your_strava_client_secret',
        redirectUri: 'http://localhost:3030/oauth/strava/callback'
    }
);

if (result.nextStep === 'approval_pending') {
    console.log('Please wait for admin approval');
} else if (result.nextStep === 'complete') {
    console.log('MCP Config:', JSON.stringify(result.mcpConfig, null, 2));
    console.log('API Key:', result.apiKey);
}
`,

    // Resume for approved user
    resumeFlow: `
// Resume onboarding after approval
const result = await pierre.resumeOnboarding(
    'user@example.com',
    'SecurePassword123',
    {
        provider: 'strava',
        clientId: 'your_strava_client_id',
        clientSecret: 'your_strava_client_secret',
        redirectUri: 'http://localhost:3030/oauth/strava/callback'
    }
);

console.log('MCP Config:', JSON.stringify(result.mcpConfig, null, 2));
`,

    // Test connection
    testConnection: `
// Test MCP connection
const apiKey = 'your_api_key_here';
const testResult = await pierre.testMcpConnection(apiKey);
console.log('Connection test:', testResult);

// List available tools
const tools = await pierre.listFitnessTools(apiKey);
console.log('Available tools:', tools);
`
};

// Add usage examples to exports
if (typeof module !== 'undefined' && module.exports) {
    module.exports.USAGE_EXAMPLES = USAGE_EXAMPLES;
} else if (typeof window !== 'undefined') {
    window.PierreClientSDK.USAGE_EXAMPLES = USAGE_EXAMPLES;
}