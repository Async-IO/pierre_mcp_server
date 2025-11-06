// ABOUTME: OAuth 2.0 HTTP route handlers for warp web framework
// ABOUTME: Provides REST endpoints for client registration, authorization, and token exchange
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
//
// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - OAuth client field ownership transfers for registration and token requests
// - Resource Arc sharing for HTTP route handlers
// - String ownership for OAuth protocol responses

use super::{
    client_registration::ClientRegistrationManager,
    endpoints::OAuth2AuthorizationServer,
    models::{AuthorizeRequest, ClientRegistrationRequest, OAuth2Error, TokenRequest},
};
use crate::admin::jwks::JwksManager;
use crate::auth::AuthManager;
use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::errors::AppError;
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use warp::{Filter, Rejection, Reply};

/// OAuth 2.0 route filters
pub fn oauth2_routes(
    database: Arc<Database>,
    auth_manager: &Arc<AuthManager>,
    jwks_manager: &Arc<JwksManager>,
    config: &Arc<crate::config::environment::ServerConfig>,
    rate_limiter: &Arc<super::rate_limiting::OAuth2RateLimiter>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let client_registration_routes = client_registration_routes(database.clone(), rate_limiter);
    let authorization_routes = authorization_routes(
        database.clone(),
        auth_manager,
        jwks_manager,
        config,
        rate_limiter,
    );
    let token_routes = token_routes(database.clone(), auth_manager, jwks_manager, rate_limiter);
    let validate_refresh_routes =
        validate_and_refresh_routes(database.clone(), auth_manager, jwks_manager);
    let token_validate_routes = token_validate_routes(database, auth_manager, jwks_manager);
    let jwks_route = jwks_route(jwks_manager);

    // OAuth routes under /oauth2 prefix
    let oauth_prefixed_routes = warp::path("oauth2").and(
        client_registration_routes
            .or(authorization_routes)
            .or(token_routes)
            .or(validate_refresh_routes)
            .or(token_validate_routes)
            .or(jwks_route),
    );

    // Discovery route at root level (RFC 8414 compliance)
    let discovery_route = oauth2_discovery_route(&config.oauth2_server.issuer_url);

    // JWKS endpoint at well-known location (OIDC/OAuth2 standard)
    let well_known_jwks_route = warp::path!(".well-known" / "jwks.json")
        .and(warp::get())
        .and(warp::header::optional::<String>("if-none-match"))
        .and(with_jwks_manager(jwks_manager))
        .and_then(handle_jwks);

    // Combine all routes: root-level discovery + well-known JWKS + prefixed OAuth routes
    discovery_route
        .or(well_known_jwks_route)
        .or(oauth_prefixed_routes)
}

/// OAuth 2.0 discovery route (RFC 8414)
fn oauth2_discovery_route(
    issuer_url: &str,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let issuer_url = issuer_url.to_owned(); // Safe: captured for warp filter closure lifetime
    warp::path!(".well-known" / "oauth-authorization-server")
        .and(warp::get())
        .map(move || {
            warp::reply::json(&serde_json::json!({
                "issuer": issuer_url,
                "authorization_endpoint": format!("{}/oauth2/authorize", issuer_url),
                "token_endpoint": format!("{}/oauth2/token", issuer_url),
                "registration_endpoint": format!("{}/oauth2/register", issuer_url),
                "jwks_uri": format!("{}/.well-known/jwks.json", issuer_url),
                "grant_types_supported": ["authorization_code", "client_credentials", "refresh_token"],
                "response_types_supported": ["code"],
                "token_endpoint_auth_methods_supported": ["client_secret_post", "client_secret_basic"],
                "scopes_supported": ["fitness:read", "activities:read", "profile:read"],
                "response_modes_supported": ["query"],
                "code_challenge_methods_supported": ["S256"]
            }))
        })
}

