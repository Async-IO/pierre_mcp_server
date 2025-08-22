# API Routes and Handlers

## Overview

Pierre MCP Server provides comprehensive HTTP REST API endpoints for web applications, mobile clients, and management interfaces. The API is organized into logical route groups, each handling specific functionality with proper authentication, validation, and error handling.

## Route Architecture

```mermaid
graph TB
    subgraph "Client Applications"
        WebApp[Web Dashboard]
        Mobile[Mobile App]
        Admin[Admin Panel]
        ThirdParty[Third-party Apps]
    end
    
    subgraph "HTTP Layer"
        Router[Axum Router]
        Middleware[Middleware Stack]
        CORS[CORS Handler]
        Logging[Request Logging]
    end
    
    subgraph "Route Groups"
        AuthRoutes[Authentication Routes]
        OAuthRoutes[OAuth Routes]
        ApiKeyRoutes[API Key Routes]
        DashboardRoutes[Dashboard Routes]
        AdminRoutes[Admin Routes]
        A2ARoutes[A2A Routes]
        TenantRoutes[Tenant Routes]
    end
    
    subgraph "Business Logic"
        AuthManager[Auth Manager]
        Database[Database Layer]
        Providers[Fitness Providers]
        RateLimit[Rate Limiter]
    end
    
    WebApp --> Router
    Mobile --> Router
    Admin --> Router
    ThirdParty --> Router
    
    Router --> Middleware
    Middleware --> CORS
    CORS --> Logging
    
    Logging --> AuthRoutes
    Logging --> OAuthRoutes
    Logging --> ApiKeyRoutes
    Logging --> DashboardRoutes
    Logging --> AdminRoutes
    Logging --> A2ARoutes
    Logging --> TenantRoutes
    
    AuthRoutes --> AuthManager
    OAuthRoutes --> Providers
    ApiKeyRoutes --> Database
    DashboardRoutes --> Database
    AdminRoutes --> Database
    A2ARoutes --> Database
    TenantRoutes --> Database
    
    AuthManager --> Database
    Providers --> Database
    Database --> RateLimit
```

## Route Groups

### 1. Authentication Routes (`/api/auth`)

Handles user registration, login, token refresh, and basic authentication operations.

#### Register User

```http
POST /api/auth/register
Content-Type: application/json

{
    "email": "user@example.com",
    "password": "SecurePassword123!",
    "firstname": "John",
    "lastname": "Doe"
}
```

**Implementation:**
```rust
// src/routes.rs
impl AuthRoutes {
    pub async fn register(&self, request: RegisterRequest) -> Result<RegisterResponse> {
        // Validate email format
        if !self.is_valid_email(&request.email) {
            return Err(anyhow!("Invalid email format"));
        }
        
        // Validate password strength
        if !self.is_strong_password(&request.password) {
            return Err(anyhow!("Password does not meet security requirements"));
        }
        
        // Check if user already exists
        if self.database.get_user_by_email(&request.email).await?.is_some() {
            return Err(anyhow!("User with this email already exists"));
        }
        
        // Hash password
        let password_hash = bcrypt::hash(&request.password, bcrypt::DEFAULT_COST)?;
        
        // Create user with pending status
        let user = User {
            id: Uuid::new_v4(),
            email: request.email.clone(),
            password_hash,
            firstname: request.firstname,
            lastname: request.lastname,
            status: UserStatus::Pending,
            tier: UserTier::Free,
            is_admin: false,
            created_at: Utc::now(),
            // ... other fields
        };
        
        let user_id = self.database.create_user(&user).await?;
        
        Ok(RegisterResponse {
            user_id: user_id.to_string(),
            message: "Registration successful. Please wait for admin approval.".to_string(),
        })
    }
}
```

**Response:**
```json
{
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "message": "Registration successful. Please wait for admin approval."
}
```

#### Login User

```http
POST /api/auth/login
Content-Type: application/json

{
    "email": "user@example.com",
    "password": "SecurePassword123!"
}
```

