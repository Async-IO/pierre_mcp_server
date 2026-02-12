# MCP Spec Compliance Validation

This document describes how to validate the Pierre-Claude Bridge against the MCP (Model Context Protocol) specification.

## ⚠️ REQUIRED: Python MCP Validator

Per the **NO EXCEPTIONS POLICY** for testing, the Python MCP validator is **REQUIRED** for all bridge development and CI/CD.

**Installation (REQUIRED):**
```bash
# Clone the validator repository
git clone https://github.com/Janix-ai/mcp-validator.git ~/mcp-validator

# Install dependencies
cd ~/mcp-validator
python3 -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate
pip install -r requirements.txt
```

**Add to PATH (REQUIRED):**
```bash
# Add to your shell profile (~/.bashrc, ~/.zshrc, etc.)
export MCP_VALIDATOR_PATH="$HOME/mcp-validator"
export PATH="$MCP_VALIDATOR_PATH:$PATH"
```

Without this, `../scripts/ci/lint-and-test.sh` will FAST FAIL.

## Quick Start

```bash
# Visual testing (opens web UI)
bun run inspect

# CLI testing (for automation)
bun run inspect:cli
```

## Tools

### 1. MCP Inspector (`@modelcontextprotocol/inspector`)

Interactive visual testing tool installed as dev dependency.

**Usage:**
- `bun run inspect` - Visual mode (http://localhost:6274)
- `bun run inspect:cli` - CLI mode for scripting

**Tests:** Real-time tool execution, resources, prompts, OAuth flows

### 2. MCP Validator (Python-based) - **REQUIRED**

Automated compliance testing suite - MANDATORY for all development.

**Installation (REQUIRED):**
```bash
# Clone and setup
git clone https://github.com/Janix-ai/mcp-validator.git ~/mcp-validator
cd ~/mcp-validator
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt
```

**Verification:**
```bash
cd ~/mcp-validator
source venv/bin/activate
python3 -c "import sys; sys.path.insert(0, '.'); import mcp_testing; print('OK')"
```

**Usage:**
```bash
cd ~/mcp-validator
source venv/bin/activate
python3 -m mcp_testing.scripts.compliance_report \
  --server-command "node /path/to/pierre/sdk/dist/cli.js" \
  --protocol-version 2025-06-18 \
  --timeout 30
```

**Tests:** Protocol negotiation, OAuth 2.1, error handling, security features

## Automated Testing (REQUIRED)

The validation runs automatically in `../scripts/ci/lint-and-test.sh` and is **REQUIRED** to pass:

```bash
cd .. && ./scripts/ci/lint-and-test.sh
```

**The script automatically:**
- ✅ Builds the Pierre MCP server (if not already built)
- ✅ Starts the server with test configuration
- ✅ Waits for server health check to pass
- ✅ Runs MCP compliance validation tests
- ✅ Shuts down the server on completion/interruption

**No manual server management required!** Just run the script and it handles everything.

**This will FAST FAIL if:**
- Python MCP validator is not installed
- Bridge build fails
- Server fails to start or become healthy
- MCP compliance tests fail

Per the NO EXCEPTIONS POLICY, all tests must pass.

## Protocol Support

- **Primary:** MCP Protocol 2025-06-18
- **Backward Compatible:** 2025-03-26, 2024-11-05

## Key Features Implemented

### Core Protocol Features
- ✅ Structured tool output
- ✅ OAuth 2.1 authentication
- ✅ Elicitation support
- ✅ Enhanced security (CORS, Origin validation)
- ✅ Bearer token validation
- ✅ PKCE flow

### Advanced MCP Features
- ✅ **Sampling** (bidirectional LLM requests) - **FULLY IMPLEMENTED**
  - ✅ `SamplingPeer` with bidirectional transport (`src/mcp/sampling_peer.rs`)
  - ✅ Request/response correlation with unique IDs
  - ✅ 30-second timeout protection
  - ✅ **2 high-value intelligence tools integrated:**
    - `get_activity_intelligence` - AI-powered activity analysis with Claude
    - `generate_recommendations` - Personalized coaching advice with Claude
  - ✅ Graceful fallback to static analysis if Claude unavailable
  - ⚠️  E2E test requires real MCP client (infrastructure ready)
- ✅ **Completion** (argument auto-completion) - **FULLY IMPLEMENTED**
  - ✅ Provides intelligent completion suggestions for tool arguments
  - ✅ 8 passing tests
  - ✅ Activity types, providers, goals, resource URIs
- ✅ **Progress Reporting** - **FULLY IMPLEMENTED**
  - ✅ `ProgressTracker` with notification channels
  - ✅ `ProgressReporter` with callback pattern
  - ✅ **Progress notifications wired to MCP transport!**
  - ✅ Notification channel connected to stdout via `TransportManager` (`src/mcp/transport_manager.rs:67-81`)
  - ✅ `ProgressReporter` callback sends to MCP client
  - ✅ Working in multiple async handlers
- ✅ **Cancellation** - **FULLY IMPLEMENTED**
  - ✅ `CancellationToken` async-safe implementation (`src/protocols/universal/types.rs`)
  - ✅ Cancellation token registry in `ServerResources` for progress token mapping
  - ✅ **`notifications/cancelled` MCP handler** triggers cancellation (`src/mcp/mcp_request_processor.rs:395-411`)
  - ✅ Cancellation tokens created and registered for all tool executions
  - ✅ **Integrated in ALL async handlers** (17 handlers across 8 files):
    - `intelligence.rs`, `fitness_api.rs`, `configuration.rs`, `connections.rs`
    - `goals.rs`, `nutrition.rs`, `sleep_recovery.rs`, `recipes.rs`
  - ✅ Full MCP flow: Client → notifications/cancelled → Registry → Token → Handler

## References

- [MCP Spec](https://modelcontextprotocol.io/specification)
- [Inspector](https://github.com/modelcontextprotocol/inspector)
- [Validator](https://github.com/Janix-ai/mcp-validator)