/// Client registration routes (RFC 7591)
fn client_registration_routes(
    database: Arc<Database>,
    rate_limiter: &Arc<super::rate_limiting::OAuth2RateLimiter>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    // Safe: filter() takes &self and clones internally for warp closure
    let rate_limiter_filter = rate_limiter.filter("register");

    warp::path("register")
        .and(warp::path::end())
        .and(warp::post())
        .and(rate_limiter_filter)
        .and(warp::body::json())
        .and(with_database(database))
        .and_then(|_rate_status, request, database| {
            // Rate limit status is checked by filter, we just pass it through
            handle_client_registration(request, database)
        })
}

/// Authorization endpoint routes
fn authorization_routes(
    database: Arc<Database>,
    auth_manager: &Arc<AuthManager>,
    jwks_manager: &Arc<JwksManager>,
    config: &Arc<crate::config::environment::ServerConfig>,
    rate_limiter: &Arc<super::rate_limiting::OAuth2RateLimiter>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    // Safe: filter() takes &self and clones internally for warp closure
    let rate_limiter_filter = rate_limiter.filter("authorize");

    // OAuth authorization endpoint with cookie support and rate limiting
    let authorize_route = warp::path("authorize")
        .and(warp::path::end())
        .and(warp::get())
        .and(rate_limiter_filter)
        .and(warp::query::<HashMap<String, String>>())
        .and(warp::header::optional::<String>("cookie"))
        .and(with_database(database.clone()))
        .and(with_auth_manager(auth_manager))
        .and(with_jwks_manager(jwks_manager))
        .and_then(|_rate_status, params, cookie, db, auth, jwks| {
            // Rate limit status is checked by filter, we just pass it through
            handle_authorization(params, cookie, db, auth, jwks)
        });

    // OAuth login page
    let login_route = warp::path("login")
        .and(warp::path::end())
        .and(warp::get())
        .and(warp::query::<HashMap<String, String>>())
        .and(with_config(config))
        .and_then(handle_oauth_login_page);

    // OAuth login form submission
    let login_submit_route = warp::path("login")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::form::<HashMap<String, String>>())
        .and(with_database(database))
        .and(with_auth_manager(auth_manager))
        .and(with_jwks_manager(jwks_manager))
        .and_then(handle_oauth_login_submit);

    authorize_route.or(login_route).or(login_submit_route)
}

/// Token endpoint routes
fn token_routes(
    database: Arc<Database>,
    auth_manager: &Arc<AuthManager>,
    jwks_manager: &Arc<JwksManager>,
    rate_limiter: &Arc<super::rate_limiting::OAuth2RateLimiter>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    // Safe: filter() takes &self and clones internally for warp closure
    let rate_limiter_filter = rate_limiter.filter("token");

    warp::path("token")
        .and(warp::path::end())
        .and(warp::post())
        .and(rate_limiter_filter)
        .and(warp::body::form())
        .and(with_database(database))
        .and(with_auth_manager(auth_manager))
        .and(with_jwks_manager(jwks_manager))
        .and_then(|_rate_status, form, db, auth, jwks| {
            // Rate limit status is checked by filter, we just pass it through
            handle_token(form, db, auth, jwks)
        })
}

/// Validate and refresh endpoint routes
fn validate_and_refresh_routes(
    database: Arc<Database>,
    auth_manager: &Arc<AuthManager>,
    jwks_manager: &Arc<JwksManager>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("validate-and-refresh")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::header::optional::<String>("authorization"))
        .and(warp::body::json())
        .and(with_database(database))
        .and(with_auth_manager(auth_manager))
        .and(with_jwks_manager(jwks_manager))
        .and_then(handle_validate_and_refresh)
}

/// Token validation route for startup validation
fn token_validate_routes(
    database: Arc<Database>,
    auth_manager: &Arc<AuthManager>,
    jwks_manager: &Arc<JwksManager>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("token-validate")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::header::optional::<String>("authorization"))
        .and(warp::body::json())
        .and(with_database(database))
        .and(with_auth_manager(auth_manager))
        .and(with_jwks_manager(jwks_manager))
        .and_then(handle_token_validate)
}

/// Helper to inject database
fn with_database(
    database: Arc<Database>,
) -> impl Filter<Extract = (Arc<Database>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || database.clone())
}

/// Helper to inject auth manager
fn with_auth_manager(
    auth_manager: &Arc<AuthManager>,
) -> impl Filter<Extract = (Arc<AuthManager>,), Error = std::convert::Infallible> + Clone {
    let auth_manager = auth_manager.clone();
    warp::any().map(move || auth_manager.clone())
}

