# Sequence Diagrams

This document contains sequence diagrams for the key flows in Pierre MCP Server, illustrating the interactions between components and external systems.

## Table of Contents

1. [MCP Connection and Tool Execution Flow](#mcp-connection-and-tool-execution-flow)
2. [OAuth Authentication Flow](#oauth-authentication-flow)
3. [A2A Registration and Discovery Flow](#a2a-registration-and-discovery-flow)
4. [API Key Creation and Usage Flow](#api-key-creation-and-usage-flow)
5. [Rate Limiting Enforcement Flow](#rate-limiting-enforcement-flow)
6. [Multi-Tenant Data Isolation Flow](#multi-tenant-data-isolation-flow)
7. [Database Migration Flow](#database-migration-flow)
8. [WebSocket Connection Management](#websocket-connection-management)

## MCP Connection and Tool Execution Flow

```mermaid
sequenceDiagram
    participant Client as MCP Client
    participant Server as Pierre Server
    participant Auth as Auth Manager
    participant Provider as Provider Manager
    participant Strava as Strava API
    participant DB as Database

    Note over Client, DB: MCP Protocol v2025-06-18 Flow

    Client->>Server: WebSocket Connection
    Server->>Client: Connection Established

    Client->>Server: initialize request
    Note right of Client: {<br/>  "jsonrpc": "2.0",<br/>  "method": "initialize",<br/>  "params": {<br/>    "protocolVersion": "2025-06-18",<br/>    "capabilities": {...}<br/>  }<br/>}

    Server->>Client: initialize response
    Note left of Server: Server capabilities<br/>and protocol info

    Client->>Server: notifications/initialized

    Client->>Server: tools/list request
    Server->>Client: tools/list response
    Note left of Server: Available tools:<br/>- get_activities<br/>- get_athlete_stats<br/>- upload_activity

    Client->>Server: tools/call request
    Note right of Client: {<br/>  "method": "tools/call",<br/>  "params": {<br/>    "name": "get_activities",<br/>    "arguments": {<br/>      "limit": 10<br/>    }<br/>  }<br/>}

    Server->>Auth: validate_api_key()
    Auth->>DB: get_api_key_by_prefix()
    DB-->>Auth: API Key Details
    Auth-->>Server: User ID & Permissions

    Server->>Provider: get_activities(user_id, args)
    Provider->>DB: get_user_tokens(user_id)
    DB-->>Provider: OAuth Tokens
    
    alt Token Valid
        Provider->>Strava: GET /athlete/activities
        Strava-->>Provider: Activities Data
        Provider-->>Server: Processed Activities
        Server->>DB: update_last_used(api_key_id)
        Server->>DB: record_usage_stats()
        Server-->>Client: tools/call response (success)
    else Token Expired
        Provider->>Strava: POST /oauth/token (refresh)
        Strava-->>Provider: New Access Token
        Provider->>DB: update_user_tokens()
        Provider->>Strava: GET /athlete/activities
        Strava-->>Provider: Activities Data
        Provider-->>Server: Processed Activities
        Server-->>Client: tools/call response (success)
    else Refresh Failed
        Server-->>Client: tools/call response (error)
        Note left of Server: {<br/>  "error": {<br/>    "code": -32603,<br/>    "message": "Token refresh failed"<br/>  }<br/>}
    end
```

## OAuth Authentication Flow

```mermaid
sequenceDiagram
    participant User as End User
    participant Client as Web Client
    participant Server as Pierre Server
    participant Auth as Auth Manager
    participant Strava as Strava OAuth
    participant DB as Database

    Note over User, DB: Complete OAuth 2.0 Authorization Code Flow

    User->>Client: Click "Connect Strava"
    Client->>Server: GET /oauth/strava/authorize
    
    Server->>Auth: get_oauth_config(tenant_id)
    Auth->>DB: get_tenant_oauth_config()
    DB-->>Auth: OAuth Credentials
    
    Server->>Server: generate_state_token()
    Server->>DB: store_oauth_state(state, user_id)
    
    Server-->>Client: Redirect to Strava
    Note right of Server: https://www.strava.com/oauth/authorize?<br/>client_id=xxx&<br/>response_type=code&<br/>redirect_uri=xxx&<br/>scope=read,activity:read&<br/>state=xxx

    Client->>Strava: Authorization Request
    Strava->>User: Login & Consent Screen
    User->>Strava: Grant Permission
    
    Strava-->>Client: Redirect with Auth Code
    Note left of Strava: /oauth/strava/callback?<br/>code=xxx&state=xxx

    Client->>Server: GET /oauth/strava/callback?code=xxx&state=xxx
    
    Server->>DB: validate_oauth_state(state)
    DB-->>Server: State Valid, User ID
    
    Server->>Strava: POST /oauth/token
    Note right of Server: {<br/>  "client_id": "xxx",<br/>  "client_secret": "xxx",<br/>  "code": "xxx",<br/>  "grant_type": "authorization_code"<br/>}
    
    Strava-->>Server: Access & Refresh Tokens
    Note left of Strava: {<br/>  "access_token": "xxx",<br/>  "refresh_token": "xxx",<br/>  "expires_at": 1234567890<br/>}
    
    Server->>Server: encrypt_tokens()
    Server->>DB: store_user_tokens(user_id, encrypted_tokens)
    Server->>DB: cleanup_oauth_state(state)
    
    Server->>Auth: generate_jwt(user_id)
    Auth-->>Server: JWT Token
    
    Server-->>Client: Success Response + JWT
    Note left of Server: {<br/>  "access_token": "jwt_token",<br/>  "token_type": "Bearer",<br/>  "expires_in": 3600<br/>}
    
    Client->>Client: Store JWT in localStorage
    Client-->>User: "Successfully connected to Strava!"
```

## A2A Registration and Discovery Flow

```mermaid
sequenceDiagram
    participant Agent as External Agent
    participant Server as Pierre Server
    participant Auth as Auth Manager
    participant Registry as A2A Registry
    participant DB as Database

    Note over Agent, DB: Agent-to-Agent Protocol Registration

    Agent->>Server: POST /a2a/register
    Note right of Agent: {<br/>  "agent_id": "discord-bot-001",<br/>  "capabilities": ["webhook", "notification"],<br/>  "metadata": {<br/>    "name": "Discord Training Bot",<br/>    "version": "1.0.0"<br/>  }<br/>}

    Server->>Auth: validate_api_key(request.headers)
    Auth->>DB: get_api_key_by_prefix()
    DB-->>Auth: API Key Details
    Auth-->>Server: User ID & Permissions

    alt Valid API Key
        Server->>Registry: register_agent(user_id, agent_info)
        Registry->>DB: store_agent_registration()
        Registry->>DB: update_agent_capabilities()
        
        Registry-->>Server: Registration Success
        Server-->>Agent: 201 Created
        Note left of Server: {<br/>  "agent_id": "discord-bot-001",<br/>  "status": "registered",<br/>  "registered_at": "2024-01-15T10:30:00Z"<br/>}
    else Invalid API Key
        Server-->>Agent: 401 Unauthorized
    end

    Note over Agent, DB: Agent Discovery Process

    Agent->>Server: GET /a2a/agents/discover?capability=webhook
    Server->>Auth: validate_api_key(request.headers)
    Auth-->>Server: User ID

    Server->>Registry: discover_agents(user_id, filters)
    Registry->>DB: query_agents_by_capability()
    DB-->>Registry: Matching Agents

    Registry-->>Server: Agent List
    Server-->>Agent: 200 OK
    Note left of Server: {<br/>  "agents": [<br/>    {<br/>      "agent_id": "analytics-001",<br/>      "capabilities": ["webhook", "analytics"],<br/>      "metadata": {...}<br/>    }<br/>  ]<br/>}

    Note over Agent, DB: Agent Card Exchange

    Agent->>Server: GET /a2a/agents/analytics-001/card
    Server->>Registry: get_agent_card(agent_id)
    Registry->>DB: get_agent_details()
    DB-->>Registry: Agent Card Data

    Registry-->>Server: Agent Card
    Server-->>Agent: 200 OK
    Note left of Server: {<br/>  "agent_card": {<br/>    "agent_id": "analytics-001",<br/>    "name": "Analytics Engine",<br/>    "description": "Real-time fitness analytics",<br/>    "endpoints": [<br/>      "/webhook/activity",<br/>      "/analytics/summary"<br/>    ],<br/>    "schemas": {...}<br/>  }<br/>}
```

## API Key Creation and Usage Flow

```mermaid
sequenceDiagram
    participant User as End User
    participant Client as Web Client
    participant Server as Pierre Server
    participant Auth as Auth Manager
    participant KeyMgr as API Key Manager
    participant DB as Database

    Note over User, DB: API Key Lifecycle Management

    User->>Client: Request New API Key
    Client->>Server: POST /api/keys/create
    Note right of Client: Authorization: Bearer jwt_token<br/>{<br/>  "name": "Production Key",<br/>  "description": "Main app key",<br/>  "tier": "premium"<br/>}

    Server->>Auth: validate_jwt(bearer_token)
    Auth-->>Server: User ID

    Server->>KeyMgr: create_api_key(user_id, request)
    KeyMgr->>KeyMgr: generate_secure_key()
    KeyMgr->>KeyMgr: create_key_prefix()
    
    KeyMgr-->>Server: (api_key_record, full_key)
    Server->>DB: create_api_key(api_key_record)
    
    Server-->>Client: 201 Created
    Note left of Server: {<br/>  "api_key": "pk_live_abc123...",<br/>  "key_info": {<br/>    "id": "key_456",<br/>    "name": "Production Key",<br/>    "tier": "premium",<br/>    "key_prefix": "pk_live_abc123"<br/>  },<br/>  "warning": "Store securely - won't be shown again"<br/>}

    Client-->>User: Display API Key (one-time)
    User->>User: Store API Key Securely

    Note over User, DB: API Key Usage in MCP Client

    participant MCP as MCP Client
    
    User->>MCP: Configure with API Key
    MCP->>Server: WebSocket + API Key Header
    Note right of MCP: X-API-Key: pk_live_abc123...

    Server->>Auth: validate_api_key(api_key)
    Auth->>DB: get_api_key_by_prefix("pk_live_abc123")
    DB-->>Auth: API Key Details

    alt Valid & Active Key
        Auth->>DB: update_last_used(key_id)
        Auth-->>Server: User ID & Permissions
        Server-->>MCP: Connection Accepted
    else Invalid Key
        Auth-->>Server: Authentication Failed
        Server-->>MCP: 401 Unauthorized
    else Inactive Key
        Auth-->>Server: Key Deactivated
        Server-->>MCP: 403 Forbidden
    end

    Note over User, DB: Key Management Operations

    User->>Client: View API Keys
    Client->>Server: GET /api/keys/list
    Server->>Auth: validate_jwt(bearer_token)
    Server->>DB: get_user_api_keys(user_id)
    DB-->>Server: User's API Keys
    Server-->>Client: Key List (without secrets)

    User->>Client: Deactivate Key
    Client->>Server: DELETE /api/keys/{key_id}
    Server->>DB: deactivate_api_key(key_id, user_id)
    Server-->>Client: 200 OK - Key Deactivated
```

## Rate Limiting Enforcement Flow

```mermaid
sequenceDiagram
    participant Client as API Client
    participant Middleware as Rate Limit Middleware
    participant Limiter as Rate Limiter
    participant Cache as Redis Cache
    participant Handler as Route Handler

    Note over Client, Handler: Token Bucket Algorithm Implementation

    Client->>Middleware: API Request
    Note right of Client: X-API-Key: pk_live_abc123...

    Middleware->>Limiter: check_rate_limit(api_key, endpoint)
    
    Limiter->>Cache: GET rate_limit:api_key:pk_live_abc123
    
    alt Cache Hit
        Cache-->>Limiter: {tokens: 45, last_refill: 1640995200}
        Limiter->>Limiter: calculate_refill_amount()
        Note right of Limiter: tokens = min(bucket_size,<br/>current_tokens + refill_amount)
        
        alt Tokens Available
            Limiter->>Limiter: consume_token()
            Limiter->>Cache: SET rate_limit:api_key:pk_live_abc123
            Note right of Limiter: {tokens: 44, last_refill: now}
            
            Limiter-->>Middleware: ALLOW (tokens: 44)
            Middleware->>Middleware: add_rate_limit_headers()
            Middleware->>Handler: Forward Request
            
            Handler-->>Middleware: Response
            Middleware-->>Client: Response + Rate Limit Headers
            Note left of Middleware: X-RateLimit-Limit: 100<br/>X-RateLimit-Remaining: 44<br/>X-RateLimit-Reset: 1640995260
            
        else No Tokens Available
            Limiter-->>Middleware: DENY (retry_after: 60)
            Middleware-->>Client: 429 Too Many Requests
            Note left of Middleware: Retry-After: 60<br/>X-RateLimit-Limit: 100<br/>X-RateLimit-Remaining: 0<br/>X-RateLimit-Reset: 1640995260
        end
        
    else Cache Miss
        Limiter->>Limiter: initialize_bucket()
        Note right of Limiter: New bucket with<br/>full token capacity
        
        Limiter->>Cache: SET rate_limit:api_key:pk_live_abc123
        Note right of Limiter: {tokens: 99, last_refill: now}
        
        Limiter-->>Middleware: ALLOW (tokens: 99)
        Middleware->>Handler: Forward Request
        Handler-->>Middleware: Response
        Middleware-->>Client: Response + Rate Limit Headers
    end
```

## Multi-Tenant Data Isolation Flow

```mermaid
sequenceDiagram
    participant Client1 as Tenant A Client
    participant Client2 as Tenant B Client
    participant Server as Pierre Server
    participant Auth as Auth Manager
    participant DB as Database
    participant Provider as Provider Manager

    Note over Client1, Provider: Tenant Data Isolation Enforcement

    Client1->>Server: MCP Tool Call (Tenant A)
    Note right of Client1: X-API-Key: pk_tenant_a_abc123...

    Server->>Auth: validate_api_key(tenant_a_key)
    Auth->>DB: get_api_key_by_prefix()
    DB-->>Auth: {user_id: user_a, tenant_id: tenant_a}
    Auth-->>Server: Authenticated (tenant_a, user_a)

    Server->>Provider: get_activities(user_a, tenant_a)
    Provider->>DB: get_user_tokens(user_a, tenant_a)
    Note right of Provider: SELECT * FROM user_tokens<br/>WHERE user_id = user_a<br/>AND tenant_id = tenant_a

    DB-->>Provider: Tenant A Tokens Only
    Provider-->>Server: Tenant A Activities
    Server-->>Client1: Filtered Response (Tenant A Data)

    Note over Client1, Provider: Concurrent Request from Different Tenant

    Client2->>Server: MCP Tool Call (Tenant B)
    Note right of Client2: X-API-Key: pk_tenant_b_xyz789...

    Server->>Auth: validate_api_key(tenant_b_key)
    Auth->>DB: get_api_key_by_prefix()
    DB-->>Auth: {user_id: user_b, tenant_id: tenant_b}
    Auth-->>Server: Authenticated (tenant_b, user_b)

    Server->>Provider: get_activities(user_b, tenant_b)
    Provider->>DB: get_user_tokens(user_b, tenant_b)
    Note right of Provider: SELECT * FROM user_tokens<br/>WHERE user_id = user_b<br/>AND tenant_id = tenant_b

    DB-->>Provider: Tenant B Tokens Only
    Provider-->>Server: Tenant B Activities
    Server-->>Client2: Filtered Response (Tenant B Data)

    Note over Client1, Provider: Cross-Tenant Access Attempt (Blocked)

    Client1->>Server: Attempt Access with Wrong Tenant Key
    Server->>Auth: validate_api_key(malformed_key)
    Auth->>DB: get_api_key_by_prefix()
    
    alt Key Not Found
        DB-->>Auth: Key Not Found
        Auth-->>Server: Authentication Failed
        Server-->>Client1: 401 Unauthorized
    else Key from Different Tenant
        DB-->>Auth: {user_id: user_x, tenant_id: tenant_x}
        Note right of Auth: user_x belongs to tenant_x,<br/>not tenant_a
        Auth-->>Server: Authentication Failed
        Server-->>Client1: 403 Forbidden
    end
```

## Database Migration Flow

```mermaid
sequenceDiagram
    participant App as Application
    participant Migration as Migration Manager
    participant DB as Database
    participant Files as Migration Files

    Note over App, Files: Database Schema Migration Process

    App->>Migration: run_migrations()
    Migration->>DB: CREATE TABLE IF NOT EXISTS migrations
    
    Migration->>DB: SELECT * FROM migrations ORDER BY version
    DB-->>Migration: Applied Migrations List
    
    Migration->>Files: scan_migration_directory()
    Files-->>Migration: Available Migration Files
    Note right of Files: 001_initial.sql<br/>002_add_tenants.sql<br/>003_add_rate_limiting.sql

    Migration->>Migration: find_pending_migrations()
    Note right of Migration: Compare applied vs available<br/>to find new migrations

    loop For Each Pending Migration
        Migration->>Files: read_migration_file(version)
        Files-->>Migration: SQL Content
        
        Migration->>DB: BEGIN TRANSACTION
        
        alt Migration Success
            Migration->>DB: EXECUTE migration_sql
            Migration->>DB: INSERT INTO migrations (version, applied_at)
            Migration->>DB: COMMIT TRANSACTION
            Note right of Migration: Migration 002 applied successfully
        else Migration Failure
            Migration->>DB: ROLLBACK TRANSACTION
            Migration-->>App: Migration Failed Error
            Note left of Migration: Stop migration process<br/>on first failure
        end
    end

    Migration-->>App: All Migrations Applied
    
    Note over App, Files: Migration File Structure
    
    Note right of Files: -- 002_add_tenants.sql<br/>-- Description: Add multi-tenant support<br/>-- Up Migration<br/>CREATE TABLE tenants (<br/>  id UUID PRIMARY KEY,<br/>  name VARCHAR NOT NULL,<br/>  created_at TIMESTAMP DEFAULT NOW()<br/>);<br/><br/>-- Add tenant_id to users<br/>ALTER TABLE users ADD COLUMN tenant_id UUID<br/>REFERENCES tenants(id);<br/><br/>-- Down Migration (in separate file)<br/>-- 002_add_tenants_down.sql<br/>ALTER TABLE users DROP COLUMN tenant_id;<br/>DROP TABLE tenants;
```

## WebSocket Connection Management

```mermaid
sequenceDiagram
    participant Client as MCP Client
    participant WS as WebSocket Handler
    participant Auth as Auth Manager
    participant ConnMgr as Connection Manager
    participant HeartBeat as Heartbeat Service

    Note over Client, HeartBeat: WebSocket Lifecycle Management

    Client->>WS: WebSocket Upgrade Request
    Note right of Client: Upgrade: websocket<br/>Connection: Upgrade<br/>X-API-Key: pk_live_abc123...

    WS->>Auth: validate_api_key(api_key)
    Auth-->>WS: User ID & Permissions

    alt Valid Authentication
        WS->>ConnMgr: register_connection(user_id, ws_stream)
        ConnMgr->>ConnMgr: store_connection_metadata()
        
        WS-->>Client: WebSocket Upgrade 101
        WS->>HeartBeat: start_heartbeat(connection_id)
        
        loop Heartbeat Loop
            HeartBeat->>Client: ping frame
            Client-->>HeartBeat: pong frame
            Note right of HeartBeat: Connection alive check<br/>every 30 seconds
        end
        
        loop Message Processing
            Client->>WS: JSON-RPC Message
            WS->>WS: validate_json_rpc()
            
            alt Valid Message
                WS->>WS: route_to_handler()
                WS-->>Client: JSON-RPC Response
            else Invalid Message
                WS-->>Client: JSON-RPC Error Response
            end
        end
        
    else Invalid Authentication
        WS-->>Client: 401 Unauthorized
        WS->>WS: close_connection()
    end

    Note over Client, HeartBeat: Connection Cleanup

    alt Client Disconnect
        Client->>WS: WebSocket Close Frame
        WS->>ConnMgr: unregister_connection(connection_id)
        WS->>HeartBeat: stop_heartbeat(connection_id)
        ConnMgr->>ConnMgr: cleanup_connection_metadata()
        
    else Heartbeat Timeout
        HeartBeat->>HeartBeat: detect_timeout()
        HeartBeat->>ConnMgr: force_disconnect(connection_id)
        ConnMgr->>WS: close_connection()
        WS-->>Client: Connection Closed
        
    else Server Shutdown
        WS->>ConnMgr: get_all_connections()
        ConnMgr-->>WS: Active Connections List
        
        loop For Each Connection
            WS->>Client: Close Frame (1001 Going Away)
            WS->>ConnMgr: unregister_connection()
        end
    end
```

## Notes on Sequence Diagrams

### Key Design Patterns Illustrated

1. **Error Handling**: Each flow shows proper error handling with meaningful error responses and rollback mechanisms.

2. **Authentication**: Consistent authentication patterns across all flows, with proper JWT and API key validation.

3. **Data Isolation**: Multi-tenant architecture ensures complete data separation between tenants.

4. **Rate Limiting**: Token bucket algorithm implementation with proper cache management and header responses.

5. **Connection Management**: WebSocket lifecycle management with heartbeat monitoring and graceful cleanup.

6. **Database Transactions**: Migration flows show proper transaction management with rollback on failure.

### Performance Considerations

- **Caching**: Rate limiting uses Redis for high-performance token bucket operations
- **Connection Pooling**: Database connections are pooled for optimal performance
- **Async Processing**: All I/O operations are asynchronous for maximum concurrency
- **Token Refresh**: OAuth token refresh is handled transparently without user intervention

### Security Features

- **Token Encryption**: All sensitive tokens are encrypted at rest using AES-256-GCM
- **State Validation**: OAuth flows include CSRF protection via state parameters
- **API Key Prefixes**: Partial key exposure for identification without compromising security
- **Tenant Isolation**: Database queries include tenant filtering to prevent cross-tenant data access

These sequence diagrams provide a comprehensive view of how the Pierre MCP Server handles complex multi-protocol interactions while maintaining security, performance, and reliability.