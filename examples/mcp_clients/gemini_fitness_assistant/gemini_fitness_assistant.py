#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence

"""
Gemini Fitness Assistant - MCP Client Example

This example demonstrates how to use Google's free Gemini API with Pierre MCP Server
to create an AI fitness assistant that can query and analyze fitness data.

Free Gemini API: https://ai.google.dev/gemini-api/docs/api-key
- 1,500 requests per day (free tier)
- Native function calling support
- No credit card required
"""

import os
import sys
import json
import asyncio
import argparse
from typing import Dict, List, Any, Optional
from dataclasses import dataclass
import subprocess

try:
    import google.generativeai as genai
    from google.generativeai import types
except ImportError:
    print("Error: google-generativeai package not installed")
    print("Install with: pip install google-generativeai")
    sys.exit(1)

try:
    import requests
except ImportError:
    print("Error: requests package not installed")
    print("Install with: pip install requests")
    sys.exit(1)


@dataclass
class MCPTool:
    """Represents an MCP tool with its schema"""
    name: str
    description: str
    parameters: Dict[str, Any]


class PierreMCPClient:
    """Client for interacting with Pierre MCP Server via HTTP"""

    def __init__(self, server_url: str, jwt_token: Optional[str] = None):
        self.server_url = server_url.rstrip('/')
        self.mcp_endpoint = f"{self.server_url}/mcp"
        self.jwt_token = jwt_token
        self.tools: List[MCPTool] = []

    def set_token(self, token: str):
        """Set JWT token for authentication"""
        self.jwt_token = token

    def _make_mcp_request(self, method: str, params: Optional[Dict] = None) -> Dict[str, Any]:
        """Make an MCP JSON-RPC request"""
        headers = {
            "Content-Type": "application/json"
        }

        if self.jwt_token:
            headers["Authorization"] = f"Bearer {self.jwt_token}"

        payload = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params or {},
            "id": 1
        }

        try:
            response = requests.post(
                self.mcp_endpoint,
                json=payload,
                headers=headers,
                timeout=30
            )
            response.raise_for_status()
            result = response.json()

            if "error" in result:
                raise Exception(f"MCP Error: {result['error']}")

            return result.get("result", {})
        except requests.exceptions.RequestException as e:
            raise Exception(f"HTTP Request failed: {e}")

    def fetch_tools(self) -> List[MCPTool]:
        """Fetch available MCP tools from the server"""
        result = self._make_mcp_request("tools/list")

        tools_data = result.get("tools", [])
        self.tools = [
            MCPTool(
                name=tool["name"],
                description=tool.get("description", ""),
                parameters=tool.get("inputSchema", {})
            )
            for tool in tools_data
        ]

        return self.tools

    def call_tool(self, tool_name: str, arguments: Dict[str, Any]) -> Any:
        """Call an MCP tool with the given arguments"""
        params = {
            "name": tool_name,
            "arguments": arguments
        }

        result = self._make_mcp_request("tools/call", params)
        return result.get("content", [])

    def get_tool_by_name(self, name: str) -> Optional[MCPTool]:
        """Get tool definition by name"""
        for tool in self.tools:
            if tool.name == name:
                return tool
        return None


