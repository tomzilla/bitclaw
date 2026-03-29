#!/bin/bash
# End-to-end test for Arcadia Tracker + OpenClaw integration
# This script:
# 1. Starts the tracker server
# 2. Lists available hubs
# 3. Registers two agents (OpenClaw client + sender client)
# 4. Demonstrates agent discovery

set -e

TRACKER_PORT="${TRACKER_PORT:-8000}"
TRACKER_URL="http://localhost:$TRACKER_PORT"
HUB_NAME="${HUB_NAME:-code-generation}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() { echo -e "${BLUE}[TEST]${NC} $1"; }
log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

ARCADIA_BIN="${ARCADIA_AGENT_BIN:-/Users/tomwu/bitagents/target/debug/arcadia-agent}"

# Check binary
if [ ! -f "$ARCADIA_BIN" ]; then
    log_error "arcadia-agent not found at $ARCADIA_BIN"
    log_info "Build with: cargo build --bin arcadia-agent"
    exit 1
fi

echo "========================================"
echo "  Arcadia Tracker E2E Test"
echo "========================================"
echo ""

# Step 1: Start tracker server
log "Step 1: Checking tracker server..."

# Check if already running
if curl -s "$TRACKER_URL/api/v1/hubs" > /dev/null 2>&1; then
    log_info "Tracker already running at $TRACKER_URL"
else
    log_info "Starting tracker in background..."
    cd /Users/tomwu/bitagents/tracker/arcadia_tracker

    # Create .env if not exists
    cat > .env << 'EOF'
RUST_LOG="info"
WEB_SERVER_HOST=0.0.0.0
WEB_SERVER_PORT=8000
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/postgres
API_KEY=test-api-key
ALLOWED_TORRENT_CLIENTS=-UT,UT,UM,KT,qB,TT,FL,qT,BC,TL,SS
NUMWANT_DEFAULT=50
NUMWANT_MAX=100
ANNOUNCE_MIN=30
ANNOUNCE_MIN_ENFORCED=30
ANNOUNCE_MAX=1800
MAX_PEERS_PER_TORRENT_PER_USER=10
FLUSH_INTERVAL_MILLISECONDS=5000
PEER_EXPIRY_INTERVAL=60
REVERSE_PROXY_CLIENT_IP_HEADER_NAME=
INACTIVE_PEER_TTL=300
ACTIVE_PEER_TTL=900
OTEL_SERVICE_NAME=
AGENT_HEARTBEAT_TTL_SECONDS=300
EOF

    # Start tracker in background
    ~/.cargo/bin/cargo run > /tmp/tracker.log 2>&1 &
    TRACKER_PID=$!
    echo $TRACKER_PID > /tmp/tracker.pid

    # Wait for server to start
    for i in {1..30}; do
        if curl -s "$TRACKER_URL/api/v1/hubs" > /dev/null 2>&1; then
            log_info "Tracker started (PID: $TRACKER_PID)"
            break
        fi
        sleep 1
    done

    if ! curl -s "$TRACKER_URL/api/v1/hubs" > /dev/null 2>&1; then
        log_error "Failed to start tracker. Check /tmp/tracker.log"
        exit 1
    fi
fi

# Show available hubs
echo ""
log "Step 2: Listing available hubs..."
$ARCADIA_BIN list-hubs --tracker-url "$TRACKER_URL"

# Step 3: Register OpenClaw client as an agent
echo ""
log "Step 3: Registering OpenClaw agent..."
log_info "Hub: $HUB_NAME, Agent: openclaw-agent"

OPENCLAW_OUTPUT=$($ARCADIA_BIN register \
    --tracker-url "$TRACKER_URL" \
    --name "openclaw-agent" \
    --description "OpenClaw AI agent for code review and development" \
    --hub "$HUB_NAME" \
    --lan-mode 2>&1)

echo "$OPENCLAW_OUTPUT"

# Extract client info (basic parsing without jq)
CLIENT_ID=$(echo "$OPENCLAW_OUTPUT" | grep -o '"client_id"[[:space:]]*:[[:space:]]*"[^"]*"' | cut -d'"' -f4)
LOCAL_ADDR=$(echo "$OPENCLAW_OUTPUT" | grep -o '"local_address"[[:space:]]*:[[:space:]]*"[^"]*"' | cut -d'"' -f4)

if [ -n "$CLIENT_ID" ]; then
    log_info "OpenClaw Client ID: $CLIENT_ID"
    log_info "OpenClaw Local Address: $LOCAL_ADDR"
fi

# Step 4: Register sender client
echo ""
log "Step 4: Registering sender agent..."
SENDER_OUTPUT=$($ARCADIA_BIN register \
    --tracker-url "$TRACKER_URL" \
    --name "sender-agent" \
    --description "Test sender agent for message demonstration" \
    --hub "$HUB_NAME" \
    --lan-mode 2>&1)

echo "$SENDER_OUTPUT"
SENDER_ID=$(echo "$SENDER_OUTPUT" | grep -o '"client_id"[[:space:]]*:[[:space:]]*"[^"]*"' | cut -d'"' -f4)
log_info "Sender Client ID: $SENDER_ID"

# Step 5: Discover agents
echo ""
log "Step 5: Discovering agents in hub '$HUB_NAME'..."

# Query the tracker API directly for agent discovery
curl -s -X POST "$TRACKER_URL/api/v1/hubs/search" \
    -H "Content-Type: application/json" \
    -d "{\"q\": \"agent\", \"limit\": 10}" | python3 -m json.tool 2>/dev/null || \
    curl -s -X POST "$TRACKER_URL/api/v1/hubs/search" \
    -H "Content-Type: application/json" \
    -d "{\"q\": \"agent\", \"limit\": 10}"

# Summary
echo ""
echo "========================================"
echo "  Test Summary"
echo "========================================"
echo ""
log_info "Tracker URL: $TRACKER_URL"
log_info "Hub: $HUB_NAME"
log_info "OpenClaw Client: $CLIENT_ID @ $LOCAL_ADDR"
log_info "Sender Client: $SENDER_ID"
echo ""
log "Both agents are registered and discoverable"
log "P2P messaging requires persistent connection (future enhancement)"
echo ""
log_warn "To clean up: kill the tracker process or run 'pkill -f arcadia_tracker'"

# Save connection info for follow-up tests
cat > /tmp/arcadia_test_state.json << EOF
{
  "tracker_url": "$TRACKER_URL",
  "hub": "$HUB_NAME",
  "openclaw_client_id": "$CLIENT_ID",
  "openclaw_address": "$LOCAL_ADDR",
  "sender_client_id": "$SENDER_ID"
}
EOF

log_info "Connection state saved to /tmp/arcadia_test_state.json"
