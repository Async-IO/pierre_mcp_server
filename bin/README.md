# bin/ - Runtime Scripts

Day-to-day scripts for running Pierre development environment.

## Prerequisites

All scripts require `.envrc` to be configured. Copy from example and edit:

```bash
cp .envrc.example .envrc
# Edit .envrc with your settings, then:
direnv allow  # or: source .envrc
```

### Required Environment Variables

Scripts will fail fast if these are missing:

| Variable | Description | Category |
|----------|-------------|----------|
| `DATABASE_URL` | Database connection string | **CRITICAL** |
| `PIERRE_MASTER_ENCRYPTION_KEY` | Master encryption key (base64) | **CRITICAL** |

Generate the encryption key with: `openssl rand -base64 32`

### Provider OAuth Variables (based on `PIERRE_DEFAULT_PROVIDER`)

| Provider | Required Variables |
|----------|-------------------|
| `strava` | `PIERRE_STRAVA_CLIENT_ID`, `PIERRE_STRAVA_CLIENT_SECRET` |
| `garmin` | `PIERRE_GARMIN_CLIENT_ID`, `PIERRE_GARMIN_CLIENT_SECRET` |
| `synthetic` | None (works out of the box) |

See `book/src/environment.md` for the complete variable reference.

## Quick Start

```bash
# Full setup: reset DB, seed all data, start all 3 servers
./bin/setup-db-with-seeds-and-oauth-and-start-servers.sh
```

## Available Scripts

| Script | Description |
|--------|-------------|
| `setup-db-with-seeds-and-oauth-and-start-servers.sh` | **THE ONE SCRIPT** - Complete dev environment setup |
| `start-server.sh` | Start Pierre MCP server only (port 8081) |
| `stop-server.sh` | Stop Pierre MCP server |
| `start-frontend.sh` | Start web frontend only (port 3000) |
| `start-tunnel.sh` | Start Cloudflare tunnel for mobile testing |

## What `setup-db-with-seeds-and-oauth-and-start-servers.sh` Does

1. Stops any running services
2. Resets database (backs up existing, runs fresh migrations)
3. Seeds all data:
   - Admin user (from `.envrc`: `ADMIN_EMAIL`, `ADMIN_PASSWORD`)
   - 9 AI coaching personas
   - Demo users (Alice, Bob, etc.)
   - Visual test users (webtest, mobiletest)
   - Mobility data (stretches, yoga poses)
4. Starts Pierre MCP Server (port 8081)
5. Starts Web Frontend (port 3000)
6. Starts Expo Mobile (port 8082)
7. Generates admin API token
8. Displays summary with credentials, URLs, and log paths

## Log Files

After running the setup script, logs are available at:

```bash
tail -f logs/pierre-server.log  # Pierre MCP Server
tail -f logs/frontend.log       # Web Frontend
tail -f logs/expo.log           # Expo Mobile
tail -f logs/*.log              # All logs
```

## Stopping Services

```bash
pkill -f pierre-mcp-server; pkill -f vite; pkill -f expo
```

## See Also

- `scripts/` - CI/Dev tools (validation, testing, release)
- `scripts/setup/setup-claude-code-mcp.sh` - Claude Code session JWT setup
