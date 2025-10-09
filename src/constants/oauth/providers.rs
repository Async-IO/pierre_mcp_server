// ABOUTME: OAuth provider identifiers and validation functions
// ABOUTME: Centralizes provider name constants to eliminate hardcoded strings
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

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

/// Strava default scopes (comma-separated as per Strava API requirements)
pub const STRAVA_DEFAULT_SCOPES: &str = "activity:read_all";

/// Fitbit default scopes
pub const FITBIT_DEFAULT_SCOPES: &str = "activity profile";
