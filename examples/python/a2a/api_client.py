#!/usr/bin/env python3
"""
A2A API Client Example
Scalable fitness data access via Agent-to-Agent protocol

Prerequisites:
1. Register an A2A client to get client_id and client_secret:
   curl -X POST http://localhost:8081/a2a/clients \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer <admin_jwt_token>" \
     -d '{
       "name": "My Fitness Bot",
       "description": "AI fitness assistant",
       "capabilities": ["fitness-data-analysis", "goal-management"],
       "contact_email": "developer@myapp.com"
     }'

2. Use the returned client_id and client_secret in this example

Environment Variables:
- PIERRE_A2A_CLIENT_ID: Your A2A client ID
- PIERRE_A2A_CLIENT_SECRET: Your A2A client secret
- PIERRE_API_BASE: API base URL (default: http://localhost:8081)
"""

import json
import os
import requests
import time
from datetime import datetime
from typing import List, Dict, Optional

class A2AClient:
    """A2A API client for business integration"""
    
    def __init__(self, base_url: str = 'http://localhost:8081', client_id: Optional[str] = None, 
                 client_secret: Optional[str] = None):
        self.base_url = base_url.rstrip('/')
        self.client_id = client_id
        self.client_secret = client_secret
        self.session_token = None
        self.session = requests.Session()
        
        # Set default headers
        self.session.headers.update({
            'Content-Type': 'application/json',
            'User-Agent': 'Pierre-A2A-Client/1.0'
        })
    
    def authenticate(self, scopes: List[str] = None) -> bool:
        """Authenticate with A2A client credentials"""
        if not self.client_id or not self.client_secret:
            print("❌ Client ID and secret required for A2A authentication")
            return False
            
        if scopes is None:
            scopes = ["read"]
            
        try:
            response = self.session.post(f'{self.base_url}/a2a/auth', json={
                'client_id': self.client_id,
                'client_secret': self.client_secret,
                'scopes': scopes
            })
            
            if response.status_code == 200:
                data = response.json()
                self.session_token = data.get('session_token')
                if self.session_token:
                    self.session.headers['Authorization'] = f'Bearer {self.session_token}'
                    print(f"✅ A2A authentication successful, expires in {data.get('expires_in')}s")
                    return True
            
            print(f"❌ A2A authentication failed: {response.status_code}")
            if response.status_code == 401:
                print("Check your client_id and client_secret")
            return False
            
        except Exception as e:
            print(f"❌ A2A authentication error: {e}")
            return False
    
    def get_agent_card(self) -> Dict:
        """Get A2A agent capability information"""
        try:
            response = self.session.get(f'{self.base_url}/a2a/agent-card')
            
            if response.status_code == 200:
                return response.json()
            else:
                print(f"❌ Failed to get agent card: {response.status_code}")
                return {}
                
        except Exception as e:
            print(f"❌ Agent card error: {e}")
            return {}
    
    def execute_tool(self, tool_name: str, parameters: Dict, timeout: int = 30) -> Dict:
        """Execute fitness tool via A2A protocol"""
        payload = {
            "jsonrpc": "2.0",
            "method": "tools.execute",
            "id": int(time.time()),
            "params": {
                "tool_name": tool_name,
                "parameters": parameters
            }
        }
        
        try:
            response = self.session.post(
                f'{self.base_url}/a2a/execute',
                json=payload,
                timeout=timeout
            )
            
            if response.status_code == 200:
                return response.json()
            else:
                return {
                    'error': f'HTTP {response.status_code}: {response.text}'
                }
                
        except requests.exceptions.Timeout:
            return {'error': f'Request timeout after {timeout}s'}
        except Exception as e:
            return {'error': f'Request failed: {e}'}
    
    def get_activities(self, provider: str = 'strava', limit: int = 100) -> List[Dict]:
        """Get activities via A2A protocol"""
        print(f"🔄 Fetching {limit} activities from {provider} via A2A...")
        
        start_time = time.time()
        result = self.execute_tool('get_activities', {
            'provider': provider,
            'limit': limit
        })
        
        fetch_time = time.time() - start_time
        
        if 'error' in result:
            print(f"❌ Failed to get activities: {result['error']}")
            return []
        
        # Handle A2A response format
        activities = []
        if 'result' in result:
            data = result['result']
            if isinstance(data, list):
                activities = data
            elif isinstance(data, dict) and 'activities' in data:
                activities = data['activities']
        
        rate = len(activities) / fetch_time if fetch_time > 0 else 0
        print(f"✅ Retrieved {len(activities)} activities in {fetch_time:.2f}s ({rate:.1f} activities/sec)")
        
        return activities
    
    def calculate_fitness_score(self, provider: str = 'strava') -> Dict:
        """Calculate fitness score via A2A"""
        print(f"🤖 Calculating fitness score...")
        
        result = self.execute_tool('calculate_fitness_score', {
            'provider': provider
        })
        
        if 'error' in result:
            print(f"❌ Fitness calculation failed: {result['error']}")
            return {}
        
        return result.get('result', {})
    
    def generate_recommendations(self, provider: str = 'strava') -> List[Dict]:
        """Generate training recommendations via A2A"""
        print(f"💡 Generating training recommendations...")
        
        result = self.execute_tool('generate_recommendations', {
            'provider': provider
        })
        
        if 'error' in result:
            print(f"❌ Recommendation generation failed: {result['error']}")
            return []
        
        return result.get('result', {}).get('training_recommendations', [])
    
    def analyze_training_load(self, provider: str = 'strava') -> Dict:
        """Analyze training load via A2A"""
        print(f"⚖️ Analyzing training load...")
        
        result = self.execute_tool('analyze_training_load', {
            'provider': provider
        })
        
        if 'error' in result:
            print(f"❌ Training load analysis failed: {result['error']}")
            return {}
        
        return result.get('result', {}).get('training_load_analysis', {})
    
    def get_api_usage_stats(self) -> Dict:
        """Get API usage statistics"""
        try:
            response = self.session.get(f'{self.base_url}/a2a/usage')
            
            if response.status_code == 200:
                return response.json()
            else:
                return {'error': f'HTTP {response.status_code}'}
                
        except Exception as e:
            return {'error': f'Request failed: {e}'}

