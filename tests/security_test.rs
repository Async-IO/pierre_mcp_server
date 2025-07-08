// Integration tests for security.rs module
// Tests for security headers configuration and validation

use pierre_mcp_server::security::{audit_security_headers, SecurityConfig};
use std::collections::HashMap;

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
        "Content-Security-Policy".into(),
        "default-src 'self'".into(),
    );
    headers.insert("X-Frame-Options".into(), "DENY".into());
    headers.insert("X-Content-Type-Options".into(), "nosniff".into());
    headers.insert(
        "Referrer-Policy".into(),
        "strict-origin-when-cross-origin".into(),
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
        "Content-Security-Policy".into(),
        "default-src 'self'; script-src 'unsafe-eval'".into(),
    );
    headers.insert("X-Frame-Options".into(), "DENY".into());
    headers.insert("X-Content-Type-Options".into(), "nosniff".into());
    headers.insert(
        "Referrer-Policy".into(),
        "strict-origin-when-cross-origin".into(),
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
        "Content-Security-Policy".into(),
        "default-src 'self'".into(),
    );
    headers.insert("X-Frame-Options".into(), "ALLOWALL".into());
    headers.insert("X-Content-Type-Options".into(), "nosniff".into());
    headers.insert(
        "Referrer-Policy".into(),
        "strict-origin-when-cross-origin".into(),
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
    headers.insert("Content-Security-Policy".into(), "default-src *".into());
    headers.insert("X-Frame-Options".into(), "DENY".into());
    headers.insert("X-Content-Type-Options".into(), "nosniff".into());
    headers.insert(
        "Referrer-Policy".into(),
        "strict-origin-when-cross-origin".into(),
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
    let config2 = &config1;

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
        assert!(headers.contains_key(*header), "Missing header: {header}");
    }
}
