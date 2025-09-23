// ABOUTME: OAuth provider identifiers and validation functions
// ABOUTME: Centralizes provider name constants to eliminate hardcoded strings

//! OAuth provider constants

/// Strava fitness provider identifier
pub const STRAVA: &str = "strava";

/// Fitbit fitness provider identifier
pub const FITBIT: &str = "fitbit";

/// Get all supported OAuth providers
#[must_use]
pub const fn all() -> &'static [&'static str] {
    &[STRAVA, FITBIT]
}

/// Check if a provider is supported
#[must_use]
pub fn is_supported(provider: &str) -> bool {
    all().contains(&provider)
}

/// Strava default scopes
pub const STRAVA_DEFAULT_SCOPES: &str = "read,activity:read_all";

/// Fitbit default scopes
pub const FITBIT_DEFAULT_SCOPES: &str = "activity profile";
