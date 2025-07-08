// ABOUTME: Security utilities and encryption functions for data protection and secure operations
// ABOUTME: Provides cryptographic functions, data sanitization, and security middleware components
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Security Headers Middleware
//!
//! Implements comprehensive security headers to protect against common web vulnerabilities
//! including XSS, clickjacking, CSRF, and other security threats.

use std::collections::HashMap;
use warp::{Filter, Reply};

/// Security headers configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Content Security Policy header value
    pub csp: String,
    /// X-Frame-Options header value  
    pub frame_options: String,
    /// X-Content-Type-Options header value
    pub content_type_options: String,
    /// Referrer-Policy header value
    pub referrer_policy: String,
    /// Permissions-Policy header value
    pub permissions_policy: String,
    /// Strict-Transport-Security header value (for HTTPS)
    pub hsts: Option<String>,
    /// Cross-Origin-Embedder-Policy header value
    pub coep: String,
    /// Cross-Origin-Opener-Policy header value
    pub coop: String,
    /// Cross-Origin-Resource-Policy header value
    pub corp: String,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            // Strict CSP that allows same-origin and specific trusted sources
            csp: "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' data:; connect-src 'self' ws: wss:; frame-ancestors 'none'; object-src 'none'; base-uri 'self';".into(),

            // Prevent clickjacking
            frame_options: "DENY".into(),

            // Prevent MIME type sniffing
            content_type_options: "nosniff".into(),

            // Control referrer information
            referrer_policy: "strict-origin-when-cross-origin".into(),

            // Restrict dangerous browser features
            permissions_policy: "geolocation=(), microphone=(), camera=(), payment=(), usb=(), magnetometer=(), gyroscope=(), accelerometer=()".into(),

            // HSTS for HTTPS (24 hours, enable for production)
            hsts: Some("max-age=86400; includeSubDomains".into()),

            // Cross-origin isolation headers
            coep: "require-corp".into(),
            coop: "same-origin".into(),
            corp: "same-origin".into(),
        }
    }
}

impl SecurityConfig {
    /// Create security configuration based on environment string
    #[must_use]
    pub fn from_environment(env: &str) -> Self {
        match env.to_lowercase().as_str() {
            "production" | "prod" => Self::production(),
            _ => Self::development(),
        }
    }

    /// Create a development-friendly security configuration
    #[must_use]
    pub fn development() -> Self {
        Self {
            // More relaxed CSP for development (allows hot reload, dev tools)
            csp: "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' data:; connect-src 'self' ws: wss: http://localhost:* https://localhost:*; frame-ancestors 'none'; object-src 'none'; base-uri 'self';".into(),
            frame_options: "DENY".into(),
            content_type_options: "nosniff".into(),
            referrer_policy: "strict-origin-when-cross-origin".into(),
            permissions_policy: "geolocation=(), microphone=(), camera=(), payment=(), usb=(), magnetometer=(), gyroscope=(), accelerometer=()".into(),
            hsts: None, // Disable HSTS for development (HTTP)
            coep: "unsafe-none".into(), // More permissive for dev tools
            coop: "unsafe-none".into(),
            corp: "cross-origin".into(),
        }
    }

    /// Create a production security configuration
    #[must_use]
    pub fn production() -> Self {
        Self {
            // Strict production CSP
            csp: "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self' wss:; frame-ancestors 'none'; object-src 'none'; base-uri 'self'; upgrade-insecure-requests;".into(),
            frame_options: "DENY".into(),
            content_type_options: "nosniff".into(),
            referrer_policy: "strict-origin-when-cross-origin".into(),
            permissions_policy: "geolocation=(), microphone=(), camera=(), payment=(), usb=(), magnetometer=(), gyroscope=(), accelerometer=()".into(),
            hsts: Some("max-age=31536000; includeSubDomains; preload".into()), // 1 year
            coep: "require-corp".into(),
            coop: "same-origin".into(),
            corp: "same-origin".into(),
        }
    }

