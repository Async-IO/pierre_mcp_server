// ABOUTME: Service Provider Interface (SPI) for pluggable provider architecture
// ABOUTME: Defines the contract that external provider crates must implement for registration
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! # Provider Service Provider Interface (SPI)
//!
//! This module defines the contract that external provider crates must implement
//! to integrate with the Pierre MCP Server. The SPI enables true pluggability by
//! allowing providers to be developed, compiled, and registered independently.
//!
//! ## Key Concepts
//!
//! - **`ProviderDescriptor`**: Describes provider capabilities (OAuth, sleep tracking, etc.)
//! - **`OAuthEndpoints`**: OAuth configuration for providers requiring authentication
//! - **`ProviderBundle`**: Complete provider package for registration
//!
//! ## Example: Implementing a Custom Provider
//!
//! ```rust,no_run
//! use pierre_mcp_server::providers::spi::{ProviderDescriptor, OAuthEndpoints, ProviderCapabilities};
//!
//! pub struct WhoopDescriptor;
//!
//! impl ProviderDescriptor for WhoopDescriptor {
//!     fn name(&self) -> &'static str {
//!         "whoop"
//!     }
//!
//!     fn display_name(&self) -> &'static str {
//!         "WHOOP"
//!     }
//!
//!     fn capabilities(&self) -> ProviderCapabilities {
//!         ProviderCapabilities {
//!             oauth: true,
//!             activities: true,
//!             sleep_tracking: true,
//!             recovery_metrics: true,
//!             health_metrics: true,
//!         }
//!     }
//!
//!     fn oauth_endpoints(&self) -> Option<OAuthEndpoints> {
//!         Some(OAuthEndpoints {
//!             auth_url: "https://api.prod.whoop.com/oauth/oauth2/auth",
//!             token_url: "https://api.prod.whoop.com/oauth/oauth2/token",
//!             revoke_url: Some("https://api.prod.whoop.com/oauth/oauth2/revoke"),
//!         })
//!     }
//!
//!     fn api_base_url(&self) -> &'static str {
//!         "https://api.prod.whoop.com/developer/v1"
//!     }
//!
//!     fn default_scopes(&self) -> &'static [&'static str] {
//!         &["read:profile", "read:workout", "read:sleep", "read:recovery"]
//!     }
//! }
//! ```

use super::core::{FitnessProvider, ProviderConfig};
use std::fmt;

/// OAuth endpoint configuration for providers requiring authentication
#[derive(Debug, Clone)]
pub struct OAuthEndpoints {
    /// OAuth authorization endpoint URL
    pub auth_url: &'static str,
    /// OAuth token endpoint URL
    pub token_url: &'static str,
    /// Optional token revocation endpoint URL
    pub revoke_url: Option<&'static str>,
}

/// Provider capability flags
///
/// Indicates which features a provider supports. Used by the system to
/// route requests to appropriate providers and generate accurate tool descriptions.
#[derive(Debug, Clone, Default)]
pub struct ProviderCapabilities {
    /// Provider requires OAuth authentication
    pub oauth: bool,
    /// Provider supports activity/workout data
    pub activities: bool,
    /// Provider supports sleep tracking data
    pub sleep_tracking: bool,
    /// Provider supports recovery/readiness metrics
    pub recovery_metrics: bool,
    /// Provider supports health metrics (weight, HRV, etc.)
    pub health_metrics: bool,
}

impl ProviderCapabilities {
    /// Create capabilities for an activity-only provider (like Strava)
    #[must_use]
    pub const fn activity_only() -> Self {
        Self {
            oauth: true,
            activities: true,
            sleep_tracking: false,
            recovery_metrics: false,
            health_metrics: false,
        }
    }

    /// Create capabilities for a full health provider (like Garmin, Fitbit)
    #[must_use]
    pub const fn full_health() -> Self {
        Self {
            oauth: true,
            activities: true,
            sleep_tracking: true,
            recovery_metrics: true,
            health_metrics: true,
        }
    }

    /// Create capabilities for a synthetic/test provider (no OAuth)
    #[must_use]
    pub const fn synthetic() -> Self {
        Self {
            oauth: false,
            activities: true,
            sleep_tracking: true,
            recovery_metrics: true,
            health_metrics: true,
        }
    }
}

