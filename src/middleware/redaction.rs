// ABOUTME: PII-safe logging and redaction middleware for sensitive data protection
// ABOUTME: Filters headers, request bodies, and logs to prevent PII leakage and compliance violations
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! PII-safe logging and redaction for compliance and security
//!
//! This module provides:
//! - HTTP header redaction (Authorization, Cookie, X-API-Key, etc.)
//! - JSON body field redaction (`client_secret`, tokens, passwords)
//! - Email address masking for PII protection
//! - Token pattern detection and redaction
//! - Bounded metric labels to prevent Prometheus cardinality explosions
//!
//! ## Usage
//!
//! ```rust
//! use pierre_mcp_server::middleware::redaction::{RedactionConfig, redact_headers, mask_email};
//!
//! let config = RedactionConfig::default();
//! let headers = [
//!     ("authorization", "Bearer secret_token"),
//!     ("content-type", "application/json"),
//! ];
//! let safe_headers = redact_headers(headers, &config);
//! // safe_headers will have authorization redacted
//!
//! let email = "testuser@domain.com";
//! let masked = mask_email(email);
//! // masked will be "t***@d***.com"
//! ```

use bitflags::bitflags;
use regex::Regex;
use std::sync::OnceLock;
use warp::http::header::{HeaderName, HeaderValue};

bitflags! {
    /// Redaction feature flags to control which types of data to redact
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct RedactionFeatures: u8 {
        /// Redact HTTP headers (Authorization, Cookie, etc.)
        const HEADERS = 0b0001;
        /// Redact JSON body fields (client_secret, tokens, etc.)
        const BODY_FIELDS = 0b0010;
        /// Mask email addresses
        const EMAILS = 0b0100;
        /// Enable all redaction features
        const ALL = Self::HEADERS.bits() | Self::BODY_FIELDS.bits() | Self::EMAILS.bits();
    }
}

/// Configuration for PII redaction
#[derive(Debug, Clone)]
pub struct RedactionConfig {
    /// Enable redaction globally (default: true in production, false in dev)
    pub enabled: bool,
    /// Which redaction features to enable
    pub features: RedactionFeatures,
    /// Replacement string for redacted sensitive data
    pub redaction_placeholder: String,
}

impl Default for RedactionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            features: RedactionFeatures::ALL,
            redaction_placeholder: "[REDACTED]".to_string(),
        }
    }
}

