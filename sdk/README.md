# Pierre MCP Client

MCP client SDK for connecting to Pierre Fitness MCP Server. Works with Claude Desktop, ChatGPT, and any MCP-compatible application.

## Installation

```bash
npm install pierre-mcp-client@next
```

## Usage

### With npx (No Installation)

```bash
npx -y pierre-mcp-client@next --server http://localhost:8081
```

### MCP Client Configuration

Add to your MCP client configuration file:

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "npx",
      "args": [
        "-y",
        "pierre-mcp-client@next",
        "--server",
        "http://localhost:8081"
      ]
    }
  }
}
```

**Configuration File Locations:**
- **Claude Desktop**: `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS)
- **ChatGPT**: `~/Library/Application Support/ChatGPT/config.json` (macOS)
- See [full installation guide](https://github.com/Async-IO/pierre_mcp_server/blob/main/book/src/installation-guides/install-mcp-client.md) for all platforms

## What It Does

The Pierre MCP Client automatically:
- Registers with Pierre MCP Server using OAuth 2.0
- Opens your browser for authentication
- Manages tokens and token refresh
- Provides stdio transport for MCP clients

No manual token management required!

## Available Tools

Once connected, your AI assistant can access 47 fitness tools including:
- Activity retrieval and analysis
- Goal setting and progress tracking
- Performance trend analysis
- Training recommendations
- Sleep and recovery analysis
- Nutrition calculations
- And more...

Ask your AI assistant: *"What fitness tools do you have access to?"*

See [Tools Reference](../book/src/tools-reference.md) for complete documentation.

## Requirements

- **Node.js**: 24.0.0 or higher
- **Pierre MCP Server**: Running on port 8081 (or custom port)

## Configuration Options

```bash
pierre-mcp-client --server <url> [options]
```

**Options:**
| Option | Description |
|--------|-------------|
| `-s, --server <url>` | Pierre MCP Server URL (required) |
| `-t, --token <jwt>` | Pre-authenticated JWT token |
| `--oauth-client-id <id>` | OAuth client ID for authentication |
| `--oauth-client-secret <secret>` | OAuth client secret |
| `--user-email <email>` | User email for password authentication |
| `--user-password <password>` | User password |
| `--callback-port <port>` | OAuth callback server port (default: 9876) |
| `--no-browser` | Disable automatic browser opening |
| `--token-validation-timeout <ms>` | Token validation timeout |
| `--proactive-connection-timeout <ms>` | Initial connection timeout |
| `--tool-call-connection-timeout <ms>` | Tool call timeout |

## Type System

The SDK provides comprehensive TypeScript type definitions auto-generated from the server's Rust tool registry, ensuring type safety between server and client.

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  Rust (Server)                                                  │
│  src/protocols/universal/tool_registry.rs                       │
│  ToolId enum + JSON schemas                                     │
└─────────────────────┬───────────────────────────────────────────┘
                      │ tools/list JSON-RPC
                      ▼
┌─────────────────────────────────────────────────────────────────┐
│  scripts/sdk/generate-sdk-types.js                               │
│  Fetches schemas, converts to TypeScript                        │
└─────────────────────┬───────────────────────────────────────────┘
                      │ generates
                      ▼
┌─────────────────────────────────────────────────────────────────┐
│  TypeScript (SDK)                                               │
│  sdk/src/types.ts                                               │
│  47 tool parameter interfaces                                   │
└─────────────────────────────────────────────────────────────────┘
```

### Generated Types

Each MCP tool has a corresponding TypeScript interface:

```typescript
// Tool: get_activities
export interface GetActivitiesParams {
  provider: string;      // required
  limit?: number;        // optional
  offset?: number;       // optional
}

// Tool: analyze_training_load
export interface AnalyzeTrainingLoadParams {
  provider: string;
  days?: number;
  include_predictions?: boolean;
}

// Tool: calculate_daily_nutrition
export interface CalculateDailyNutritionParams {
  weight_kg: number;
  height_cm: number;
  age: number;
  gender: "male" | "female";
  activity_level: "sedentary" | "light" | "moderate" | "active" | "very_active";
  goal: "maintenance" | "weight_loss" | "muscle_gain" | "endurance";
}
```

### Type Categories

| Category | Interfaces | Description |
|----------|------------|-------------|
| Core Fitness | 7 | Activities, athlete, stats, connections |
| Goals | 4 | Goal setting, progress, feasibility |
| Analysis | 10 | Performance, trends, patterns, predictions |
| Sleep & Recovery | 5 | Sleep quality, recovery scores |
| Nutrition | 5 | BMR, TDEE, macros, food search |
| Configuration | 10 | User settings, zones, profiles |
| OAuth | 4 | Notifications, connection status |

### Benefits

- **Compile-time Safety**: TypeScript catches parameter errors before runtime
- **IDE Support**: Auto-completion and inline documentation
- **Schema Sync**: Types always match server expectations
- **Self-documenting**: JSDoc comments from tool descriptions

### Usage Example

```typescript
import { GetActivitiesParams, AnalyzeTrainingLoadParams } from 'pierre-mcp-client';

// Type-safe parameter construction
const activityParams: GetActivitiesParams = {
  provider: 'strava',
  limit: 10
};

const loadParams: AnalyzeTrainingLoadParams = {
  provider: 'strava',
  days: 30,
  include_predictions: true
};
```

## Development

### Type Generation

TypeScript type definitions in `src/types.ts` are **auto-generated** from server tool schemas. Do not edit this file manually.

**Regenerate types after**:
- Adding new MCP tools to `src/protocols/universal/tool_registry.rs`
- Modifying tool parameters or schemas
- Changing tool descriptions

**Prerequisites**:
1. Pierre MCP Server must be running on port 8081 (or HTTP_PORT)
2. Server must be accessible at `http://localhost:8081`
3. Optional: Set `PIERRE_JWT_TOKEN` environment variable if authentication enabled

**Command**:
```bash
# Start server (in project root)
cargo run --bin pierre-mcp-server

# Generate types (in sdk/ directory)
cd sdk
bun run generate-types
```

**What Happens**:
1. Script connects to `http://localhost:8081/mcp`
2. Sends `tools/list` JSON-RPC request
3. Converts JSON schemas to TypeScript interfaces
4. Writes to `sdk/src/types.ts`
5. Generates 47 tool parameter interfaces

**Output**: `src/types.ts` (~500 lines with full type definitions)

**Troubleshooting**:
- **Server connection failed**: Ensure server is running and accessible
- **Authentication error**: Set `PIERRE_JWT_TOKEN` environment variable
- **Port conflict**: Change server port via `HTTP_PORT` environment variable

**Version Sync**: Always regenerate types after pulling changes to tool definitions to ensure SDK types match server schemas.

### Building from Source

```bash
cd sdk
bun install
bun run build
```

## Example

```bash
# Start Pierre MCP Server
cargo run --bin pierre-mcp-server

# In another terminal, test the client
npx -y pierre-mcp-client@next --server http://localhost:8081 --verbose
```

## Troubleshooting

### Authentication Issues

If the browser doesn't open for authentication, check:
```bash
# Verify server is running
curl http://localhost:8081/health
```

### Token Storage

Tokens are stored securely using OS-native credential storage:
- **macOS**: Keychain Access
- **Windows**: Windows Credential Manager
- **Linux**: Secret Service (libsecret)

Encrypted fallback storage: `~/.pierre-mcp-tokens.enc`

To force re-authentication:
```bash
# macOS: Remove from Keychain via Keychain Access app
# Or remove encrypted fallback:
rm ~/.pierre-mcp-tokens.enc
```

## Documentation

- [Tools Reference](../book/src/tools-reference.md)
- [Installation Guide](https://github.com/Async-IO/pierre_mcp_server/blob/main/book/src/installation-guides/install-mcp-client.md)
- [Server Documentation](https://github.com/Async-IO/pierre_mcp_server)

## Support

- **GitHub Issues**: https://github.com/Async-IO/pierre_mcp_server/issues
- **Discussions**: https://github.com/Async-IO/pierre_mcp_server/discussions

## License

MIT
