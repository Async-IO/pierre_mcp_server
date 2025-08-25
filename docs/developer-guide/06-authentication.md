# Authentication & Security

## Overview

Pierre MCP Server implements a comprehensive authentication system supporting multiple authentication flows for different client types. The system uses JWT tokens for user sessions, API keys for A2A communication, and OAuth2 for fitness provider integration.

## Authentication Architecture

```mermaid
graph TB
    subgraph "Client Types"
        WebApp[Web Application]
        MCP[MCP Client/Claude]
        A2A[A2A System]
        Admin[Admin Interface]
    end
    
    subgraph "Authentication Layer"
        AuthManager[Auth Manager]
        JWT[JWT Service]
        OAuth[OAuth Manager]
        ApiKeys[API Key Service]
    end
    
    subgraph "Token Storage"
        UserTokens[(User JWT Tokens)]
        ApiKeyStore[(API Keys)]
        OAuthTokens[(OAuth Tokens)]
        RefreshTokens[(Refresh Tokens)]
    end
    
    subgraph "External Providers"
        Strava[Strava OAuth]
        Fitbit[Fitbit OAuth]
    end
    
    WebApp --> AuthManager
    MCP --> AuthManager
    A2A --> ApiKeys
    Admin --> AuthManager
    
    AuthManager --> JWT
    AuthManager --> OAuth
    
    JWT --> UserTokens
    JWT --> RefreshTokens
    ApiKeys --> ApiKeyStore
    OAuth --> OAuthTokens
    
    OAuth --> Strava
    OAuth --> Fitbit
```

## Complete User Flow Examples

### Cloud Deployment Setup

For these examples, assume Pierre is deployed at:
- **Production HTTP API**: `https://pierre-api.example.com` (port 8081 behind load balancer)
- **Production MCP**: `https://pierre-api.example.com:8080` (MCP protocol)
- **Development HTTP API**: `http://localhost:8081` (authentication and admin)
- **Development MCP**: `http://localhost:8080` (MCP protocol)

### 1. Admin Setup Flow

First, an administrator sets up the system and creates the first admin user.

#### Step 1: Initial Admin Setup (Server-side)

```bash
# On the cloud server, create the first admin user
docker exec -it pierre-server /app/admin-setup create-admin-user

# Or using the binary directly
./admin-setup create-admin-user --email admin@example.com --password SecurePassword123!
```

**Admin Setup Process:**
```mermaid
sequenceDiagram
    participant Admin as Administrator
    participant Setup as Admin Setup Tool
    participant DB as Database
    participant Crypto as Crypto Service
    
    Admin->>Setup: Run admin-setup
    Setup->>Crypto: Generate password hash
    Crypto-->>Setup: Hashed password
    
    Setup->>DB: Create admin user (is_admin: true)
    DB-->>Setup: Admin user created
    
    Setup->>DB: Generate and store admin JWT secret
    DB-->>Setup: JWT secret stored
    
    Setup-->>Admin: Admin user ready for token generation
    
    Note over Admin: Admin can now access system
```

**Expected Output:**
```
Admin user created successfully!

Email: admin@example.com
User ID: 550e8400-e29b-41d4-a716-446655440000
Admin privileges: ENABLED (is_admin: true)
JWT secret: GENERATED and stored in database

You can now:
1. Login to https://pierre.example.com with your credentials  
2. Generate admin tokens using: admin-setup generate-token --service "your-service"
3. Create additional admin users and manage the system
```

#### Step 2: Admin Login and Token Generation

```bash
# Admin logs in via API to get JWT token
curl -X POST https://pierre-api.example.com/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@example.com",
    "password": "SecurePassword123!"
  }'
```

**Response:**
```json
{
  "success": true,
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "refresh_token": "rt_abc123def456...",
  "expires_at": "2024-01-16T10:30:00Z",
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "admin@example.com",
    "is_admin": true,
    "tenant_id": "550e8400-e29b-41d4-a716-446655440000"
  }
}
```

### 2. User Registration and Approval Flow

#### Step 2.1: User Self-Registration

```bash
# New user registers for an account (creates pending status)
curl -X POST https://pierre-api.example.com/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "UserPassword123!",
    "display_name": "John Doe"
  }'
```

