// ABOUTME: OAuth 2.0 client implementation for connecting to fitness providers
// ABOUTME: Provides OAuth flows for Strava, Fitbit, and Garmin with multi-tenant support
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! # OAuth 2.0 Client Module
//!
//! Pierre acts as an OAuth 2.0 client to connect to third-party fitness providers
//! (Strava, Fitbit, Garmin) on behalf of users. This module handles:
//! - OAuth 2.0 authorization flows with PKCE
//! - Token management and automatic refresh
//! - Multi-tenant credential isolation
//! - Provider-specific authentication

pub mod client;
pub mod flow_manager;
pub mod tenant_client;

// Re-export main OAuth 2.0 client types
pub use client::{OAuth2Client, OAuth2Config, OAuth2Token, PkceParams};

// Re-export tenant-aware client
pub use tenant_client::{StoreCredentialsRequest, TenantOAuthClient};

// Re-export OAuth flow manager
pub use flow_manager::OAuthFlowManager;
