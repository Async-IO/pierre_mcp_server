// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Integration tests for security headers middleware

use pierre_mcp_server::security::{audit_security_headers, SecurityConfig};
use std::collections::HashMap;

#[test]
fn test_security_headers_configuration() {
    let config = SecurityConfig::development();
    let headers = config.to_headers();

    // Check that critical security headers are present
    assert!(
        headers.contains_key("Content-Security-Policy"),
        "Missing Content-Security-Policy header"
    );
    assert!(
        headers.contains_key("X-Frame-Options"),
        "Missing X-Frame-Options header"
    );
    assert!(
        headers.contains_key("X-Content-Type-Options"),
        "Missing X-Content-Type-Options header"
    );
    assert!(
        headers.contains_key("Referrer-Policy"),
        "Missing Referrer-Policy header"
    );

    // Check specific header values
    let csp = headers.get("Content-Security-Policy").unwrap();
    assert!(
        csp.contains("default-src 'self'"),
        "CSP doesn't contain default-src 'self'"
    );
    assert!(
        csp.contains("localhost"),
        "Development CSP should allow localhost"
    );

    let frame_options = headers.get("X-Frame-Options").unwrap();
    assert_eq!(frame_options, "DENY", "X-Frame-Options should be DENY");

    let content_type_options = headers.get("X-Content-Type-Options").unwrap();
    assert_eq!(
        content_type_options, "nosniff",
        "X-Content-Type-Options should be nosniff"
    );
}

#[test]
fn test_development_vs_production_config() {
    let dev_config = SecurityConfig::development();
    let prod_config = SecurityConfig::production();

    // Development should not have HSTS
    assert!(
        dev_config.hsts.is_none(),
        "Development config should not have HSTS"
    );

    // Production should have HSTS
    assert!(
        prod_config.hsts.is_some(),
        "Production config should have HSTS"
    );

    // Development should be more permissive
    assert!(
        dev_config.csp.contains("localhost"),
        "Development CSP should allow localhost"
    );
    assert!(
        dev_config.coep == "unsafe-none",
        "Development COEP should be unsafe-none"
    );

    // Production should be stricter
    assert!(
        prod_config.csp.contains("upgrade-insecure-requests"),
        "Production CSP should upgrade insecure requests"
    );
    assert!(
        prod_config.coep == "require-corp",
        "Production COEP should be require-corp"
    );
}

#[test]
fn test_security_audit_functionality() {
    // Test secure headers
    let mut secure_headers = HashMap::new();
    secure_headers.insert(
        "Content-Security-Policy".to_string(),
        "default-src 'self'".to_string(),
    );
    secure_headers.insert("X-Frame-Options".to_string(), "DENY".to_string());
    secure_headers.insert("X-Content-Type-Options".to_string(), "nosniff".to_string());
    secure_headers.insert(
        "Referrer-Policy".to_string(),
        "strict-origin-when-cross-origin".to_string(),
    );

    let audit = audit_security_headers(&secure_headers);
    assert!(audit.is_secure, "Secure headers should pass audit");
    assert_eq!(audit.score, 100, "Secure headers should get perfect score");
    assert!(
        audit.missing_headers.is_empty(),
        "No headers should be missing"
    );
    assert!(audit.warnings.is_empty(), "No warnings should be generated");

    // Test missing headers
    let empty_headers = HashMap::new();
    let audit = audit_security_headers(&empty_headers);
    assert!(!audit.is_secure, "Empty headers should fail audit");
    assert_eq!(
        audit.missing_headers.len(),
        4,
        "Should report 4 missing critical headers"
    );
    assert_eq!(audit.score, 20, "Should lose 20 points per missing header");

    // Test unsafe CSP
    let mut unsafe_headers = HashMap::new();
    unsafe_headers.insert(
        "Content-Security-Policy".to_string(),
        "default-src 'self'; script-src 'unsafe-eval'".to_string(),
    );
    unsafe_headers.insert("X-Frame-Options".to_string(), "DENY".to_string());
    unsafe_headers.insert("X-Content-Type-Options".to_string(), "nosniff".to_string());
    unsafe_headers.insert(
        "Referrer-Policy".to_string(),
        "strict-origin-when-cross-origin".to_string(),
    );

    let audit = audit_security_headers(&unsafe_headers);
    assert!(!audit.is_secure, "Unsafe CSP should fail audit");
    assert!(
        !audit.warnings.is_empty(),
        "Should generate warnings for unsafe CSP"
    );
    assert!(
        audit.warnings[0].contains("unsafe-eval"),
        "Warning should mention unsafe-eval"
    );
    assert_eq!(audit.score, 90, "Should lose 10 points for unsafe-eval");
}

#[test]
fn test_config_header_conversion() {
    let config = SecurityConfig::default();
    let headers = config.to_headers();

    // Check that all expected headers are present
    let expected_headers = [
        "Content-Security-Policy",
        "X-Frame-Options",
        "X-Content-Type-Options",
        "Referrer-Policy",
        "Permissions-Policy",
        "Cross-Origin-Embedder-Policy",
        "Cross-Origin-Opener-Policy",
        "Cross-Origin-Resource-Policy",
    ];

    for header in &expected_headers {
        assert!(headers.contains_key(*header), "Missing header: {}", header);
        assert!(
            !headers[*header].is_empty(),
            "Header {} should not be empty",
            header
        );
    }

    // HSTS should be included in default config
    assert!(
        headers.contains_key("Strict-Transport-Security"),
        "Missing HSTS header"
    );
}

