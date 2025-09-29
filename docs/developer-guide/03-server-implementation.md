# Server Implementation

## Overview

The Pierre MCP Server implementation centers around the `MultiTenantMcpServer` struct which orchestrates all server components and manages the server lifecycle.

## Main Binary Entry Point

### `src/bin/pierre-mcp-server.rs`

The main binary handles server initialization and startup:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();
    
    // Load configuration
    let config = ServerConfig::from_env()?;
    
    // Initialize logging
    logging::init_from_env()?;
    
    // Load or generate encryption key
    let encryption_key = load_or_generate_key(&config.database.encryption_key_path)?;
    
    // Load or generate JWT secret
    let jwt_secret = load_or_generate_jwt_secret(&config.auth.jwt_secret_path)?;
    
    // Initialize database
    let database = Database::new(
        &config.database.url.to_connection_string(),
        encryption_key.to_vec(),
    ).await?;
    
    // Initialize authentication manager
    let auth_manager = AuthManager::new(
        jwt_secret.to_vec(), 
        config.auth.jwt_expiry_hours as i64
    );
    
    // Create and run server with ServerResources dependency injection
    let server = MultiTenantMcpServer::new(
        database,
        auth_manager,
        &jwt_secret_string,
        Arc::new(config.clone()),
    );
    server.run(config.mcp_port).await?;
    
    Ok(())
}
```

## Server Architecture

### `src/mcp/multitenant.rs`

The server uses **dependency injection** through a centralized `ServerResources` struct:

```rust
pub struct MultiTenantMcpServer {
    resources: Arc<ServerResources>,
}

pub struct ServerResources {
    pub database: Arc<Database>,
    pub auth_manager: Arc<AuthManager>,
    pub auth_middleware: Arc<McpAuthMiddleware>,
    pub websocket_manager: Arc<WebSocketManager>,
    pub tenant_oauth_client: Arc<TenantOAuthClient>,
    pub tenant_provider_factory: Arc<TenantProviderFactory>,
    pub admin_jwt_secret: Arc<str>,
    pub config: Arc<ServerConfig>,
    pub activity_intelligence: Arc<ActivityIntelligence>,
    pub oauth_manager: Arc<RwLock<OAuthManager>>,
    pub a2a_client_manager: Arc<A2AClientManager>,
    pub a2a_system_user_service: Arc<A2ASystemUserService>,
}
```

### Architectural Benefits

- **Single resource creation**: All expensive resources created once at startup
- **Shared ownership**: Components share resources via Arc references  
- **No cloning**: Resources shared by reference, not cloned per component
- **Testability**: Easy to inject test doubles via ServerResources
- **Performance**: Eliminates resource creation anti-patterns

## Server Initialization Flow

```mermaid
sequenceDiagram
    participant Main
    participant Config
    participant Keys
    participant DB
    participant Auth
    participant Server
    
    Main->>Config: Load ServerConfig from environment
    Config-->>Main: Configuration
    
    Main->>Keys: Load/Generate encryption key
    Keys-->>Main: 32-byte key
    
    Main->>Keys: Load/Generate JWT secret
    Keys-->>Main: 64-byte secret
    
    Main->>DB: Initialize Database
    DB->>DB: Run migrations
    DB-->>Main: Database instance
    
    Main->>Auth: Create AuthManager
    Auth-->>Main: AuthManager
    
    Main->>Server: Create MultiTenantMcpServer
    Server->>Server: Initialize components
    Server-->>Main: Server instance
    
    Main->>Server: Run server
    Server->>Server: Start listeners
```

## Component Initialization

### Database Setup

```rust
impl Database {
    pub async fn new(url: &str, encryption_key: Vec<u8>) -> Result<Self> {
        // Parse database URL
        let db_type = DatabaseType::from_url(url)?;
        
        // Create appropriate backend
        let provider: Box<dyn DatabaseProvider> = match db_type {
            DatabaseType::Sqlite => {
                Box::new(SqliteProvider::new(url, encryption_key).await?)
            },
            DatabaseType::Postgres => {
                Box::new(PostgresProvider::new(url, encryption_key).await?)
            },
        };
        
        // Run migrations
        provider.run_migrations().await?;
        
        Ok(Self { provider })
    }
}
```

### Authentication Setup

```rust
impl AuthManager {
    pub fn new(secret: Vec<u8>, expiry_hours: i64) -> Self {
        Self {
            secret,
            expiry_hours,
            algorithm: Algorithm::HS256,
        }
    }
    
