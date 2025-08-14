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
from pierre_mcp import PierreMCPClient, AuthenticationError, TenantError


async def main():
    # Configuration from environment variables
    server_url = os.getenv("PIERRE_SERVER_URL", "http://localhost:8081")
    tenant_id = os.getenv("PIERRE_TENANT_ID")
    jwt_token = os.getenv("PIERRE_JWT_TOKEN")
    
    if not tenant_id or not jwt_token:
        print("Error: Set PIERRE_TENANT_ID and PIERRE_JWT_TOKEN environment variables")
        print("Example:")
        print("  export PIERRE_TENANT_ID=your-tenant-id")
        print("  export PIERRE_JWT_TOKEN=your-jwt-token")
        return
    
    try:
        # Connect to Pierre MCP Server
        async with PierreMCPClient(
            server_url=server_url,
            tenant_id=tenant_id,
            jwt_token=jwt_token
        ) as client:
            print(f"Connected to Pierre MCP Server at {server_url}")
            
            # List available tools
            print("\nAvailable tools:")
            tools = await client.list_tools()
            for tool in tools:
                print(f"  - {tool['name']}: {tool.get('description', 'No description')}")
            
            # Check OAuth status via MCP protocol
            print("\nChecking OAuth status...")
            try:
                # Use get_connection_status tool instead of direct OAuth endpoints
                connection_status = await client.call_tool("get_connection_status", {})
                
                # Check if Strava is connected
                strava_connected = False
                if isinstance(connection_status, list):
                    for provider in connection_status:
                        if provider.get("provider") == "strava" and provider.get("connected"):
                            strava_connected = True
                            print("✓ Strava OAuth connected")
                            break
                
                if not strava_connected:
                    print("✗ Strava OAuth not connected")
                    try:
                        auth_url = await client.get_authorization_url("strava")
                        print(f"Connect at: {auth_url}")
                    except TenantError:
                        print("✗ Tenant OAuth not configured. Configure with:")
                        print(f"  curl -X POST {server_url}/api/tenants/{tenant_id}/oauth \\")
                        print('    -H "Authorization: Bearer {jwt_token}" \\')
                        print('    -d \'{"provider": "strava", "client_id": "...", "client_secret": "...", "redirect_uri": "...", "scopes": ["read", "activity:read_all"]}\'')
                    return
            except Exception as e:
                print(f"✗ Failed to check OAuth status: {e}")
                print("Make sure tenant is configured and JWT token is valid")
                return
            
            # Get recent activities
            print("\nFetching recent activities...")
            activities = await client.call_tool(
                "get_activities",
                {"provider": "strava", "limit": 5}
            )
            
            if activities:
                print(f"Found {len(activities)} recent activities:")
                for activity in activities[:3]:  # Show first 3
                    print(f"  - {activity.get('name', 'Unnamed')}: {activity.get('distance', 0)} meters")
                
                # Analyze first activity
                if activities:
                    activity_id = activities[0].get('id')
                    if activity_id:
                        print(f"\nAnalyzing activity {activity_id}...")
                        analysis = await client.call_tool(
                            "analyze_activity",
                            {"activity_id": activity_id, "provider": "strava"}
                        )
                        print(f"Analysis: {analysis}")
            else:
                print("No activities found")
    
    except AuthenticationError:
        print("Error: Invalid JWT token. Generate a new one with:")
        print(f"  curl -X POST {server_url}/api/tenants/{tenant_id}/jwt \\")
        print('    -d \'{"scopes": ["fitness:read", "activity:read"]}\'')
    
    except TenantError as e:
        print(f"Error: Tenant issue - {e}")
        print("Make sure your tenant exists and you have access")
    
    except Exception as e:
        print(f"Unexpected error: {e}")


if __name__ == "__main__":
    asyncio.run(main())