#!/usr/bin/env python3
"""
MCP stdio Transport Example

This example demonstrates using the Pierre MCP Server with the stdio transport,
which is the primary MCP transport for local AI assistant connections.

Prerequisites:
1. Server running: cargo run --bin pierre-mcp-server
2. Valid JWT token from authentication
3. Strava OAuth connected (optional)

Usage:
    python3 mcp_stdio_example.py | cargo run --bin pierre-mcp-server
    
    Or for interactive testing:
    python3 mcp_stdio_example.py
"""

import json
import sys
import subprocess
import time
from typing import Optional, Dict, Any, List

class MCPStdioClient:
    """
    MCP stdio Transport Client
    
    Demonstrates proper MCP protocol usage with stdio transport
    """
    
    def __init__(self, jwt_token: str):
        self.jwt_token = jwt_token
        self.request_id = 0
        
    def _next_id(self) -> int:
        """Get next request ID"""
        self.request_id += 1
        return self.request_id
    
    def initialize(self) -> Dict[str, Any]:
        """Initialize MCP connection"""
        request = {
            "jsonrpc": "2.0",
            "id": self._next_id(),
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "roots": {"listChanged": True},
                    "sampling": {}
                },
                "clientInfo": {
                    "name": "stdio-example-client",
                    "version": "1.0.0"
                }
            }
        }
        return request
    
    def tools_list(self) -> Dict[str, Any]:
        """List available MCP tools"""
        request = {
            "jsonrpc": "2.0",
            "id": self._next_id(),
            "method": "tools/list",
            "auth": f"Bearer {self.jwt_token}"
        }
        return request
    
    def call_tool(self, tool_name: str, arguments: Dict[str, Any]) -> Dict[str, Any]:
        """Call an MCP tool"""
        request = {
            "jsonrpc": "2.0",
            "id": self._next_id(),
            "method": "tools/call",
            "auth": f"Bearer {self.jwt_token}",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
        }
        return request
    
    def get_connection_status(self) -> Dict[str, Any]:
        """Get OAuth connection status"""
        return self.call_tool("get_connection_status", {})
    
    def get_athlete(self, provider: str = "strava") -> Dict[str, Any]:
        """Get athlete profile"""
        return self.call_tool("get_athlete", {"provider": provider})
    
    def get_activities(self, provider: str = "strava", limit: int = 5) -> Dict[str, Any]:
        """Get recent activities"""
        return self.call_tool("get_activities", {
            "provider": provider,
            "limit": limit
        })
    
    def get_activity_intelligence(self, activity_id: str, provider: str = "strava") -> Dict[str, Any]:
        """Get AI activity analysis"""
        return self.call_tool("get_activity_intelligence", {
            "provider": provider,
            "activity_id": activity_id,
            "include_weather": True,
            "include_location": True
        })
    
    def get_stats(self, provider: str = "strava") -> Dict[str, Any]:
        """Get fitness statistics"""
        return self.call_tool("get_stats", {"provider": provider})
    
    def generate_recommendations(self, provider: str = "strava") -> Dict[str, Any]:
        """Get training recommendations"""
        return self.call_tool("generate_recommendations", {
            "provider": provider,
            "recommendation_type": "training"
        })

def generate_stdio_requests(jwt_token: str) -> List[Dict[str, Any]]:
    """Generate a sequence of MCP requests for stdio transport"""
    client = MCPStdioClient(jwt_token)
    
    requests = [
        # Initialize MCP connection
        client.initialize(),
        
        # List available tools
        client.tools_list(),
        
        # Check connection status
        client.get_connection_status(),
        
        # Get athlete profile
        client.get_athlete("strava"),
        
        # Get recent activities
        client.get_activities("strava", 3),
        
        # Get fitness stats
        client.get_stats("strava"),
        
        # Get training recommendations
        client.generate_recommendations("strava"),
    ]
    
    return requests

