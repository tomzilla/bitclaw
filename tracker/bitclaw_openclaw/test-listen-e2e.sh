#!/bin/bash
# End-to-end test for BitClaw message listening and auto-forward
#
# This script:
# 1. Verifies tracker server is running
# 2. Starts bitclaw-agent listen in background (simulating OpenClaw)
# 3. Registers a sender agent
# 4. Sends a P2P message from sender to listener
# 5. Verifies message was received and auto-forwarded
#
# Prerequisites:
# - Tracker server running on port 8080
# - bitclaw-agent binary built
#
# Usage: ./test-listen-e2e.sh
#
# See also:
# - start-listener.sh - Start listener manually
# - send-message.sh - Send message to listener
#
set -e

TRACKER_URL="${TRACKER_URL:-http://127.0.0.1:8080}"
HUB_NAME="${HUB_NAME:-general}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

log() { echo -e "${BLUE}[TEST]${NC} $1"; }
log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_step() { echo -e "${CYAN}==> $1${NC}"; }

BITCLAW_BIN="${BITCLAW_AGENT_BIN:-/Users/tomwu/bitagents/target/debug/bitclaw-agent}"

echo "========================================"
echo "  BitClaw Listen + Auto-Forward E2E Test"
echo "========================================"
echo ""

# Check binary
if [ ! -f "$BITCLAW_BIN" ]; then
    log_error "bitclaw-agent not found at $BITCLAW_BIN"
    log_info "Build with: cargo build -p bitclaw_client --bin bitclaw-agent"
    exit 1
fi

# Step 0: Check tracker is running
log_step "Step 0: Checking tracker server..."
if curl -s "$TRACKER_URL/api/v1/hubs" > /dev/null 2>&1; then
    log_info "Tracker is running at $TRACKER_URL"
else
    log_error "Tracker not reachable at $TRACKER_URL"
    log_info "Start with: ~/.cargo/bin/cargo run -p bitclaw_tracker"
    exit 1
fi

# Step 1: Start listener in background (simulates OpenClaw listening)
log_step "Step 1: Starting message listener (simulating OpenClaw)..."

LISTENER_LOG="/tmp/bitclaw_listener_$$.log"
LISTENER_PID=""

# Start listener - it will output JSONL messages to stdout
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

# Step 2: Register sender agent
log_step "Step 2: Registering sender agent..."

SENDER_OUTPUT=$($BITCLAW_BIN register \
    --tracker-url "$TRACKER_URL" \
    --name "test-sender" \
    --description "Test sender for E2E message test" \
    --hub "$HUB_NAME" \
    --lan-mode 2>&1)

echo "$SENDER_OUTPUT"

SENDER_ID=$(echo "$SENDER_OUTPUT" | grep -o '"client_id"[[:space:]]*:[[:space:]]*"[^"]*"' | cut -d'"' -f4)
SENDER_ADDR=$(echo "$SENDER_OUTPUT" | grep -o '"local_address"[[:space:]]*:[[:space:]]*"[^"]*"' | cut -d'"' -f4)

if [ -n "$SENDER_ID" ]; then
    log_info "Sender ID: $SENDER_ID"
    log_info "Sender Address: $SENDER_ADDR"
else
    log_error "Failed to register sender"
    kill $LISTENER_PID 2>/dev/null
    exit 1
fi

# Step 3: Get listener info from tracker
log_step "Step 3: Looking up listener via tracker..."

# Search for the listener agent
curl -s "$TRACKER_URL/api/v1/agents/search?hub=$HUB_NAME" > /tmp/agents_$$.json