**Implementation:**
```rust
impl AuthRoutes {
    pub async fn login(&self, request: LoginRequest) -> Result<LoginResponse> {
        // Get user by email
        let user = self.database
            .get_user_by_email(&request.email)
            .await?
            .ok_or_else(|| anyhow!("Invalid credentials"))?;
        
        // Verify password
        if !bcrypt::verify(&request.password, &user.password_hash)? {
            return Err(anyhow!("Invalid credentials"));
        }
        
        // Check user status
        if user.status != UserStatus::Active {
            return Err(anyhow!("Account not active. Status: {:?}", user.status));
        }
        
        // Generate JWT token
        let token = self.auth_manager.generate_token(&user)?;
        
        // Update last active
        self.database.update_last_active(user.id).await?;
        
        Ok(LoginResponse {
            jwt_token: token.token,
            expires_at: token.expires_at.to_rfc3339(),
            user: UserInfo {
                user_id: user.id.to_string(),
                email: user.email,
                display_name: user.display_name(),
            },
        })
    }
}
```

**Response:**
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

#### Refresh Token

```http
POST /api/auth/refresh
Content-Type: application/json

{
    "refresh_token": "rt_abc123def456..."
}
```

**Response:**
```json
{
    "jwt_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "expires_at": "2024-01-16T11:30:00Z",
    "refresh_token": "rt_def789ghi012..."
}
```

### 2. OAuth Routes (`/api/oauth`)

Manages OAuth connections with fitness providers.

#### List Available Providers

```http
GET /api/oauth/providers
Authorization: Bearer <jwt_token>
```

**Response:**
```json
{
    "providers": [
        {
            "name": "strava",
            "display_name": "Strava",
            "scopes": ["read", "activity:read_all", "profile:read_all"],
            "connected": false
        },
        {
            "name": "fitbit",
            "display_name": "Fitbit",
            "scopes": ["activity", "heartrate", "profile"],
            "connected": true,
            "expires_at": "2024-06-15T10:00:00Z"
        }
    ]
}
```

#### Initiate OAuth Flow

```http
GET /api/oauth/strava/auth
Authorization: Bearer <jwt_token>
```

**Implementation:**
```rust
// src/routes.rs
impl OAuthRoutes {
    pub async fn initiate_strava_auth(&self, user_id: Uuid) -> Result<OAuthAuthorizationResponse> {
        let state = generate_oauth_state();
        let redirect_uri = format!("{}/api/oauth/strava/callback", self.base_url);
        
        // Store state in database with expiry
        self.database.store_oauth_state(&state, user_id, "strava", Utc::now() + Duration::minutes(10)).await?;
        
        let auth_url = format!(
            "https://www.strava.com/oauth/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
            self.strava_client_id,
            urlencoding::encode(&redirect_uri),
            urlencoding::encode("read,activity:read_all,profile:read_all"),
            state
        );
        
        Ok(OAuthAuthorizationResponse {
            authorization_url: auth_url,
            state,
            instructions: "Visit the URL to authorize Pierre to access your Strava data".to_string(),
            expires_in_minutes: 10,
        })
    }
}
```

**Response:**
```json
{
    "authorization_url": "https://www.strava.com/oauth/authorize?client_id=12345&redirect_uri=...",
    "state": "abc123def456ghi789",
    "instructions": "Visit the URL to authorize Pierre to access your Strava data",
    "expires_in_minutes": 10
}
```

#### OAuth Callback

```http
GET /api/oauth/strava/callback?code=AUTH_CODE&state=abc123def456ghi789
```

