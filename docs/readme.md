# documentation

concise documentation for pierre mcp server.

## essential docs

- **[getting-started.md](getting-started.md)** - installation and quick start
- **[architecture.md](architecture.md)** - system design and components
- **[protocols.md](protocols.md)** - mcp, oauth2, a2a, rest protocols
- **[authentication.md](authentication.md)** - jwt, api keys, oauth2
- **[configuration.md](configuration.md)** - environment variables and settings
- **[contributing.md](contributing.md)** - development guidelines

## quick links

### for ai assistant users
start with [getting-started.md](getting-started.md) → connect claude/chatgpt to your fitness data

### for developers
1. [getting-started.md](getting-started.md) - setup dev environment
2. [architecture.md](architecture.md) - understand the system
3. [contributing.md](contributing.md) - coding standards
4. [protocols.md](protocols.md) - protocol details

### for integrators
- mcp clients: [protocols.md#mcp](protocols.md#mcp-model-context-protocol)
- web apps: [protocols.md#rest-api](protocols.md#rest-api)
- autonomous agents: [protocols.md#a2a](protocols.md#a2a-agent-to-agent-protocol)

## installation guides

located in `installation-guides/`:
- `install-mcp-client.md` - sdk installation for claude desktop, chatgpt

## additional resources

- openapi spec: `openapi.yaml`
- sdk documentation: `../sdk/README.md`
- main readme: `../README.md`

## documentation philosophy

- **concise**: developers won't read walls of text
- **accurate**: verified against actual code
- **practical**: code examples that work
- **lowercase**: consistent naming

based on https://github.com/github/github-mcp-server style.
