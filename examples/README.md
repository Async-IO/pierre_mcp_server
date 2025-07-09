# Pierre Fitness API - Examples

This directory contains comprehensive examples demonstrating both MCP (Model Context Protocol) and A2A (Agent-to-Agent) protocols for the Pierre Fitness API.

## Quick Start

```bash
# Run the demo menu
python3 run_demos.py

# Or run individual demos
python3 python/multitenant_mcp_example.py     # NEW: Multi-tenant MCP workflow
python3 python/mcp_stdio_example.py           # NEW: MCP stdio transport
python3 python/mcp/investor_demo.py           # MCP investor demonstration
python3 python/a2a/enterprise_demo.py         # A2A enterprise demonstration
```

### Multi-Tenant MCP Setup (NEW)

For the new multi-tenant MCP examples, ensure you have:

1. **Fresh database**: `./scripts/fresh-start.sh`
2. **Admin token**: `cargo run --bin admin-setup generate-token --service "demo"`
3. **Server running**: `cargo run --bin pierre-mcp-server`

Then run the examples:
```bash
# Complete multi-tenant workflow
python3 python/multitenant_mcp_example.py

# MCP stdio transport (pipe to server)
python3 python/mcp_stdio_example.py | cargo run --bin pierre-mcp-server
```

## Architecture Overview

The Pierre Fitness API supports two protocols:

### 🔄 MCP (Model Context Protocol)
- **Use Case**: Real-time fitness analysis, mobile apps, dashboards
- **Benefits**: Low latency, interactive clients, WebSocket/TCP support
- **Target**: Consumer applications, real-time analysis

### 🏢 A2A (Agent-to-Agent) 
- **Use Case**: Enterprise integration, B2B platforms, batch processing
- **Benefits**: High throughput, REST API, scalable architecture
- **Target**: Enterprise clients, system integration

## Directory Structure

```
examples/
├── python/
│   ├── mcp/                    # MCP Protocol Examples
│   │   ├── data_collection.py  # Bulk data collection via MCP
│   │   └── investor_demo.py    # Complete investor demonstration
│   ├── a2a/                    # A2A Protocol Examples  
│   │   ├── api_client.py       # Enterprise API client
│   │   └── enterprise_demo.py  # Complete enterprise demonstration
│   ├── multitenant_mcp_example.py  # Multi-tenant MCP workflow
│   ├── mcp_stdio_example.py    # MCP stdio transport example
│   └── common/                 # Shared Utilities
│       ├── auth_utils.py       # Authentication management
│       └── data_utils.py       # Fitness data processing
├── run_demos.py               # Demo runner script
└── README.md                  # This file
```

## Protocol Comparison

| Feature | MCP | A2A |
|---------|-----|-----|
| Connection | WebSocket/TCP | REST API |
| Latency | Ultra-low | Standard |
| Throughput | Medium | High |
| State | Stateful | Stateless |
| Use Case | Real-time | Batch/Enterprise |
| Auth | JWT | JWT + API Keys |

## Examples Guide

### 1. MCP Examples

#### Multi-Tenant MCP Workflow (`python/multitenant_mcp_example.py`)
- **NEW**: Complete multi-tenant setup workflow
- User registration and JWT authentication
- Strava OAuth integration
- MCP protocol usage (HTTP transport)
- Real fitness data analysis with AI insights
- Rate limiting and error handling

#### MCP stdio Transport (`python/mcp_stdio_example.py`)
- **NEW**: MCP stdio transport (primary for AI assistants)
- JSON-RPC 2.0 message format
- Interactive and pipe modes
- Proper MCP protocol sequence
- Compliant with MCP 2024-11-05 specification

#### Data Collection (`python/mcp/data_collection.py`)
- Connects to MCP server via WebSocket
- Collects fitness activities in real-time
- Performs data quality validation
- Saves results for analysis

#### Investor Demo (`python/mcp/investor_demo.py`)
- Complete demonstration for investors
- Real-time fitness scoring and analysis
- Sport-specific intelligence
- Interactive presentation format

### 2. A2A Examples

#### API Client (`python/a2a/api_client.py`)
- REST API client with comprehensive error handling
- API key management and authentication
- Bulk data processing capabilities
- Robust error handling and retry logic

#### Enterprise Demo (`python/a2a/enterprise_demo.py`)
- Complete B2B client demonstration
- High-volume data processing
- Enterprise reporting and analytics
- API usage monitoring

### 3. Common Utilities

#### Authentication (`python/common/auth_utils.py`)
- JWT token management with caching
- API key retrieval and storage (keys provisioned by admin)
- Environment-based configuration
- Robust error handling

