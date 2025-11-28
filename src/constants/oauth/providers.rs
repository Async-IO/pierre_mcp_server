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

/// Fitbit fitness provider identifier
#[cfg(feature = "provider-fitbit")]
pub const FITBIT: &str = "fitbit";

/// Garmin fitness provider identifier
#[cfg(feature = "provider-garmin")]
pub const GARMIN: &str = "garmin";

/// Terra unified fitness provider identifier (150+ wearables)
#[cfg(feature = "provider-terra")]
pub const TERRA: &str = "terra";

/// WHOOP fitness provider identifier
#[cfg(feature = "provider-whoop")]
pub const WHOOP: &str = "whoop";

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
    &["strava", "fitbit", "garmin", "whoop", "synthetic"]
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

/// Fitbit default scopes (space-separated as per Fitbit API requirements)
#[cfg(feature = "provider-fitbit")]
pub const FITBIT_DEFAULT_SCOPES: &str = "activity profile sleep heartrate weight";

/// Garmin default scopes
#[cfg(feature = "provider-garmin")]
pub const GARMIN_DEFAULT_SCOPES: &str = "wellness:read,activities:read";

/// Terra default scopes (data types)
#[cfg(feature = "provider-terra")]
pub const TERRA_DEFAULT_SCOPES: &str = "activity,sleep,body,daily,nutrition";

/// WHOOP default scopes (space-separated as per WHOOP API requirements)
/// - `offline`: Required for refresh tokens
/// - `read:profile`: Access to user profile information
/// - `read:body_measurement`: Access to height, weight, max heart rate
/// - `read:workout`: Access to workout/activity data
/// - `read:sleep`: Access to sleep data
/// - `read:recovery`: Access to recovery scores
/// - `read:cycles`: Access to cycle data (strain, recovery aggregation)
#[cfg(feature = "provider-whoop")]
pub const WHOOP_DEFAULT_SCOPES: &str =
    "offline read:profile read:body_measurement read:workout read:sleep read:recovery read:cycles";
