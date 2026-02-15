#!/bin/bash
# ABOUTME: Stops all Pierre development services (server, frontend, mobile, tunnel)
# ABOUTME: Kills processes by name pattern and by port as fallback to prevent stale processes

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

echo -e "${YELLOW}=== Stopping All Pierre Services ===${NC}"

killed=0

kill_by_pattern() {
    local pattern="$1"
    local label="$2"
    if pgrep -f "$pattern" > /dev/null 2>&1; then
        pkill -f "$pattern" 2>/dev/null || true
        echo "  Stopped: $label"
        killed=$((killed + 1))
    fi
}

kill_by_port() {
    local port="$1"
    local label="$2"
    local pid
    pid=$(lsof -ti :"$port" 2>/dev/null | head -1) || true
    if [ -n "$pid" ]; then
        kill "$pid" 2>/dev/null || true
        echo "  Stopped: $label (port $port, PID $pid)"
        killed=$((killed + 1))
    fi
}

# Pierre backend
kill_by_pattern "pierre-mcp-server" "Pierre MCP Server"
kill_by_pattern "cargo.*pierre-mcp-server" "Cargo (server build)"

# Vite frontend — match the actual binary path, not "vite.*frontend"
kill_by_pattern "node_modules/.bin/vite" "Vite dev server(s)"
kill_by_pattern "node_modules/@esbuild" "esbuild (Vite companion)"

# Expo / Metro
kill_by_pattern "expo start" "Expo CLI"
kill_by_pattern "node_modules/.bin/expo" "Expo binary"
kill_by_pattern "jest-worker/build/workers/processChild" "Metro workers"
kill_by_pattern "nativewind.*child" "NativeWind worker"

# Cloudflare tunnel
kill_by_pattern "cloudflared tunnel" "Cloudflare tunnel"

# Give processes time to exit
sleep 1

# Fallback: kill anything still holding our ports
kill_by_port 8081 "Pierre server (port fallback)"
kill_by_port 5173 "Vite frontend (port fallback)"
kill_by_port 8082 "Expo/Metro (port fallback)"

if [ "$killed" -eq 0 ]; then
    echo -e "${YELLOW}  No running services found${NC}"
else
    echo ""
fi

echo -e "${GREEN}All services stopped${NC}"
