// ABOUTME: Feature flag validation module for compile-time feature configuration
// ABOUTME: Validates feature flag combinations at startup and logs enabled features
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Feature Configuration Validation
//!
//! This module provides compile-time feature flag validation to ensure that
//! feature combinations are valid before the server starts. It validates:
//!
//! - Protocol features require appropriate transport support
//! - OAuth is enabled when protocols need provider authentication
//! - Logs all enabled features at startup for debugging
//!
//! ## Usage
//!
//! Call `FeatureConfig::validate()` early in the server startup process to
//! catch configuration errors before resources are initialized.
//!
//! ## Design
//!
//! This module uses compile-time `cfg!()` checks directly in methods rather
//! than storing boolean fields. This approach:
//! - Has zero runtime overhead (all checks are const)
//! - Avoids struct field proliferation
//! - Ensures feature detection matches actual compilation

use crate::errors::{AppError, AppResult};
use tracing::{info, warn};

/// Feature configuration validator
///
/// A zero-sized type that provides methods to check compile-time feature flags.
/// All checks use `cfg!()` macros directly, making them compile-time constants.
///
/// Features are organized into logical groups:
/// - Protocols: REST, MCP, A2A protocol support
/// - Transports: HTTP, WebSocket, SSE, stdio communication
/// - Clients: Dashboard, settings, chat, coaches, admin, etc.
/// - Infrastructure: OAuth, `OpenAPI`
#[derive(Debug, Clone, Copy, Default)]
pub struct FeatureConfig;

impl FeatureConfig {
    /// Create a new feature configuration
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Validate feature configuration and log enabled features
    ///
    /// This should be called early in server startup to catch configuration
    /// errors before expensive resource initialization.
    ///
    /// # Errors
    ///
    /// Returns an error if feature combinations are invalid:
    /// - `protocol-mcp` requires at least one transport feature
    /// - `protocol-a2a` requires `transport-http` feature
    /// - `protocol-rest` requires `transport-http` feature
    pub fn validate() -> AppResult<Self> {
        // Log all enabled features
        Self::log_enabled_features();

        // Validate feature combinations
        Self::validate_protocol_transports()?;
        Self::validate_oauth_dependencies();

        info!("Feature configuration validated successfully");
        Ok(Self::new())
    }

    // ==========================================================================
    // Protocol feature checks
    // ==========================================================================

    /// Check if REST protocol is enabled
    #[must_use]
    pub const fn protocol_rest() -> bool {
        cfg!(feature = "protocol-rest")
    }

    /// Check if MCP protocol is enabled
    #[must_use]
    pub const fn protocol_mcp() -> bool {
        cfg!(feature = "protocol-mcp")
    }

    /// Check if A2A protocol is enabled
    #[must_use]
    pub const fn protocol_a2a() -> bool {
        cfg!(feature = "protocol-a2a")
    }

    /// Check if any protocol is enabled
    #[must_use]
    pub const fn has_any_protocol() -> bool {
        Self::protocol_rest() || Self::protocol_mcp() || Self::protocol_a2a()
    }

    // ==========================================================================
    // Transport feature checks
    // ==========================================================================

    /// Check if HTTP transport is enabled
    #[must_use]
    pub const fn transport_http() -> bool {
        cfg!(feature = "transport-http")
    }

    /// Check if WebSocket transport is enabled
    #[must_use]
    pub const fn transport_websocket() -> bool {
        cfg!(feature = "transport-websocket")
    }

    /// Check if SSE transport is enabled
    #[must_use]
    pub const fn transport_sse() -> bool {
        cfg!(feature = "transport-sse")
    }

    /// Check if stdio transport is enabled
    #[must_use]
    pub const fn transport_stdio() -> bool {
        cfg!(feature = "transport-stdio")
    }

    /// Check if any transport is enabled
    #[must_use]
    pub const fn has_any_transport() -> bool {
        Self::transport_http()
            || Self::transport_websocket()
            || Self::transport_sse()
            || Self::transport_stdio()
    }

