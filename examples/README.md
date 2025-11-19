# Pierre Fitness API - Examples

This directory contains examples demonstrating both MCP (Model Context Protocol) and A2A (Agent-to-Agent) protocol integration.

## MCP Client Examples

### Gemini Fitness Assistant (`mcp_clients/gemini_fitness_assistant/`)

An interactive AI fitness assistant using **Google's free Gemini API** with Pierre MCP Server:

- **Free LLM Integration**: Uses Gemini API (1,500 requests/day, no credit card)
- **MCP Protocol**: Direct HTTP JSON-RPC communication with Pierre
- **Function Calling**: Native tool calling for fitness data analysis
- **End-to-End Example**: Complete open-source AI assistant

```bash
# Run the Gemini fitness assistant
cd examples/mcp_clients/gemini_fitness_assistant
pip install -r requirements.txt
export GEMINI_API_KEY='your-api-key'
export PIERRE_EMAIL='user@example.com'
export PIERRE_PASSWORD='password'
python gemini_fitness_assistant.py
```

Get a free Gemini API key at: https://ai.google.dev/gemini-api/docs/api-key

## Agent Examples

### Fitness Analysis Agent (`agents/fitness_analyzer/`)

A fully autonomous agent that demonstrates real-world A2A protocol usage:

- **Autonomous Operation**: Runs on schedule without human intervention
- **A2A Protocol**: Direct JSON-RPC over HTTP communication
- **Intelligent Analysis**: Detects fitness patterns, performance trends, and anomalies
- **Business Value**: Generates actionable insights and reports

```bash
# Run the fitness analysis agent
cd examples/agents/fitness_analyzer
cargo run
```

## Architecture Overview

The Pierre Fitness API supports two integration patterns:

### 🔄 MCP (Model Context Protocol)
- **Use Case**: Real-time fitness analysis for MCP clients and AI assistants
- **Benefits**: Low latency, interactive queries, tool-based interface
- **Target**: AI assistant integration, human-in-the-loop analysis

### 🤖 A2A (Agent-to-Agent)
- **Use Case**: Autonomous agents, business automation, batch processing
- **Benefits**: High throughput, autonomous operation, scalable architecture
- **Target**: Business automation, autonomous analysis, system integration

## Directory Structure

```
examples/
├── mcp_clients/
│   └── gemini_fitness_assistant/  # Interactive AI assistant with free Gemini API
│       ├── gemini_fitness_assistant.py  # Main client script
│       ├── requirements.txt       # Python dependencies
│       ├── .env.example           # Environment configuration template
│       └── README.md              # Detailed documentation
├── agents/
│   └── fitness_analyzer/          # Autonomous fitness analysis agent (A2A)
│       ├── src/                   # Agent implementation
│       ├── tests/                 # Comprehensive test suite
│       ├── Cargo.toml             # Dependencies and configuration
│       └── README.md              # Agent documentation
├── data/                          # Sample data for testing
└── README.md                      # This file
```

## Protocol Comparison

| Feature | MCP | A2A |
|---------|-----|-----|
| Connection | HTTP JSON-RPC | REST API |
| Latency | Ultra-low | Standard |
| Throughput | Medium | High |
| State | Stateful | Stateless |
| Use Case | Interactive AI Assistants | Autonomous Agents |
| Example | Gemini Fitness Assistant | Fitness Analyzer |
| Auth | JWT (OAuth2) | Client Credentials |
| Human Interaction | Yes (conversational) | No (automated) |

## Getting Started

### 1. Start Pierre Server
```bash
cargo run --bin pierre-mcp-server
```

### 2. Create a User Account
```bash
curl -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "SecurePass123!",
    "display_name": "Test User"
  }'
```

### 3. Choose Your Example

**Option A: MCP Client (Interactive AI Assistant)**
```bash
# Setup Gemini Fitness Assistant
cd examples/mcp_clients/gemini_fitness_assistant
pip install -r requirements.txt

# Get free Gemini API key: https://ai.google.dev/gemini-api/docs/api-key
export GEMINI_API_KEY='your-api-key'
export PIERRE_EMAIL='user@example.com'
export PIERRE_PASSWORD='SecurePass123!'

# Run interactive assistant
python gemini_fitness_assistant.py
```

**Option B: A2A Agent (Autonomous Analysis)**
```bash
# Register A2A client (get admin token first)
curl -X POST http://localhost:8081/a2a/clients \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "Fitness Analyzer", "description": "Autonomous fitness analysis"}'

# Run autonomous agent
cd examples/agents/fitness_analyzer
cargo run
```