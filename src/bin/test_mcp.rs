use anyhow::Result;
use pierre_mcp_server::config::{Config, ProviderConfig};
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    println!("Testing MCP Server with Strava credentials...\n");
    
    // Create config with Strava credentials
    let mut providers = HashMap::new();
    providers.insert("strava".to_string(), ProviderConfig {
        auth_type: "oauth2".to_string(),
        client_id: Some("163846".to_string()),
        client_secret: Some("1dfc45ad0a1f6983b835e4495aa9473d111d03bc".to_string()),
        access_token: None, // Would need real token from OAuth flow
        refresh_token: None,
        api_key: None,
    });
    
    let _config = Config { providers };
    println!("✅ Configuration created with Strava credentials");
    
    // Test MCP request format
    let test_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": null,
        "id": 1
    });
    
    println!("✅ MCP initialize request format:");
    println!("{}", serde_json::to_string_pretty(&test_request)?);
    
    // Test tool call format
    let tool_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_activities",
            "arguments": {
                "provider": "strava",
                "limit": 10
            }
        },
        "id": 2
    });
    
    println!("\n✅ MCP tool call request format:");
    println!("{}", serde_json::to_string_pretty(&tool_request)?);
    
    println!("\n🎉 MCP server configuration is ready!");
    println!("\nTo use the server:");
    println!("1. Get real OAuth tokens using: cargo run --bin auth-setup -- strava --client-id 163846 --client-secret 1dfc45ad0a1f6983b835e4495aa9473d111d03bc");
    println!("2. Run the server: cargo run");
    println!("3. Connect via MCP protocol on port 8080");
    
    Ok(())
}