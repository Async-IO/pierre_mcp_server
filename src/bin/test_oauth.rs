use anyhow::Result;
use pierre_mcp_server::providers::strava::StravaProvider;
use pierre_mcp_server::providers::{FitnessProvider, AuthData};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    println!("Testing Strava OAuth2 implementation...\n");
    
    // Test 1: Provider initialization
    let mut provider = StravaProvider::new();
    println!("✅ Provider created successfully");
    
    // Test 2: Authentication setup
    provider.authenticate(AuthData::OAuth2 {
        client_id: "163846".to_string(),
        client_secret: "1dfc45ad0a1f6983b835e4495aa9473d111d03bc".to_string(),
        access_token: None,
        refresh_token: None,
    }).await?;
    println!("✅ Authentication configured");
    
    // Test 3: Generate auth URL
    let auth_url = provider.get_auth_url("http://localhost:8080/callback", "test-state")?;
    println!("✅ Auth URL generated: {}", auth_url);
    
    // Verify URL components
    let url = url::Url::parse(&auth_url)?;
    assert_eq!(url.host_str(), Some("www.strava.com"));
    assert!(url.path().contains("oauth/authorize"));
    
    let params: std::collections::HashMap<_, _> = url.query_pairs().collect();
    assert_eq!(params.get("client_id"), Some(&"163846".into()));
    assert_eq!(params.get("response_type"), Some(&"code".into()));
    assert_eq!(params.get("scope"), Some(&"read,activity:read_all".into()));
    assert_eq!(params.get("state"), Some(&"test-state".into()));
    
    println!("✅ URL parameters verified");
    
    // Test 4: Simulate having tokens
    let mut provider2 = StravaProvider::new();
    provider2.authenticate(AuthData::OAuth2 {
        client_id: "163846".to_string(),
        client_secret: "1dfc45ad0a1f6983b835e4495aa9473d111d03bc".to_string(),
        access_token: Some("test_access_token".to_string()),
        refresh_token: Some("test_refresh_token".to_string()),
    }).await?;
    println!("✅ Provider authenticated with existing tokens");
    
    println!("\n🎉 All OAuth2 tests passed!");
    println!("\nThe OAuth2 implementation is working correctly.");
    println!("To complete the flow, you would:");
    println!("1. Visit the auth URL in a browser");
    println!("2. Authorize the application");
    println!("3. Get redirected to the callback URL with a code");
    println!("4. Exchange the code for tokens");
    
    Ok(())
}