**Implementation:**
```rust
impl OAuthRoutes {
    pub async fn handle_strava_callback(
        &self,
        code: String,
        state: String,
    ) -> Result<OAuthCallbackResponse> {
        // Validate state and get user
        let oauth_state = self.database
            .get_oauth_state(&state)
            .await?
            .ok_or_else(|| anyhow!("Invalid or expired OAuth state"))?;
        
        if oauth_state.expires_at < Utc::now() {
            return Err(anyhow!("OAuth state expired"));
        }
        
        // Exchange code for tokens
        let token_response = self.exchange_strava_code(&code).await?;
        
        // Store encrypted tokens
        self.database.update_strava_token(
            oauth_state.user_id,
            &token_response.access_token,
            &token_response.refresh_token,
            Utc::now() + Duration::seconds(token_response.expires_in),
            token_response.scope.unwrap_or_default(),
        ).await?;
        
        // Clean up OAuth state
        self.database.delete_oauth_state(&state).await?;
        
        Ok(OAuthCallbackResponse {
            user_id: oauth_state.user_id.to_string(),
            provider: "strava".to_string(),
            expires_at: (Utc::now() + Duration::seconds(token_response.expires_in)).to_rfc3339(),
            scopes: token_response.scope.unwrap_or_default(),
        })
    }
}
```

#### Check Connection Status

```http
GET /api/oauth/status
Authorization: Bearer <jwt_token>
```

**Response:**
```json
{
    "connections": [
        {
            "provider": "strava",
            "connected": true,
            "expires_at": "2024-06-15T10:00:00Z",
            "scopes": "read,activity:read_all,profile:read_all"
        },
        {
            "provider": "fitbit",
            "connected": false,
            "expires_at": null,
            "scopes": null
        }
    ]
}
```

### 3. API Key Routes (`/api/keys`)

Manages API keys for programmatic access.

#### List User API Keys

```http
GET /api/keys
Authorization: Bearer <jwt_token>
```

**Implementation:**
```rust
// src/api_key_routes.rs
impl ApiKeyRoutes {
    pub async fn list_api_keys(&self, user_id: Uuid) -> Result<ApiKeyListResponse> {
        let api_keys = self.database.get_api_keys_for_user(user_id).await?;
        
        let api_key_infos = api_keys.into_iter().map(|key| ApiKeyInfo {
            id: key.id.to_string(),
            name: key.name,
            description: key.description,
            tier: key.tier,
            key_prefix: key.key_prefix,
            is_active: key.is_active,
            last_used_at: key.last_used_at,
            expires_at: key.expires_at,
            created_at: key.created_at,
        }).collect();
        
        Ok(ApiKeyListResponse {
            api_keys: api_key_infos,
        })
    }
}
```

**Response:**
```json
{
    "api_keys": [
        {
            "id": "770a0622-g4bd-63f6-c938-668877662222",
            "name": "Production API Key",
            "description": "Main API key for production app",
            "tier": "premium",
            "key_prefix": "pk_live_abc123",
            "is_active": true,
            "last_used_at": "2024-01-20T14:30:00Z",
            "expires_at": null,
            "created_at": "2024-01-15T10:00:00Z"
        }
    ]
}
```

#### Create API Key

```http
POST /api/keys
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
    "name": "Mobile App Key",
    "description": "API key for mobile application",
    "tier": "standard",
    "scopes": ["fitness:read", "analytics:read"]
}
```

**Implementation:**
```rust
impl ApiKeyRoutes {
    pub async fn create_api_key(
        &self,
        user_id: Uuid,
        request: CreateApiKeyRequest,
    ) -> Result<ApiKeyCreateResponse> {
        // Validate tier and user permissions
        if !self.can_user_create_tier(&user_id, &request.tier).await? {
            return Err(anyhow!("Insufficient permissions for requested tier"));
        }
        
        // Generate API key
        let api_key = self.api_key_manager.generate_api_key(&request.tier)?;
        
        // Create database record
        let key_record = ApiKey {
            id: Uuid::new_v4(),
            user_id,
            name: request.name.clone(),
            description: request.description.clone(),
            key_hash: hash_api_key(&api_key),
            key_prefix: api_key[..16].to_string(),
            tier: request.tier.clone(),
            scopes: request.scopes.clone(),
            is_active: true,
            created_at: Utc::now(),
            // ... other fields
        };
        
        let key_id = self.database.create_api_key(&key_record).await?;
        
        Ok(ApiKeyCreateResponse {
            api_key,
            key_info: ApiKeyInfo::from(key_record),
            warning: "Store this API key securely. It will not be shown again.".to_string(),
        })
    }
}
```