    /// Check if any web transport is enabled (HTTP, WebSocket, or SSE)
    #[must_use]
    pub const fn has_web_transport() -> bool {
        Self::transport_http() || Self::transport_websocket() || Self::transport_sse()
    }

    // ==========================================================================
    // Client feature checks
    // ==========================================================================

    /// Check if dashboard client is enabled
    #[must_use]
    pub const fn client_dashboard() -> bool {
        cfg!(feature = "client-dashboard")
    }

    /// Check if settings client is enabled
    #[must_use]
    pub const fn client_settings() -> bool {
        cfg!(feature = "client-settings")
    }

    /// Check if chat client is enabled
    #[must_use]
    pub const fn client_chat() -> bool {
        cfg!(feature = "client-chat")
    }

    /// Check if coaches client is enabled
    #[must_use]
    pub const fn client_coaches() -> bool {
        cfg!(feature = "client-coaches")
    }

    /// Check if OAuth apps client is enabled
    #[must_use]
    pub const fn client_oauth_apps() -> bool {
        cfg!(feature = "client-oauth-apps")
    }

    /// Check if admin API client is enabled
    #[must_use]
    pub const fn client_admin_api() -> bool {
        cfg!(feature = "client-admin-api")
    }

    /// Check if admin UI client is enabled
    #[must_use]
    pub const fn client_admin_ui() -> bool {
        cfg!(feature = "client-admin-ui")
    }

    /// Check if API keys client is enabled
    #[must_use]
    pub const fn client_api_keys() -> bool {
        cfg!(feature = "client-api-keys")
    }

    /// Check if tenants client is enabled
    #[must_use]
    pub const fn client_tenants() -> bool {
        cfg!(feature = "client-tenants")
    }

    /// Check if impersonation client is enabled
    #[must_use]
    pub const fn client_impersonation() -> bool {
        cfg!(feature = "client-impersonation")
    }

    /// Check if LLM settings client is enabled
    #[must_use]
    pub const fn client_llm_settings() -> bool {
        cfg!(feature = "client-llm-settings")
    }

    /// Check if tool selection client is enabled
    #[must_use]
    pub const fn client_tool_selection() -> bool {
        cfg!(feature = "client-tool-selection")
    }

    /// Check if mobile client is enabled
    #[must_use]
    pub const fn client_mobile() -> bool {
        cfg!(feature = "client-mobile")
    }

    /// Check if MCP tokens client is enabled
    #[must_use]
    pub const fn client_mcp_tokens() -> bool {
        cfg!(feature = "client-mcp-tokens")
    }

    /// Check if store client is enabled (Coach Store REST API)
    #[must_use]
    pub const fn client_store() -> bool {
        cfg!(feature = "client-store")
    }

    /// Check if store tools are enabled (Coach Store MCP tools)
    #[must_use]
    pub const fn tools_store() -> bool {
        cfg!(feature = "tools-store")
    }

    /// Check if any client feature is enabled
    #[must_use]
    pub const fn has_any_client() -> bool {
        Self::client_dashboard()
            || Self::client_settings()
            || Self::client_chat()
            || Self::client_coaches()
            || Self::client_oauth_apps()
            || Self::client_admin_api()
            || Self::client_admin_ui()
            || Self::client_api_keys()
            || Self::client_tenants()
            || Self::client_impersonation()
            || Self::client_llm_settings()
            || Self::client_tool_selection()
            || Self::client_mobile()
            || Self::client_mcp_tokens()
            || Self::client_store()
    }

    // ==========================================================================
    // Infrastructure feature checks
    // ==========================================================================

    /// Check if OAuth is enabled
    #[must_use]
    pub const fn oauth() -> bool {
        cfg!(feature = "oauth")
    }

    /// Check if `OpenAPI` is enabled
    #[must_use]
    pub const fn openapi() -> bool {
        cfg!(feature = "openapi")
    }

    // ==========================================================================
    // Validation methods
    // ==========================================================================

