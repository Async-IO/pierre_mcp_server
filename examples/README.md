# Pierre Fitness API - Examples

This directory contains examples demonstrating A2A (Agent-to-Agent) protocol integration with autonomous agents.

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
- **Use Case**: Real-time fitness analysis for AI assistants (Claude, ChatGPT)
- **Benefits**: Low latency, interactive queries, tool-based interface
- **Target**: AI assistant integration, human-in-the-loop analysis

### 🤖 A2A (Agent-to-Agent)
- **Use Case**: Autonomous agents, business automation, batch processing
- **Benefits**: High throughput, autonomous operation, scalable architecture
- **Target**: Business automation, autonomous analysis, system integration

## Directory Structure

```
examples/
├── agents/
│   └── fitness_analyzer/        # Autonomous fitness analysis agent
│       ├── src/                 # Agent implementation
│       ├── tests/               # Comprehensive test suite
│       ├── Cargo.toml           # Dependencies and configuration
│       └── README.md            # Agent documentation
├── data/                        # Sample data for testing
└── README.md                    # This file
```

## Protocol Comparison

| Feature | MCP | A2A |
|---------|-----|-----|
| Connection | WebSocket/HTTP | REST API |
| Latency | Ultra-low | Standard |
| Throughput | Medium | High |
| State | Stateful | Stateless |
| Use Case | Interactive | Autonomous |
| Auth | JWT | Client Credentials |

## Getting Started

1. **Start Pierre Server**:
   ```bash
   cargo run --bin pierre-mcp-server
   ```

2. **Setup A2A Client**:
   ```bash
   # Register A2A client (get admin token first)
   curl -X POST http://localhost:8081/a2a/clients \
     -H "Authorization: Bearer $ADMIN_TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"name": "Fitness Analyzer", "description": "Autonomous fitness analysis"}'
   ```

3. **Run Agent**:
   ```bash
   cd examples/agents/fitness_analyzer
   cargo run
   ```

The agent will authenticate via A2A protocol and begin autonomous fitness data analysis.