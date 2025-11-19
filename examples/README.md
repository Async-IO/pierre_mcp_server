# Pierre A2A Examples

Comprehensive examples demonstrating the **A2A (Agent-to-Agent) protocol** for autonomous agent communication.

## What is A2A?

The **Agent-to-Agent (A2A) protocol** is an open standard for AI agent communication, developed by Google and housed by the Linux Foundation. A2A enables autonomous agents to:

- **Discover Capabilities**: Find and evaluate other agents via agent cards
- **Delegate Tasks**: Request work from other agents without understanding their internals
- **Monitor Progress**: Track long-running tasks with status updates
- **Collaborate**: Multiple agents working together on complex problems
- **Operate Autonomously**: Agents run independently, making decisions without human intervention

## Available Examples

### 1. **Agent Discovery** (`agents/agent_discovery/`)
**What it demonstrates**: Agent card discovery and capability negotiation

Learn how agents discover each other's capabilities before collaboration:
- Fetch agent cards to see available tools and features
- Parse and validate agent capabilities
- Negotiate authentication methods (OAuth2, API Key)
- Make informed decisions about which agent to use

```bash
cd agents/agent_discovery
cargo run
```

**Key Concepts**: Agent cards, capability matching, authentication selection

---

### 2. **Task Lifecycle Management** (`agents/task_manager/`)
**What it demonstrates**: Long-running task management and status tracking

See how A2A handles asynchronous operations:
- Create tasks for long-running analysis
- Poll task status (pending → running → completed/failed)
- Retrieve task results when complete
- List and filter tasks by status
- Handle task failures gracefully

```bash
cd agents/task_manager
export PIERRE_A2A_CLIENT_ID="your_client_id"
export PIERRE_A2A_CLIENT_SECRET="your_secret"
cargo run
```

**Key Concepts**: Task lifecycle, status polling, asynchronous execution

---

### 3. **Fitness Analysis Agent** (`agents/fitness_analyzer/`)
**What it demonstrates**: Production-ready autonomous agent

A complete autonomous agent that:
- Runs continuously without human intervention
- Authenticates via A2A client credentials
- Fetches fitness data from connected providers
- Performs intelligent pattern analysis
- Generates JSON reports with insights
- Handles errors and retries gracefully

```bash
cd agents/fitness_analyzer
./run.sh --setup-demo --dev
```

**Key Concepts**: Autonomous operation, data analysis, production deployment

---

## A2A Protocol Architecture

### Core Components

```
┌─────────────┐                    ┌─────────────┐
│ Agent A     │                    │ Agent B     │
│ (Client)    │                    │ (Remote)    │
└──────┬──────┘                    └──────┬──────┘
       │                                  │
       │ 1. GET /a2a/agent-card          │
       ├─────────────────────────────────>│
       │ 2. Agent Card (capabilities)    │
       │<─────────────────────────────────┤
       │                                  │
       │ 3. POST /a2a/auth               │
       ├─────────────────────────────────>│
       │ 4. Session Token                │
       │<─────────────────────────────────┤
       │                                  │
       │ 5. POST /a2a/execute (tool)     │
       ├─────────────────────────────────>│
       │ 6. Tool Result                  │
       │<─────────────────────────────────┤
       │                                  │
       │ 7. POST /a2a/execute (task)     │
       ├─────────────────────────────────>│
       │ 8. Task ID                      │
       │<─────────────────────────────────┤
       │                                  │
       │ 9. Poll task status             │
       ├─────────────────────────────────>│
       │ 10. Task status + result        │
       │<─────────────────────────────────┤
```

### A2A vs MCP: When to Use Each

| Scenario | Protocol | Reason |
|----------|----------|--------|
| AI assistant (Claude, ChatGPT) asking questions | **MCP** | Interactive, low-latency, human-in-loop |
| Scheduled fitness report generation | **A2A** | Autonomous, no human needed |
| Real-time data exploration | **MCP** | Stateful session, context preservation |
| Multi-agent collaboration | **A2A** | Agents delegating work to each other |
| Batch processing 1000s of records | **A2A** | High throughput, async tasks |
| Interactive debugging | **MCP** | Rich tooling, IDE integration |

**Rule of Thumb**:
- Use **MCP** when a human or AI assistant is actively involved
- Use **A2A** when agents work autonomously or delegate to other agents

---

## Quick Start Guide

### Prerequisites

1. **Start Pierre Server**:
   ```bash
   cd pierre_mcp_server
   cargo run --bin pierre-mcp-server
   ```

2. **Register A2A Client** (one-time setup):
   ```bash
   # Get admin token
   ADMIN_TOKEN=$(curl -s -X POST http://localhost:8081/admin/setup \
     -H "Content-Type: application/json" \
     -d '{"email": "admin@example.com", "password": "SecurePass123!", "display_name": "Admin"}' | \
     jq -r '.admin_token')

   # Register A2A client
   CREDENTIALS=$(curl -s -X POST http://localhost:8081/a2a/clients \
     -H "Authorization: Bearer $ADMIN_TOKEN" \
     -H "Content-Type: application/json" \
     -d '{
       "name": "Demo Agent",
       "description": "A2A demo client",
       "capabilities": ["fitness-analysis"],
       "contact_email": "demo@example.com"
     }')

   echo $CREDENTIALS | jq '.'
   # Save client_id and client_secret for examples
   ```

3. **Connect Provider** (for fitness_analyzer):
   - Open http://localhost:8081 in browser
   - Sign up / login
   - Connect Strava or Fitbit account

### Running Examples

