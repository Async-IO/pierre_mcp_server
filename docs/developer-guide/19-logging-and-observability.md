# Logging and Observability

Pierre MCP Server includes a comprehensive logging and observability system designed for both development and production environments. The system provides tenant-aware logging, request correlation, and structured output for easy debugging and monitoring.

## Overview

The logging system is built on Rust's `tracing` ecosystem and provides:

- **Tenant-aware logging**: All logs include tenant and user context for precise filtering
- **Request correlation**: Unique request IDs flow through all operations
- **MCP protocol tracing**: Full request/response logging for MCP operations
- **Smart filtering**: Infrastructure noise reduction while preserving application logs
- **Dual environment support**: Pretty format for development, JSON for production
- **Performance tracking**: Duration and success metrics for all major operations

## Configuration

### Environment Variables

```bash
# Log level (trace, debug, info, warn, error)
RUST_LOG=debug

# Log format (json, pretty, compact)
LOG_FORMAT=pretty

# Environment (development, staging, production)
ENVIRONMENT=development

# Service information
SERVICE_NAME=pierre-mcp-server
SERVICE_VERSION=0.1.0

# Request ID header name
REQUEST_ID_HEADER=x-request-id

# Enable features
LOG_INCLUDE_LOCATION=1      # Include file/line numbers
LOG_INCLUDE_THREAD=1        # Include thread information
LOG_INCLUDE_SPANS=1         # Include tracing spans
ENABLE_TELEMETRY=1          # Enable OpenTelemetry (when available)

# GCP Cloud Logging (production)
GCP_PROJECT_ID=your-project # Enables GCP-optimized format
```

### Default Behavior

The logging system automatically:
- Uses **pretty format** in development
- Uses **JSON format** in production (when `ENVIRONMENT=production`)
- Filters noisy dependencies (hyper, sqlx, reqwest) to reduce log volume
- Includes detailed context in production environments
- Enables GCP-optimized logging when `GCP_PROJECT_ID` is set

## Log Levels and Filtering

### Application vs Infrastructure Logs

The system intelligently filters logs to reduce noise:

```bash
# Infrastructure logs (filtered to reduce noise)
hyper=warn              # HTTP client/server internals
hyper::proto=warn       # HTTP protocol details
sqlx=info              # Database query logs
sqlx::query=info       # Individual query logs
reqwest=warn           # HTTP request client
warp::server=info      # Web server logs
tower_http=info        # HTTP middleware

# Application logs (full detail)
pierre_mcp_server=debug # Your application logs at specified level
```

### Runtime Log Control

```bash
# Default: Smart filtering with application at debug level
RUST_LOG=debug

# Override specific components
RUST_LOG="pierre_mcp_server=debug,hyper=error,sqlx=warn"

# Maximum detail (noisy but comprehensive)
RUST_LOG=trace

# Production: Info level with error details
RUST_LOG=info
```

## Structured Logging

### Log Entry Format

All logs include structured fields for easy parsing and filtering:

```bash
# Pretty format (development)
2025-09-15T18:00:32.562114Z  INFO pierre_mcp_server::mcp::multitenant: MCP tool call completed
  user_id=a956df56-a8a2-4a9b-83fb-e6e081c5bf3b
  tenant_id=5e561660-fdc6-40ae-901b-684613ccf1ad
  tool_name=get_activities
  success=true
  duration_ms=1250

# JSON format (production)
{"timestamp":"2025-09-15T18:00:32.562114Z","level":"INFO","target":"pierre_mcp_server::mcp::multitenant","fields":{"user_id":"a956df56-a8a2-4a9b-83fb-e6e081c5bf3b","tenant_id":"5e561660-fdc6-40ae-901b-684613ccf1ad","tool_name":"get_activities","success":true,"duration_ms":1250},"span":{"name":"mcp_tool_call"}}
```

### Key Log Fields