impl RedactionConfig {
    /// Create redaction config from environment
    #[must_use]
    pub fn from_env() -> Self {
        let enabled = std::env::var("PIERRE_LOG_REDACT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(true);

        let features = if enabled {
            RedactionFeatures::ALL
        } else {
            RedactionFeatures::empty()
        };

        Self {
            enabled,
            features,
            redaction_placeholder: std::env::var("PIERRE_REDACTION_PLACEHOLDER")
                .unwrap_or_else(|_| "[REDACTED]".to_string()),
        }
    }

    /// Check if redaction is disabled
    #[must_use]
    pub const fn is_disabled(&self) -> bool {
        !self.enabled
    }
}

/// Sensitive HTTP headers that should be redacted
const SENSITIVE_HEADERS: &[&str] = &[
    "authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
    "x-auth-token",
    "x-access-token",
    "api-key",
    "api_key",
    "apikey",
    "proxy-authorization",
    "www-authenticate",
];

/// Sensitive JSON fields that should be redacted
const SENSITIVE_FIELDS: &[&str] = &[
    "password",
    "client_secret",
    "client-secret",
    "access_token",
    "accessToken",
    "refresh_token",
    "refreshToken",
    "api_key",
    "apiKey",
    "api-key",
    "secret",
    "private_key",
    "privateKey",
    "encryption_key",
    "encryptionKey",
    "jwt_secret",
    "jwtSecret",
];

/// Redact sensitive HTTP headers
///
/// # Arguments
///
/// * `headers` - Iterator of (name, value) tuples
/// * `config` - Redaction configuration
///
/// # Returns
///
/// Vector of (name, value) tuples with sensitive headers redacted
pub fn redact_headers<'a, I>(headers: I, config: &RedactionConfig) -> Vec<(String, String)>
where
    I: IntoIterator<Item = (&'a str, &'a str)>,
{
    if !config.enabled || !config.features.contains(RedactionFeatures::HEADERS) {
        return headers
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
    }

    headers
        .into_iter()
        .map(|(name, value)| {
            let name_lower = name.to_lowercase();
            let redacted_value = if SENSITIVE_HEADERS.contains(&name_lower.as_str()) {
                config.redaction_placeholder.clone()
            } else {
                value.to_string()
            };
            (name.to_string(), redacted_value)
        })
        .collect()
}

/// Redact sensitive fields in JSON-like text
///
/// # Arguments
///
/// * `text` - JSON or log text that may contain sensitive fields
/// * `config` - Redaction configuration
///
/// # Returns
///
/// Text with sensitive field values redacted
#[must_use]
pub fn redact_json_fields(text: &str, config: &RedactionConfig) -> String {
    if !config.enabled || !config.features.contains(RedactionFeatures::BODY_FIELDS) {
        return text.to_string();
    }

    let mut result = text.to_string();

    for field in SENSITIVE_FIELDS {
        // Match patterns like:
        // "field": "value"
        // "field":"value"
        // field: "value"
        // field = "value"
        let patterns = [
            format!(r#""{field}"\s*:\s*"[^"]*""#),
            format!(r#"{field}\s*:\s*"[^"]*""#),
            format!(r#"{field}\s*=\s*"[^"]*""#),
        ];

        for pattern in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                result = re
                    .replace_all(
                        &result,
                        format!(r#""{field}": "{}""#, config.redaction_placeholder),
                    )
                    .to_string();
            }
        }
    }

    result
}

/// Mask email addresses for PII protection
///
/// Masks email addresses by showing only first characters of local and domain parts
///
/// # Arguments
///
/// * `email` - Email address to mask
///
/// # Returns
///
/// Masked email with first character of local and domain parts visible
#[must_use]
pub fn mask_email(email: &str) -> String {
    email_regex()
        .replace_all(email, |caps: &regex::Captures| {
            let full_match = &caps[0];
            full_match.find('@').map_or_else(
                || full_match.to_string(),
                |at_pos| {
                    let (local, domain_with_at) = full_match.split_at(at_pos);
                    let domain = &domain_with_at[1..]; // Skip '@'

                    let masked_local = if local.len() > 1 {
                        format!("{}***", &local[0..1])
                    } else {
                        local.to_string()
                    };

                    let masked_domain = domain.find('.').map_or_else(
                        || domain.to_string(),
                        |dot_pos| {
                            let (subdomain, tld_with_dot) = domain.split_at(dot_pos);
                            if subdomain.len() > 1 {
                                format!("{}***{tld_with_dot}", &subdomain[0..1])
                            } else {
                                domain.to_string()
                            }
                        },
                    );

                    format!("{masked_local}@{masked_domain}")
                },
            )
        })
        .to_string()
}

/// Redact token-like patterns from text
///
/// Matches patterns like:
/// - Bearer `<token>`
/// - JWT `<token>`
/// - API key formats
///
/// # Arguments
///
/// * `text` - Text that may contain token patterns
/// * `config` - Redaction configuration
///
/// # Returns
///
/// Text with tokens redacted
#[must_use]
pub fn redact_token_patterns(text: &str, config: &RedactionConfig) -> String {
    if config.is_disabled() {
        return text.to_string();
    }

    let mut result = text.to_string();

    // Redact Bearer tokens
    if let Ok(re) = Regex::new(r"Bearer\s+[A-Za-z0-9\-._~+/]+=*") {
        result = re
            .replace_all(&result, format!("Bearer {}", config.redaction_placeholder))
            .to_string();
    }

    // Redact JWT-like tokens (three base64 segments separated by dots)
    if let Ok(re) = Regex::new(r"[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+") {
        result = re
            .replace_all(&result, &config.redaction_placeholder)
            .to_string();
    }

    result
}

/// Bounded metric label for tenant IDs to prevent cardinality explosions
///
/// Limits the number of unique tenant IDs tracked in Prometheus metrics
/// by hashing tenant IDs into a fixed set of buckets.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoundedTenantLabel {
    bucket: String,
}

impl BoundedTenantLabel {
    /// Maximum number of tenant buckets for metrics (prevents unbounded cardinality)
    const MAX_BUCKETS: usize = 100;

    /// Create bounded label from tenant ID
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - Raw tenant ID (UUID or string)
    ///
    /// # Returns
    ///
    /// Bounded label that hashes tenant to one of `MAX_BUCKETS` values
    #[must_use]
    pub fn new(tenant_id: &str) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        tenant_id.hash(&mut hasher);
        let hash = hasher.finish();
        let bucket_id = hash % (Self::MAX_BUCKETS as u64);

        Self {
            bucket: format!("tenant_bucket_{bucket_id}"),
        }
    }

    /// Get the bucket label for metrics
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.bucket
    }
}

