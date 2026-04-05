#!/bin/bash
# BitClaw Sender Script
#
# Sends a P2P message to a running listener.
#
# Usage: ./send-message.sh --target-port <port> [--target-ip <ip>] [--message <msg>]
#
# Environment variables:
#   BITCLAW_AGENT_BIN - Path to bitclaw-agent binary (default: /Users/tomwu/bitagents/target/debug/bitclaw-agent)
#
# Examples:
#   ./send-message.sh --target-port 60000
#   ./send-message.sh --target-port 60000 --message "Hello!"
#   ./send-message.sh --target-ip 192.168.1.100 --target-port 60000 --message "Test"
#
# Tip: If you started a listener with start-listener.sh, use the port from:
#   cat /tmp/bitclaw_listener_state.json | python3 -c "import sys,json; print(json.load(sys.stdin).get('listener_port',''))"
#

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m'

log() { echo -e "${BLUE}[SENDER]${NC} $1"; }
log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_step() { echo -e "${CYAN}==> $1${NC}"; }

BITCLAW_BIN="${BITCLAW_AGENT_BIN:-/Users/tomwu/bitagents/target/debug/bitclaw-agent}"
SENDER_BIN="${BITCLAW_BIN%-agent}-sender"

# Parse arguments
TARGET_IP="127.0.0.1"
TARGET_PORT=""
MESSAGE="Hello from bitclaw-sender at $(date '+%Y-%m-%d %H:%M:%S')"
TRACKER_URL="http://127.0.0.1:8080"
NAME="test-sender"

while [[ $# -gt 0 ]]; do
    case $1 in
        --target-ip)
            TARGET_IP="$2"
            shift 2
            ;;
        --target-port)
            TARGET_PORT="$2"
            shift 2
            ;;
        --message)
            MESSAGE="$2"
            shift 2
            ;;
        --tracker-url)
            TRACKER_URL="$2"
            shift 2
            ;;
        --name)
            NAME="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 --target-port <port> [--target-ip <ip>] [--message <msg>]"
            echo ""
            echo "Options:"
            echo "  --target-ip     Target listener IP (default: 127.0.0.1)"
            echo "  --target-port   Target listener port (required)"
            echo "  --message       Message to send (default: auto-generated)"
            echo "  --tracker-url   Tracker URL for registration (default: http://127.0.0.1:8080)"
            echo "  --name          Sender name (default: test-sender)"
            echo "  -h, --help      Show this help"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo "========================================"
echo "  BitClaw Sender"
echo "========================================"
echo ""

# Validate port
if [ -z "$TARGET_PORT" ]; then
    log_error "Missing required argument: --target-port"
    echo ""
    echo "Usage: $0 --target-port <port> [--target-ip <ip>] [--message <msg>]"
    echo ""
    echo "Tip: Get the port from a running listener:"
    echo "  cat /tmp/bitclaw_listener_state.json | python3 -c \"import sys,json; print(json.load(sys.stdin)['listener_port'])\""
    exit 1
fi

# Check sender binary
if [ ! -f "$SENDER_BIN" ]; then
    log_warn "bitclaw-sender not found at $SENDER_BIN"
    log_info "Building bitclaw-sender..."
    ~/.cargo/bin/cargo build -p bitclaw_client --bin bitclaw-sender 2>&1 | tail -3
fi

if [ ! -f "$SENDER_BIN" ]; then
    log_error "Still cannot find bitclaw-sender after build attempt"
    exit 1
fi

# Check tracker is running
log_step "Checking tracker server..."
if curl -s "$TRACKER_URL/api/v1/hubs" > /dev/null 2>&1; then
    log_info "Tracker is running at $TRACKER_URL"
else
    log_warn "Tracker not reachable at $TRACKER_URL - message may not be delivered"
fi

# Send message
log_step "Sending message..."
log_info "Target: $TARGET_IP:$TARGET_PORT"
log_info "Message: $MESSAGE"
echo ""

$SENDER_BIN \
    --target-ip "$TARGET_IP" \
    --target-port "$TARGET_PORT" \
    --message "$MESSAGE" \
    --tracker-url "$TRACKER_URL" \
    --name "$NAME" \
    2>&1

EXIT_CODE=$?

echo ""
if [ $EXIT_CODE -eq 0 ]; then
    log_info "Message sent successfully!"
else
    log_error "Failed to send message (exit code: $EXIT_CODE)"
fi

# Try to verify if listener received the message
LISTENER_STATE="/tmp/bitclaw_listener_state.json"
if [ -f "$LISTENER_STATE" ]; then
    LISTENER_LOG=$(cat "$LISTENER_STATE" | python3 -c "import sys,json; print(json.load(sys.stdin).get('listener_log',''))" 2>/dev/null || echo "")
    if [ -n "$LISTENER_LOG" ] && [ -f "$LISTENER_LOG" ]; then
        echo ""
        log_step "Checking if message was received..."
        if grep -q "$(echo $MESSAGE | cut -d' ' -f1-3)" "$LISTENER_LOG" 2>/dev/null; then
            log_info "Message was received by listener!"
            echo ""
            log_info "Latest listener output:"
            tail -5 "$LISTENER_LOG"
        else
            log_warn "Message reception not confirmed - listener may need more time"
        fi
    fi
fi

exit $EXIT_CODE