/// Describes a provider's identity and capabilities
///
/// This trait is the primary interface for provider metadata. External provider
/// crates implement this trait to describe what they support.
pub trait ProviderDescriptor: Send + Sync {
    /// Unique provider identifier (e.g., "strava", "garmin", "whoop")
    ///
    /// This must be lowercase, alphanumeric, and match the provider name used
    /// in configuration and API requests.
    fn name(&self) -> &'static str;

    /// Human-readable display name (e.g., "Strava", "Garmin Connect", "WHOOP")
    fn display_name(&self) -> &'static str;

    /// Provider capabilities (OAuth, sleep tracking, etc.)
    fn capabilities(&self) -> ProviderCapabilities;

    /// OAuth endpoints if provider requires authentication
    ///
    /// Returns `None` for providers that don't require OAuth (e.g., synthetic provider).
    fn oauth_endpoints(&self) -> Option<OAuthEndpoints>;

    /// Base URL for provider API calls
    fn api_base_url(&self) -> &'static str;

    /// Default OAuth scopes to request
    ///
    /// Returns an empty slice for providers without OAuth.
    fn default_scopes(&self) -> &'static [&'static str];

    /// Whether this provider requires OAuth authentication
    fn requires_oauth(&self) -> bool {
        self.capabilities().oauth
    }

    /// Whether this provider supports sleep tracking
    fn supports_sleep(&self) -> bool {
        self.capabilities().sleep_tracking
    }

    /// Whether this provider supports recovery metrics
    fn supports_recovery(&self) -> bool {
        self.capabilities().recovery_metrics
    }

    /// Whether this provider supports health metrics
    fn supports_health(&self) -> bool {
        self.capabilities().health_metrics
    }

    /// Build a `ProviderConfig` from this descriptor
    ///
    /// Uses the descriptor's endpoints and scopes to create a configuration
    /// suitable for provider instantiation.
    fn to_config(&self) -> ProviderConfig {
        let (auth_url, token_url, revoke_url) = self.oauth_endpoints().map_or_else(
            || {
                // Synthetic/test providers use placeholder URLs
                (
                    format!("http://localhost/{}/auth", self.name()),
                    format!("http://localhost/{}/token", self.name()),
                    None,
                )
            },
            |endpoints| {
                (
                    endpoints.auth_url.to_owned(),
                    endpoints.token_url.to_owned(),
                    endpoints.revoke_url.map(str::to_owned),
                )
            },
        );

        ProviderConfig {
            name: self.name().to_owned(),
            auth_url,
            token_url,
            api_base_url: self.api_base_url().to_owned(),
            revoke_url,
            default_scopes: self
                .default_scopes()
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
        }
    }
}

/// Factory function type for creating provider instances
pub type ProviderFactoryFn = fn(ProviderConfig) -> Box<dyn FitnessProvider>;

/// Complete provider bundle for registration
///
/// Combines a provider descriptor with its factory function for easy registration.
/// External crates export a function that returns this bundle.
pub struct ProviderBundle {
    /// Provider descriptor with metadata and capabilities
    pub descriptor: Box<dyn ProviderDescriptor>,
    /// Factory function for creating provider instances
    pub factory: ProviderFactoryFn,
}

impl ProviderBundle {
    /// Create a new provider bundle
    pub fn new(descriptor: Box<dyn ProviderDescriptor>, factory: ProviderFactoryFn) -> Self {
        Self {
            descriptor,
            factory,
        }
    }

    /// Get the provider name from the descriptor
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.descriptor.name()
    }

    /// Create a provider instance using the factory and descriptor's config
    #[must_use]
    pub fn create_provider(&self) -> Box<dyn FitnessProvider> {
        let config = self.descriptor.to_config();
        (self.factory)(config)
    }

    /// Create a provider instance with custom config
    #[must_use]
    pub fn create_provider_with_config(&self, config: ProviderConfig) -> Box<dyn FitnessProvider> {
        (self.factory)(config)
    }
}

impl fmt::Debug for ProviderBundle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProviderBundle")
            .field("name", &self.descriptor.name())
            .field("display_name", &self.descriptor.display_name())
            .field("capabilities", &self.descriptor.capabilities())
            .finish_non_exhaustive()
    }
}

