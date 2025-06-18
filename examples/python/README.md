# Python Client Examples

This directory contains comprehensive examples for integrating with Pierre's AI Fitness Platform using both supported protocols.

## 📁 Structure

```
examples/python/
├── mcp/                    # Model Context Protocol examples
│   ├── data_collection.py  # Bulk activity collection
│   ├── fitness_analysis.py # Comprehensive fitness analysis
│   ├── running_analysis.py # Sport-specific analysis
│   └── investor_demo.py    # Complete investor demonstration
├── a2a/                    # Agent-to-Agent Protocol examples  
│   ├── api_client.py       # A2A API client library
│   ├── fitness_report.py   # Generate fitness reports via A2A
│   ├── bulk_analysis.py    # Bulk data processing via A2A
│   └── enterprise_demo.py  # Enterprise A2A demonstration
├── common/                 # Shared utilities
│   ├── auth_utils.py       # Authentication helpers
│   ├── data_utils.py       # Data processing utilities
│   └── visualization.py    # Data visualization helpers
└── README.md              # This file
```

## 🚀 Quick Start

### MCP Protocol (Real-time Analysis)
```bash
# Run comprehensive fitness analysis
python examples/python/mcp/fitness_analysis.py

# Collect and analyze running data
python examples/python/mcp/running_analysis.py

# Full investor demonstration
python examples/python/mcp/investor_demo.py
```

### A2A Protocol (Enterprise Integration)
```bash
# Generate fitness report via API
python examples/python/a2a/fitness_report.py

# Enterprise bulk processing
python examples/python/a2a/bulk_analysis.py

# Complete enterprise demo
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

## 📊 Example Use Cases

### MCP Examples
- **Real-time fitness coaching** - Immediate analysis and recommendations
- **Interactive dashboards** - Live data updates and visualizations  
- **Mobile applications** - Responsive fitness tracking
- **AI assistants** - Conversational fitness guidance

### A2A Examples
- **Enterprise reporting** - Scheduled fitness report generation
- **Bulk data processing** - Large-scale analysis workflows
- **Third-party integrations** - Connect with existing systems
- **B2B services** - White-label fitness analytics

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

## 💼 Enterprise Examples

Both protocols include enterprise-ready examples demonstrating:
- Error handling and retry logic
- Rate limiting and throttling
- Bulk data processing
- Professional reporting
- Performance monitoring

## 🔐 Security Best Practices

- Store API keys and credentials securely
- Use environment variables for configuration
- Implement proper error handling
- Monitor API usage and limits
- Follow rate limiting guidelines

---

*Choose the protocol that best fits your integration needs. Both provide access to the same powerful AI fitness intelligence.*