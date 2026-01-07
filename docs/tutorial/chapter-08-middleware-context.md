<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 08: Middleware & Request Context

This chapter explores how the Pierre Fitness Platform uses Axum middleware to extract authentication, tenant context, rate limiting information, and tracing data from HTTP requests before routing to handlers. You'll learn about middleware composition, request ID generation, CORS configuration, and PII-safe logging.

## What You'll Learn

- Axum middleware architecture and Tower layers
- Request ID generation for distributed tracing
- Request context lifecycle and propagation
- CORS configuration for web clients
- Rate limiting headers (X-RateLimit-* family)
- PII redaction and sensitive data protection
- Middleware ordering and composition
- Structured logging with tenant context
- Request/response tracing spans
- Security headers and best practices

## Middleware Stack Overview

The Pierre platform uses a layered middleware stack that processes every HTTP request before it reaches handlers:

```
┌────────────────────────────────────────────────────────────┐
│                      HTTP Request                          │
└───────────────────────┬────────────────────────────────────┘
                        │
                        ▼
          ┌──────────────────────────┐
          │   CORS Middleware        │  ← Allow cross-origin requests
          │   (OPTIONS preflight)    │
          └──────────────────────────┘
                        │
                        ▼
          ┌──────────────────────────┐
          │   Request ID Middleware  │  ← Generate UUID for tracing
          │   x-request-id: ...      │
          └──────────────────────────┘
                        │
                        ▼
          ┌──────────────────────────┐
          │   Tracing Middleware     │  ← Create span with metadata
          │   RequestContext         │
          └──────────────────────────┘
                        │
                        ▼
          ┌──────────────────────────┐
          │   Auth Middleware        │  ← Validate JWT/API key
          │   Extract user_id        │
          └──────────────────────────┘
                        │
                        ▼
          ┌──────────────────────────┐
          │   Tenant Middleware      │  ← Extract tenant context
          │   TenantContext          │
          └──────────────────────────┘
                        │
                        ▼
          ┌──────────────────────────┐
          │   Rate Limit Middleware  │  ← Check usage limits
          │   Add X-RateLimit-*      │
          └──────────────────────────┘
                        │
                        ▼
          ┌──────────────────────────┐
          │   Route Handler          │  ← Business logic
          │   Process request        │
          └──────────────────────────┘
                        │
                        ▼
          ┌──────────────────────────┐
          │   Response               │  ← Add security headers
          │   x-request-id: ...      │
          └──────────────────────────┘
```

**Source**: src/middleware/mod.rs:1-77
```rust
// ABOUTME: HTTP middleware for request tracing, authentication, and context propagation
// ABOUTME: Provides request ID generation, span creation, and tenant context for structured logging

/// Authentication middleware for MCP and API requests
pub mod auth;
/// CORS middleware configuration
pub mod cors;
/// Rate limiting middleware and utilities
pub mod rate_limiting;
/// PII redaction and sensitive data masking
pub mod redaction;
/// Request ID generation and propagation
pub mod request_id;
/// Request tracing and context propagation
pub mod tracing;

// Authentication middleware

/// MCP authentication middleware
pub use auth::McpAuthMiddleware;

// CORS middleware

/// Setup CORS layer for HTTP endpoints
pub use cors::setup_cors;

// Rate limiting middleware and utilities

/// Check rate limit and send error response
pub use rate_limiting::check_rate_limit_and_respond;
/// Create rate limit error
pub use rate_limiting::create_rate_limit_error;
/// Create rate limit headers
pub use rate_limiting::create_rate_limit_headers;
/// Rate limit headers module
pub use rate_limiting::headers;

// PII-safe logging and redaction

/// Mask email addresses for logging
pub use redaction::mask_email;
/// Redact sensitive HTTP headers
pub use redaction::redact_headers;
/// Redact JSON fields by pattern
pub use redaction::redact_json_fields;
/// Redact token patterns from strings
pub use redaction::redact_token_patterns;
/// Bounded tenant label for tracing
pub use redaction::BoundedTenantLabel;
/// Bounded user label for tracing
pub use redaction::BoundedUserLabel;
/// Redaction configuration
pub use redaction::RedactionConfig;
/// Redaction features toggle
pub use redaction::RedactionFeatures;

// Request ID middleware

/// Request ID middleware function
pub use request_id::request_id_middleware;
/// Request ID extractor
pub use request_id::RequestId;

// Request tracing and context management

/// Create database operation span
pub use tracing::create_database_span;
/// Create MCP operation span
pub use tracing::create_mcp_span;
/// Create HTTP request span
pub use tracing::create_request_span;
/// Request context for tracing
pub use tracing::RequestContext;
```

