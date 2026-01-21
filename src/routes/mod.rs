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
//!
//! Routes are conditionally compiled based on feature flags to support
//! modular server configurations.

// ═══════════════════════════════════════════════════════════════
// ALWAYS ENABLED - Core infrastructure
// ═══════════════════════════════════════════════════════════════

/// Health check and system status routes
pub mod health;

// ═══════════════════════════════════════════════════════════════
// PROTOCOL FEATURES
// ═══════════════════════════════════════════════════════════════

/// Agent-to-Agent (A2A) protocol routes
#[cfg(feature = "protocol-a2a")]
pub mod a2a;

/// Model Context Protocol (MCP) server routes
#[cfg(feature = "protocol-mcp")]
pub mod mcp;

/// Authentication and authorization routes (REST protocol)
#[cfg(feature = "protocol-rest")]
pub mod auth;

// ═══════════════════════════════════════════════════════════════
// TRANSPORT FEATURES
// ═══════════════════════════════════════════════════════════════

/// WebSocket routes for real-time communication
#[cfg(feature = "transport-websocket")]
pub mod websocket;

// ═══════════════════════════════════════════════════════════════
// OAUTH FEATURE
// ═══════════════════════════════════════════════════════════════

/// OAuth 2.0 server implementation routes
#[cfg(feature = "oauth")]
pub mod oauth2;

// ═══════════════════════════════════════════════════════════════
// CLIENT-WEB FEATURES
// ═══════════════════════════════════════════════════════════════

/// Dashboard and monitoring routes
#[cfg(feature = "client-dashboard")]
pub mod dashboard;

/// Configuration management routes
#[cfg(feature = "client-settings")]
pub mod configuration;

/// Fitness configuration routes
#[cfg(feature = "client-settings")]
pub mod fitness;

/// Chat conversation routes for AI assistants
#[cfg(feature = "client-chat")]
pub mod chat;

/// Coaches (custom AI personas) routes
#[cfg(feature = "client-coaches")]
pub mod coaches;

/// User OAuth app management routes
#[cfg(feature = "client-oauth-apps")]
pub mod user_oauth_apps;

// ═══════════════════════════════════════════════════════════════
// CLIENT-ADMIN FEATURES
// ═══════════════════════════════════════════════════════════════

/// Admin API routes for user management and configuration
#[cfg(feature = "client-admin-api")]
pub mod admin;

/// Web-facing admin routes (cookie auth for admin users)
#[cfg(feature = "client-admin-ui")]
pub mod web_admin;

/// API key management routes
#[cfg(feature = "client-api-keys")]
pub mod api_keys;

/// Tenant management routes
#[cfg(feature = "client-tenants")]
pub mod tenants;

/// Impersonation routes for super admin user impersonation
#[cfg(feature = "client-impersonation")]
pub mod impersonation;

/// LLM provider settings routes for per-tenant API key configuration
#[cfg(feature = "client-llm-settings")]
pub mod llm_settings;

/// Tool selection admin routes for per-tenant MCP tool configuration
#[cfg(feature = "client-tool-selection")]
pub mod tool_selection;

// ═══════════════════════════════════════════════════════════════
// OTHER CLIENT FEATURES
// ═══════════════════════════════════════════════════════════════

/// User MCP token management routes for AI client authentication
#[cfg(feature = "client-mcp-tokens")]
pub mod user_mcp_tokens;

// ═══════════════════════════════════════════════════════════════
// OPENAPI FEATURE
// ═══════════════════════════════════════════════════════════════

/// `OpenAPI` documentation routes (feature-gated)
#[cfg(feature = "openapi")]
pub mod openapi;

// ═══════════════════════════════════════════════════════════════
// RE-EXPORTS
// ═══════════════════════════════════════════════════════════════

// Re-export commonly used types from each domain for backward compatibility

/// Health check route handlers
pub use health::HealthRoutes;

// Protocol re-exports
#[cfg(feature = "protocol-a2a")]
pub use a2a::A2ARoutes;

#[cfg(feature = "protocol-mcp")]
pub use mcp::McpRoutes;

#[cfg(feature = "protocol-rest")]
pub use auth::{
    AuthRoutes, AuthService, ConnectionStatus, LoginRequest, LoginResponse,
    OAuthAuthorizationResponse, OAuthCallbackResponse, OAuthService, OAuthStatus,
    RefreshTokenRequest, RegisterRequest, RegisterResponse, UserInfo,
};
// SetupStatusResponse is defined in crate::auth and re-exported here for backward compatibility
#[cfg(feature = "protocol-rest")]
pub use crate::auth::SetupStatusResponse;

// Transport re-exports
#[cfg(feature = "transport-websocket")]
pub use websocket::WebSocketRoutes;

// OAuth re-exports
#[cfg(feature = "oauth")]
pub use oauth2::OAuth2Routes;

// Client-web re-exports
#[cfg(feature = "client-dashboard")]
pub use dashboard::DashboardRoutes;

#[cfg(feature = "client-settings")]
pub use configuration::ConfigurationRoutes;

#[cfg(feature = "client-settings")]
pub use fitness::FitnessConfigurationRoutes;

#[cfg(feature = "client-chat")]
pub use chat::ChatRoutes;

#[cfg(feature = "client-coaches")]
pub use coaches::CoachesRoutes;

#[cfg(feature = "client-oauth-apps")]
pub use user_oauth_apps::UserOAuthAppRoutes;

// Client-admin re-exports
#[cfg(feature = "client-admin-api")]
pub use admin::{AdminApiContext, AdminRoutes};

#[cfg(feature = "client-admin-ui")]
pub use web_admin::WebAdminRoutes;

#[cfg(feature = "client-api-keys")]
pub use api_keys::ApiKeyRoutes;

#[cfg(feature = "client-tenants")]
pub use tenants::TenantRoutes;

#[cfg(feature = "client-impersonation")]
pub use impersonation::ImpersonationRoutes;

#[cfg(feature = "client-tool-selection")]
pub use tool_selection::{ToolSelectionContext, ToolSelectionRoutes};

// Other client re-exports
#[cfg(feature = "client-mcp-tokens")]
pub use user_mcp_tokens::UserMcpTokenRoutes;

// OpenAPI re-exports
#[cfg(feature = "openapi")]
pub use openapi::OpenApiRoutes;

// For backward compatibility, re-export OAuth functionality
#[cfg(feature = "protocol-rest")]
/// OAuth routes (alias for `OAuthService`)
pub type OAuthRoutes = OAuthService;
