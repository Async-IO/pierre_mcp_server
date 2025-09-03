// ABOUTME: Integration tests for security headers middleware
// ABOUTME: Tests security header validation and configuration
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Integration tests for security headers middleware

use pierre_mcp_server::security::{audit_security_headers, headers::SecurityConfig};
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

    let dev_headers = dev_config.to_headers();
    let prod_headers = prod_config.to_headers();

    // Development should not have HSTS
    assert!(
        !dev_headers.contains_key("Strict-Transport-Security"),
        "Development config should not have HSTS"
    );

    // Production should have HSTS
    assert!(
        prod_headers.contains_key("Strict-Transport-Security"),
        "Production config should have HSTS"
    );

    // Development should be more permissive
    let dev_csp = dev_headers.get("Content-Security-Policy").unwrap();
    assert!(
        dev_csp.contains("'unsafe-inline'"),
        "Development CSP should allow unsafe-inline"
    );

    // Production should be stricter
    let prod_csp = prod_headers.get("Content-Security-Policy").unwrap();
    assert!(
        !prod_csp.contains("'unsafe-eval'"),
        "Production CSP should not allow unsafe-eval"
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

    let audit = audit_security_headers(&secure_headers);
    assert!(audit, "Secure headers should pass audit");

    // Test missing headers
    let empty_headers = HashMap::new();
    let audit = audit_security_headers(&empty_headers);
    assert!(!audit, "Empty headers should fail audit");

    // Test partial headers - should fail because missing required headers
    let mut partial_headers = HashMap::new();
    partial_headers.insert(
        "Content-Security-Policy".to_string(),
        "default-src 'self'".to_string(),
    );
    // Missing X-Frame-Options and X-Content-Type-Options

    let audit = audit_security_headers(&partial_headers);
    assert!(!audit, "Partial headers should fail audit");
}

#[test]
fn test_config_header_conversion() {
    let config = SecurityConfig::development();
    let headers = config.to_headers();

    // Check that all expected basic headers are present
    let expected_headers = [
        "Content-Security-Policy",
        "X-Frame-Options",
        "X-Content-Type-Options",
        "Referrer-Policy",
        "Permissions-Policy",
    ];

    for header in &expected_headers {
        assert!(headers.contains_key(*header), "Missing header: {header}");
        assert!(
            !headers[*header].is_empty(),
            "Header {header} should not be empty"
        );
    }
}

#[test]
fn test_csp_policies() {
    let dev_config = SecurityConfig::development();
    let prod_config = SecurityConfig::production();

    let dev_headers = dev_config.to_headers();
    let prod_headers = prod_config.to_headers();

    let dev_csp = dev_headers.get("Content-Security-Policy").unwrap();
    let prod_csp = prod_headers.get("Content-Security-Policy").unwrap();

    // Development CSP should allow unsafe-inline and unsafe-eval for dev tools
    assert!(
        dev_csp.contains("'unsafe-inline'"),
        "Dev CSP should allow unsafe-inline"
    );
    assert!(
        dev_csp.contains("'unsafe-eval'"),
        "Dev CSP should allow unsafe-eval"
    );

    // Production CSP should be stricter
    assert!(
        !prod_csp.contains("'unsafe-eval'"),
        "Prod CSP should not allow unsafe-eval"
    );
}

#[test]
fn test_permissions_policy() {
    let config = SecurityConfig::development();
    let headers = config.to_headers();

    let permissions_policy = headers.get("Permissions-Policy").unwrap();

    // Should disable dangerous features
    assert!(
        permissions_policy.contains("geolocation=()"),
        "Should disable geolocation"
    );
    assert!(
        permissions_policy.contains("microphone=()"),
        "Should disable microphone"
    );
    assert!(
        permissions_policy.contains("camera=()"),
        "Should disable camera"
    );
}

#[test]
fn test_hsts_configuration() {
    let dev_config = SecurityConfig::development();
    let prod_config = SecurityConfig::production();

    let dev_headers = dev_config.to_headers();
    let prod_headers = prod_config.to_headers();

    // Development should not have HSTS (HTTP)
    assert!(
        !dev_headers.contains_key("Strict-Transport-Security"),
        "Development should not have HSTS"
    );

    // Production should have strong HSTS
    assert!(
        prod_headers.contains_key("Strict-Transport-Security"),
        "Production should have HSTS"
    );

    let prod_hsts = prod_headers.get("Strict-Transport-Security").unwrap();
    assert!(
        prod_hsts.contains("max-age=31536000"),
        "Prod HSTS should have 1 year max-age"
    );
    assert!(
        prod_hsts.contains("includeSubDomains"),
        "Prod HSTS should include subdomains"
    );
}

#[test]
fn test_from_environment_integration() {
    let dev_config = SecurityConfig::from_environment("development");
    let prod_config = SecurityConfig::from_environment("production");
    let unknown_config = SecurityConfig::from_environment("unknown");

    // Should create appropriate configs
    assert_eq!(dev_config.environment, "development");
    assert_eq!(prod_config.environment, "production");
    assert_eq!(unknown_config.environment, "development"); // fallback

    // Headers should be different between environments
    let dev_headers = dev_config.to_headers();
    let prod_headers = prod_config.to_headers();

    assert!(
        dev_headers.get("Content-Security-Policy") != prod_headers.get("Content-Security-Policy")
    );
}

#[test]
fn test_integration_with_security_audit() {
    // Test the integration between configuration and audit
    let config = SecurityConfig::development();
    let headers = config.to_headers();

    // Convert to the format expected by audit function
    let mut response_headers = HashMap::new();
    for (name, value) in headers {
        response_headers.insert(name.clone(), value.clone());
    }

    let audit_result = audit_security_headers(&response_headers);

    // Development config should pass audit with required headers present
    assert!(
        audit_result,
        "Development config should pass security audit"
    );
}

#[test]
fn test_header_values_are_valid() {
    let configs = [
        SecurityConfig::development(),
        SecurityConfig::production(),
        SecurityConfig::from_environment("development"),
        SecurityConfig::from_environment("production"),
    ];

    for config in &configs {
        let headers = config.to_headers();

        // Verify headers are properly formatted strings
        for (name, value) in headers {
            assert!(!name.is_empty(), "Header name should not be empty");
            assert!(!value.is_empty(), "Header value should not be empty");
            // Basic validation that header names don't contain spaces
            assert!(
                !name.contains(' '),
                "Header name '{name}' should not contain spaces"
            );
            // Header values should not contain newlines (basic security check)
            assert!(
                !value.contains('\n'),
                "Header value should not contain newlines"
            );
            assert!(
                !value.contains('\r'),
                "Header value should not contain carriage returns"
            );
        }
    }
}