class GeminiFitnessAssistant:
    """AI Fitness Assistant using Gemini and Pierre MCP"""

    def __init__(self, gemini_api_key: str, pierre_client: PierreMCPClient):
        genai.configure(api_key=gemini_api_key)

        # Use Gemini 2.0 Flash for fast, free inference with function calling
        self.model = genai.GenerativeModel(
            model_name='gemini-2.0-flash-exp',
            system_instruction="""You are a fitness assistant with access to the user's fitness data
            through the Pierre Fitness Platform. You can analyze activities, provide training insights,
            suggest goals, and help with nutrition planning.

            When the user asks about their fitness data, use the available tools to fetch and analyze
            the information. Be specific, data-driven, and provide actionable insights.

            Available data sources: Strava, Garmin, Fitbit (depending on user's connections).
            """
        )

        self.pierre = pierre_client
        self.chat = None
        self.gemini_tools = []

    def _convert_mcp_tools_to_gemini(self) -> List[types.FunctionDeclaration]:
        """Convert MCP tool schemas to Gemini function declarations"""
        gemini_tools = []

        for mcp_tool in self.pierre.tools:
            # Convert MCP tool schema to Gemini format
            parameters = mcp_tool.parameters.get("properties", {})
            required = mcp_tool.parameters.get("required", [])

            # Build Gemini-compatible parameter definitions
            gemini_params = {}
            for param_name, param_schema in parameters.items():
                gemini_params[param_name] = {
                    "type_": self._map_json_type_to_gemini(param_schema.get("type", "string")),
                    "description": param_schema.get("description", "")
                }

            function = types.FunctionDeclaration(
                name=mcp_tool.name,
                description=mcp_tool.description,
                parameters={
                    "type": "object",
                    "properties": gemini_params,
                    "required": required
                }
            )

            gemini_tools.append(function)

        return gemini_tools

    def _map_json_type_to_gemini(self, json_type: str) -> str:
        """Map JSON Schema types to Gemini types"""
        type_mapping = {
            "string": "string",
            "number": "number",
            "integer": "integer",
            "boolean": "boolean",
            "array": "array",
            "object": "object"
        }
        return type_mapping.get(json_type.lower(), "string")

    def initialize(self):
        """Initialize the assistant by fetching tools and setting up Gemini"""
        print("🔧 Fetching available MCP tools from Pierre...")
        self.pierre.fetch_tools()
        print(f"✅ Loaded {len(self.pierre.tools)} MCP tools")

        # Convert tools for Gemini
        self.gemini_tools = self._convert_mcp_tools_to_gemini()

        # Create chat session with tools
        self.chat = self.model.start_chat(enable_automatic_function_calling=True)

        print("🤖 Gemini Fitness Assistant ready!")
        print("   Model: gemini-2.0-flash-exp (free tier)")
        print("   Rate limit: 1,500 requests/day\n")

    async def process_query(self, user_query: str) -> str:
        """Process a user query using Gemini and MCP tools"""
        print(f"\n💬 You: {user_query}")
        print("🤔 Thinking...")

        try:
            # Create tool config for this request
            tool_config = types.ToolConfig(
                function_calling_config=types.FunctionCallingConfig(
                    mode=types.FunctionCallingConfig.Mode.AUTO
                )
            )

            # Send message to Gemini with tools
            response = self.chat.send_message(
                user_query,
                tools=self.gemini_tools,
                tool_config=tool_config
            )

            # Process function calls if any
            while response.candidates[0].content.parts:
                parts = response.candidates[0].content.parts

                # Check if there are function calls
                function_calls = [part.function_call for part in parts if hasattr(part, 'function_call') and part.function_call]

                if not function_calls:
                    # No more function calls, we have the final response
                    break

                # Execute function calls via MCP
                function_responses = []
                for fc in function_calls:
                    print(f"🔧 Calling tool: {fc.name}")
                    print(f"   Arguments: {dict(fc.args)}")

                    try:
                        # Call MCP tool
                        result = self.pierre.call_tool(fc.name, dict(fc.args))

                        function_responses.append(
                            types.FunctionResponse(
                                name=fc.name,
                                response={"result": result}
                            )
                        )
                        print(f"   ✅ Tool executed successfully")
                    except Exception as e:
                        print(f"   ❌ Tool execution failed: {e}")
                        function_responses.append(
                            types.FunctionResponse(
                                name=fc.name,
                                response={"error": str(e)}
                            )
                        )

                # Send function results back to Gemini
                response = self.chat.send_message(
                    types.Content(parts=[types.Part(function_response=fr) for fr in function_responses])
                )

            # Extract final text response
            final_response = response.text
            print(f"\n🤖 Assistant: {final_response}\n")

            return final_response

        except Exception as e:
            error_msg = f"Error processing query: {e}"
            print(f"\n❌ {error_msg}\n")
            return error_msg


def get_jwt_token(server_url: str, email: str, password: str) -> str:
    """Get JWT token via OAuth flow"""
    # This is a simplified version - in production, use proper OAuth flow
    login_url = f"{server_url}/api/auth/login"

    try:
        response = requests.post(
            login_url,
            json={"email": email, "password": password},
            timeout=10
        )
        response.raise_for_status()
        result = response.json()
        return result.get("token") or result.get("access_token")
    except Exception as e:
        print(f"❌ Login failed: {e}")
        print("   Make sure you have created a user account on Pierre server")
        sys.exit(1)


