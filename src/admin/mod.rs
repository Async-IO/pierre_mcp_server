// ABOUTME: Admin token system module organization and exports
// ABOUTME: Provides secure admin authentication for API key provisioning and user management
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
//! Admin Token System
//!
//! This module provides secure admin authentication for API key provisioning.
//! Admin services can authenticate using JWT tokens to provision, revoke, and
//! manage API keys for users.

/// Admin authentication service
pub mod auth;
/// JWKS (JSON Web Key Set) management for RS256 JWT
pub mod jwks;
/// JWT token generation and validation for admin auth
pub mod jwt;
/// Admin system data models and permissions
pub mod models;

// Admin authentication service
/// Admin authentication middleware for Axum
pub use auth::middleware;
pub use auth::AdminAuthService;

// JWT token management for admin authentication

/// Admin JWT token manager
pub use jwt::AdminJwtManager;
/// Token generation configuration
pub use jwt::TokenGenerationConfig;

// JWKS (JSON Web Key Set) management for asymmetric JWT

/// JSON Web Key representation
pub use jwks::JsonWebKey;
/// JSON Web Key Set container
pub use jwks::JsonWebKeySet;
/// JWKS manager for key rotation
pub use jwks::JwksManager;
/// RSA key pair for JWT signing
pub use jwks::RsaKeyPair;

// Admin system data models and permissions

/// Admin action enumeration
pub use models::AdminAction;
/// Individual admin permission
pub use models::AdminPermission;
/// Set of admin permissions
pub use models::AdminPermissions;
/// Admin token with metadata
pub use models::AdminToken;
/// Admin token usage tracking
pub use models::AdminTokenUsage;
/// Request to provision API key
pub use models::ApiKeyProvisionRequest;
/// Request to create admin token
pub use models::CreateAdminTokenRequest;
/// Generated admin token response
pub use models::GeneratedAdminToken;
/// Provisioned API key response
pub use models::ProvisionedApiKey;
/// Rate limit period configuration
pub use models::RateLimitPeriod;
/// Validated admin token
pub use models::ValidatedAdminToken;
