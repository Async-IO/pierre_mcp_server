// ABOUTME: Tests for OAuth template-based HTML rendering
// ABOUTME: Validates template compilation, placeholder replacement, and HTML generation
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Tests for OAuth template-based HTML rendering
//!
//! This test suite validates:
//! 1. Template files exist and compile at build time
//! 2. All placeholders in templates match code expectations
//! 3. HTML rendering produces valid output
//! 4. Error templates render correctly

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

mod common;

use pierre_mcp_server::routes::oauth2::OAuth2Routes;

/// Test that OAuth login template compiles and contains required placeholders
#[test]
fn test_oauth_login_template_exists() {
    // This test validates that the template file exists and can be loaded at compile time
    // If the file doesn't exist, compilation will fail
    const TEMPLATE: &str = include_str!("../templates/oauth_login.html");

    // Verify all required placeholders exist
    let required_placeholders = [
        "{{CLIENT_ID}}",
        "{{REDIRECT_URI}}",
        "{{RESPONSE_TYPE}}",
        "{{STATE}}",
        "{{SCOPE}}",
        "{{CODE_CHALLENGE}}",
        "{{CODE_CHALLENGE_METHOD}}",
        "{{DEFAULT_EMAIL}}",
        "{{DEFAULT_PASSWORD}}",
    ];

    for placeholder in &required_placeholders {
        assert!(
            TEMPLATE.contains(placeholder),
            "OAuth login template missing required placeholder: {placeholder}"
        );
    }

    // Verify template structure - should be valid HTML
    assert!(TEMPLATE.contains("<!DOCTYPE html>"), "Missing DOCTYPE");
    assert!(TEMPLATE.contains("<html"), "Missing html tag");
    assert!(TEMPLATE.contains("</html>"), "Missing closing html tag");
    assert!(
        TEMPLATE.contains("<form method=\"post\" action=\"/oauth2/login\">"),
        "Missing login form"
    );
    assert!(
        TEMPLATE.contains("type=\"email\""),
        "Missing email input field"
    );
    assert!(
        TEMPLATE.contains("type=\"password\""),
        "Missing password input field"
    );
    assert!(
        TEMPLATE.contains("type=\"submit\""),
        "Missing submit button"
    );
}

/// Test that OAuth login error template compiles and contains required placeholders
#[test]
fn test_oauth_login_error_template_exists() {
    // This test validates that the error template file exists and can be loaded at compile time
    // If the file doesn't exist, compilation will fail
    const TEMPLATE: &str = include_str!("../templates/oauth_login_error.html");

    // Verify all required placeholders exist
    let required_placeholders = [
        "{{ERROR_MESSAGE}}",
        "{{CLIENT_ID}}",
        "{{REDIRECT_URI}}",
        "{{RESPONSE_TYPE}}",
        "{{STATE}}",
        "{{SCOPE}}",
        "{{CODE_CHALLENGE}}",
        "{{CODE_CHALLENGE_METHOD}}",
    ];

    for placeholder in &required_placeholders {
        assert!(
            TEMPLATE.contains(placeholder),
            "OAuth login error template missing required placeholder: {placeholder}"
        );
    }

    // Verify template structure - should be valid HTML with error messaging
    assert!(TEMPLATE.contains("<!DOCTYPE html>"), "Missing DOCTYPE");
    assert!(TEMPLATE.contains("<html"), "Missing html tag");
    assert!(TEMPLATE.contains("</html>"), "Missing closing html tag");
    assert!(
        TEMPLATE.contains("Back to Login") || TEMPLATE.contains("back to login"),
        "Missing back to login link"
    );
}