/// Helper to inject JWKS manager
fn with_jwks_manager(
    jwks_manager: &Arc<JwksManager>,
) -> impl Filter<Extract = (Arc<JwksManager>,), Error = std::convert::Infallible> + Clone {
    let jwks_manager = jwks_manager.clone();
    warp::any().map(move || jwks_manager.clone())
}

/// Helper to inject server config
fn with_config(
    config: &Arc<crate::config::environment::ServerConfig>,
) -> impl Filter<
    Extract = (Arc<crate::config::environment::ServerConfig>,),
    Error = std::convert::Infallible,
> + Clone {
    let config = config.clone();
    warp::any().map(move || config.clone())
}

/// Handle client registration (POST /oauth/register)
async fn handle_client_registration(
    request: ClientRegistrationRequest,
    database: Arc<Database>,
) -> Result<impl Reply, Rejection> {
    let client_manager = ClientRegistrationManager::new(database);

    match client_manager.register_client(request).await {
        Ok(response) => {
            let json = warp::reply::json(&response);
            Ok(warp::reply::with_status(
                json,
                warp::http::StatusCode::CREATED,
            ))
        }
        Err(error) => {
            let json = warp::reply::json(&error);
            Ok(warp::reply::with_status(
                json,
                warp::http::StatusCode::BAD_REQUEST,
            ))
        }
    }
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

/// Handle authorization request (GET /oauth/authorize)
async fn handle_authorization(
    params: HashMap<String, String>,
    cookie_header: Option<String>,
    database: Arc<Database>,
    auth_manager: Arc<AuthManager>,
    jwks_manager: Arc<JwksManager>,
) -> Result<Box<dyn warp::Reply>, Rejection> {
    // Parse query parameters into AuthorizeRequest
    let request = match parse_authorize_request(&params) {
        Ok(req) => req,
        Err(error) => {
            let html = render_oauth_error_html(&error);
            let reply = warp::reply::html(html);
            return Ok(Box::new(warp::reply::with_status(
                reply,
                warp::http::StatusCode::BAD_REQUEST,
            )));
        }
    };

    // Check if user is authenticated via session cookie
    let (user_id, tenant_id) = cookie_header
        .and_then(|cookie_value| {
            extract_session_token(&cookie_value).and_then(|token| {
                match auth_manager.validate_token(&token, &jwks_manager) {
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
                        tracing::warn!("Invalid session token in OAuth authorization: {}", e);
                        None
                    }
                }
            })
        })
        .map_or((None, None), |(uid, tid)| (Some(uid), tid));

    // If no authenticated user, redirect to login page with OAuth parameters
    let Some(authenticated_user_id) = user_id else {
        tracing::info!("No authenticated session for OAuth authorization, redirecting to login");
        let login_url = build_login_url_with_oauth_params(&request);

        let redirect_response = warp::reply::with_header(warp::reply(), "Location", login_url);
        return Ok(Box::new(warp::reply::with_status(
            redirect_response,
            warp::http::StatusCode::FOUND,
        )));
    };

    // User is authenticated - proceed with OAuth authorization
    let auth_server = OAuth2AuthorizationServer::new(database, auth_manager, jwks_manager);
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

            // Return 302 redirect response as per OAuth 2.0 spec
            let redirect_response =
                warp::reply::with_header(warp::reply(), "Location", final_redirect_url);
            Ok(Box::new(warp::reply::with_status(
                redirect_response,
                warp::http::StatusCode::FOUND,
            )))
        }
        Err(error) => {
            tracing::error!(
                "OAuth authorization failed for user {}: {:?}",
                authenticated_user_id,
                error
            );
            let html = render_oauth_error_html(&error);
            let reply = warp::reply::html(html);
            Ok(Box::new(warp::reply::with_status(
                reply,
                warp::http::StatusCode::BAD_REQUEST,
            )))
        }
    }
}

