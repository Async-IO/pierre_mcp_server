# Pierre Fitness API - Technical Architecture Documentation

## Table of Contents
1. [System Overview](#system-overview)
2. [Architecture Components](#architecture-components)
3. [Service Architecture](#service-architecture)
4. [Authentication & Security](#authentication--security)
5. [API Key Management System](#api-key-management-system)
6. [Database Schema](#database-schema)
7. [Integration Points](#integration-points)
8. [Deployment Architecture](#deployment-architecture)
9. [What's Next](#whats-next)

## System Overview

Pierre Fitness API is a multi-tenant fitness data platform that provides secure access to fitness data from multiple providers (Strava, Fitbit) through the Model Context Protocol (MCP), A2A (Agent-to-Agent) Protocol, and traditional REST APIs. The system is designed for B2B use cases, allowing AI assistants, autonomous agents, and applications to access fitness data with proper authentication and rate limiting.

### Core Services

1. **MCP Protocol Handler (Port 8080)**: Handles Model Context Protocol connections for AI assistants
2. **A2A & HTTP API Server (Port 8081)**: Provides REST endpoints for authentication, API key management, dashboard access, and A2A protocol
3. **Admin Service (Port 8082)**: Manages API key approval workflows and email notifications
4. **Frontend (Port 5173)**: React-based dashboard for user management and analytics
5. **A2A Protocol Handler**: Embedded in HTTP API server, handles agent-to-agent communication

## Architecture Components

### Technology Stack

- **Backend**: Rust with Tokio async runtime
- **Web Framework**: Warp for HTTP endpoints
- **Database**: SQLite with SQLx for async operations
- **Authentication**: JWT tokens and API keys with SHA-256 hashing
- **Frontend**: React with TypeScript
- **Email Service**: SMTP integration for notifications
- **Encryption**: AES-256-GCM for sensitive data at rest

### Key Libraries

```toml
# Core dependencies
tokio = { version = "1", features = ["full"] }
warp = "0.3"
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio-native-tls"] }
serde = { version = "1.0", features = ["derive"] }
jsonwebtoken = "9"
bcrypt = "0.15"
sha2 = "0.10"
aes-gcm = "0.10"
```

## Service Architecture

### 1. MCP Server (Multi-tenant)

The MCP server implements the Model Context Protocol for AI assistant integration:

```rust
pub struct MultiTenantMcpServer {
    database: Arc<Database>,
    auth_manager: Arc<AuthManager>,
    auth_middleware: Arc<McpAuthMiddleware>,
    websocket_manager: Arc<WebSocketManager>,
    user_providers: Arc<RwLock<HashMap<String, HashMap<String, Box<dyn FitnessProvider>>>>>,
}
```

**Key Features:**
- User-scoped provider instances
- JWT and API key authentication
- Rate limiting per API key tier
- WebSocket support for real-time updates
- Tool-based interface for fitness data access

**Available Tools:**
- `get_activities`: Retrieve user activities
- `get_athlete`: Get athlete profile information
- `get_stats`: Fetch fitness statistics
- `get_activity_intelligence`: AI-enhanced activity analysis
- `connect_strava/connect_fitbit`: OAuth integration
- `set_goal/track_progress`: Goal management
- `analyze_training_load`: Training analysis
- `calculate_fitness_score`: Fitness scoring

### 2. HTTP API Server

REST API endpoints for web clients and management:

```rust
// Authentication
POST   /auth/register
POST   /auth/login

// OAuth
GET    /oauth/auth/{provider}/{user_id}
GET    /oauth/callback/{provider}

// API Keys
POST   /api/keys                 // Create API key
POST   /api/keys/trial          // Create trial key
GET    /api/keys                // List user's keys
DELETE /api/keys/{id}           // Deactivate key
GET    /api/keys/{id}/usage     // Get usage stats

// Dashboard
GET    /dashboard/overview      // Dashboard data
GET    /dashboard/analytics     // Usage analytics
GET    /dashboard/rate-limits   // Rate limit status

// Health
GET    /health                  // Comprehensive health check
GET    /ready                   // Readiness probe
GET    /live                    // Liveness probe
```

### 3. Admin Service

Separate service for administrative functions:

- API key request management
- Approval/denial workflow
- Email notifications
- Auto-approval configuration
- Audit logging

## Authentication & Security

### Multi-Layer Authentication

1. **JWT Token Authentication**
   - Used for web dashboard and user sessions
   - 24-hour expiration (configurable)
   - Contains user ID, email, and available providers
   - Enhanced error handling with detailed expiration info

2. **API Key Authentication**
   - Production keys: `pk_live_` prefix
   - Trial keys: `pk_trial_` prefix
   - SHA-256 hashed storage
   - Rate limiting based on tier

### Security Features

```rust
// JWT Token Structure
pub struct Claims {
    pub sub: String,              // User ID
    pub email: String,
    pub iat: i64,                // Issued at
    pub exp: i64,                // Expiration
    pub providers: Vec<String>,   // Available fitness providers
}

// API Key Validation
pub enum JwtValidationError {
    TokenExpired { expired_at: DateTime<Utc>, current_time: DateTime<Utc> },
    TokenInvalid { reason: String },
    TokenMalformed { details: String },
}
```

### CORS Configuration

```rust
let cors = warp::cors()
    .allow_any_origin()
    .allow_headers(vec![
        "content-type", 
        "authorization", 
        "x-requested-with",
        "accept",
        "origin",
        "access-control-request-method",
        "access-control-request-headers"
    ])
    .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"]);
```

## API Key Management System

### Tier System

```rust
pub enum ApiKeyTier {
    Trial,        // 1,000 requests/month, 14-day expiration
    Starter,      // 10,000 requests/month
    Professional, // 100,000 requests/month
    Enterprise,   // Unlimited
}
```

### Key Features

1. **Trial Keys**
   - Automatic 14-day expiration
   - One trial key per user limit
   - 1,000 request monthly limit
   - `pk_trial_` prefix for identification

2. **Rate Limiting**
   - Monthly request limits per tier
   - Real-time usage tracking
   - Automatic reset on month boundary
   - Enterprise tier unlimited

3. **Usage Tracking**
   ```rust
   pub struct ApiKeyUsage {
       pub api_key_id: String,
       pub timestamp: DateTime<Utc>,
       pub tool_name: String,
       pub response_time_ms: Option<u32>,
       pub status_code: u16,
       pub error_message: Option<String>,
   }
   ```

4. **Automatic Cleanup**
   - Hourly background task
   - Deactivates expired trial keys
   - Maintains audit trail

## A2A (Agent-to-Agent) Protocol

### Overview

The A2A Protocol enables agent-to-agent communication and collaboration, allowing other AI systems to discover, authenticate with, and utilize Pierre's fitness intelligence capabilities through a standardized JSON-RPC 2.0 interface.

### Protocol Implementation

Pierre's A2A implementation provides:

- **JSON-RPC 2.0 Communication**: Standard protocol for agent-to-agent messaging
- **Agent Card Discovery**: Automated capability discovery via `/.well-known/agent.json`
- **Client Registration**: Secure registration and management of AI agents
- **Session Management**: Persistent sessions with configurable expiration
- **Task Management**: Asynchronous task execution with persistent storage
- **Universal Tool Access**: Same tools available through both MCP and A2A protocols

### Agent Card & Discovery

Pierre exposes its capabilities through an Agent Card located at:

```
GET http://localhost:8081/a2a/agent-card
```

The Agent Card includes:
- Agent identity and version information
- Available tool schemas and parameters
- Authentication requirements
- Rate limits and usage tiers
- Supported protocol capabilities

Example Agent Card structure:
```json
{
  "agent": {
    "name": "Pierre Fitness Intelligence Agent",
    "version": "1.0.0",
    "description": "AI-powered fitness data analysis platform"
  },
  "authentication": {
    "type": "api_key",
    "description": "API key authentication required"
  },
  "tools": [
    {
      "name": "get_activities",
      "description": "Retrieve user fitness activities",
      "parameters": { ... }
    },
    {
      "name": "analyze_activity", 
      "description": "AI analysis of fitness activity",
      "parameters": { ... }
    }
  ]
}
```

### Authentication & Authorization

A2A clients must register before accessing the protocol:

1. **Client Registration**: Clients register with name, description, and capabilities
2. **Credential Issuance**: System generates client_id, client_secret, and API key
3. **API Key Authentication**: Clients use API key for subsequent requests
4. **Session Creation**: Authenticated clients can create persistent sessions

Registration flow:
```bash
POST /a2a/clients
{
  "name": "FitnessAnalyzer",
  "description": "AI agent for fitness analytics",
  "capabilities": ["fitness-data-analysis", "goal-management"]
}
```

### Available Methods

Core A2A Protocol methods:

| Method | Description |
|--------|-------------|
| `a2a/initialize` | Initialize A2A connection and get capabilities |
| `a2a/tools/list` | List all available tools |
| `a2a/tools/call` | Execute a specific tool |
| `a2a/tasks/create` | Create an asynchronous task |
| `a2a/tasks/get` | Get task status and results |
| `a2a/tasks/list` | List tasks for a client |
| `a2a/message/send` | Send a message to Pierre |
| `a2a/message/stream` | Stream messages (future feature) |

### Message Types & Formats

A2A uses standard JSON-RPC 2.0 message formats:

**Request Format:**
```json
{
  "jsonrpc": "2.0",
  "method": "a2a/tools/call",
  "params": {
    "tool_name": "get_activities",
    "parameters": {
      "provider": "strava",
      "limit": 10
    }
  },
  "id": 1
}
```

**Response Format:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "activities": [...],
    "total_count": 10
  },
  "id": 1
}
```

### Task Management

A2A supports asynchronous task execution for long-running operations:

1. **Task Creation**: Client creates a task with parameters
2. **Task Execution**: Server executes task asynchronously
3. **Status Polling**: Client polls for task completion
4. **Result Retrieval**: Client retrieves task results

Task lifecycle:
- `pending` → `running` → `completed`/`failed`/`cancelled`

### Database Schema Integration

A2A protocol data is stored in dedicated tables:

- **a2a_clients**: Registered A2A clients and their capabilities
- **a2a_sessions**: Active sessions with expiration tracking
- **a2a_tasks**: Asynchronous tasks with status and results

These tables integrate with the existing user and API key system for authentication.

### Integration with Universal Tool Layer

A2A requests are processed through the Universal Tool Layer, which provides:

- **Protocol Abstraction**: Same tools work with MCP and A2A
- **Request Translation**: A2A requests converted to universal format
- **Response Formatting**: Universal responses formatted for A2A protocol
- **Error Handling**: Consistent error responses across protocols

This architecture ensures feature parity between MCP and A2A while maintaining protocol-specific optimizations.

## Database Schema

### Core Tables

```sql
-- Users table
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    display_name TEXT,
    is_active BOOLEAN DEFAULT true,
    created_at TEXT NOT NULL,
    last_active_at TEXT
);

-- API Keys
CREATE TABLE api_keys (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    key_prefix TEXT NOT NULL,
    key_hash TEXT NOT NULL,
    description TEXT,
    tier TEXT NOT NULL,
    rate_limit_requests INTEGER NOT NULL,
    rate_limit_window INTEGER NOT NULL,
    is_active BOOLEAN DEFAULT true,
    last_used_at TEXT,
    expires_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

-- OAuth Tokens (Encrypted)
CREATE TABLE oauth_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    encrypted_access_token TEXT NOT NULL,
    encrypted_refresh_token TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    scope TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(user_id, provider)
);

-- API Key Usage
CREATE TABLE api_key_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    api_key_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    response_time_ms INTEGER,
    status_code INTEGER NOT NULL,
    error_message TEXT,
    request_size_bytes INTEGER,
    response_size_bytes INTEGER,
    ip_address TEXT,
    user_agent TEXT,
    FOREIGN KEY (api_key_id) REFERENCES api_keys(id)
);

-- A2A Clients table - registered A2A agents
CREATE TABLE a2a_clients (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    api_key_id TEXT NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    public_key TEXT,
    capabilities TEXT NOT NULL DEFAULT '[]',
    redirect_uris TEXT NOT NULL DEFAULT '[]',
    agent_version TEXT,
    contact_email TEXT,
    documentation_url TEXT,
    is_verified BOOLEAN NOT NULL DEFAULT false,
    verification_token TEXT,
    verified_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(name)
);

-- A2A Sessions table - persistent agent sessions
CREATE TABLE a2a_sessions (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    client_id TEXT NOT NULL REFERENCES a2a_clients(id) ON DELETE CASCADE,
    user_id TEXT REFERENCES users(id) ON DELETE CASCADE,
    granted_scopes TEXT NOT NULL DEFAULT '[]',
    session_token TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_activity TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    requests_count INTEGER NOT NULL DEFAULT 0
);

-- A2A Tasks table - asynchronous task management
CREATE TABLE a2a_tasks (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    client_id TEXT NOT NULL REFERENCES a2a_clients(id) ON DELETE CASCADE,
    session_id TEXT REFERENCES a2a_sessions(id) ON DELETE SET NULL,
    task_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    parameters TEXT,
    result TEXT,
    error_message TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    started_at TIMESTAMP,
    completed_at TIMESTAMP
);

-- Indexes for A2A tables
CREATE INDEX idx_a2a_clients_api_key ON a2a_clients(api_key_id);
CREATE INDEX idx_a2a_sessions_client ON a2a_sessions(client_id);
CREATE INDEX idx_a2a_sessions_token ON a2a_sessions(session_token);
CREATE INDEX idx_a2a_sessions_expires ON a2a_sessions(expires_at);
CREATE INDEX idx_a2a_tasks_client ON a2a_tasks(client_id);
CREATE INDEX idx_a2a_tasks_status ON a2a_tasks(status);
```

### Encryption

All sensitive data (OAuth tokens) are encrypted using AES-256-GCM:

```rust
pub struct EncryptedData {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
}
```

## Integration Points

### 1. Fitness Provider Integration

```rust
#[async_trait]
pub trait FitnessProvider: Send + Sync {
    async fn authenticate(&mut self, auth_data: AuthData) -> Result<()>;
    async fn get_activities(&self, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<Activity>>;
    async fn get_athlete(&self) -> Result<Athlete>;
    async fn get_stats(&self) -> Result<Stats>;
}
```

### 2. OAuth2 Flow

```mermaid
sequenceDiagram
    participant User
    participant Frontend
    participant API
    participant Provider
    
    User->>Frontend: Click "Connect Strava"
    Frontend->>API: GET /oauth/auth/strava/{user_id}
    API->>Frontend: Return authorization URL
    Frontend->>Provider: Redirect to OAuth
    Provider->>User: Show consent screen
    User->>Provider: Approve
    Provider->>API: Redirect with code
    API->>Provider: Exchange code for tokens
    API->>API: Encrypt and store tokens
    API->>Frontend: Success response
```

### 3. WebSocket Integration

Real-time updates for dashboard:
- API key usage notifications
- Rate limit warnings
- Connection status updates

## Deployment Architecture

### Docker Configuration

```yaml
version: '3.8'
services:
  mcp-server:
    build: .
    ports:
      - "8080:8080"  # MCP protocol
      - "8081:8081"  # HTTP API
    environment:
      - DATABASE_URL=sqlite:./data/users.db
      - RUST_LOG=info
    volumes:
      - ./data:/app/data

  admin-service:
    build: ./admin
    ports:
      - "8082:8082"
    environment:
      - DATABASE_URL=sqlite:./data/users.db
      - SMTP_HOST=${SMTP_HOST}
      - SMTP_PORT=${SMTP_PORT}
```

### Security Considerations

1. **Network Security**
   - All services bind to localhost by default
   - TLS termination at reverse proxy
   - Rate limiting at application level

2. **Data Security**
   - Encryption keys stored separately
   - JWT secrets rotated regularly
   - API keys hashed with SHA-256
   - OAuth tokens encrypted at rest

3. **Access Control**
   - User-scoped data access
   - API key tied to user account
   - Provider access validated per request

## What's Next

### 1. Frontend Enhancements (High Priority)
- **API Key Request Status Tracking**
  - Show pending/approved/denied status
  - Display approval timeline
  - Email notification preferences
  
- **Enhanced Dashboard Analytics**
  - Real-time usage graphs
  - Tool-specific usage breakdown
  - Cost projection based on usage

### 2. Advanced Features (Medium Priority)

- **Webhook Support**
  - Activity upload notifications
  - Goal achievement alerts
  - Rate limit warnings

- **Data Export API**
  - Bulk activity export
  - Custom date ranges
  - Multiple format support (JSON, CSV, GPX)

- **Enhanced Intelligence Features**
  - ML-based performance predictions
  - Personalized training recommendations
  - Injury risk assessment

### 3. Enterprise Features (Future)

- **Team Management**
  - Multi-user organizations
  - Role-based access control
  - Centralized billing

- **Advanced Analytics**
  - Custom metrics definition
  - A/B testing for training plans
  - Population-level insights

- **SLA Management**
  - Uptime guarantees
  - Custom rate limits
  - Priority support queue

### 4. Technical Improvements

- **Performance Optimization**
  - Redis caching layer
  - Database query optimization
  - Connection pooling enhancements

- **Monitoring & Observability**
  - OpenTelemetry integration
  - Distributed tracing
  - Custom metrics dashboards

- **API Gateway**
  - Kong or similar for advanced routing
  - GraphQL endpoint
  - API versioning strategy

### 5. Compliance & Certification

- **Security Certifications**
  - SOC 2 Type II
  - ISO 27001
  - HIPAA compliance (for health data)

- **Data Privacy**
  - GDPR compliance tools
  - Data retention policies
  - User data export/deletion

The platform is now production-ready with core B2B features implemented. The next phase focuses on enhancing the user experience, adding enterprise features, and preparing for scale.