    /// Validate that protocols have required transports
    fn validate_protocol_transports() -> AppResult<()> {
        // MCP protocol requires at least one transport
        if Self::protocol_mcp() && !Self::has_any_transport() {
            return Err(AppError::config(
                "protocol-mcp requires at least one transport feature \
                 (transport-http, transport-websocket, transport-sse, or transport-stdio)",
            ));
        }

        // A2A protocol requires at least HTTP transport for its REST-like API
        if Self::protocol_a2a() && !Self::transport_http() {
            return Err(AppError::config(
                "protocol-a2a requires transport-http feature for HTTP-based A2A endpoints",
            ));
        }

        // REST protocol requires HTTP transport
        if Self::protocol_rest() && !Self::transport_http() {
            return Err(AppError::config(
                "protocol-rest requires transport-http feature for HTTP endpoints",
            ));
        }

        Ok(())
    }

    /// Validate OAuth dependencies and warn if missing
    fn validate_oauth_dependencies() {
        // Warn if protocols are enabled but OAuth is disabled
        if Self::has_any_protocol() && !Self::oauth() {
            warn!(
                "OAuth feature is disabled but protocols are enabled. \
                 Provider authentication will not be available."
            );
        }

        // Warn if client-oauth-apps is enabled but oauth is disabled
        if Self::client_oauth_apps() && !Self::oauth() {
            warn!(
                "client-oauth-apps is enabled but oauth feature is disabled. \
                 OAuth app management will be non-functional."
            );
        }
    }

    /// Log all enabled features at startup
    fn log_enabled_features() {
        info!("=== Feature Configuration ===");
        Self::log_protocols();
        Self::log_transports();
        Self::log_clients();
        Self::log_other_features();
        info!("=== End Feature Configuration ===");
    }

    fn log_protocols() {
        let protocols = collect_enabled(&[
            (Self::protocol_rest(), "rest"),
            (Self::protocol_mcp(), "mcp"),
            (Self::protocol_a2a(), "a2a"),
        ]);
        log_feature_category("Protocols", &protocols, true);
    }

    fn log_transports() {
        let transports = collect_enabled(&[
            (Self::transport_http(), "http"),
            (Self::transport_websocket(), "websocket"),
            (Self::transport_sse(), "sse"),
            (Self::transport_stdio(), "stdio"),
        ]);
        log_feature_category("Transports", &transports, true);
    }

    fn log_clients() {
        let clients = collect_enabled(&[
            (Self::client_dashboard(), "dashboard"),
            (Self::client_settings(), "settings"),
            (Self::client_chat(), "chat"),
            (Self::client_coaches(), "coaches"),
            (Self::client_oauth_apps(), "oauth-apps"),
            (Self::client_admin_api(), "admin-api"),
            (Self::client_admin_ui(), "admin-ui"),
            (Self::client_api_keys(), "api-keys"),
            (Self::client_tenants(), "tenants"),
            (Self::client_impersonation(), "impersonation"),
            (Self::client_llm_settings(), "llm-settings"),
            (Self::client_tool_selection(), "tool-selection"),
            (Self::client_mobile(), "mobile"),
            (Self::client_mcp_tokens(), "mcp-tokens"),
            (Self::client_store(), "store"),
        ]);
        log_feature_category("Clients", &clients, false);
    }

    fn log_other_features() {
        let other = collect_enabled(&[(Self::oauth(), "oauth"), (Self::openapi(), "openapi")]);
        if !other.is_empty() {
            info!("Other: {}", other.join(", "));
        }
    }
}

/// Collect enabled features from a slice of (enabled, name) pairs
fn collect_enabled(features: &[(bool, &'static str)]) -> Vec<&'static str> {
    features
        .iter()
        .filter_map(|(enabled, name)| enabled.then_some(*name))
        .collect()
}

/// Log a feature category with appropriate warning if empty
fn log_feature_category(category: &str, features: &[&str], warn_if_empty: bool) {
    if features.is_empty() {
        if warn_if_empty {
            warn!("No {} features enabled!", category.to_lowercase());
        } else {
            info!("{category}: none");
        }
    } else {
        info!("{category}: {}", features.join(", "));
    }
}