/// Handle token request (POST /oauth/token)
async fn handle_token(
    form: HashMap<String, String>,
    database: Arc<Database>,
    auth_manager: Arc<AuthManager>,
    jwks_manager: Arc<JwksManager>,
) -> Result<impl Reply, Rejection> {
    // Debug: Log the incoming form data (excluding sensitive info)
    tracing::debug!(
        "OAuth token request received with grant_type: {:?}, client_id: {:?}",
        form.get("grant_type"),
        form.get("client_id")
    );

    // Parse form data into TokenRequest
    let request = match parse_token_request(&form) {
        Ok(req) => req,
        Err(error) => {
            tracing::warn!("OAuth token request parsing failed: {:?}", error);
            let json = warp::reply::json(&error);
            return Ok(warp::reply::with_status(
                json,
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }
    };

    let auth_server = OAuth2AuthorizationServer::new(database, auth_manager, jwks_manager);

    match auth_server.token(request).await {
        Ok(response) => {
            tracing::info!(
                "OAuth token exchange successful for client: {}",
                form.get("client_id").map_or("unknown", |v| v)
            );
            let json = warp::reply::json(&response);
            Ok(warp::reply::with_status(json, warp::http::StatusCode::OK))
        }
        Err(error) => {
            tracing::warn!(
                "OAuth token exchange failed for client {}: {:?}",
                form.get("client_id").map_or("unknown", |v| v),
                error
            );
            let json = warp::reply::json(&error);
            Ok(warp::reply::with_status(
                json,
                warp::http::StatusCode::BAD_REQUEST,
            ))
        }
    }
}

/// Handle validate and refresh request (POST /oauth2/validate-and-refresh)
async fn handle_validate_and_refresh(
    auth_header: Option<String>,
    request: super::models::ValidateRefreshRequest,
    database: Arc<Database>,
    auth_manager: Arc<AuthManager>,
    jwks_manager: Arc<JwksManager>,
) -> Result<impl Reply, Rejection> {
    // Extract Bearer token from Authorization header
    let access_token = if let Some(header) = auth_header {
        if let Some(token) = header.strip_prefix("Bearer ") {
            token.to_owned()
        } else {
            tracing::warn!("Invalid Authorization header format - missing Bearer prefix");
            return Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "error": "invalid_request",
                    "error_description": "Authorization header must use Bearer scheme"
                })),
                warp::http::StatusCode::UNAUTHORIZED,
            ));
        }
    } else {
        tracing::warn!("Missing Authorization header in validate-and-refresh request");
        return Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": "invalid_request",
                "error_description": "Authorization header is required"
            })),
            warp::http::StatusCode::UNAUTHORIZED,
        ));
    };

    tracing::debug!(
        "Validate-and-refresh request received with token (first 20 chars): {}...",
        &access_token[..std::cmp::min(20, access_token.len())]
    );

    let auth_server = OAuth2AuthorizationServer::new(database, auth_manager, jwks_manager);

    match auth_server
        .validate_and_refresh(&access_token, request)
        .await
    {
        Ok(response) => {
            tracing::info!(
                "Token validation completed with status: {:?}",
                response.status
            );
            let json = warp::reply::json(&response);
            Ok(warp::reply::with_status(json, warp::http::StatusCode::OK))
        }
        Err(error) => {
            tracing::error!("Validate-and-refresh failed: {}", error);
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "error": "internal_error",
                    "error_description": "Failed to validate token"
                })),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

