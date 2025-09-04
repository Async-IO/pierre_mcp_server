# Fitness Analysis Agent

A fully autonomous agent demonstrating real-world A2A (Agent-to-Agent) protocol usage for fitness data analysis.

## Overview

This agent showcases:
- **Autonomous Operation**: Runs on schedule without human intervention
- **A2A Protocol**: Direct JSON-RPC over HTTP communication with Pierre server
- **Intelligent Analysis**: Detects fitness patterns, performance trends, and risk indicators
- **Business Value**: Generates actionable insights and automated reports

## Architecture

```
FitnessAnalysisAgent
├── a2a_client.rs      # Raw A2A JSON-RPC client
├── analyzer.rs        # Pattern detection and analysis
├── scheduler.rs       # Autonomous scheduling system
├── config.rs          # Configuration management
└── main.rs           # Agent entry point
```

## Key Features

### 🔐 A2A Authentication
- Client credentials flow with automatic token refresh
- Demonstrates production-ready authentication patterns
- Shows actual JSON-RPC request/response format

### 🔬 Intelligent Analysis
- **Pattern Detection**: Training frequency, sport distribution, progression trends
- **Risk Assessment**: Overtraining, injury risk, training monotony
- **Performance Trends**: Pace, distance, frequency analysis
- **Recommendations**: Actionable training advice

### ⏰ Autonomous Scheduling
- Configurable analysis intervals (1 hour to daily)
- Development mode for testing (single analysis)
- Production mode for continuous operation
- Automatic error recovery and retry logic

### 📊 Reporting System
- JSON analysis reports with timestamps
- Execution statistics and performance metrics
- Automatic cleanup of old reports
- Configurable output directory

## Quick Start

### 1. Prerequisites

**Start Pierre Server:**
```bash
cd pierre_mcp_server
cargo run --bin pierre-mcp-server
```

**Register A2A Client:**
```bash
# Get admin token first
ADMIN_TOKEN=$(curl -s -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{"email": "admin@example.com", "password": "SecurePass123!", "display_name": "Admin"}' | \
  jq -r '.admin_token')

# Register A2A client
curl -X POST http://localhost:8081/a2a/clients \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Fitness Analyzer Agent",
    "description": "Autonomous fitness data analysis",
    "capabilities": ["fitness-data-analysis", "pattern-detection"]
  }'
```

### 2. Configuration

Copy and customize environment configuration:
```bash
cd examples/agents/fitness_analyzer
cp .env.example .env
# Edit .env with your A2A client credentials
```

### 3. Run the Agent

**Demo Mode (Recommended First Run):**
```bash
./run.sh --setup-demo --dev
```

**Development Mode:**
```bash
export PIERRE_A2A_CLIENT_ID="your_client_id"
export PIERRE_A2A_CLIENT_SECRET="your_client_secret"
./run.sh --dev
```

**Production Mode:**
```bash
./run.sh --production
```

## A2A Protocol Demonstration

The agent demonstrates raw A2A protocol usage:

### Authentication
```json
POST /a2a/auth
{
  "client_id": "fitness_analyzer_client",
  "client_secret": "client_secret_here",
  "grant_type": "client_credentials",
  "scope": "read write"
}

Response:
{
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbG...",
  "expires_in": 3600,
  "token_type": "Bearer"
}
```

### Tool Execution
```json
POST /a2a/execute
Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbG...
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "get_activities",
    "arguments": {
      "provider": "strava",
      "limit": 100
    }
  },
  "id": "req_12345"
}

Response:
{
  "jsonrpc": "2.0",
  "result": [
    {
      "id": "activity_123",
      "name": "Morning Run",
      "sport_type": "Run",
      "distance_meters": 5000,
      "duration_seconds": 1800,
      "start_date": "2024-01-15T08:00:00Z"
    }
  ],
  "id": "req_12345"
}
```

## Analysis Capabilities

### Pattern Detection
- **Training Frequency**: High, moderate, or low frequency patterns
- **Sport Distribution**: Specialization vs variety analysis
- **Progression Patterns**: Distance and performance trends
- **Weekly Rhythms**: Peak training day identification

### Risk Assessment
- **Volume Spikes**: Sudden training load increases
- **Recovery Risk**: Insufficient rest day detection
- **Monotony Risk**: Lack of training variety

### Performance Trends
- **Pace Analysis**: Speed improvement or decline
- **Distance Progression**: Training volume changes
- **Frequency Changes**: Activity consistency trends
- **Heart Rate Trends**: Fitness indicator analysis

