#!/usr/bin/env python3
"""
Complete Multi-Tenant MCP Server Example

This example demonstrates the complete workflow for using the Pierre MCP Server
in multi-tenant mode with proper authentication, OAuth integration, and MCP protocol usage.

Prerequisites:
1. Server running: cargo run --bin pierre-mcp-server
2. Database cleaned: ./scripts/fresh-start.sh
3. Admin token generated: cargo run --bin admin-setup generate-token --service "demo"

Usage:
    python3 multitenant_mcp_example.py
"""

import json
import requests
import sys
import time
from typing import Optional, Dict, Any

class PierreMCPClient:
    """
    Pierre MCP Server Client for Multi-Tenant Mode
    
    Supports both HTTP and stdio transports with JWT authentication
    """
    
    def __init__(self, 
                 http_base_url: str = "http://localhost:8081",
                 mcp_base_url: str = "http://localhost:8080"):
        self.http_base_url = http_base_url
        self.mcp_base_url = mcp_base_url
        self.jwt_token: Optional[str] = None
        self.user_id: Optional[str] = None
        
    def register_user(self, email: str, password: str, display_name: str) -> Dict[str, Any]:
        """Register a new user account"""
        print(f"📝 Registering user: {email}")
        
        response = requests.post(
            f"{self.http_base_url}/auth/register",
            json={
                "email": email,
                "password": password,
                "display_name": display_name
            }
        )
        
        if response.status_code == 200:
            data = response.json()
            self.user_id = data["user_id"]
            print(f"✅ User registered successfully: {self.user_id}")
            return data
        else:
            print(f"❌ Registration failed: {response.text}")
            raise Exception(f"Registration failed: {response.text}")
    
    def login(self, email: str, password: str) -> Dict[str, Any]:
        """Login and get JWT token"""
        print(f"🔐 Logging in user: {email}")
        
        response = requests.post(
            f"{self.http_base_url}/auth/login",
            json={
                "email": email,
                "password": password
            }
        )
        
        if response.status_code == 200:
            data = response.json()
            self.jwt_token = data["jwt_token"]
            self.user_id = data["user"]["user_id"]
            print(f"✅ Login successful! Token expires: {data['expires_at']}")
            return data
        else:
            print(f"❌ Login failed: {response.text}")
            raise Exception(f"Login failed: {response.text}")
    
    def setup_strava_oauth(self) -> str:
        """Setup Strava OAuth and return authorization URL"""
        if not self.user_id:
            raise Exception("User ID not available. Please login first.")
            
        print(f"🔗 Setting up Strava OAuth for user: {self.user_id}")
        
        response = requests.get(f"{self.http_base_url}/oauth/auth/strava/{self.user_id}")
        
        if response.status_code == 200:
            data = response.json()
            auth_url = data["authorization_url"]
            print(f"✅ Strava OAuth URL generated")
            print(f"🌐 Visit this URL to authorize: {auth_url}")
            print(f"⏰ URL expires in: {data['expires_in_minutes']} minutes")
            return auth_url
        else:
            print(f"❌ OAuth setup failed: {response.text}")
            raise Exception(f"OAuth setup failed: {response.text}")
    
    def check_connection_status(self) -> Dict[str, Any]:
        """Check OAuth connection status using MCP protocol"""
        print("🔍 Checking connection status...")
        
        mcp_request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "auth": f"Bearer {self.jwt_token}",
            "params": {
                "name": "get_connection_status",
                "arguments": {}
            }
        }
        
        return self._send_mcp_request(mcp_request)
    
    def get_athlete_profile(self, provider: str = "strava") -> Dict[str, Any]:
        """Get athlete profile using MCP protocol"""
        print(f"👤 Getting athlete profile from {provider}...")
        
        mcp_request = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "auth": f"Bearer {self.jwt_token}",
            "params": {
                "name": "get_athlete",
                "arguments": {"provider": provider}
            }
        }
        
        return self._send_mcp_request(mcp_request)
    
    def get_activities(self, provider: str = "strava", limit: int = 5) -> Dict[str, Any]:
        """Get recent activities using MCP protocol"""
        print(f"🏃 Getting {limit} recent activities from {provider}...")
        
        mcp_request = {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "auth": f"Bearer {self.jwt_token}",
            "params": {
                "name": "get_activities",
                "arguments": {
                    "provider": provider,
                    "limit": limit
                }
            }
        }
        
        return self._send_mcp_request(mcp_request)
    
    def get_activity_intelligence(self, activity_id: str, provider: str = "strava") -> Dict[str, Any]:
        """Get AI-powered activity intelligence using MCP protocol"""
        print(f"🧠 Getting activity intelligence for {activity_id}...")
        
        mcp_request = {
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "auth": f"Bearer {self.jwt_token}",
            "params": {
                "name": "get_activity_intelligence",
                "arguments": {
                    "provider": provider,
                    "activity_id": activity_id,
                    "include_weather": True,
                    "include_location": True
                }
            }
        }
        
        return self._send_mcp_request(mcp_request)
    
    def get_stats(self, provider: str = "strava") -> Dict[str, Any]:
        """Get fitness statistics using MCP protocol"""
        print(f"📊 Getting fitness stats from {provider}...")
        
        mcp_request = {
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "auth": f"Bearer {self.jwt_token}",
            "params": {
                "name": "get_stats",
                "arguments": {"provider": provider}
            }
        }
        
        return self._send_mcp_request(mcp_request)
    
    def get_recommendations(self, provider: str = "strava") -> Dict[str, Any]:
        """Get training recommendations using MCP protocol"""
        print(f"💡 Getting training recommendations from {provider}...")
        
        mcp_request = {
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "auth": f"Bearer {self.jwt_token}",
            "params": {
                "name": "generate_recommendations",
                "arguments": {
                    "provider": provider,
                    "recommendation_type": "training"
                }
            }
        }
        
        return self._send_mcp_request(mcp_request)
    
    def _send_mcp_request(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """Send MCP request via HTTP transport"""
        if not self.jwt_token:
            raise Exception("JWT token not available. Please login first.")
        
        try:
            response = requests.post(
                f"{self.mcp_base_url}/mcp",
                headers={
                    "Content-Type": "application/json",
                    "Accept": "application/json",
                    "Origin": "http://localhost"
                },
                json=request,
                timeout=30
            )
            
            if response.status_code == 202:  # MCP HTTP returns 202 Accepted
                result = response.json()
                
                if result.get("error"):
                    print(f"❌ MCP Error: {result['error']['message']}")
                    return result
                else:
                    print(f"✅ MCP Response received")
                    return result
            else:
                print(f"❌ HTTP Error: {response.status_code} - {response.text}")
                raise Exception(f"HTTP Error: {response.status_code}")
                
        except Exception as e:
            print(f"❌ Request failed: {e}")
            raise

def main():
    """
    Complete multi-tenant MCP example workflow
    """
    print("🚀 Pierre MCP Server Multi-Tenant Example")
    print("=" * 50)
    
    # Initialize client
    client = PierreMCPClient()
    
    # User credentials
    email = "demo@example.com"
    password = "password123"
    display_name = "Demo User"
    
    try:
        # Step 1: Register user
        client.register_user(email, password, display_name)
        
        # Step 2: Login and get JWT token
        login_data = client.login(email, password)
        print(f"🔑 JWT Token: {client.jwt_token[:50]}...")
        
        # Step 3: Setup Strava OAuth
        auth_url = client.setup_strava_oauth()
        print(f"\\n⚠️  MANUAL STEP REQUIRED:")
        print(f"   1. Visit: {auth_url}")
        print(f"   2. Authorize the application")
        print(f"   3. Wait for redirect to complete")
        print(f"   4. Press Enter to continue...")
        input()
        
        # Step 4: Check connection status
        connection_status = client.check_connection_status()
        if connection_status.get("result"):
            for provider in connection_status["result"]:
                status = "✅ Connected" if provider["connected"] else "❌ Disconnected"
                print(f"   {provider['provider']}: {status}")
        
        # Step 5: Get athlete profile
        athlete_data = client.get_athlete_profile()
        if athlete_data.get("result"):
            athlete = athlete_data["result"]
            print(f"   Name: {athlete.get('firstname', '')} {athlete.get('lastname', '')}")
            print(f"   ID: {athlete.get('id')}")
            print(f"   Username: {athlete.get('username')}")
        
        # Step 6: Get recent activities
        activities_data = client.get_activities(limit=3)
        if activities_data.get("result"):
            activities = activities_data["result"]
            print(f"   Found {len(activities)} activities")
            
            # Step 7: Get intelligence for first activity
            if activities:
                first_activity = activities[0]
                activity_id = first_activity["id"]
                print(f"   Analyzing activity: {first_activity.get('name', 'Unnamed')}")
                
                intelligence_data = client.get_activity_intelligence(activity_id)
                if intelligence_data.get("result"):
                    intelligence = intelligence_data["result"]
                    print(f"   Activity: {intelligence.get('activity_name')}")
                    print(f"   Summary: {intelligence.get('summary')}")
                    
                    # Show key insights
                    insights = intelligence.get("key_insights", [])
                    if insights:
                        print(f"   Key Insights:")
                        for insight in insights[:2]:  # Show first 2 insights
                            print(f"     • {insight.get('message')}")
        
        # Step 8: Get fitness stats
        stats_data = client.get_stats()
        if stats_data.get("result"):
            stats = stats_data["result"]
            print(f"   Total Activities: {stats.get('total_activities', 0):,}")
            print(f"   Total Distance: {stats.get('total_distance', 0)/1000:.1f} km")
            print(f"   Total Duration: {stats.get('total_duration', 0)/3600:.1f} hours")
        
        # Step 9: Get training recommendations
        recommendations_data = client.get_recommendations()
        if recommendations_data.get("result"):
            recommendations = recommendations_data["result"]
            training_recs = recommendations.get("training_recommendations", [])
            if training_recs:
                print(f"   Training Recommendations:")
                for rec in training_recs[:2]:  # Show first 2 recommendations
                    print(f"     • {rec.get('title')}: {rec.get('description')}")
        
        print("\\n🎉 Multi-tenant MCP example completed successfully!")
        print("\\n📋 Summary:")
        print("   ✅ User registration and authentication")
        print("   ✅ JWT token management")
        print("   ✅ Strava OAuth integration")
        print("   ✅ MCP protocol usage (HTTP transport)")
        print("   ✅ Real fitness data analysis")
        print("   ✅ Rate limiting and error handling")
        
    except Exception as e:
        print(f"\\n❌ Example failed: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()