/// Handle token validation request (POST /oauth2/token-validate)
async fn handle_token_validate(
    auth_header: Option<String>,
    request: serde_json::Value,
    database: Arc<Database>,
    auth_manager: Arc<AuthManager>,
    jwks_manager: Arc<JwksManager>,
) -> Result<impl Reply, Rejection> {
    tracing::debug!("Token validation request received");

    // Extract client_id from request body (optional)
    let client_id = request.get("client_id").and_then(|v| v.as_str());

    // Validate access token if provided
    let token_valid = if let Some(header) = auth_header {
        if let Some(token) = header.strip_prefix("Bearer ") {
            // Validate JWT token using RS256
            match auth_manager.validate_token(token, &jwks_manager) {
                Ok(_) => true,
                Err(e) => {
                    tracing::debug!("Token validation failed: {}", e);
                    return Ok(warp::reply::with_status(
                        warp::reply::json(&serde_json::json!({
                            "valid": false,
                            "error": "invalid_token",
                            "error_description": "Access token is invalid or expired"
                        })),
                        warp::http::StatusCode::OK,
                    ));
                }
            }
        } else {
            return Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "valid": false,
                    "error": "invalid_request",
                    "error_description": "Authorization header must use Bearer scheme"
                })),
                warp::http::StatusCode::OK,
            ));
        }
    } else {
        // No token provided - only validate client_id
        false
    };

    // Validate client_id if provided
    if let Some(cid) = client_id {
        let client_manager = super::client_registration::ClientRegistrationManager::new(database);
        match client_manager.get_client(cid).await {
            Ok(_) => {
                // Both token (if provided) and client_id are valid
                tracing::info!("Credentials validated successfully for client_id: {}", cid);
                Ok(warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({
                        "valid": true
                    })),
                    warp::http::StatusCode::OK,
                ))
            }
            Err(e) => {
                tracing::debug!("Client validation failed for {}: {}", cid, e);
                Ok(warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({
                        "valid": false,
                        "error": "invalid_client",
                        "error_description": "Client ID not found or invalid"
                    })),
                    warp::http::StatusCode::OK,
                ))
            }
        }
    } else if token_valid {
        // Only token was provided and it's valid
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "valid": true
            })),
            warp::http::StatusCode::OK,
        ))
    } else {
        // Neither token nor client_id provided
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "valid": false,
                "error": "invalid_request",
                "error_description": "Either access token or client_id must be provided"
            })),
            warp::http::StatusCode::OK,
        ))
    }
}

/// OAuth error template embedded at compile-time
/// Loaded with `include_str`!() to avoid blocking filesystem IO at runtime
const OAUTH_ERROR_TEMPLATE: &str = include_str!("../../templates/oauth_error.html");

