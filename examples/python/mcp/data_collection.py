#!/usr/bin/env python3
"""
MCP Data Collection Example
Efficient bulk collection of fitness activities via Model Context Protocol
"""

import json
import socket
import time
from datetime import datetime
from typing import List, Dict, Optional

class MCPDataCollector:
    """Efficient data collection via MCP protocol"""
    
    def __init__(self, host: str = 'localhost', port: int = 8080):
        self.host = host
        self.port = port
        self.sock = None
        self.jwt_token = None
        
    def connect(self) -> bool:
        """Connect to MCP server and authenticate"""
        try:
            self.sock = socket.create_connection((self.host, self.port))
            
            # Get JWT token
            if not self._get_jwt_token():
                return False
                
            # Initialize MCP connection
            return self._initialize_mcp()
            
        except Exception as e:
            print(f"❌ Connection failed: {e}")
            return False
    
    def _get_jwt_token(self) -> bool:
        """Get JWT token for authentication"""
        import subprocess
        
        result = subprocess.run([
            'curl', '-X', 'POST', 'http://localhost:8081/auth/login',
            '-H', 'Content-Type: application/json',
            '-d', '{"email": "test@example.com", "password": "password123"}'
        ], capture_output=True, text=True)
        
        if result.returncode == 0:
            try:
                login_data = json.loads(result.stdout)
                self.jwt_token = login_data.get('jwt_token')
                return self.jwt_token is not None
            except:
                return False
        return False
    
    def _initialize_mcp(self) -> bool:
        """Initialize MCP connection"""
        init_req = {
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": {},
            "id": 1
        }
        
        response = self._send_request(init_req)
        return response.get('result') is not None
    
    def _send_request(self, request: Dict) -> Dict:
        """Send MCP request and get response"""
        if not self.sock:
            raise Exception("Not connected")
            
        request_str = json.dumps(request)
        self.sock.send(request_str.encode() + b'\n')
        
        # Read response
        response = b''
        while b'\n' not in response:
            chunk = self.sock.recv(1024)
            if not chunk:
                break
            response += chunk
        
        return json.loads(response.decode().strip())
    
    def collect_activities(self, limit: int = 100, provider: str = 'strava') -> List[Dict]:
        """Collect activities efficiently"""
        print(f"🔄 Collecting {limit} activities from {provider}...")
        
        start_time = time.time()
        
        activities_req = {
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "get_activities",
                "arguments": {
                    "provider": provider,
                    "limit": limit
                }
            },
            "id": 2,
            "auth": f"Bearer {self.jwt_token}"
        }
        
        response = self._send_request(activities_req)
        collection_time = time.time() - start_time
        
        if response.get('result'):
            result = response['result']
            if isinstance(result, list):
                activities = result
            elif isinstance(result, dict) and 'activities' in result:
                activities = result['activities']
            else:
                activities = []
            
            rate = len(activities) / collection_time if collection_time > 0 else 0
            print(f"✅ Collected {len(activities)} activities in {collection_time:.2f}s ({rate:.1f} activities/sec)")
            
            return activities
        else:
            error = response.get('error', {})
            print(f"❌ Collection failed: {error.get('message', 'Unknown error')}")
            return []
    
    def analyze_data_quality(self, activities: List[Dict]) -> Dict:
        """Analyze quality of collected data"""
        if not activities:
            return {'quality_score': 0, 'issues': ['No activities collected']}
        
        issues = []
        has_distance = sum(1 for a in activities if a.get('distance_meters', 0) > 0)
        has_duration = sum(1 for a in activities if a.get('moving_time_seconds', 0) > 0)
        has_sport_type = sum(1 for a in activities if a.get('sport_type'))
        
        distance_completeness = has_distance / len(activities)
        duration_completeness = has_duration / len(activities)
        sport_completeness = has_sport_type / len(activities)
        
        if distance_completeness < 0.8:
            issues.append(f"Low distance completeness: {distance_completeness:.1%}")
        if duration_completeness < 0.9:
            issues.append(f"Low duration completeness: {duration_completeness:.1%}")
        if sport_completeness < 0.95:
            issues.append(f"Low sport type completeness: {sport_completeness:.1%}")
        
        quality_score = (distance_completeness + duration_completeness + sport_completeness) / 3 * 100
        
        return {
            'quality_score': quality_score,
            'completeness': {
                'distance': distance_completeness,
                'duration': duration_completeness,
                'sport_type': sport_completeness
            },
            'issues': issues
        }
    
    def save_data(self, activities: List[Dict], filename: str) -> bool:
        """Save collected data with metadata"""
        try:
            metadata = {
                'collection_timestamp': datetime.now().isoformat(),
                'total_activities': len(activities),
                'quality_analysis': self.analyze_data_quality(activities)
            }
            
            data = {
                'metadata': metadata,
                'activities': activities
            }
            
            with open(filename, 'w') as f:
                json.dump(data, f, indent=2)
            
            print(f"💾 Saved {len(activities)} activities to {filename}")
            return True
            
        except Exception as e:
            print(f"❌ Save failed: {e}")
            return False
    
    def close(self):
        """Close connection"""
        if self.sock:
            self.sock.close()
            self.sock = None

def main():
    """Example usage of MCP data collection"""
    print("🚀 MCP Data Collection Example")
    print("=" * 40)
    
    # Initialize collector
    collector = MCPDataCollector()
    
    if not collector.connect():
        print("❌ Failed to connect to MCP server")
        return
    
    # Collect activities
    activities = collector.collect_activities(limit=100)
    
    if activities:
        # Analyze quality
        quality = collector.analyze_data_quality(activities)
        print(f"\n📊 Data Quality: {quality['quality_score']:.1f}/100")
        
        if quality['issues']:
            print("⚠️ Quality Issues:")
            for issue in quality['issues']:
                print(f"   • {issue}")
        
        # Save data
        filename = f"mcp_activities_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
        collector.save_data(activities, filename)
        
        print(f"\n✅ Collection complete!")
        print(f"📁 Data saved to: {filename}")
        print(f"📊 Quality score: {quality['quality_score']:.1f}/100")
    
    collector.close()

if __name__ == "__main__":
    main()