**Rust Idiom**: Re-exporting with `pub use`

The `middleware/mod.rs` file acts as a facade, re-exporting commonly used types from submodules. This allows handlers to `use crate::middleware::RequestId` instead of `use crate::middleware::request_id::RequestId`, reducing coupling to internal module organization.

## Request ID Generation

Every HTTP request receives a unique identifier for distributed tracing and log correlation:

**Source**: src/middleware/request_id.rs:39-61
```rust
/// Request ID middleware that generates and propagates correlation IDs
///
/// This middleware:
/// 1. Generates a unique UUID v4 for each request
/// 2. Adds the request ID to request extensions for handler access
/// 3. Records the request ID in the current tracing span
/// 4. Includes the request ID in the response header
pub async fn request_id_middleware(mut req: Request, next: Next) -> Response {
    // Generate unique request ID
    let request_id = Uuid::new_v4().to_string();

    // Record request ID in current tracing span
    let span = Span::current();
    span.record("request_id", &request_id);

    // Add to request extensions for handler access
    req.extensions_mut().insert(RequestId(request_id.clone()));

    // Process request
    let mut response = next.run(req).await;

    // Add request ID to response header
    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        response
            .headers_mut()
            .insert(REQUEST_ID_HEADER, header_value);
    }

    response
}
```

**Flow**:
1. **Generate**: Create UUID v4 for globally unique ID
2. **Record**: Add to current tracing span for structured logs
3. **Extend**: Store in request extensions for handler access
4. **Process**: Call next middleware/handler with `next.run(req)`
5. **Respond**: Include `x-request-id` header in response

**Rust Idiom**: Request extensions for typed data

Axum's `req.extensions_mut().insert(RequestId(...))` provides type-safe request-scoped storage. Handlers can extract `RequestId` using:
```rust
async fn handler(Extension(request_id): Extension<RequestId>) -> String {
    format!("Request ID: {}", request_id.0)
}
```

The type system ensures you can't accidentally insert or extract the wrong type.

### Requestid Extractor

**Source**: src/middleware/request_id.rs:75-90
```rust
/// Request ID extractor for use in handlers
///
/// This can be extracted in any Axum handler to access the request ID
/// generated by the middleware.
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

impl RequestId {
    /// Get the request ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

**Newtype pattern**: Wrapping `String` in `RequestId` provides:
- Type safety: Can't confuse request ID with other strings
- Display trait: Use `{request_id}` in format strings
- Documentation: Self-documenting API (function signature says "I need a RequestId")

## Request Context and Tracing

The `RequestContext` struct flows through the entire request lifecycle, accumulating metadata:

**Source**: src/middleware/tracing.rs:10-67
```rust
/// Request context that flows through the entire request lifecycle
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Unique identifier for this request
    pub request_id: String,
    /// Authenticated user ID (if available)
    pub user_id: Option<Uuid>,
    /// Tenant ID for multi-tenancy (if available)
    pub tenant_id: Option<Uuid>,
    /// Authentication method used (e.g., "Bearer", "ApiKey")
    pub auth_method: Option<String>,
}

