#!/bin/bash
# Manual test guide for OpenClaw + BitClaw integration
# This shows how to test the integration step by step

cat << 'EOF'
=================================================
  OpenClaw + BitClaw Tracker Integration Test
=================================================

PREREQUISITES:
1. BitClaw Tracker server running on http://localhost:8000
2. bitclaw-agent binary built and in PATH
3. OpenClaw installed with the bitclaw plugin

STEP 1: Start the Tracker Server
-------------------------------------------------
cd /Users/tomwu/bitagents/tracker/bitclaw_tracker
cat > .env << 'ENVEOF'
RUST_LOG=info
WEB_SERVER_HOST=0.0.0.0
WEB_SERVER_PORT=8000
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/postgres
ENVEOF

# Start in background
cargo run &
sleep 5

# Verify it's running
curl http://localhost:8000/api/v1/hubs


STEP 2: Verify bitclaw-agent CLI works
-------------------------------------------------
# List hubs (should return JSON with available hubs)
/Users/tomwu/bitagents/target/debug/bitclaw-agent list-hubs --tracker-url http://localhost:8000

# Connect to a hub
/Users/tomwu/bitagents/target/debug/bitclaw-agent connect-hub \
  --tracker-url http://localhost:8000 \
  --hub-name "test-hub" \
  --client-name "test-client"


STEP 3: Test via OpenClaw
-------------------------------------------------
# In your OpenClaw session, use the bitclaw_tracker tool:

# List available hubs
{
  "action": "list_hubs"
}

# Connect to a hub
{
  "action": "connect_hub",
  "hub_name": "test-hub",
  "client_name": "openclaw-agent-1",
  "lan_mode": true
}

# Find agents (including the one we just connected)
{
  "action": "find_agent",
  "hub": "*",
  "query": "openclaw"
}


STEP 4: Two-Client Test
-------------------------------------------------
# Terminal 1: Start OpenClaw with bitclaw plugin
openclaw

# In the chat, invoke the tool to connect:
# "Connect to the test-hub using bitclaw tracker"

# Terminal 2: Run a second client that sends messages
/Users/tomwu/bitagents/target/debug/bitclaw-agent connect-hub \
  --tracker-url http://localhost:8000 \
  --hub-name "test-hub" \
  --client-name "sender-client"

# The sender can now discover the OpenClaw client


STEP 5: Verify P2P Connection
-------------------------------------------------
# Both clients should be able to see each other
# Run this from either terminal:

/Users/tomwu/bitagents/target/debug/bitclaw-agent find-agent \
  --tracker-url http://localhost:8000 \
  --hub "test-hub" \
  --query "client"


CLEANUP:
-------------------------------------------------
# Stop the tracker server
pkill -f "bitclaw_tracker"

# Kill any running bitclaw-agent processes
pkill -f "bitclaw-agent"


TROUBLESHOOTING:
-------------------------------------------------
1. "Connection refused" on tracker URL:
   - Make sure tracker server is running
   - Check port 8000 is not in use

2. "Binary not found":
   - Build: cargo build --bin bitclaw-agent
   - Or set: export BITCLAW_AGENT_BIN=/path/to/bitclaw-agent

3. "Hub not found":
   - Check hub name matches exactly (case-sensitive)
   - List hubs first to see available options

=================================================
EOF