#[test]
fn test_csp_policies() {
    let dev_config = SecurityConfig::development();
    let prod_config = SecurityConfig::production();

    // Development CSP should allow unsafe-inline and unsafe-eval for dev tools
    assert!(
        dev_config.csp.contains("'unsafe-inline'"),
        "Dev CSP should allow unsafe-inline"
    );
    assert!(
        dev_config.csp.contains("'unsafe-eval'"),
        "Dev CSP should allow unsafe-eval"
    );
    assert!(
        dev_config.csp.contains("localhost"),
        "Dev CSP should allow localhost"
    );

    // Production CSP should be stricter
    assert!(
        !prod_config.csp.contains("'unsafe-eval'"),
        "Prod CSP should not allow unsafe-eval"
    );
    assert!(
        prod_config.csp.contains("upgrade-insecure-requests"),
        "Prod CSP should upgrade insecure requests"
    );
}

#[test]
fn test_permissions_policy() {
    let config = SecurityConfig::default();

    // Should disable dangerous features
    assert!(
        config.permissions_policy.contains("geolocation=()"),
        "Should disable geolocation"
    );
    assert!(
        config.permissions_policy.contains("microphone=()"),
        "Should disable microphone"
    );
    assert!(
        config.permissions_policy.contains("camera=()"),
        "Should disable camera"
    );
    assert!(
        config.permissions_policy.contains("payment=()"),
        "Should disable payment"
    );
}

#[test]
fn test_cross_origin_policies() {
    let dev_config = SecurityConfig::development();
    let prod_config = SecurityConfig::production();

    // Development should be more permissive for cross-origin
    assert_eq!(
        dev_config.coep, "unsafe-none",
        "Dev COEP should be unsafe-none"
    );
    assert_eq!(
        dev_config.coop, "unsafe-none",
        "Dev COOP should be unsafe-none"
    );
    assert_eq!(
        dev_config.corp, "cross-origin",
        "Dev CORP should be cross-origin"
    );

    // Production should be restrictive
    assert_eq!(
        prod_config.coep, "require-corp",
        "Prod COEP should be require-corp"
    );
    assert_eq!(
        prod_config.coop, "same-origin",
        "Prod COOP should be same-origin"
    );
    assert_eq!(
        prod_config.corp, "same-origin",
        "Prod CORP should be same-origin"
    );
}

#[test]
fn test_hsts_configuration() {
    let dev_config = SecurityConfig::development();
    let prod_config = SecurityConfig::production();

    // Development should not have HSTS (HTTP)
    assert!(
        dev_config.hsts.is_none(),
        "Development should not have HSTS"
    );

    // Production should have strong HSTS
    let prod_hsts = prod_config.hsts.as_ref().unwrap();
    assert!(
        prod_hsts.contains("max-age=31536000"),
        "Prod HSTS should have 1 year max-age"
    );
    assert!(
        prod_hsts.contains("includeSubDomains"),
        "Prod HSTS should include subdomains"
    );
    assert!(
        prod_hsts.contains("preload"),
        "Prod HSTS should be preloadable"
    );
}

#[test]
fn test_audit_scoring_system() {
    // Perfect score
    let mut perfect_headers = HashMap::new();
    perfect_headers.insert(
        "Content-Security-Policy".to_string(),
        "default-src 'self'".to_string(),
    );
    perfect_headers.insert("X-Frame-Options".to_string(), "DENY".to_string());
    perfect_headers.insert("X-Content-Type-Options".to_string(), "nosniff".to_string());
    perfect_headers.insert(
        "Referrer-Policy".to_string(),
        "strict-origin-when-cross-origin".to_string(),
    );

    let audit = audit_security_headers(&perfect_headers);
    assert_eq!(audit.score, 100, "Perfect headers should score 100");

    // Missing one critical header
    let mut missing_one = perfect_headers.clone();
    missing_one.remove("X-Frame-Options");

    let audit = audit_security_headers(&missing_one);
    assert_eq!(
        audit.score, 80,
        "Missing one critical header should score 80"
    );

    // Unsafe CSP
    let mut unsafe_csp = perfect_headers.clone();
    unsafe_csp.insert(
        "Content-Security-Policy".to_string(),
        "default-src 'self'; script-src 'unsafe-eval'".to_string(),
    );

    let audit = audit_security_headers(&unsafe_csp);
    assert_eq!(audit.score, 90, "Unsafe CSP should score 90");

    // Multiple issues
    let mut multiple_issues = HashMap::new();
    multiple_issues.insert(
        "Content-Security-Policy".to_string(),
        "default-src *".to_string(),
    );
    multiple_issues.insert("X-Frame-Options".to_string(), "ALLOWALL".to_string());

    let audit = audit_security_headers(&multiple_issues);
    assert!(
        audit.score < 70,
        "Multiple issues should significantly lower score"
    );
}

#[test]
fn test_integration_with_security_audit() {
    // Test the integration between configuration and audit
    let config = SecurityConfig::development();
    let headers = config.to_headers();

    // Simulate what the server should apply
    let mut response_headers = HashMap::new();
    for (name, value) in headers {
        response_headers.insert(name.to_string(), value);
    }

    let audit = audit_security_headers(&response_headers);

    // Development config should be reasonably secure but not perfect due to dev allowances
    assert!(
        audit.score >= 80,
        "Development config should score at least 80"
    );
    assert!(
        audit.missing_headers.is_empty(),
        "No critical headers should be missing"
    );

    // Should have warnings about unsafe-eval in development
    let has_eval_warning = audit.warnings.iter().any(|w| w.contains("unsafe-eval"));
    assert!(
        has_eval_warning,
        "Should warn about unsafe-eval in development"
    );
}
