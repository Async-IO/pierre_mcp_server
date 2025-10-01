# MCP Spec Compliance Validation

This document describes how to validate the Pierre-Claude Bridge against the MCP (Model Context Protocol) specification.

## ⚠️ REQUIRED: Python MCP Validator

Per the **NO EXCEPTIONS POLICY** for testing, the Python MCP validator is **REQUIRED** for all bridge development and CI/CD.

**Installation (REQUIRED):**
```bash
pip install git+https://github.com/Janix-ai/mcp-validator.git
```

Without this, `../scripts/lint-and-test.sh` will FAST FAIL.

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

### 2. MCP Validator (Python-based) - **REQUIRED**

Automated compliance testing suite - MANDATORY for all development.

**Installation (REQUIRED):**
```bash
pip install git+https://github.com/Janix-ai/mcp-validator.git
```

**Verification:**
```bash
python3 -c "import mcp_testing; print('OK')"
```

**Usage:**
```bash
python3 -m mcp_testing.scripts.compliance_report \
  --server-command "node dist/cli.js" \
  --protocol-version 2025-06-18 \
  --timeout 30
```

**Tests:** Protocol negotiation, OAuth 2.1, error handling, security features

## Automated Testing (REQUIRED)

The validation runs automatically in `../scripts/lint-and-test.sh` and is **REQUIRED** to pass:

```bash
cd .. && ./scripts/lint-and-test.sh
```

**This will FAST FAIL if:**
- Python MCP validator is not installed
- Bridge build fails
- MCP compliance tests fail

Per the NO EXCEPTIONS POLICY, all tests must pass.

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
