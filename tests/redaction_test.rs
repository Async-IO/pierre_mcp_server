// ABOUTME: Tests for PII-safe logging and redaction middleware
// ABOUTME: Validates header redaction, email masking, and token pattern detection
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::middleware::redaction::{
    mask_email, redact_headers, redact_json_fields, redact_session_id, redact_token_patterns,
    BoundedTenantLabel, BoundedUserLabel, RedactionConfig,
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
fn test_redact_session_id_typical() {
    let sid = "session_a1b2c3d4-e5f6-7890-abcd-ef1234567890";
    let redacted = redact_session_id(sid);
    assert_eq!(redacted, "session_a1b2...");
    assert!(!redacted.contains("ef1234567890"));
}

#[test]
fn test_redact_session_id_preserves_short_ids() {
    let short = "session_abc";
    let redacted = redact_session_id(short);
    assert_eq!(redacted, "session_abc");
}

#[test]
fn test_redact_session_id_exactly_at_boundary() {
    // Exactly 12 chars should not get ellipsis
    let exactly_12 = "session_abcd";
    assert_eq!(exactly_12.len(), 12);
    let redacted = redact_session_id(exactly_12);
    assert_eq!(redacted, "session_abcd");

    // 13 chars should get truncated
    let thirteen = "session_abcde";
    let redacted = redact_session_id(thirteen);
    assert_eq!(redacted, "session_abcd...");
}

#[test]
fn test_redact_session_id_empty() {
    let empty = "";
    let redacted = redact_session_id(empty);
    assert_eq!(redacted, "");
}

#[test]
fn test_custom_placeholder() {
    let config = RedactionConfig {
        redaction_placeholder: "***".to_owned(),
        ..Default::default()
    };

    let headers = [("Authorization", "Bearer secret_token")];
    let redacted = redact_headers(headers.iter().map(|(k, v)| (*k, *v)), &config);

    assert_eq!(redacted[0].1, "***");
}