impl RequestContext {
    /// Create new request context with generated request ID
    #[must_use]
    pub fn new() -> Self {
        Self {
            request_id: format!("req_{}", Uuid::new_v4().simple()),
            user_id: None,
            tenant_id: None,
            auth_method: None,
        }
    }

    /// Update context with authentication information
    #[must_use]
    pub fn with_auth(mut self, user_id: Uuid, auth_method: String) -> Self {
        self.user_id = Some(user_id);
        self.tenant_id = Some(user_id); // For now, user_id serves as tenant_id
        self.auth_method = Some(auth_method);
        self
    }

    /// Record context in current tracing span
    pub fn record_in_span(&self) {
        let span = Span::current();
        span.record("request_id", &self.request_id);

        if let Some(user_id) = &self.user_id {
            span.record("user_id", user_id.to_string());
        }

        if let Some(tenant_id) = &self.tenant_id {
            span.record("tenant_id", tenant_id.to_string());
        }

        if let Some(auth_method) = &self.auth_method {
            span.record("auth_method", auth_method);
        }
    }
}
```

**Builder pattern**: The `with_auth` method allows chaining:
```rust
let context = RequestContext::new()
    .with_auth(user_id, "Bearer".into());
```

**Span recording**: The `record_in_span` method populates tracing fields declared as `Empty`:
```rust
let span = tracing::info_span!("request", user_id = tracing::field::Empty);
context.record_in_span(); // Now span has user_id field
```

### Span Creation Utilities

The platform provides helpers for creating tracing spans with pre-configured fields:

**Source**: src/middleware/tracing.rs:69-110
```rust
/// Create a tracing span for HTTP requests
pub fn create_request_span(method: &str, path: &str) -> tracing::Span {
    tracing::info_span!(
        "http_request",
        method = %method,
        path = %path,
        request_id = tracing::field::Empty,
        user_id = tracing::field::Empty,
        tenant_id = tracing::field::Empty,
        auth_method = tracing::field::Empty,
        status_code = tracing::field::Empty,
        duration_ms = tracing::field::Empty,
    )
}

/// Create a tracing span for MCP operations
pub fn create_mcp_span(operation: &str) -> tracing::Span {
    tracing::info_span!(
        "mcp_operation",
        operation = %operation,
        request_id = tracing::field::Empty,
        user_id = tracing::field::Empty,
        tenant_id = tracing::field::Empty,
        tool_name = tracing::field::Empty,
        duration_ms = tracing::field::Empty,
        success = tracing::field::Empty,
    )
}