    pub fn generate_token(&self, user_id: &Uuid) -> Result<String> {
        let claims = Claims {
            sub: user_id.to_string(),
            exp: (Utc::now() + Duration::hours(self.expiry_hours)).timestamp(),
            iat: Utc::now().timestamp(),
        };
        
        encode(&self.secret, &claims, &self.algorithm)
    }
}
```

## Server Runtime

### Running the Server

```rust
impl MultiTenantMcpServer {
    pub async fn run(self, mcp_port: u16) -> Result<()> {
        let http_port = mcp_port + 1;
        
        // Start HTTP server in background
        let http_server = self.clone();
        tokio::spawn(async move {
            http_server.run_http_server(http_port).await
        });
        
        // Start WebSocket server in background
        let ws_server = self.clone();
        tokio::spawn(async move {
            ws_server.run_websocket_server(http_port).await
        });
        
        // Run MCP server on main thread
        self.run_mcp_server(mcp_port).await
    }
}
```

### HTTP Server Implementation

```rust
// src/mcp/multitenant.rs
    pub async fn run_http_server_with_resources(
        port: u16,
        resources: Arc<ServerResources>,
    ) -> Result<()> {
        use warp::Filter;

        info!("HTTP authentication server starting on port {}", port);

        // Initialize security configuration
        let security_config = Self::setup_security_config(&resources.config);

        // Initialize all route handlers using shared resources
        let (
            auth_routes,
            oauth_routes,
            api_key_routes,
            dashboard_routes,
            a2a_routes,
            configuration_routes,
            fitness_configuration_routes,
        ) = HttpSetup::setup_route_handlers_with_resources(&resources);

        // Use JWT secret from resources
        let jwt_secret_str = resources.admin_jwt_secret.as_ref();
        info!("Using admin JWT secret from server startup");

        // Setup admin routes - API requires owned values
        let admin_context = crate::admin_routes::AdminApiContext::new(
            resources.database.clone(),
            jwt_secret_str,
            resources.auth_manager.clone(),
        );
        let admin_routes_filter = crate::admin_routes::admin_routes_with_rejection(admin_context);

        // Setup tenant management routes - API requires owned values
        let tenant_routes_filter = Self::create_tenant_routes_filter(
            resources.database.clone(),
            resources.auth_manager.clone(),
        );

        // Configure CORS
        let cors = HttpSetup::setup_cors();

        // Create all route groups using helper functions
        let auth_route_filter = Self::create_auth_routes(auth_routes);
        let oauth_route_filter = Self::create_oauth_routes(&oauth_routes, &resources);

        // Create OAuth 2.0 server routes for MCP client compatibility
        let oauth2_server_routes =
            oauth2_routes(resources.database.clone(), &resources.auth_manager, port);
        let api_key_route_filter = Self::create_api_key_routes(&api_key_routes);
        let api_key_usage_filter = Self::create_api_key_usage_route(api_key_routes.clone()); // Safe: Arc clone for HTTP route sharing
        let dashboard_route_filter = Self::create_dashboard_routes(&dashboard_routes);
        let dashboard_detailed_filter = Self::create_dashboard_detailed_routes(&dashboard_routes);

        // Create A2A routes
        let a2a_basic_filter = Self::create_a2a_basic_routes(&a2a_routes);
        let a2a_client_filter = Self::create_a2a_client_routes(&a2a_routes);
        let a2a_monitoring_filter = Self::create_a2a_monitoring_routes(&a2a_routes);
        let a2a_execution_filter = Self::create_a2a_execution_routes(&a2a_routes);

        // Create configuration routes
        let configuration_filter = Self::create_configuration_routes(&configuration_routes);
        let user_configuration_filter =
            Self::create_user_configuration_routes(&configuration_routes);
        let specialized_configuration_filter =
            Self::create_specialized_configuration_routes(&configuration_routes);

        // Create fitness configuration routes
        let fitness_configuration_filter =
            Self::create_fitness_configuration_routes(&fitness_configuration_routes);

        // Security headers middleware
        let security_headers = Self::create_security_headers_filter(&security_config);

        // Health check route
        let health_route = Self::create_health_route();

        // SSE notification routes
        let sse_routes = crate::notifications::sse::sse_routes(resources.sse_manager.clone()); // Safe: Arc clone for HTTP routes

        // MCP SSE endpoint for MCP protocol transport
        let mcp_sse_routes = Self::create_mcp_sse_routes(&resources);
        let mcp_endpoint_routes = Self::create_mcp_endpoint_routes(&resources);

        // Create websocket route (method needs to exist)
        // let websocket_route = Self::create_websocket_route(resources.websocket_manager.clone());

        // Combine all routes
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

        // Start the server
        info!("HTTP server listening on http://127.0.0.1:{}", port);
        Box::pin(warp::serve(routes).run(([127, 0, 0, 1], port))).await;

        Ok(())
    }
