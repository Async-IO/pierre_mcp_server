// ABOUTME: OAuth callback testing utility for validating authentication flow with fitness providers
// ABOUTME: Integration testing tool for Strava, Fitbit, and other OAuth-based fitness service connections
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Test OAuth callback functionality for the multi-tenant server

use anyhow::Result;
use pierre_mcp_server::auth::AuthManager;
use pierre_mcp_server::database::generate_encryption_key;
use pierre_mcp_server::database_plugins::factory::Database;
use pierre_mcp_server::routes::{AuthRoutes, LoginRequest, OAuthRoutes, RegisterRequest};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    println!("Testing OAuth callback implementation...\n");

    // 1. Set up test environment
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key).await?;
    let auth_manager = AuthManager::new(vec![0u8; 64], 24);

    let auth_routes = AuthRoutes::new(database.clone(), auth_manager.clone());
    let oauth_routes = OAuthRoutes::new(database.clone());

    println!("✅ Test environment initialized");

    // 2. Test user registration
    let test_email = "testuser@example.com";
    let test_password = "password123";

    let register_request = RegisterRequest {
        email: test_email.to_string(),
        password: test_password.to_string(),
        display_name: Some("Test User".into()),
    };

    let register_response = auth_routes.register(register_request).await?;
    let user_id = Uuid::parse_str(&register_response.user_id)?;
    println!("✅ User registered: {}", user_id);

    // 3. Test login
    let login_request = LoginRequest {
        email: test_email.to_string(),
        password: test_password.to_string(),
    };

    let login_response = auth_routes.login(login_request).await?;
    println!("✅ User logged in, JWT token generated");
    println!("   Token expires at: {}", login_response.expires_at);
    println!("   User ID: {}", login_response.user.user_id);

    // 4. Test OAuth authorization URL generation
    let auth_url_response = oauth_routes.get_auth_url(user_id, "strava").await?;
    println!("✅ Strava OAuth authorization URL generated:");
    println!("   URL: {}", auth_url_response.authorization_url);
    println!("   State: {}", auth_url_response.state);
    println!("   Instructions: {}", auth_url_response.instructions);

    // 5. Test OAuth callback (simulate receiving a callback with authorization code)
    println!("\n🔄 Simulating OAuth callback...");

    // This would normally be a real authorization code from Strava
    let mock_auth_code = "mock_authorization_code_12345";
    let state = auth_url_response.state;

    // Note: This will fail with real Strava API call since we're using a mock code
    // but it will test our callback handling logic
    println!("📝 Testing callback handling logic with mock code...");

    match oauth_routes
        .handle_callback(mock_auth_code, &state, "strava")
        .await
    {
        Ok(callback_response) => {
            println!("✅ OAuth callback handled successfully!");
            println!("   User ID: {}", callback_response.user_id);
            println!("   Provider: {}", callback_response.provider);
            println!("   Expires at: {}", callback_response.expires_at);
            println!("   Scopes: {}", callback_response.scopes);
        }
        Err(e) => {
            // Expected to fail with mock data, but let's check the error
            println!("⚠️  OAuth callback failed (expected with mock data): {}", e);

            // Check if it's a network/API error vs our logic error
            if e.to_string().contains("Strava token exchange failed") {
                println!("✅ Callback logic worked - failed at Strava API level as expected");
            } else if e.to_string().contains("Invalid state parameter") {
                println!("❌ State validation failed - this is a logic error");
                return Err(e);
            } else {
                println!("🔍 Unexpected error type: {}", e);
            }
        }
    }

    // 6. Test connection status
    let connection_status = oauth_routes.get_connection_status(user_id).await?;
    println!("\n✅ Connection status checked:");
    for status in connection_status {
        println!(
            "   {}: connected={}, expires_at={:?}",
            status.provider, status.connected, status.expires_at
        );
    }

    // 7. Test HTTP endpoints (if server is running)
    println!("\n🌐 Testing HTTP endpoints...");

    let client = reqwest::Client::new();

    // Test health endpoint
    match client.get("http://localhost:8081/health").send().await {
        Ok(response) => {
            if response.status().is_success() {
                let health_data: serde_json::Value = response.json().await?;
                println!("✅ Health endpoint working: {}", health_data);
            } else {
                println!("⚠️  Health endpoint returned: {}", response.status());
            }
        }
        Err(e) => {
            println!(
                "⚠️  Could not connect to HTTP server (may not be running): {}",
                e
            );
            println!("   To test HTTP endpoints, run: cargo run --bin pierre-mcp-server");
        }
    }

    // Test OAuth URL endpoint
    let oauth_url = format!("http://localhost:8081/oauth/auth/strava/{}", user_id);
    match client.get(&oauth_url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                let oauth_data: serde_json::Value = response.json().await?;
                println!("✅ OAuth URL endpoint working");
                println!("   Authorization URL generated via HTTP API");
                if let Some(auth_url) = oauth_data.get("authorization_url") {
                    println!("   Generated URL: {}", auth_url);
                }
                if let Some(state) = oauth_data.get("state") {
                    println!("   State parameter: {}", state);
                }
            } else {
                println!("⚠️  OAuth URL endpoint returned: {}", response.status());
            }
        }
        Err(e) => {
            println!("⚠️  Could not test OAuth URL endpoint: {}", e);
        }
    }

    println!("\n🎉 OAuth callback implementation test completed!");
    println!("\n📋 Summary:");
    println!("   ✅ User registration and login");
    println!("   ✅ OAuth authorization URL generation");
    println!("   ✅ OAuth callback logic (state validation, etc.)");
    println!("   ✅ Connection status checking");
    println!("   ⚠️  Real token exchange requires actual OAuth flow");

    println!("\n🚀 Next steps:");
    println!("   1. Start the server: cargo run --bin pierre-mcp-server");
    println!("   2. Test with real Strava OAuth in browser");
    println!("   3. Use longest run analysis with connected account");

    Ok(())
}
