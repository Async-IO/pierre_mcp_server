// ABOUTME: OAuth 2.0 HTTP route handlers for warp web framework
// ABOUTME: Provides REST endpoints for client registration, authorization, and token exchange
//
// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - OAuth client field ownership transfers for registration and token requests
// - Resource Arc sharing for HTTP route handlers
// - String ownership for OAuth protocol responses

use crate::auth::AuthManager;
use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::oauth2::{
    client_registration::ClientRegistrationManager,
    endpoints::OAuth2AuthorizationServer,
    models::{AuthorizeRequest, ClientRegistrationRequest, OAuth2Error, TokenRequest},
};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use warp::{Filter, Rejection, Reply};

/// OAuth 2.0 route filters
pub fn oauth2_routes(
    database: Arc<Database>,
    auth_manager: &Arc<AuthManager>,
    http_port: u16,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let client_registration_routes = client_registration_routes(database.clone());
    let authorization_routes = authorization_routes(database.clone(), auth_manager);
    let token_routes = token_routes(database, auth_manager);
    let jwks_route = jwks_route();

    // OAuth routes under /oauth2 prefix
    let oauth_prefixed_routes = warp::path("oauth2").and(
        client_registration_routes
            .or(authorization_routes)
            .or(token_routes)
            .or(jwks_route),
    );

    // Discovery route at root level (RFC 8414 compliance)
    let discovery_route = oauth2_discovery_route(http_port);

    // Combine root-level discovery with prefixed OAuth routes
    discovery_route.or(oauth_prefixed_routes)
}

/// OAuth 2.0 discovery route (RFC 8414)
fn oauth2_discovery_route(
    http_port: u16,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!(".well-known" / "oauth-authorization-server")
        .and(warp::get())
        .map(move || {
            let host = std::env::var("HOST").unwrap_or_else(|_| "localhost".to_string());
            let base_url = format!("http://{host}:{http_port}");
            warp::reply::json(&serde_json::json!({
                "issuer": base_url,
                "authorization_endpoint": format!("{}/oauth2/authorize", base_url),
                "token_endpoint": format!("{}/oauth2/token", base_url),
                "registration_endpoint": format!("{}/oauth2/register", base_url),
                "grant_types_supported": ["authorization_code", "client_credentials", "refresh_token"],
                "response_types_supported": ["code"],
                "token_endpoint_auth_methods_supported": ["client_secret_post", "client_secret_basic"],
                "scopes_supported": ["fitness:read", "activities:read", "profile:read"],
                "response_modes_supported": ["query"],
                "code_challenge_methods_supported": ["S256", "plain"]
            }))
        })
}

/// Client registration routes (RFC 7591)
fn client_registration_routes(
    database: Arc<Database>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("register")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(with_database(database))
        .and_then(handle_client_registration)
}

/// Authorization endpoint routes
fn authorization_routes(
    database: Arc<Database>,
    auth_manager: &Arc<AuthManager>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    // OAuth authorization endpoint with cookie support
    let authorize_route = warp::path("authorize")
        .and(warp::path::end())
        .and(warp::get())
        .and(warp::query::<HashMap<String, String>>())
        .and(warp::header::optional::<String>("cookie"))
        .and(with_database(database.clone()))
        .and(with_auth_manager(auth_manager))
        .and_then(handle_authorization);

    // OAuth login page
    let login_route = warp::path("login")
        .and(warp::path::end())
        .and(warp::get())
        .and(warp::query::<HashMap<String, String>>())
        .and_then(handle_oauth_login_page);

    // OAuth login form submission
    let login_submit_route = warp::path("login")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::form::<HashMap<String, String>>())
        .and(with_database(database))
        .and(with_auth_manager(auth_manager))
        .and_then(handle_oauth_login_submit);

    authorize_route.or(login_route).or(login_submit_route)
}

/// Token endpoint routes
fn token_routes(
    database: Arc<Database>,
    auth_manager: &Arc<AuthManager>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("token")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::form())
        .and(with_database(database))
        .and(with_auth_manager(auth_manager))
        .and_then(handle_token)
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

