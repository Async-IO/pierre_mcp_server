# A2A (Agent-to-Agent) Protocol - Integration Guide

## Overview

The A2A (Agent-to-Agent) Protocol is an open standard for AI agent communication that enables autonomous agents to discover, authenticate with, and collaborate through Pierre's fitness intelligence platform. This guide provides comprehensive documentation for integrating with Pierre's A2A implementation.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Protocol Specification](#protocol-specification)
3. [Agent Registration](#agent-registration)
4. [Authentication](#authentication)
5. [Available Tools](#available-tools)
6. [Message Formats](#message-formats)
7. [Task Management](#task-management)
8. [Error Handling](#error-handling)
9. [SDK Examples](#sdk-examples)
10. [Best Practices](#best-practices)

## Quick Start

### 1. Register Your Agent

```bash
# Register a new A2A client
curl -X POST http://localhost:8081/a2a/clients \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "FitnessCoach",
    "description": "AI fitness coaching agent",
    "capabilities": ["fitness-data-analysis", "goal-management"],
    "contact_email": "dev@example.com"
  }'
```

### 2. Initialize Connection

```bash
# Initialize A2A connection
curl -X POST http://localhost:8081/a2a \
  -H "Authorization: Bearer a2a_YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "a2a/initialize",
    "id": 1
  }'
```

### 3. Execute Tools

```bash
# Get user activities
curl -X POST http://localhost:8081/a2a \
  -H "Authorization: Bearer a2a_YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "a2a/tools/call",
    "params": {
      "tool_name": "get_activities",
      "parameters": {
        "provider": "strava",
        "limit": 10
      }
    },
    "id": 2
  }'
```

## Protocol Specification

### Base URL

```
Production: https://api.pierre.ai
Development: http://localhost:8081
```

### Agent Card Discovery

Pierre's Agent Card is available at:

```
GET /a2a/agent-card
```

Example response:
```json
{
  "agent": {
    "name": "Pierre Fitness Intelligence Agent",
    "version": "1.0.0",
    "description": "AI-powered fitness data analysis and coaching platform",
    "homepage": "https://pierre.ai",
    "repository": "https://github.com/jfarcand/pierre_mcp_server"
  },
  "authentication": {
    "type": "api_key",
    "description": "API key authentication via Authorization header",
    "location": "header",
    "parameter": "Authorization",
    "scheme": "Bearer"
  },
  "capabilities": [
    "fitness-data-analysis",
    "activity-intelligence", 
    "goal-management",
    "performance-prediction",
    "training-analytics",
    "provider-integration"
  ],
  "tools": [
    {
      "name": "get_activities",
      "description": "Retrieve user fitness activities from connected providers",
      "parameters": {
        "type": "object",
        "properties": {
          "provider": {
            "type": "string",
            "enum": ["strava", "fitbit"],
            "description": "Fitness data provider"
          },
          "limit": {
            "type": "number",
            "description": "Number of activities to retrieve (max 100)",
            "default": 10
          },
          "before": {
            "type": "string",
            "description": "ISO 8601 date to retrieve activities before"
          }
        },
        "required": ["provider"]
      }
    },
    {
      "name": "analyze_activity",
      "description": "AI-powered analysis of fitness activity with environmental context",
      "parameters": {
        "type": "object",
        "properties": {
          "activity_id": {
            "type": "string",
            "description": "Unique activity identifier"
          },
          "provider": {
            "type": "string", 
            "enum": ["strava", "fitbit"],
            "description": "Fitness data provider"
          },
          "include_weather": {
            "type": "boolean",
            "description": "Include weather analysis",
            "default": true
          },
          "include_location": {
            "type": "boolean",
            "description": "Include location intelligence",
            "default": true
          }
        },
        "required": ["activity_id", "provider"]
      }
    }
  ],
  "rate_limits": {
    "trial": {
      "requests_per_month": 1000,
      "burst_limit": 10
    },
    "professional": {
      "requests_per_month": 100000,
      "burst_limit": 100
    },
    "enterprise": {
      "requests_per_month": "unlimited",
      "burst_limit": 1000
    }
  },
  "protocol": {
    "version": "0.1.0",
    "transport": "https",
    "format": "json-rpc-2.0"
  }
}
```

## Agent Registration

### Registration Endpoint

```
POST /a2a/clients
```

### Required Fields

```json
{
  "name": "string",              // Unique agent name
  "description": "string",       // Agent description
  "capabilities": ["string"],    // Agent capabilities
  "contact_email": "string"      // Contact email
}
```

### Optional Fields

```json
{
  "redirect_uris": ["string"],   // OAuth redirect URIs
  "documentation_url": "string", // Agent documentation URL
  "agent_version": "string"      // Agent version
}
```

### Valid Capabilities

- `fitness-data-analysis`: Access to fitness data and analytics
- `activity-intelligence`: AI-powered activity analysis
- `goal-management`: Goal setting and tracking
- `performance-prediction`: Performance forecasting
- `training-analytics`: Training plan analysis
- `provider-integration`: Multi-provider data access

### Registration Response

```json
{
  "client_id": "a2a_client_1234567890abcdef",
  "client_secret": "a2a_secret_abcdef1234567890", 
  "api_key": "a2a_9876543210fedcba",
  "registration_date": "2024-01-15T10:30:00Z",
  "status": "active"
}
```

## Authentication

### API Key Authentication

All A2A requests must include the API key in the Authorization header:

```
Authorization: Bearer a2a_YOUR_API_KEY
```

### Session Management

Create persistent sessions for multi-request workflows:

```json
{
  "jsonrpc": "2.0",
  "method": "a2a/session/create",
  "params": {
    "client_id": "a2a_client_123",
    "user_id": "user_456",
    "scopes": ["fitness:read", "analytics:read"]
  },
  "id": 1
}
```

Session response:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "session_id": "session_789",
    "expires_at": "2024-01-16T10:30:00Z"
  },
  "id": 1
}
```

## Available Tools

### get_activities

Retrieve user fitness activities from connected providers.

**Parameters:**
- `provider` (required): "strava" or "fitbit"
- `limit` (optional): Number of activities (1-100, default: 10)
- `before` (optional): ISO 8601 date string
- `after` (optional): ISO 8601 date string

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "a2a/tools/call",
  "params": {
    "tool_name": "get_activities",
    "parameters": {
      "provider": "strava",
      "limit": 5,
      "before": "2024-01-15T00:00:00Z"
    }
  },
  "id": 1
}
```

### analyze_activity

AI-powered analysis of fitness activities with environmental context.

**Parameters:**
- `activity_id` (required): Activity identifier
- `provider` (required): "strava" or "fitbit"
- `include_weather` (optional): Include weather analysis (default: true)
- `include_location` (optional): Include location intelligence (default: true)

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "a2a/tools/call",
  "params": {
    "tool_name": "analyze_activity",
    "parameters": {
      "activity_id": "12345",
      "provider": "strava",
      "include_weather": true,
      "include_location": true
    }
  },
  "id": 2
}
```

### get_athlete

Retrieve athlete profile information.

**Parameters:**
- `provider` (required): "strava" or "fitbit"

### get_connection_status

Check provider connection status for a user.

**Parameters:**
- `provider` (optional): Specific provider to check

### set_goal

Set a fitness goal for a user.

**Parameters:**
- `goal_type` (required): "distance", "time", "frequency", etc.
- `target_value` (required): Numeric target value
- `timeframe` (required): "weekly", "monthly", "yearly"
- `activity_type` (optional): Specific activity type

## Message Formats

### Request Format

All A2A requests use JSON-RPC 2.0:

```json
{
  "jsonrpc": "2.0",
  "method": "string",
  "params": {},
  "id": 1
}
```

### Response Format

Successful responses:
```json
{
  "jsonrpc": "2.0",
  "result": {},
  "id": 1
}
```

Error responses:
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32000,
    "message": "Error description",
    "data": {}
  },
  "id": 1
}
```

### Message Parts

A2A supports rich message types:

```json
{
  "id": "msg_123",
  "parts": [
    {
      "type": "text",
      "content": "Activity analysis complete"
    },
    {
      "type": "data",
      "content": {
        "efficiency_score": 85.2,
        "relative_effort": 7.8
      }
    },
    {
      "type": "file",
      "name": "route_map.png",
      "mime_type": "image/png",
      "content": "base64_encoded_data"
    }
  ],
  "metadata": {
    "timestamp": "2024-01-15T10:30:00Z",
    "source": "pierre_intelligence"
  }
}
```

## Task Management

### Creating Tasks

For long-running operations, create asynchronous tasks:

```json
{
  "jsonrpc": "2.0",
  "method": "a2a/tasks/create",
  "params": {
    "task_type": "bulk_analysis",
    "parameters": {
      "activity_ids": ["123", "456", "789"],
      "analysis_type": "comprehensive"
    }
  },
  "id": 1
}
```

### Task Status

Tasks progress through these states:
- `pending`: Task created, waiting to execute
- `running`: Task currently executing  
- `completed`: Task finished successfully
- `failed`: Task encountered an error
- `cancelled`: Task was cancelled

### Polling for Results

```json
{
  "jsonrpc": "2.0",
  "method": "a2a/tasks/get",
  "params": {
    "task_id": "task_abc123"
  },
  "id": 2
}
```

## Error Handling

### Standard Error Codes

| Code | Message | Description |
|------|---------|-------------|
| -32700 | Parse error | Invalid JSON |
| -32600 | Invalid Request | Invalid JSON-RPC 2.0 |
| -32601 | Method not found | Unknown method |
| -32602 | Invalid params | Invalid parameters |
| -32603 | Internal error | Server error |
| -32000 | Tool error | Tool execution failed |
| -32001 | Auth error | Authentication failed |
| -32002 | Rate limit | Rate limit exceeded |
| -32003 | Provider error | External provider error |

### Error Response Example

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32001,
    "message": "Authentication failed",
    "data": {
      "reason": "invalid_api_key",
      "details": "API key not found or expired"
    }
  },
  "id": 1
}
```

## SDK Examples

### Python SDK Example

```python
import json
import requests

class PierreA2AClient:
    def __init__(self, api_key, base_url="http://localhost:8081"):
        self.api_key = api_key
        self.base_url = base_url
        self.session = requests.Session()
        self.session.headers.update({
            'Authorization': f'Bearer {api_key}',
            'Content-Type': 'application/json'
        })
    
    def call_tool(self, tool_name, parameters):
        payload = {
            "jsonrpc": "2.0",
            "method": "a2a/tools/call",
            "params": {
                "tool_name": tool_name,
                "parameters": parameters
            },
            "id": 1
        }
        
        response = self.session.post(f"{self.base_url}/a2a", json=payload)
        return response.json()
    
    def get_activities(self, provider="strava", limit=10):
        return self.call_tool("get_activities", {
            "provider": provider,
            "limit": limit
        })
    
    def analyze_activity(self, activity_id, provider="strava"):
        return self.call_tool("analyze_activity", {
            "activity_id": activity_id,
            "provider": provider
        })

# Usage
client = PierreA2AClient("a2a_your_api_key")
activities = client.get_activities(limit=5)
analysis = client.analyze_activity("12345")
```

### JavaScript SDK Example

```javascript
class PierreA2AClient {
    constructor(apiKey, baseUrl = 'http://localhost:8081') {
        this.apiKey = apiKey;
        this.baseUrl = baseUrl;
    }

    async callTool(toolName, parameters) {
        const response = await fetch(`${this.baseUrl}/a2a`, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${this.apiKey}`,
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                jsonrpc: '2.0',
                method: 'a2a/tools/call',
                params: {
                    tool_name: toolName,
                    parameters: parameters
                },
                id: 1
            })
        });

        return await response.json();
    }

    async getActivities(provider = 'strava', limit = 10) {
        return await this.callTool('get_activities', {
            provider: provider,
            limit: limit
        });
    }

    async analyzeActivity(activityId, provider = 'strava') {
        return await this.callTool('analyze_activity', {
            activity_id: activityId,
            provider: provider
        });
    }
}

// Usage
const client = new PierreA2AClient('a2a_your_api_key');
const activities = await client.getActivities(5);
const analysis = await client.analyzeActivity('12345');
```

## Best Practices

### 1. Authentication Management
- Store API keys securely
- Implement token refresh logic
- Use HTTPS in production
- Rotate keys regularly

### 2. Error Handling
- Implement exponential backoff for retries
- Handle rate limiting gracefully
- Log errors for debugging
- Provide meaningful error messages to users

### 3. Performance Optimization
- Use persistent sessions for multiple requests
- Batch requests when possible
- Implement caching for repeated data
- Monitor API usage and costs

### 4. Rate Limiting
- Respect rate limits
- Monitor usage against quotas
- Implement client-side rate limiting
- Use appropriate tier for your usage

### 5. Data Privacy
- Follow data minimization principles
- Implement proper data retention policies
- Ensure user consent for data access
- Use encryption for sensitive data

### 6. Testing
- Test against development environment first
- Implement comprehensive error scenarios
- Use mock data for testing
- Validate all tool parameters

### 7. Monitoring
- Monitor API response times
- Track error rates
- Set up alerts for failures
- Log all API interactions

## Support

For technical support and questions:

- **Documentation**: https://docs.pierre.ai/a2a
- **GitHub Issues**: https://github.com/jfarcand/pierre_mcp_server/issues
- **Email**: support@pierre.ai
- **Discord**: https://discord.gg/pierre-ai

## License

The A2A Protocol implementation in Pierre is licensed under Apache 2.0 or MIT License.