```

### MCP Server Implementation

```rust
async fn run_mcp_server(&self, port: u16) -> Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", port)).await?;
    info!("MCP server listening on port {}", port);
    
    loop {
        let (stream, addr) = listener.accept().await?;
        let server = self.clone();
        
        tokio::spawn(async move {
            if let Err(e) = server.handle_mcp_connection(stream, addr).await {
                error!("MCP connection error: {}", e);
            }
        });
    }
}

async fn handle_mcp_connection(&self, stream: TcpStream, addr: SocketAddr) -> Result<()> {
    let (reader, writer) = stream.into_split();
    let reader = BufReader::new(reader);
    let writer = Arc::new(Mutex::new(writer));
    
    let mut lines = reader.lines();
    while let Some(line) = lines.next_line().await? {
        let request: JsonRpcRequest = serde_json::from_str(&line)?;
        let response = self.handle_mcp_request(request).await;
        
        let response_json = serde_json::to_string(&response)?;
        let mut writer = writer.lock().await;
        writer.write_all(response_json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }
    
    Ok(())
}
```

## Request Handling

### MCP Request Processing

```rust
async fn handle_mcp_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
    match request.method.as_str() {
        "initialize" => self.handle_initialize(request.params).await,
        "tools/list" => self.handle_tools_list().await,
        "tools/call" => self.handle_tool_call(request.params).await,
        _ => JsonRpcResponse::error(
            request.id,
            ERROR_METHOD_NOT_FOUND,
            "Method not found"
        ),
    }
}
```

### Authentication Flow

```rust
async fn authenticate_request(&self, headers: &HeaderMap) -> Result<AuthResult> {
    // Check for Bearer token
    if let Some(auth_header) = headers.get("Authorization") {
        let token = extract_bearer_token(auth_header)?;
        let claims = self.auth_manager.verify_token(&token)?;
        
        // Load user context
        let user = self.database.get_user(&claims.user_id).await?;
        let tenant_context = self.load_tenant_context(&user).await?;
        
        return Ok(AuthResult::Authenticated {
            user_id: user.id,
            tenant_context,
        });
    }
    
    // Check for API key
    if let Some(api_key) = headers.get("X-API-Key") {
        let system_user = self.database.get_system_user_by_api_key(api_key).await?;
        
        return Ok(AuthResult::SystemUser {
            system_id: system_user.id,
            capabilities: system_user.capabilities,
        });
    }
    
    Err(AuthError::Unauthorized)
}
```

## Connection Management

### WebSocket Connections

```rust
pub struct WebSocketManager {
    connections: Arc<RwLock<HashMap<Uuid, WebSocketConnection>>>,
    database: Arc<Database>,
}

impl WebSocketManager {
    pub async fn handle_connection(&self, ws: WebSocket, user_id: Uuid) {
        let (tx, rx) = ws.split();
        
        let connection = WebSocketConnection {
            user_id,
            tx: Arc::new(Mutex::new(tx)),
            last_ping: Instant::now(),
        };
        
        self.connections.write().await.insert(user_id, connection);
        
        // Handle incoming messages
        self.handle_messages(rx, user_id).await;
        
        // Clean up on disconnect
        self.connections.write().await.remove(&user_id);
    }
}
```

### Connection Pooling

```rust
pub struct ConnectionPool {
    connections: Vec<Connection>,
    available: Arc<Semaphore>,
    config: PoolConfig,
}

impl ConnectionPool {
    pub async fn acquire(&self) -> PooledConnection {
        let permit = self.available.acquire().await.unwrap();
        let conn = self.connections.pop().unwrap();
        
        PooledConnection {
            connection: conn,
            pool: self.clone(),
            _permit: permit,
        }
    }
}
```

## Error Handling

### Global Error Handler

```rust
#[derive(Debug)]
pub struct ServerError {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<Value>,
}

