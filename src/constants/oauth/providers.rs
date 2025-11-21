// ABOUTME: OAuth provider identifiers and validation functions
// ABOUTME: Centralizes provider name constants to eliminate hardcoded strings
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! OAuth provider constants
//!
//! Note: For dynamic provider discovery, prefer using `ProviderRegistry::supported_providers()`
//! instead of the static `all()` function. The registry respects feature flags and includes
//! externally registered providers.

/// Strava fitness provider identifier
#[cfg(feature = "provider-strava")]
pub const STRAVA: &str = "strava";

/// Fitbit fitness provider identifier (future)
pub const FITBIT: &str = "fitbit";

/// Garmin fitness provider identifier
#[cfg(feature = "provider-garmin")]
pub const GARMIN: &str = "garmin";

/// Synthetic fitness provider identifier (for testing)
#[cfg(feature = "provider-synthetic")]
pub const SYNTHETIC: &str = "synthetic";

/// Get statically-known OAuth providers
///
/// **Deprecated**: Use `crate::providers::get_supported_providers()` instead,
/// which respects feature flags and includes externally registered providers.
#[must_use]
#[deprecated(
    since = "0.2.0",
    note = "Use crate::providers::get_supported_providers() for dynamic provider discovery"
)]
pub const fn all() -> &'static [&'static str] {
    // This is a compile-time constant, so we include all potential providers
    // For runtime checking, use the registry
    &["strava", "fitbit", "garmin", "synthetic"]
}

/// Check if a provider is statically known
///
/// **Deprecated**: Use `crate::providers::is_provider_supported()` instead,
/// which respects feature flags and includes externally registered providers.
#[must_use]
#[deprecated(
    since = "0.2.0",
    note = "Use crate::providers::is_provider_supported() for dynamic provider validation"
)]
#[allow(deprecated)]
pub fn is_supported(provider: &str) -> bool {
    all().contains(&provider)
}

/// Strava default scopes (comma-separated as per Strava API requirements)
#[cfg(feature = "provider-strava")]
pub const STRAVA_DEFAULT_SCOPES: &str = "activity:read_all";

/// Fitbit default scopes
pub const FITBIT_DEFAULT_SCOPES: &str = "activity profile";

/// Garmin default scopes
#[cfg(feature = "provider-garmin")]
pub const GARMIN_DEFAULT_SCOPES: &str = "wellness:read,activities:read";
