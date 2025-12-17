// ABOUTME: OAuth 2.0 server implementation with JWT tokens underneath
// ABOUTME: Provides RFC 7591 client registration and OAuth 2.0 endpoints for MCP client compatibility
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

/// RFC 7591 dynamic client registration implementation
pub mod client_registration;
/// OAuth 2.0 authorization server endpoints
pub mod endpoints;
/// OAuth 2.0 data models and types
pub mod models;
/// Rate limiting for OAuth 2.0 endpoints
pub mod rate_limiting;
/// Typestate pattern for compile-time OAuth flow safety
pub mod typestate;

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

// OAuth 2.0 typestate pattern for compile-time flow safety
/// Authenticated OAuth flow state (has tokens)
pub use typestate::Authenticated;
/// Authorized OAuth flow state (has authorization code)
pub use typestate::Authorized;
/// Initial OAuth flow state
pub use typestate::Initial;
/// OAuth flow with compile-time state transitions
pub use typestate::OAuthFlow;
/// PKCE configuration for enhanced security
pub use typestate::PkceConfig;
/// PKCE code challenge method
pub use typestate::PkceMethod;
/// Refreshable OAuth flow state (access token expired)
pub use typestate::Refreshable;