/// Render HTML error page for OAuth errors shown in browser
/// Uses compile-time embedded template to avoid blocking async executor on IO
fn render_oauth_error_html(error: &OAuth2Error) -> String {
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

    // Use embedded template - zero filesystem IO, guaranteed to exist at compile-time
    OAUTH_ERROR_TEMPLATE
        .replace("{{error_title}}", error_title)
        .replace("{{ERROR}}", &error.error)
        .replace("{{PROVIDER}}", "Pierre MCP Server")
        .replace(
            "{{DESCRIPTION}}",
            &format!(r#"<div class="description">{error_description}</div>"#),
        )
}

/// Parse query parameters into `AuthorizeRequest`
fn parse_authorize_request(
    params: &HashMap<String, String>,
) -> Result<AuthorizeRequest, OAuth2Error> {
    tracing::trace!(
        "Parsing OAuth authorize request with {} parameters",
        params.len()
    );
    for (key, value) in params {
        tracing::trace!("  {}: {}", key, value);
    }

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
            .clone()
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
) -> Result<String> {
    // Look up user by email
    let user = database
        .get_user_by_email(email)
        .await?
        .ok_or_else(|| AppError::not_found("User not found"))?;

    // Verify password hash
    if !verify_password(password, &user.password_hash).await {
        return Err(AppError::auth_invalid("Invalid password").into());
    }

    // Use AuthManager to generate JWT token with RS256 (proper architecture)
    // This ensures consistent JWT handling across the entire system
    let token = auth_manager.generate_token(&user, jwks_manager)?;

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

/// Handle OAuth login page (GET /oauth2/login)
async fn handle_oauth_login_page(
    params: HashMap<String, String>,
    config: Arc<crate::config::environment::ServerConfig>,
) -> Result<impl Reply, Rejection> {
    // Extract OAuth parameters to preserve them through login flow (including PKCE)
    let client_id = params.get("client_id").map_or("", |v| v);
    let redirect_uri = params.get("redirect_uri").map_or("", |v| v);
    let response_type = params.get("response_type").map_or("", |v| v);
    let state = params.get("state").map_or("", |v| v);
    let scope = params.get("scope").map_or("", |v| v);
    let code_challenge = params.get("code_challenge").map_or("", |v| v);
    let code_challenge_method = params.get("code_challenge_method").map_or("", |v| v);

    // Get default form values from ServerConfig (for dev/test only)
    // Note: In production, these config values should never be set
    // Set OAUTH_DEFAULT_EMAIL and OAUTH_DEFAULT_PASSWORD in .envrc for dev convenience
    let default_email = config
        .oauth2_server
        .default_login_email
        .as_deref()
        .unwrap_or_default();
    let default_password = config
        .oauth2_server
        .default_login_password
        .as_deref()
        .unwrap_or_default();

    // Simple HTML login form that preserves OAuth parameters
    let html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Pierre MCP Server - OAuth Login</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; }}
        .login-form {{ max-width: 400px; margin: 0 auto; padding: 20px; border: 1px solid #ddd; border-radius: 8px; }}
        .form-group {{ margin-bottom: 15px; }}
        label {{ display: block; margin-bottom: 5px; font-weight: bold; }}
        input {{ width: 100%; padding: 8px; border: 1px solid #ccc; border-radius: 4px; }}
        button {{ background-color: #007bff; color: white; padding: 10px 20px; border: none; border-radius: 4px; cursor: pointer; }}
        button:hover {{ background-color: #0056b3; }}
        .oauth-info {{ background-color: #f8f9fa; padding: 15px; border-radius: 4px; margin-bottom: 20px; }}
    </style>
</head>
<body>
    <div class="login-form">
        <h2>OAuth Login Required</h2>
        <div class="oauth-info">
            <strong>Application:</strong> {client_id}<br>
            <strong>Requested Permissions:</strong> {scope}
        </div>
        <form method="post" action="/oauth2/login">
            <input type="hidden" name="client_id" value="{client_id}">
            <input type="hidden" name="redirect_uri" value="{redirect_uri}">
            <input type="hidden" name="response_type" value="{response_type}">
            <input type="hidden" name="state" value="{state}">
            <input type="hidden" name="scope" value="{scope}">
            <input type="hidden" name="code_challenge" value="{code_challenge}">
            <input type="hidden" name="code_challenge_method" value="{code_challenge_method}">

            <div class="form-group">
                <label for="email">Email:</label>
                <input type="email" id="email" name="email" value="{default_email}" required>
            </div>

            <div class="form-group">
                <label for="password">Password:</label>
                <input type="password" id="password" name="password" value="{default_password}" required>
            </div>

            <button type="submit">Login and Authorize</button>
        </form>
    </div>
</body>
</html>
    "#,
        client_id = client_id,
        redirect_uri = redirect_uri,
        response_type = response_type,
        state = state,
        scope = if scope.is_empty() {
            "fitness:read activities:read profile:read"
        } else {
            scope
        },
        code_challenge = code_challenge,
        code_challenge_method = code_challenge_method,
        default_email = default_email,
        default_password = default_password
    );

    Ok(warp::reply::with_header(
        html,
        "content-type",
        "text/html; charset=utf-8",
    ))
}

/// Handle OAuth login form submission (POST /oauth2/login)
async fn handle_oauth_login_submit(
    form: HashMap<String, String>,
    database: Arc<Database>,
    auth_manager: Arc<AuthManager>,
    jwks_manager: Arc<JwksManager>,
) -> Result<Box<dyn warp::Reply>, Rejection> {
    // Extract credentials from form
    let Some(email) = form.get("email") else {
        return Ok(Box::new(warp::reply::with_status(
            "Missing email",
            warp::http::StatusCode::BAD_REQUEST,
        )));
    };

    let Some(password) = form.get("password") else {
        return Ok(Box::new(warp::reply::with_status(
            "Missing password",
            warp::http::StatusCode::BAD_REQUEST,
        )));
    };

    // Authenticate user using database lookup and password verification
    match authenticate_user_with_auth_manager(
        database.clone(),
        email,
        password,
        &auth_manager,
        &jwks_manager,
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

            let auth_url = build_authorization_url_from_form(
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
            let redirect_response = warp::reply::with_header(
                warp::reply::with_header(warp::reply(), "Location", auth_url),
                "Set-Cookie",
                format!(
                    "pierre_session={token}; HttpOnly; Secure; Path=/; SameSite=Lax; Max-Age=86400"
                ),
            );

            Ok(Box::new(warp::reply::with_status(
                redirect_response,
                warp::http::StatusCode::FOUND,
            )))
        }
        Err(e) => {
            tracing::warn!("Authentication failed for OAuth login: {}", e);

            // Return to login page with error message
            let error_html = format!(
                r#"
<!DOCTYPE html>
<html>
<head>
    <title>Pierre MCP Server - Login Error</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; }}
        .error {{ color: red; background-color: #ffe6e6; padding: 15px; border-radius: 4px; margin-bottom: 20px; }}
    </style>
</head>
<body>
    <div class="error">
        <strong>Authentication Failed:</strong> Invalid email or password. Please try again.
    </div>
    <a href="/oauth2/login?client_id={}&redirect_uri={}&response_type={}&state={}&scope={}&code_challenge={}&code_challenge_method={}">← Back to Login</a>
</body>
</html>
            "#,
                form.get("client_id").map_or("", |v| v),
                urlencoding::encode(form.get("redirect_uri").map_or("", |v| v)),
                form.get("response_type").map_or("", |v| v),
                form.get("state").map_or("", |v| v),
                urlencoding::encode(form.get("scope").map_or("", |v| v)),
                urlencoding::encode(form.get("code_challenge").map_or("", |v| v)),
                form.get("code_challenge_method").map_or("", |v| v)
            );

            Ok(Box::new(warp::reply::with_header(
                warp::reply::with_status(error_html, warp::http::StatusCode::UNAUTHORIZED),
                "content-type",
                "text/html; charset=utf-8",
            )))
        }
    }
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

/// JWKS (JSON Web Key Set) endpoint route
fn jwks_route(
    jwks_manager: &Arc<JwksManager>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("jwks")
        .and(warp::path::end())
        .and(warp::get())
        .and(warp::header::optional::<String>("if-none-match"))
        .and(with_jwks_manager(jwks_manager))
        .and_then(handle_jwks)
}

/// Handle JWKS endpoint (GET /oauth2/jwks or GET /.well-known/jwks.json)
async fn handle_jwks(
    if_none_match: Option<String>,
    jwks_manager: Arc<JwksManager>,
) -> Result<Box<dyn warp::Reply>, Rejection> {
    // Return JWKS with RS256 public keys for token validation
    match jwks_manager.get_jwks() {
        Ok(jwks) => {
            tracing::debug!("JWKS endpoint accessed, returning {} keys", jwks.keys.len());

            // Calculate ETag from JWKS content for efficient caching
            let jwks_json = serde_json::to_string(&jwks).map_err(|e| {
                tracing::error!("Failed to serialize JWKS: {}", e);
                warp::reject::reject()
            })?;

            let mut hasher = Sha256::new();
            hasher.update(jwks_json.as_bytes());
            let hash = hasher.finalize();
            let etag = format!(r#""{}""#, hex::encode(&hash[..16]));

            // Check if client's cached version matches current version
            if let Some(client_etag) = if_none_match {
                if client_etag == etag {
                    tracing::debug!("JWKS ETag match, returning 304 Not Modified");
                    // Client has current version - return 304 Not Modified
                    return Ok(Box::new(warp::reply::with_status(
                        warp::reply::with_header(warp::reply(), "ETag", etag),
                        warp::http::StatusCode::NOT_MODIFIED,
                    )));
                }
            }

            // Return JWKS with ETag and Cache-Control headers
            let json = warp::reply::json(&jwks);
            Ok(Box::new(warp::reply::with_header(
                warp::reply::with_header(json, "Cache-Control", "public, max-age=3600"),
                "ETag",
                etag,
            )))
        }
        Err(e) => {
            tracing::error!("Failed to generate JWKS: {}", e);
            // Return empty JWKS on error (graceful degradation)
            let json = warp::reply::json(&serde_json::json!({
                "keys": []
            }));
            Ok(Box::new(warp::reply::with_header(
                json,
                "Cache-Control",
                "public, max-age=3600",
            )))
        }
    }
}