def main():
    """Example A2A client usage"""
    print("🚀 A2A API Client Example")
    print("=" * 40)
    
    # Example 1: Get agent capabilities (no auth required)
    client = A2AClient()
    agent_card = client.get_agent_card()
    if agent_card:
        print(f"🤖 Agent: {agent_card.get('name', 'Pierre Fitness AI')}")
        print(f"📋 Capabilities: {', '.join(agent_card.get('capabilities', []))}")
        print()
    
    # Example 2: Initialize client with A2A credentials
    # Get credentials from environment variables or use demo values
    client_id = os.getenv('PIERRE_A2A_CLIENT_ID', 'demo_client_123')
    client_secret = os.getenv('PIERRE_A2A_CLIENT_SECRET', 'demo_secret_456')
    base_url = os.getenv('PIERRE_API_BASE', 'http://localhost:8081')
    
    client = A2AClient(base_url=base_url, client_id=client_id, client_secret=client_secret)
    
    # Authenticate with A2A protocol
    if not client.authenticate(scopes=["read", "write"]):
        print("❌ A2A authentication failed")
        print("💡 Make sure you have registered an A2A client first")
        return
    
    # Get activities
    activities = client.get_activities(limit=50)
    
    if activities:
        print(f"\n📊 Analysis of {len(activities)} activities:")
        
        # Calculate fitness score
        fitness_data = client.calculate_fitness_score()
        if fitness_data:
            score = fitness_data.get('fitness_score', {}).get('overall_score', 0)
            print(f"🏆 Fitness Score: {score}/100")
        
        # Get recommendations
        recommendations = client.generate_recommendations()
        if recommendations:
            print(f"💡 Training Recommendations:")
            for i, rec in enumerate(recommendations[:3], 1):
                title = rec.get('title', 'Recommendation')
                priority = rec.get('priority', 'medium').upper()
                print(f"   {i}. [{priority}] {title}")
        
        # Analyze training load
        load_data = client.analyze_training_load()
        if load_data:
            load_level = load_data.get('load_level', 'unknown')
            weekly_hours = load_data.get('weekly_hours', 0)
            print(f"⚖️ Training Load: {load_level.upper()} ({weekly_hours:.1f}h/week)")
    
    # Check API usage
    usage_stats = client.get_api_usage_stats()
    if 'error' not in usage_stats:
        print(f"\n📈 API Usage Stats: {usage_stats}")
    
    print(f"\n✅ A2A demo complete!")

if __name__ == "__main__":
    main()