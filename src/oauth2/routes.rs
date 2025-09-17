// ABOUTME: OAuth 2.0 HTTP route handlers for warp web framework
// ABOUTME: Provides REST endpoints for client registration, authorization, and token exchange

use crate::auth::AuthManager;
use crate::database_plugins::factory::Database;
use crate::oauth2::{
    client_registration::ClientRegistrationManager,
    endpoints::OAuth2AuthorizationServer,
    models::{AuthorizeRequest, ClientRegistrationRequest, OAuth2Error, TokenRequest},
};
use std::collections::HashMap;
use std::sync::Arc;
use warp::{Filter, Rejection, Reply};

/// OAuth 2.0 route filters
pub fn oauth2_routes(
    database: Arc<Database>,
    auth_manager: Arc<AuthManager>,
    http_port: u16,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let client_registration_routes = client_registration_routes(database.clone());
    let authorization_routes = authorization_routes(database.clone(), auth_manager.clone());
    let token_routes = token_routes(database, auth_manager);
    let discovery_route = oauth2_discovery_route(http_port);

    warp::path("oauth").and(
        discovery_route
            .or(client_registration_routes)
            .or(authorization_routes)
            .or(token_routes),
    )
}

/// OAuth 2.0 discovery route (RFC 8414)
fn oauth2_discovery_route(
    http_port: u16,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!(".well-known" / "oauth-authorization-server")
        .and(warp::get())
        .map(move || {
            let base_url = format!("http://localhost:{http_port}");
            warp::reply::json(&serde_json::json!({
                "issuer": base_url,
                "authorization_endpoint": format!("{}/oauth/authorize", base_url),
                "token_endpoint": format!("{}/oauth/token", base_url),
                "registration_endpoint": format!("{}/oauth/register", base_url),
                "grant_types_supported": ["authorization_code", "client_credentials"],
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
    auth_manager: Arc<AuthManager>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("authorize")
        .and(warp::path::end())
        .and(warp::get())
        .and(warp::query::<HashMap<String, String>>())
        .and(with_database(database))
        .and(with_auth_manager(auth_manager))
        .and_then(handle_authorization)
}

/// Token endpoint routes
fn token_routes(
    database: Arc<Database>,
    auth_manager: Arc<AuthManager>,
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
    auth_manager: Arc<AuthManager>,
) -> impl Filter<Extract = (Arc<AuthManager>,), Error = std::convert::Infallible> + Clone {
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
    database: Arc<Database>,
    auth_manager: Arc<AuthManager>,
) -> Result<impl Reply, Rejection> {
    // Parse query parameters into AuthorizeRequest
    let request = match parse_authorize_request(&params) {
        Ok(req) => req,
        Err(error) => {
            let json = warp::reply::json(&error);
            return Ok(warp::reply::with_status(
                json,
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }
    };

    let auth_server = OAuth2AuthorizationServer::new(database, auth_manager);

    // For now, we'll auto-approve without user authentication
    // In a real implementation, this would check for an authenticated session
    let dummy_user_id = uuid::Uuid::new_v4(); // This should come from session

    match auth_server.authorize(request, Some(dummy_user_id)).await {
        Ok(response) => {
            // In a real OAuth flow, this would redirect to the redirect_uri with the code
            // For now, we'll return JSON
            let json = warp::reply::json(&response);
            Ok(warp::reply::with_status(json, warp::http::StatusCode::OK))
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

/// Handle token request (POST /oauth/token)
async fn handle_token(
    form: HashMap<String, String>,
    database: Arc<Database>,
    auth_manager: Arc<AuthManager>,
) -> Result<impl Reply, Rejection> {
    // Parse form data into TokenRequest
    let request = match parse_token_request(&form) {
        Ok(req) => req,
        Err(error) => {
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
            let json = warp::reply::json(&response);
            Ok(warp::reply::with_status(json, warp::http::StatusCode::OK))
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

/// Parse query parameters into `AuthorizeRequest`
fn parse_authorize_request(
    params: &HashMap<String, String>,
) -> Result<AuthorizeRequest, OAuth2Error> {
    let response_type = params
        .get("response_type")
        .ok_or_else(|| OAuth2Error::invalid_request("Missing response_type parameter"))?
        .clone();

    let client_id = params
        .get("client_id")
        .ok_or_else(|| OAuth2Error::invalid_request("Missing client_id parameter"))?
        .clone();

    let redirect_uri = params
        .get("redirect_uri")
        .ok_or_else(|| OAuth2Error::invalid_request("Missing redirect_uri parameter"))?
        .clone();

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
        .clone();

    let client_id = form
        .get("client_id")
        .ok_or_else(|| OAuth2Error::invalid_request("Missing client_id parameter"))?
        .clone();

    let client_secret = form
        .get("client_secret")
        .ok_or_else(|| OAuth2Error::invalid_request("Missing client_secret parameter"))?
        .clone();

    let code = form.get("code").cloned();
    let redirect_uri = form.get("redirect_uri").cloned();
    let scope = form.get("scope").cloned();

    Ok(TokenRequest {
        grant_type,
        code,
        redirect_uri,
        client_id,
        client_secret,
        scope,
    })
}
