---
title: "Development"
---


# Development

Development workflow, tools, and dashboard setup for Pierre Fitness Platform.

## Server Management

### Startup Scripts

```bash
./bin/start-server.sh     # start backend (loads .envrc, port 8081)
./bin/stop-server.sh      # stop backend (graceful shutdown)
./bin/start-frontend.sh   # start dashboard (port 5173)
```

### Manual Startup

```bash
# backend
cargo run --bin pierre-mcp-server

# frontend (separate terminal)
cd frontend && npm run dev
```

## Development Workflow

### Fresh Start

```bash
# clean database and start fresh
./scripts/fresh-start.sh
./bin/start-server.sh &

# run complete setup (admin + user + tenant + MCP test)
./scripts/complete-user-workflow.sh

# load saved credentials
source .workflow_test_env
echo "JWT Token: ${JWT_TOKEN:0:50}..."
```

### Automated Setup Script

`./scripts/complete-user-workflow.sh` creates:
- Admin user: `admin@pierre.mcp`
- Regular user: `user@example.com`
- Default tenant: `User Organization`
- JWT token (saved in `.workflow_test_env`)

## Management Dashboard

React + Vite web dashboard for monitoring and administration.

### Quick Start

```bash
# terminal 1: backend
./bin/start-server.sh

# terminal 2: frontend
./bin/start-frontend.sh
```

Access at `http://localhost:5173`

### Features

- **User Management**: Registration approval, tenant management
- **API Key Monitoring**: Usage statistics, rate limits
- **Usage Analytics**: Request patterns, tool usage breakdown
- **Real-time Updates**: WebSocket-based live data
- **OAuth Status**: Provider connection monitoring

### Manual Setup

```bash
cd frontend
npm install
npm run dev
```

### Environment

Add to `.envrc` for custom backend URL:
```bash
export VITE_BACKEND_URL="http://localhost:8081"
```

See [frontend/README.md](../frontend/README.md) for detailed frontend documentation.

## Admin Tools

### admin-setup Binary

Manage admin users and API tokens:

```bash
# create admin user for frontend login
cargo run --bin admin-setup -- create-admin-user \
  --email admin@example.com \
  --password SecurePassword123

# generate API token for a service
cargo run --bin admin-setup -- generate-token \
  --service my_service \
  --expires-days 30

# generate super admin token (no expiry, all permissions)
cargo run --bin admin-setup -- generate-token \
  --service admin_console \
  --super-admin

# list all admin tokens
cargo run --bin admin-setup -- list-tokens --detailed

# revoke a token
cargo run --bin admin-setup -- revoke-token <token_id>
```

### curl-based Setup

```bash
# create admin (first run only)
curl -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@example.com",
    "password": "SecurePass123!",
    "display_name": "Admin"
  }'

# register user (requires admin token)
curl -X POST http://localhost:8081/api/auth/register \
  -H "Authorization: Bearer {admin_token}" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "userpass123",
    "display_name": "User"
  }'

# approve user (requires admin token)
curl -X POST http://localhost:8081/admin/approve-user/{user_id} \
  -H "Authorization: Bearer {admin_token}" \
  -H "Content-Type: application/json" \
  -d '{
    "reason": "Approved",
    "create_default_tenant": true,
    "tenant_name": "User Org",
    "tenant_slug": "user-org"
  }'
```

## Testing

### Quick Validation

```bash
./scripts/smoke-test.sh           # ~3 minutes
./scripts/fast-tests.sh           # ~5 minutes
./scripts/pre-push-tests.sh       # ~10 minutes
```

### Full Test Suite

```bash
cargo test                        # all tests (~13 min)
./scripts/lint-and-test.sh        # full CI suite
```

### Targeted Testing

```bash
cargo test test_training_load     # by test name
cargo test --test intelligence_test  # by test file
cargo test intelligence::         # by module path
cargo test <pattern> -- --nocapture  # with output
```

See [testing.md](testing.md) for comprehensive testing documentation.

## Validation

### Pre-commit Checklist

```bash
cargo fmt                              # format code
./scripts/architectural-validation.sh  # architectural patterns
cargo clippy -- -D warnings            # linting
cargo test <relevant_tests>            # targeted tests
```

### CI Validation

```bash
./scripts/lint-and-test.sh        # runs everything CI runs
```

## Scripts Reference

30+ scripts in `scripts/` directory:

| Category | Scripts |
|----------|---------|
| **Development** | `dev-start.sh`, `fresh-start.sh` |
| **Testing** | `smoke-test.sh`, `fast-tests.sh`, `safe-test-runner.sh` |
| **Validation** | `architectural-validation.sh`, `lint-and-test.sh` |
| **Deployment** | `deploy.sh` |
| **SDK** | `generate-sdk-types.js`, `run_bridge_tests.sh` |

See [scripts/README.md](../scripts/README.md) for complete documentation.

## Debugging

### Server Logs

```bash
# real-time logs
RUST_LOG=debug cargo run --bin pierre-mcp-server

# log to file
./bin/start-server.sh  # logs to server.log
```

### SDK Debugging

```bash
npx pierre-mcp-client@next --server http://localhost:8081 --verbose
```

### Health Check

```bash
curl http://localhost:8081/health
```

## Database

### SQLite (Development)

```bash
# location
./data/users.db

# reset
./scripts/fresh-start.sh
```

### PostgreSQL (Production)

```bash
# test postgresql integration
./scripts/test-postgres.sh
```

See [configuration.md](configuration.md) for database configuration.
