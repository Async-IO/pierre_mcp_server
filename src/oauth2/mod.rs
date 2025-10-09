// ABOUTME: OAuth 2.0 server implementation with JWT tokens underneath
// ABOUTME: Provides RFC 7591 client registration and OAuth 2.0 endpoints for MCP client compatibility
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

pub mod client_registration;
pub mod endpoints;
pub mod models;
pub mod routes;

// RFC 7591 client registration management
pub use client_registration::ClientRegistrationManager;

// OAuth 2.0 authorization server implementation
pub use endpoints::OAuth2AuthorizationServer;

// OAuth 2.0 data models and request/response types
pub use models::{
    AuthorizeRequest, AuthorizeResponse, ClientRegistrationRequest, ClientRegistrationResponse,
    OAuth2AccessToken, OAuth2AuthCode, OAuth2Client, OAuth2Error, TokenRequest, TokenResponse,
};

// OAuth 2.0 HTTP route handlers
pub use routes::oauth2_routes;