/// Handle authorization request (GET /oauth/authorize)
async fn handle_authorization(
    params: HashMap<String, String>,
    cookie_header: Option<String>,
    database: Arc<Database>,
    auth_manager: Arc<AuthManager>,
) -> Result<Box<dyn warp::Reply>, Rejection> {
    // Parse query parameters into AuthorizeRequest
    let request = match parse_authorize_request(&params) {
        Ok(req) => req,
        Err(error) => {
            let json = warp::reply::json(&error);
            return Ok(Box::new(warp::reply::with_status(
                json,
                warp::http::StatusCode::BAD_REQUEST,
            )));
        }
    };

    // Check if user is authenticated via session cookie
    let user_id = cookie_header.and_then(|cookie_value| {
        extract_session_token(&cookie_value).and_then(|token| {
            match auth_manager.validate_token(&token) {
                Ok(claims) => {
                    tracing::info!(
                        "OAuth authorization for authenticated user: {}",
                        claims.email
                    );
                    // Parse user ID from JWT claims
                    if let Ok(user_uuid) = uuid::Uuid::parse_str(&claims.sub) {
                        Some(user_uuid)
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
    });

    // If no authenticated user, redirect to login page with OAuth parameters
    let Some(authenticated_user_id) = user_id else {
        tracing::info!("No authenticated session for OAuth authorization, redirecting to login");
        // Build login URL with OAuth parameters preserved
        let login_url = format!(
            "/oauth2/login?client_id={}&redirect_uri={}&response_type={}&state={}{}",
            request.client_id,
            urlencoding::encode(&request.redirect_uri),
            request.response_type,
            request.state.as_deref().unwrap_or(""),
            request
                .scope
                .as_ref()
                .map_or_else(String::new, |scope| format!(
                    "&scope={}",
                    urlencoding::encode(scope)
                ))
        );

        let redirect_response = warp::reply::with_header(warp::reply(), "Location", login_url);
        return Ok(Box::new(warp::reply::with_status(
            redirect_response,
            warp::http::StatusCode::FOUND,
        )));
    };

    // User is authenticated - proceed with OAuth authorization
    let auth_server = OAuth2AuthorizationServer::new(database, auth_manager);
    let redirect_uri = request.redirect_uri.clone(); // Safe: OAuth redirect URI needed for response

    match auth_server
        .authorize(request, Some(authenticated_user_id))
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
            let json = warp::reply::json(&error);
            Ok(Box::new(warp::reply::with_status(
                json,
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

    let auth_server = OAuth2AuthorizationServer::new(database, auth_manager);

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

/// Parse query parameters into `AuthorizeRequest`
fn parse_authorize_request(
    params: &HashMap<String, String>,
) -> Result<AuthorizeRequest, OAuth2Error> {
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

    Ok(AuthorizeRequest {
        response_type,
        client_id,
        redirect_uri,
        scope,
        state,
    })
}

/// Parse form data into `TokenRequest`
fn parse_token_request(form: &HashMap<String, String>) -> Result<TokenRequest, OAuth2Error> {
    let grant_type = form
        .get("grant_type")
        .ok_or_else(|| OAuth2Error::invalid_request("Missing grant_type parameter"))?
        .clone(); // Safe: String ownership required for OAuth2 request struct

    let client_id = form
        .get("client_id")
        .ok_or_else(|| OAuth2Error::invalid_request("Missing client_id parameter"))?
        .clone(); // Safe: String ownership required for OAuth2 request struct

    let client_secret = form
        .get("client_secret")
        .ok_or_else(|| OAuth2Error::invalid_request("Missing client_secret parameter"))?
        .replace(' ', "+"); // Fix URL decoding: spaces back to + for Base64

    let code = form.get("code").cloned();
    let redirect_uri = form.get("redirect_uri").cloned();
    let scope = form.get("scope").cloned();
    let refresh_token = form.get("refresh_token").cloned();

    Ok(TokenRequest {
        grant_type,
        code,
        redirect_uri,
        client_id,
        client_secret,
        scope,
        refresh_token,
    })
}

/// Authenticate user credentials using `AuthManager` (proper architecture)
async fn authenticate_user_with_auth_manager(
    database: Arc<Database>,
    email: &str,
    password: &str,
    auth_manager: &AuthManager,
) -> Result<String> {
    // Look up user by email
    let user = database
        .get_user_by_email(email)
        .await?
        .ok_or_else(|| anyhow::anyhow!("User not found"))?;

    // Verify password hash
    if !verify_password(password, &user.password_hash) {
        return Err(anyhow::anyhow!("Invalid password"));
    }

    // Use AuthManager to generate JWT token (proper architecture)
    // This ensures consistent JWT handling across the entire system
    let token = auth_manager.generate_token(&user)?;

    Ok(token)
}

/// Verify password against hash using bcrypt
fn verify_password(password: &str, hash: &str) -> bool {
    // Use bcrypt to verify password against stored hash
    bcrypt::verify(password, hash).unwrap_or(false)
}

/// Handle OAuth login page (GET /oauth2/login)
async fn handle_oauth_login_page(params: HashMap<String, String>) -> Result<impl Reply, Rejection> {
    // Extract OAuth parameters to preserve them through login flow
    let client_id = params.get("client_id").map_or("", |v| v);
    let redirect_uri = params.get("redirect_uri").map_or("", |v| v);
    let response_type = params.get("response_type").map_or("", |v| v);
    let state = params.get("state").map_or("", |v| v);
    let scope = params.get("scope").map_or("", |v| v);

    // Get default form values from environment variables (for dev/test only)
    let default_email = std::env::var("OAUTH_DEFAULT_EMAIL").unwrap_or_default();
    let default_password = std::env::var("OAUTH_DEFAULT_PASSWORD").unwrap_or_default();

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
    match authenticate_user_with_auth_manager(database.clone(), email, password, &auth_manager)
        .await
    {
        Ok(token) => {
            // Extract OAuth parameters from form to continue authorization flow
            let client_id = form.get("client_id").map_or("", |v| v);
            let redirect_uri = form.get("redirect_uri").map_or("", |v| v);
            let response_type = form.get("response_type").map_or("", |v| v);
            let state = form.get("state").map_or("", |v| v);
            let scope = form.get("scope").map_or("", |v| v);

            // Build authorization URL with all preserved parameters
            let auth_url = format!(
                "/oauth2/authorize?client_id={}&redirect_uri={}&response_type={}&state={}{}",
                client_id,
                urlencoding::encode(redirect_uri),
                response_type,
                state,
                if scope.is_empty() {
                    String::new()
                } else {
                    format!("&scope={}", urlencoding::encode(scope))
                }
            );

            tracing::info!(
                "User {} authenticated successfully for OAuth, redirecting to authorization",
                email
            );

            // Set session cookie and redirect to authorization endpoint
            let redirect_response = warp::reply::with_header(
                warp::reply::with_header(warp::reply(), "Location", auth_url),
                "Set-Cookie",
                format!("pierre_session={token}; HttpOnly; Path=/; SameSite=Lax"),
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
    <a href="/oauth2/login?client_id={}&redirect_uri={}&response_type={}&state={}&scope={}">← Back to Login</a>
</body>
</html>
            "#,
                form.get("client_id").map_or("", |v| v),
                urlencoding::encode(form.get("redirect_uri").map_or("", |v| v)),
                form.get("response_type").map_or("", |v| v),
                form.get("state").map_or("", |v| v),
                urlencoding::encode(form.get("scope").map_or("", |v| v))
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
            return Some(session_token.to_string());
        }
    }
    None
}

/// JWKS (JSON Web Key Set) endpoint route
fn jwks_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("jwks")
        .and(warp::path::end())
        .and(warp::get())
        .and_then(|| async move {
            // Generate JWKS placeholder (simplified for HMAC)
            // AuthManager handles JWT internally with proper secret management
            // In a full OAuth implementation, this would use RSA/ECDSA keys
            let jwks = serde_json::json!({
                "keys": [{
                    "kty": "oct",
                    "use": "sig",
                    "alg": "HS256",
                    "kid": "pierre-oauth-key-1"
                    // Note: Never expose the actual secret in JWKS
                    // This is a placeholder structure for HMAC validation
                }]
            });

            Ok::<_, warp::Rejection>(warp::reply::json(&jwks))
        })
}
