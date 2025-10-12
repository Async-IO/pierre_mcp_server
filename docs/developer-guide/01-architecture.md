# Architecture Overview

## System Architecture

Pierre MCP Server follows a layered, multi-protocol architecture designed for scalability, security, and extensibility.

```mermaid
graph TB
    subgraph "Client Layer"
        Claude[Claude AI]
        WebApp[Web Application]
        A2AClient[A2A Client]
        SDK[Pierre SDK]
    end
    
    subgraph "Protocol Layer"
        MCP[MCP Protocol Handler]
        OAuth2[OAuth 2.0 Authorization Server]
        REST[REST API]
        WS[WebSocket]
        A2A[A2A Protocol]
    end
    
    subgraph "Authentication & Security"
        JWT[JWT Manager]
        Auth[Auth Middleware]
        RateLimit[Rate Limiter]
    end
    
    subgraph "Business Logic Layer"
        TenantMgr[Tenant Manager]
        ProviderFactory[Provider Factory]
        IntelEngine[Intelligence Engine]
        ConfigMgr[Configuration Manager]
    end
    
    subgraph "Provider Layer"
        Strava[Strava Provider]
        Garmin[Garmin Provider]
        Fitbit[Fitbit Provider]
        Universal[Universal Provider]
    end
    
    subgraph "Data Layer"
        DB[(Database)]
        Cache[Cache Layer]
        Encryption[Encryption Service]
    end
    
    Claude --> MCP
    Claude --> OAuth2
    WebApp --> REST
    WebApp --> WS
    A2AClient --> A2A
    SDK --> REST
    
    MCP --> Auth
    OAuth2 --> Auth
    REST --> Auth
    A2A --> Auth
    WS --> Auth
    
    Auth --> JWT
    Auth --> RateLimit
    
    MCP --> TenantMgr
    REST --> TenantMgr
    A2A --> TenantMgr
    
    TenantMgr --> ProviderFactory
    TenantMgr --> IntelEngine
    TenantMgr --> ConfigMgr
    
    ProviderFactory --> Strava
    ProviderFactory --> Garmin
    ProviderFactory --> Fitbit
    ProviderFactory --> Universal
    
    Strava --> DB
    Garmin --> DB
    Fitbit --> DB
    Universal --> DB
    
    IntelEngine --> DB
    ConfigMgr --> DB
    
    DB --> Encryption
    DB --> Cache
```

## Core Design Patterns

### 1. Multi-Tenant Architecture

Every request operates within a tenant context, ensuring complete data isolation:

```rust
pub struct TenantContext {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub role: TenantRole,
    pub rate_limit_multiplier: f32,
    pub features: HashSet<String>,
}
```

### 2. Consolidated Server Architecture

All protocols run on a single port (8081) for simplified deployment. The server uses `warp`'s filter system to compose the routes:

```rust
// src/mcp/multitenant.rs
let routes = auth_route_filter
    .or(oauth_route_filter)
    .or(oauth2_server_routes)
    .or(api_key_route_filter)
    .or(api_key_usage_filter)
    .or(dashboard_route_filter)
    .or(dashboard_detailed_filter)
    .or(a2a_basic_filter)
    .or(a2a_client_filter)
    .or(a2a_monitoring_filter)
    .or(a2a_execution_filter)
    .or(configuration_filter)
    .or(user_configuration_filter)
    .or(specialized_configuration_filter)
    .or(fitness_configuration_filter)
    .or(admin_routes_filter)
    .or(tenant_routes_filter)
    .or(sse_routes)
    .or(mcp_sse_routes)
    .or(mcp_endpoint_routes)
    .or(health_route)
    .with(cors)
    .with(security_headers)
    .recover(handle_rejection);
```

### 3. Routing Design (Warp)

The project uses the `warp` web framework, which employs a filter-based system for routing. Each route is a combination of filters that match on different aspects of the incoming request, such as the path, method, headers, and body.

**Pros:**

*   **Type-safe:** `warp`'s filter system is type-safe, which means that the compiler can catch errors at compile time.
*   **Composable:** Filters can be combined and reused to create complex routing logic.
*   **Flexible:** The filter system is very flexible and can be used to match on almost any aspect of the request.

**Cons:**