def run_interactive_mode(assistant: GeminiFitnessAssistant):
    """Run the assistant in interactive mode"""
    print("\n" + "="*60)
    print("  Gemini Fitness Assistant - Interactive Mode")
    print("="*60)
    print("Ask me anything about your fitness data!")
    print("Examples:")
    print("  - What were my last 5 activities?")
    print("  - Analyze my training load for the past month")
    print("  - Suggest goals based on my activity history")
    print("  - Calculate my daily nutrition needs")
    print("\nType 'quit' or 'exit' to stop.\n")

    while True:
        try:
            user_input = input("You: ").strip()

            if not user_input:
                continue

            if user_input.lower() in ['quit', 'exit', 'bye']:
                print("\n👋 Goodbye! Keep training hard!\n")
                break

            # Process query
            asyncio.run(assistant.process_query(user_input))

        except KeyboardInterrupt:
            print("\n\n👋 Goodbye! Keep training hard!\n")
            break
        except Exception as e:
            print(f"\n❌ Error: {e}\n")


def run_demo_mode(assistant: GeminiFitnessAssistant):
    """Run predefined demo queries"""
    print("\n" + "="*60)
    print("  Gemini Fitness Assistant - Demo Mode")
    print("="*60)
    print("Running predefined fitness queries...\n")

    demo_queries = [
        "What were my last 3 activities?",
        "Get my athlete profile information",
        "What fitness data connections do I have?",
    ]

    for query in demo_queries:
        asyncio.run(assistant.process_query(query))
        print("\n" + "-"*60 + "\n")


def main():
    parser = argparse.ArgumentParser(
        description="Gemini Fitness Assistant - Free LLM with Pierre MCP Server"
    )
    parser.add_argument(
        "--server",
        default=os.getenv("PIERRE_SERVER_URL", "http://localhost:8081"),
        help="Pierre server URL (default: http://localhost:8081)"
    )
    parser.add_argument(
        "--gemini-key",
        default=os.getenv("GEMINI_API_KEY"),
        help="Google Gemini API key (or set GEMINI_API_KEY env var)"
    )
    parser.add_argument(
        "--email",
        default=os.getenv("PIERRE_EMAIL"),
        help="Pierre user email for authentication"
    )
    parser.add_argument(
        "--password",
        default=os.getenv("PIERRE_PASSWORD"),
        help="Pierre user password for authentication"
    )
    parser.add_argument(
        "--demo",
        action="store_true",
        help="Run in demo mode with predefined queries"
    )

    args = parser.parse_args()

    # Validate Gemini API key
    if not args.gemini_key:
        print("❌ Error: Gemini API key is required")
        print("\nGet a free API key at: https://ai.google.dev/gemini-api/docs/api-key")
        print("Then set it via:")
        print("  export GEMINI_API_KEY='your-api-key'")
        print("  or use --gemini-key flag")
        sys.exit(1)

    # Validate Pierre credentials
    if not args.email or not args.password:
        print("❌ Error: Pierre credentials are required")
        print("\nSet credentials via:")
        print("  export PIERRE_EMAIL='your-email'")
        print("  export PIERRE_PASSWORD='your-password'")
        print("  or use --email and --password flags")
        sys.exit(1)

    print("🚀 Initializing Gemini Fitness Assistant...")
    print(f"   Pierre Server: {args.server}")
    print(f"   User: {args.email}")

    # Authenticate with Pierre
    print("\n🔐 Authenticating with Pierre server...")
    jwt_token = get_jwt_token(args.server, args.email, args.password)
    print("✅ Authentication successful")

    # Initialize MCP client
    pierre_client = PierreMCPClient(args.server, jwt_token)

    # Initialize Gemini assistant
    assistant = GeminiFitnessAssistant(args.gemini_key, pierre_client)
    assistant.initialize()

    # Run in appropriate mode
    if args.demo:
        run_demo_mode(assistant)
    else:
        run_interactive_mode(assistant)


if __name__ == "__main__":
    main()
