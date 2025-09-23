// ABOUTME: Route module organization for Pierre MCP Server HTTP endpoints
// ABOUTME: Provides centralized route definitions organized by domain with clean separation of concerns

//! Route module for Pierre MCP Server
//!
//! This module organizes all HTTP routes by domain for better maintainability
//! and clear separation of concerns. Each domain module contains only route
//! definitions and thin handler functions that delegate to service layers.

pub mod a2a;
pub mod admin;
pub mod auth;
pub mod health;
pub mod mcp;
pub mod oauth2;
pub mod sse;
pub mod websocket;

// Re-export commonly used types from each domain for backward compatibility
pub use a2a::A2ARoutes;
pub use admin::{AdminApiContext, AdminRoutes};
pub use auth::{
    AuthRoutes, AuthService, ConnectionStatus, LoginRequest, LoginResponse,
    OAuthAuthorizationResponse, OAuthCallbackResponse, OAuthService, OAuthStatus,
    RefreshTokenRequest, RegisterRequest, RegisterResponse, SetupStatusResponse, UserInfo,
};
pub use health::HealthRoutes;
pub use mcp::McpRoutes;
pub use oauth2::OAuth2Routes;
pub use sse::SseRoutes;
pub use websocket::WebSocketRoutes;

// For backward compatibility, re-export OAuth functionality
pub type OAuthRoutes = OAuthService;
