#!/bin/bash
# Test script for OpenClaw + Arcadia Tracker integration
# This script:
# 1. Starts the tracker server
# 2. Starts OpenClaw client (connects to hub)
# 3. Starts a second client that sends messages

set -e

echo "=== Arcadia Tracker Test Setup ==="

# Configuration
TRACKER_PORT=${TRACKER_PORT:-8000}
TRACKER_URL="http://localhost:$TRACKER_PORT"
HUB_NAME="test-hub"
OPENCLAW_CLIENT_NAME="openclaw-agent"
SENDER_CLIENT_NAME="sender-client"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if arcadia-agent binary exists
ARCADIA_BIN="${ARCADIA_AGENT_BIN:-/Users/tomwu/bitagents/target/debug/arcadia-agent}"
if [ ! -f "$ARCADIA_BIN" ]; then
    log_error "arcadia-agent binary not found at $ARCADIA_BIN"
    log_info "Build it with: cd tracker/arcadia_client && cargo build --bin arcadia-agent"
    exit 1
fi

log_info "Using arcadia-agent: $ARCADIA_BIN"

# Step 1: Start tracker server
log_info "Step 1: Starting tracker server..."

# Check if tracker server is already running
if curl -s "$TRACKER_URL/api/v1/hubs" > /dev/null 2>&1; then
    log_info "Tracker server already running at $TRACKER_URL"
else
    log_info "Starting tracker server on port $TRACKER_PORT..."
    # You need to start your tracker server here
    # Example: cargo run -p arcadia_tracker --bin tracker
    log_warn "Please start your tracker server manually:"
    log_warn "  cd tracker/arcadia_tracker && cargo run"
    exit 1
fi

# Step 2: List available hubs
log_info "Step 2: Listing hubs..."
$ARCADIA_BIN list-hubs --tracker-url "$TRACKER_URL"

# Step 3: Connect OpenClaw client to hub
log_info "Step 3: Connecting OpenClaw client to hub '$HUB_NAME'..."
OPENCLAW_RESULT=$($ARCADIA_BIN connect-hub \
    --tracker-url "$TRACKER_URL" \
    --hub-name "$HUB_NAME" \
    --client-name "$OPENCLAW_CLIENT_NAME" 2>&1)

echo "$OPENCLAW_RESULT"

# Extract client info from JSON (you can use jq if available)
if command -v jq &> /dev/null; then
    CLIENT_ID=$(echo "$OPENCLAW_RESULT" | jq -r '.data.client_id')
    LOCAL_ADDR=$(echo "$OPENCLAW_RESULT" | jq -r '.data.local_address')
    log_info "OpenClaw client ID: $CLIENT_ID"
    log_info "OpenClaw local address: $LOCAL_ADDR"
fi

# Step 4: Start sender client in background
log_info "Step 4: Starting sender client..."

# Create a sender script that will send messages
cat > /tmp/arcadia_sender_$$.sh << 'SENDER_EOF'
#!/bin/bash
TRACKER_URL="$1"
HUB_NAME="$2"
CLIENT_NAME="$3"
RECIPIENT_ID="$4"

ARCADIA_BIN="${ARCADIA_AGENT_BIN:-/Users/tomwu/bitagents/target/debug/arcadia-agent}"

# Connect sender to same hub
echo "Connecting sender to hub..."
$ARCADIA_BIN connect-hub \
    --tracker-url "$TRACKER_URL" \
    --hub-name "$HUB_NAME" \
    --client-name "$CLIENT_NAME"

# Send messages
echo "Sending test messages..."
for i in 1 2 3; do
    echo "Message $i from sender"
    # Note: send_message requires persistent connection (not yet implemented)
    # This is a placeholder for the test
    sleep 1
done
SENDER_EOF

chmod +x /tmp/arcadia_sender_$$.sh

# Step 5: Test find-agent
log_info "Step 5: Testing agent discovery..."
$ARCADIA_BIN find-agent \
    --tracker-url "$TRACKER_URL" \
    --hub "$HUB_NAME" \
    --query "$OPENCLAW_CLIENT_NAME"

log_info "=== Test setup complete ==="
log_info "OpenClaw is now connected to the tracker network"
log_info "Other agents can discover it via: find_agent(hub='$HUB_NAME', query='$OPENCLAW_CLIENT_NAME')"

# Cleanup on exit
trap "rm -f /tmp/arcadia_sender_$$.sh" EXIT
