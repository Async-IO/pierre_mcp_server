// ABOUTME: OAuth 2.0 server implementation with JWT tokens underneath
// ABOUTME: Provides RFC 7591 client registration and OAuth 2.0 endpoints for MCP client compatibility
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

/// RFC 7591 dynamic client registration implementation
pub mod client_registration;
/// OAuth 2.0 authorization server endpoints
pub mod endpoints;
/// OAuth 2.0 data models and types
pub mod models;
/// Rate limiting for OAuth 2.0 endpoints
pub mod rate_limiting;

// RFC 7591 client registration management
pub use client_registration::ClientRegistrationManager;

// OAuth 2.0 authorization server implementation

/// OAuth 2.0 authorization server
pub use endpoints::OAuth2AuthorizationServer;

// OAuth 2.0 data models and request/response types

/// Authorization request
pub use models::AuthorizeRequest;
/// Authorization response
pub use models::AuthorizeResponse;
/// Client registration request
pub use models::ClientRegistrationRequest;
/// Client registration response
pub use models::ClientRegistrationResponse;
/// OAuth 2.0 access token
pub use models::OAuth2AccessToken;
/// OAuth 2.0 authorization code
pub use models::OAuth2AuthCode;
/// OAuth 2.0 client
pub use models::OAuth2Client;
/// OAuth 2.0 error response
pub use models::OAuth2Error;
/// Token exchange request
pub use models::TokenRequest;
/// Token exchange response
pub use models::TokenResponse;

// OAuth 2.0 rate limiting
pub use rate_limiting::OAuth2RateLimiter;
