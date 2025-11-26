// ABOUTME: OAuth 2.0 server route handlers for RFC-compliant authorization server endpoints
// ABOUTME: Provides OAuth 2.0 protocol endpoints including client registration, authorization, and token exchange
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
//
// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - OAuth client field ownership transfers for registration and token requests
// - Resource Arc sharing for HTTP route handlers
// - String ownership for OAuth protocol responses

use crate::admin::jwks::JwksManager;
use crate::auth::AuthManager;
use crate::config::environment::ServerConfig;
use crate::database_plugins::factory::Database;
use crate::database_plugins::DatabaseProvider;
use crate::errors::AppError;
use crate::errors::AppResult;
use crate::oauth2_server::{
    client_registration::ClientRegistrationManager,
    endpoints::OAuth2AuthorizationServer,
    models::{AuthorizeRequest, ClientRegistrationRequest, OAuth2Error, TokenRequest},
    rate_limiting::OAuth2RateLimiter,
};
use axum::{
    extract::{ConnectInfo, Form, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, Router,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

/// OAuth 2.0 server context shared across all handlers
#[derive(Clone)]
pub struct OAuth2Context {
    /// Database for client and token storage
    pub database: Arc<Database>,
    /// Authentication manager for JWT operations
    pub auth_manager: Arc<AuthManager>,
    /// JWKS manager for public key operations
    pub jwks_manager: Arc<JwksManager>,
    /// Server configuration
    pub config: Arc<ServerConfig>,
    /// Rate limiter for OAuth endpoints
    pub rate_limiter: Arc<OAuth2RateLimiter>,
}

/// OAuth 2.0 routes implementation
pub struct OAuth2Routes;

/// Parameters for generating OAuth login HTML
#[derive(Clone, Copy)]
pub struct LoginHtmlParams<'a> {
    /// OAuth client identifier
    pub client_id: &'a str,
    /// OAuth redirect URI after authorization
    pub redirect_uri: &'a str,
    /// OAuth response type (typically "code")
    pub response_type: &'a str,
    /// OAuth state parameter for CSRF protection
    pub state: &'a str,
    /// OAuth scope for requested permissions
    pub scope: &'a str,
    /// PKCE code challenge
    pub code_challenge: &'a str,
    /// PKCE code challenge method (e.g., "S256")
    pub code_challenge_method: &'a str,
    /// Default email to pre-fill in login form (dev/test only)
    pub default_email: &'a str,
    /// Default password to pre-fill in login form (dev/test only)
    pub default_password: &'a str,
}

impl OAuth2Routes {
    /// Create all OAuth 2.0 routes with context
    pub fn routes(context: OAuth2Context) -> Router {
        Router::new()
            // RFC 8414: OAuth 2.0 Authorization Server Metadata
            .route(
                "/.well-known/oauth-authorization-server",
                get(Self::handle_discovery),
            )
            // RFC 7517: JWKS endpoint
            .route("/.well-known/jwks.json", get(Self::handle_jwks))
            // RFC 7591: Dynamic Client Registration
            .route("/oauth2/register", post(Self::handle_client_registration))
            // OAuth 2.0 Authorization endpoint
            .route("/oauth2/authorize", get(Self::handle_authorization))
            // OAuth 2.0 Token endpoint
            .route("/oauth2/token", post(Self::handle_token))
            // Login page and submission
            .route("/oauth2/login", get(Self::handle_oauth_login_page))
            .route("/oauth2/login", post(Self::handle_oauth_login_submit))
            // Token validation endpoints
            .route(
                "/oauth2/validate-and-refresh",
                post(Self::handle_validate_and_refresh),
            )
            .route("/oauth2/token-validate", post(Self::handle_token_validate))
            // JWKS also available at /oauth2/jwks
            .route("/oauth2/jwks", get(Self::handle_jwks))
            .with_state(context)
    }

    /// Handle OAuth 2.0 discovery (RFC 8414)
    async fn handle_discovery(State(context): State<OAuth2Context>) -> Json<serde_json::Value> {
        let issuer_url = context.config.oauth2_server.issuer_url.clone();

        // Use spawn_blocking for JSON serialization (CPU-bound operation)
        let discovery_json = tokio::task::spawn_blocking(move || {
            serde_json::json!({
                "issuer": issuer_url,
                "authorization_endpoint": format!("{issuer_url}/oauth2/authorize"),
                "token_endpoint": format!("{issuer_url}/oauth2/token"),
                "registration_endpoint": format!("{issuer_url}/oauth2/register"),
                "jwks_uri": format!("{issuer_url}/.well-known/jwks.json"),
                "grant_types_supported": ["authorization_code", "client_credentials", "refresh_token"],
                "response_types_supported": ["code"],
                "token_endpoint_auth_methods_supported": ["client_secret_post", "client_secret_basic"],
                "scopes_supported": ["fitness:read", "activities:read", "profile:read"],
                "response_modes_supported": ["query"],
                "code_challenge_methods_supported": ["S256"]
            })
        })
        .await
        .unwrap_or_else(|_| {
            serde_json::json!({
                "error": "internal_error",
                "error_description": "Failed to generate discovery document"
            })
        });

        Json(discovery_json)
    }

    /// Handle client registration (RFC 7591)
    async fn handle_client_registration(
        State(context): State<OAuth2Context>,
        ConnectInfo(addr): ConnectInfo<SocketAddr>,
        Json(request): Json<ClientRegistrationRequest>,
    ) -> Response {
        // Extract client IP from connection using Axum's ConnectInfo extractor
        let client_ip = addr.ip();
        let rate_status = context.rate_limiter.check_rate_limit("register", client_ip);

        if rate_status.is_limited {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                Json(serde_json::json!({
                    "error": "too_many_requests",
                    "error_description": "Rate limit exceeded"
                })),
            )
                .into_response();
        }

        let client_manager = ClientRegistrationManager::new(context.database);

        match client_manager.register_client(request).await {
            Ok(response) => (StatusCode::CREATED, Json(response)).into_response(),
            Err(error) => (StatusCode::BAD_REQUEST, Json(error)).into_response(),
        }
    }

    /// Handle authorization request (GET /oauth2/authorize)
    async fn handle_authorization(
        State(context): State<OAuth2Context>,
        ConnectInfo(addr): ConnectInfo<SocketAddr>,
        Query(params): Query<HashMap<String, String>>,
        headers: HeaderMap,
    ) -> Response {
        // Extract client IP from connection using Axum's ConnectInfo extractor
        let client_ip = addr.ip();
        let rate_status = context
            .rate_limiter
            .check_rate_limit("authorize", client_ip);

        if rate_status.is_limited {
            return Self::render_oauth_error_response(&OAuth2Error {
                error: "too_many_requests".to_owned(),
                error_description: Some("Rate limit exceeded".to_owned()),
                error_uri: None,
            });
        }

        // Parse query parameters into AuthorizeRequest
        let request = match Self::parse_authorize_request(&params) {
            Ok(req) => req,
            Err(error) => return Self::render_oauth_error_response(&error),
        };

        // Check if user is authenticated via session cookie
        let (user_id, tenant_id) = headers
            .get(header::COOKIE)
            .and_then(|cookie_value| {
                cookie_value.to_str().ok().and_then(|cookie_str| {
                    Self::extract_session_token(cookie_str).and_then(|token| {
                        match context
                            .auth_manager
                            .validate_token(&token, &context.jwks_manager)
                        {
                            Ok(claims) => {
                                tracing::info!(
                                    "OAuth authorization for authenticated user: {}",
                                    claims.email
                                );
                                // Parse user ID from JWT claims
                                if let Ok(user_uuid) = uuid::Uuid::parse_str(&claims.sub) {
                                    Some((user_uuid, claims.tenant_id))
                                } else {
                                    tracing::warn!("Invalid user ID format in JWT: {}", claims.sub);
                                    None
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Invalid session token in OAuth authorization: {}",
                                    e
                                );
                                None
                            }
                        }
                    })
                })
            })
            .map_or((None, None), |(uid, tid)| (Some(uid), tid));

        // If no authenticated user, redirect to login page with OAuth parameters
        let Some(authenticated_user_id) = user_id else {
            tracing::info!(
                "No authenticated session for OAuth authorization, redirecting to login"
            );
            let login_url = Self::build_login_url_with_oauth_params(&request);
            return Redirect::to(&login_url).into_response();
        };

        // User is authenticated - proceed with OAuth authorization
        let auth_server = OAuth2AuthorizationServer::new(
            context.database,
            context.auth_manager,
            context.jwks_manager,
        );
        let redirect_uri = request.redirect_uri.clone(); // Safe: OAuth redirect URI needed for response

        match auth_server
            .authorize(request, Some(authenticated_user_id), tenant_id)
            .await
        {
            Ok(response) => {
                // OAuth 2.0 specification requires redirecting to redirect_uri with code
                // Build redirect URL with authorization code and state
                let mut final_redirect_url = format!("{}?code={}", redirect_uri, response.code);
                if let Some(state) = response.state {
                    use std::fmt::Write;
                    write!(&mut final_redirect_url, "&state={state}").ok();
                }

                tracing::info!(
                    "OAuth authorization successful for user {}, redirecting with code",
                    authenticated_user_id
                );

                Redirect::to(&final_redirect_url).into_response()
            }
            Err(error) => {
                tracing::error!(
                    "OAuth authorization failed for user {}: {:?}",
                    authenticated_user_id,
                    error
                );
                Self::render_oauth_error_response(&error)
            }
        }
    }

    /// Handle token request (POST /oauth2/token)
    async fn handle_token(
        State(context): State<OAuth2Context>,
        ConnectInfo(addr): ConnectInfo<SocketAddr>,
        Form(form): Form<HashMap<String, String>>,
    ) -> Response {
        // Extract client IP from connection using Axum's ConnectInfo extractor
        let client_ip = addr.ip();
        let rate_status = context.rate_limiter.check_rate_limit("token", client_ip);

        if rate_status.is_limited {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                Json(serde_json::json!({
                    "error": "too_many_requests",
                    "error_description": "Rate limit exceeded"
                })),
            )
                .into_response();
        }

        // Debug: Log the incoming form data (excluding sensitive info)
        tracing::debug!(
            "OAuth token request received with grant_type: {:?}, client_id: {:?}",
            form.get("grant_type"),
            form.get("client_id")
        );

        // Parse form data into TokenRequest
        let request = match Self::parse_token_request(&form) {
            Ok(req) => req,
            Err(error) => {
                tracing::warn!("OAuth token request parsing failed: {:?}", error);
                return (StatusCode::BAD_REQUEST, Json(error)).into_response();
            }
        };

        let auth_server = OAuth2AuthorizationServer::new(
            context.database,
            context.auth_manager,
            context.jwks_manager,
        );

        match auth_server.token(request).await {
            Ok(response) => {
                tracing::info!(
                    "OAuth token exchange successful for client: {}",
                    form.get("client_id").map_or("unknown", |v| v)
                );
                (StatusCode::OK, Json(response)).into_response()
            }
            Err(error) => {
                tracing::warn!(
                    "OAuth token exchange failed for client {}: {:?}",
                    form.get("client_id").map_or("unknown", |v| v),
                    error
                );
                (StatusCode::BAD_REQUEST, Json(error)).into_response()
            }
        }
    }

    /// Handle validate and refresh request (POST /oauth2/validate-and-refresh)
    async fn handle_validate_and_refresh(
        State(context): State<OAuth2Context>,
        headers: HeaderMap,
        Json(request): Json<crate::oauth2_server::models::ValidateRefreshRequest>,
    ) -> Response {
        // Extract Bearer token from Authorization header
        let access_token = match Self::extract_bearer_token(&headers) {
            Ok(token) => token,
            Err(response) => return response,
        };

        tracing::debug!(
            "Validate-and-refresh request received with token (first 20 chars): {}...",
            &access_token[..std::cmp::min(20, access_token.len())]
        );

        let auth_server = OAuth2AuthorizationServer::new(
            context.database,
            context.auth_manager,
            context.jwks_manager,
        );

        match auth_server
            .validate_and_refresh(&access_token, request)
            .await
        {
            Ok(response) => {
                tracing::info!(
                    "Token validation completed with status: {:?}",
                    response.status
                );
                (StatusCode::OK, Json(response)).into_response()
            }
            Err(error) => {
                tracing::error!("Validate-and-refresh failed: {}", error);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "internal_error",
                        "error_description": "Failed to validate token"
                    })),
                )
                    .into_response()
            }
        }
    }

    /// Handle token validation request (POST /oauth2/token-validate)
    async fn handle_token_validate(
        State(context): State<OAuth2Context>,
        headers: HeaderMap,
        Json(request): Json<serde_json::Value>,
    ) -> Response {
        tracing::debug!("Token validation request received");

        // Extract client_id from request body (optional)
        let client_id = request.get("client_id").and_then(|v| v.as_str());

        // Validate access token if provided
        let token_valid = if let Some(header) = headers.get(header::AUTHORIZATION) {
            if let Ok(header_str) = header.to_str() {
                if let Some(token) = header_str.strip_prefix("Bearer ") {
                    // Validate JWT token using RS256
                    match context
                        .auth_manager
                        .validate_token(token, &context.jwks_manager)
                    {
                        Ok(_) => true,
                        Err(e) => {
                            tracing::debug!("Token validation failed: {}", e);
                            return (
                                StatusCode::OK,
                                Json(serde_json::json!({
                                    "valid": false,
                                    "error": "invalid_token",
                                    "error_description": "Access token is invalid or expired"
                                })),
                            )
                                .into_response();
                        }
                    }
                } else {
                    return (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "valid": false,
                            "error": "invalid_request",
                            "error_description": "Authorization header must use Bearer scheme"
                        })),
                    )
                        .into_response();
                }
            } else {
                return (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "valid": false,
                        "error": "invalid_request",
                        "error_description": "Invalid Authorization header encoding"
                    })),
                )
                    .into_response();
            }
        } else {
            // No token provided - only validate client_id
            false
        };

        // Validate client_id if provided
        if let Some(cid) = client_id {
            let client_manager = ClientRegistrationManager::new(context.database);
            match client_manager.get_client(cid).await {
                Ok(_) => {
                    // Both token (if provided) and client_id are valid
                    tracing::info!("Credentials validated successfully for client_id: {}", cid);
                    (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "valid": true
                        })),
                    )
                        .into_response()
                }
                Err(e) => {
                    tracing::debug!("Client validation failed for {}: {}", cid, e);
                    (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "valid": false,
                            "error": "invalid_client",
                            "error_description": "Client ID not found or invalid"
                        })),
                    )
                        .into_response()
                }
            }
        } else if token_valid {
            // Only token was provided and it's valid
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "valid": true
                })),
            )
                .into_response()
        } else {
            // Neither token nor client_id provided
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "valid": false,
                    "error": "invalid_request",
                    "error_description": "Either access token or client_id must be provided"
                })),
            )
                .into_response()
        }
    }

    /// Generate OAuth login page HTML
    #[must_use]
    pub fn generate_login_html(params: LoginHtmlParams<'_>) -> String {
        // Use embedded template - zero filesystem IO, guaranteed to exist at compile-time
        let displayed_scope = if params.scope.is_empty() {
            "fitness:read activities:read profile:read"
        } else {
            params.scope
        };

        Self::OAUTH_LOGIN_TEMPLATE
            .replace("{{CLIENT_ID}}", params.client_id)
            .replace("{{REDIRECT_URI}}", params.redirect_uri)
            .replace("{{RESPONSE_TYPE}}", params.response_type)
            .replace("{{STATE}}", params.state)
            .replace("{{SCOPE}}", displayed_scope)
            .replace("{{CODE_CHALLENGE}}", params.code_challenge)
            .replace("{{CODE_CHALLENGE_METHOD}}", params.code_challenge_method)
            .replace("{{DEFAULT_EMAIL}}", params.default_email)
            .replace("{{DEFAULT_PASSWORD}}", params.default_password)
    }

    /// Handle OAuth login page (GET /oauth2/login)
    async fn handle_oauth_login_page(
        State(context): State<OAuth2Context>,
        Query(params): Query<HashMap<String, String>>,
    ) -> Html<String> {
        // Extract OAuth parameters to preserve them through login flow (including PKCE)
        let client_id = params
            .get("client_id")
            .map_or_else(String::new, ToString::to_string);
        let redirect_uri = params
            .get("redirect_uri")
            .map_or_else(String::new, ToString::to_string);
        let response_type = params
            .get("response_type")
            .map_or_else(String::new, ToString::to_string);
        let state = params
            .get("state")
            .map_or_else(String::new, ToString::to_string);
        let scope = params
            .get("scope")
            .map_or_else(String::new, ToString::to_string);
        let code_challenge = params
            .get("code_challenge")
            .map_or_else(String::new, ToString::to_string);
        let code_challenge_method = params
            .get("code_challenge_method")
            .map_or_else(String::new, ToString::to_string);

        // Get default form values from ServerConfig (for dev/test only)
        // Safe: Option<String> ownership for HTML template
        let default_email = context
            .config
            .oauth2_server
            .default_login_email
            .clone()
            .unwrap_or_default();
        let default_password = context
            .config
            .oauth2_server
            .default_login_password
            .clone()
            .unwrap_or_default();

        // Use spawn_blocking for HTML generation (CPU-bound string formatting)
        let html = tokio::task::spawn_blocking(move || {
            Self::generate_login_html(LoginHtmlParams {
                client_id: &client_id,
                redirect_uri: &redirect_uri,
                response_type: &response_type,
                state: &state,
                scope: &scope,
                code_challenge: &code_challenge,
                code_challenge_method: &code_challenge_method,
                default_email: &default_email,
                default_password: &default_password,
            })
        })
        .await
        .unwrap_or_else(|_| {
            "<html><body><h1>Error</h1><p>Failed to generate login page</p></body></html>"
                .to_owned()
        });

        Html(html)
    }

    /// Handle OAuth login form submission (POST /oauth2/login)
    async fn handle_oauth_login_submit(
        State(context): State<OAuth2Context>,
        Form(form): Form<HashMap<String, String>>,
    ) -> Response {
        // Extract credentials from form
        let Some(email) = form.get("email") else {
            return (StatusCode::BAD_REQUEST, "Missing email").into_response();
        };

        let Some(password) = form.get("password") else {
            return (StatusCode::BAD_REQUEST, "Missing password").into_response();
        };

        // Authenticate user using database lookup and password verification
        match Self::authenticate_user_with_auth_manager(
            context.database.clone(),
            email,
            password,
            &context.auth_manager,
            &context.jwks_manager,
        )
        .await
        {
            Ok(token) => {
                // Extract OAuth parameters from form to continue authorization flow (including PKCE)
                let client_id = form.get("client_id").map_or("", |v| v);
                let redirect_uri = form.get("redirect_uri").map_or("", |v| v);
                let response_type = form.get("response_type").map_or("", |v| v);
                let state = form.get("state").map_or("", |v| v);
                let scope = form.get("scope").map_or("", |v| v);
                let code_challenge = form.get("code_challenge").map_or("", |v| v);
                let code_challenge_method = form.get("code_challenge_method").map_or("", |v| v);

                let auth_url = Self::build_authorization_url_from_form(
                    client_id,
                    redirect_uri,
                    response_type,
                    state,
                    scope,
                    code_challenge,
                    code_challenge_method,
                );

                tracing::info!(
                    "User {} authenticated successfully for OAuth, redirecting to authorization",
                    email
                );

                // Set session cookie and redirect to authorization endpoint
                // Cookie security: HttpOnly prevents XSS, Secure enforces HTTPS, SameSite=Lax prevents CSRF
                // Max-Age matches JWT expiration (24 hours = 86400 seconds)
                let cookie_header = format!(
                    "pierre_session={token}; HttpOnly; Secure; Path=/; SameSite=Lax; Max-Age=86400"
                );

                (
                    StatusCode::FOUND,
                    [
                        (header::LOCATION, auth_url),
                        (header::SET_COOKIE, cookie_header),
                    ],
                )
                    .into_response()
            }
            Err(e) => {
                tracing::warn!("Authentication failed for OAuth login: {}", e);

                // Use embedded template - zero filesystem IO, guaranteed to exist at compile-time
                let error_html = Self::OAUTH_LOGIN_ERROR_TEMPLATE
                    .replace(
                        "{{ERROR_MESSAGE}}",
                        "Authentication Failed: Invalid email or password. Please try again.",
                    )
                    .replace("{{CLIENT_ID}}", form.get("client_id").map_or("", |v| v))
                    .replace(
                        "{{REDIRECT_URI}}",
                        urlencoding::encode(form.get("redirect_uri").map_or("", |v| v)).as_ref(),
                    )
                    .replace(
                        "{{RESPONSE_TYPE}}",
                        form.get("response_type").map_or("", |v| v),
                    )
                    .replace("{{STATE}}", form.get("state").map_or("", |v| v))
                    .replace(
                        "{{SCOPE}}",
                        urlencoding::encode(form.get("scope").map_or("", |v| v)).as_ref(),
                    )
                    .replace(
                        "{{CODE_CHALLENGE}}",
                        urlencoding::encode(form.get("code_challenge").map_or("", |v| v)).as_ref(),
                    )
                    .replace(
                        "{{CODE_CHALLENGE_METHOD}}",
                        form.get("code_challenge_method").map_or("", |v| v),
                    );

                (StatusCode::UNAUTHORIZED, Html(error_html)).into_response()
            }
        }
    }

    /// Handle JWKS endpoint (GET /oauth2/jwks or GET /.well-known/jwks.json)
    async fn handle_jwks(State(context): State<OAuth2Context>, headers: HeaderMap) -> Response {
        // Return JWKS with RS256 public keys for token validation
        let jwks = match context.jwks_manager.get_jwks() {
            Ok(jwks) => jwks,
            Err(e) => {
                tracing::error!("Failed to generate JWKS: {}", e);
                // Return empty JWKS on error (graceful degradation)
                return (
                    StatusCode::OK,
                    [(header::CACHE_CONTROL, "public, max-age=3600")],
                    Json(serde_json::json!({ "keys": [] })),
                )
                    .into_response();
            }
        };

        tracing::debug!("JWKS endpoint accessed, returning {} keys", jwks.keys.len());

        // Calculate ETag from JWKS content for efficient caching
        let (_jwks_json, etag) = match Self::compute_jwks_etag(jwks.clone()).await {
            Ok(result) => result,
            Err(response) => return response,
        };

        // Check if client's cached version matches current version
        if Self::check_etag_match(&headers, &etag) {
            tracing::debug!("JWKS ETag match, returning 304 Not Modified");
            return (StatusCode::NOT_MODIFIED, [(header::ETAG, etag)]).into_response();
        }

        // Return JWKS with ETag and Cache-Control headers
        (
            StatusCode::OK,
            [
                (header::CACHE_CONTROL, "public, max-age=3600".to_owned()),
                (header::ETAG, etag),
            ],
            Json(jwks),
        )
            .into_response()
    }

    // ============================================================================
    // Helper Functions
    // ============================================================================

    /// Compute JWKS ETag from JSON content
    async fn compute_jwks_etag(
        jwks: crate::admin::jwks::JsonWebKeySet,
    ) -> Result<(String, String), Response> {
        let etag_result = tokio::task::spawn_blocking(move || {
            let jwks_json = serde_json::to_string(&jwks)?;
            let mut hasher = Sha256::new();
            hasher.update(jwks_json.as_bytes());
            let hash = hasher.finalize();
            let etag = format!(r#""{}""#, hex::encode(&hash[..16]));
            Ok::<(String, String), serde_json::Error>((jwks_json, etag))
        })
        .await;

        match etag_result {
            Ok(Ok((json, tag))) => Ok((json, tag)),
            Ok(Err(_)) => {
                tracing::error!("Failed to serialize JWKS for ETag calculation");
                Err(Self::jwks_error_response())
            }
            Err(_) => {
                tracing::error!("Spawn blocking task panicked during JWKS serialization");
                Err(Self::jwks_error_response())
            }
        }
    }

    /// Create error response for JWKS endpoint
    fn jwks_error_response() -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "keys": []
            })),
        )
            .into_response()
    }

    /// Check if client has current JWKS version (ETag match)
    fn check_etag_match(headers: &HeaderMap, etag: &str) -> bool {
        headers
            .get(header::IF_NONE_MATCH)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|client_etag| client_etag == etag)
    }

    /// Extract Bearer token from Authorization header
    fn extract_bearer_token(headers: &HeaderMap) -> Result<String, Response> {
        let header = headers.get(header::AUTHORIZATION).ok_or_else(|| {
            tracing::warn!("Missing Authorization header");
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "invalid_request",
                    "error_description": "Authorization header is required"
                })),
            )
                .into_response()
        })?;

        let header_str = header.to_str().map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "invalid_request",
                    "error_description": "Invalid Authorization header encoding"
                })),
            )
                .into_response()
        })?;

        header_str
            .strip_prefix("Bearer ")
            .map(str::to_owned)
            .ok_or_else(|| {
                tracing::warn!("Invalid Authorization header format - missing Bearer prefix");
                (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({
                        "error": "invalid_request",
                        "error_description": "Authorization header must use Bearer scheme"
                    })),
                )
                    .into_response()
            })
    }

    /// Build login URL with OAuth parameters preserved for redirect
    fn build_login_url_with_oauth_params(request: &AuthorizeRequest) -> String {
        let mut login_url = format!(
            "/oauth2/login?client_id={}&redirect_uri={}&response_type={}&state={}",
            request.client_id,
            urlencoding::encode(&request.redirect_uri),
            request.response_type,
            request.state.as_deref().unwrap_or("")
        );

        if let Some(ref scope) = request.scope {
            use std::fmt::Write;
            write!(&mut login_url, "&scope={}", urlencoding::encode(scope)).ok();
        }

        if let Some(ref code_challenge) = request.code_challenge {
            use std::fmt::Write;
            write!(
                &mut login_url,
                "&code_challenge={}",
                urlencoding::encode(code_challenge)
            )
            .ok();
        }

        if let Some(ref code_challenge_method) = request.code_challenge_method {
            use std::fmt::Write;
            write!(
                &mut login_url,
                "&code_challenge_method={code_challenge_method}"
            )
            .ok();
        }

        login_url
    }

    /// Build authorization URL from form data with OAuth parameters preserved for redirect
    fn build_authorization_url_from_form(
        client_id: &str,
        redirect_uri: &str,
        response_type: &str,
        state: &str,
        scope: &str,
        code_challenge: &str,
        code_challenge_method: &str,
    ) -> String {
        let mut auth_url = format!(
            "/oauth2/authorize?client_id={}&redirect_uri={}&response_type={}&state={}",
            client_id,
            urlencoding::encode(redirect_uri),
            response_type,
            state
        );

        if !scope.is_empty() {
            use std::fmt::Write;
            write!(&mut auth_url, "&scope={}", urlencoding::encode(scope)).ok();
        }

        if !code_challenge.is_empty() {
            use std::fmt::Write;
            write!(
                &mut auth_url,
                "&code_challenge={}",
                urlencoding::encode(code_challenge)
            )
            .ok();
        }

        if !code_challenge_method.is_empty() {
            use std::fmt::Write;
            write!(
                &mut auth_url,
                "&code_challenge_method={code_challenge_method}"
            )
            .ok();
        }

        auth_url
    }

    /// Parse query parameters into `AuthorizeRequest`
    fn parse_authorize_request(
        params: &HashMap<String, String>,
    ) -> Result<AuthorizeRequest, OAuth2Error> {
        tracing::trace!(
            "Parsing OAuth authorize request with {} parameters",
            params.len()
        );

        let response_type = params
            .get("response_type")
            .ok_or_else(|| OAuth2Error::invalid_request("Missing response_type parameter"))?
            .clone(); // Safe: String ownership required for OAuth2 request struct

        let client_id = params
            .get("client_id")
            .ok_or_else(|| OAuth2Error::invalid_request("Missing client_id parameter"))?
            .clone(); // Safe: String ownership required for OAuth2 request struct

        let redirect_uri = params
            .get("redirect_uri")
            .ok_or_else(|| OAuth2Error::invalid_request("Missing redirect_uri parameter"))?
            .clone(); // Safe: String ownership required for OAuth2 request struct

        let scope = params.get("scope").cloned();
        let state = params.get("state").cloned();
        let code_challenge = params.get("code_challenge").cloned();
        let code_challenge_method = params.get("code_challenge_method").cloned();

        Ok(AuthorizeRequest {
            response_type,
            client_id,
            redirect_uri,
            scope,
            state,
            code_challenge,
            code_challenge_method,
        })
    }

    /// Parse form data into `TokenRequest`
    fn parse_token_request(form: &HashMap<String, String>) -> Result<TokenRequest, OAuth2Error> {
        let grant_type = form
            .get("grant_type")
            .ok_or_else(|| OAuth2Error::invalid_request("Missing grant_type parameter"))?
            .clone(); // Safe: String ownership required for OAuth2 request struct

        // For refresh_token grants, client credentials are optional (RFC 6749 recommends but doesn't require)
        // The refresh_token itself authenticates the request
        let is_refresh = grant_type == "refresh_token";

        let client_id = if is_refresh {
            form.get("client_id").cloned().unwrap_or_default()
        } else {
            form.get("client_id")
                .ok_or_else(|| OAuth2Error::invalid_request("Missing client_id parameter"))?
                .clone() // Safe: String ownership for OAuth validation
        };

        let client_secret = if is_refresh {
            form.get("client_secret")
                .cloned()
                .unwrap_or_default()
                .replace(' ', "+")
        } else {
            form.get("client_secret")
                .ok_or_else(|| OAuth2Error::invalid_request("Missing client_secret parameter"))?
                .replace(' ', "+")
        };

        let code = form.get("code").cloned();
        let redirect_uri = form.get("redirect_uri").cloned();
        let scope = form.get("scope").cloned();
        let refresh_token = form.get("refresh_token").cloned();
        let code_verifier = form.get("code_verifier").cloned();

        Ok(TokenRequest {
            grant_type,
            code,
            redirect_uri,
            client_id,
            client_secret,
            scope,
            refresh_token,
            code_verifier,
        })
    }

    /// Authenticate user credentials using `AuthManager` (proper architecture)
    async fn authenticate_user_with_auth_manager(
        database: Arc<Database>,
        email: &str,
        password: &str,
        auth_manager: &AuthManager,
        jwks_manager: &JwksManager,
    ) -> AppResult<String> {
        // Look up user by email
        let user = database
            .get_user_by_email(email)
            .await
            .map_err(|e| AppError::database(e.to_string()))?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        // Verify password hash
        if !Self::verify_password(password, &user.password_hash).await {
            return Err(AppError::auth_invalid("Invalid password"));
        }

        // Use AuthManager to generate JWT token with RS256 (proper architecture)
        // This ensures consistent JWT handling across the entire system
        let token = auth_manager
            .generate_token(&user, jwks_manager)
            .map_err(|e| AppError::internal(format!("Token generation failed: {e}")))?;

        Ok(token)
    }

    /// Verify password against hash using bcrypt with `spawn_blocking`
    ///
    /// Uses `tokio::task::spawn_blocking` to avoid blocking the async executor
    /// with CPU-intensive bcrypt operations.
    async fn verify_password(password: &str, hash: &str) -> bool {
        let password = password.to_owned();
        let hash = hash.to_owned();

        tokio::task::spawn_blocking(move || bcrypt::verify(&password, &hash).unwrap_or(false))
            .await
            .unwrap_or(false)
    }

    /// Extract session token from cookie header
    fn extract_session_token(cookie_header: &str) -> Option<String> {
        // Parse cookies and look for pierre_session
        for cookie in cookie_header.split(';') {
            let cookie = cookie.trim();
            if let Some(session_token) = cookie.strip_prefix("pierre_session=") {
                return Some(session_token.to_owned());
            }
        }
        None
    }

    /// OAuth error template embedded at compile-time
    const OAUTH_ERROR_TEMPLATE: &'static str = include_str!("../../templates/oauth_error.html");

    /// OAuth login page template embedded at compile-time
    /// Loaded with `include_str`!() to avoid blocking filesystem IO at runtime
    const OAUTH_LOGIN_TEMPLATE: &'static str = include_str!("../../templates/oauth_login.html");

    /// OAuth login error template embedded at compile-time
    /// Loaded with `include_str`!() to avoid blocking filesystem IO at runtime
    const OAUTH_LOGIN_ERROR_TEMPLATE: &'static str =
        include_str!("../../templates/oauth_login_error.html");

    /// Render HTML error page for OAuth errors shown in browser
    fn render_oauth_error_response(error: &OAuth2Error) -> Response {
        let error_title = match error.error.as_str() {
            "invalid_client" => "✗ Invalid Client",
            "unauthorized_client" => "✗ Unauthorized Client",
            "access_denied" => "✗ Access Denied",
            "unsupported_response_type" => "✗ Unsupported Response Type",
            "invalid_scope" => "✗ Invalid Scope",
            "server_error" => "✗ Server Error",
            "temporarily_unavailable" => "✗ Temporarily Unavailable",
            _ => "✗ OAuth Error",
        };

        let default_description =
            "An error occurred during the OAuth authorization process.".to_owned();
        let error_description = error
            .error_description
            .as_ref()
            .unwrap_or(&default_description);

        let html = Self::OAUTH_ERROR_TEMPLATE
            .replace("{{error_title}}", error_title)
            .replace("{{ERROR}}", &error.error)
            .replace("{{PROVIDER}}", "Pierre MCP Server")
            .replace(
                "{{DESCRIPTION}}",
                &format!(r#"<div class="description">{error_description}</div>"#),
            );

        (StatusCode::BAD_REQUEST, Html(html)).into_response()
    }
}
