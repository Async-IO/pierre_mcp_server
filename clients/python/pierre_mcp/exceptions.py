# ABOUTME: Custom exception classes for Pierre MCP Client
# ABOUTME: Provides specific error types for authentication, tenant, and API issues

"""
Pierre MCP Client Exceptions

Custom exception classes for handling different types of errors
when communicating with Pierre MCP Server.
"""


class PierreMCPError(Exception):
    """Base exception for all Pierre MCP Client errors"""
    pass


class AuthenticationError(PierreMCPError):
    """Raised when authentication fails (invalid JWT token, expired token, etc.)"""
    pass


class TenantError(PierreMCPError):
    """Raised when tenant-related operations fail (invalid tenant, access denied, etc.)"""
    pass