/// Create a tracing span for database operations
pub fn create_database_span(operation: &str, table: &str) -> tracing::Span {
    tracing::debug_span!(
        "database_operation",
        operation = %operation,
        table = %table,
        request_id = tracing::field::Empty,
        user_id = tracing::field::Empty,
        tenant_id = tracing::field::Empty,
        duration_ms = tracing::field::Empty,
        rows_affected = tracing::field::Empty,
    )
}
```

**Usage pattern**:
```rust
async fn handle_request() -> Result<Response> {
    let span = create_request_span("POST", "/api/activities");
    let _guard = span.enter();

    // All logs within this scope include span fields
    tracing::info!("Processing activity request");

    // Later: record additional fields
    Span::current().record("status_code", 200);
    Span::current().record("duration_ms", 42);

    Ok(response)
}
```

## CORS Configuration

The platform configures Cross-Origin Resource Sharing (CORS) for web client access:

**Source**: src/middleware/cors.rs:40-96
```rust
/// Configure CORS settings for the MCP server
///
/// Configures cross-origin requests based on `CORS_ALLOWED_ORIGINS` environment variable.
/// Supports both wildcard ("*") for development and specific origin lists for production.
pub fn setup_cors(config: &crate::config::environment::ServerConfig) -> CorsLayer {
    // Parse allowed origins from configuration
    let allow_origin =
        if config.cors.allowed_origins.is_empty() || config.cors.allowed_origins == "*" {
            // Development mode: allow any origin
            AllowOrigin::any()
        } else {
            // Production mode: parse comma-separated origin list
            let origins: Vec<HeaderValue> = config
                .cors
                .allowed_origins
                .split(',')
                .filter_map(|s| {
                    let trimmed = s.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        HeaderValue::from_str(trimmed).ok()
                    }
                })
                .collect();

            if origins.is_empty() {
                // Fallback to any if parsing failed
                AllowOrigin::any()
            } else {
                AllowOrigin::list(origins)
            }
        };

    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_headers([
            HeaderName::from_static("content-type"),
            HeaderName::from_static("authorization"),
            HeaderName::from_static("x-requested-with"),
            HeaderName::from_static("accept"),
            HeaderName::from_static("origin"),
            HeaderName::from_static("access-control-request-method"),
            HeaderName::from_static("access-control-request-headers"),
            HeaderName::from_static("x-strava-client-id"),
            HeaderName::from_static("x-strava-client-secret"),
            HeaderName::from_static("x-fitbit-client-id"),
            HeaderName::from_static("x-fitbit-client-secret"),
            HeaderName::from_static("x-pierre-api-key"),
            HeaderName::from_static("x-tenant-name"),
            HeaderName::from_static("x-tenant-id"),
        ])
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
            Method::PATCH,
        ])
}
```

**Configuration examples**:
```bash
# Development: allow all origins
export CORS_ALLOWED_ORIGINS="*"

# Production: specific origins only
export CORS_ALLOWED_ORIGINS="https://app.pierre.fitness,https://admin.pierre.fitness"
```

**Security**: The platform allows custom headers for:
- **Provider OAuth**: `x-strava-client-id`, `x-fitbit-client-id` for dynamic OAuth configuration
- **Multi-tenancy**: `x-tenant-name`, `x-tenant-id` for tenant routing
- **API keys**: `x-pierre-api-key` for alternative authentication

**Rust Idiom**: `filter_map` for parsing

The CORS configuration uses `filter_map` to parse origin strings while skipping invalid entries:
```rust
config.cors.allowed_origins
    .split(',')
    .filter_map(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None  // Skip empty strings
        } else {
            HeaderValue::from_str(trimmed).ok()  // Parse or skip invalid
        }
    })
    .collect();
