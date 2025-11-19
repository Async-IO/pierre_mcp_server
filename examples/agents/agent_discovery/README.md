# Agent Discovery Example

Demonstrates A2A protocol's **agent card discovery** and **capability negotiation** mechanisms.

## What This Example Demonstrates

### A2A Agent Discovery Workflow:
1. **Fetch Agent Card** - Retrieve agent capabilities via `/a2a/agent-card` endpoint
2. **Parse Capabilities** - Analyze available tools, authentication methods, and features
3. **Capability Matching** - Check if agent supports required capabilities
4. **Tool Discovery** - Find relevant tools for specific tasks
5. **Authentication Negotiation** - Determine best auth method (OAuth2, API Key, etc.)
6. **Suitability Assessment** - Decide if agent fits the use case

## Why Agent Discovery Matters

In A2A protocol, agents **must discover each other's capabilities** before collaboration:

- **Avoid Assumptions**: Don't assume what an agent can do
- **Dynamic Discovery**: Agents advertise capabilities that may change over time
- **Informed Decisions**: Choose the right agent for the task
- **Auth Negotiation**: Select appropriate authentication method
- **Version Compatibility**: Check protocol and tool versions

## Quick Start

```bash
# 1. Start Pierre server (in another terminal)
cd ../../../
cargo run --bin pierre-mcp-server

# 2. Run the discovery example
cd examples/agents/agent_discovery
cargo run
```

## Example Output

```
🚀 A2A Agent Discovery Example
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📡 Fetching agent card from: http://localhost:8081
✅ Successfully fetched agent card for: Pierre Fitness AI

📊 Agent Capability Analysis:
   Agent: Pierre Fitness AI v1.0.0
   Description: AI-powered fitness data analysis and insights platform...

🔧 Available Capabilities (6):
   • fitness-data-analysis
   • activity-intelligence
   • goal-management
   • performance-prediction
   • training-analytics
   • provider-integration

🛠️  Available Tools (4):
   • get_activities - Retrieve user fitness activities from connected providers
   • analyze_activity - AI-powered analysis of a specific fitness activity
   • get_athlete - Retrieve athlete profile information
   • set_goal - Set a fitness goal for the user

🔐 Authentication Methods:
   • api-key
   • oauth2

   OAuth2 Configuration:
      Authorization URL: https://pierre.ai/oauth/authorize
      Token URL: https://pierre.ai/oauth/token
      Scopes: fitness:read, analytics:read, goals:read, goals:write

🔍 Capability Check:
   ✅ Has capability: fitness-data-analysis
   ✅ Has capability: activity-intelligence
   ✅ Has capability: performance-prediction

🔎 Finding fitness-related tools:
   • get_activities - Retrieve user fitness activities from connected providers
   • analyze_activity - AI-powered analysis of a specific fitness activity

🔐 Authentication Method Recommendation:
💡 Recommendation: Use OAuth2 for secure user-delegated access

✅ Agent Suitability Assessment:
   ✅ This agent is suitable for fitness data analysis tasks
   ✅ Supports 2 tools for fitness analysis
   ✅ Recommended for integration
```

## Key Concepts Demonstrated

### 1. Agent Card Fetching
```rust
let agent_card = self.fetch_agent_card().await?;
```
The agent card is a JSON document describing the agent's capabilities, similar to OpenAPI/Swagger for REST APIs.

### 2. Capability Checking
```rust
fn has_capability(&self, card: &AgentCard, required_capability: &str) -> bool {
    card.capabilities.iter().any(|cap| cap.contains(required_capability))
}
```
Before using an agent, check if it supports the required capabilities.

### 3. Tool Discovery
```rust
let fitness_tools = self.find_tools(&agent_card, "activit");
```
Find relevant tools by name or description matching.

### 4. Authentication Selection
```rust
fn recommend_auth_method(&self, card: &AgentCard) -> String {
    // Choose OAuth2 for user delegation, API Key for service-to-service
}
```
Select the appropriate authentication method based on use case.

## Real-World Use Cases

1. **Multi-Agent Systems**: Before delegating a task to another agent, check if it has the required capabilities
2. **Dynamic Agent Selection**: Choose from multiple available agents based on their advertised capabilities
3. **Version Compatibility**: Ensure agent supports the required protocol version and tools
4. **Fallback Strategies**: If preferred agent is unavailable, select alternative based on capability match

## Integration with Other Examples

- **fitness_analyzer**: Uses this discovery pattern before connecting to Pierre
- **task_manager**: Discovers task management capabilities before creating long-running tasks
- **multi_agent**: Multiple agents discover each other's capabilities for collaboration

## Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `PIERRE_SERVER_URL` | `http://localhost:8081` | Pierre server base URL |

## A2A Specification Compliance

This example demonstrates the following A2A specification requirements:

- ✅ Agent Card Format (name, version, capabilities, tools, authentication)
- ✅ Capability Discovery (fetching and parsing agent cards)
- ✅ Authentication Negotiation (OAuth2, API Key selection)
- ✅ Tool Schema Discovery (input/output schemas)
- ✅ Metadata Parsing (rate limits, supported providers, contact info)

## Learn More

- [A2A Protocol Specification](https://github.com/google/A2A)
- [Pierre A2A Documentation](../../../docs/tutorial/chapter-18-a2a-protocol.md)
- [Agent Card Design](../../../src/a2a/agent_card.rs)
