#!/usr/bin/env python3
# ABOUTME: Advanced fitness analysis example using Pierre MCP Python client
# ABOUTME: Demonstrates complex analysis patterns, date filtering, and performance metrics

"""
Pierre MCP Advanced Analysis Example

This example demonstrates:
1. Date-based activity filtering
2. Performance trend analysis
3. Activity comparison
4. Statistical analysis
"""

import asyncio
import os
from datetime import datetime, timedelta
from pierre_mcp import PierreMCPClient


async def analyze_running_performance(client: PierreMCPClient):
    """Analyze running performance over different time periods"""
    print("=== Running Performance Analysis ===")
    
    # Get activities from last 30 days
    end_date = datetime.now()
    start_date = end_date - timedelta(days=30)
    
    activities = await client.call_tool(
        "get_activities",
        {
            "provider": "strava",
            "after": start_date.isoformat(),
            "before": end_date.isoformat(),
            "activity_type": "Run"
        }
    )
    
    if not activities:
        print("No running activities found in the last 30 days")
        return
    
    print(f"Found {len(activities)} running activities")
    
    # Calculate basic stats
    total_distance = sum(activity.get('distance', 0) for activity in activities)
    total_time = sum(activity.get('moving_time', 0) for activity in activities)
    
    print(f"Total distance: {total_distance/1000:.2f} km")
    print(f"Total time: {total_time/3600:.2f} hours")
    print(f"Average pace: {(total_time/60)/(total_distance/1000):.2f} min/km")
    
    # Analyze longest run
    longest_run = max(activities, key=lambda x: x.get('distance', 0))
    print(f"\nLongest run: {longest_run.get('name')} - {longest_run.get('distance', 0)/1000:.2f} km")
    
    # Get detailed analysis for longest run
    analysis = await client.call_tool(
        "analyze_activity",
        {"activity_id": longest_run.get('id'), "provider": "strava"}
    )
    print(f"Longest run analysis: {analysis}")


async def compare_activities(client: PierreMCPClient):
    """Compare different types of activities"""
    print("\n=== Activity Comparison ===")
    
    # Get different activity types
    for activity_type in ["Run", "Ride", "Swim"]:
        activities = await client.call_tool(
            "get_activities",
            {
                "provider": "strava",
                "limit": 10,
                "activity_type": activity_type
            }
        )
        
        if activities:
            total_distance = sum(activity.get('distance', 0) for activity in activities)
            avg_distance = total_distance / len(activities) if activities else 0
            print(f"{activity_type}: {len(activities)} activities, avg distance: {avg_distance/1000:.2f} km")
        else:
            print(f"{activity_type}: No activities found")


async def analyze_segments(client: PierreMCPClient):
    """Analyze segment performance"""
    print("\n=== Segment Analysis ===")
    
    # Get recent activities to find segments
    activities = await client.call_tool(
        "get_activities",
        {"provider": "strava", "limit": 5}
    )
    
    if not activities:
        print("No activities found for segment analysis")
        return
    
    # Analyze segments for the most recent activity
    recent_activity = activities[0]
    print(f"Analyzing segments for: {recent_activity.get('name')}")
    
    segments = await client.call_tool(
        "get_segments",
        {"activity_id": recent_activity.get('id'), "provider": "strava"}
    )
    
    if segments:
        print(f"Found {len(segments)} segments")
        for i, segment in enumerate(segments[:3]):  # Show first 3 segments
            print(f"  Segment {i+1}: {segment.get('name', 'Unnamed')} - {segment.get('elapsed_time', 0)} seconds")
    else:
        print("No segments found")


async def get_athlete_overview(client: PierreMCPClient):
    """Get overall athlete statistics"""
    print("\n=== Athlete Overview ===")
    
    stats = await client.call_tool(
        "get_athlete_stats",
        {"provider": "strava"}
    )
    
    if stats:
        print("Recent totals:")
        recent_totals = stats.get('recent_run_totals', {})
        print(f"  Runs: {recent_totals.get('count', 0)} activities")
        print(f"  Distance: {recent_totals.get('distance', 0)/1000:.2f} km")
        print(f"  Time: {recent_totals.get('elapsed_time', 0)/3600:.2f} hours")
        
        ytd_totals = stats.get('ytd_run_totals', {})
        print(f"Year to date:")
        print(f"  Runs: {ytd_totals.get('count', 0)} activities")
        print(f"  Distance: {ytd_totals.get('distance', 0)/1000:.2f} km")
    else:
        print("No athlete stats available")


async def main():
    # Configuration
    server_url = os.getenv("PIERRE_SERVER_URL", "http://localhost:8081")
    tenant_id = os.getenv("PIERRE_TENANT_ID")
    jwt_token = os.getenv("PIERRE_JWT_TOKEN")
    
    if not tenant_id or not jwt_token:
        print("Error: Set PIERRE_TENANT_ID and PIERRE_JWT_TOKEN environment variables")
        return
    
    try:
        async with PierreMCPClient(
            server_url=server_url,
            tenant_id=tenant_id,
            jwt_token=jwt_token
        ) as client:
            print(f"Connected to Pierre MCP Server for advanced analysis")
            
            # Check OAuth status first
            oauth_status = await client.get_oauth_status("strava")
            if not oauth_status.get("connected"):
                print("Error: Strava OAuth not connected")
                auth_url = await client.get_authorization_url("strava")
                print(f"Connect at: {auth_url}")
                return
            
            # Run all analysis functions
            await analyze_running_performance(client)
            await compare_activities(client)
            await analyze_segments(client)
            await get_athlete_overview(client)
            
            print("\n=== Analysis Complete ===")
    
    except Exception as e:
        print(f"Error during analysis: {e}")


if __name__ == "__main__":
    asyncio.run(main())