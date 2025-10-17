// ABOUTME: Tests for PII-safe logging and redaction middleware
// ABOUTME: Validates header redaction, email masking, and token pattern detection
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use pierre_mcp_server::middleware::redaction::{
    mask_email, redact_headers, redact_json_fields, redact_token_patterns, BoundedTenantLabel,
    BoundedUserLabel, RedactionConfig,
};

#[test]
fn test_redact_authorization_header() {
    let config = RedactionConfig::default();
    let headers = [
        ("Authorization", "Bearer secret_token_12345"),
        ("Content-Type", "application/json"),
    ];

    let redacted = redact_headers(headers.iter().map(|(k, v)| (*k, *v)), &config);

    assert_eq!(redacted[0].0, "Authorization");
    assert_eq!(redacted[0].1, "[REDACTED]");
    assert_eq!(redacted[1].0, "Content-Type");
    assert_eq!(redacted[1].1, "application/json");
}

#[test]
fn test_redact_cookie_header() {
    let config = RedactionConfig::default();
    let headers = [
        ("Cookie", "session=abc123; userid=456"),
        ("Accept", "application/json"),
    ];

    let redacted = redact_headers(headers.iter().map(|(k, v)| (*k, *v)), &config);

    assert_eq!(redacted[0].1, "[REDACTED]");
    assert_eq!(redacted[1].1, "application/json");
}

#[test]
fn test_redact_json_fields() {
    let config = RedactionConfig::default();
    let json = r#"{"username":"testuser","client_secret":"secret123","password":"pass456"}"#;

    let redacted = redact_json_fields(json, &config);

    assert!(redacted.contains(r#""client_secret": "[REDACTED]""#));
    assert!(!redacted.contains("secret123"));
    assert!(!redacted.contains("pass456"));
}

#[test]
fn test_mask_email_basic() {
    let email = "testuser@domain.com";
    let masked = mask_email(email);
    assert_eq!(masked, "t***@d***.com");
}

#[test]
fn test_mask_email_short_local() {
    let email = "a@domain.com";
    let masked = mask_email(email);
    assert_eq!(masked, "a@d***.com");
}

#[test]
fn test_mask_email_in_text() {
    let text = "Contact testuser@domain.com or admin@service.org for help";
    let masked = mask_email(text);
    assert!(masked.contains("t***@d***.com"));
    assert!(masked.contains("a***@s***.org"));
}

#[test]
fn test_redact_bearer_token() {
    let config = RedactionConfig::default();
    let text = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.payload.signature";

    let redacted = redact_token_patterns(text, &config);

    assert!(!redacted.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
    assert!(redacted.contains("Bearer [REDACTED]"));
}

#[test]
fn test_redact_jwt_pattern() {
    let config = RedactionConfig::default();
    let text =
        "JWT token: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.signature";

    let redacted = redact_token_patterns(text, &config);

    assert!(!redacted.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
    assert!(redacted.contains("[REDACTED]"));
}

#[test]
fn test_bounded_tenant_label() {
    let tenant1 = BoundedTenantLabel::new("tenant_uuid_123");
    let tenant2 = BoundedTenantLabel::new("tenant_uuid_123");
    let tenant3 = BoundedTenantLabel::new("different_tenant_456");

    assert_eq!(tenant1, tenant2);

    assert!(tenant1.as_str().starts_with("tenant_bucket_"));
    assert!(tenant3.as_str().starts_with("tenant_bucket_"));
}

#[test]
fn test_bounded_user_label() {
    let user1 = BoundedUserLabel::new("user_uuid_123");
    let user2 = BoundedUserLabel::new("user_uuid_123");

    assert_eq!(user1, user2);
    assert!(user1.as_str().starts_with("user_bucket_"));
}

#[test]
fn test_redaction_disabled() {
    let config = RedactionConfig {
        enabled: false,
        ..Default::default()
    };

    let headers = [("Authorization", "Bearer secret_token")];
    let redacted = redact_headers(headers.iter().map(|(k, v)| (*k, *v)), &config);

    assert_eq!(redacted[0].1, "Bearer secret_token");
}

#[test]
fn test_custom_placeholder() {
    let config = RedactionConfig {
        redaction_placeholder: "***".to_string(),
        ..Default::default()
    };

    let headers = [("Authorization", "Bearer secret_token")];
    let redacted = redact_headers(headers.iter().map(|(k, v)| (*k, *v)), &config);

    assert_eq!(redacted[0].1, "***");
}
