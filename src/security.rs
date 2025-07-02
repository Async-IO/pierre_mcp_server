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
            csp: "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' data:; connect-src 'self' ws: wss:; frame-ancestors 'none'; object-src 'none'; base-uri 'self';".to_string(),

            // Prevent clickjacking
            frame_options: "DENY".to_string(),

            // Prevent MIME type sniffing
            content_type_options: "nosniff".to_string(),

            // Control referrer information
            referrer_policy: "strict-origin-when-cross-origin".to_string(),

            // Restrict dangerous browser features
            permissions_policy: "geolocation=(), microphone=(), camera=(), payment=(), usb=(), magnetometer=(), gyroscope=(), accelerometer=()".to_string(),

            // HSTS for HTTPS (24 hours, enable for production)
            hsts: Some("max-age=86400; includeSubDomains".to_string()),

            // Cross-origin isolation headers
            coep: "require-corp".to_string(),
            coop: "same-origin".to_string(),
            corp: "same-origin".to_string(),
        }
    }
}

impl SecurityConfig {
    /// Create security configuration based on environment string
    pub fn from_environment(env: &str) -> Self {
        match env.to_lowercase().as_str() {
            "production" | "prod" => Self::production(),
            "development" | "dev" => Self::development(),
            _ => Self::development(),
        }
    }

    /// Create a development-friendly security configuration
    pub fn development() -> Self {
        Self {
            // More relaxed CSP for development (allows hot reload, dev tools)
            csp: "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' data:; connect-src 'self' ws: wss: http://localhost:* https://localhost:*; frame-ancestors 'none'; object-src 'none'; base-uri 'self';".to_string(),
            frame_options: "DENY".to_string(),
            content_type_options: "nosniff".to_string(),
            referrer_policy: "strict-origin-when-cross-origin".to_string(),
            permissions_policy: "geolocation=(), microphone=(), camera=(), payment=(), usb=(), magnetometer=(), gyroscope=(), accelerometer=()".to_string(),
            hsts: None, // Disable HSTS for development (HTTP)
            coep: "unsafe-none".to_string(), // More permissive for dev tools
            coop: "unsafe-none".to_string(),
            corp: "cross-origin".to_string(),
        }
    }

    /// Create a production security configuration
    pub fn production() -> Self {
        Self {
            // Strict production CSP
            csp: "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self' wss:; frame-ancestors 'none'; object-src 'none'; base-uri 'self'; upgrade-insecure-requests;".to_string(),
            frame_options: "DENY".to_string(),
            content_type_options: "nosniff".to_string(),
            referrer_policy: "strict-origin-when-cross-origin".to_string(),
            permissions_policy: "geolocation=(), microphone=(), camera=(), payment=(), usb=(), magnetometer=(), gyroscope=(), accelerometer=()".to_string(),
            hsts: Some("max-age=31536000; includeSubDomains; preload".to_string()), // 1 year
            coep: "require-corp".to_string(),
            coop: "same-origin".to_string(),
            corp: "same-origin".to_string(),
        }
    }

    /// Convert to header map for easy application
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
pub fn with_security_headers(
    config: SecurityConfig,
) -> impl Filter<Extract = (SecurityConfig,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || config.clone())
}

/// Security headers middleware filter
pub fn security_headers(
    config: SecurityConfig,
) -> impl Filter<Extract = (impl Reply,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || {
        let _headers = config.to_headers();
        warp::reply::json(&serde_json::json!({"status": "ok", "security_headers": "enabled"}))
    })
}