**Registration and Approval Flow:**
```mermaid
sequenceDiagram
    participant User as New User
    participant API as Auth API
    participant DB as Database
    participant Admin as Administrator
    participant Crypto as Crypto Service
    
    User->>API: POST /api/auth/register
    API->>Crypto: Hash password (bcrypt)
    Crypto-->>API: Password hash
    
    API->>DB: Create user (status: pending)
    DB-->>API: User created
    
    API-->>User: Registration success (pending approval)
    
    Note over Admin: Admin receives notification
    Admin->>API: GET /admin/pending-users
    API->>DB: List pending users
    DB-->>API: Pending users list
    API-->>Admin: Users awaiting approval
    
    Admin->>API: POST /admin/approve-user/{user_id}
    API->>DB: Update user status to active
    DB-->>API: User approved
    API-->>Admin: Approval confirmation
    
    Note over User: User can now login
```

**Registration Response:**
```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "User registered successfully. Your account is pending admin approval."
}
```

#### Step 2.2: Admin Approval Process

Administrators must approve new users before they can access the system:

```bash
# Admin lists pending users
curl -X GET https://pierre-api.example.com/admin/pending-users \
  -H "Authorization: Bearer admin_jwt_token"

# Admin approves a specific user
curl -X POST https://pierre-api.example.com/admin/approve-user/550e8400-e29b-41d4-a716-446655440000 \
  -H "Authorization: Bearer admin_jwt_token" \
  -H "Content-Type: application/json" \
  -d '{
    "reason": "Approved for production access"
  }'
```

**Admin Approval Response:**
```json
{
  "success": true,
  "message": "User user@example.com approved successfully",
  "user": {
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "user@example.com",
    "display_name": "John Doe",
    "user_status": "active",
    "approved_by": "admin-user-id",
    "approved_at": "2024-01-15T11:30:00Z"
  }
}
```

#### Step 2.3: User Login (After Approval)

Only users with `active` status can successfully login:

```bash
# User logs in to get JWT token (only works after admin approval)
curl -X POST https://pierre-api.example.com/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "UserPassword123!"
  }'
```

**Login Response (Success):**
```json
{
  "jwt_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_at": "2024-01-16T10:30:00Z",
  "user": {
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "user@example.com",
    "display_name": "John Doe"
  }
}
```

**Login Response (Pending Approval):**
```json
{
  "error": "authentication_failed",
  "message": "Your account is pending admin approval",
  "details": {
    "user_status": "pending"
  }
}
```

**Login Response (Suspended):**
```json
{
  "error": "authentication_failed", 
  "message": "Your account has been suspended",
  "details": {
    "user_status": "suspended"
  }
}
```

### 3. Provider Connection Flow (OAuth)

#### Step 3.1: Initiate Strava Connection

```bash
# User initiates Strava connection
curl -X GET "https://pierre-api.example.com/api/oauth/strava/auth" \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
```

**OAuth Flow:**
```mermaid
sequenceDiagram
    participant User as User
    participant Pierre as Pierre API
    participant Strava as Strava OAuth
    participant DB as Database
    
    User->>Pierre: GET /oauth/strava/auth
    Pierre->>Pierre: Generate OAuth state
    Pierre->>DB: Store OAuth state
    Pierre-->>User: Strava auth URL
    
    User->>Strava: Visit auth URL
    Strava->>Strava: User authorizes
    Strava-->>Pierre: Callback with auth code
    
    Pierre->>Strava: Exchange code for tokens
    Strava-->>Pierre: Access & refresh tokens
    
    Pierre->>DB: Store encrypted tokens
    Pierre-->>User: Connection successful
```

**Response:**
```json
{
  "authorization_url": "https://www.strava.com/oauth/authorize?client_id=12345&redirect_uri=https%3A//pierre-api.example.com/api/oauth/strava/callback&response_type=code&scope=read,activity:read_all&state=abc123...",
  "state": "abc123def456ghi789",
  "expires_in": 600
}
```

#### Step 3.2: OAuth Callback (Automatic)

When user authorizes on Strava, they're redirected to:
```
https://pierre-api.example.com/api/oauth/strava/callback?code=AUTH_CODE&state=abc123def456ghi789
```