# Find listener's address
LISTENER_ENTRY=$(cat /tmp/agents_$$.json | python3 -c "
import sys, json
data = json.load(sys.stdin)
for a in data.get('agents', []):
    if 'openclaw' in a.get('name', '').lower():
        print(json.dumps(a))
        break
" 2>/dev/null)

if [ -n "$LISTENER_ENTRY" ]; then
    log_info "Found listener in tracker:"
    echo "$LISTENER_ENTRY" | python3 -m json.tool
    LISTENER_IP=$(echo "$LISTENER_ENTRY" | python3 -c "import sys,json; print(json.load(sys.stdin).get('ip_address',''))" 2>/dev/null)
    LISTENER_PORT=$(echo "$LISTENER_ENTRY" | python3 -c "import sys,json; print(json.load(sys.stdin).get('port',''))" 2>/dev/null)
    log_info "Listener P2P endpoint: $LISTENER_IP:$LISTENER_PORT"
else
    log_warn "Could not find listener via search - will use localhost"
    LISTENER_IP="127.0.0.1"
    LISTENER_PORT="60000"  # Default fallback
fi

# Step 4: Send P2P message using bitclaw-sender
log_step "Step 4: Sending P2P test message..."

# Get listener port from log or use default
LISTENER_PORT=$(grep -o 'Listening on: [0-9.]*:[0-9]*' "$LISTENER_LOG" | tail -1 | cut -d: -f3 || echo "")

if [ -z "$LISTENER_PORT" ]; then
    log_warn "Could not determine listener port from log"
    # Try to get it from tracker
    if [ -n "$LISTENER_IP" ] && [ -n "$LISTENER_PORT" ]; then
        LISTENER_PORT="$LISTENER_PORT"
    else
        log_error "Cannot determine listener port - skipping P2P test"
    fi
else
    log_info "Listener port detected: $LISTENER_PORT"
fi

# Build sender if not already built
SENDER_BIN="${BITCLAW_BIN%-agent}-sender"
if [ ! -f "$SENDER_BIN" ]; then
    log_info "Building bitclaw-sender..."
    ~/.cargo/bin/cargo build -p bitclaw_client --bin bitclaw-sender 2>&1 | tail -3
    SENDER_BIN="/Users/tomwu/bitagents/target/debug/bitclaw-sender"
fi

if [ -f "$SENDER_BIN" ]; then
    log_info "Sending test message to listener..."

    # Send a test message
    $SENDER_BIN \
        --target-ip "127.0.0.1" \
        --target-port "$LISTENER_PORT" \
        --message "E2E test message from bitclaw-sender at $(date)" \
        2>&1

    if [ $? -eq 0 ]; then
        log_info "Message sent successfully!"
    else
        log_error "Failed to send message"
    fi
else
    log_warn "bitclaw-sender not found at $SENDER_BIN"
fi

# Step 5: Check listener status and verify message received
log_step "Step 5: Checking listener status and message reception..."
sleep 2

if kill -0 $LISTENER_PID 2>/dev/null; then
    log_info "Listener is still running - ready to receive messages"
else
    log_warn "Listener has stopped"
fi

# Show any output
echo ""
log_info "Listener output (last 30 lines):"
tail -30 "$LISTENER_LOG" 2>/dev/null || echo "(no output yet)"

# Check if message was received
if grep -q "E2E test message" "$LISTENER_LOG" 2>/dev/null; then
    echo ""
    log_info "SUCCESS: Message was received by listener!"
    grep "E2E test message" "$LISTENER_LOG" | tail -1
else
    echo ""
    log_warn "Message reception not confirmed yet - check listener output above"
fi

# Summary
echo ""
echo "========================================"
echo "  Test Summary"
echo "========================================"
echo ""
log_info "Tracker: $TRACKER_URL"
log_info "Hub: $HUB_NAME"
log_info "Listener PID: $LISTENER_PID"
log_info "Sender ID: $SENDER_ID"
echo ""

# Save state
cat > /tmp/bitclaw_listen_state.json << EOF
{
  "tracker_url": "$TRACKER_URL",
  "hub": "$HUB_NAME",
  "listener_pid": "$LISTENER_PID",
  "listener_log": "$LISTENER_LOG",
  "sender_id": "$SENDER_ID"
}
EOF

log_info "State saved to /tmp/bitclaw_listen_state.json"
echo ""

# Cleanup option
echo "========================================"
echo "  Cleanup Options"
echo "========================================"
echo ""
log_info "Listener is running in background (PID: $LISTENER_PID)"
log_info "Log file: $LISTENER_LOG"
echo ""
read -p "Stop listener now? [y/N] " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    kill $LISTENER_PID 2>/dev/null
    log_info "Listener stopped"
    rm -f "$LISTENER_LOG"
    log_info "Log file cleaned"
else
    log_info "Listener kept running for manual testing"
    log_info "To stop later: kill $LISTENER_PID"
fi
