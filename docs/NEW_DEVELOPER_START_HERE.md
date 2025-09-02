# New Developer? Start Here!

Welcome to Pierre MCP Server.

## What is Pierre MCP Server?

Pierre is a fitness data API platform that connects AI assistants (like Claude) to fitness providers (like Strava). It supports:
- **MCP protocol** for AI assistants
- **A2A protocol** for enterprise integrations  
- **REST API** for web applications

## Choose Your Path

### Path 1: "I want to contribute code" (5 minutes)
```bash
git clone YOUR_FORK
cd pierre_mcp_server
cargo build --release

# Quick setup with automated script
./scripts/fresh-start.sh
source .envrc && cargo run --bin pierre-mcp-server &
./scripts/complete-user-workflow.sh  # Creates admin, user, tenant, tests MCP

# Or manual startup
cargo run --bin pierre-mcp-server
# ✅ Server ready on ports 8080 + 8081
curl http://localhost:8081/api/health  # Should return {"status":"healthy"}
```
**Automated script creates**: admin user, regular user, tenant, and validates 25 MCP tools work correctly.
**Environment saved**: `source .workflow_test_env` to reuse JWT tokens.

Ready to code - see [CONTRIBUTING.md](../CONTRIBUTING.md) for your first contribution.

### Path 2: "I want to understand the system" (15 minutes)
1. **Architecture**: Read [Architecture Overview](developer-guide/01-architecture.md) (5 min)
2. **APIs**: Check [API Reference](developer-guide/14-api-reference.md) (5 min)  
3. **Try it**: Follow Path 1 above to run locally (5 min)

### Path 3: "I want to integrate with Pierre" (30 minutes)
1. **MCP Integration**: [MCP Protocol Guide](developer-guide/04-mcp-protocol.md)
2. **A2A Integration**: [A2A Integration Examples](developer-guide/A2A-INTEGRATION-GUIDE.md)
3. **REST API**: [API Reference](developer-guide/14-api-reference.md)

## Essential Reading

### For All Developers
- **[CONTRIBUTING.md](../CONTRIBUTING.md)** - How to contribute (must read)
- **[Architecture Overview](developer-guide/01-architecture.md)** - System design
- **[API Reference](developer-guide/14-api-reference.md)** - Complete API docs

### For Backend Developers  
- **[Getting Started](developer-guide/15-getting-started.md)** - Detailed setup
- **[Database Layer](developer-guide/08-database.md)** - Data models
- **[Authentication](developer-guide/06-authentication.md)** - Security system

### For Integration Developers
- **[MCP Protocol](developer-guide/04-mcp-protocol.md)** - AI assistant integration
- **[A2A Protocol](developer-guide/05-a2a-protocol.md)** - Enterprise integration
- **[A2A Examples](developer-guide/A2A-INTEGRATION-GUIDE.md)** - Discord bots, IoT, analytics

## Quick Reference

### API Endpoints
| Purpose | Endpoint | Port | Auth |
|---------|----------|------|------|
| Health check | `GET /api/health` | 8081 | None |
| User registration | `POST /api/auth/register` | 8081 | None |
| Admin actions | `POST /admin/*` | 8081 | Admin JWT |
| A2A protocol | `POST /a2a/*` | 8081 | Client credentials |
| MCP protocol | All MCP calls | 8080 | User JWT |

### Key Binaries
| Binary | Purpose |
|--------|---------|
| `pierre-mcp-server` | Main server (always running) |
| `pierre-mcp-client` | MCP client for Claude Desktop |
| `/admin/setup` API | Admin user management via server API |

### Directory Structure
```
src/
├── bin/                 # Executables (pierre-mcp-server)
├── mcp/                 # MCP protocol implementation
├── a2a/                 # A2A protocol implementation  
├── providers/           # Fitness providers (Strava, Fitbit)
├── intelligence/        # Data analysis and insights
├── database/           # Database operations
└── routes.rs           # HTTP API routes

docs/developer-guide/   # Complete technical documentation
tests/                  # Integration and unit tests
frontend/              # Admin dashboard (React + TypeScript)
```

## Common First-Time Issues

**Build fails?**
```bash
rustup update
cargo clean && cargo build
```

**Server won't start?**
```bash
lsof -i :8080 -i :8081  # Check if ports in use
./scripts/fresh-start.sh # Clean database restart
```

**Tests failing?**
```bash
RUST_LOG=debug cargo test -- --nocapture
```

## Good First Contributions

### Easy (30 min)
- Fix typos in documentation
- Add examples to API reference
- Improve error messages

### Medium (2-4 hours)
- Add new MCP tool
- Add test coverage
- Frontend improvements

### Advanced (1+ days)
- New fitness provider (Garmin, Polar)
- Performance optimization
- Security enhancements

## Getting Help

1. **Check docs first** - Most questions answered in [developer-guide/](developer-guide/)
2. **Search issues** - Check closed issues for similar problems
3. **Enable debug logs** - `RUST_LOG=debug cargo run --bin pierre-mcp-server`
4. **Ask in discussions** - We're friendly and responsive!

## TL;DR

```bash
# For impatient developers who just want to start coding
git clone YOUR_FORK
cd pierre_mcp_server
cargo build && cargo run --bin pierre-mcp-server
# Make changes, test with ./scripts/lint-and-test.sh, submit PR
```

Ready to contribute? Head to [CONTRIBUTING.md](../CONTRIBUTING.md) for the quick start guide.

---

*This guide is maintained to ensure new developers can contribute quickly. Please update it when processes change.*