/// Test OAuth login HTML generation with template replacement
#[tokio::test]
async fn test_generate_login_html() {
    common::init_server_config();

    // Create test parameters with known values
    let test_client_id = "test_client_123";
    let test_redirect = "https://example.com/callback";
    let test_response_type = "code";
    let test_state = "random_state_xyz";
    let test_scope = "read:activities write:profile";
    let test_challenge = "challenge_abc";
    let test_method = "S256";
    let test_email = "test@example.com";
    let test_password = "test_pass_123";

    // Generate HTML using the OAuth2Routes method
    let html =
        OAuth2Routes::generate_login_html(pierre_mcp_server::routes::oauth2::LoginHtmlParams {
            client_id: test_client_id,
            redirect_uri: test_redirect,
            response_type: test_response_type,
            state: test_state,
            scope: test_scope,
            code_challenge: test_challenge,
            code_challenge_method: test_method,
            default_email: test_email,
            default_password: test_password,
        });

    // Verify all placeholders were replaced with actual values
    assert!(
        !html.contains("{{CLIENT_ID}}"),
        "CLIENT_ID placeholder not replaced"
    );
    assert!(
        !html.contains("{{REDIRECT_URI}}"),
        "REDIRECT_URI placeholder not replaced"
    );
    assert!(
        !html.contains("{{RESPONSE_TYPE}}"),
        "RESPONSE_TYPE placeholder not replaced"
    );
    assert!(
        !html.contains("{{STATE}}"),
        "STATE placeholder not replaced"
    );
    assert!(
        !html.contains("{{SCOPE}}"),
        "SCOPE placeholder not replaced"
    );
    assert!(
        !html.contains("{{CODE_CHALLENGE}}"),
        "CODE_CHALLENGE placeholder not replaced"
    );
    assert!(
        !html.contains("{{CODE_CHALLENGE_METHOD}}"),
        "CODE_CHALLENGE_METHOD placeholder not replaced"
    );
    assert!(
        !html.contains("{{DEFAULT_EMAIL}}"),
        "DEFAULT_EMAIL placeholder not replaced"
    );
    assert!(
        !html.contains("{{DEFAULT_PASSWORD}}"),
        "DEFAULT_PASSWORD placeholder not replaced"
    );

    // Verify actual values appear in the HTML
    assert!(
        html.contains(test_client_id),
        "Generated HTML missing client_id value"
    );
    assert!(
        html.contains(test_redirect),
        "Generated HTML missing redirect_uri value"
    );
    assert!(
        html.contains(test_response_type),
        "Generated HTML missing response_type value"
    );
    assert!(
        html.contains(test_state),
        "Generated HTML missing state value"
    );
    assert!(
        html.contains(test_scope),
        "Generated HTML missing scope value"
    );
    assert!(
        html.contains(test_challenge),
        "Generated HTML missing code_challenge value"
    );
    assert!(
        html.contains(test_method),
        "Generated HTML missing code_challenge_method value"
    );
    assert!(
        html.contains(test_email),
        "Generated HTML missing default_email value"
    );

    // Note: password value is in password field, verify it's there
    assert!(
        html.contains(&format!("value=\"{test_password}\"")),
        "Generated HTML missing default_password value"
    );

    // Verify HTML structure is intact
    assert!(html.contains("<!DOCTYPE html>"), "Missing DOCTYPE");
    assert!(
        html.contains("<form method=\"post\" action=\"/oauth2/login\">"),
        "Missing form element"
    );
}

/// Test OAuth login HTML generation with empty scope (should use default)
#[tokio::test]
async fn test_generate_login_html_empty_scope() {
    common::init_server_config();

    // Generate HTML with empty scope
    let html =
        OAuth2Routes::generate_login_html(pierre_mcp_server::routes::oauth2::LoginHtmlParams {
            client_id: "test",
            redirect_uri: "https://example.com",
            response_type: "code",
            state: "state",
            scope: "", // Empty scope
            code_challenge: "challenge",
            code_challenge_method: "S256",
            default_email: "test@example.com",
            default_password: "",
        });

    // Verify default scope is used when scope is empty
    assert!(
        html.contains("fitness:read activities:read profile:read"),
        "Empty scope should be replaced with default scope"
    );
    assert!(
        !html.contains("{{SCOPE}}"),
        "SCOPE placeholder not replaced"
    );
}

/// Test OAuth login error HTML rendering
#[tokio::test]
async fn test_oauth_login_error_rendering() {
    // Load the error template
    const TEMPLATE: &str = include_str!("../templates/oauth_login_error.html");

    // Simulate error HTML generation
    let test_error_msg = "Authentication Failed: Invalid credentials";
    let test_client_id = "error_client_123";
    let test_redirect = "https://example.com/callback";
    let test_state = "error_state";

    let error_html = TEMPLATE
        .replace("{{ERROR_MESSAGE}}", test_error_msg)
        .replace("{{CLIENT_ID}}", test_client_id)
        .replace("{{REDIRECT_URI}}", test_redirect)
        .replace("{{RESPONSE_TYPE}}", "code")
        .replace("{{STATE}}", test_state)
        .replace("{{SCOPE}}", "read")
        .replace("{{CODE_CHALLENGE}}", "challenge")
        .replace("{{CODE_CHALLENGE_METHOD}}", "S256");

    // Verify error message appears
    assert!(
        error_html.contains(test_error_msg),
        "Error message not in HTML"
    );

    // Verify no placeholders remain
    assert!(
        !error_html.contains("{{ERROR_MESSAGE}}"),
        "ERROR_MESSAGE placeholder not replaced"
    );
    assert!(
        !error_html.contains("{{CLIENT_ID}}"),
        "CLIENT_ID placeholder not replaced in error template"
    );

    // Verify back link contains parameters
    assert!(
        error_html.contains("/oauth2/login?"),
        "Missing back to login link"
    );
    assert!(
        error_html.contains(&format!("client_id={test_client_id}")),
        "Back link missing client_id parameter"
    );
    assert!(
        error_html.contains(&format!("state={test_state}")),
        "Back link missing state parameter"
    );
}