```

This handles malformed configuration gracefully without panicking.

## Rate Limiting Headers

The platform adds standard HTTP rate limiting headers to all responses:

**Source**: src/middleware/rate_limiting.rs:17-32
```rust
/// HTTP header names for rate limiting
pub mod headers {
    /// HTTP header name for maximum requests allowed in the current window
    pub const X_RATE_LIMIT_LIMIT: &str = "X-RateLimit-Limit";
    /// HTTP header name for remaining requests in the current window
    pub const X_RATE_LIMIT_REMAINING: &str = "X-RateLimit-Remaining";
    /// HTTP header name for Unix timestamp when rate limit resets
    pub const X_RATE_LIMIT_RESET: &str = "X-RateLimit-Reset";
    /// HTTP header name for rate limit window duration in seconds
    pub const X_RATE_LIMIT_WINDOW: &str = "X-RateLimit-Window";
    /// HTTP header name for rate limit tier information
    pub const X_RATE_LIMIT_TIER: &str = "X-RateLimit-Tier";
    /// HTTP header name for authentication method used
    pub const X_RATE_LIMIT_AUTH_METHOD: &str = "X-RateLimit-AuthMethod";
    /// HTTP header name for retry-after duration in seconds
    pub const RETRY_AFTER: &str = "Retry-After";
}
```

**Standard headers**:
- `X-RateLimit-Limit`: Total requests allowed (e.g., "5000")
- `X-RateLimit-Remaining`: Requests left in window (e.g., "4832")
- `X-RateLimit-Reset`: Unix timestamp when limit resets (e.g., "1706054400")
- `Retry-After`: Seconds until reset for 429 responses (e.g., "3600")

**Custom headers**:
- `X-RateLimit-Window`: Duration in seconds (e.g., "2592000" for 30 days)
- `X-RateLimit-Tier`: User's subscription tier (e.g., "free", "premium")
- `X-RateLimit-AuthMethod`: Authentication type (e.g., "JwtToken", "ApiKey")

### Creating Rate Limit Headers

**Source**: src/middleware/rate_limiting.rs:34-82
```rust
/// Create a `HeaderMap` with rate limit headers
#[must_use]
pub fn create_rate_limit_headers(rate_limit_info: &UnifiedRateLimitInfo) -> HeaderMap {
    let mut headers = HeaderMap::new();

    // Add rate limit headers if we have the information
    if let Some(limit) = rate_limit_info.limit {
        if let Ok(header_value) = HeaderValue::from_str(&limit.to_string()) {
            headers.insert(headers::X_RATE_LIMIT_LIMIT, header_value);
        }
    }

    if let Some(remaining) = rate_limit_info.remaining {
        if let Ok(header_value) = HeaderValue::from_str(&remaining.to_string()) {
            headers.insert(headers::X_RATE_LIMIT_REMAINING, header_value);
        }
    }

    if let Some(reset_at) = rate_limit_info.reset_at {
        // Add reset timestamp as Unix epoch
        let reset_timestamp = reset_at.timestamp();
        if let Ok(header_value) = HeaderValue::from_str(&reset_timestamp.to_string()) {
            headers.insert(headers::X_RATE_LIMIT_RESET, header_value);
        }

        // Add Retry-After header (seconds until reset)
        let retry_after = (reset_at - chrono::Utc::now()).num_seconds().max(0);
        if let Ok(header_value) = HeaderValue::from_str(&retry_after.to_string()) {
            headers.insert(headers::RETRY_AFTER, header_value);
        }
    }

    // Add tier and authentication method information
    if let Ok(header_value) = HeaderValue::from_str(&rate_limit_info.tier) {
        headers.insert(headers::X_RATE_LIMIT_TIER, header_value);
    }

    if let Ok(header_value) = HeaderValue::from_str(&rate_limit_info.auth_method) {
        headers.insert(headers::X_RATE_LIMIT_AUTH_METHOD, header_value);
    }

    // Add rate limit window (always 30 days for monthly limits)
    headers.insert(
        headers::X_RATE_LIMIT_WINDOW,
        HeaderValue::from_static("2592000"), // 30 days in seconds
    );

    headers
}
```

**Error handling**: All header insertions use `if let Ok(...)` to gracefully handle invalid header values. If conversion fails, the header is skipped rather than panicking.

**Rust Idiom**: `HeaderValue::from_static`

The `X_RATE_LIMIT_WINDOW` uses `from_static` for compile-time constant strings, avoiding runtime allocation. For dynamic values, use `HeaderValue::from_str` which validates UTF-8 and HTTP header constraints.

### Rate Limit Error Responses

**Source**: src/middleware/rate_limiting.rs:84-111
```rust
/// Create a rate limit exceeded error response with proper headers
#[must_use]
pub fn create_rate_limit_error(rate_limit_info: &UnifiedRateLimitInfo) -> AppError {
    let limit = rate_limit_info.limit.unwrap_or(0);

    AppError::new(
        ErrorCode::RateLimitExceeded,
        format!(
            "Rate limit exceeded. You have reached your limit of {} requests for the {} tier",
            limit, rate_limit_info.tier
        ),
    )
}