*   **Verbose:** The filter chains can be verbose and difficult to read, especially for complex routes.
*   **Imperative Style:** The routing is defined in an imperative style, which can make it difficult to see the overall routing structure at a glance.
*   **Clones and Closures:** The use of closures and `Arc` for sharing state can lead to a lot of `clone()` calls, which can add visual noise to the code.

While `warp` is a powerful and flexible framework, the routing implementation in this project could be simplified by using a more declarative approach or by extracting common logic into middleware.

### 4. OAuth 2.0 Authorization Server

Pierre implements a standards-compliant OAuth 2.0 Authorization Server for MCP client compatibility:

```rust
// src/oauth2/routes.rs
pub fn oauth2_routes(
    db: Arc<Database>,
    auth_manager: &AuthManager,
    http_port: u16,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let registration = register_client(db.clone());
    let authorization = authorize_client(db.clone(), auth_manager, http_port);
    let token = exchange_token(db, auth_manager.clone());

    warp::path("oauth2").and(registration.or(authorization).or(token))
}
```

### 4. Provider Factory Pattern

Providers are created dynamically based on tenant configuration:

```rust
pub struct TenantProviderFactory {
    oauth_client: Arc<TenantOAuthClient>,
}

impl TenantProviderFactory {
    pub async fn create_provider(
        &self,
        provider_type: &str,
        context: &TenantContext,
    ) -> Result<Box<dyn FitnessProvider>>;
}
```

### 3. Protocol Abstraction

Core business logic is protocol-agnostic:

```rust
// Core functionality
pub trait FitnessProvider: Send + Sync {
    async fn get_athlete(&self) -> Result<Athlete>;
    async fn get_activities(&self, params: ActivityParams) -> Result<Vec<Activity>>;
    // ...
}

// Protocol handlers call core functionality
impl McpHandler {
    async fn handle_tool(&self, tool: &str, params: Value) -> Result<Value> {
        let provider = self.get_provider()?;
        match tool {
            "get_athlete" => provider.get_athlete().await,
            // ...
        }
    }
}
```

## Request Flow

### MCP Request Flow

```mermaid
sequenceDiagram
    participant Client as Claude/MCP Client
    participant Server as MCP Server
    participant Auth as Auth Middleware
    participant Tenant as Tenant Manager
    participant Provider as Provider
    participant DB as Database
    
    Client->>Server: MCP Request (with auth token)
    Server->>Auth: Validate Token
    Auth->>DB: Check User/Token
    DB-->>Auth: User Context
    Auth-->>Server: AuthResult
    
    Server->>Tenant: Create Tenant Context
    Tenant->>DB: Load Tenant Config
    DB-->>Tenant: Config Data
    
    Server->>Provider: Execute Tool
    Provider->>DB: Fetch/Store Data
    DB-->>Provider: Data
    Provider-->>Server: Result
    
    Server-->>Client: MCP Response
```

### A2A Request Flow

```mermaid
sequenceDiagram
    participant System as External System
    participant A2A as A2A Handler
    participant Auth as System Auth
    participant Agent as Agent Card
    participant Tool as Tool Engine
    participant DB as Database
    
    System->>A2A: A2A Request (API Key)
    A2A->>Auth: Validate System User
    Auth->>DB: Check API Key
    DB-->>Auth: System Context
    
    A2A->>Agent: Load Agent Card
    Agent->>DB: Fetch Agent Config
    DB-->>Agent: Agent Capabilities
    
    A2A->>Tool: Execute Tool
    Tool->>DB: Process Request
    DB-->>Tool: Result
    
    Tool-->>A2A: Tool Response
    A2A-->>System: A2A Response
```

## Data Flow Architecture

### Write Path

```mermaid
graph LR
    Request[Request] --> Validate[Validation]
    Validate --> Tenant[Tenant Context]
    Tenant --> Encrypt[Encryption]
    Encrypt --> DB[(Database)]
    DB --> Audit[Audit Log]
    Audit --> Response[Response]
```

### Read Path

```mermaid
graph LR
    Request[Request] --> Cache{Cache Hit?}
    Cache -->|Yes| Response[Response]
    Cache -->|No| DB[(Database)]
    DB --> Decrypt[Decryption]
    Decrypt --> Transform[Transform]
    Transform --> Cache2[Update Cache]
    Cache2 --> Response
```

