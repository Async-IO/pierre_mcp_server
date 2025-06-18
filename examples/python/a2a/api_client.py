#!/usr/bin/env python3
"""
A2A API Client Example
Enterprise-grade fitness data access via Agent-to-Agent protocol
"""

import json
import requests
import time
from datetime import datetime
from typing import List, Dict, Optional

class A2AClient:
    """Professional A2A API client for enterprise integration"""
    
    def __init__(self, base_url: str = 'http://localhost:8081', api_key: Optional[str] = None):
        self.base_url = base_url.rstrip('/')
        self.api_key = api_key
        self.session = requests.Session()
        
        # Set default headers
        self.session.headers.update({
            'Content-Type': 'application/json',
            'User-Agent': 'Pierre-A2A-Client/1.0'
        })
        
        if self.api_key:
            self.session.headers['Authorization'] = f'Bearer {self.api_key}'
    
    def authenticate_with_jwt(self, email: str, password: str) -> bool:
        """Authenticate and get JWT token (for demo purposes)"""
        try:
            response = self.session.post(f'{self.base_url}/auth/login', json={
                'email': email,
                'password': password
            })
            
            if response.status_code == 200:
                data = response.json()
                jwt_token = data.get('jwt_token')
                if jwt_token:
                    self.session.headers['Authorization'] = f'Bearer {jwt_token}'
                    print("✅ JWT authentication successful")
                    return True
            
            print(f"❌ Authentication failed: {response.status_code}")
            return False
            
        except Exception as e:
            print(f"❌ Authentication error: {e}")
            return False
    
    def create_api_key(self, name: str, description: str = "", tier: str = "trial") -> Optional[str]:
        """Create a new API key via A2A endpoint"""
        try:
            response = self.session.post(f'{self.base_url}/api/keys', json={
                'name': name,
                'description': description,
                'tier': tier
            })
            
            if response.status_code == 200:
                data = response.json()
                api_key = data.get('api_key')
                print(f"✅ Created API key: {data.get('key_info', {}).get('key_prefix', 'Unknown')}")
                return api_key
            else:
                print(f"❌ API key creation failed: {response.status_code}")
                return None
                
        except Exception as e:
            print(f"❌ API key creation error: {e}")
            return None
    
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
    
    # Initialize client
    client = A2AClient()
    
    # Authenticate (for demo - in production use API keys)
    if not client.authenticate_with_jwt('test@example.com', 'password123'):
        print("❌ Authentication failed")
        return
    
    # Optional: Create API key for production use
    api_key = client.create_api_key('A2A Demo Key', 'Example A2A integration')
    if api_key:
        print(f"🔑 API Key created (use this for production): {api_key[:12]}...")
    
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