#### Data Processing (`python/common/data_utils.py`)
- Advanced fitness scoring algorithms
- Sport-specific performance analysis
- Data quality validation
- Comprehensive metrics calculation

## Authentication Setup

### Environment Variables
```bash
# Optional - defaults provided
export PIERRE_EMAIL="your-email@example.com"
export PIERRE_PASSWORD="your-password"
export PIERRE_API_KEY="your-api-key"
export MCP_SERVER_HOST="localhost"
export MCP_SERVER_PORT="8080"
```

### Automatic Authentication
All examples include automatic authentication setup:
- JWT tokens are cached for 23 hours
- API keys are retrieved from environment or config (provisioned by admin)
- Fallback to default test credentials

## Prerequisites

### For Real Data (Production Demo):

1. **Server Running**: Ensure Pierre AI server is running on localhost:8081
2. **MCP Server**: MCP server should be available on localhost:8080  
3. **Strava Connected**: Complete Strava OAuth connection for real fitness data
4. **Python Dependencies**: Install required packages

```bash
pip3 install requests websockets asyncio
```

### For CI/Testing (Mock Data):

1. **Python Only**: No server required - examples use mock data automatically
2. **Dependencies**: Same Python packages as above
3. **Automatic Fallback**: Examples detect server availability and use mock data when needed

## Data Modes

The examples support two data modes:

### 🔴 **Real Data Mode** (Commented Out by Default)
- Connects to live Pierre AI server
- Uses actual Strava/Fitbit fitness data  
- Requires OAuth setup and running server
- **Privacy**: Data is automatically anonymized (GPS, names, personal details removed)

### 🟢 **Mock Data Mode** (Default for CI)
- Generates realistic synthetic fitness activities
- No server dependencies required
- Safe for CI/CD pipelines and testing
- Produces similar analytics results for demonstration

## Switching Between Data Modes

### To Enable Real Data:

1. **Start Pierre AI Server**:
   ```bash
   cargo run --bin pierre-mcp-server
   ```

2. **Connect Strava Account**:
   ```bash
   # Visit in browser and complete OAuth
   curl "http://localhost:8081/auth/strava/connect"
   ```

3. **Uncomment Real Data Sections** in Python files:
   ```python
   # FOR REAL DATA: Uncomment this section and ensure Pierre AI server is running
   # with connected Strava account (see README.md for OAuth setup)
   #
   # activities = client.get_activities(limit=200)  # Real Strava data
   ```

### To Use Mock Data (Default):

- No changes needed - examples automatically use mock data when server unavailable
- Safe for CI/CD pipelines and testing environments
- All analytics and reports work identically

## Example Output

### MCP Investor Demo
```
🚀 PIERRE AI FITNESS PLATFORM - MCP DEMONSTRATION
============================================================
🎯 Purpose: Real-time AI fitness analysis for investors
📡 Protocol: Model Context Protocol (MCP)
⚡ Benefits: Low latency, real-time analysis, interactive clients

✅ Connected to MCP server
✅ Successfully collected 100 real activities
🏆 FITNESS SCORE: 89/100
📊 Data Quality Score: 94.2/100
```

### A2A Enterprise Demo
```
🏢 PIERRE AI FITNESS PLATFORM - A2A ENTERPRISE DEMO
=================================================================
🎯 Purpose: Scalable fitness analytics for B2B clients
📡 Protocol: Agent-to-Agent (A2A) REST API
⚡ Benefits: High throughput, scalable, multi-tenant integration

✅ Processed 200 activities in 2.34s
📈 Processing rate: 85.5 activities/second
📊 Data Quality Score: 96.8/100
```

## Troubleshooting

### Common Issues

1. **Connection Failed**
   - Check server is running: `curl http://localhost:8081/health`
   - Verify MCP server: `netstat -an | grep 8080`

2. **Authentication Issues**
   - Delete cached tokens: `rm ~/.pierre_auth.json`
   - Check credentials in environment variables

3. **No Data Retrieved**
   - Ensure Strava is connected via `/auth/strava/connect`
   - Check server logs for OAuth issues

### Debug Mode
Add debug output to any script:
```python
import logging
logging.basicConfig(level=logging.DEBUG)
```

## Production Deployment

### MCP Protocol
- Use secure WebSocket (WSS) in production
- Implement connection pooling for scale
- Add authentication middleware

### A2A Protocol  
- Deploy behind load balancer
- Implement rate limiting
- Use API key rotation
- Monitor usage metrics

## Support

For technical support or questions:
- Check server logs in `server.log`
- Review authentication setup
- Verify network connectivity
- Test with provided demo scripts