| Field | Description | Example |
|-------|-------------|---------|
| `user_id` | User identifier | `a956df56-a8a2-4a9b-83fb-e6e081c5bf3b` |
| `tenant_id` | Tenant identifier | `5e561660-fdc6-40ae-901b-684613ccf1ad` |
| `request_id` | Unique request identifier | `req_abc123def456` |
| `mcp_method` | MCP method being called | `tools/call`, `initialize` |
| `mcp_id` | MCP request ID | `123` |
| `tool_name` | Specific tool being executed | `get_activities`, `get_stats` |
| `duration_ms` | Operation duration | `1250` |
| `success` | Operation success status | `true`, `false` |
| `auth_method` | Authentication method used | `jwt`, `api_key` |
| `transport` | Communication transport | `stdio`, `http` |

## MCP Request/Response Logging

### Debug Level MCP Tracing

When `RUST_LOG=debug`, the system logs complete MCP request/response cycles:

```bash
# Transport-level logging
DEBUG pierre_mcp_server::mcp::multitenant: Received MCP request via stdio transport
  transport=stdio mcp_method=tools/list line_length=45

DEBUG pierre_mcp_server::mcp::multitenant: Received MCP request via HTTP transport
  transport=http origin=Some("http://localhost:3000") mcp_method=initialize body_size=128

# Full request/response bodies
DEBUG pierre_mcp_server::mcp::multitenant: Received MCP request
  mcp_request=McpRequest {
    method: "tools/call",
    params: {"name": "get_activities", "arguments": {"limit": 10}},
    id: Some(123)
  }

DEBUG pierre_mcp_server::mcp::multitenant: Sending MCP response
  mcp_response=McpResponse {
    result: Some({"activities": [...]}),
    error: None,
    id: Some(123)
  }
  duration_ms=45
```

### MCP Operation Tracking

Each MCP operation is instrumented with structured spans:

```bash
INFO pierre_mcp_server::mcp::tool_handlers: MCP tool call completed
  user_id=a956df56-a8a2-4a9b-83fb-e6e081c5bf3b
  tenant_id=5e561660-fdc6-40ae-901b-684613ccf1ad
  tool_name=get_activities
  success=true
  duration_ms=1250
  event_type=mcp_tool_call
```

## Tenant-Aware Logging

### Automatic Context Injection

The system automatically injects tenant context into all relevant operations:

```bash
# Authentication events
INFO pierre_mcp_server::auth: Authentication successful
  user_id=a956df56-a8a2-4a9b-83fb-e6e081c5bf3b
  tenant_id=5e561660-fdc6-40ae-901b-684613ccf1ad
  auth_method=jwt
  success=true

# Database operations
DEBUG pierre_mcp_server::database: Database operation completed
  user_id=a956df56-a8a2-4a9b-83fb-e6e081c5bf3b
  tenant_id=5e561660-fdc6-40ae-901b-684613ccf1ad
  db_operation=select
  db_table=user_oauth_tokens
  success=true
  duration_ms=125
  rows_affected=1

# Provider API calls
DEBUG pierre_mcp_server::providers: Provider API call completed
  user_id=a956df56-a8a2-4a9b-83fb-e6e081c5bf3b
  tenant_id=5e561660-fdc6-40ae-901b-684613ccf1ad
  provider=strava
  api_endpoint=/api/v3/activities
  api_method=GET
  success=true
  duration_ms=850
  status_code=200
```

## Request Correlation

### Request ID Generation

Every HTTP request receives a unique request ID that flows through all operations:

```bash
# Request ID in headers or auto-generated
X-Request-ID: req_abc123def456

# All related logs include the request ID
INFO pierre_mcp_server::middleware: HTTP request completed
  request_id=req_abc123def456
  http_method=POST
  http_path=/mcp
  user_id=a956df56-a8a2-4a9b-83fb-e6e081c5bf3b
  tenant_id=5e561660-fdc6-40ae-901b-684613ccf1ad
  duration_ms=1250
  status_code=200
```

## Filtering and Analysis

### Development Filtering