// ============================================================================
// Built-in Provider Descriptors
// ============================================================================

/// Strava provider descriptor
pub struct StravaDescriptor;

impl ProviderDescriptor for StravaDescriptor {
    fn name(&self) -> &'static str {
        "strava"
    }

    fn display_name(&self) -> &'static str {
        "Strava"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::activity_only()
    }

    fn oauth_endpoints(&self) -> Option<OAuthEndpoints> {
        Some(OAuthEndpoints {
            auth_url: "https://www.strava.com/oauth/authorize",
            token_url: "https://www.strava.com/oauth/token",
            revoke_url: Some("https://www.strava.com/oauth/deauthorize"),
        })
    }

    fn api_base_url(&self) -> &'static str {
        "https://www.strava.com/api/v3"
    }

    fn default_scopes(&self) -> &'static [&'static str] {
        &["activity:read_all"]
    }
}

/// Garmin provider descriptor
pub struct GarminDescriptor;

impl ProviderDescriptor for GarminDescriptor {
    fn name(&self) -> &'static str {
        "garmin"
    }

    fn display_name(&self) -> &'static str {
        "Garmin Connect"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::full_health()
    }

    fn oauth_endpoints(&self) -> Option<OAuthEndpoints> {
        Some(OAuthEndpoints {
            auth_url: "https://connect.garmin.com/oauthConfirm",
            token_url: "https://connectapi.garmin.com/oauth-service/oauth/access_token",
            revoke_url: Some("https://connectapi.garmin.com/oauth-service/oauth/revoke"),
        })
    }

    fn api_base_url(&self) -> &'static str {
        "https://apis.garmin.com/wellness-api/rest"
    }

    fn default_scopes(&self) -> &'static [&'static str] {
        // Garmin uses comma-separated scopes in some flows
        &[
            "activity:read",
            "sleep:read",
            "health:read",
            "user_metrics:read",
        ]
    }
}

/// Synthetic provider descriptor (for development/testing)
pub struct SyntheticDescriptor;

impl ProviderDescriptor for SyntheticDescriptor {
    fn name(&self) -> &'static str {
        "synthetic"
    }

    fn display_name(&self) -> &'static str {
        "Synthetic (Test)"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::synthetic()
    }

    fn oauth_endpoints(&self) -> Option<OAuthEndpoints> {
        None // Synthetic provider doesn't need OAuth
    }

    fn api_base_url(&self) -> &'static str {
        "http://localhost/synthetic/api"
    }

    fn default_scopes(&self) -> &'static [&'static str] {
        &[]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strava_descriptor() {
        let desc = StravaDescriptor;
        assert_eq!(desc.name(), "strava");
        assert_eq!(desc.display_name(), "Strava");
        assert!(desc.requires_oauth());
        assert!(!desc.supports_sleep());
        assert!(!desc.supports_recovery());

        let config = desc.to_config();
        assert_eq!(config.name, "strava");
        assert!(config.auth_url.contains("strava.com"));
    }

    #[test]
    fn test_garmin_descriptor() {
        let desc = GarminDescriptor;
        assert_eq!(desc.name(), "garmin");
        assert!(desc.requires_oauth());
        assert!(desc.supports_sleep());
        assert!(desc.supports_recovery());
        assert!(desc.supports_health());
    }

    #[test]
    fn test_synthetic_descriptor() {
        let desc = SyntheticDescriptor;
        assert_eq!(desc.name(), "synthetic");
        assert!(!desc.requires_oauth());
        assert!(desc.supports_sleep()); // Synthetic supports all for testing
        assert!(desc.oauth_endpoints().is_none());
    }

    #[test]
    fn test_provider_capabilities() {
        let activity = ProviderCapabilities::activity_only();
        assert!(activity.oauth);
        assert!(activity.activities);
        assert!(!activity.sleep_tracking);

        let full = ProviderCapabilities::full_health();
        assert!(full.oauth);
        assert!(full.sleep_tracking);
        assert!(full.recovery_metrics);

        let synthetic = ProviderCapabilities::synthetic();
        assert!(!synthetic.oauth);
        assert!(synthetic.activities);
    }
}
