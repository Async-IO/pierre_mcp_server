# Fitness Analysis Agent

Autonomous agent that demonstrates A2A protocol integration by analyzing fitness activity data.

## What This Agent Actually Does

**Data Processing:**
1. Authenticates with Pierre server using A2A client credentials
2. Fetches activity records via `get_activities` tool (runs, rides, etc.)
3. Calculates training frequency, sport distribution, distance progression
4. Detects potential overtraining patterns and injury risk indicators
5. Generates JSON reports with analysis results

**Analysis Algorithms:**
- Training frequency: activities per week calculation
- Sport distribution: percentage breakdown by activity type  
- Volume trend analysis: linear regression on distance/duration over time
- Overtraining detection: volume spike detection (>30% increase in 2-week windows)
- Weekly pattern analysis: identifies peak training days

## Architecture

```
FitnessAnalysisAgent
├── a2a_client.rs      # Raw A2A JSON-RPC client
├── analyzer.rs        # Pattern detection and analysis
├── scheduler.rs       # Autonomous scheduling system
├── config.rs          # Configuration management
└── main.rs           # Agent entry point
```

## Technical Implementation

**A2A Protocol Usage:**
- HTTP POST to `/a2a/auth` with client_id/client_secret
- JWT token management with expiry tracking
- JSON-RPC 2.0 requests to `/a2a/execute` with `tools/call` method
- Request/response correlation via UUID

**Analysis Logic:**
```rust
// Training frequency calculation
let weeks = (date_range.num_days() as f64 / 7.0).max(1.0);
let activities_per_week = activities.len() as f64 / weeks;

// Volume spike detection  
let recent_volume: u32 = recent_14.iter().map(|a| a.duration_seconds.unwrap_or(0)).sum();
let previous_volume: u32 = previous_14.iter().map(|a| a.duration_seconds.unwrap_or(0)).sum();
let volume_increase = (recent_volume as f64 / previous_volume as f64 - 1.0) * 100.0;
```

**Scheduling:**
- `tokio::time::interval()` for periodic execution
- Development mode: single run and exit
- Production mode: continuous loop with error recovery
- Configurable intervals via environment variables

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

## Actual Output

```
INFO fitness_analyzer: 🤖 Starting Fitness Analysis Agent
INFO fitness_analyzer::a2a_client: 🔐 Authenticating via A2A protocol
INFO fitness_analyzer::a2a_client: ✅ A2A authentication successful, token expires in 3600s
INFO fitness_analyzer::a2a_client: 📊 Fetching 200 activities from strava via A2A
INFO fitness_analyzer::a2a_client: ✅ Retrieved 187 activities via A2A
INFO fitness_analyzer::analyzer: 🔬 Starting comprehensive fitness analysis
INFO fitness_analyzer::analyzer: 📊 Analyzing 187 activities
INFO fitness_analyzer::analyzer: 🔍 Detected 4 patterns
INFO fitness_analyzer::scheduler: 📊 Analysis Summary:
INFO fitness_analyzer::scheduler:   • Activities analyzed: 187
INFO fitness_analyzer::scheduler:   • Patterns detected: 4
INFO fitness_analyzer::scheduler:   • Recommendations: 3
INFO fitness_analyzer::scheduler:   • Risk indicators: 1
INFO fitness_analyzer::scheduler: 📄 Analysis report saved: fitness_analysis_report_20240904_142337.json
INFO fitness_analyzer::scheduler: ✅ Agent completed successfully
```

**Generated Report Structure:**
```json
{
  "report_metadata": {
    "generated_at": "2024-09-04T14:23:37.123Z",
    "activities_processed": 187,
    "processing_time_seconds": 2.45
  },
  "patterns": [
    {
      "pattern_type": "high_frequency",
      "confidence": 0.9,
      "description": "High training frequency: 5.2 activities per week",
      "supporting_data": {"activities_per_week": 5.2, "total_activities": 187}
    }
  ],
  "risk_indicators": [
    {
      "risk_type": "volume_spike", 
      "severity": "medium",
      "probability": 0.45,
      "description": "Training volume increased by 35% in recent 2 weeks"
    }
  ]
}
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

## Technical Details

**Pattern Detection Algorithms:**

1. **Training Frequency**: `activities.len() / weeks` - categorizes as high (≥6/week), moderate (3-6), or low (<3)
2. **Sport Distribution**: Groups by `sport_type`, calculates percentages, identifies specialization (≥80%) vs variety
3. **Volume Progression**: Linear regression on `duration_seconds` over time, detects increasing/decreasing trends
4. **Risk Assessment**: Compares 2-week volume windows, flags increases >30% as potential overtraining
5. **Weekly Patterns**: Maps activities to weekdays, identifies peak training days

**Data Sources:**
- Activity records from Pierre server via A2A `get_activities` tool
- Fields used: `sport_type`, `duration_seconds`, `distance_meters`, `start_date`
- Handles missing data gracefully (None values)

**Report Generation:**
- JSON files written to configurable directory
- Automatic cleanup (keeps last 10 reports)
- Structured data suitable for downstream processing

## License

This example is part of the Pierre MCP Server project and follows the same dual licensing:
- Apache License, Version 2.0
- MIT License