```bash
# Filter by tenant ID
RUST_LOG=debug cargo run | grep "tenant_id=5e561660-fdc6-40ae-901b-684613ccf1ad"

# Filter by request ID for full trace
RUST_LOG=debug cargo run | grep "request_id=req_abc123def456"

# Monitor MCP tool calls
RUST_LOG=debug cargo run | grep "mcp_tool_call"

# See full MCP request/response flow
RUST_LOG=debug cargo run | grep "mcp_request\|mcp_response"

# Monitor authentication events
RUST_LOG=debug cargo run | grep "authentication"

# Track database performance
RUST_LOG=debug cargo run | grep "db_operation" | grep "duration_ms"
```

### Production Analysis

```bash
# Count requests by tenant (JSON logs)
cat logs.json | jq -r '.fields.tenant_id' | sort | uniq -c

# Average response times
cat logs.json | jq '.fields.duration_ms' | grep -v null | awk '{sum+=$1; count++} END {print sum/count}'

# Error rate analysis
cat logs.json | jq -r 'select(.fields.success == false) | .fields.tool_name' | sort | uniq -c

# Top active tenants
cat logs.json | jq -r '.fields.tenant_id' | grep -v null | sort | uniq -c | sort -nr | head -10
```

### Pipeline Combinations

```bash
# Monitor real-time MCP traffic
RUST_LOG=debug cargo run | grep "Received MCP request" | while read line; do
  echo "$(date): $line"
done

# Count MCP operations by type
RUST_LOG=debug cargo run | grep "mcp_method" | awk -F'mcp_method=' '{print $2}' | awk '{print $1}' | sort | uniq -c

# Monitor slow operations
RUST_LOG=debug cargo run | grep "duration_ms" | awk -F'duration_ms=' '{if($2 > 1000) print $0}'
```

## Security and Sensitive Data

### Safe Logging Practices

The logging system is designed to avoid logging sensitive data:

- **Passwords**: Never logged
- **API tokens**: Only prefixes logged (first 10 characters)
- **OAuth secrets**: Never logged in plaintext
- **Personal data**: Limited to user/tenant IDs
- **Request bodies**: Only logged in debug mode, filtered for sensitive fields

### Example Safe Logging

```bash
# Safe: Token prefixes only
INFO pierre_mcp_server::auth: JWT token validation successful
  token_prefix=eyJ0eXAiOi...
  user_id=a956df56-a8a2-4a9b-83fb-e6e081c5bf3b

# Safe: Redacted sensitive fields
DEBUG pierre_mcp_server::oauth: OAuth token exchange
  provider=strava
  client_id=163846
  redirect_uri=http://localhost:8081/auth/strava/callback
  # client_secret and tokens are never logged
```

## Performance Monitoring

### Built-in Metrics

The logging system automatically tracks performance metrics:

```bash
# Operation timing
INFO pierre_mcp_server::mcp::tool_handlers: MCP tool call completed
  tool_name=get_activities
  duration_ms=1250
  success=true

# Database operation performance
DEBUG pierre_mcp_server::database: Database operation completed
  db_operation=select
  db_table=activities
  duration_ms=125
  rows_affected=15

# Provider API performance
DEBUG pierre_mcp_server::providers: Provider API call completed
  provider=strava
  api_endpoint=/api/v3/activities
  duration_ms=850
  status_code=200
```

### Performance Analysis Queries

```bash
# Find slow operations
grep "duration_ms" logs.txt | awk -F'duration_ms=' '{if($2 > 2000) print $0}'

# Database query performance
grep "db_operation" logs.txt | awk -F'duration_ms=' '{sum+=$2; count++} END {print "Avg DB time:", sum/count "ms"}'

# Provider API latency by provider
grep "provider=" logs.txt | grep "duration_ms" |
  awk -F'provider=' '{split($2, a, " "); provider=a[1]}
       /duration_ms=/ {split($0, b, "duration_ms="); duration=b[2]+0;
       providers[provider]+=duration; counts[provider]++}
       END {for(p in providers) print p":", providers[p]/counts[p]"ms"}'
```