## Example Output

```
🤖 Starting Fitness Analysis Agent
📡 Demonstrating A2A Protocol Integration

🔐 Authenticating via A2A protocol
✅ A2A authentication successful, token expires in 3600s

📊 Fetching 200 activities from strava via A2A
✅ Retrieved 187 activities via A2A

🔬 Starting comprehensive fitness analysis
📊 Analysis Summary:
  • Activities analyzed: 187
  • Patterns detected: 4
  • Recommendations: 3
  • Risk indicators: 1
  • Performance trend: improving

🔍 DETECTED PATTERNS:
1. High training frequency: 5.2 activities per week (confidence: 90%)
2. Sport specialization: 85% Run activities (confidence: 90%)
3. Increasing distance trend: +127m per activity (confidence: 80%)
4. Weekly rhythm: Peak activity on Saturday and Sunday (confidence: 70%)

💡 RECOMMENDATIONS:
1. [HIGH] Prioritize Recovery: High training frequency detected
2. [MEDIUM] Add Cross-Training: Consider adding variety to prevent overuse
3. [MEDIUM] Progressive Loading: Continue gradual distance increases

⚠️ RISK INDICATORS:
1. [MEDIUM] Volume spike (45% probability): Training volume increased by 35%

📄 Analysis report saved: fitness_analysis_report_20240115_142337.json
✅ Agent completed successfully
```

## Configuration Options

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PIERRE_A2A_CLIENT_ID` | *required* | A2A client identifier |
| `PIERRE_A2A_CLIENT_SECRET` | *required* | A2A client secret |
| `PIERRE_SERVER_URL` | `http://localhost:8081` | Pierre server base URL |
| `ANALYSIS_INTERVAL_HOURS` | `24` | Hours between analyses |
| `DEVELOPMENT_MODE` | `false` | Single analysis vs continuous |
| `MAX_ACTIVITIES_PER_ANALYSIS` | `200` | Activity limit per analysis |
| `GENERATE_REPORTS` | `true` | Enable JSON report generation |
| `REPORT_OUTPUT_DIR` | `/tmp/fitness_reports` | Report output directory |

### Command Line Options

```bash
./run.sh [OPTIONS]

Options:
  --dev                    Development mode (single analysis)
  --production            Production mode (continuous operation)
  --validate-only         Validate configuration only
  --setup-demo            Setup demo environment
  --help                  Show help message
```

## Testing

### Unit Tests
```bash
cargo test unit_tests
```

### Integration Tests
```bash
cargo test integration_tests
```

### Full Test Suite
```bash
cargo test
```

## Deployment Considerations

### Production Deployment
- Use secure A2A credentials (not demo values)
- Configure appropriate analysis intervals
- Set up log rotation for `agent.log`
- Monitor report directory disk usage
- Consider running as systemd service

### Monitoring
- Check `agent.log` for execution details
- Monitor report generation in output directory
- Track A2A authentication token refresh cycles
- Watch for pattern detection accuracy

### Scaling
- Multiple agents can run with different configurations
- Consider load balancing for high-frequency analysis
- Implement external alert system for high-risk indicators

## Troubleshooting

### Common Issues

**A2A Authentication Fails:**
```bash
# Verify client credentials
curl -X POST http://localhost:8081/a2a/auth \
  -H "Content-Type: application/json" \
  -d '{"client_id": "your_id", "client_secret": "your_secret"}'

# Check if client is registered
curl "http://localhost:8081/a2a/clients" \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```

**No Activities Retrieved:**
- Ensure user has connected Strava/Fitbit via Pierre UI
- Check OAuth token validity in Pierre server logs
- Verify A2A client has proper scopes

**Agent Crashes:**
- Check `agent.log` for detailed error messages
- Validate environment variables with `./run.sh --validate-only`
- Ensure Pierre server is accessible and healthy

### Debug Mode
```bash
export RUST_LOG=debug
./run.sh --dev
```

## Business Value

This agent demonstrates how A2A protocol enables:

1. **Autonomous Operation**: No human intervention required
2. **Scalable Analysis**: Process hundreds of activities efficiently
3. **Business Intelligence**: Generate actionable insights automatically
4. **System Integration**: Easy integration with existing business systems
5. **Cost Efficiency**: Automated analysis reduces manual effort

## License

This example is part of the Pierre MCP Server project and follows the same dual licensing:
- Apache License, Version 2.0
- MIT License