**Response:**
```json
{
    "api_key": "pk_live_abc123def456ghi789jkl012mno345pqr678stu901vwx234yz",
    "key_info": {
        "id": "880b1733-h5ce-74g7-d049-779988773333",
        "name": "Mobile App Key",
        "description": "API key for mobile application",
        "tier": "standard",
        "key_prefix": "pk_live_abc123",
        "is_active": true,
        "last_used_at": null,
        "expires_at": null,
        "created_at": "2024-01-20T15:00:00Z"
    },
    "warning": "Store this API key securely. It will not be shown again."
}
```

### 4. Dashboard Routes (`/api/dashboard`)

Provides data for web dashboard and analytics.

#### Dashboard Overview

```http
GET /api/dashboard
Authorization: Bearer <jwt_token>
```

**Implementation:**
```rust
// src/dashboard_routes.rs
impl DashboardRoutes {
    pub async fn get_overview(&self, user_id: Uuid) -> Result<DashboardOverview> {
        let today = Utc::now().date_naive();
        let month_start = today.with_day(1).unwrap();
        
        // Get API key statistics
        let api_keys = self.database.get_api_keys_for_user(user_id).await?;
        let total_api_keys = api_keys.len() as u32;
        let active_api_keys = api_keys.iter().filter(|k| k.is_active).count() as u32;
        
        // Get usage statistics
        let today_usage = self.database.get_usage_for_date(user_id, today).await?;
        let month_usage = self.database.get_usage_for_period(user_id, month_start, today).await?;
        
        // Get usage by tier
        let tier_usage = self.calculate_tier_usage(&api_keys, &month_usage).await?;
        
        // Get recent activity
        let recent_activity = self.database.get_recent_activity(user_id, 10).await?;
        
        Ok(DashboardOverview {
            total_api_keys,
            active_api_keys,
            total_requests_today: today_usage.total_requests,
            total_requests_this_month: month_usage.total_requests,
            current_month_usage_by_tier: tier_usage,
            recent_activity,
        })
    }
}
```

**Response:**
```json
{
    "total_api_keys": 3,
    "active_api_keys": 2,
    "total_requests_today": 1247,
    "total_requests_this_month": 45678,
    "current_month_usage_by_tier": [
        {
            "tier": "premium",
            "key_count": 1,
            "total_requests": 35000,
            "average_requests_per_key": 35000.0
        },
        {
            "tier": "standard",
            "key_count": 1,
            "total_requests": 10678,
            "average_requests_per_key": 10678.0
        }
    ],
    "recent_activity": [
        {
            "timestamp": "2024-01-20T16:45:00Z",
            "api_key_name": "Production API Key",
            "tool_name": "get_activities",
            "status_code": 200,
            "response_time_ms": 234
        }
    ]
}
```

#### Usage Analytics

```http
GET /api/dashboard/analytics?timeframe=7d
Authorization: Bearer <jwt_token>
```

**Response:**
```json
{
    "time_series": [
        {
            "timestamp": "2024-01-20T00:00:00Z",
            "request_count": 1247,
            "error_count": 12,
            "average_response_time": 234.5
        }
    ],
    "top_tools": [
        {
            "tool_name": "get_activities",
            "request_count": 5678,
            "success_rate": 0.987,
            "average_response_time": 245.2
        }
    ],
    "error_rate": 0.013,
    "average_response_time": 234.5
}
```

### 5. Admin Routes (`/api/admin`)

Administrative endpoints for user and system management.

#### List All Users

```http
GET /api/admin/users?status=pending&page=1&limit=20
Authorization: Bearer <admin_jwt_token>
```

