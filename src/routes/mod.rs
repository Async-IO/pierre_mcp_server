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
/// Chat conversation routes for AI assistants
pub mod chat;
/// Coaches (custom AI personas) routes
pub mod coaches;
/// Configuration management routes
pub mod configuration;
/// Dashboard and monitoring routes
pub mod dashboard;
/// Fitness configuration routes
pub mod fitness;
/// Health check and system status routes
pub mod health;
/// Impersonation routes for super admin user impersonation
pub mod impersonation;
/// LLM provider settings routes for per-tenant API key configuration
pub mod llm_settings;
/// Model Context Protocol (MCP) server routes
pub mod mcp;
/// OAuth 2.0 server implementation routes
pub mod oauth2;
/// `OpenAPI` documentation routes (feature-gated)
#[cfg(feature = "openapi")]
pub mod openapi;
/// Tenant management routes
pub mod tenants;
/// Tool selection admin routes for per-tenant MCP tool configuration
pub mod tool_selection;
/// User MCP token management routes for AI client authentication
pub mod user_mcp_tokens;
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
/// Chat conversation route handlers
pub use chat::ChatRoutes;
/// Coaches route handlers
pub use coaches::CoachesRoutes;
/// Configuration route handlers
pub use configuration::ConfigurationRoutes;
/// Dashboard route handlers
pub use dashboard::DashboardRoutes;
/// Fitness configuration route handlers
pub use fitness::FitnessConfigurationRoutes;
/// Impersonation route handlers
pub use impersonation::ImpersonationRoutes;
/// `OpenAPI` documentation route handlers (feature-gated)
#[cfg(feature = "openapi")]
pub use openapi::OpenApiRoutes;
/// Tenant route handlers
pub use tenants::TenantRoutes;
/// Tool selection context
pub use tool_selection::ToolSelectionContext;
/// Tool selection route handlers
pub use tool_selection::ToolSelectionRoutes;
/// User MCP token route handlers
pub use user_mcp_tokens::UserMcpTokenRoutes;
/// User OAuth app route handlers
pub use user_oauth_apps::UserOAuthAppRoutes;
/// Web admin route handlers
pub use web_admin::WebAdminRoutes;