## Production Deployment

### Docker Logging

```yaml
# docker-compose.yml
services:
  pierre-mcp-server:
    environment:
      - RUST_LOG=info
      - LOG_FORMAT=json
      - ENVIRONMENT=production
      - SERVICE_NAME=pierre-mcp-server
      - SERVICE_VERSION=0.1.0
    logging:
      driver: "json-file"
      options:
        max-size: "100m"
        max-file: "5"
```

### GCP Cloud Logging

```bash
# Environment setup
export GCP_PROJECT_ID=your-project-id
export ENVIRONMENT=production
export LOG_FORMAT=json

# The system automatically optimizes for GCP Cloud Logging
cargo run --bin pierre-mcp-server
```

### Log Aggregation

The structured JSON logs integrate seamlessly with log aggregation systems:

- **ELK Stack**: Direct JSON ingestion
- **Prometheus**: Metrics extraction from structured logs
- **Grafana**: Dashboard visualization
- **GCP Cloud Logging**: Native integration
- **AWS CloudWatch**: JSON log parsing
- **Datadog**: Structured log analysis

## Troubleshooting

### Common Issues

**Too much log noise:**
```bash
# Reduce infrastructure logging
RUST_LOG="pierre_mcp_server=debug,hyper=error,sqlx=warn"
```

**Missing tenant context:**
```bash
# Check authentication
grep "Authentication" logs.txt | grep "failed"

# Verify tenant assignment
grep "tenant_id=null" logs.txt
```

**Performance issues:**
```bash
# Find slow operations
grep "duration_ms" logs.txt | sort -k2 -nr | head -20

# Check database connection pool
grep "database" logs.txt | grep "error"
```

**MCP request failures:**
```bash
# Debug MCP protocol issues
RUST_LOG=debug cargo run | grep "mcp_" | grep "error"

# Check authentication
grep "mcp_request" logs.txt | grep "auth_present=false"
```

### Debug Mode Quick Reference

```bash
# Full system tracing
RUST_LOG=trace cargo run

# Application only with infrastructure quiet
RUST_LOG="pierre_mcp_server=debug,hyper=warn,sqlx=info" cargo run

# Focus on MCP operations
RUST_LOG=debug cargo run | grep "mcp_"

# Monitor specific tenant
RUST_LOG=debug cargo run | grep "tenant_id=YOUR_TENANT_ID"

# Track request end-to-end
RUST_LOG=debug cargo run | grep "request_id=YOUR_REQUEST_ID"
```

## Integration Examples

### Log Analysis Scripts

```bash
#!/bin/bash
# analyze-logs.sh - Extract key metrics from logs

# Most active users
echo "Top 10 Active Users:"
grep "user_id=" logs.txt |
  awk -F'user_id=' '{print $2}' |
  awk '{print $1}' |
  sort | uniq -c | sort -nr | head -10

# Error summary
echo "Error Summary:"
grep "success=false" logs.txt |
  awk -F'tool_name=' '{print $2}' |
  awk '{print $1}' |
  sort | uniq -c | sort -nr

# Performance summary
echo "Average Response Times:"
grep "duration_ms=" logs.txt |
  awk -F'duration_ms=' '{sum+=$2; count++} END {print "Average:", sum/count "ms"}'
```

### Monitoring Alerts

```bash
# High error rate alert
if [ $(grep "success=false" logs.txt | wc -l) -gt 100 ]; then
  echo "ALERT: High error rate detected"
fi

# Slow response alert
if [ $(grep "duration_ms=" logs.txt | awk -F'duration_ms=' '$2 > 5000' | wc -l) -gt 10 ]; then
  echo "ALERT: Slow responses detected"
fi
```

The logging system provides comprehensive observability for Pierre MCP Server, enabling effective debugging, monitoring, and performance analysis in both development and production environments.