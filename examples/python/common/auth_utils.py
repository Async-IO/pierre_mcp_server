#!/usr/bin/env python3
"""
Common Authentication Utilities
Shared authentication helpers for both MCP and A2A protocols
"""

import os
import json
import requests
import subprocess
from typing import Optional, Dict

class AuthManager:
    """Centralized authentication management"""
    
    def __init__(self, config_file: str = None):
        self.config_file = config_file or os.path.expanduser('~/.pierre_auth.json')
        self.config = self._load_config()
    
    def _load_config(self) -> Dict:
        """Load authentication configuration"""
        try:
            with open(self.config_file, 'r') as f:
                return json.load(f)
        except FileNotFoundError:
            return {}
        except json.JSONDecodeError:
            print(f"⚠️ Invalid config file: {self.config_file}")
            return {}
    
    def _save_config(self):
        """Save authentication configuration"""
        try:
            os.makedirs(os.path.dirname(self.config_file), exist_ok=True)
            with open(self.config_file, 'w') as f:
                json.dump(self.config, f, indent=2)
        except Exception as e:
            print(f"⚠️ Failed to save config: {e}")
    
    def get_jwt_token(self, email: str = None, password: str = None, 
                     base_url: str = 'http://localhost:8081') -> Optional[str]:
        """Get JWT token for MCP/API authentication"""
        
        # Try to use cached token first
        import time
        cache_key = f"jwt_token_{base_url}"
        if cache_key in self.config:
            token_data = self.config[cache_key]
            # Simple expiry check (tokens usually last 24h)
            if time.time() - token_data.get('timestamp', 0) < 23 * 3600:  # 23 hours
                return token_data['token']
        
        # Get credentials from environment or parameters
        email = email or os.getenv('PIERRE_EMAIL', 'test@example.com')
        password = password or os.getenv('PIERRE_PASSWORD', 'password123')
        
        try:
            # Use curl for robust authentication
            result = subprocess.run([
                'curl', '-X', 'POST', f'{base_url}/auth/login',
                '-H', 'Content-Type: application/json',
                '-d', json.dumps({'email': email, 'password': password})
            ], capture_output=True, text=True, timeout=10)
            
            if result.returncode == 0:
                login_data = json.loads(result.stdout)
                jwt_token = login_data.get('jwt_token')
                
                if jwt_token:
                    # Cache the token
                    self.config[cache_key] = {
                        'token': jwt_token,
                        'timestamp': time.time()
                    }
                    self._save_config()
                    return jwt_token
            
            print(f"❌ JWT authentication failed: {result.stderr}")
            return None
            
        except Exception as e:
            print(f"❌ JWT authentication error: {e}")
            return None
    
    def get_api_key(self, key_name: str = None) -> Optional[str]:
        """Get API key from config or environment"""
        
        # Try environment variable first
        api_key = os.getenv('PIERRE_API_KEY')
        if api_key:
            return api_key
        
        # Try config file
        if key_name and key_name in self.config.get('api_keys', {}):
            return self.config['api_keys'][key_name]
        
        # Try default key
        default_key = self.config.get('api_keys', {}).get('default')
        if default_key:
            return default_key
        
        return None
    
    def save_api_key(self, api_key: str, key_name: str = 'default'):
        """Save API key to config"""
        if 'api_keys' not in self.config:
            self.config['api_keys'] = {}
        
        self.config['api_keys'][key_name] = api_key
        self._save_config()
        print(f"✅ API key saved as '{key_name}'")
    
    def create_api_key(self, name: str, description: str = "", tier: str = "trial",
                      base_url: str = 'http://localhost:8081') -> Optional[str]:
        """Create new API key via API"""
        
        jwt_token = self.get_jwt_token(base_url=base_url)
        if not jwt_token:
            print("❌ Failed to get JWT token for API key creation")
            return None
        
        try:
            response = requests.post(f'{base_url}/api/keys', 
                headers={
                    'Authorization': f'Bearer {jwt_token}',
                    'Content-Type': 'application/json'
                },
                json={
                    'name': name,
                    'description': description,
                    'tier': tier
                },
                timeout=10
            )
            
            if response.status_code == 200:
                data = response.json()
                api_key = data.get('api_key')
                
                if api_key:
                    # Save the key
                    self.save_api_key(api_key, name.lower().replace(' ', '_'))
                    
                    key_info = data.get('key_info', {})
                    print(f"✅ Created API key: {key_info.get('key_prefix', 'Unknown')}")
                    print(f"🔒 Tier: {key_info.get('tier', 'Unknown')}")
                    
                    return api_key
            
            print(f"❌ API key creation failed: {response.status_code}")
            return None
            
        except Exception as e:
            print(f"❌ API key creation error: {e}")
            return None
    
    def setup_demo_auth(self, base_url: str = 'http://localhost:8081') -> Dict[str, Optional[str]]:
        """Setup authentication for demo purposes"""
        print("🔐 Setting up authentication...")
        
        # Get JWT token
        jwt_token = self.get_jwt_token(base_url=base_url)
        
        # Try to get or create API key
        api_key = self.get_api_key()
        if not api_key:
            api_key = self.create_api_key(
                name='Demo API Key',
                description='Auto-created for demo purposes',
                tier='trial'
            )
        
        return {
            'jwt_token': jwt_token,
            'api_key': api_key
        }

class EnvironmentConfig:
    """Environment-based configuration helper"""
    
    @staticmethod
    def get_server_config() -> Dict[str, str]:
        """Get server configuration from environment"""
        return {
            'mcp_host': os.getenv('MCP_SERVER_HOST', 'localhost'),
            'mcp_port': int(os.getenv('MCP_SERVER_PORT', '8080')),
            'api_base': os.getenv('PIERRE_API_BASE', 'http://localhost:8081'),
            'provider': os.getenv('FITNESS_PROVIDER', 'strava')
        }
    
    @staticmethod
    def setup_environment():
        """Setup environment variables for examples"""
        config = {
            'MCP_SERVER_HOST': 'localhost',
            'MCP_SERVER_PORT': '8080', 
            'PIERRE_API_BASE': 'http://localhost:8081',
            'FITNESS_PROVIDER': 'strava'
        }
        
        for key, default_value in config.items():
            if key not in os.environ:
                os.environ[key] = default_value

def main():
    """Test authentication utilities"""
    print("🔐 Authentication Utilities Test")
    print("=" * 40)
    
    auth = AuthManager()
    
    # Setup demo authentication
    auth_data = auth.setup_demo_auth()
    
    if auth_data['jwt_token']:
        print("✅ JWT token obtained")
    else:
        print("❌ JWT token failed")
    
    if auth_data['api_key']:
        print("✅ API key available")
    else:
        print("❌ API key not available")
    
    # Show environment setup
    EnvironmentConfig.setup_environment()
    config = EnvironmentConfig.get_server_config()
    print(f"\n📋 Server Configuration:")
    for key, value in config.items():
        print(f"   • {key}: {value}")

if __name__ == "__main__":
    main()