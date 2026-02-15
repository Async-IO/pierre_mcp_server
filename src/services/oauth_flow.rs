// ABOUTME: OAuth flow business logic extracted from route handlers
// ABOUTME: State parsing, redirect URL validation, and PKCE-related utilities
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use tracing::warn;

/// Allowed URL schemes for OAuth mobile redirect URLs
const ALLOWED_SCHEMES: &[&str] = &["pierre://", "exp://", "http://localhost", "https://"];

/// Extract mobile redirect URL from the OAuth state string
///
/// State format: `{user_id}:{random}:{base64_redirect_url}`
/// The redirect URL is embedded as base64-encoded data in the third segment.
///
/// Returns `None` if the state doesn't contain a redirect URL or decoding fails.
#[must_use]
pub fn extract_mobile_redirect_from_state(state: &str) -> Option<String> {
    let parts: Vec<&str> = state.splitn(3, ':').collect();
    parts
        .get(2)
        .filter(|s| !s.is_empty())
        .and_then(|encoded| decode_and_validate_redirect_url(encoded))
}

/// Decode a base64-encoded redirect URL and validate its scheme
///
/// Only URLs with allowed schemes are accepted to prevent open redirect attacks.
/// Allowed schemes: `pierre://`, `exp://`, `http://localhost`, `https://`
///
/// Returns `None` if decoding fails or the URL scheme is not allowed.
#[must_use]
pub fn decode_and_validate_redirect_url(encoded: &str) -> Option<String> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

    URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|e| {
            warn!("Failed to decode base64 redirect URL: {}", e);
            e
        })
        .ok()
        .and_then(|bytes| {
            String::from_utf8(bytes)
                .map_err(|e| {
                    warn!("Failed to decode redirect URL as UTF-8: {}", e);
                    e
                })
                .ok()
        })
        .and_then(|url| {
            if is_allowed_redirect_scheme(&url) {
                Some(url)
            } else {
                warn!("Invalid redirect URL scheme in OAuth state: {}", url);
                None
            }
        })
}

/// Check if a URL uses an allowed redirect scheme
///
/// Prevents open redirect attacks by restricting redirect URLs to known-safe schemes.
fn is_allowed_redirect_scheme(url: &str) -> bool {
    ALLOWED_SCHEMES.iter().any(|scheme| url.starts_with(scheme))
}
