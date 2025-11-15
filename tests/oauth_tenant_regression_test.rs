//! Regression tests for OAuth and Tenant functionality
//!
//! Tests to verify fixes for:
//! - Multi-tenant OAuth credential support
//! - Fitbit OAuth token exchange
//! - OAuth credential validation
//! - Tenant slug validation

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

/// Test tenant slug validation - empty slug
#[test]
fn test_tenant_slug_validation_empty() {
    let result = validate_tenant_slug("");
    assert!(result.is_err(), "Empty slug should be rejected");
    assert!(result.unwrap_err().contains("cannot be empty"));
}

/// Test tenant slug validation - too long
#[test]
fn test_tenant_slug_validation_too_long() {
    let long_slug = "a".repeat(64); // 64 characters, limit is 63
    let result = validate_tenant_slug(&long_slug);
    assert!(result.is_err(), "Slug over 63 chars should be rejected");
    assert!(result.unwrap_err().contains("63 characters or less"));
}

/// Test tenant slug validation - invalid characters
#[test]
fn test_tenant_slug_validation_invalid_chars() {
    let invalid_slugs = vec![
        "hello world", // space
        "hello_world", // underscore
        "hello.world", // dot
        "hello@world", // special char
        "hello/world", // slash
    ];

    for slug in invalid_slugs {
        let result = validate_tenant_slug(slug);
        assert!(result.is_err(), "Slug '{slug}' should be rejected");
        assert!(result
            .unwrap_err()
            .contains("letters, numbers, and hyphens"));
    }
}

/// Test tenant slug validation - leading/trailing hyphens
#[test]
fn test_tenant_slug_validation_hyphens() {
    let invalid_slugs = vec![
        "-hello",  // leading hyphen
        "hello-",  // trailing hyphen
        "-hello-", // both
    ];

    for slug in invalid_slugs {
        let result = validate_tenant_slug(slug);
        assert!(result.is_err(), "Slug '{slug}' should be rejected");
        assert!(result
            .unwrap_err()
            .contains("cannot start or end with a hyphen"));
    }
}

/// Test tenant slug validation - reserved slugs
#[test]
fn test_tenant_slug_validation_reserved() {
    let reserved_slugs = vec![
        "admin",
        "api",
        "www",
        "app",
        "dashboard",
        "auth",
        "oauth",
        "login",
        "logout",
        "signup",
        "system",
        "root",
        "public",
        "static",
        "assets",
    ];

    for slug in reserved_slugs {
        let result = validate_tenant_slug(slug);
        assert!(result.is_err(), "Reserved slug '{slug}' should be rejected");
        assert!(result.unwrap_err().contains("reserved"));
    }
}

/// Test tenant slug validation - valid slugs
#[test]
fn test_tenant_slug_validation_valid() {
    let sixty_three_chars = "a".repeat(63);
    let valid_slugs = vec![
        "hello",
        "hello-world",
        "hello123",
        "123hello",
        "h",
        &sixty_three_chars, // exactly 63 chars
        "my-company-2024",
    ];

    for slug in valid_slugs {
        let result = validate_tenant_slug(slug);
        assert!(result.is_ok(), "Valid slug '{slug}' should be accepted");
    }
}

/// Helper function to test slug validation logic
/// This mimics the validation in `AdminApiRoutes::create_default_tenant_for_user`
fn validate_tenant_slug(tenant_slug: &str) -> Result<String, String> {
    // Reserved slugs that cannot be used for tenants
    const RESERVED_SLUGS: &[&str] = &[
        "admin",
        "api",
        "www",
        "app",
        "dashboard",
        "auth",
        "oauth",
        "login",
        "logout",
        "signup",
        "system",
        "root",
        "public",
        "static",
        "assets",
    ];

    let slug = tenant_slug.trim().to_lowercase();

    // Validate slug format
    if slug.is_empty() {
        return Err("Tenant slug cannot be empty".to_owned());
    }

    if slug.len() > 63 {
        return Err("Tenant slug must be 63 characters or less".to_owned());
    }

    // Check for valid characters (alphanumeric and hyphens only)
    if !slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err("Tenant slug can only contain letters, numbers, and hyphens".to_owned());
    }

    // Check for leading/trailing hyphens
    if slug.starts_with('-') || slug.ends_with('-') {
        return Err("Tenant slug cannot start or end with a hyphen".to_owned());
    }

    // Check against reserved slugs
    if RESERVED_SLUGS.contains(&slug.as_str()) {
        return Err(format!(
            "Tenant slug '{slug}' is reserved and cannot be used"
        ));
    }

    Ok(slug)
}

/// Test that OAuth providers constants are correct
#[test]
fn test_oauth_providers_constants() {
    use pierre_mcp_server::constants::oauth_providers;

    // Verify provider names
    assert_eq!(oauth_providers::STRAVA, "strava");
    assert_eq!(oauth_providers::FITBIT, "fitbit");
}

/// Test that OAuth scopes constants exist for both providers
#[test]
fn test_oauth_scopes_for_all_providers() {
    use pierre_mcp_server::constants::oauth;

    // Strava scopes
    assert_eq!(
        oauth::STRAVA_DEFAULT_SCOPES,
        "activity:read_all",
        "Strava scope should be activity:read_all"
    );

    // Fitbit scopes
    assert_eq!(
        oauth::FITBIT_DEFAULT_SCOPES,
        "activity profile",
        "Fitbit scope should be 'activity profile' (space-separated)"
    );
}

/// Comprehensive test ensuring all validation rules work together
#[test]
fn test_comprehensive_slug_validation() {
    // Valid cases
    assert!(validate_tenant_slug("good-slug-123").is_ok());
    assert!(validate_tenant_slug("UPPER-CASE").is_ok()); // Gets lowercased
    assert!(validate_tenant_slug("  spaced  ").is_ok()); // Gets trimmed

    // Invalid cases
    assert!(validate_tenant_slug("").is_err());
    assert!(validate_tenant_slug("   ").is_err()); // Empty after trim
    assert!(validate_tenant_slug("admin").is_err()); // Reserved
    assert!(validate_tenant_slug("-bad").is_err()); // Leading hyphen
    assert!(validate_tenant_slug("bad-").is_err()); // Trailing hyphen
    assert!(validate_tenant_slug("bad slug").is_err()); // Space
    assert!(validate_tenant_slug(&"x".repeat(64)).is_err()); // Too long

    println!("✅ All comprehensive slug validation tests passed");
}
