# Python Client Examples

This directory contains comprehensive examples for integrating with Pierre Fitness API using both supported protocols.

## 📁 Structure

```
examples/python/
├── provisioning/           # Enterprise provisioning examples
│   ├── provisioning_example.py    # Complete B2B workflow demo
│   └── mock_strava_provider.py    # Realistic fitness data generator
├── mcp/                    # Model Context Protocol examples
│   ├── data_collection.py  # Bulk activity collection via MCP
│   └── investor_demo.py    # Complete investor demonstration
├── a2a/                    # Agent-to-Agent Protocol examples  
│   ├── api_client.py       # A2A REST API client library
│   └── enterprise_demo.py  # Enterprise A2A demonstration
├── common/                 # Shared utilities
│   ├── auth_utils.py       # Authentication helpers
│   └── data_utils.py       # Data processing & analytics
└── README.md              # This file
```

## 🚀 Quick Start

### Enterprise Provisioning (B2B Setup)
```bash
# Complete enterprise provisioning workflow
python examples/python/provisioning/provisioning_example.py

# Generate mock fitness data for testing
python examples/python/provisioning/mock_strava_provider.py
```

### MCP Protocol (Real-time Analysis)
```bash
# Collect fitness data via MCP
python examples/python/mcp/data_collection.py

# Full investor demonstration with real-time analysis
python examples/python/mcp/investor_demo.py
```

### A2A Protocol (Enterprise Integration)
```bash
# Test A2A API client
python examples/python/a2a/api_client.py

# Complete enterprise demonstration
python examples/python/a2a/enterprise_demo.py
```

## 🔧 Protocol Comparison

| Feature | MCP Protocol | A2A Protocol |
|---------|-------------|--------------|
| **Use Case** | Real-time analysis | Enterprise integration |
| **Connection** | WebSocket/TCP | HTTP REST API |
| **Authentication** | JWT tokens | API keys |
| **Best For** | Interactive clients | Server-to-server |
| **Performance** | Low latency | High throughput |

## 📊 Available Examples

### Provisioning Examples
- **provisioning_example.py** - Complete B2B enterprise provisioning workflow based on [docs/PROVISIONING.md](../../docs/PROVISIONING.md)
- **mock_strava_provider.py** - Realistic fitness data generator for testing and development

### MCP Examples
- **data_collection.py** - Shows how to connect to MCP server and collect fitness activities
- **investor_demo.py** - Complete demonstration featuring real-time fitness analysis, scoring, and insights

### A2A Examples
- **api_client.py** - A2A client library demonstrating authentication, tool execution, and API usage
- **enterprise_demo.py** - Full enterprise demonstration with bulk processing, analytics, and reporting

## 🛠️ Setup Requirements

### Dependencies
```bash
pip install requests websockets aiohttp pandas matplotlib
```

### Environment Variables
```bash
# For MCP examples
export MCP_SERVER_HOST=localhost
export MCP_SERVER_PORT=8080

# For A2A examples  
export PIERRE_API_BASE=http://localhost:8081
export PIERRE_API_KEY=your_api_key_here
```

### Authentication Setup
1. **JWT Authentication** (MCP): Use login credentials
2. **API Key Authentication** (A2A): Generate via dashboard or API

## 📈 Performance Guidelines

### MCP Protocol
- **Ideal for:** <100 concurrent connections
- **Latency:** <50ms response time
- **Use when:** Real-time interactivity needed

### A2A Protocol  
- **Ideal for:** High-volume batch processing
- **Throughput:** 1000+ requests/minute
- **Use when:** Enterprise integration required

## 🎯 Getting Started

1. **Choose Protocol** based on your use case
2. **Set up Authentication** (JWT or API key)
3. **Run Basic Example** to verify connection
4. **Adapt to Your Needs** using provided templates

## 💼 What the Examples Demonstrate

### Enterprise Provisioning Examples
- **provisioning_example.py**: Multi-tenant B2B workflows, admin token generation, API key provisioning, customer integration patterns
- **mock_strava_provider.py**: Realistic fitness data generation, testing scenarios, performance simulation

### Common Utilities
- **auth_utils.py**: JWT authentication, API key management, environment configuration
- **data_utils.py**: Fitness scoring algorithms, data validation, anonymization for privacy

### MCP Protocol Examples
- **data_collection.py**: WebSocket connection, real-time data streaming, error handling
- **investor_demo.py**: Complete fitness analysis workflow, AI insights generation, report creation

### A2A Protocol Examples  
- **api_client.py**: REST API integration, authentication flows, tool execution
- **enterprise_demo.py**: Bulk processing, enterprise reporting, API usage monitoring

## 🔐 Security Best Practices

- Store API keys and credentials securely
- Use environment variables for configuration
- Implement proper error handling
- Monitor API usage and limits
- Follow rate limiting guidelines

---

*Choose the protocol that best fits your integration needs. Both provide access to the same powerful AI fitness intelligence.*