/// Test that templates use Pierre design system colors
#[test]
fn test_templates_use_pierre_design_system() {
    const LOGIN_TEMPLATE: &str = include_str!("../templates/oauth_login.html");
    const ERROR_TEMPLATE: &str = include_str!("../templates/oauth_login_error.html");

    // Verify Pierre brand colors are used (hex colors from BRAND.md)
    // Primary: Violet #7C3AED, Cyan #06B6D4
    // Activity: Emerald #10B981
    // Nutrition: Amber #F59E0B
    // Recovery: Indigo #6366F1
    let pierre_colors = [
        "#7C3AED", // Pierre Violet
        "#06B6D4", // Pierre Cyan
    ];

    for color in &pierre_colors {
        assert!(
            LOGIN_TEMPLATE.contains(color),
            "Login template missing Pierre brand color: {color}"
        );
        assert!(
            ERROR_TEMPLATE.contains(color),
            "Error template missing Pierre brand color: {color}"
        );
    }

    // Verify Pierre branding elements
    assert!(
        LOGIN_TEMPLATE.contains("Pierre") || LOGIN_TEMPLATE.contains("pierre"),
        "Login template missing Pierre branding"
    );
    assert!(
        ERROR_TEMPLATE.contains("Pierre") || ERROR_TEMPLATE.contains("pierre"),
        "Error template missing Pierre branding"
    );
}

/// Test template accessibility features
#[test]
fn test_templates_accessibility() {
    const LOGIN_TEMPLATE: &str = include_str!("../templates/oauth_login.html");
    const ERROR_TEMPLATE: &str = include_str!("../templates/oauth_login_error.html");

    // Verify proper HTML lang attribute
    assert!(
        LOGIN_TEMPLATE.contains("lang=\"en\""),
        "Login template missing lang attribute"
    );
    assert!(
        ERROR_TEMPLATE.contains("lang=\"en\""),
        "Error template missing lang attribute"
    );

    // Verify meta viewport for responsive design
    assert!(
        LOGIN_TEMPLATE.contains("viewport"),
        "Login template missing viewport meta"
    );
    assert!(
        ERROR_TEMPLATE.contains("viewport"),
        "Error template missing viewport meta"
    );

    // Verify form inputs have proper labels or aria-labels
    assert!(
        LOGIN_TEMPLATE.contains("aria-label") || LOGIN_TEMPLATE.contains("<label"),
        "Login template missing accessibility labels"
    );

    // Verify no autoplay or autofocus that could be disruptive
    assert!(
        !LOGIN_TEMPLATE.contains("autofocus"),
        "Login template should not use autofocus"
    );
}

/// Test template security features
#[test]
fn test_templates_security_features() {
    const LOGIN_TEMPLATE: &str = include_str!("../templates/oauth_login.html");
    const ERROR_TEMPLATE: &str = include_str!("../templates/oauth_login_error.html");

    // Verify noindex meta tag (OAuth pages should not be indexed)
    assert!(
        LOGIN_TEMPLATE.contains("noindex"),
        "Login template should have noindex meta"
    );
    assert!(
        ERROR_TEMPLATE.contains("noindex"),
        "Error template should have noindex meta"
    );

    // Verify referrer policy
    assert!(
        LOGIN_TEMPLATE.contains("referrer"),
        "Login template should have referrer policy"
    );
    assert!(
        ERROR_TEMPLATE.contains("referrer"),
        "Error template should have referrer policy"
    );

    // Verify autocomplete attributes for security-sensitive fields
    assert!(
        LOGIN_TEMPLATE.contains("autocomplete"),
        "Login template should have autocomplete attributes"
    );
}

// ============================================================================
// HTML Escaping Tests
// ============================================================================

use pierre_mcp_server::utils::html::escape_html_attribute;

#[test]
fn test_escape_html_attribute_no_special_chars() {
    assert_eq!(escape_html_attribute("hello world"), "hello world");
}

