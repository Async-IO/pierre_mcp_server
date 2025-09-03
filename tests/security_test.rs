// Integration tests for security.rs module
// Tests for security headers configuration and validation

use pierre_mcp_server::security::{audit_security_headers, headers::SecurityConfig};
use std::collections::HashMap;

#[test]
fn test_development_config() {
    let config = SecurityConfig::development();
    let headers = config.to_headers();

    assert!(headers
        .get("Content-Security-Policy")
        .unwrap()
        .contains("'unsafe-inline'"));
    assert_eq!(headers.get("X-Frame-Options").unwrap(), "DENY");
    assert_eq!(headers.get("X-Content-Type-Options").unwrap(), "nosniff");
    assert!(!headers.contains_key("Strict-Transport-Security")); // No HSTS in dev
}

#[test]
fn test_production_config() {
    let config = SecurityConfig::production();
    let headers = config.to_headers();

    assert!(!headers
        .get("Content-Security-Policy")
        .unwrap()
        .contains("'unsafe-eval'"));
    assert!(headers.contains_key("Strict-Transport-Security"));
    assert_eq!(headers.get("X-Frame-Options").unwrap(), "DENY");
    assert_eq!(headers.get("X-Content-Type-Options").unwrap(), "nosniff");
}

#[test]
fn test_headers_conversion() {
    let config = SecurityConfig::development();
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

    let audit = audit_security_headers(&headers);
    assert!(audit, "Secure headers should pass audit");
}

#[test]
fn test_security_audit_missing_headers() {
    let headers = HashMap::new();
    let audit = audit_security_headers(&headers);
    assert!(!audit, "Empty headers should fail audit");
}

#[test]
fn test_security_audit_partial_headers() {
    let mut headers = HashMap::new();
    headers.insert(
        "Content-Security-Policy".to_string(),
        "default-src 'self'".to_string(),
    );
    // Missing other required headers

    let audit = audit_security_headers(&headers);
    assert!(!audit, "Partial headers should fail audit");
}

#[test]
fn test_from_environment() {
    let dev_config = SecurityConfig::from_environment("development");
    assert_eq!(dev_config.environment, "development");

    let prod_config = SecurityConfig::from_environment("production");
    assert_eq!(prod_config.environment, "production");

    // Test case insensitive
    let prod_config2 = SecurityConfig::from_environment("PRODUCTION");
    assert_eq!(prod_config2.environment, "production");

    // Test default fallback
    let default_config = SecurityConfig::from_environment("unknown");
    assert_eq!(default_config.environment, "development");
}

#[test]
fn test_config_consistency() {
    let dev_config = SecurityConfig::development();
    let prod_config = SecurityConfig::production();

    // Both should have basic required headers
    let dev_headers = dev_config.to_headers();
    let prod_headers = prod_config.to_headers();

    let required_headers = [
        "Content-Security-Policy",
        "X-Frame-Options",
        "X-Content-Type-Options",
    ];

    for header in &required_headers {
        assert!(
            dev_headers.contains_key(*header),
            "Dev config missing header: {header}"
        );
        assert!(
            prod_headers.contains_key(*header),
            "Prod config missing header: {header}"
        );
    }
}

#[test]
fn test_to_headers_format() {
    let config = SecurityConfig::development();
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
    }
}