impl From<anyhow::Error> for ServerError {
    fn from(err: anyhow::Error) -> Self {
        // Map errors to appropriate codes
        if err.is::<AuthError>() {
            ServerError {
                code: ErrorCode::Unauthorized,
                message: "Authentication failed".to_string(),
                details: None,
            }
        } else if err.is::<ValidationError>() {
            ServerError {
                code: ErrorCode::InvalidRequest,
                message: err.to_string(),
                details: None,
            }
        } else {
            ServerError {
                code: ErrorCode::InternalError,
                message: "Internal server error".to_string(),
                details: None,
            }
        }
    }
}
```

## Graceful Shutdown

```rust
impl MultiTenantMcpServer {
    pub async fn shutdown(&self) -> Result<()> {
        info!("Starting graceful shutdown");
        
        // Stop accepting new connections
        self.shutdown_signal.send(()).ok();
        
        // Wait for ongoing requests to complete
        tokio::time::timeout(
            Duration::from_secs(30),
            self.wait_for_requests()
        ).await?;
        
        // Close provider connections
        self.tenant_provider_factory.shutdown().await?;
        
        // Close database connections
        self.database.close().await?;
        
        // Close WebSocket connections
        self.websocket_manager.close_all().await?;
        
        info!("Shutdown complete");
        Ok(())
    }
}
```

## Health Monitoring

### Health Check Endpoint

```rust
pub async fn health_check(State(state): State<AppState>) -> Json<HealthStatus> {
    let mut status = HealthStatus {
        status: HealthState::Healthy,
        timestamp: Utc::now(),
        components: HashMap::new(),
    };
    
    // Check database
    match state.database.ping().await {
        Ok(_) => {
            status.components.insert("database", ComponentHealth::Healthy);
        }
        Err(e) => {
            status.status = HealthState::Degraded;
            status.components.insert("database", ComponentHealth::Unhealthy(e.to_string()));
        }
    }
    
    // Check providers
    for provider in &["strava", "fitbit"] {
        if state.provider_factory.is_available(provider).await {
            status.components.insert(provider, ComponentHealth::Healthy);
        } else {
            status.components.insert(provider, ComponentHealth::Degraded);
        }
    }
    
    Json(status)
}
```

## Performance Optimization

### Request Batching

```rust
pub struct RequestBatcher {
    buffer: Arc<Mutex<Vec<Request>>>,
    interval: Duration,
}

impl RequestBatcher {
    pub async fn add(&self, request: Request) -> BatchedResponse {
        let mut buffer = self.buffer.lock().await;
        buffer.push(request);
        
        if buffer.len() >= self.batch_size {
            self.flush().await
        } else {
            self.wait_for_batch().await
        }
    }
    
    async fn flush(&self) -> Vec<Response> {
        let mut buffer = self.buffer.lock().await;
        let requests = std::mem::take(&mut *buffer);
        
        // Process batch
        self.process_batch(requests).await
    }
}
```

### Response Caching

```rust
pub struct ResponseCache {
    cache: Arc<RwLock<HashMap<String, CachedResponse>>>,
    ttl: Duration,
}

impl ResponseCache {
    pub async fn get_or_compute<F, Fut>(&self, key: &str, compute: F) -> Result<Response>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Response>>,
    {
        // Check cache
        if let Some(cached) = self.get(key).await {
            if cached.expires_at > Instant::now() {
                return Ok(cached.response);
            }
        }
        
        // Compute and cache
        let response = compute().await?;
        self.set(key, response.clone()).await;
        
        Ok(response)
    }
}
```

## Monitoring & Metrics

### Request Metrics

```rust
pub struct RequestMetrics {
    request_count: AtomicU64,
    error_count: AtomicU64,
    latency_histogram: Histogram,
}

impl RequestMetrics {
    pub fn record_request(&self, duration: Duration, status: StatusCode) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
        
        if !status.is_success() {
            self.error_count.fetch_add(1, Ordering::Relaxed);
        }
        
        self.latency_histogram.record(duration.as_millis() as f64);
    }
}
```

## Configuration

### Server Configuration

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    // Server settings
    pub mcp_port: u16,
    pub http_port: u16,
    pub max_connections: usize,
    pub request_timeout: Duration,
    
    // Database configuration
    pub database: DatabaseConfig,
    
    // Authentication settings
    pub auth: AuthConfig,
    
    // Provider settings
    pub providers: HashMap<String, ProviderConfig>,
    
    // Feature flags
    pub features: HashSet<String>,
}
```

### Environment Variables

```bash
# Server configuration (unified port architecture)
PIERRE_HTTP_PORT=8081
PIERRE_MAX_CONNECTIONS=1000
PIERRE_REQUEST_TIMEOUT=30

# Database
DATABASE_URL=postgresql://user:pass@localhost/pierre
DATABASE_MAX_CONNECTIONS=100
DATABASE_ENCRYPTION_KEY_PATH=/secrets/encryption.key

# Authentication
JWT_SECRET_PATH=/secrets/jwt.secret
JWT_EXPIRY_HOURS=24
REFRESH_TOKEN_EXPIRY_DAYS=30

# Logging
RUST_LOG=info,pierre_mcp_server=debug
LOG_FORMAT=json
```