#[test]
fn test_escape_html_attribute_quotes() {
    assert_eq!(
        escape_html_attribute(r#"value"with"quotes"#),
        "value&quot;with&quot;quotes"
    );
}

#[test]
fn test_escape_html_attribute_xss_payload() {
    assert_eq!(
        escape_html_attribute(r#""><script>alert(1)</script>"#),
        "&quot;&gt;&lt;script&gt;alert(1)&lt;/script&gt;"
    );
}

#[test]
fn test_escape_html_attribute_ampersand() {
    assert_eq!(escape_html_attribute("a&b=c"), "a&amp;b=c");
}

#[test]
fn test_escape_html_attribute_single_quotes() {
    assert_eq!(escape_html_attribute("it's"), "it&#x27;s");
}

#[test]
fn test_escape_html_attribute_empty_string() {
    assert_eq!(escape_html_attribute(""), "");
}

#[test]
fn test_escape_html_attribute_base64url_safe() {
    // Base64url characters should pass through unchanged
    assert_eq!(
        escape_html_attribute("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
        "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
    );
}

// ============================================================================
// XSS Prevention Tests
// ============================================================================

/// Test that XSS payloads in the state parameter are properly escaped in login HTML
#[tokio::test]
async fn test_login_html_escapes_xss_in_state() {
    common::init_server_config();

    let xss_state = r#""><script>alert('xss')</script>"#;
    let html =
        OAuth2Routes::generate_login_html(pierre_mcp_server::routes::oauth2::LoginHtmlParams {
            client_id: "test_client",
            redirect_uri: "https://example.com/callback",
            response_type: "code",
            state: xss_state,
            scope: "fitness:read",
            code_challenge: "challenge",
            code_challenge_method: "S256",
            default_email: "",
            default_password: "",
        });

    // The XSS payload should NOT appear unescaped
    assert!(
        !html.contains("<script>"),
        "XSS payload in state was not HTML-escaped"
    );
    // The escaped version should appear
    assert!(
        html.contains("&lt;script&gt;"),
        "State value should be HTML-escaped"
    );
}

/// Test that XSS payloads in `redirect_uri` are properly escaped in login HTML
#[tokio::test]
async fn test_login_html_escapes_xss_in_redirect_uri() {
    common::init_server_config();

    let xss_redirect = r#"https://evil.com" onload="alert(1)"#;
    let html =
        OAuth2Routes::generate_login_html(pierre_mcp_server::routes::oauth2::LoginHtmlParams {
            client_id: "test_client",
            redirect_uri: xss_redirect,
            response_type: "code",
            state: "safe_state",
            scope: "fitness:read",
            code_challenge: "challenge",
            code_challenge_method: "S256",
            default_email: "",
            default_password: "",
        });

    // The attribute breakout should NOT appear unescaped
    assert!(
        !html.contains(r#"" onload="#),
        "XSS payload in redirect_uri was not HTML-escaped"
    );
    assert!(
        html.contains("&quot;"),
        "Quotes in redirect_uri should be HTML-escaped"
    );
}

/// Integration test: Full OAuth login page rendering flow
#[tokio::test]
async fn test_oauth_login_page_integration() {
    common::init_server_config();

    // Generate login HTML with test values (no full server config needed for this test)
    let html =
        OAuth2Routes::generate_login_html(pierre_mcp_server::routes::oauth2::LoginHtmlParams {
            client_id: "integration_test_client",
            redirect_uri: "https://integration.test/callback",
            response_type: "code",
            state: "integration_state_123",
            scope: "read:all write:all",
            code_challenge: "integration_challenge",
            code_challenge_method: "S256",
            default_email: "test@pierre.test",
            default_password: "test123",
        });

    // Verify complete HTML structure
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("integration_test_client"));
    assert!(html.contains("integration_state_123"));
    assert!(html.contains("test@pierre.test"));
    assert!(html.contains("read:all write:all"));

    // Verify no template placeholders remain
    assert!(!html.contains("{{"));
    assert!(!html.contains("}}"));

    // Verify form has all required hidden fields for OAuth flow
    assert!(html.contains("name=\"client_id\""));
    assert!(html.contains("name=\"redirect_uri\""));
    assert!(html.contains("name=\"response_type\""));
    assert!(html.contains("name=\"state\""));
    assert!(html.contains("name=\"scope\""));
    assert!(html.contains("name=\"code_challenge\""));
    assert!(html.contains("name=\"code_challenge_method\""));

    // Verify visible form fields
    assert!(html.contains("name=\"email\""));
    assert!(html.contains("name=\"password\""));
}
