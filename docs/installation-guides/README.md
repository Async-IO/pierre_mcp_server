# Installation Guides

This directory contains detailed installation instructions for Pierre MCP Client SDK.

## Quick Start

Get started with Pierre MCP Client in under 5 minutes:

**[MCP Client Installation Guide](install-mcp-client.md)**

This unified guide covers installation for all MCP-compatible applications:
- **Claude Desktop** - Full support
- **ChatGPT Desktop** - Full support
- **Other MCP Clients** - Generic stdio configuration

### Server Installation

Refer to the [main README.md](../../README.md) for Pierre Fitness Intelligence installation and setup.

## Installation Methods

Pierre Fitness Intelligence can be installed using several methods:

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

When installing Pierre Fitness Intelligence:

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
DATABASE_URL=sqlite:./data/users.db cargo run --bin pierre-mcp-server
```

### Port Conflicts
```bash
# Use custom port (unified architecture)
HTTP_PORT=8081 cargo run --bin pierre-mcp-server
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
2. Review the developer guide documentation
3. Open an issue on GitHub with:
   - Your operating system
   - MCP client and version
   - Error messages
   - Steps to reproduce