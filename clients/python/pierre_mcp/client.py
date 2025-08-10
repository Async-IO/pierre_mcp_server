# ABOUTME: Main client implementation for Pierre MCP Server connectivity
# ABOUTME: Handles authentication, tool execution, and tenant-aware API calls

"""
Pierre MCP Client Implementation

Provides async client for connecting to Pierre MCP Server with proper
tenant isolation and authentication.
"""

import aiohttp
import asyncio
from typing import Dict, List, Any, Optional
from .exceptions import PierreMCPError, AuthenticationError, TenantError


class PierreMCPClient:
    """
    Async client for Pierre MCP Server
    
    Handles tenant-aware authentication and tool execution
    for fitness data analysis.
    """
    
    def __init__(
        self,
        server_url: str,
        tenant_id: str,
        jwt_token: str,
        timeout: int = 30
    ):
        """
        Initialize Pierre MCP Client
        
        Args:
            server_url: Base URL of Pierre MCP Server (e.g., http://localhost:8081)
            tenant_id: Your tenant organization ID
            jwt_token: JWT token for authentication
            timeout: Request timeout in seconds
        """
        self.server_url = server_url.rstrip('/')
        self.tenant_id = tenant_id
        self.jwt_token = jwt_token
        self.timeout = timeout
        self.session: Optional[aiohttp.ClientSession] = None
        
    async def connect(self):
        """Establish connection to the server"""
        self.session = aiohttp.ClientSession(
            timeout=aiohttp.ClientTimeout(total=self.timeout),
            headers={
                'Authorization': f'Bearer {self.jwt_token}',
                'X-Tenant-ID': self.tenant_id,
                'Content-Type': 'application/json'
            }
        )
        
        # Test connection
        try:
            async with self.session.get(f'{self.server_url}/health') as response:
                if response.status != 200:
                    raise PierreMCPError(f"Server health check failed: {response.status}")
        except aiohttp.ClientError as e:
            raise PierreMCPError(f"Failed to connect to server: {e}")
    
    async def close(self):
        """Close the client connection"""
        if self.session:
            await self.session.close()
            self.session = None
    
    async def list_tools(self) -> List[Dict[str, Any]]:
        """
        List available MCP tools
        
        Returns:
            List of tool definitions with names, descriptions, and parameters
        """
        if not self.session:
            raise PierreMCPError("Client not connected. Call connect() first.")
        
        try:
            async with self.session.post(
                f'{self.server_url}/mcp',
                json={
                    "jsonrpc": "2.0",
                    "method": "tools/list",
                    "id": 1
                }
            ) as response:
                if response.status == 401:
                    raise AuthenticationError("Invalid JWT token")
                elif response.status == 403:
                    raise TenantError("Tenant access denied")
                elif response.status != 200:
                    raise PierreMCPError(f"Failed to list tools: {response.status}")
                
                data = await response.json()
                if "error" in data:
                    raise PierreMCPError(f"Server error: {data['error']}")
                
                return data.get("result", {}).get("tools", [])
                
        except aiohttp.ClientError as e:
            raise PierreMCPError(f"Network error: {e}")
    
    async def call_tool(self, tool_name: str, parameters: Dict[str, Any]) -> Any:
        """
        Execute a specific tool
        
        Args:
            tool_name: Name of the tool to execute
            parameters: Parameters to pass to the tool
            
        Returns:
            Tool execution result
        """
        if not self.session:
            raise PierreMCPError("Client not connected. Call connect() first.")
        
        try:
            async with self.session.post(
                f'{self.server_url}/mcp',
                json={
                    "jsonrpc": "2.0",
                    "method": "tools/call",
                    "params": {
                        "name": tool_name,
                        "arguments": parameters
                    },
                    "id": 1
                }
            ) as response:
                if response.status == 401:
                    raise AuthenticationError("Invalid JWT token")
                elif response.status == 403:
                    raise TenantError("Tenant access denied or OAuth not configured")
                elif response.status != 200:
                    raise PierreMCPError(f"Failed to execute tool: {response.status}")
                
                data = await response.json()
                if "error" in data:
                    raise PierreMCPError(f"Tool execution error: {data['error']}")
                
                return data.get("result")
                
        except aiohttp.ClientError as e:
            raise PierreMCPError(f"Network error: {e}")
    
    async def get_oauth_status(self, provider: str = "strava") -> Dict[str, Any]:
        """
        Check OAuth connection status for a provider
        
        Args:
            provider: OAuth provider name (strava, fitbit)
            
        Returns:
            OAuth status information
        """
        if not self.session:
            raise PierreMCPError("Client not connected. Call connect() first.")
        
        try:
            async with self.session.get(
                f'{self.server_url}/oauth/status/{provider}'
            ) as response:
                if response.status == 401:
                    raise AuthenticationError("Invalid JWT token")
                elif response.status == 404:
                    raise TenantError("Tenant OAuth not configured")
                elif response.status != 200:
                    raise PierreMCPError(f"Failed to get OAuth status: {response.status}")
                
                return await response.json()
                
        except aiohttp.ClientError as e:
            raise PierreMCPError(f"Network error: {e}")
    
    async def get_authorization_url(self, provider: str = "strava") -> str:
        """
        Get OAuth authorization URL for connecting to a provider
        
        Args:
            provider: OAuth provider name (strava, fitbit)
            
        Returns:
            Authorization URL to redirect user to
        """
        if not self.session:
            raise PierreMCPError("Client not connected. Call connect() first.")
        
        try:
            async with self.session.get(
                f'{self.server_url}/oauth/authorize/{provider}',
                params={'tenant_id': self.tenant_id}
            ) as response:
                if response.status == 401:
                    raise AuthenticationError("Invalid JWT token")
                elif response.status == 404:
                    raise TenantError("Tenant OAuth not configured")
                elif response.status != 200:
                    raise PierreMCPError(f"Failed to get authorization URL: {response.status}")
                
                # Server returns redirect, extract URL from location header
                return str(response.url)
                
        except aiohttp.ClientError as e:
            raise PierreMCPError(f"Network error: {e}")
    
    async def __aenter__(self):
        """Async context manager entry"""
        await self.connect()
        return self
    
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit"""
        await self.close()