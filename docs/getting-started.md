# getting started

## prerequisites

- rust 1.91+ (matches `rust-toolchain`)
- sqlite3 (or postgresql for production)
- node 24+ (for sdk)

## installation

```bash
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
cargo build --release
```

Binary: `target/release/pierre-mcp-server`

## configuration

### using direnv (recommended)

```bash
brew install direnv
cd pierre_mcp_server
direnv allow
```

Edit `.envrc` for your environment. Development defaults included.

### manual setup

Required:
```bash
export DATABASE_URL="sqlite:./data/users.db"
export PIERRE_MASTER_ENCRYPTION_KEY="$(openssl rand -base64 32)"
```

Optional provider oauth (connect to strava/garmin/fitbit):
```bash
# local development only
export STRAVA_CLIENT_ID=your_id
export STRAVA_CLIENT_SECRET=your_secret
export STRAVA_REDIRECT_URI=http://localhost:8081/api/oauth/callback/strava  # local dev

export GARMIN_CLIENT_ID=your_key
export GARMIN_CLIENT_SECRET=your_secret
export GARMIN_REDIRECT_URI=http://localhost:8081/api/oauth/callback/garmin  # local dev

# production: use https for callback urls (required)
# export STRAVA_REDIRECT_URI=https://api.example.com/api/oauth/callback/strava
# export GARMIN_REDIRECT_URI=https://api.example.com/api/oauth/callback/garmin
```

**security**: http callback urls only for local development. Production must use https to protect authorization codes.

See `src/constants/mod.rs` for all environment variables.

## running the server

```bash
cargo run --bin pierre-mcp-server
```

Server starts on `http://localhost:8081`

Logs show available endpoints:
- `/health` - health check
- `/mcp` - mcp protocol endpoint
- `/oauth2/*` - oauth2 authorization server
- `/api/*` - rest api
- `/admin/*` - admin endpoints

## create admin user

```bash
curl -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@example.com",
    "password": "SecurePass123!",
    "display_name": "Admin"
  }'
```

Response includes jwt token. Save it.

## connect mcp client

### option 1: npm package (recommended)

```bash
npm install -g pierre-mcp-client@next
```

Claude desktop config (`~/Library/Application Support/Claude/claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "pierre": {
      "command": "npx",
      "args": ["-y", "pierre-mcp-client@next", "--server", "http://localhost:8081"]
    }
  }
}
```

### option 2: build from source

```bash
cd sdk
npm install
npm run build
```

Claude desktop config:
```json
{
  "mcpServers": {
    "pierre": {
      "command": "node",
      "args": ["/absolute/path/to/sdk/dist/cli.js", "--server", "http://localhost:8081"]
    }
  }
}
```

Restart claude desktop.

## authentication flow

Sdk handles oauth2 automatically:
1. Registers oauth2 client with Pierre Fitness Intelligence (rfc 7591)
2. Opens browser for login
3. Handles callback and token exchange
4. Stores jwt token
5. Uses jwt for all mcp requests

No manual token management needed.

## verify connection

In claude desktop, ask:
- "connect to strava" - initiates oauth flow
- "get my last 5 activities" - fetches strava data
- "analyze my training load" - runs intelligence engine

## available tools

Pierre Fitness Intelligence exposes dozens of MCP tools:

**fitness data:**
- `get_activities` - fetch activities
- `get_athlete` - athlete profile
- `get_stats` - athlete statistics
- `analyze_activity` - detailed activity analysis

**goals:**
- `set_goal` - create fitness goal
- `suggest_goals` - ai-suggested goals
- `track_progress` - goal progress tracking
- `analyze_goal_feasibility` - feasibility analysis

**performance:**
- `calculate_metrics` - custom metrics
- `analyze_performance_trends` - trend detection
- `compare_activities` - activity comparison
- `detect_patterns` - pattern recognition
- `generate_recommendations` - training recommendations
- `analyze_training_load` - load analysis

**configuration:**
- `get_user_configuration` - current config
- `update_user_configuration` - update config
- `calculate_personalized_zones` - training zones

See `src/protocols/universal/tool_registry.rs` for complete tool definitions.

## development workflow

```bash
# clean start
./scripts/fresh-start.sh
cargo run --bin pierre-mcp-server &

# run complete workflow test
./scripts/complete-user-workflow.sh

# load saved credentials
source .workflow_test_env
echo $JWT_TOKEN
```

## testing

```bash
# all tests
cargo test

# specific suite
cargo test --test mcp_multitenant_complete_test

# with output
cargo test -- --nocapture

# lint + test
./scripts/lint-and-test.sh
```

## troubleshooting

### server won't start

Check logs for:
- database connection errors → verify `DATABASE_URL`
- encryption key errors → verify `PIERRE_MASTER_ENCRYPTION_KEY`
- port conflicts → check port 8081 availability

### sdk connection fails

1. Verify server is running: `curl http://localhost:8081/health`
2. Check claude desktop logs: `~/Library/Logs/Claude/mcp*.log`
3. Test sdk directly: `npx pierre-mcp-client@next --server http://localhost:8081`

### oauth2 flow fails

- verify redirect uri matches: server must be accessible at configured uri
- check browser console for errors
- verify provider credentials (strava_client_id, etc.)

## next steps

- [architecture.md](architecture.md) - system design
- [protocols.md](protocols.md) - protocol details
- [authentication.md](authentication.md) - auth guide
- [configuration.md](configuration.md) - configuration reference