## Security Architecture

### Authentication Layers

1. **Transport Security**: HTTPS/TLS for all communications
2. **Token Authentication**: JWT tokens with expiry
3. **API Key Authentication**: For A2A communication
4. **OAuth2**: For provider authentication

### Authorization Model

```mermaid
graph TB
    User[User Request] --> Token{Valid Token?}
    Token -->|No| Reject[Reject]
    Token -->|Yes| Role{Check Role}
    
    Role --> Admin[Admin]
    Role --> Owner[Tenant Owner]
    Role --> Member[Tenant Member]
    Role --> System[System User]
    
    Admin --> AllAccess[Full Access]
    Owner --> TenantAccess[Tenant Admin Access]
    Member --> UserAccess[User Data Access]
    System --> ApiAccess[API Scope Access]
```

## Scalability Considerations

### Horizontal Scaling

```mermaid
graph TB
    LB[Load Balancer]
    
    subgraph "Server Instances"
        S1[Server 1]
        S2[Server 2]
        S3[Server N]
    end
    
    subgraph "Shared Infrastructure"
        DB[(PostgreSQL)]
        Redis[(Redis Cache)]
    end
    
    LB --> S1
    LB --> S2
    LB --> S3
    
    S1 --> DB
    S1 --> Redis
    S2 --> DB
    S2 --> Redis
    S3 --> DB
    S3 --> Redis
```

### Database Sharding Strategy

- **By Tenant**: Each tenant's data in separate tables/schemas
- **By Time**: Historical data partitioned by date
- **By Provider**: Provider-specific data in dedicated tables

## Performance Optimization

### Caching Strategy

1. **Request Cache**: 15-minute TTL for identical requests
2. **Provider Cache**: OAuth tokens and provider metadata
3. **Computation Cache**: Intelligence engine results
4. **Database Query Cache**: Frequently accessed data

### Async Processing

```rust
// All I/O operations are async
pub async fn handle_request() -> Result<Response> {
    // Concurrent provider calls
    let (athlete, activities, stats) = tokio::join!(
        provider.get_athlete(),
        provider.get_activities(params),
        provider.get_stats()
    );
    
    // Process results
    Ok(build_response(athlete?, activities?, stats?))
}
```

## Monitoring & Observability

### Metrics Collection

- Request latency
- Error rates
- Provider API usage
- Database query performance
- Cache hit rates
- Token validation time

### Health Checks

```rust
pub struct HealthStatus {
    pub status: HealthState,
    pub database: ComponentHealth,
    pub cache: ComponentHealth,
    pub providers: HashMap<String, ComponentHealth>,
    pub uptime_seconds: u64,
}
```

## Error Handling Strategy

### Error Categories

1. **Client Errors (4xx)**: Invalid requests, authentication failures
2. **Server Errors (5xx)**: Internal errors, provider failures
3. **Provider Errors**: Rate limits, API failures
4. **Database Errors**: Connection issues, query failures

### Error Propagation

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Authentication failed: {0}")]
    Auth(#[from] AuthError),
    
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),
    
    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),
    
    #[error("Rate limit exceeded")]
    RateLimit,
}
```

## Development Principles

### SOLID Principles

- **Single Responsibility**: Each module has one clear purpose
- **Open/Closed**: Extensible through traits, not modification
- **Liskov Substitution**: All providers implement common interface
- **Interface Segregation**: Minimal, focused interfaces
- **Dependency Inversion**: Depend on abstractions, not concretions

### Code Organization

```
src/
├── protocols/         # Protocol handlers (MCP, A2A, REST)
├── mcp/              # MCP protocol implementation
├── a2a/              # A2A protocol implementation
├── providers/        # External fitness integrations
├── database_plugins/ # Database backends (SQLite/PostgreSQL)
├── intelligence/     # Analytics and recommendations
├── tenant/           # Multi-tenant management
├── security/         # Security components
├── oauth/            # OAuth management
├── config/           # Configuration management
├── crypto/           # Cryptographic utilities
└── utils/            # Shared utilities
```

### Testing Strategy

- **Unit Tests**: Pure business logic
- **Integration Tests**: Database and provider interactions
- **E2E Tests**: Full request/response cycles
- **Performance Tests**: Load and stress testing