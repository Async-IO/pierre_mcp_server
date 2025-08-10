# ABOUTME: Python client library for Pierre MCP Server
# ABOUTME: Provides async client for connecting AI applications to fitness data

"""
Pierre MCP Client

Python client for connecting to Pierre MCP Server for fitness data analysis.
Supports both MCP protocol and direct HTTP API access.
"""

from .client import PierreMCPClient
from .exceptions import PierreMCPError, AuthenticationError, TenantError

__version__ = "0.1.0"
__all__ = ["PierreMCPClient", "PierreMCPError", "AuthenticationError", "TenantError"]