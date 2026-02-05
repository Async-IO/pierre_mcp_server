# Manual Testing Guide for Clean Token Fix

This guide covers **Scenarios 7-8** which require manual testing with the MCP Inspector.

---

## Prerequisites

Before starting these tests:

1. ✅ Pierre server running on `http://localhost:8081`
2. ✅ SDK built: `cd sdk && bun run build`
3. ✅ User account created on Pierre server (or will be created during OAuth)
4. ✅ Strava app credentials configured on Pierre server

---

## Scenario 7: Port Conflict Handling

**Goal**: Verify EADDRINUSE error handling still works from main branch

### Setup

1. **Start something on port 35535** (simulate conflict):
   ```bash
   # In a separate terminal, occupy port 35535
   python3 -m http.server 35535
   # OR
   nc -l 35535
   ```

2. **Start MCP Inspector**:
   ```bash
   cd sdk
   bun run inspect:cli
   ```

### Test Steps

1. In the Inspector prompt, run:
   ```
   tools/call connect_to_pierre {}
   ```

2. **Watch the bridge logs** (Inspector shows them) for:
   ```
   [Pierre OAuth] Port 35535 is already in use - likely from previous session
   [Pierre OAuth] Attempting to use dynamic port assignment instead...
   [Pierre OAuth] Callback server started on localhost:XXXXX (dynamic port)
   ```

3. **Browser should open** with OAuth page

4. **Complete authentication** - enter credentials

5. **Success page** should display with callback URL showing the dynamic port

6. **Inspector should show** success response from `connect_to_pierre`

### Expected Results

✅ **PASS Criteria**:
- Bridge detects port conflict (log message)
- Falls back to dynamic port automatically
- OAuth completes successfully on fallback port
- No errors or failures

❌ **FAIL Criteria**:
- Bridge crashes with EADDRINUSE error
- OAuth callback fails
- Success page shows error

### Cleanup

```bash
# Stop the process occupying port 35535
# Ctrl+C in the terminal where you started it
```

---

## Scenario 8: Complete 7-Step OAuth Flow (End-to-End)

**Goal**: Verify the exact flow ChefFamille described

### Setup

1. **Delete tokens** for clean start:
   ```bash
   rm ~/.pierre-claude-tokens.json
   ```

2. **Start MCP Inspector**:
   ```bash
   cd sdk
   bun run inspect:cli
   ```

### Test Steps

#### **Step 1: Connect to Pierre**

Run in Inspector:
```
tools/call connect_to_pierre {}
```

#### **Step 2: OAuth Page Opens**

✅ Verify:
- Browser opens automatically
- Pierre OAuth page loads
- Shows login/registration form

#### **Step 3: User Authenticates**

1. Enter credentials (or register if needed)
2. Authorize the application
3. ✅ Verify redirect to callback URL
4. ✅ Verify success page displays:
   ```
   ✅ Authorization Successful
   You can close this tab and return to Claude Desktop
   ```

#### **Step 4: Focus Recovery** *(Inspector Limitation - Skip)*

Note: The CLI Inspector can't restore focus like Claude Desktop GUI. This is expected.

#### **Step 5: Connect to Strava**

Run in Inspector:
```
tools/call connect_provider {"provider": "strava"}
```

✅ Verify Inspector response shows Pierre authentication step succeeded

#### **Step 6: Strava OAuth Page Opens**

✅ Verify:
- Browser opens automatically (second window)
- Strava OAuth authorization page loads
- Shows requested permissions

1. Click "Authorize"
2. ✅ Verify redirect to callback URL (Pierre server)
3. ✅ Verify success page displays:
   ```
   ✅ Strava Connected Successfully
   You can close this tab and return to Claude Desktop
   ```

#### **Step 7: Focus Recovery** *(Inspector Limitation - Skip)*

Note: CLI Inspector can't restore focus. Skip this step.

#### **Step 8: Verify Complete Integration**

Run in Inspector:
```
tools/call get_activities {"limit": 5}
```

✅ Verify:
- Returns Strava activities (not auth error)
- Shows activity data (distance, time, etc.)
- Response includes both text and structured content

### Verify Token Storage

```bash
cat ~/.pierre-claude-tokens.json | jq .
```

