#!/usr/bin/env python3
# ABOUTME: Simple Pierre MCP Server example using direct HTTP requests
# ABOUTME: Demonstrates API key authentication and basic tool usage

"""
Simple Pierre MCP Server Example

This example shows how to use the Pierre MCP Server with direct HTTP requests:
1. Connect using API key authentication
2. List available tools
3. Check OAuth status  
4. Fetch activities if connected

Prerequisites:
- Server running: cargo run --bin pierre-mcp-server
- User registered with API key created
- OAuth credentials configured in database (optional)

Usage:
    export PIERRE_API_KEY=pk_live_YOUR_API_KEY
    python3 simple_mcp_example.py
"""

import json
import os
import requests
from typing import Dict, Any

def send_mcp_request(mcp_url: str, api_key: str, method: str, params: Dict[str, Any], id: int = 1) -> Dict[str, Any]:
    """Send an MCP JSON-RPC request"""
    request = {
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": id
    }
    
    response = requests.post(
        f"{mcp_url}/mcp",
        headers={
            "Authorization": api_key,
            "Content-Type": "application/json"
        },
        json=request
    )
    
    if response.status_code == 200:
        return response.json()
    else:
        raise Exception(f"HTTP {response.status_code}: {response.text}")

def main():
    # Configuration
    mcp_url = os.getenv("PIERRE_MCP_URL", "http://localhost:8080")
    api_key = os.getenv("PIERRE_API_KEY")
    
    if not api_key:
        print("Error: Set PIERRE_API_KEY environment variable")
        print("Example: export PIERRE_API_KEY=pk_live_YOUR_API_KEY")
        return
    
    try:
        print("🚀 Pierre MCP Server Simple Example")
        print("=" * 40)
        
        # List available tools
        print("\n📋 Listing available tools...")
        tools_result = send_mcp_request(mcp_url, api_key, "tools/list", {})
        
        if "result" in tools_result:
            tools = tools_result["result"]["tools"]
            print(f"Found {len(tools)} tools:")
            for tool in tools[:5]:  # Show first 5
                print(f"  • {tool['name']}: {tool.get('description', 'No description')[:60]}...")
            if len(tools) > 5:
                print(f"  ... and {len(tools) - 5} more tools")
        
        # Check connection status
        print("\n🔍 Checking OAuth connection status...")
        status_result = send_mcp_request(
            mcp_url, api_key, 
            "tools/call", 
            {"name": "get_connection_status", "arguments": {}},
            id=2
        )
        
        if "result" in status_result:
            connections = status_result["result"]
            if isinstance(connections, list):
                for conn in connections:
                    provider = conn.get("provider", "Unknown")
                    connected = "✅ Connected" if conn.get("connected") else "❌ Not connected"
                    print(f"  {provider}: {connected}")
            else:
                print(f"  Status: {connections}")
        
        # Try to get activities (will only work if OAuth is configured)
        print("\n🏃 Attempting to fetch activities...")
        try:
            activities_result = send_mcp_request(
                mcp_url, api_key,
                "tools/call",
                {"name": "get_activities", "arguments": {"provider": "strava", "limit": 3}},
                id=3
            )
            
            if "result" in activities_result:
                activities = activities_result["result"]
                if activities:
                    print(f"  Found {len(activities)} recent activities:")
                    for activity in activities:
                        name = activity.get("name", "Unnamed")
                        distance = activity.get("distance", 0)
                        print(f"    • {name}: {distance}m")
                else:
                    print("  No activities found")
            elif "error" in activities_result:
                error = activities_result["error"]
                print(f"  ❌ Error: {error.get('message', 'Unknown error')}")
        except Exception as e:
            print(f"  ❌ Failed to fetch activities: {e}")
            print("  (This is expected if OAuth is not configured)")
        
        print("\n✅ Example completed successfully!")
        print("\nNext steps:")
        print("1. Configure OAuth credentials in database")
        print("2. Complete OAuth flow for fitness providers")
        print("3. Use MCP tools to analyze fitness data")
        
    except Exception as e:
        print(f"\n❌ Error: {e}")
        print("Make sure the server is running and API key is valid")

if __name__ == "__main__":
    main()