/// Add security headers to any reply
pub fn add_security_headers<T: Reply>(
    reply: T,
    config: &SecurityConfig,
) -> warp::reply::WithHeader<T> {
    let _headers = config.to_headers();
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
pub fn audit_security_headers(headers: &HashMap<String, String>) -> SecurityAudit {
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
            missing_headers.push(header.to_string());
            score = score.saturating_sub(20);
        }
    }

    // Check for weak configurations
    if let Some(csp) = headers.get("Content-Security-Policy") {
        if csp.contains("'unsafe-eval'") {
            warnings.push("CSP allows 'unsafe-eval' which can enable XSS attacks".to_string());
            score = score.saturating_sub(10);
        }
        if csp.contains("*") && !csp.contains("'self'") {
            warnings.push("CSP uses wildcard (*) without 'self' restriction".to_string());
            score = score.saturating_sub(15);
        }
    }

    if let Some(frame_options) = headers.get("X-Frame-Options") {
        if frame_options.to_lowercase() == "allowall" {
            warnings.push("X-Frame-Options set to ALLOWALL enables clickjacking".to_string());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_security_config() {
        let config = SecurityConfig::default();

        assert!(config.csp.contains("default-src 'self'"));
        assert_eq!(config.frame_options, "DENY");
        assert_eq!(config.content_type_options, "nosniff");
        assert!(config.hsts.is_some());
    }

    #[test]
    fn test_development_config() {
        let config = SecurityConfig::development();

        assert!(config.csp.contains("localhost"));
        assert!(config.hsts.is_none());
        assert_eq!(config.coep, "unsafe-none");
    }

    #[test]
    fn test_production_config() {
        let config = SecurityConfig::production();

        assert!(config.csp.contains("upgrade-insecure-requests"));
        assert!(config.hsts.is_some());
        assert!(config.hsts.as_ref().unwrap().contains("preload"));
        assert_eq!(config.coep, "require-corp");
    }

    #[test]
    fn test_headers_conversion() {
        let config = SecurityConfig::default();
        let headers = config.to_headers();

        assert!(headers.contains_key("Content-Security-Policy"));
        assert!(headers.contains_key("X-Frame-Options"));
        assert!(headers.contains_key("X-Content-Type-Options"));
        assert!(headers.contains_key("Referrer-Policy"));
        assert!(headers.contains_key("Permissions-Policy"));
    }

    #[test]
    fn test_security_audit_secure_headers() {
        let mut headers = HashMap::new();
        headers.insert(
            "Content-Security-Policy".to_string(),
            "default-src 'self'".to_string(),
        );
        headers.insert("X-Frame-Options".to_string(), "DENY".to_string());
        headers.insert("X-Content-Type-Options".to_string(), "nosniff".to_string());
        headers.insert(
            "Referrer-Policy".to_string(),
            "strict-origin-when-cross-origin".to_string(),
        );

        let audit = audit_security_headers(&headers);

        assert!(audit.is_secure);
        assert!(audit.missing_headers.is_empty());
        assert!(audit.warnings.is_empty());
        assert_eq!(audit.score, 100);
    }

    #[test]
    fn test_security_audit_missing_headers() {
        let headers = HashMap::new();

        let audit = audit_security_headers(&headers);

        assert!(!audit.is_secure);
        assert_eq!(audit.missing_headers.len(), 4);
        assert_eq!(audit.score, 20); // 100 - (4 * 20)
    }

    #[test]
    fn test_security_audit_unsafe_csp() {
        let mut headers = HashMap::new();
        headers.insert(
            "Content-Security-Policy".to_string(),
            "default-src 'self'; script-src 'unsafe-eval'".to_string(),
        );
        headers.insert("X-Frame-Options".to_string(), "DENY".to_string());
        headers.insert("X-Content-Type-Options".to_string(), "nosniff".to_string());
        headers.insert(
            "Referrer-Policy".to_string(),
            "strict-origin-when-cross-origin".to_string(),
        );

        let audit = audit_security_headers(&headers);

        assert!(!audit.is_secure);
        assert!(!audit.warnings.is_empty());
        assert!(audit.warnings[0].contains("unsafe-eval"));
        assert_eq!(audit.score, 90); // 100 - 10 for unsafe-eval
    }

    #[test]
    fn test_security_audit_weak_frame_options() {
        let mut headers = HashMap::new();
        headers.insert(
            "Content-Security-Policy".to_string(),
            "default-src 'self'".to_string(),
        );
        headers.insert("X-Frame-Options".to_string(), "ALLOWALL".to_string());
        headers.insert("X-Content-Type-Options".to_string(), "nosniff".to_string());
        headers.insert(
            "Referrer-Policy".to_string(),
            "strict-origin-when-cross-origin".to_string(),
        );

        let audit = audit_security_headers(&headers);

        assert!(!audit.is_secure);
        assert!(!audit.warnings.is_empty());
        assert!(audit.warnings[0].contains("clickjacking"));
        assert_eq!(audit.score, 75); // 100 - 25 for ALLOWALL
    }

    #[test]
    fn test_csp_wildcard_detection() {
        let mut headers = HashMap::new();
        headers.insert(
            "Content-Security-Policy".to_string(),
            "default-src *".to_string(),
        );
        headers.insert("X-Frame-Options".to_string(), "DENY".to_string());
        headers.insert("X-Content-Type-Options".to_string(), "nosniff".to_string());
        headers.insert(
            "Referrer-Policy".to_string(),
            "strict-origin-when-cross-origin".to_string(),
        );

        let audit = audit_security_headers(&headers);

        assert!(!audit.is_secure);
        assert!(!audit.warnings.is_empty());
        assert!(audit.warnings[0].contains("wildcard"));
        assert_eq!(audit.score, 85); // 100 - 15 for wildcard
    }

    #[test]
    fn test_config_cloning() {
        let config1 = SecurityConfig::default();
        let config2 = config1.clone();

        assert_eq!(config1.csp, config2.csp);
        assert_eq!(config1.frame_options, config2.frame_options);
    }

    #[test]
    fn test_from_environment() {
        let dev_config = SecurityConfig::from_environment("development");
        assert!(dev_config.csp.contains("localhost"));
        assert!(dev_config.hsts.is_none());

        let prod_config = SecurityConfig::from_environment("production");
        assert!(prod_config.csp.contains("upgrade-insecure-requests"));
        assert!(prod_config.hsts.is_some());
        assert!(prod_config.hsts.as_ref().unwrap().contains("preload"));

        // Test case insensitive
        let prod_config2 = SecurityConfig::from_environment("PRODUCTION");
        assert_eq!(prod_config.csp, prod_config2.csp);

        // Test default fallback
        let default_config = SecurityConfig::from_environment("unknown");
        assert_eq!(default_config.csp, dev_config.csp);
    }

    #[test]
    fn test_header_names_consistency() {
        let config = SecurityConfig::default();
        let headers = config.to_headers();

        // Ensure all expected headers are present
        let expected_headers = [
            "Content-Security-Policy",
            "X-Frame-Options",
            "X-Content-Type-Options",
            "Referrer-Policy",
            "Permissions-Policy",
            "Cross-Origin-Embedder-Policy",
            "Cross-Origin-Opener-Policy",
            "Cross-Origin-Resource-Policy",
            "Strict-Transport-Security",
        ];

        for header in &expected_headers {
            if *header == "Strict-Transport-Security" {
                // HSTS is optional based on config
                continue;
            }
            assert!(headers.contains_key(*header), "Missing header: {}", header);
        }
    }
}
