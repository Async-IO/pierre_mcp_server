#!/usr/bin/env bash
# ABOUTME: Starts a Cloudflare tunnel for mobile device testing
# ABOUTME: Updates BASE_URL in .envrc and optionally starts Expo

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TUNNEL_LOG="/tmp/cloudflare-tunnel.log"
START_EXPO=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --start-expo)
            START_EXPO=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--start-expo]"
            exit 1
            ;;
    esac
done

# Check if cloudflared is installed
if ! command -v cloudflared &> /dev/null; then
    echo -e "\033[0;31m[ERROR]\033[0m cloudflared is not installed."
    echo "Install it with: brew install cloudflare/cloudflare/cloudflared"
    exit 1
fi

# Warn if .envrc doesn't exist (tunnel will create it, but it needs other vars)
ENVRC_FILE="$PROJECT_ROOT/.envrc"
if [[ ! -f "$ENVRC_FILE" ]]; then
    echo -e "\033[0;33m[WARN]\033[0m .envrc not found at $ENVRC_FILE"
    echo "The tunnel will create it with BASE_URL, but you need other variables."
    echo "Consider running: cp .envrc.example .envrc"
fi

# Check if Pierre server is running on port 8081
if ! curl -s http://localhost:8081/health > /dev/null 2>&1; then
    echo -e "\033[0;33m[WARN]\033[0m Pierre server not running on port 8081."
    echo "Start it with: ./bin/start-server.sh"
    echo "Continuing anyway - tunnel will connect once server starts..."
fi

# Kill any existing tunnel
pkill -f 'cloudflared tunnel' 2>/dev/null || true

echo -e "\033[0;32m[INFO]\033[0m Starting Cloudflare tunnel to localhost:8081..."

# Start tunnel in background
cloudflared tunnel --url http://localhost:8081 > "$TUNNEL_LOG" 2>&1 &
TUNNEL_PID=$!

# Wait for tunnel URL to be available
echo -e "\033[0;32m[INFO]\033[0m Waiting for tunnel URL..."
for i in {1..30}; do
    TUNNEL_URL=$(grep -o 'https://[a-z0-9-]*\.trycloudflare\.com' "$TUNNEL_LOG" 2>/dev/null | head -1)
    if [[ -n "$TUNNEL_URL" ]]; then
        break
    fi
    sleep 1
done

if [[ -z "$TUNNEL_URL" ]]; then
    echo -e "\033[0;31m[ERROR]\033[0m Failed to get tunnel URL after 30 seconds."
    echo "Check $TUNNEL_LOG for details."
    kill $TUNNEL_PID 2>/dev/null || true
    exit 1
fi

echo -e "\033[0;32m[SUCCESS]\033[0m Tunnel URL: $TUNNEL_URL"

# Update backend .envrc with BASE_URL
ENVRC_FILE="$PROJECT_ROOT/.envrc"
if [[ -f "$ENVRC_FILE" ]]; then
    if grep -q '^export BASE_URL=' "$ENVRC_FILE" 2>/dev/null; then
        # Update existing BASE_URL
        sed -i.bak "s|^export BASE_URL=.*|export BASE_URL=\"$TUNNEL_URL\"|" "$ENVRC_FILE"
        rm -f "$ENVRC_FILE.bak"
    else
        # Add BASE_URL
        echo "export BASE_URL=\"$TUNNEL_URL\"" >> "$ENVRC_FILE"
    fi
    echo -e "\033[0;32m[INFO]\033[0m Updated BASE_URL in .envrc"
else
    echo "export BASE_URL=\"$TUNNEL_URL\"" > "$ENVRC_FILE"
    echo -e "\033[0;32m[INFO]\033[0m Created .envrc with BASE_URL"
fi

# Update mobile .env
MOBILE_ENV="$PROJECT_ROOT/frontend-mobile/.env"
echo "EXPO_PUBLIC_API_URL=\"$TUNNEL_URL\"" > "$MOBILE_ENV"
echo -e "\033[0;32m[INFO]\033[0m Updated EXPO_PUBLIC_API_URL in frontend-mobile/.env"

echo ""
echo -e "\033[0;33m>>> IMPORTANT: Run these commands to complete setup <<<\033[0m"
echo ""
echo "  1. In the backend directory:"
echo "     direnv allow && ./bin/stop-server.sh && ./bin/start-server.sh"
echo ""
echo "  2. The tunnel is running in the background (PID: $TUNNEL_PID)"
echo "     Stop it with: bun run tunnel:stop (from frontend-mobile/)"
echo ""

if [[ "$START_EXPO" == "true" ]]; then
    echo -e "\033[0;32m[INFO]\033[0m Starting Expo on port 8082..."
    cd "$PROJECT_ROOT/frontend-mobile"
    exec expo start --go --port 8082
else
    echo "  3. Start Expo manually:"
    echo "     cd frontend-mobile && bun start"
    echo ""
fi
