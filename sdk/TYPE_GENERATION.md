# TypeScript Type Generation for Pierre SDK

This document explains how to generate TypeScript type definitions for all Pierre MCP tools.

## Overview

The Pierre SDK includes an **auto-generation script** that fetches tool schemas from the running Pierre server and generates TypeScript interfaces for:
- Tool parameter types (47 interfaces)
- Tool response types
- Common data structures (Activity, Athlete, Stats, etc.)
- Tool name union types

## Prerequisites

1. **Running Pierre Server**: The server must be running on port 8081 (or custom port via `HTTP_PORT` env var)
2. **Node.js**: Version 18+ required

## Quick Start

### 1. Start Pierre Server

```bash
# In terminal 1
RUST_LOG=warn HTTP_PORT=8081 \
  DATABASE_URL=sqlite:./data/users.db \
  PIERRE_MASTER_ENCRYPTION_KEY=$(openssl rand -base64 32) \
  cargo run --bin pierre-mcp-server
```

Wait for the server to fully start (you'll see "Server listening on..." message).

### 2. Generate Types

```bash
# In terminal 2
cd sdk
bun run generate-types
```

### 3. Verify Output

The script will create `sdk/src/types.ts` with:
- 47 parameter interfaces (`GetActivitiesParams`, `GetAthleteParams`, etc.)
- Common data types (`Activity`, `Athlete`, `Stats`, etc.)
- Type unions and mappings

## Output Example

```typescript
// Generated file: sdk/src/types.ts

/**
 * get_activities tool parameters
 */
export interface GetActivitiesParams {
  /** Start date for activity filter (ISO 8601) */
  start_date?: string;

  /** End date for activity filter (ISO 8601) */
  end_date?: string;

  /** Maximum number of activities to return */
  limit?: number;

  /** Provider name (strava, garmin, fitbit, whoop, terra) */
  provider?: string;
}

/**
 * Fitness activity data structure
 */
export interface Activity {
  id: string;
  name: string;
  type: string;
  distance?: number;
  duration?: number;
  // ... all fields with proper types
}

// Union type of all tool names
export type ToolName = "get_activities" | "get_athlete" | "get_stats" | ...;
```

## Using Generated Types

```typescript
import { GetActivitiesParams, Activity } from './types';

// Type-safe tool call
async function getRecentActivities(limit: number): Promise<Activity[]> {
  const params: GetActivitiesParams = { limit };
  const result = await client.callTool('get_activities', params);
  return result.activities;
}
```

## Advanced Usage

### Custom Server URL/Port

```bash
export PIERRE_SERVER_URL=http://localhost:8081
export HTTP_PORT=8081
bun run generate-types
```

### With Authentication

If the server requires authentication:

```bash
# Generate a JWT token first
cargo run --bin pierre-cli -- token generate --service type_gen --expires-days 1

# Use the token
export PIERRE_JWT_TOKEN="your_jwt_token_here"
bun run generate-types
```

### Regenerate After Server Changes

Whenever you add/modify tools on the server:

```bash
# 1. Rebuild server
cargo build --release

# 2. Restart server
# (kill old process, start new one)

# 3. Regenerate types
cd sdk
bun run generate-types

# 4. Rebuild SDK
bun run build
```

## Troubleshooting

### Error: "Failed to connect to server"

**Problem**: Server is not running or not accessible

**Solution**:
```bash
# Check if server is running
lsof -i :8081

# If not, start it
cargo run --bin pierre-mcp-server
```

### Error: "MCP error: Authentication required"

**Problem**: Server requires JWT authentication

**Solution**:
```bash
# Generate token
cargo run --bin pierre-cli -- token generate --service type_gen

# Export and retry
export PIERRE_JWT_TOKEN="your_token"
bun run generate-types
```

### Error: "Failed to parse response"

**Problem**: Server returned non-JSON response

**Solution**:
- Check server logs for errors
- Ensure server is fully started (not still initializing)
- Verify database is accessible

## CI/CD Integration

To run type generation in CI:

```yaml
# .github/workflows/types.yml
- name: Start server
  run: |
    cargo build --release
    DATABASE_URL=sqlite::memory: \
    PIERRE_MASTER_ENCRYPTION_KEY=$(openssl rand -base64 32) \
    ./target/release/pierre-mcp-server &
    sleep 5

- name: Generate types
  run: cd sdk && bun run generate-types

- name: Verify types compile
  run: cd sdk && bun run build
```

## Manual Generation (Without bun script)

```bash
node ../scripts/sdk/generate-sdk-types.js
```

## Files

- **Generator Script**: `scripts/sdk/generate-sdk-types.js`
- **Output File**: `sdk/src/types.ts`
- **bun Script**: `bun run generate-types` (defined in `sdk/package.json`)

## What Gets Generated

### 1. Tool Parameter Types (47 interfaces)

One interface per tool for type-safe parameter passing:
- `GetActivitiesParams`
- `GetAthleteParams`
- `SetGoalParams`
- ... (46 more)

### 2. Common Data Types

Shared structures used across multiple tools:
- `Activity` - Fitness activity data
- `Athlete` - Athlete profile
- `Stats` - Athlete statistics
- `FitnessConfig` - Configuration profile
- `Goal` - Training goal
- `Zone` - Training zone
- `ConnectionStatus` - Provider connection
- `Notification` - System notification

### 3. Response Wrappers

- `McpToolResponse<T>` - Generic tool response
- `McpErrorResponse` - Error response structure

### 4. Type Utilities

- `ToolName` - Union of all 47 tool names
- `ToolParamsMap` - Map tool names to parameter types

## Benefits

✅ **IntelliSense**: Auto-completion in IDEs
✅ **Type Safety**: Catch errors at compile time
✅ **Documentation**: Types serve as inline docs
✅ **Refactoring**: Easier to update when APIs change
✅ **Always Fresh**: Generate from live server schemas

## Next Steps

After generating types:

1. **Update `bridge.ts`** to use generic types
2. **Export from `index.ts`** for external use
3. **Write tests** using typed interfaces
4. **Update documentation** with typed examples
5. **Bump version** to 0.2.0 (adds types = minor bump)

---

**Generated**: This doc explains the type generation system
**Maintainer**: Pierre SDK Team
**Last Updated**: 2025-11-28