/// Helper function to check rate limits and return appropriate response
///
/// # Errors
///
/// Returns an error if the rate limit has been exceeded
pub fn check_rate_limit_and_respond(
    rate_limit_info: &UnifiedRateLimitInfo,
) -> Result<(), AppError> {
    if rate_limit_info.is_rate_limited {
        Err(create_rate_limit_error(rate_limit_info))
    } else {
        Ok(())
    }
}
```

**Usage in handlers**:
```rust
async fn api_handler(auth: AuthResult) -> Result<Json<Response>> {
    // Check rate limit first
    check_rate_limit_and_respond(&auth.rate_limit)?;

    // Process request
    let data = fetch_data().await?;

    Ok(Json(Response { data }))
}
```

## Pii Redaction and Data Protection

The platform redacts Personally Identifiable Information (PII) from logs to comply with GDPR, CCPA, and other privacy regulations:

**Source**: src/middleware/redaction.rs:38-95
```rust
bitflags! {
    /// Redaction feature flags to control which types of data to redact
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct RedactionFeatures: u8 {
        /// Redact HTTP headers (Authorization, Cookie, etc.)
        const HEADERS = 0b0001;
        /// Redact JSON body fields (client_secret, tokens, etc.)
        const BODY_FIELDS = 0b0010;
        /// Mask email addresses
        const EMAILS = 0b0100;
        /// Enable all redaction features
        const ALL = Self::HEADERS.bits() | Self::BODY_FIELDS.bits() | Self::EMAILS.bits();
    }
}

/// Configuration for PII redaction
#[derive(Debug, Clone)]
pub struct RedactionConfig {
    /// Enable redaction globally (default: true in production, false in dev)
    pub enabled: bool,
    /// Which redaction features to enable
    pub features: RedactionFeatures,
    /// Replacement string for redacted sensitive data
    pub redaction_placeholder: String,
}

impl Default for RedactionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            features: RedactionFeatures::ALL,
            redaction_placeholder: "[REDACTED]".to_owned(),
        }
    }
}

impl RedactionConfig {
    /// Create redaction config from environment
    #[must_use]
    pub fn from_env() -> Self {
        let config = crate::constants::get_server_config();
        let enabled = config.is_none_or(|c| c.logging.redact_pii);

        let features = if enabled {
            RedactionFeatures::ALL
        } else {
            RedactionFeatures::empty()
        };

        Self {
            enabled,
            features,
            redaction_placeholder: config.map_or_else(
                || "[REDACTED]".to_owned(),
                |c| c.logging.redaction_placeholder.clone(),
            ),
        }
    }

    /// Check if redaction is disabled
    #[must_use]
    pub const fn is_disabled(&self) -> bool {
        !self.enabled
    }
}
```

**Bitflags pattern**: Using the `bitflags!` macro allows fine-grained control:
```rust
// Enable only header and email redaction, skip body fields
let features = RedactionFeatures::HEADERS | RedactionFeatures::EMAILS;

// Check if headers should be redacted
if features.contains(RedactionFeatures::HEADERS) {
    redact_authorization_header();
}
```

**Configuration**:
```bash
# Disable PII redaction in development
export REDACT_PII=false

# Customize redaction placeholder
export REDACTION_PLACEHOLDER="***"
```

### Sensitive Headers

The platform redacts sensitive HTTP headers before logging:
- `Authorization`: JWT tokens and API keys
- `Cookie`: Session cookies
- `X-API-Key`: Alternative API key header
- `X-Strava-Client-Secret`: Provider OAuth secrets
- `X-Fitbit-Client-Secret`: Provider OAuth secrets

### Email Masking

Email addresses are masked to prevent PII leakage:
```rust
mask_email("john.doe@example.com")
// Returns: "j***@e***.com"
```

This preserves enough information for debugging (first letter and domain) while protecting user identity.

## Middleware Ordering

Middleware order matters! The platform applies middleware in this sequence:

```rust
let app = Router::new()
    .route("/api/activities", get(get_activities))
    // 1. CORS (must be outermost for OPTIONS preflight)
    .layer(setup_cors(&config))
    // 2. Request ID (early for correlation)
    .layer(middleware::from_fn(request_id_middleware))
    // 3. Tracing (after request ID, before auth)
    .layer(TraceLayer::new_for_http())
    // 4. Authentication (extract user_id)
    .layer(Extension(Arc::new(auth_middleware)))
    // 5. Tenant isolation (requires user_id)
    .layer(Extension(Arc::new(tenant_isolation)))
    // 6. Rate limiting (requires auth context)
    .layer(Extension(Arc::new(rate_limiter)));