✅ Expected structure:
```json
{
  "pierre": {
    "access_token": "eyJ0eXA...",
    "expires_in": 3600,
    "saved_at": 1728334567
  },
  "providers": {
    "strava": {
      "access_token": "...",
      "refresh_token": "...",
      "expires_at": 1728421000000
    }
  }
}
```

### Expected Results

✅ **PASS Criteria**:
- All 8 steps complete without errors
- Browser opens exactly twice (Pierre + Strava)
- Success pages display correctly
- Token file contains both Pierre and Strava tokens
- `get_activities` returns real Strava data
- No authentication errors

❌ **FAIL Criteria**:
- OAuth flow fails or hangs
- Browser doesn't open
- Multiple unnecessary browser windows
- Authentication errors after completion
- Token file missing or incomplete

---

## Bonus Test: Re-Run Scenario 8 to Test Optimizations

After completing Scenario 8 once, **WITHOUT deleting tokens**, repeat steps 5-8:

### Expected Optimized Behavior

#### **Step 5 (Second Time): Connect to Strava**
```
tools/call connect_provider {"provider": "strava"}
```

✅ **Expected**: Returns "Already connected to STRAVA!" **WITHOUT opening browser**

This tests the provider connection optimization from our clean fix!

#### **Step 8 (Second Time): Get Activities**
```
tools/call get_activities {"limit": 5}
```

✅ **Expected**: Returns Strava data immediately (cached tokens used)

---

## Troubleshooting

### Issue: Browser Doesn't Open

**Check**:
1. Platform detection in logs (darwin/win32/linux)
2. Command used (`open`, `start`, or `xdg-open`)

**Fix**: Manually open the URL shown in logs

### Issue: Callback Never Completes

**Check**:
1. Callback server started (log message)
2. Port not blocked by firewall
3. Redirect URI matches server configuration

**Fix**: Check Pierre server logs for errors

### Issue: Token File Not Created

**Check**:
1. File permissions on home directory
2. Token save errors in logs

**Fix**:
```bash
touch ~/.pierre-claude-tokens.json
chmod 600 ~/.pierre-claude-tokens.json
```

### Issue: Inspector Shows "Connection Refused"

**Check**:
1. Pierre server is running: `curl http://localhost:8081/health`
2. Bridge process is running

**Fix**: Start Pierre server or restart bridge

---

## Quick Inspector Commands Reference

### List all available tools
```
tools/list
```

### Call a tool with arguments
```
tools/call TOOL_NAME {"arg1": "value1", "arg2": "value2"}
```

### Example tool calls
```
tools/call connect_to_pierre {}
tools/call connect_provider {"provider": "strava"}
tools/call get_connection_status {"provider": "strava"}
tools/call get_activities {"limit": 10}
tools/call get_athlete {}
tools/call get_stats {"athlete_id": "12345"}
```

### Exit Inspector
```
Ctrl+C
```

---

## Test Checklist

Use this checklist when running the manual tests:

### Scenario 7: Port Conflict Handling
- [ ] Port 35535 occupied before test
- [ ] Bridge detects conflict (log message seen)
- [ ] Falls back to dynamic port
- [ ] OAuth completes on fallback port
- [ ] No errors or crashes

### Scenario 8: Complete 7-Step Flow
- [ ] Step 1: `connect_to_pierre` called
- [ ] Step 2: Browser opened for Pierre OAuth
- [ ] Step 3: Authentication completed, success page shown
- [ ] Step 5: `connect_provider` called for Strava
- [ ] Step 6: Browser opened for Strava OAuth
- [ ] Step 6: Authorization completed, success page shown
- [ ] Step 8: `get_activities` returns Strava data
- [ ] Token file contains both Pierre and Strava tokens

### Bonus: Optimization Test
- [ ] Second `connect_provider` call doesn't open browser
- [ ] Returns "Already connected" message
- [ ] `get_activities` still works with cached tokens

---

## Reporting Results

After completing the manual tests, document:

1. **Pass/Fail for each scenario**
2. **Any unexpected behavior**
3. **Log messages that helped diagnose issues**
4. **Screenshots of success pages** (optional but helpful)

Save your notes for the final validation with Claude Desktop!

---

## Next Steps

After both automated (scenarios 1-6) and manual (scenarios 7-8) tests pass:

1. ✅ Clean fix implementation validated
2. 🚀 Ready for final validation with Claude Desktop
3. 🎯 Ready to merge to main branch

---

**Happy Testing, ChefFamille!** 🚀
