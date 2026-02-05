// ABOUTME: OAuth 2.0 client implementation for connecting to fitness providers
// ABOUTME: Provides OAuth flows for Strava, Fitbit, Garmin, WHOOP, and Terra with multi-tenant support
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # OAuth 2.0 Client Module
//!
//! Pierre acts as an OAuth 2.0 client to connect to third-party fitness providers
//! (Strava, Fitbit, Garmin, WHOOP, Terra) on behalf of users. This module handles:
//! - OAuth 2.0 authorization flows with PKCE
//! - Token management and automatic refresh
//! - Multi-tenant credential isolation
//! - Provider-specific authentication

/// Core OAuth 2.0 client implementation
pub mod client;
/// Multi-tenant OAuth client wrapper
pub mod tenant_client;

// Re-export main OAuth 2.0 client types
pub use client::{OAuth2Client, OAuth2Config, OAuth2Token, OAuthClientState, PkceParams};

/// Re-export tenant-aware OAuth client types
pub use tenant_client::{StoreCredentialsRequest, TenantOAuthClient};