```

**Ordering rules**:
1. **CORS first**: Must handle OPTIONS preflight before other middleware
2. **Request ID early**: Needed for all subsequent logs
3. **Tracing after ID**: Span can include request ID immediately
4. **Auth before tenant**: Need user_id to look up tenant
5. **Tenant before rate limit**: Rate limits may be per-tenant
6. **Handlers last**: Process after all middleware

**Rust Idiom**: Tower layers are applied bottom-to-top

Axum uses Tower's `Layer` trait, which applies middleware in reverse order. The outermost `.layer()` call wraps the innermost. Visualize as:
```
CORS(RequestID(Tracing(Auth(Handler))))
```

## Security Headers

The platform adds security headers to all responses:
- `X-Request-ID`: Request correlation ID
- `X-Content-Type-Options: nosniff`: Prevent MIME sniffing
- `X-Frame-Options: DENY`: Prevent clickjacking
- `Strict-Transport-Security`: Force HTTPS (production only)
- `Content-Security-Policy`: Restrict resource loading

**Example**: Adding security headers in middleware:
```rust
pub async fn security_headers_middleware(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );

    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );

    if is_production() {
        headers.insert(
            HeaderName::from_static("strict-transport-security"),
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }

    response
}
```

## Key Takeaways

1. **Middleware stack**: Layered architecture processes requests through CORS, request ID, tracing, auth, tenant isolation, and rate limiting before reaching handlers.

2. **Request ID**: Every request gets a UUID v4 for distributed tracing. Included in response headers and all log entries.

3. **Request context**: `RequestContext` flows through the request lifecycle, accumulating user_id, tenant_id, and auth_method for structured logging.

4. **CORS configuration**: Environment-driven origin allowlist supports development (`*`) and production (specific domains). Custom headers for provider OAuth and multi-tenancy.

5. **Rate limit headers**: Standard `X-RateLimit-*` headers inform clients about usage limits. `Retry-After` tells clients when to retry 429 responses.

6. **PII redaction**: Configurable redaction of authorization headers, email addresses, and sensitive JSON fields protects user privacy in logs.

7. **Middleware ordering**: CORS → Request ID → Tracing → Auth → Tenant → Rate Limit → Handler. Order matters for dependencies.

8. **Span creation**: Helper functions (`create_request_span`, `create_mcp_span`, `create_database_span`) provide consistent tracing across the platform.

9. **Type-safe extensions**: Axum's extension system allows storing typed data (RequestId, RequestContext) in requests for handler access.

10. **Security headers**: Platform adds `X-Content-Type-Options`, `X-Frame-Options`, and `Strict-Transport-Security` to prevent common web vulnerabilities.

---

**End of Part II: Authentication & Security**

You've completed the authentication and security section of the Pierre platform tutorial. You now understand:
- Error handling with structured errors (Chapter 2)
- Configuration management (Chapter 3)
- Dependency injection with Arc (Chapter 4)
- Cryptographic key management (Chapter 5)
- JWT authentication with RS256 (Chapter 6)
- Multi-tenant database isolation (Chapter 7)
- Middleware and request context (Chapter 8)

**Next Chapter**: [Chapter 09: JSON-RPC 2.0 Foundation](./chapter-09-jsonrpc-foundation.md) - Begin Part III by learning how the Model Context Protocol (MCP) builds on JSON-RPC 2.0 for structured client-server communication.
