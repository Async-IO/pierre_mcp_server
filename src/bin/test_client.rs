use anyhow::Result;
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing MCP Server...\n");
    
    // Connect to the MCP server
    let stream = TcpStream::connect("127.0.0.1:8080").await?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    
    // Test 1: Initialize request
    let init_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": null,
        "id": 1
    });
    
    println!("🔄 Sending initialize request...");
    let request_str = format!("{}\n", init_request.to_string());
    writer.write_all(request_str.as_bytes()).await?;
    
    // Read response
    let mut response = String::new();
    reader.read_line(&mut response).await?;
    println!("📥 Initialize response: {}", response.trim());
    
    // Test 2: Tool call request (will fail without auth, but tests the protocol)
    let tool_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_activities",
            "arguments": {
                "provider": "strava",
                "limit": 5
            }
        },
        "id": 2
    });
    
    println!("\n🔄 Sending tool call request...");
    let tool_str = format!("{}\n", tool_request.to_string());
    writer.write_all(tool_str.as_bytes()).await?;
    
    // Read response
    let mut tool_response = String::new();
    reader.read_line(&mut tool_response).await?;
    println!("📥 Tool call response: {}", tool_response.trim());
    
    println!("\n✅ MCP protocol test completed!");
    
    Ok(())
}