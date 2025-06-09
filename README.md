# Pierre MCP Server

An MCP (Model Context Protocol) server that provides access to fitness data from multiple providers including Strava, Garmin Connect, and Runkeeper.

## Features

- Unified API for multiple fitness tracking services
- OAuth2 and API key authentication support
- Query activities, athlete profiles, and statistics
- Extensible provider architecture

## Installation

```bash
cargo build --release
```

## OAuth2 Setup

### Strava

1. Create a Strava application at https://www.strava.com/settings/api
2. Note your Client ID and Client Secret
3. Run the auth setup tool:

```bash
cargo run --bin auth-setup -- strava \
  --client-id YOUR_CLIENT_ID \
  --client-secret YOUR_CLIENT_SECRET
```

4. Follow the browser prompts to authorize the application
5. The tool will save your tokens to the config file

## Configuration

The server supports multiple configuration methods:

### Using direnv (.envrc):
```bash
# Copy the example file
cp .envrc.example .envrc

# Edit with your credentials
vim .envrc

# Allow direnv to load the file
direnv allow
```

### Using .env file:
```env
STRAVA_CLIENT_ID=your_client_id
STRAVA_CLIENT_SECRET=your_client_secret
STRAVA_ACCESS_TOKEN=your_access_token
STRAVA_REFRESH_TOKEN=your_refresh_token
```

### Using config.toml:
```toml
[providers.strava]
auth_type = "oauth2"
client_id = "your_client_id"
client_secret = "your_client_secret"
access_token = "your_access_token"
refresh_token = "your_refresh_token"
```

## Usage

```bash
# Run with default settings
cargo run

# Run with custom port
cargo run -- --port 9000

# Run with custom config file
cargo run -- --config /path/to/config.toml
```

## MCP Tools

The server exposes the following tools:

- `get_activities`: Fetch fitness activities from a provider
- `get_athlete`: Get athlete profile information
- `get_stats`: Get aggregated statistics

## Adding to Claude or GitHub Copilot

Add to your MCP configuration:

```json
{
  "mcpServers": {
    "fitness": {
      "command": "path/to/pierre-mcp-server",
      "args": ["--port", "8080"]
    }
  }
}
```