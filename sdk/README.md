# Pierre MCP Server Client SDK

A comprehensive JavaScript SDK for interacting with the Pierre MCP Server from Claude Desktop and other MCP clients. This SDK handles the complete user onboarding flow including registration, admin approval, OAuth credential storage, and API key management.

## Features

- **User Registration**: Register new users with pending approval status
- **Admin Approval Flow**: Check approval status and handle pending users
- **OAuth Credential Storage**: Securely store per-user OAuth app credentials
- **API Key Management**: Create and manage API keys for MCP access
- **MCP Configuration**: Generate Claude Desktop configuration automatically
- **Connection Testing**: Validate MCP connections and list available tools
- **Error Handling**: Comprehensive error handling with debugging support

## Installation

### For Claude Desktop (Node.js environment)

1. Save the SDK file to your local machine:
```bash
curl -o pierre-client-sdk.js https://raw.githubusercontent.com/your-repo/pierre_mcp_server/main/sdk/pierre-client-sdk.js
```

2. Or copy the SDK file directly from this repository.

### For Browser/Web environments

Include the SDK in your HTML:
```html
<script src="pierre-client-sdk.js"></script>
```

## Quick Start

### 1. Initialize the SDK

```javascript
const PierreClientSDK = require('./pierre-client-sdk.js'); // Node.js
// or use window.PierreClientSDK in browser

const pierre = new PierreClientSDK('http://localhost:3030', { 
    debug: true,
    timeout: 30000 
});
```

### 2. Complete New User Onboarding

For first-time users who need to register and set up their account:

```javascript
async function setupNewUser() {
    try {
        const result = await pierre.completeOnboarding(
            // User registration data
            {
                email: 'athlete@example.com',
                password: 'SecurePassword123',
                displayName: 'Elite Athlete'
            },
            // OAuth app credentials (from your Strava/Fitbit app)
            {
                provider: 'strava',
                clientId: 'your_strava_client_id',
                clientSecret: 'your_strava_client_secret', 
                redirectUri: 'http://localhost:3030/oauth/strava/callback'
            }
        );

        if (result.nextStep === 'approval_pending') {
            console.log('✅ Registration successful!');
            console.log('⏳ Your account is pending admin approval.');
            console.log('📧 Please contact the administrator to approve your account.');
            console.log('🔄 Run the resume flow after approval.');
        } else if (result.nextStep === 'complete') {
            console.log('🎉 Onboarding complete!');
            console.log('🔑 API Key:', result.apiKey);
            console.log('⚙️  Add this to your Claude Desktop config:');
            console.log(JSON.stringify(result.mcpConfig, null, 2));
        }
    } catch (error) {
        console.error('❌ Onboarding failed:', error.message);
    }
}

setupNewUser();
```

### 3. Resume After Admin Approval

For users whose accounts have been approved by an administrator:

```javascript
async function resumeAfterApproval() {
    try {
        const result = await pierre.resumeOnboarding(
            'athlete@example.com',
            'SecurePassword123',
            {
                provider: 'strava',
                clientId: 'your_strava_client_id',
                clientSecret: 'your_strava_client_secret',
                redirectUri: 'http://localhost:3030/oauth/strava/callback'
            }
        );

        console.log('🎉 Onboarding completed!');
        console.log('🔑 API Key:', result.apiKey);
        console.log('⚙️  Add this to your Claude Desktop config:');
        console.log(JSON.stringify(result.mcpConfig, null, 2));
        
        // Save API key for future use
        console.log('💾 Save this API key:', result.apiKey);
        
    } catch (error) {
        console.error('❌ Resume failed:', error.message);
    }
}

resumeAfterApproval();
```

### 4. Test Your Setup

After completing onboarding, test the MCP connection:

```javascript
async function testSetup() {
    const apiKey = 'your_api_key_from_onboarding';
    
    // Test MCP connection
    const connectionTest = await pierre.testMcpConnection(apiKey);
    if (connectionTest.success) {
        console.log('✅ MCP connection successful');
        console.log('📋 Server info:', connectionTest.serverInfo);
    } else {
        console.log('❌ MCP connection failed:', connectionTest.error);
        return;
    }
    
    // List available fitness tools
    const tools = await pierre.listFitnessTools(apiKey);
    console.log('🏃 Available fitness tools:');
    tools.result.tools.forEach(tool => {
        console.log(`  - ${tool.name}: ${tool.description}`);
    });
}

testSetup();
```

## Step-by-Step Manual Process

If you prefer to go through each step manually:

### Step 1: Register User

```javascript
const registration = await pierre.registerUser(
    'athlete@example.com',
    'SecurePassword123',
    'Elite Athlete'
);
console.log('User registered:', registration.user_id);
```

### Step 2: Check Approval Status

```javascript
const approval = await pierre.checkApprovalStatus(
    'athlete@example.com',
    'SecurePassword123'
);

if (approval.approved) {
    console.log('User approved! Session:', approval.session);
} else {
    console.log('Still pending approval:', approval.message);
}
```