```bash
# 1. Agent Discovery (no auth needed)
cd agents/agent_discovery
cargo run

# 2. Task Manager (needs A2A credentials)
cd agents/task_manager
export PIERRE_A2A_CLIENT_ID="your_client_id"
export PIERRE_A2A_CLIENT_SECRET="your_client_secret"
cargo run

# 3. Fitness Analyzer (automated setup)
cd agents/fitness_analyzer
./run.sh --setup-demo --dev
```

---

## Directory Structure

```
examples/
├── agents/
│   ├── agent_discovery/         # Agent card discovery & capability negotiation
│   │   ├── src/main.rs         # Discovery client implementation
│   │   ├── Cargo.toml          # Dependencies
│   │   └── README.md           # Detailed documentation
│   │
│   ├── task_manager/           # Task lifecycle management
│   │   ├── src/main.rs         # Task polling and monitoring
│   │   ├── Cargo.toml          # Dependencies
│   │   └── README.md           # Detailed documentation
│   │
│   └── fitness_analyzer/       # Production autonomous agent
│       ├── src/
│       │   ├── main.rs         # Entry point
│       │   ├── a2a_client.rs   # A2A protocol client
│       │   ├── analyzer.rs     # Fitness analysis logic
│       │   ├── scheduler.rs    # Autonomous scheduling
│       │   └── config.rs       # Configuration management
│       ├── tests/              # Unit and integration tests
│       ├── run.sh              # Helper script
│       ├── Cargo.toml          # Dependencies
│       └── README.md           # Detailed documentation
│
└── README.md                   # This file
```

---

## Key A2A Concepts

### 1. Agent Cards

Agent cards are JSON documents describing an agent's capabilities, similar to OpenAPI specs for REST APIs.

**What's in an agent card:**
- Agent name, version, description
- Available capabilities (e.g., "fitness-data-analysis")
- Tool definitions with input/output schemas
- Authentication methods (OAuth2, API Key)
- Rate limits, contact info, metadata

**Why it matters**: Agents discover each other's capabilities dynamically, avoiding hard-coded assumptions.

### 2. Task Management

A2A supports both synchronous and asynchronous task execution.

**Task Lifecycle:**
```
pending → running → completed
                 ↘ failed
                 ↘ cancelled
```

**When to use tasks:**
- Operations taking > 30 seconds
- Batch processing
- Scheduled jobs
- Operations that may fail and need retry logic

### 3. JSON-RPC 2.0

A2A uses JSON-RPC 2.0 over HTTP(S) as the transport protocol.

**Standard methods:**
- `a2a/initialize` - Protocol handshake
- `tools/list` - Get available tools
- `tools/call` - Execute a tool
- `tasks/create` - Create long-running task
- `tasks/get` - Query task status
- `tasks/list` - List all tasks
- `tasks/cancel` - Cancel a task

### 4. Authentication

A2A supports multiple authentication schemes:

| Method | Use Case | Example |
|--------|----------|---------|
| **Client Credentials** | Service-to-service | Automated agents |
| **OAuth2** | User-delegated access | User authorizes agent to access their data |
| **API Key** | Simple service auth | Quick integrations |

---

## A2A Specification Compliance

Pierre's A2A implementation follows the [official A2A specification](https://github.com/google/A2A):

- ✅ Agent cards with capability discovery
- ✅ JSON-RPC 2.0 over HTTP(S)
- ✅ Client credentials authentication
- ✅ OAuth2 authentication support
- ✅ Task lifecycle management (create, get, list, cancel)
- ✅ Tool execution with schemas
- ✅ Error handling with standard codes
- ⚠️ Server-Sent Events (SSE) - acknowledged as not supported (stateless design preference)
- ⚠️ Webhooks - configured but not yet active

---

## Protocol Comparison

| Feature | MCP | A2A |
|---------|-----|-----|
| **Communication** | WebSocket/SSE | HTTP REST |
| **Session Model** | Stateful | Stateless |
| **Latency** | Ultra-low (ms) | Standard (100s ms) |
| **Throughput** | Medium | High |
| **Use Case** | Interactive | Autonomous |
| **Auth** | JWT (user context) | Client credentials |
| **Context** | Rich session context | Request/response only |
| **Discovery** | Resources/Tools | Agent cards |
| **Best For** | AI assistants, IDEs | Agent-to-agent, automation |

---

## Learn More

- **A2A Specification**: [github.com/google/A2A](https://github.com/google/A2A)
- **Pierre A2A Documentation**: [docs/tutorial/chapter-18-a2a-protocol.md](../docs/tutorial/chapter-18-a2a-protocol.md)
- **MCP Specification**: [modelcontextprotocol.io](https://modelcontextprotocol.io)
- **Pierre MCP Documentation**: [docs/tutorial/chapter-01-introduction.md](../docs/tutorial/chapter-01-introduction.md)

---

## Troubleshooting

### "Authentication failed"
- Make sure you've registered an A2A client
- Check that `PIERRE_A2A_CLIENT_ID` and `PIERRE_A2A_CLIENT_SECRET` are set correctly
- Verify Pierre server is running

### "No activities found"
- Connect a Strava or Fitbit account via Pierre web UI
- Ensure OAuth connection is active
- Check server logs for provider API errors

### "Agent card fetch failed"
- Verify Pierre server is running on the expected URL
- Check `PIERRE_SERVER_URL` environment variable
- Ensure `/a2a/agent-card` endpoint is accessible

---

## Contributing

Found a bug or want to add a new example? Contributions welcome!

1. Examples should demonstrate real-world A2A usage patterns
2. Include comprehensive README with "What it demonstrates" section
3. Follow Rust best practices and Pierre coding standards
4. Add tests for new functionality

---

## License

All examples are licensed under either:
- Apache License, Version 2.0
- MIT License

at your option.