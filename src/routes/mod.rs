// ABOUTME: Route module organization for Pierre MCP Server HTTP endpoints
// ABOUTME: Provides centralized route definitions organized by domain with clean separation of concerns
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Route module for Pierre MCP Server
//!
//! This module organizes all HTTP routes by domain for better maintainability
//! and clear separation of concerns. Each domain module contains only route
//! definitions and thin handler functions that delegate to service layers.

/// Agent-to-Agent (A2A) protocol routes
pub mod a2a;
/// Admin API routes for user management and configuration
pub mod admin;
/// API key management routes
pub mod api_keys;
/// Authentication and authorization routes
pub mod auth;
/// Configuration management routes
pub mod configuration;
/// Dashboard and monitoring routes
pub mod dashboard;
/// Fitness configuration routes
pub mod fitness;
/// Health check and system status routes
pub mod health;
/// Model Context Protocol (MCP) server routes
pub mod mcp;
/// OAuth 2.0 server implementation routes
pub mod oauth2;
/// Tenant management routes
pub mod tenants;
/// User OAuth app management routes
pub mod user_oauth_apps;
/// Web-facing admin routes (cookie auth for admin users)
pub mod web_admin;
/// WebSocket routes for real-time communication
pub mod websocket;

// Re-export commonly used types from each domain for backward compatibility

/// Agent-to-Agent protocol route handlers
pub use a2a::A2ARoutes;
/// Admin API context and route handlers
pub use admin::AdminApiContext;
/// Admin route handlers
pub use admin::AdminRoutes;
/// Authentication route handlers
pub use auth::AuthRoutes;
/// Authentication service
pub use auth::AuthService;
/// OAuth connection status
pub use auth::ConnectionStatus;
/// Login request payload
pub use auth::LoginRequest;
/// Login response with token
pub use auth::LoginResponse;
/// OAuth authorization response
pub use auth::OAuthAuthorizationResponse;
/// OAuth callback response
pub use auth::OAuthCallbackResponse;
/// OAuth service for provider integration
pub use auth::OAuthService;
/// OAuth connection status enum
pub use auth::OAuthStatus;
/// Refresh token request payload
pub use auth::RefreshTokenRequest;
/// User registration request
pub use auth::RegisterRequest;
/// Registration response with user details
pub use auth::RegisterResponse;
/// Setup status response
pub use auth::SetupStatusResponse;
/// User information
pub use auth::UserInfo;
/// Health check route handlers
pub use health::HealthRoutes;
/// MCP protocol route handlers
pub use mcp::McpRoutes;
/// OAuth 2.0 server route handlers
pub use oauth2::OAuth2Routes;
/// WebSocket route handlers
pub use websocket::WebSocketRoutes;

// For backward compatibility, re-export OAuth functionality

/// OAuth routes (alias for `OAuthService`)
pub type OAuthRoutes = OAuthService;

// Re-export new route handlers
/// API key route handlers
pub use api_keys::ApiKeyRoutes;
/// Configuration route handlers
pub use configuration::ConfigurationRoutes;
/// Dashboard route handlers
pub use dashboard::DashboardRoutes;
/// Fitness configuration route handlers
pub use fitness::FitnessConfigurationRoutes;
/// Tenant route handlers
pub use tenants::TenantRoutes;
/// User OAuth app route handlers
pub use user_oauth_apps::UserOAuthAppRoutes;
/// Web admin route handlers
pub use web_admin::WebAdminRoutes;
