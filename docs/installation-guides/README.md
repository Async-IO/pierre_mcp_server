# Installation Guides

This directory contains detailed installation instructions for Pierre MCP Server across different MCP clients and environments.

## Available Guides

### MCP Client Installation

| Client | Guide | Support |
|--------|-------|---------|
| Claude Desktop | [install-claude.md](install-claude.md) | Full Support |
| ChatGPT Desktop | [install-chatgpt.md](install-chatgpt.md) | Full Support |
| Cursor | [install-cursor.md](install-cursor.md) | Full Support |
| Other MCP Clients | [install-other-clients.md](install-other-clients.md) | Generic Guide |

### Environment-Specific Installation

| Environment | Guide | Description |
|-------------|-------|-------------|
| Docker | [install-docker.md](install-docker.md) | Containerized deployment |
| Development | [install-development.md](install-development.md) | Local development setup |
| Production | [install-production.md](install-production.md) | Production deployment |

## Installation Methods

Pierre MCP Server can be installed using several methods:

1. **Pre-built Binary**
   - Download from releases
   - Setup with automated scripts

2. **Docker Container**
   - Isolated environment
   - Deployment and scaling

3. **Build from Source**
   - Latest development features
   - Custom compilation options

## Quick Start

Start with the automated setup:

```bash
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
./scripts/complete-user-workflow.sh
```

## Security Best Practices

When installing Pierre MCP Server:

1. **Use Environment Variables** for sensitive configuration
2. **Enable HTTPS** in production deployments
3. **Rotate JWT Tokens** regularly
4. **Limit OAuth Scopes** to minimum required permissions
5. **Enable Audit Logging** for security monitoring

## Troubleshooting

Common installation issues and solutions:

### Database Connection Issues
```bash
# Check database connectivity
DATABASE_URL=sqlite:./data/pierre.db cargo run --bin pierre-mcp-server
```

### Port Conflicts
```bash
# Use custom ports
MCP_PORT=8080 HTTP_PORT=8081 cargo run --bin pierre-mcp-server
```

### OAuth Configuration
```bash
# Verify OAuth environment variables
echo $STRAVA_CLIENT_ID
echo $STRAVA_CLIENT_SECRET
```

## Getting Help

If you encounter issues during installation:

1. Check the specific installation guide for your client
2. Review the [troubleshooting guide](../developer-guide/16-testing-strategy.md)
3. Open an issue on GitHub with:
   - Your operating system
   - MCP client and version
   - Error messages
   - Steps to reproduce