def run_stdio_example_interactive():
    """Run stdio example interactively with server process"""
    print("🔧 MCP stdio Transport Interactive Example", file=sys.stderr)
    print("=" * 50, file=sys.stderr)
    
    # Example JWT token - replace with your actual token
    jwt_token = input("Enter your JWT token: ").strip()
    
    if not jwt_token:
        print("❌ JWT token required", file=sys.stderr)
        sys.exit(1)
    
    print("🚀 Starting MCP server process...", file=sys.stderr)
    
    # Start the server process
    server_cmd = ["cargo", "run", "--bin", "pierre-mcp-server"]
    server_process = subprocess.Popen(
        server_cmd,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        cwd="/Users/jeanfrancoisarcand/workspace/strava_ai/pierre_mcp_server"
    )
    
    try:
        # Generate requests
        requests = generate_stdio_requests(jwt_token)
        
        print(f"📤 Sending {len(requests)} MCP requests...", file=sys.stderr)
        
        # Send requests and read responses
        for i, request in enumerate(requests, 1):
            print(f"\\n📤 Request {i}: {request['method']}", file=sys.stderr)
            
            # Send request
            request_json = json.dumps(request)
            server_process.stdin.write(request_json + "\\n")
            server_process.stdin.flush()
            
            # Read response
            try:
                response_line = server_process.stdout.readline()
                if response_line:
                    response = json.loads(response_line.strip())
                    print(f"✅ Response {i}: {response.get('result', response.get('error'))}", file=sys.stderr)
                    
                    # Pretty print interesting responses
                    if response.get("result"):
                        method = request["method"]
                        if method == "initialize":
                            print(f"   Server: {response['result']['serverInfo']['name']}", file=sys.stderr)
                        elif method == "tools/list":
                            tools = response['result'].get('tools', [])
                            print(f"   Available tools: {len(tools)}", file=sys.stderr)
                        elif method == "tools/call":
                            tool_name = request["params"]["name"]
                            if tool_name == "get_athlete":
                                athlete = response['result']
                                print(f"   Athlete: {athlete.get('firstname', '')} {athlete.get('lastname', '')}", file=sys.stderr)
                            elif tool_name == "get_activities":
                                activities = response['result']
                                print(f"   Activities found: {len(activities)}", file=sys.stderr)
                            elif tool_name == "get_stats":
                                stats = response['result']
                                print(f"   Total activities: {stats.get('total_activities', 0)}", file=sys.stderr)
                else:
                    print(f"❌ No response for request {i}", file=sys.stderr)
                    
            except Exception as e:
                print(f"❌ Error reading response {i}: {e}", file=sys.stderr)
        
        print("\\n🎉 stdio example completed!", file=sys.stderr)
        
    except Exception as e:
        print(f"❌ Example failed: {e}", file=sys.stderr)
        
    finally:
        # Clean up server process
        server_process.terminate()
        server_process.wait()

def run_stdio_example_pipe():
    """Run stdio example for piping to server"""
    print("🔧 MCP stdio Transport Pipe Example", file=sys.stderr)
    print("Pipe this output to the server:", file=sys.stderr)
    print("python3 mcp_stdio_example.py | cargo run --bin pierre-mcp-server", file=sys.stderr)
    print("", file=sys.stderr)
    
    # Example JWT token - replace with your actual token
    jwt_token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMGNmMDM2Ny1hZjZmLTRiYzMtYjVjOC1lN2NiMTMyOGIyM2EiLCJlbWFpbCI6InRlc3RAZXhhbXBsZS5jb20iLCJpYXQiOjE3NTIwODI4NDUwMDAsImV4cCI6MTc1MjE2OTI0NSwicHJvdmlkZXJzIjpbXX0.AMbMtfj60OPyP27zzs_Uoysl5jrxNTzLy5cHIjWISnY"
    
    requests = generate_stdio_requests(jwt_token)
    
    # Output requests to stdout for piping
    for request in requests:
        print(json.dumps(request))
        sys.stdout.flush()
        time.sleep(0.1)  # Small delay between requests

def main():
    """Main function - choose between interactive or pipe mode"""
    if len(sys.argv) > 1 and sys.argv[1] == "--interactive":
        run_stdio_example_interactive()
    else:
        run_stdio_example_pipe()

if __name__ == "__main__":
    main()