**Callback Processing:**
```json
{
  "success": true,
  "provider": "strava",
  "athlete_id": "12345",
  "athlete": {
    "id": "12345",
    "username": "john_doe",
    "firstname": "John",
    "lastname": "Doe",
    "profile": "https://dgalywyr863hv.cloudfront.net/pictures/athletes/12345/123456/2/large.jpg"
  },
  "connected_at": "2024-01-15T11:45:00Z"
}
```

### 4. MCP Client Usage Flow

#### Step 4.1: MCP Client Configuration

**Claude Desktop Configuration** (`claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "node",
      "args": ["pierre-mcp-client.js"],
      "env": {
        "PIERRE_API_URL": "https://pierre-api.example.com",
        "PIERRE_MCP_URL": "wss://pierre-api.example.com:8080",
        "PIERRE_AUTH_TOKEN": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
      }
    }
  }
}
```

**MCP Client Bridge** (`pierre-mcp-client.js`):
```javascript
#!/usr/bin/env node

const { Client } = require('@modelcontextprotocol/sdk/client/index.js');
const { StdioClientTransport } = require('@modelcontextprotocol/sdk/client/stdio.js');
const WebSocket = require('ws');

class PierreMCPClient {
    constructor() {
        this.apiUrl = process.env.PIERRE_API_URL;
        this.mcpUrl = process.env.PIERRE_MCP_URL;
        this.authToken = process.env.PIERRE_AUTH_TOKEN;
        this.client = null;
    }
    
    async connect() {
        const ws = new WebSocket(this.mcpUrl, {
            headers: {
                'Authorization': `Bearer ${this.authToken}`
            }
        });
        
        const transport = new WebSocketTransport(ws);
        this.client = new Client({
            name: "pierre-mcp-client",
            version: "1.0.0"
        }, { capabilities: {} });
        
        await this.client.connect(transport);
        
        // Handle MCP protocol messages
        process.stdin.on('data', async (data) => {
            const request = JSON.parse(data.toString());
            const response = await this.handleRequest(request);
            process.stdout.write(JSON.stringify(response) + '\n');
        });
    }
    
    async handleRequest(request) {
        switch (request.method) {
            case 'initialize':
                return await this.client.initialize();
            case 'tools/list':
                return await this.client.listTools();
            case 'tools/call':
                return await this.client.callTool(request.params.name, request.params.arguments);
            default:
                return { error: { code: -32601, message: "Method not found" } };
        }
    }
}

const client = new PierreMCPClient();
client.connect().catch(console.error);
```

#### Step 4.2: MCP Protocol Handshake

**Complete MCP Flow:**
```mermaid
sequenceDiagram
    participant Claude as Claude AI
    participant Bridge as MCP Bridge
    participant Pierre as Pierre MCP Server
    participant Auth as Auth Service
    participant Strava as Strava API
    
    Claude->>Bridge: Initialize MCP
    Bridge->>Pierre: Connect WebSocket + JWT
    Pierre->>Auth: Validate JWT token
    Auth-->>Pierre: User context
    
    Bridge->>Pierre: MCP Initialize
    Pierre-->>Bridge: Server capabilities
    Bridge-->>Claude: Available tools
    
    Claude->>Bridge: tools/call get_athlete
    Bridge->>Pierre: Execute get_athlete
    Pierre->>Auth: Check permissions
    Auth-->>Pierre: Authorized
    
    Pierre->>Strava: Fetch athlete data
    Strava-->>Pierre: Athlete profile
    Pierre-->>Bridge: Formatted response
    Bridge-->>Claude: Athlete information
```

#### Step 4.3: MCP Initialize Request

```json
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
        "protocolVersion": "2025-06-18",
        "capabilities": {
            "roots": {
                "listChanged": true
            }
        },
        "clientInfo": {
            "name": "claude-desktop",
            "version": "0.4.0"
        }
    }
}
```

**Server Response:**
```json
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "protocolVersion": "2025-06-18",
        "capabilities": {
            "tools": {
                "listChanged": true
            },
            "logging": {}
        },
        "serverInfo": {
            "name": "pierre-mcp-server",
            "version": "0.1.0"
        }
    }
}
```

#### Step 4.4: Tool Discovery

```json
{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/list"
}
```

**Response:**
```json
{
    "jsonrpc": "2.0",
    "id": 2,
    "result": {
        "tools": [
            {
                "name": "get_athlete",
                "description": "Get authenticated athlete profile information",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "provider": {
                            "type": "string",
                            "enum": ["strava", "fitbit"],
                            "description": "Fitness provider to query"
                        }
                    },
                    "required": ["provider"]
                }
            },
            {
                "name": "get_activities",
                "description": "Retrieve recent activities from connected provider",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "provider": {
                            "type": "string",
                            "enum": ["strava", "fitbit"]
                        },
                        "limit": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 100,
                            "default": 10
                        },
                        "offset": {
                            "type": "integer",
                            "minimum": 0,
                            "default": 0
                        }
                    },
                    "required": ["provider"]
                }
            },
            {
                "name": "analyze_activity",
                "description": "Perform detailed analysis of a specific activity",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "provider": {"type": "string"},
                        "activity_id": {"type": "string"}
                    },
                    "required": ["provider", "activity_id"]
                }
            }
        ]
    }
}
```

#### Step 4.5: Tool Execution

**Get Athlete Profile:**
```json
{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "tools/call",
    "params": {
        "name": "get_athlete",
        "arguments": {
            "provider": "strava"
        }
    }
}
```

**Response:**
```json
{
    "jsonrpc": "2.0",
    "id": 3,
    "result": {
        "content": [
            {
                "type": "text",
                "text": "Successfully retrieved athlete profile for John Doe from Strava"
            }
        ],
        "isError": false,
        "structuredContent": {
            "id": "12345",
            "username": "john_doe",
            "firstname": "John",
            "lastname": "Doe",
            "profile": "https://dgalywyr863hv.cloudfront.net/pictures/athletes/12345/123456/2/large.jpg",
            "city": "San Francisco",
            "state": "California",
            "country": "United States",
            "sex": "M",
            "follower_count": 156,
            "friend_count": 78,
            "athlete_type": "cyclist",
            "date_preference": "%m/%d/%Y",
            "measurement_preference": "feet",
            "ftp": 285,
            "weight": 70.5
        }
    }
}
```

**Get Recent Activities:**
```json
{
    "jsonrpc": "2.0",
    "id": 4,
    "method": "tools/call",
    "params": {
        "name": "get_activities",
        "arguments": {
            "provider": "strava",
            "limit": 5
        }
    }
}
```

**Response:**
```json
{
    "jsonrpc": "2.0",
    "id": 4,
    "result": {
        "content": [
            {
                "type": "text",
                "text": "Retrieved 5 recent activities from Strava"
            }
        ],
        "isError": false,
        "structuredContent": {
            "activities": [
                {
                    "id": "10394910581",
                    "name": "Morning Run",
                    "type": "Run",
                    "distance": 5000.0,
                    "moving_time": 1800,
                    "elapsed_time": 1920,
                    "total_elevation_gain": 45.2,
                    "start_date": "2024-01-15T06:30:00Z",
                    "start_date_local": "2024-01-14T22:30:00",
                    "timezone": "America/Los_Angeles",
                    "average_speed": 2.78,
                    "max_speed": 3.89,
                    "average_heartrate": 155.4,
                    "max_heartrate": 178,
                    "elev_high": 145.6,
                    "elev_low": 98.2,
                    "calories": 387.2
                }
            ],
            "total_count": 5,
            "summary": {
                "total_distance": 23.5,
                "total_time": 8640,
                "avg_pace": "3:42/km",
                "activities_by_type": {
                    "Run": 3,
                    "Ride": 2
                }
            }
        }
    }
}
```

## Admin Architecture 

### Admin User Model

Pierre MCP Server uses explicit admin privileges through the `is_admin` boolean field in the User model, replacing the previous tier-based admin system.

#### Admin User Creation

Admin users are created using the `admin-setup` binary:

```bash
# Create first admin user
./admin-setup create-admin-user --email admin@example.com --password SecurePassword123!
```

**What happens during admin user creation:**
1. User created with `is_admin: true` (explicit admin privileges)
2. User status set to `active` (no approval required for admin)
3. Admin JWT secret generated and stored in database
4. Admin can immediately login and access admin endpoints

#### Admin Identification

```rust
// Admin lookup logic
async fn get_system_admin_user_id(database: &Database) -> Result<String> {
    let users = database.get_users_by_status("active").await?;
    
    for user in &users {
        if user.is_admin {  // ✅ NEW: Check explicit admin flag
            return Ok(user.id.to_string());
        }
    }
    
    anyhow::bail!("No active admin user found for approval operations")
}
```

#### Admin Token Generation

```bash
# Generate admin JWT token for API access
./admin-setup generate-token --service "admin-dashboard"
```

**Admin vs Regular Users:**

| Field | Admin User | Regular User |
|-------|-----------|--------------|
| `is_admin` | `true` | `false` |
| `user_status` | `active` (auto) | `pending` → `active` (after approval) |
| `approved_by` | `None` (no approval needed) | Admin user ID |
| Admin endpoints | ✅ Full access | ❌ Forbidden |
| User approval capability | ✅ Can approve others | ❌ Cannot approve |

### Migration from Tier-Based System

**Before (Deprecated):**
```rust
// OLD: Admin based on UserTier::Enterprise
if user.tier == UserTier::Enterprise {
    // Admin operations
}
```

**After (Current):**
```rust
// NEW: Admin based on explicit flag
if user.is_admin {
    // Admin operations  
}
```

This change provides:
- **Explicit admin privileges** - No ambiguity about admin status
- **Separation of concerns** - User tier is for billing/features, admin flag is for privileges
- **Security clarity** - Easy to audit who has admin access
- **Simplified logic** - Boolean check instead of enum comparison

## Security Implementation Details

### JWT Token Structure

```rust
// src/auth.rs
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,        // User ID
    pub tenant_id: String,  // Tenant ID for isolation
    pub role: String,       // User role (admin, owner, member)
    pub iat: i64,          // Issued at
    pub exp: i64,          // Expiry
    pub aud: String,       // Audience (pierre-mcp-server)
    pub iss: String,       // Issuer (pierre-api)
}

impl AuthManager {
    pub fn generate_token(&self, user: &User) -> Result<TokenResponse> {
        let now = Utc::now();
        let expiry = now + Duration::hours(self.expiry_hours);
        
        let claims = Claims {
            sub: user.id.to_string(),
            tenant_id: user.tenant_id.to_string(),
            role: user.role.to_string(),
            iat: now.timestamp(),
            exp: expiry.timestamp(),
            aud: "pierre-mcp-server".to_string(),
            iss: "pierre-api".to_string(),
        };
        
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(&self.secret)
        )?;
        
        let refresh_token = self.generate_refresh_token()?;
        
        Ok(TokenResponse {
            token,
            refresh_token,
            expires_at: expiry,
            token_type: "Bearer".to_string(),
        })
    }
}
```

### Authentication Middleware

```rust
// src/auth.rs
pub struct McpAuthMiddleware {
    auth_manager: AuthManager,
    database: Arc<Database>,
}

impl McpAuthMiddleware {
    pub async fn authenticate(&self, headers: &HeaderMap) -> Result<AuthResult> {
        // Extract Bearer token
        let auth_header = headers.get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or(AuthError::MissingToken)?;
        
        if !auth_header.starts_with("Bearer ") {
            return Err(AuthError::InvalidTokenFormat);
        }
        
        let token = &auth_header[7..];
        
        // Verify and decode JWT
        let claims = self.auth_manager.verify_token(token)?;
        
        // Load user context
        let user = self.database
            .get_user(&Uuid::parse_str(&claims.sub)?)
            .await?
            .ok_or(AuthError::UserNotFound)?;
        
        // Create tenant context
        let tenant_context = TenantContext {
            tenant_id: Uuid::parse_str(&claims.tenant_id)?,
            user_id: user.id,
            role: TenantRole::from_str(&claims.role)?,
            rate_limit_multiplier: user.rate_limit_multiplier,
            features: user.enabled_features,
        };
        
        Ok(AuthResult::Authenticated {
            user_id: user.id,
            tenant_context,
        })
    }
}
```

### Password Security

```rust
// src/auth.rs
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{SaltString, rand_core::OsRng};

pub struct PasswordManager;

impl PasswordManager {
    pub fn hash_password(password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)?
            .to_string();
        
        Ok(password_hash)
    }
    
    pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
        let parsed_hash = PasswordHash::new(hash)?;
        let argon2 = Argon2::default();
        
        match argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }
}
```

### Token Refresh Flow

```bash
# When JWT token expires, refresh it
curl -X POST https://pierre-api.example.com/api/auth/refresh \
  -H "Content-Type: application/json" \
  -d '{
    "refresh_token": "rt_def789ghi012..."
  }'
```

**Response:**
```json
{
  "success": true,
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "refresh_token": "rt_new456token789...",
  "expires_at": "2024-01-16T12:30:00Z"
}
```

## Error Handling

### Authentication Errors

```json
// Invalid token
{
  "error": {
    "code": "AUTH_INVALID_TOKEN",
    "message": "Invalid or expired token",
    "details": {
      "token_expired": true,
      "expired_at": "2024-01-15T10:30:00Z"
    }
  }
}

// Missing token
{
  "error": {
    "code": "AUTH_MISSING_TOKEN",
    "message": "Authorization token required",
    "details": {
      "expected_header": "Authorization: Bearer <token>"
    }
  }
}

// Insufficient permissions
{
  "error": {
    "code": "AUTH_INSUFFICIENT_PERMISSIONS",
    "message": "Insufficient permissions for this operation",
    "details": {
      "required_role": "admin",
      "current_role": "member"
    }
  }
}
```

### OAuth Errors

```json
// Provider not connected
{
  "error": {
    "code": "OAUTH_PROVIDER_NOT_CONNECTED",
    "message": "Strava account not connected",
    "details": {
      "provider": "strava",
      "connect_url": "https://pierre-api.example.com/api/oauth/strava/auth"
    }
  }
}

// OAuth token expired
{
  "error": {
    "code": "OAUTH_TOKEN_EXPIRED",
    "message": "Provider authorization has expired",
    "details": {
      "provider": "strava",
      "expired_at": "2024-01-10T15:30:00Z",
      "reconnect_url": "https://pierre-api.example.com/api/oauth/strava/auth"
    }
  }
}
```

## Rate Limiting

### User Rate Limits

```rust
// src/rate_limiting.rs
#[derive(Debug, Clone)]
pub struct RateLimit {
    pub requests_per_minute: u32,
    pub burst_size: u32,
    pub window_seconds: u64,
}

impl Default for RateLimit {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            burst_size: 10,
            window_seconds: 60,
        }
    }
}

pub struct UserRateLimiter {
    limits: HashMap<TenantRole, RateLimit>,
    usage: HashMap<Uuid, TokenBucket>,
}

impl UserRateLimiter {
    pub fn new() -> Self {
        let mut limits = HashMap::new();
        limits.insert(TenantRole::Admin, RateLimit {
            requests_per_minute: 300,
            burst_size: 50,
            window_seconds: 60,
        });
        limits.insert(TenantRole::Owner, RateLimit {
            requests_per_minute: 120,
            burst_size: 20,
            window_seconds: 60,
        });
        limits.insert(TenantRole::Member, RateLimit {
            requests_per_minute: 60,
            burst_size: 10,
            window_seconds: 60,
        });
        
        Self {
            limits,
            usage: HashMap::new(),
        }
    }
}
```

## Production Security Considerations

### Two-Tier Key Management System

Pierre implements a secure two-tier key management system with automatic bootstrap and mode detection:

**Tier 1: Master Encryption Key (MEK)**
- Loaded from `PIERRE_MASTER_ENCRYPTION_KEY` environment variable (production)
- Auto-generated with warnings if not set (development)
- Used exclusively to encrypt/decrypt the Database Encryption Key
- Must be stored securely (HSM, Kubernetes secrets, etc.) in production
- Never stored in database or application files

**Tier 2: Database Encryption Key (DEK)**
- Generated automatically on first startup using secure random
- Stored encrypted with MEK in database as base64-encoded value
- Used for all actual data encryption operations (OAuth tokens, user data)
- Can be rotated independently without changing MEK
- Seamlessly loads from database on subsequent starts

**Bootstrap Process:**
1. **Phase 1**: Load MEK from environment, generate temporary DEK
2. **Phase 2**: Initialize database with temporary DEK  
3. **Phase 3**: Load existing encrypted DEK or store current DEK encrypted with MEK

**Development to Production Migration:**
- Copy generated MEK from development logs to production environment
- Database transfers seamlessly - no data migration required

### Environment Variables

**Development Environment** (MEK auto-generated):
```bash
# Development - no MEK required, system generates and logs it
export DATABASE_URL="sqlite:./data/users.db"

# OAuth credentials for testing
export STRAVA_CLIENT_ID="your_test_strava_client_id"
export STRAVA_CLIENT_SECRET="your_test_strava_client_secret"

# System will log: "Generated MEK (save for production): PIERRE_MASTER_ENCRYPTION_KEY=<key>"
```

**Production Environment** (explicit MEK):
```bash
# Production deployment environment
export DATABASE_URL="postgresql://pierre:$DB_PASSWORD@db.pierre.com:5432/pierre_prod"

# Two-tier key management - CRITICAL for production
export PIERRE_MASTER_ENCRYPTION_KEY="base64_encoded_32_byte_key"  # From development logs or newly generated

export JWT_EXPIRY_HOURS=24
export REFRESH_TOKEN_EXPIRY_DAYS=30

# OAuth credentials
export STRAVA_CLIENT_ID="your_production_strava_client_id"
export STRAVA_CLIENT_SECRET="your_production_strava_client_secret"
export FITBIT_CLIENT_ID="your_production_fitbit_client_id"
export FITBIT_CLIENT_SECRET="your_production_fitbit_client_secret"

# Server configuration
export PIERRE_MCP_PORT=8080
export PIERRE_HTTP_PORT=8081
export PIERRE_BASE_URL="https://pierre-api.example.com"

# Security settings
export BCRYPT_COST=12
export RATE_LIMIT_ENABLED=true
export AUDIT_LOGGING=true
export CORS_ORIGINS="https://pierre.example.com"
```

**Generate New Production MEK:**
```bash
# Generate a new MEK for production
export PIERRE_MASTER_ENCRYPTION_KEY="$(openssl rand -base64 32)"
echo "Store securely: $PIERRE_MASTER_ENCRYPTION_KEY"
```

### Docker Compose with Security

```yaml
# docker-compose.prod.yml
version: '3.8'

services:
  pierre-api:
    image: pierre-mcp-server:latest
    ports:
      - "8080:8080"
      - "8081:8081"
    environment:
      - DATABASE_URL=postgresql://pierre:${DB_PASSWORD}@postgres:5432/pierre
      # Two-tier key management - MEK from secure source
      - PIERRE_MASTER_ENCRYPTION_KEY_FILE=/run/secrets/master_encryption_key
    secrets:
      - master_encryption_key
      - db_password
    depends_on:
      - postgres
    restart: unless-stopped
    
  postgres:
    image: postgres:15
    environment:
      - POSTGRES_DB=pierre
      - POSTGRES_USER=pierre
      - POSTGRES_PASSWORD_FILE=/run/secrets/db_password
    secrets:
      - db_password
    volumes:
      - postgres_data:/var/lib/postgresql/data
    restart: unless-stopped

secrets:
  master_encryption_key:
    file: ./secrets/master_encryption_key
  db_password:
    file: ./secrets/db_password

volumes:
  postgres_data:
```

### Health Check Endpoint

```bash
# Check authentication system health
curl -X GET https://pierre-api.example.com/api/health/auth \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
```

**Response:**
```json
{
  "status": "healthy",
  "components": {
    "jwt_service": "healthy",
    "password_service": "healthy",
    "oauth_providers": {
      "strava": "healthy",
      "fitbit": "healthy"
    },
    "rate_limiter": "healthy",
    "token_store": "healthy"
  },
  "metrics": {
    "active_sessions": 1247,
    "tokens_issued_today": 3456,
    "failed_auth_attempts_today": 23,
    "oauth_connections_active": 892
  },
  "timestamp": "2024-01-15T12:00:00Z"
}
```

This comprehensive authentication documentation covers the complete user journey from admin setup to MCP client usage, including all security considerations for production deployment.