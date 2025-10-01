# MCP Spec Compliance Validation

This document describes how to validate the Pierre-Claude Bridge against the MCP (Model Context Protocol) specification.

## Quick Start

```bash
# Visual testing (opens web UI)
npm run inspect

# CLI testing (for automation)
npm run inspect:cli
```

## Tools

### 1. MCP Inspector (`@modelcontextprotocol/inspector`)

Interactive visual testing tool installed as dev dependency.

**Usage:**
- `npm run inspect` - Visual mode (http://localhost:6274)
- `npm run inspect:cli` - CLI mode for scripting

**Tests:** Real-time tool execution, resources, prompts, OAuth flows

### 2. MCP Validator (Python-based)

Automated compliance testing suite.

**Installation:**
```bash
pip install git+https://github.com/Janix-ai/mcp-validator.git
```

**Usage:**
```bash
python3 -m mcp_testing.scripts.compliance_report \
  --server-command "node dist/cli.js" \
  --protocol-version 2025-06-18 \
  --timeout 30
```

**Tests:** Protocol negotiation, OAuth 2.1, error handling, security features

## Automated Testing

The validation runs automatically in `../scripts/lint-and-test.sh`:

```bash
cd .. && ./scripts/lint-and-test.sh
```

## Protocol Support

- **Primary:** MCP Protocol 2025-06-18
- **Backward Compatible:** 2025-03-26, 2024-11-05

## Key Features Implemented

- ✅ Structured tool output
- ✅ OAuth 2.1 authentication
- ✅ Elicitation support
- ✅ Enhanced security (CORS, Origin validation)
- ✅ Bearer token validation
- ✅ PKCE flow

## References

- [MCP Spec](https://modelcontextprotocol.io/specification)
- [Inspector](https://github.com/modelcontextprotocol/inspector)
- [Validator](https://github.com/Janix-ai/mcp-validator)
