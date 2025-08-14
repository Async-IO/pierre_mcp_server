#!/usr/bin/env python3
# ABOUTME: Basic usage example for Pierre MCP Python client
# ABOUTME: Demonstrates tenant setup, OAuth connection, and activity analysis

"""
Pierre MCP Basic Usage Example

This example shows how to:
1. Connect to Pierre MCP Server
2. Check OAuth status
3. Fetch and analyze activities
4. Handle errors properly
"""

import asyncio
import os
import requests
from typing import Optional, Dict, Any


def main():
    # Configuration from environment variables
    http_base_url = os.getenv("PIERRE_HTTP_URL", "http://localhost:8081")
    mcp_base_url = os.getenv("PIERRE_MCP_URL", "http://localhost:8080")
    api_key = os.getenv("PIERRE_API_KEY")
    
    if not api_key:
        print("Error: Set PIERRE_API_KEY environment variable")
        print("Example:")
        print("  export PIERRE_API_KEY=pk_live_YOUR_API_KEY")
        return
    
    try:
        print(f"Testing Pierre MCP Server at {mcp_base_url}")
        
        # Test MCP tools/list endpoint
        mcp_request = {
            "jsonrpc": "2.0",
            "method": "tools/list",
            "params": {},
            "id": 1
        }
        
        response = requests.post(
            f"{mcp_base_url}/mcp",
            headers={
                "Authorization": api_key,
                "Content-Type": "application/json"
            },
            json=mcp_request
        )
        
        if response.status_code == 200:
            result = response.json()
            tools = result.get("result", {}).get("tools", [])
            print(f"Connected to Pierre MCP Server")
            
            # List available tools
            print("\nAvailable tools:")
            for tool in tools:
                print(f"  - {tool['name']}: {tool.get('description', 'No description')}")
            
            # Check OAuth connection status
            print("\nChecking OAuth status...")
            status_request = {
                "jsonrpc": "2.0",
                "method": "tools/call",
                "params": {
                    "name": "get_connection_status",
                    "arguments": {}
                },
                "id": 2
            }
            
            status_response = requests.post(
                f"{mcp_base_url}/mcp",
                headers={
                    "Authorization": api_key,
                    "Content-Type": "application/json"
                },
                json=status_request
            )
            
            if status_response.status_code == 200:
                status_result = status_response.json()
                connection_status = status_result.get("result", [])
                
                strava_connected = False
                if isinstance(connection_status, list):
                    for provider in connection_status:
                        if provider.get("provider") == "strava" and provider.get("connected"):
                            strava_connected = True
                            print("✓ Strava OAuth connected")
                            break
                
                if not strava_connected:
                    print("✗ Strava OAuth not connected")
                    print("Configure OAuth credentials in database first")
                    return
            else:
                print(f"✗ Failed to check OAuth status: {status_response.text}")
                return
            
            # Get recent activities
            print("\nFetching recent activities...")
            activities_request = {
                "jsonrpc": "2.0",
                "method": "tools/call",
                "params": {
                    "name": "get_activities",
                    "arguments": {"provider": "strava", "limit": 5}
                },
                "id": 3
            }
            
            activities_response = requests.post(
                f"{mcp_base_url}/mcp",
                headers={
                    "Authorization": api_key,
                    "Content-Type": "application/json"
                },
                json=activities_request
            )
            
            if activities_response.status_code == 200:
                activities_result = activities_response.json()
                activities = activities_result.get("result", [])
                
                if activities:
                    print(f"Found {len(activities)} recent activities:")
                    for activity in activities[:3]:  # Show first 3
                        print(f"  - {activity.get('name', 'Unnamed')}: {activity.get('distance', 0)} meters")
                else:
                    print("No activities found")
            else:
                print(f"✗ Failed to fetch activities: {activities_response.text}")
        else:
            print(f"✗ Failed to connect to MCP server: {response.text}")
    
    except Exception as e:
        print(f"Error: {e}")
        print("Make sure the server is running and API key is valid")


if __name__ == "__main__":
    main()