#!/bin/bash
# BitClaw Listener Script
#
# Starts a persistent message listener that simulates OpenClaw listening for P2P messages.
# Outputs received messages to stdout and log file.
#
# Usage: ./start-listener.sh [hub_name]
#
# Environment variables:
#   TRACKER_URL - Tracker base URL (default: http://127.0.0.1:8080)
#   BITCLAW_AGENT_BIN - Path to bitclaw-agent binary (default: /Users/tomwu/bitagents/target/debug/bitclaw-agent)
#
# Example:
#   ./start-listener.sh general
#   TRACKER_URL=http://localhost:9000 ./start-listener.sh my-hub
#

set -e

TRACKER_URL="${TRACKER_URL:-http://127.0.0.1:8080}"
HUB_NAME="${1:-general}"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m'

log() { echo -e "${BLUE}[LISTENER]${NC} $1"; }
log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_step() { echo -e "${CYAN}==> $1${NC}"; }

BITCLAW_BIN="${BITCLAW_AGENT_BIN:-/Users/tomwu/bitagents/target/debug/bitclaw-agent}"

echo "========================================"
echo "  BitClaw Listener"
echo "========================================"
echo ""

# Check binary
if [ ! -f "$BITCLAW_BIN" ]; then
    log_error "bitclaw-agent not found at $BITCLAW_BIN"
    log_info "Build with: cargo build -p bitclaw_client --bin bitclaw-agent"
    exit 1
fi

# Check tracker is running
log_step "Checking tracker server..."
if curl -s "$TRACKER_URL/api/v1/hubs" > /dev/null 2>&1; then
    log_info "Tracker is running at $TRACKER_URL"
else
    log_error "Tracker not reachable at $TRACKER_URL"
    log_info "Start with: ~/.cargo/bin/cargo run -p bitclaw_tracker"
    exit 1
fi

# Start listener
log_step "Starting message listener..."

LISTENER_LOG="/tmp/bitclaw_listener_$$.log"

$BITCLAW_BIN listen \
    --tracker-url "$TRACKER_URL" \
    --name "openclaw-listener" \
    --hub "$HUB_NAME" \
    --lan-mode \
    > "$LISTENER_LOG" 2>&1 &

LISTENER_PID=$!

log_info "Listener started (PID: $LISTENER_PID)"
log_info "Log file: $LISTENER_LOG"

# Wait for listener to initialize
sleep 3

# Check if listener is still running
if ! kill -0 $LISTENER_PID 2>/dev/null; then
    log_error "Listener crashed. Log output:"
    cat "$LISTENER_LOG"
    exit 1
fi

log_info "Listener is running and waiting for messages..."

# Wait for listener to initialize and capture port from log
sleep 3

# Get listener port from log file (more reliable than tracker API)
LISTENER_PORT=$(grep -o 'Listening on: [0-9.]*:[0-9]*' "$LISTENER_LOG" | tail -1 | sed 's/.*://' || echo "")
LISTENER_IP="127.0.0.1"

if [ -n "$LISTENER_PORT" ]; then
    log_info "Listener P2P endpoint: $LISTENER_IP:$LISTENER_PORT"
else
    log_warn "Could not determine listener port from log"

    # Fallback: try to get from tracker
    log_step "Trying to get listener info from tracker..."

    curl -s "$TRACKER_URL/api/v1/agents/search?hub=$HUB_NAME" > /tmp/agents_$$.json

    LISTENER_ENTRY=$(cat /tmp/agents_$$.json | python3 -c "
import sys, json
data = json.load(sys.stdin)
for a in data.get('agents', []):
    if 'openclaw' in a.get('name', '').lower():
        print(json.dumps(a))
        break
" 2>/dev/null || echo "")

    if [ -n "$LISTENER_ENTRY" ]; then
        log_info "Found listener in tracker:"
        echo "$LISTENER_ENTRY" | python3 -m json.tool
        LISTENER_IP=$(echo "$LISTENER_ENTRY" | python3 -c "import sys,json; print(json.load(sys.stdin).get('ip_address','127.0.0.1'))" 2>/dev/null)
        LISTENER_PORT=$(echo "$LISTENER_ENTRY" | python3 -c "import sys,json; print(json.load(sys.stdin).get('port',''))" 2>/dev/null)
        if [ -n "$LISTENER_PORT" ]; then
            log_info "Listener P2P endpoint: $LISTENER_IP:$LISTENER_PORT"
        fi
    else
        log_warn "Could not find listener in tracker (may not be registered yet)"
    fi
fi

# Save state
cat > /tmp/bitclaw_listener_state.json << EOF
{
  "tracker_url": "$TRACKER_URL",
  "hub": "$HUB_NAME",
  "listener_pid": "$LISTENER_PID",
  "listener_log": "$LISTENER_LOG",
  "listener_ip": "$LISTENER_IP",
  "listener_port": "$LISTENER_PORT"
}
EOF

log_info "State saved to /tmp/bitclaw_listener_state.json"

# Summary
echo ""
echo "========================================"
echo "  Listener Status"
echo "========================================"
echo ""
log_info "Tracker: $TRACKER_URL"
log_info "Hub: $HUB_NAME"
log_info "Listener PID: $LISTENER_PID"
log_info "Log file: $LISTENER_LOG"
echo ""
log_info "Listener is active and waiting for P2P messages"
log_info "Messages will be appended to: $LISTENER_LOG"
echo ""
log_info "To view messages in real-time: tail -f $LISTENER_LOG"
log_info "To stop: kill $LISTENER_PID"
echo ""

# Show last few lines of output
log_info "Current listener output (last 10 lines):"
tail -10 "$LISTENER_LOG" 2>/dev/null || echo "(no output yet)"

echo ""
log_warn "Keep this terminal open or run 'tail -f $LISTENER_LOG' to monitor messages"
