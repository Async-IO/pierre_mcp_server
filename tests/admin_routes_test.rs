// Integration tests for admin_routes.rs module
// Tests for admin API endpoints and authentication

use pierre_mcp_server::{
    admin_routes::ProvisionApiKeyRequest, utils::auth::extract_bearer_token_owned,
};

#[test]
fn test_extract_bearer_token() {
    assert_eq!(
        extract_bearer_token_owned("Bearer test_token").unwrap(),
        "test_token"
    );
    assert_eq!(
        extract_bearer_token_owned("Bearer   spaced_token   ").unwrap(),
        "spaced_token"
    );
    assert!(extract_bearer_token_owned("Basic test").is_err());
    assert!(extract_bearer_token_owned("Bearer ").is_err());
    assert!(extract_bearer_token_owned("").is_err());
}

#[test]
fn test_provision_request_validation() {
    let request = ProvisionApiKeyRequest {
        user_email: "test@example.com".into(),
        tier: "starter".into(),
        description: Some("Test key".into()),
        expires_in_days: Some(30),
        rate_limit_requests: Some(100),
        rate_limit_period: Some("hour".into()),
    };

    assert_eq!(request.user_email, "test@example.com");
    assert_eq!(request.tier, "starter");
}