**Implementation:**
```rust
// src/admin_routes.rs
impl AdminRoutes {
    pub async fn list_users(
        &self,
        admin_user_id: Uuid,
        status_filter: Option<String>,
        page: Option<u32>,
        limit: Option<u32>,
    ) -> Result<UserListResponse> {
        // Verify admin permissions
        self.verify_admin_permissions(admin_user_id).await?;
        
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(20).min(100); // Max 100 per page
        let offset = (page - 1) * limit;
        
        let users = if let Some(status) = status_filter {
            self.database.get_users_by_status(&status).await?
        } else {
            self.database.get_all_users_paginated(offset, limit).await?
        };
        
        let total_count = self.database.get_user_count().await?;
        
        Ok(UserListResponse {
            users: users.into_iter().map(UserInfo::from).collect(),
            total_count: total_count as u32,
            page,
            limit,
            total_pages: ((total_count as f64) / (limit as f64)).ceil() as u32,
        })
    }
}
```

**Response:**
```json
{
    "users": [
        {
            "user_id": "550e8400-e29b-41d4-a716-446655440000",
            "email": "user@example.com",
            "display_name": "John Doe",
            "status": "pending",
            "tier": "free",
            "created_at": "2024-01-20T10:00:00Z",
            "last_active": null
        }
    ],
    "total_count": 156,
    "page": 1,
    "limit": 20,
    "total_pages": 8
}
```

#### Approve User

```http
POST /api/admin/users/550e8400-e29b-41d4-a716-446655440000/approve
Authorization: Bearer <admin_jwt_token>
Content-Type: application/json

{
    "tier": "basic",
    "notes": "Approved for basic tier access"
}
```

**Response:**
```json
{
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "active",
    "tier": "basic",
    "approved_at": "2024-01-20T16:30:00Z",
    "approved_by": "admin@example.com"
}
```

### 6. A2A Routes (`/api/a2a`)

Agent-to-Agent system registration and management.

#### Register A2A System

```http
POST /api/a2a/register
Authorization: Bearer <admin_jwt_token>
Content-Type: application/json

{
    "name": "Fitness Analytics Bot",
    "description": "Analytics platform for fitness data processing",
    "capabilities": ["fitness-analysis", "goal-tracking", "recommendations"],
    "webhook_url": "https://analytics.example.com/webhook",
    "rate_limit": {
        "requests_per_day": 10000,
        "burst_size": 100
    }
}
```

**Implementation:**
```rust
// src/a2a_routes.rs
impl A2ARoutes {
    pub async fn register_system(
        &self,
        admin_user_id: Uuid,
        request: A2ARegistrationRequest,
    ) -> Result<A2ARegistrationResponse> {
        // Verify admin permissions
        self.verify_admin_permissions(admin_user_id).await?;
        
        // Generate API key for A2A system
        let api_key = format!("A2A_{}", generate_secure_token(32));
        let api_key_hash = hash_api_key(&api_key);
        
        // Create A2A client record
        let a2a_client = A2AClient {
            id: Uuid::new_v4(),
            name: request.name.clone(),
            description: request.description,
            api_key_hash,
            capabilities: request.capabilities,
            webhook_url: request.webhook_url,
            rate_limit: request.rate_limit.unwrap_or_default(),
            is_active: true,
            created_at: Utc::now(),
            // ... other fields
        };
        
        let client_id = self.database.create_a2a_client(&a2a_client).await?;
        
        Ok(A2ARegistrationResponse {
            system_id: client_id,
            name: request.name,
            api_key,
            status: "active".to_string(),
            created_at: Utc::now(),
            rate_limit: a2a_client.rate_limit,
        })
    }
}
```

**Response:**
```json
{
    "system_id": "990c2844-i6df-85h8-e15a-88aa99884444",
    "name": "Fitness Analytics Bot",
    "api_key": "A2A_abc123def456ghi789jkl012mno345pqr678stu901vwx234yz",
    "status": "active",
    "created_at": "2024-01-20T17:00:00Z",
    "rate_limit": {
        "requests_per_day": 10000,
        "burst_size": 100
    }
}
```

#### Get Agent Card

```http
GET /api/a2a/agent-card
X-API-Key: A2A_abc123def456ghi789jkl012mno345pqr678stu901vwx234yz
```

