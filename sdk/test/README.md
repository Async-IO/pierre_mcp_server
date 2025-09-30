# Pierre MCP Server Test Suite

Formal test suite that verifies Pierre MCP Server works correctly with Claude Desktop using SSE/Streamable HTTP Transport.

## Test Files

### `test_sse_claude_desktop.js` ⭐ PRIMARY TEST
Tests MCP Streamable HTTP Transport - the ACTUAL transport Claude Desktop uses.

**Run:** `npm test`

### Other Tests
- `test_e2e_claude_desktop.js` - Comprehensive E2E test
- `test_claude_desktop.js` - stdio bridge test
- `test_oauth_flow.js` - OAuth flows
- `test_tools_list.js` - Tool discovery
- `test_complete_flow.js` - Full workflow

## Quick Start

```bash
# 1. Start server (in another terminal)
cd ../..
cargo run --bin pierre-mcp-server

# 2. Run primary SSE test
cd sdk
npm test
```

## What Gets Tested

1. ✅ JWT authentication
2. ✅ SSE/Streamable HTTP transport
3. ✅ MCP protocol handshake
4. ✅ Session management
5. ✅ Tool listing
6. ✅ OAuth-protected tools
7. ✅ Real-time notifications

## Expected Output

```
[SSE Test] 🧪 Testing MCP Streamable HTTP Transport...
[SSE Test] ✅ Authentication successful
[SSE Test] ✅ MCP client connected via SSE!
[SSE Test] ✅ Session ID: session_abc123...
[SSE Test] ✅ Found 15 tools
[SSE Test] 🎉 All tests passed!
```