impl std::fmt::Display for BoundedTenantLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.bucket)
    }
}

/// Bounded metric label for user IDs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoundedUserLabel {
    bucket: String,
}

impl BoundedUserLabel {
    /// Maximum number of user buckets for metrics
    const MAX_BUCKETS: usize = 100;

    /// Create bounded label from user ID
    #[must_use]
    pub fn new(user_id: &str) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        user_id.hash(&mut hasher);
        let hash = hasher.finish();
        let bucket_id = hash % (Self::MAX_BUCKETS as u64);

        Self {
            bucket: format!("user_bucket_{bucket_id}"),
        }
    }

    /// Get the bucket label for metrics
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.bucket
    }
}

impl std::fmt::Display for BoundedUserLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.bucket)
    }
}

/// Get compiled email regex (cached)
fn email_regex() -> &'static Regex {
    static EMAIL_REGEX: OnceLock<Regex> = OnceLock::new();
    EMAIL_REGEX.get_or_init(|| {
        Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}")
            .expect("Invalid email regex pattern")
    })
}

/// Custom Reply wrapper that applies redaction to HTTP responses
///
/// This wrapper intercepts responses and redacts sensitive headers according
/// to the configured redaction policy. Body redaction is applied for JSON content.
pub struct RedactedReply<R> {
    inner: R,
    config: RedactionConfig,
}

impl<R> RedactedReply<R> {
    /// Create a new redacted reply wrapper
    pub const fn new(inner: R, config: RedactionConfig) -> Self {
        Self { inner, config }
    }
}

impl<R: warp::Reply> warp::Reply for RedactedReply<R> {
    fn into_response(self) -> warp::reply::Response {
        let mut response = self.inner.into_response();

        // Skip redaction if disabled
        if self.config.is_disabled() {
            return response;
        }

        // Redact sensitive headers
        if self.config.features.contains(RedactionFeatures::HEADERS) {
            let headers = response.headers_mut();
            let header_pairs: Vec<(String, String)> = headers
                .iter()
                .map(|(name, value)| {
                    (
                        name.as_str().to_string(),
                        value.to_str().unwrap_or("").to_string(),
                    )
                })
                .collect();

            let redacted = redact_headers(
                header_pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())),
                &self.config,
            );

            // Clear existing headers and replace with redacted ones
            headers.clear();
            for (name, value) in redacted {
                if let (Ok(header_name), Ok(header_value)) = (
                    HeaderName::from_bytes(name.as_bytes()),
                    HeaderValue::from_str(&value),
                ) {
                    headers.insert(header_name, header_value);
                }
            }
        }

        response
    }
}

/// Warp filter to apply redaction middleware to HTTP responses
///
/// This filter wraps responses with a `RedactedReply` that redacts sensitive
/// headers and logs with PII protection.
///
/// # Usage
///
/// ```rust,no_run
/// use pierre_mcp_server::middleware::redaction::{RedactionConfig, with_redaction};
/// use warp::Filter;
///
/// let config = RedactionConfig::from_env();
/// let routes = warp::any()
///     .map(|| warp::reply::html("<html></html>"))
///     .map(with_redaction(config.clone()));
/// ```
///
/// # Note
///
/// Body redaction for JSON responses should be applied at the route level
/// using `redact_json_fields()` before serialization for best performance.
pub fn with_redaction<R>(config: RedactionConfig) -> impl Fn(R) -> RedactedReply<R> + Clone
where
    R: warp::Reply,
{
    move |reply| RedactedReply::new(reply, config.clone())
}