### Step 3: Store OAuth Credentials

```javascript
// After approval and login
await pierre.storeOAuthCredentials(
    'strava',
    'your_strava_client_id',
    'your_strava_client_secret',
    'http://localhost:3030/oauth/strava/callback'
);
```

### Step 4: Create API Key

```javascript
const apiKeyResponse = await pierre.createApiKey('My MCP Client', {
    tier: 'starter',
    expiresInDays: 365
});
console.log('API Key:', apiKeyResponse.api_key);
```

### Step 5: Generate MCP Config

```javascript
const mcpConfig = pierre.getMcpServerConfig(apiKeyResponse.api_key);
console.log('Claude Desktop config:', JSON.stringify(mcpConfig, null, 2));
```

## Configuration for Claude Desktop

After successful onboarding, add the generated MCP configuration to your Claude Desktop settings:

1. Open Claude Desktop
2. Go to Settings > MCP Servers
3. Add the configuration provided by the SDK:

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "node",
      "args": ["-e", "/* Bridge code generated by SDK */"]
    }
  }
}
```

## OAuth App Setup

Before using the SDK, you need to create OAuth applications with fitness providers:

### Strava OAuth App Setup

1. Go to [Strava API Settings](https://www.strava.com/settings/api)
2. Create a new application
3. Set the redirect URI to: `http://localhost:3030/oauth/strava/callback`
4. Note down your Client ID and Client Secret

### Fitbit OAuth App Setup

1. Go to [Fitbit Developer Console](https://dev.fitbit.com/apps)
2. Register a new application
3. Set the redirect URI to: `http://localhost:3030/oauth/fitbit/callback`
4. Note down your Client ID and Client Secret

## Admin Approval Process

For administrators who need to approve user accounts:

1. Users register through the SDK with "pending" status
2. Administrators can view pending users at: `GET /admin/pending-users`
3. Administrators approve users via: `POST /admin/approve-user/{user_id}`
4. Approved users can then complete their onboarding

## Error Handling

The SDK includes comprehensive error handling:

```javascript
try {
    await pierre.completeOnboarding(userData, oauthCredentials);
} catch (error) {
    if (error.message.includes('pending admin approval')) {
        console.log('Account needs approval');
    } else if (error.message.includes('Invalid email format')) {
        console.log('Check email format');
    } else if (error.message.includes('Password must be at least 8 characters')) {
        console.log('Password too short');
    } else {
        console.log('Unexpected error:', error.message);
    }
}
```

## Debug Mode

Enable debug logging to troubleshoot issues:

```javascript
const pierre = new PierreClientSDK('http://localhost:3030', { debug: true });
```

This will log all HTTP requests and responses to the console.

## API Reference

### Constructor

```javascript
new PierreClientSDK(serverUrl, options)
```

- `serverUrl` (string): Base URL of Pierre MCP Server
- `options.timeout` (number): Request timeout in ms (default: 30000)
- `options.debug` (boolean): Enable debug logging (default: false)

### Methods

#### `registerUser(email, password, displayName)`
Register a new user account (status: pending)

#### `checkApprovalStatus(email, password)`
Check if user account has been approved by admin

#### `storeOAuthCredentials(provider, clientId, clientSecret, redirectUri)`
Store OAuth app credentials for a fitness provider

#### `createApiKey(description, options)`
Create an API key for MCP access

#### `getMcpServerConfig(apiKey)`
Generate MCP server configuration for Claude Desktop

#### `completeOnboarding(userData, oauthCredentials)`
Complete full onboarding flow for new users

#### `resumeOnboarding(email, password, oauthCredentials)`
Resume onboarding for approved users

#### `testMcpConnection(apiKey)`
Test MCP connection and server availability

#### `listFitnessTools(apiKey)`
List available fitness tools from the MCP server

## Troubleshooting

### Common Issues

1. **"User pending admin approval"**
   - Solution: Contact administrator to approve your account

2. **"Invalid email or password"**
   - Solution: Check credentials or register first

3. **"Connection refused"**
   - Solution: Ensure Pierre MCP Server is running on the specified URL

4. **"OAuth credentials validation failed"**
   - Solution: Verify your OAuth app client ID and secret

5. **"API key authentication failed"**
   - Solution: Ensure API key is valid and not expired

### Debug Steps

1. Enable debug mode: `{ debug: true }`
2. Check server logs for detailed error messages
3. Verify server is running: `curl http://localhost:3030/health`
4. Test admin endpoints if you have admin access

## Security Considerations

- Store API keys securely - never commit them to version control
- OAuth client secrets should be kept confidential
- Use HTTPS in production environments
- Regularly rotate API keys and OAuth credentials
- Monitor usage and access patterns

## Support

For issues and questions:

1. Check the server logs for detailed error messages
2. Enable debug mode in the SDK
3. Verify all prerequisites (OAuth apps, server running, etc.)
4. Contact the Pierre development team

## License

This SDK is part of the Pierre MCP Server project. See the main project LICENSE file for details.