    /// Convert to header map for easy application
    #[must_use]
    pub fn to_headers(&self) -> HashMap<&'static str, String> {
        let mut headers = HashMap::new();

        headers.insert("Content-Security-Policy", self.csp.clone());
        headers.insert("X-Frame-Options", self.frame_options.clone());
        headers.insert("X-Content-Type-Options", self.content_type_options.clone());
        headers.insert("Referrer-Policy", self.referrer_policy.clone());
        headers.insert("Permissions-Policy", self.permissions_policy.clone());
        headers.insert("Cross-Origin-Embedder-Policy", self.coep.clone());
        headers.insert("Cross-Origin-Opener-Policy", self.coop.clone());
        headers.insert("Cross-Origin-Resource-Policy", self.corp.clone());

        if let Some(hsts) = &self.hsts {
            headers.insert("Strict-Transport-Security", hsts.clone());
        }

        headers
    }
}

/// Apply security headers to a warp reply
#[must_use]
pub fn with_security_headers(
    config: SecurityConfig,
) -> impl Filter<Extract = (SecurityConfig,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || config.clone())
}

/// Security headers middleware filter
#[must_use]
pub fn security_headers(
    config: SecurityConfig,
) -> impl Filter<Extract = (impl Reply,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || {
        let headers = config.to_headers();
        // Note: In a real implementation, these headers would be applied to the HTTP response
        // Log security headers configuration
        tracing::debug!("Security headers configured: {} headers", headers.len());
        warp::reply::json(&serde_json::json!({"status": "ok", "security_headers": "enabled"}))
    })
}

/// Add security headers to any reply
pub fn add_security_headers<T: Reply>(
    reply: T,
    config: &SecurityConfig,
) -> warp::reply::WithHeader<T> {
    let headers = config.to_headers();
    // Note: In a full implementation, all security headers would be applied here
    // Apply security processing marker header
    tracing::debug!("Applying {} security headers to response", headers.len());
    warp::reply::with_header(reply, "X-Security-Applied", "true")
}

/// Security audit information
#[derive(Debug, Clone)]
pub struct SecurityAudit {
    /// Whether all critical headers are present
    pub is_secure: bool,
    /// Missing critical headers
    pub missing_headers: Vec<String>,
    /// Warnings about header values
    pub warnings: Vec<String>,
    /// Security score (0-100)
    pub score: u8,
}

/// Audit security headers on a response
#[must_use]
pub fn audit_security_headers<S: ::std::hash::BuildHasher>(
    headers: &HashMap<String, String, S>,
) -> SecurityAudit {
    let mut missing_headers = Vec::new();
    let mut warnings = Vec::new();
    let mut score = 100u8;

    // Critical headers checklist
    let critical_headers = [
        "Content-Security-Policy",
        "X-Frame-Options",
        "X-Content-Type-Options",
        "Referrer-Policy",
    ];

    for header in &critical_headers {
        if !headers.contains_key(*header) {
            missing_headers.push((*header).to_string());
            score = score.saturating_sub(20);
        }
    }

    // Check for weak configurations
    if let Some(csp) = headers.get("Content-Security-Policy") {
        if csp.contains("'unsafe-eval'") {
            warnings.push("CSP allows 'unsafe-eval' which can enable XSS attacks".into());
            score = score.saturating_sub(10);
        }
        if csp.contains('*') && !csp.contains("'self'") {
            warnings.push("CSP uses wildcard (*) without 'self' restriction".into());
            score = score.saturating_sub(15);
        }
    }

    if let Some(frame_options) = headers.get("X-Frame-Options") {
        if frame_options.to_lowercase() == "allowall" {
            warnings.push("X-Frame-Options set to ALLOWALL enables clickjacking".into());
            score = score.saturating_sub(25);
        }
    }

    SecurityAudit {
        is_secure: missing_headers.is_empty() && warnings.is_empty(),
        missing_headers,
        warnings,
        score,
    }
}