**Response:**
```json
{
    "name": "Pierre Fitness AI",
    "description": "AI-powered fitness data analysis and insights platform",
    "version": "1.0.0",
    "capabilities": [
        "fitness-data-analysis",
        "activity-intelligence",
        "goal-management",
        "performance-prediction"
    ],
    "authentication": {
        "schemes": ["api-key"],
        "api_key": {
            "header_name": "X-API-Key",
            "prefix": "A2A_",
            "registration_url": "https://pierre-api.example.com/api/a2a/register"
        }
    },
    "tools": [
        {
            "name": "analyze_activity",
            "description": "Perform detailed analysis of a fitness activity",
            "input_schema": {
                "type": "object",
                "properties": {
                    "activity_data": {
                        "type": "object",
                        "description": "Raw activity data"
                    }
                }
            }
        }
    ]
}
```

## Middleware Stack

### Authentication Middleware

```rust
// src/middleware/auth.rs
pub struct AuthMiddleware {
    auth_manager: Arc<AuthManager>,
    database: Arc<Database>,
}

impl AuthMiddleware {
    pub async fn authenticate_request(
        &self,
        req: &Request,
    ) -> Result<AuthContext, AuthError> {
        // Extract authorization header
        let auth_header = req.headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or(AuthError::MissingToken)?;
        
        if auth_header.starts_with("Bearer ") {
            // JWT authentication
            let token = &auth_header[7..];
            let claims = self.auth_manager.verify_token(token)?;
            
            let user = self.database
                .get_user(&Uuid::parse_str(&claims.sub)?)
                .await?
                .ok_or(AuthError::UserNotFound)?;
            
            Ok(AuthContext::User {
                user_id: user.id,
                tenant_id: user.tenant_id,
                role: user.role,
                tier: user.tier,
            })
        } else if let Some(api_key) = req.headers().get("X-API-Key") {
            // API key authentication
            let key_str = api_key.to_str().map_err(|_| AuthError::InvalidApiKey)?;
            
            if key_str.starts_with("A2A_") {
                // A2A system authentication
                let system = self.database
                    .get_system_user_by_api_key(&hash_api_key(key_str))
                    .await?
                    .ok_or(AuthError::InvalidApiKey)?;
                
                Ok(AuthContext::A2ASystem {
                    system_id: system.id,
                    capabilities: system.capabilities,
                    rate_limit: system.rate_limit,
                })
            } else {
                // Regular API key authentication
                let api_key = self.database
                    .get_api_key_by_hash(&hash_api_key(key_str))
                    .await?
                    .ok_or(AuthError::InvalidApiKey)?;
                
                Ok(AuthContext::ApiKey {
                    key_id: api_key.id,
                    user_id: api_key.user_id,
                    tier: api_key.tier,
                    scopes: api_key.scopes,
                })
            }
        } else {
            Err(AuthError::InvalidAuthMethod)
        }
    }
}
```

### Rate Limiting Middleware

```rust
// src/middleware/rate_limit.rs
pub struct RateLimitMiddleware {
    rate_limiter: Arc<RateLimiter>,
}

impl RateLimitMiddleware {
    pub async fn check_rate_limit(
        &self,
        auth_context: &AuthContext,
        request_path: &str,
    ) -> Result<RateLimitInfo, RateLimitError> {
        match auth_context {
            AuthContext::User { user_id, tier, .. } => {
                self.rate_limiter.check_user_limit(*user_id, tier, request_path).await
            }
            AuthContext::ApiKey { key_id, tier, .. } => {
                self.rate_limiter.check_api_key_limit(*key_id, tier, request_path).await
            }
            AuthContext::A2ASystem { system_id, rate_limit, .. } => {
                self.rate_limiter.check_system_limit(*system_id, rate_limit, request_path).await
            }
        }
    }
}
```

### Request Logging Middleware

