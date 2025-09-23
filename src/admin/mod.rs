// ABOUTME: Admin token system module organization and exports
// ABOUTME: Provides secure admin authentication for API key provisioning and user management
//! Admin Token System
//!
//! This module provides secure admin authentication for API key provisioning.
//! Admin services can authenticate using JWT tokens to provision, revoke, and
//! manage API keys for users.

pub mod auth;
pub mod jwt;
pub mod models;

// Admin authentication service and middleware
pub use auth::{middleware, AdminAuthService};

// JWT token management for admin authentication
pub use jwt::{AdminJwtManager, TokenGenerationConfig};

// Admin system data models and permissions
pub use models::{
    AdminAction, AdminPermission, AdminPermissions, AdminToken, AdminTokenUsage,
    ApiKeyProvisionRequest, CreateAdminTokenRequest, GeneratedAdminToken, ProvisionedApiKey,
    RateLimitPeriod, ValidatedAdminToken,
};