```rust
// src/middleware/logging.rs
pub struct RequestLoggingMiddleware;

impl RequestLoggingMiddleware {
    pub async fn log_request(
        &self,
        req: &Request,
        response: &Response,
        duration: Duration,
        auth_context: &AuthContext,
    ) -> Result<()> {
        let log_entry = RequestLog {
            timestamp: Utc::now(),
            method: req.method().to_string(),
            path: req.uri().path().to_string(),
            status_code: response.status().as_u16(),
            duration_ms: duration.as_millis() as u32,
            user_agent: req.headers()
                .get("user-agent")
                .and_then(|h| h.to_str().ok())
                .map(String::from),
            ip_address: extract_client_ip(req),
            auth_method: auth_context.auth_method(),
            user_id: auth_context.user_id(),
        };
        
        // Log to structured logging
        info!(
            method = log_entry.method,
            path = log_entry.path,
            status = log_entry.status_code,
            duration_ms = log_entry.duration_ms,
            user_id = ?log_entry.user_id,
            "HTTP request processed"
        );
        
        // Store in database for analytics
        self.database.store_request_log(&log_entry).await?;
        
        Ok(())
    }
}
```

## Error Handling

### Standardized Error Responses

```rust
// src/errors.rs
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorInfo,
    pub timestamp: DateTime<Utc>,
    pub request_id: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
    pub details: Option<Value>,
}

impl From<AppError> for ErrorResponse {
    fn from(error: AppError) -> Self {
        Self {
            error: ErrorInfo {
                code: error.code().to_string(),
                message: error.message(),
                details: error.details(),
            },
            timestamp: Utc::now(),
            request_id: Uuid::new_v4().to_string(),
        }
    }
}
```

### Error Response Examples

```json
// Authentication Error
{
    "error": {
        "code": "AUTH_INVALID_TOKEN",
        "message": "Invalid or expired JWT token",
        "details": {
            "token_expired": true,
            "expired_at": "2024-01-20T10:00:00Z"
        }
    },
    "timestamp": "2024-01-20T16:30:00Z",
    "request_id": "req_abc123def456"
}

// Validation Error
{
    "error": {
        "code": "VALIDATION_ERROR",
        "message": "Request validation failed",
        "details": {
            "field_errors": [
                {
                    "field": "email",
                    "message": "Invalid email format"
                },
                {
                    "field": "password",
                    "message": "Password must be at least 8 characters"
                }
            ]
        }
    },
    "timestamp": "2024-01-20T16:30:00Z",
    "request_id": "req_def789ghi012"
}

// Rate Limit Error
{
    "error": {
        "code": "RATE_LIMIT_EXCEEDED",
        "message": "Rate limit exceeded for your tier",
        "details": {
            "limit": 1000,
            "used": 1000,
            "reset_at": "2024-02-01T00:00:00Z",
            "tier": "basic"
        }
    },
    "timestamp": "2024-01-20T16:30:00Z",
    "request_id": "req_ghi345jkl678"
}
```

## API Documentation

### OpenAPI Specification

The API routes are documented using OpenAPI 3.0 specification:

```yaml
# docs/openapi.yaml
openapi: 3.0.0
info:
  title: Pierre MCP Server API
  description: Comprehensive fitness data API for AI assistants and applications
  version: 1.0.0
  license:
    name: MIT OR Apache-2.0

servers:
  - url: https://pierre-api.example.com
    description: Production server
  - url: https://staging-pierre-api.example.com
    description: Staging server

paths:
  /api/auth/register:
    post:
      summary: Register new user
      tags:
        - Authentication
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/RegisterRequest'
      responses:
        '201':
          description: User registered successfully
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/RegisterResponse'
        '400':
          description: Validation error
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ErrorResponse'

components:
  schemas:
    RegisterRequest:
      type: object
      required:
        - email
        - password
      properties:
        email:
          type: string
          format: email
          example: user@example.com
        password:
          type: string
          minLength: 8
          example: SecurePassword123!
        firstname:
          type: string
          example: John
        lastname:
          type: string
          example: Doe
          
  securitySchemes:
    BearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT
    ApiKeyAuth:
      type: apiKey
      in: header
      name: X-API-Key
```

This comprehensive API routes documentation provides developers with everything needed to integrate with Pierre MCP Server's HTTP endpoints, including authentication, error handling, and proper request/response patterns.